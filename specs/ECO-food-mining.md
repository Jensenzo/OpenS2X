# ECO-FOOD: Mines, farm, fisher, hunter, donkey farm, shipyard

Status: draft (control flow read from the binary; values from the scripts; no in-game measurement yet)
Owner question(s): plan step 3.3; part of Q-005 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

How a mine finds the ore it digs and what happens when the ore runs out, and
the work cycles of the farm, fisherman, hunter, donkey farm and shipyard
(boat and ship mode). The woodcutter, stonemason and forester are in
`ECO-outdoor-workers.md` (ECO-OUT); several cycles here reuse their steps.
Workshop production rules (inputs, consumption, product choice, switch) are
in `ECO-production-chains.md` (ECO-PROD); building data in
`ECO-buildings-catalog.md` (ECO-CAT).

## Rules

### ECO-FOOD-1 Which cycle a worker runs

- **ECO-FOOD-1.1** When a worker has entered his building
  (`ECO-worker-cycle.md`), his cycle is chosen by the building type:
  woodcutter and quarry: ECO-OUT-3; forester: ECO-OUT-4; farm: ECO-FOOD-3;
  fisherman: ECO-FOOD-4; hunter: ECO-FOOD-5; donkey farm: ECO-FOOD-6;
  shipyard: ECO-FOOD-7; lookout tower and catapult: their own (not here);
  every other building with a worker (the four mines, the well, all
  workshops, the pig farm): the workshop cycle of `ECO-worker-cycle.md`
  with the production rules of ECO-PROD. Confidence: high.
- **ECO-FOOD-1.2** Every finished work action (product made, animal caught,
  ship work step) updates the building's productivity as in
  ECO-OUT-3.11. **The farmer's sowing does not** (corrected 2026-10-28,
  E-eco-031: only the end of the harvest's work action raises it;
  measured `ECO-outdoor-workers.md` T26: the Hard game's farm 90720 keeps the
  0.5 of its placement, less two falls, through two sowings). Confidence:
  high.
- **ECO-FOOD-1.3 Retry when a step cannot start.** When the next step of a
  cycle cannot start (typically because the building is switched off, or a
  search found nothing, below), the worker stays inside and tries again:
  the hunter, donkey breeder, shipwright and farmer every tick; the
  fisherman only in ticks where `(tick number + n) mod 100 = 0`, with `n` a
  fixed number of that settler (its object number, not traced), so once
  per 100 ticks. Confidence: high (control flow); medium for what `n` is.

### ECO-FOOD-2 Mines: ore field

- **ECO-FOOD-2.1** A mine (iron, gold, coal, stone) has a current ore field
  or none (ECO-PROD-3.3). A mine digs only the good it produces (iron ore,
  gold, coal, stone) from the map's resource layer
  (`FMT-s2m-map.md` FMT-S2M-4.5). Confidence: high.
- **ECO-FOOD-2.2 Search timer.** Every finished mine runs its own counter:
  +1 per tick; when it has reached 100 it is set to 0 and the search below
  is tried, so **one try every 101 ticks** (a counter separate from the
  target search of ECO-OUT-2.1). Confidence: high.
- **ECO-FOOD-2.2a Phase (Q-133).** The counter is 0 when the building is
  created and only runs once the building is finished, in the same
  per-building update as the worker request (ECO-REQ-1.4, also 0 at
  creation, also every 101 ticks). So both fire in the same tick, the 101st
  update after the building is finished, and then every 101 ticks; in that
  tick the worker request comes first, then the mine's try. The try does
  **not** run in the tick the miner arrives: an arriving miner waits for the
  next 101-tick try. Confidence: high.
- **ECO-FOOD-2.3 When the search runs.** The try is made only if the mine
  has its worker (he has arrived), the worker is a miner, the product good
  is a per-field resource (`gp_enableFields`: stone, coal, iron ore, gold;
  also fish, which no mine produces), and the mine has **no** current ore
  field. A mine whose field still holds ore never searches. Production
  being switched off does not stop the search. Confidence: high.
- **ECO-FOOD-2.4 Search area.** A flood (breadth first) from the mine's own
  field in ring order (directions 5, 0, 1, 2, 3, 4; MAP-GRID-3.2) as in
  ECO-OUT-2.3, but a field is entered **whenever its distance to the mine's
  field is at most the working radius** (`bp_setWorkingRadius`, 3 for all
  mines); terrain, walkability and field states do not matter. Goal test,
  for every entered field including the mine's own: the resource layer
  holds the mine's product good on it (amount at least 1, since a field
  whose amount reaches 0 loses its good). **The first goal in flood order
  becomes the current ore field.** Confidence: high.
- **ECO-FOOD-2.5 No reservation.** The ore field is not reserved: two mines
  (or a mine and the geologist's report) can use the same field; each dig
  takes one unit from whatever is there (ECO-PROD-3.3). Confidence: high.
- **ECO-FOOD-2.6 Nothing found.** If the search finds no field and the mine
  is switched on and its productivity is below 0.5 (a mine that has never
  produced counts as -1, ECO-OUT-3.11), the player gets the message "No
  resources found." (repeats limited as in ECO-REQ-3.8). The mine keeps
  searching every 101 ticks. Confidence: high.
- **ECO-FOOD-2.7 Running out.** When the miner begins a cycle with a current
  ore field he takes one unit at once (ECO-PROD-3.3); the unit that empties
  the field also clears the mine's ore field, and that cycle still produces.
  Without an ore field the miner does not begin a cycle (ECO-PROD-8.4) and
  waits as in ECO-WORK-3.1 until the next successful search (at most 101
  ticks later if ore is left within the radius). When no ore is left within
  the radius the mine stays idle for good and keeps showing the message of
  2.6 as its productivity falls below 0.5 (ECO-OUT-3.11: -0.05 every 600
  ticks). Confidence: high.
- **ECO-FOOD-2.8** So a mine digs the fields within distance 3 one after the
  other, nearest first in flood order, 7 units per field on the release maps
  (FMT-S2M-4.5). Confidence: high.

### ECO-FOOD-3 Farm

- **ECO-FOOD-3.1 Choice.** Each time the farmer is ready (after entering, and
  after every harvest or sowing trip), he checks in that tick whether the
  farm has a reserved ripe grain field: if yes, he goes to harvest it
  (3.2); if not, he goes sowing (3.3). If the chosen step cannot start (farm
  switched off), he checks again the next tick. Confidence: high.
- **ECO-FOOD-3.2 Harvest.** The farm finds grain fields with the building
  target search of ECO-OUT-2 (every 51 ticks, ECO-OUT-2.1; product corn, a deposit
  good; working radius 2; the field must be ripe, size 1.0, ECO-OUT-6). The
  harvest trip is the woodcutter's ECO-OUT-3.2 to 3.10 with the farm's
  production time **83 ticks** and the farmer's rest time (55, so 56 ticks).
  The harvested grain field starts being removed (ECO-OUT-6.4; `field_01`
  has no next deposit). Confidence: high (same objects as the woodcutter).
- **ECO-FOOD-3.2a Decision timing (play-test 2, chunk 4).** The farmer decides
  in the tick his rest ends, and again **in the tick after** a sowing step ends
  with no spot (the step's end takes its own tick; corrected 2026-10-27 b, was
  "at once"; measured T49): a ripe field reserved by the building's 51-tick search -> harvest,
  which starts with the woodcutter's target wait (`ECO-outdoor-workers.md`
  ECO-OUT-3.2: out 51 ticks after the decision; added 2026-10-27 night,
  E-eco-022, measured T21 there);
  otherwise the sowing step, which waits 50 ticks inside before its first
  spot search and 50 more after every search that could not finish (flood
  queue empty before 11 candidates or the radius + 4); a finished search with
  no candidate ends the step without a spot. After sowing he walks to the
  flag, in, and rests (ECO-OUT-9.1). As for the other outdoor workers
  (`ECO-outdoor-workers.md` ECO-OUT-3.10) no tick lies between the rest and
  the decision: the tick the rest is over is already the sowing step's first
  waiting tick (measured 2026-10-27 night, E-trace-011, farmer 91017: in at
  10326, out at 10434, `ECO-outdoor-workers.md` T20). Confidence: high
  (E-pt2-001).
- **ECO-FOOD-3.3 Sowing.** As the forester's ECO-OUT-4.2 to 4.8 with these
  values: working radius 2; planting good corn, so the new deposit is
  `field_01` and the spot must have planting terrain
  (`dp_setNeedForPlantablePattern`); production time 83; the new field
  starts at size 0.2 and grows by 0.00075 per tick (**1067 ticks** to
  ripe); its terrain becomes the acre pattern when sowing ends (its
  underground pattern); its lifetime of 3000 ticks counts down while it is
  not reserved (ECO-OUT-6.5). After sowing the farmer walks back to the
  flag, walks in and rests (ECO-OUT-4.8). If the spot search finishes
  without a spot, he goes straight back to 3.1 without resting.
  Confidence: high.
- **ECO-FOOD-3.4** A grain field that is ripe but not harvested within its
  lifetime withers (removed). A field reserved for harvest does not count
  down. Confidence: high (from ECO-OUT-6.5).
- **ECO-FOOD-3.5 Lifetime count in detail (Q-131).** Each tick, in the
  resource update (after the settler update, ECO-OUT-6.4), a grain field that
  is not being removed and has lifetime left above 0 loses 1 from it **only
  if no one has reserved it**. Its size and ripeness do not matter: the count
  runs while the field is still growing. Any reservation pauses it: the
  farmer's while he sows (so the first decrement is in the tick sowing ends,
  when the reservation ends), and the farm's harvest reservation from the
  target search (ECO-FOOD-3.2), also while the farmer is not yet on his way.
  In the tick the lifetime reaches 0 the field is marked for removal with
  removal counter 1, and the counter does not advance in that tick; it rises
  by 1 in each later tick and the field is deleted in the tick it reaches 20:
  **19 ticks after the tick the lifetime reached 0** (a harvested field, marked
  in the settler update, is gone after 18, ECO-OUT-6.4). So an unreserved
  field sown in tick S (sowing ends in S) has its last decrement in tick
  S + 2999 and is deleted in tick S + 3018. Confidence: high.

### ECO-FOOD-4 Fisherman

- **ECO-FOOD-4.1 Start.** The trip starts only while the hut is switched on.
  At the start, in the same tick, the fishing spot search (4.2) runs. If it
  finds no spot, the trip does not start (retry: ECO-FOOD-1.3, once per 100
  ticks). Confidence: high.
- **ECO-FOOD-4.1a The walk out comes a tick later.** In the start tick the
  fisherman only chooses (4.2, 4.3: the spot and its draw; he stands, the
  state already shows the trip). The walk out to the flag (4.4) is ordered
  in the **next tick**, and his first step begins then. This holds for
  every trip: the first after his arrival as well as each one after a
  rest; the start tick itself is unchanged. Confidence: high (E-eco-034: the start runs the spot choice, the walk out is
  begun by the first update after it; E-trace-014, test T51).
- **ECO-FOOD-4.2 Fishing spot search.** A flood from the hut's **flag field**
  (not the hut's field), ring order as in ECO-OUT-2.3. A field is entered
  only if it is walkable (ECO-OUT-2.3's walkable test) and its distance to
  the flag field is at most the working radius (8). Every entered field
  (the flag field included) is a **candidate** if it is a Coast field
  (`FMT-s2m-map.md` FMT-S2M-4.4, bit 3) and the resource layer holds fish
  on it or within the fishing distance around it (`cfg_setFishingDistance`,
  1: the field and its six neighbours). The flood runs until the queue is
  empty (no early end). Confidence: high.
- **ECO-FOOD-4.3 Choice.** No candidate: the message "No resources found."
  (repeats limited as in ECO-REQ-3.8) and no trip. Otherwise one candidate
  is chosen at random, each equally likely (one draw, `x mod n`,
  ECO-OUT-4.9), in flood order. The hut's **fish field** is then the first
  field holding fish in a spiral around the spot (the spot, then its ring
  at distance 1 in ring order, up to the fishing distance). Confidence:
  high.
- **ECO-FOOD-4.4 Trip.** Walk out to the flag (23 ticks), walk to the spot
  over the terrain (free walk, `TRN-free-walk.md`) and stands ON the spot,
  the Coast field (no stopping short, no fine approach; the fish field of
  4.3 only names the stock he takes from), then a timed action of the
  production time (**333 ticks**). Confidence: high (E-pt2-001).
- **ECO-FOOD-4.5 Catch.** When the action ends: if the hut has a fish field,
  one unit is taken from it (amount - 1, capped at 7; at 0 the field loses
  its good and the hut its fish field). If it has none (another hut or
  trip emptied it meanwhile), the first field holding fish within the
  fishing distance of the field the fisherman stands on loses one unit, if
  any. Confidence: high.
- **ECO-FOOD-4.6 Product.** The fisherman then takes a fish and carries it
  home exactly as ECO-OUT-3.7 to 3.10 (take, walk back to the flag, put
  down, walk in, rest: 1111, so 1112 ticks). As there, the next trip
  **starts in the tick the rest is over** (4.1: the spot search and its draw in
  that tick, the walk out ordered in the next, 4.1a): no tick lies between the
  rest and the start (stated 2026-10-29, measured T53). **The fish is made in every
  case**, also when 4.5 found no unit to take (the take step always takes
  the building's next product). Confidence: high (re-read 2026-10-17: the
  take step reads the building's next product, never the catch; E-pt2-001).
- **ECO-FOOD-4.6a A standing tick before the walk back.** He stands on the
  spot's point (4.4), so the lattice walk back of `ECO-outdoor-workers.md`
  ECO-OUT-3.8 has no step. In the tick the take ends (ECO-OUT-3.7), where another
  worker's walk back begins, he stands. His walk to the flag, the free walk
  of ECO-OUT-3.8 from the spot, begins in the next tick. The entry check of
  ECO-OUT-3.8a (1) is still made in the tick the take ends. While he is not
  admitted, he checks again in each following tick and goes on standing, as
  in ECO-OUT-3.8's standing tick.
  Confidence: high on the timing (measured, 3 of 3 trips of E-trace-014's
  fisher, T52), medium on the check in the standing tick (not seen at a
  full flag).

### ECO-FOOD-5 Hunter

- **ECO-FOOD-5.1 Animal search (the building).** A hunter's hut searches for
  game with its own rule. The hut shares the counter of the target search
  (ECO-OUT-2.1), which the hunter's own update also advances (that update
  runs for every building, so every target search is 51-tick too): in a tick
  where the counter is not 0 it gains 2 (1 + 1); when it has reached 100 it
  is set to 0 and, in that same tick, the animal search runs. So **one
  search every 51 ticks**. The search runs only if the hut has no reserved
  animal. The worker request (ECO-REQ-1.4) has its own 101-tick counter and
  is not affected (Q-132 a). Confidence: high.
- **ECO-FOOD-5.2 Search area.** A flood from the hut's field in ring order;
  a field is entered whenever its distance to the hut's field is at most
  the working radius (8), regardless of terrain. Goal: an animal stands on
  the field (the field it is registered on) and **the field its current walk
  ends on** (its own field while it stands) has been explored by the hut's
  owner (the per-player exploration of `ECO-warehouse-construction.md`
  ECO-HQ-3; corrected 2026-10-28, E-eco-032: the test reads the
  animal's walk goal, not the flood's field; measured T50).
  Only an animal that **no hut has reserved** counts: on a field with several
  animals the first unreserved one is taken, a field with only reserved
  animals is no goal (Q-132 b). **"First" is the order in which the animals
  were registered on the field** (5.10a: an animal is registered there when a
  step onto it completes), the earliest first, not the order of the animals'
  numbers (clarified 2026-11-01, T59; measured, the field's list not read in
  the code: medium-high). The
  first goal in flood order is taken: the animal is reserved for the hut and
  switched to a "hunted" state (it presumably stops roaming; animal
  behaviour is not specified here). Confidence: high for the search and the
  exploration test (code and T50), medium for the hunted state.
- **ECO-FOOD-5.3 Trip.** As ECO-OUT-3.2 to 3.4: the hunter waits inside for
  a reserved animal (countdown 50, first check 51 ticks after the step
  starts, "No resources found." under the productivity rule), walks out to
  the flag, walks to the animal over the terrain (one field short, then on
  the lattice to 2 steps short of the animal's point: `ECO-outdoor-workers.md`
  ECO-OUT-8.1, T18). On arrival, if the animal
  is still reserved for the hut, it is stopped and a timed action of the
  production time (**55 ticks**) runs; when it ends the reservation ends and
  the animal is killed (removed from the map). Confidence: high (order; the
  walk end point measured 2026-10-25 evening).
- **ECO-FOOD-5.4 Product.** Then, as ECO-OUT-3.7 to 3.10: take a ham, walk
  back, put it down, walk in, rest (222, so 223 ticks). If the animal was
  gone on arrival, there is no action and no kill, but the hunter still
  takes a ham and carries it home (same reason as ECO-FOOD-4.6). Confidence:
  high (E-pt2-001: the hunt step ends as done and the
  normal take follows).
- **ECO-FOOD-5.5** The hunter never consumes pigs (ECO-PROD-2.3) and has no
  inputs. Confidence: high.
- **ECO-FOOD-5.6 Switched off (Q-132 c).** The hunter's trip starts with the
  same step as the woodcutter's (ECO-OUT-3.1): while the hut is switched off
  the trip does not start (retry every tick, ECO-FOOD-1.3). The hut's animal
  search (5.1) still runs. Confidence: high.
- **ECO-FOOD-5.7 Animals roam (Q-132 d, Q-058).** Animals are not fixed to
  their map field. Every animal has its own update each tick, in the resource
  update, visiting the animals in the world's animal list order. An animal
  has a home field (where it was placed), a current field and a state:
  waiting, choosing, walking or hunted. A reserved animal is switched to the
  hunted state, in which this update does nothing: it never waits or chooses
  again, but **a walk it is on goes on to its end** (moved as in walking, at its
  speed), and from there it stands still until killed; the end of that walk sets
  no wait (corrected 2026-10-25 evening, measured E-trace-010: rabbit 90092,
  reserved in trace tick 2206 on its walk from (139, 54) to (146, 61), walked the
  remaining 7 fields and stood on (146, 61) from 2348; an experiment build that
  stopped it at once kept it on (140, 54), where the rabbit 90085's choice of
  trace tick 2434 counts it as registered: generator 2434 -> 2687 when
  corrected). **Released during that walk** (its hut removed, `MIL-conquest.md`
  MIL-CON-4.1), the animal is walking again with its walk's original start time:
  the walk goes on, and its end sets the wait of 5.11 from that start time,
  the reserved stretch included (measured 2026-10-25 evening, T46). **Reserved
  while waiting**, the animal keeps its wait counter unchanged (it does not
  count down while reserved); released, it is waiting again with that counter
  at once, in the removal's tick (that tick shows the kept value), and counts
  down from its next update; no new wait is drawn and it does not choose
  (measured 2026-10-27 night, E-trace-011, T47; an experiment build that
  releases it with wait 0 makes it choose at once, generator 7955 -> 9156 when
  corrected). **Released while standing** (2026-11-15 day; E-eco-045, the R6 recording E-trace-019
  and an experiment build): an animal whose hunted walk is over and which stands waiting for its
  hunter is likewise waiting again **in the removal's tick**, with the wait its walk left, which is 0
  (a walk does not count a wait). So its next update finds the wait below 0 and makes it choosing
  with ticket -1 (5.8), and it chooses by 5.9 from the update after that. It does not spend an
  update first in the hunted state. Measured in R6: player 2's hunter's hut on (46, 20) is torn down
  at counter 6631; its deer, standing on (55, 14) since 6377, is waiting with 0 at 6631, choosing
  with ticket -1 at 6632 and 0 at 6633, and walks to (57, 18) at 6634 with its three draws (5.14).
  An experiment build that left it hunted for one more update drew them at 6635 (the fight game's
  generator 6635 -> 6714 when corrected; the other recorded games unchanged). Test T62. Kind values (`animals.lua`; defaults in brackets): range
  `an_setRange` (deer 6, rabbit 8, elk 6; [-1]), move distance
  `an_setMoveDistance` (deer 2, rabbit 4, elk 2; [3]), speed
  `an_setMovementSpeed` (deer 0.045, rabbit 0.07, elk 0.045; [0.1]), avoid
  buildings `an_setAvoidBuildings` (all true; [true]), idle time factor
  `an_setIdleTimeFactor` (no script sets it; [3.0], never below 2.0).
  Confidence: high.
- **ECO-FOOD-5.8 Waiting.** A waiting animal counts its wait down by 1 per
  update; when the count goes below 0 it switches to choosing with ticket -1
  (it chooses from the next update on). So a wait of W lasts W + 1 updates.
  Confidence: high.
- **ECO-FOOD-5.9 Start limit (tickets).** The resource update keeps two
  numbers for **all** animals together: `started`, set to 0 at the start of
  each tick's animal pass, and `top` (kept across ticks, starts at 0). A
  choosing animal with ticket `q`:
  1. if `started >= 2`: `q` rises by 1, nothing else;
  2. else if `top < q`: `top = q`, and the animal chooses a walk now (5.10);
  3. else if `top == q`: `top = 0`, `started` rises by 1, and the animal
     chooses now;
  4. else (`q < top`): `q` rises by 1.
  So an animal entering the choosing state with ticket -1 usually chooses in
  its second choosing update, and at most two animals per tick go through
  rule 3. `top` is never reset by the tick, only by rule 3. The animals update
  in ascending object number (the original keeps them in a map keyed by the
  64-bit object id; the map's records get ascending ids in file order, a
  respawned animal the next id). Confidence: high (the rules as the original
  applies them; the queues they build, tickets up to 11 at trace tick 140 of
  E-trace-004, are matched by the generator; seen directly in E-trace-005's animals
  table: every state, ticket and start of trace ticks 530 .. 561 follows rules 1 to 4
  in id order, T42).
- **ECO-FOOD-5.10 Choosing a target.** The centre is the animal's **current
  field**; only when the current field is farther than `range` from home is the
  centre the home field. A number `N` is drawn: `draw mod (3 x md x (md + 1)) + 1`,
  `md` = move distance (deer: 1..18, rabbit: 1..60). Then a breadth-first search
  over walkable fields starts at the centre (the centre itself first, without a
  walkable test; from each field its six neighbours in the ring order of
  MAP-GRID-3.2, directions 5, 0, 1, 2, 3, 4), taking only fields within `range`
  of the animal's **home** field (lattice distance, MAP-GRID-2; no other bound)
  that are walkable (ECO-OUT-2.3's bits). Each visited field is acceptable when:
  it is not a road, mining, border or harbour field (field state bits Street,
  ForMining, Border, ForHarbor clear, FMT-S2M-4.4); no animal is registered on
  it (5.10a); **no flag stands on it** (a flag counts on its own field only, for
  every kind; measured 2026-10-27 night, T48: counting flags within 2 like
  buildings breaks the generator at trace tick 20; a flag with a road carries
  the Street state anyway, `TRN-roads-flags.md` TRN-ROAD-3.1, so this clause may
  be that state; whether a flag without roads is refused is not measured); and, for a kind that avoids
  buildings, no building or construction
  site stands on any field within 2 of it, **the field itself included** (a
  building counts on its own field only, not on the rest of its footprint). The
  N-th acceptable field is the target; if the search ends with fewer than N,
  the last acceptable field found is the target. Confidence: high (centre and
  bound corrected 2026-10-19 from the code, E-sim-002, and
  measured: with the current field as centre the generator of E-trace-002 is
  matched through trace tick 108 instead of 98; the within-2 area and the
  building layer from the code and the raw dump of E-trace-002).
- **ECO-FOOD-5.10a Where an animal is registered.** An animal is registered on
  one field: the field it stands on; while it walks, the field it last reached
  (the walk's start field during the first step). The registration moves when a
  step completes, never when a step or a walk starts; a choosing animal is
  registered on its own field, so that field is never acceptable to it (Q-233).
  Confidence: high (E-sim-002; the other placements part
  from the recording earlier: creation field at trace tick 48, step end at 19).
- **ECO-FOOD-5.11 Walking.** With a target the animal stores the tick clock
  (5.11b), starts walking to it (5.11a) and switches to walking. With no
  acceptable field at all, or when its walk cannot start, it stays in choosing
  with ticket 100. A walking animal, once its movement has ended, switches to
  waiting with `W = trunc((t_end - t_start) x idle factor x 10)`, where `t_start`
  is the tick clock read when it chose and `t_end` the tick clock read in the
  update in which its last step completed (5.11b). Confidence: high (measured,
  5.11b).
- **ECO-FOOD-5.11a The animal's walk.** The walk is the free walk's search and
  read-out (TRN-FREE-2, 3) from the animal's field to the target, with two
  differences: the step cost of a kind that avoids buildings is the free walk's
  cost plus 10 for each building or construction site (on the building's own
  field, as in 5.10) among **the field the animal steps from** and its six
  neighbours; and there is no second try with every field passable (TRN-FREE-2.8): if
  the first search fails the animal does not walk (ticket 100, 5.11). The draws
  are the free walk's (5.14). Confidence: medium (code, E-sim-002; not yet exercised by a recording, no building near the animals of
  the matched ticks).
  - **5.11a (a) Which field carries the extra (Q-356).** In the search's terms
    (TRN-FREE-2.5: the search runs from the target, the walker steps from the
    neighbour `Q` to the field `N` taken from the list) the extra belongs to `Q`:
    `c(N, Q) = free cost + 10 x (buildings and sites among Q and its six
    neighbours)`. So a field's label already holds the extra for leaving it, and
    the read-out (TRN-FREE-3.1) avoids a field next to a building wherever an
    equal route without it exists; the walk's first field `W` pays for its own
    surroundings, the target's surroundings cost nothing. The extra is added only
    when the free cost is a cost (the field is passable, TRN-FREE-2.5). Nothing
    else counts: flags and road fields are not in the buildings layer (one entry
    per building, on its own field; raw layer dump of E-trace-002, 5.10). Confidence: high (measured: rabbit 90092 of
    E-trace-008, counter 1030, T43, takes the recorded route only with the extra
    on `Q`; with it s2-reborn's generator matches the original's through trace
    tick 1139 instead of 1030; code E-sim-012:
    the area of radius 1 around the cost function's second field, which is the
    neighbour being labelled).
- **ECO-FOOD-5.11b The tick clock.** The clock the animals read is the game's
  "next tick time", a 32-bit float that starts at 0 and gains 0.1 (as a 32-bit
  float) once per tick, **after** the animals' update of that tick. So in trace
  tick T the animals read the float32 sum of T - 1 terms of 0.1f (0 in trace tick 1,
  0.1 in 2, 0.70000005 in 8, 1.0000001 in 11, 7.1999955 in 73, 7.2999954 in 74),
  and the recorded column `next` of a trace tick (TRACE-4.6, read between ticks)
  holds the sum of T terms, the value the animals read in the next tick. The wait
  of 5.11 is computed in **single precision** throughout (the x87 runs at 24-bit
  precision, as Direct3D leaves it): `d = f32(t_end - t_start)`, `p = f32(d x idle)`,
  `q = f32(p x 10)`, `W = trunc(q)`; the result is 3 x (walk ticks) only when the
  rounding falls that way, else often one less (T35). Confidence: high (measured:
  with the clock one tick behind and the single-precision product the generator of
  E-trace-002 is matched through trace tick 330 and of E-trace-004 through 424; with
  the clock of the current tick or a double product the first differences are 227,
  then 301 / 298; code E-sim-002).
- **ECO-FOOD-5.12 Walking speed (play-test 2, chunk 3).** A walking animal has a
  step progress like a settler (TRN-MOVE-2): each tick the progress grows by
  its kind's speed (`an_setMovementSpeed`: deer 0.045, rabbit 0.07, elk
  0.045 per tick) divided by the lattice distance of the step (1 between
  neighbours); the step is complete in the tick the progress **exceeds** 1.0 (a
  progress of exactly 1.0 completes one tick later), the excess carries into
  the next step (progress and excess are 0 at a walk's start, and the walk's
  waypoints are every field after the animal's own up to and including the
  target), and the walk ends when the path's last field is reached. **Slopes do not slow animals**: the slope factor
  of TRN-MOVE-1.3 applies to settlers only. So a deer needs 22.2 ticks per
  field, a rabbit 14.3, on any terrain. With the default idle factor the
  wait after a walk of k fields is about 3 x 22.2 x k ticks (5.11), so an
  animal stands about three quarters of the time and walks in bursts of a
  few fields (move distance 2: the target is mostly 1 to 3 fields away).
  Confidence: high (E-eco-016).


- **ECO-FOOD-5.13 Animals at the start of a game (Q-232).** A free game's animals
  are the map's animal records (FMT-S2M-5.4), each with the state and the counter the
  record holds; nothing is drawn for them at load. On MP_2P_Storm_Coast all 261 are
  waiting (state 0) with counters from 0 to 169 (three at 0, one at 1, two at 2, ...),
  so their first choices spread over the first 170 ticks. The animal update of 5.7
  visits them in ascending object number, which is the records' order in the map
  file. An animal placed during the game (respawn, 5.15) draws its first wait instead
  (5.15). Confidence: high (measured: the map's counters put through 5.8 and 5.9
  reproduce the original's draw pattern tick for tick, see T31; map order from
  the save layout, medium).
- **ECO-FOOD-5.14 Draws (Q-232).** Per choice an animal draws once, for N (5.10);
  then its walk draws like a settler's free walk (TRN-FREE-3.2a: one draw for the
  starting direction when the target is not a neighbour of the animal's field, none
  for a single-step route, and one more at every step where the kept direction is
  not allowed; the animal's own walkable test applies). The animal's clock for the
  wait after a walk (5.11) is the tick clock read at the walk's start and end
  (5.11b: a float32 sum, so the elapsed time is not exactly 0.1 s per tick). Confidence: high (measured: the draws per tick of
  the original equal the N-draws of T31 plus 0 to 14 walk draws, never fewer, and
  the two path read-outs are the same code; the formula for the wait from code).
- **ECO-FOOD-5.15 Respawn (Q-235).** The map's respawn record (FMT-S2M-5.5) holds
  an accumulator, a rate (40 on MP_2P_Storm_Coast) and a cell cursor (x, y). Every
  tick, after the animals' update, the accumulator gains the rate; while it is 1000 or
  more: the cursor advances (x + 1; at the grid's width x = 0 and y + 1; at the grid's
  height y = 0), 1000 is taken off, and one respawn check runs on the cursor's cell.
  The cells are **16 x 16 fields** (cell = field div 16), the grid ceil(width / 16) x
  ceil(height / 16) (11 x 11 on a 170 map); from the record's (9, 9) the first check,
  in trace tick 25, is on cell (10, 9). A check:
  1. does nothing when the cell's animal count is above 4: the count of living
     animals whose **home** field lies in the cell (counted at creation, not where
     the animals stand);
  2. does nothing when the wood recorded for the cell in the computer players'
     low-resolution resource map (16-field cells; the amounts of the surface deposits
     yielding wood, the trees) is 30 or less;
  3. else looks at the cell's 4 x 4 sub-cells, the resource map's 4-field cells
     (4cx + i, 4cy + j), i (x) outer and j (y) inner; the first sub-cell whose wood
     is above 12 and whose flood finds a field gets one animal: the flood starts at
     the sub-cell's top-left field (16cx + 4i, 16cy + 4j), visits walkable fields
     (the start without a walkable test) within lattice distance 8 of the start in
     the ring order of 5.10, and accepts the first field on which, and on each of
     whose six neighbours, a tree deposit (a deposit yielding wood) stands;
  4. the animal's kind is drawn, `draw mod (number of animal kinds)`, the kinds in
     **ascending id order** (rabbit 41797b76, deer 4a9bef83, elk 706e7c94: 0 places
     a rabbit, 1 a deer, 2 an elk), then the placement draws its first wait,
     `draw mod trunc(move distance / speed x idle factor)` (deer 2 / 0.045 x 3.0 =
     133: 0 .. 132; rabbit 171): two draws per placed animal, none for a check that
     places nothing. The new animal's home is the field; it waits, chooses and walks
     like the others.
  On MP_2P_Storm_Coast the count went 261, 266, 271, 274, 276, 276, 278, 281, 281,
  281, 283 at trace ticks 1, 1000, ..., 10 000 (E-trace-002): 5 placements in the
  first 40 checks, none in the checks at 25 .. 125. In the one-player game
  (E-trace-004) the first placement is in trace tick 425, the 17th check: the
  generator rows show its two draws after the animals' draws of that tick, and with
  two draws added there s2-reborn's generator (5.11b corrected) matches through
  trace tick 452 (T41). Confidence: high for the
  accumulator, cursor, cell size and the two draws (E-sim-002, animal counts
  measured). The wood of a cell or sub-cell is the number of tree deposits in it
  (deposits whose good is wood), and every rule above is measured: an experiment
  build with 5.11b and this rule matches the one-player generator (E-trace-004)
  through trace tick 4737, with 21 placements (T40, T41). Confidence: high.
- **ECO-FOOD-5.16 Animals under a new building (2026-10-25 evening).** When a
  building is created (a construction site placed by any player, or a building
  created finished), every animal registered (5.10a) on one of its footprint
  fields is removed from the game at once, in the same tick, without a draw:
  the building's field and, for a large building or a harbour, its west,
  north-west and north-east neighbours (`ECO-building-placement.md`
  ECO-PLACE-3.6, 5.1), in that order. The flag's field is not part of it. The
  removal is the same as a hunter's kill (5.3) without the ham: the animal is
  gone from the lists of 5.9 and 5.15 and from a hunter's reservation (a hunter
  on his way finds it gone, T13). Measured (T45). Confidence: high for the
  footprint and the moment (E-eco-018),
  medium for the effect on the respawn count (shared code with the kill, not
  measured).

### ECO-FOOD-6 Donkey farm

- **ECO-FOOD-6.1** The donkey breeder runs the workshop production cycle
  (ECO-PROD: can-produce check, inputs corn 1 and water 1, production time
  333, consumption and re-order at work end). Confidence: high.
- **ECO-FOOD-6.2** After each production the breeder does **not** carry a
  ware out. Instead, in the next tick, a new settler of the profession
  `good_donkey` (D4157C5F, the donkey that works on roads) is created at the
  farm's door, as a carrier of the donkey kind. The ware `donkey`
  (2448356E) is never made. There is no rest between productions, but the
  next production does not start in the donkey's tick: in that tick the
  breeder only goes back to waiting, and his first can-produce check is in
  the next tick (then every tick until it succeeds). With work ticks W ..
  W + 332 he stands in W + 333 (the "next tick" above), the donkey appears in
  W + 334 (on the door field, `ECO-worker-cycle.md` T-door-5) while he waits,
  and with corn and water at hand the next work begins in W + 335: a cycle of
  335 ticks, not 334 (corrected 2026-10-31, measured T58). Confidence: high
  for the creation and the absent rest; high for the timing (two recordings).
- **ECO-FOOD-6.3 The new donkey's first moves.** The donkey is created with
  no road. In its first update it walks out of the farm to the farm's flag,
  exactly like any settler leaving a building (the door walk of
  `ECO-worker-cycle.md`). When it stands at the flag it **goes home**
  (`TRN-free-walk.md` TRN-FREE-6.4): it walks to the warehouse with the
  lowest route cost from the flag that accepts donkeys (`ECO-distribution.md`
  ECO-DIST-1.5), enters it and is added to its stock. It never goes straight
  to a road: a road that wants a donkey files a request, and the request is
  served from warehouse stock like any settler request
  (`ECO-requests-dispatch.md` ECO-REQ-3). If no warehouse can be reached, the
  going-home fallbacks of TRN-FREE-6.4 apply. Confidence: high for the
  sequence (walk out, then go home); the donkey type check is explicit in the
  carrier update.

### ECO-FOOD-7 Shipyard

- **ECO-FOOD-7.1 Mode.** The shipyard's switch (ECO-PROD-1) selects the
  mode: not switched = boats, switched = ships. The mode is read each time
  the shipwright is ready for the next step. Confidence: high.
- **ECO-FOOD-7.2 Boat mode.** The workshop production cycle (ECO-PROD) with
  product boat: inputs plank 1 (the boat's cost), production time 333;
  then the boat is carried to the flag and put down like a workshop
  product, the shipwright walks in and rests (222). Confidence: high.
- **ECO-FOOD-7.3 Ship mode, finding the ship.** Inside, a counter runs to
  100 (one check **101 ticks** after the step starts). Then, if the
  shipyard can produce (switched on, worker present, a plank in stock:
  ECO-PROD-3.1), the shipwright is linked to the player's ship under
  construction within the working radius (8) of the shipyard; if there is
  none, a new ship site is looked for within the same radius and, if one is
  found, a ship is started there (7.3a, 7.3b). If he has
  no ship after this, the step ends and the shipwright starts it again (the
  next check is 101 ticks later). Confidence: high.
  **Starting the step** (added 2026-10-30, T54; E-eco-040, E-eco-009): between steps the shipwright is in his mode check (7.1), once per
  tick. The ship step **starts only while the shipyard's production is switched
  on** (ECO-PROD-1); while it is off the start fails and he stays in the mode
  check, trying again every tick, and the 101-tick count only begins in the tick
  the start succeeds. A step that ends without a ship returns him to the mode
  check for that tick, and the next step starts in the following tick: the
  checks of a shipwright who finds no ship come **102 ticks** apart (one check 101
  ticks after the step's start, the new start one tick after the check).
  Confidence: high (code; measured T54).
- **ECO-FOOD-7.3a Which ship he joins (Q-130).** Among all ships in the game,
  the candidates are those that belong to the shipyard's player, are not
  finished (frame or main work left), have **no shipwright linked** to them,
  and whose building field (7.3b) lies on the same continent
  (`FMT-s2m-map.md` continent layer) as the shipyard's field and within the
  working radius (8) of the shipyard's field. The nearest one wins (first in
  the game's ship list on equal distance). Linking is exclusive: one
  shipwright per ship. The link ends at the end of every work step (7.4), so
  on his next trip the same shipwright normally finds the same ship again;
  while he is on a trip, another shipyard's shipwright cannot join that ship
  and starts a new one. Confidence: high.
- **ECO-FOOD-7.3b Where a new ship is started (Q-130).** A flood from the
  shipyard's own field, each field visited at most once, neighbours taken in
  the flood's ring order of `MAP-grid-geometry.md` MAP-GRID-3.2 (directions 5, 0, 1, 2, 3,
  4: the original's own direction indices 0 to 5 start at MAP-GRID direction 5)
  from each field taken out of the queue (first in, first out); a field is tested
  in the order it was entered (clarified 2026-10-30, measured T57: taking the
  MAP-GRID directions 0 to 5 instead starts the ship one field off). The shipyard's field is always entered; any other field is
  entered only if it is walkable (ECO-OUT-2.3's walkable test) and within the
  working radius (8) of the shipyard's field. The search ends at the first
  entered field (the shipyard's field included) that is a **site**:
  - it is a Coast field (MAP-GRID-7) on the same continent as the shipyard's
    field, and
  - at least one of its six neighbours is inside the map, is Water, is not
    DoodadBlocked (FMT-S2M-4.4 bit 10), and is not the water field of an
    existing ship (of any player) **still under construction**: a finished ship
    (it has launched, ECO-FOOD-7.4b) no longer holds it, so the next ship can
    be started on the same building field (added 2026-11-02 night, T61).
  The neighbours are tested in the order: direction 4, 5, 0, 1, 2, 3 of the
  original's indices (MAP-GRID directions 3, 4, 5, 0, 1, 2 by the same shift;
  medium, not measured: T57's site has one Water neighbour), and the
  **last** one that passes is the ship's water field. The ship is created
  with its building field = the Coast field and its water field = that
  neighbour, frame work 1 and main work 1 (both 0 in a game where buildings
  are created already finished, as for game-created warehouses, ECO-HQ-1.2).
  Territory is not checked, and the number of ships a player may have is not
  limited here. If the flood ends without a site, no ship is started.
  Confidence: high.
- **ECO-FOOD-7.4 Ship mode, a work trip.** With a ship: the shipyard's
  inputs are consumed as at the end of a production (one plank, re-ordered,
  ECO-PROD-4) and the shipwright carries a plank; he walks out to the flag,
  walks over the terrain to the ship's **building field** (the Coast field of
  7.3b) with the walk-to-target step of ECO-OUT-3.4 (given the parameter 2;
  where exactly he stops is not traced), puts the plank down there (the
  timed put-down step of ECO-WORK-3.6, drop time then pickup time), and works
  one step (7.4a). Then he walks back to the flag, walks in and rests.
  **Timing at the ship (measured 2026-10-30, T55):** arriving on the building
  field in tick A, he puts the plank down in A to A + 7 (the drop for 4 ticks,
  then the pick-up motion for 4, as at a flag), stands in A + 8, the work step
  of 7.4a starts in A + 9 with its first decrement and makes its 56th in A + 64,
  where the step ends; his walk back is ordered in A + 65 (progress 0) and first
  advances in A + 66, so he stands on the field for 66 ticks, A to A + 65 (A is
  the first tick he stands there, the drop's first tick; clarified 2026-10-30
  with the implementation's count). Confidence: high for the order; high for
  the put-down timing (measured T55).
  **The plank he carries is a ware object** (`SIM-game-start.md` SIM-START-4.2):
  it takes the next number in the tick the trip starts (after the ship's own
  number when that check started a new ship) and ends in A + 4, with the drop at
  the ship (added 2026-10-30, measured T56).
- **ECO-FOOD-7.4a The work step (Q-130).** When the step starts, the amount
  per decrement is fixed: `1 / (55 x 4)` while the ship has frame work left,
  else `1 / (55 x 8)` (ship construct time `sh_setConstructTime` 55, plank
  counts of `sh_addFrameCost` / `sh_addCost`), and a countdown is set to 55.
  Each tick while the countdown is above 0: a timed work action of 55 is
  started if the shipwright has none running, the countdown falls by 1 and
  the ship's work falls by the amount. In the tick the countdown is 0: the
  link between shipwright and ship ends, the work falls by the amount once
  more (so **56 decrements per trip**), and the step ends. Each decrement
  lowers the frame work while any is left, else the main work, and never
  goes below 0 (a decrement that empties the frame does not carry over into
  the main part). So the frame is done in trip 4 (4 x 56 > 220) with 4
  decrements of 1/220 going to the main part, and the main part is done in
  trip 12: **12 trips, 12 planks**. Confidence: high.
- **ECO-FOOD-7.4b Ship complete.** At the end of a work step (the tick the
  countdown is 0), if both frame and main work are 0, the player gets the
  message **"Ship constructed"** (message type 4, placed at the ship's water
  field). **It does not name the ship** (2026-11-16 day, Q-1065; E-ship-009, code reading,
  high): the row is the text alone. Of all messages, only "Ship waiting for Orders" (SHIP-2.3,
  SHIP-6) is made with the ship's name; the game's other uses of the ship's name are its
  windows (the ship window, the fleet page). **The ship itself is finished earlier**: in the tick a decrement takes
  the main work to 0, partway through the last trip, and its state machine starts
  then (`SHIP-ships-expeditions.md` SHIP-2.1), while the shipwright's step goes
  on to its 56th decrement (added 2026-11-02 night, T60). The work parts are
  single-precision floats lowered by the fixed amount of 7.4a, so the decrement
  that empties a part is found by that arithmetic (a residue below the amount is
  taken by one more decrement, clamped at 0), not by counting whole trips. What the finished ship does next: `SHIP-ships-expeditions.md`
  SHIP-2.1.
  Confidence: high for the message and its moment.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| mine search period | 101 | ticks | high | E-eco-009 |
| mine search radius | working radius (3) | fields | high | E-eco-009 |
| hunter search period | 51 | ticks | high | E-eco-009 |
| hunter search radius | working radius (8) | fields | high | E-eco-009 |
| fishing distance | 1 | fields | high | `config.lua`, E-eco-009 (read from the config) |
| fisher retry grid | 100 | ticks | high | E-eco-009 |
| farm production / radius | 83 / 2 | ticks / fields | high | ECO-CAT |
| grain field start size, speed, lifetime | 0.2, 0.00075, 3000 | | high | `deposits.lua` |
| ship trips per ship | 12 (4 + 8) | | high | `ship.lua`, E-eco-009 |
| ship-mode check period | 101 | ticks | high | E-eco-009 |

## Test cases

Level ground, room at the flags, switched on, unless stated.

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | coal mine at M, coal 7 on a field at distance 2, coal 7 on a field at distance 1, worker arrived | first search | the distance-1 field (earlier in flood order) is chosen |
| T2 | mine with one field of 7 units in reach, food in stock | run | 7 products; after the 7th dig the mine has no ore field; no further cycle |
| T3 | mine whose only ore field (amount 3) lies at distance 4 | run | never finds it; "No resources found." while productivity < 0.5 |
| T4 | two coal mines with the same single field of 7 in reach | run | 7 products in total between them |
| T5 | hunter's hut finished at tick 0, an explored animal at distance 5 | animal search | searches in ticks 51, 102, ...; the animal is reserved at the first search after it is in reach |
| T6 | fisherman's hut, no Coast field with fish within 8 of its flag | worker ready | "No resources found."; next try in the first tick with (tick + n) mod 100 = 0 |
| T7 | fisherman, coast field C with fish 1 on its water neighbour W | one trip | W emptied (good cleared); one fish put down at the flag; the next trip needs another candidate |
| T8 | fisherman, fish field emptied by another hut during the trip | catch | no unit taken (none within 1 of where he stands), a fish is still made |
| T9 | farm, no grain field, plantable meadow around | worker ready | sows; the field is ripe 1067 ticks after sowing ended; harvested by a later trip |
| T10 | farm, ripe field reserved | worker ready | harvest trip; production time 83; the field is removed 18 ticks after the harvest tick |
| T11 | donkey farm with corn 1 and water 1 | one production | a donkey settler appears at the door in the tick after production ends; no ware at the flag; the next production starts that tick |
| T11a | as T11, HQ reachable by road, no road requests a donkey | follow the donkey | it walks out to the farm's flag, then along the roads to the HQ and enters it; HQ donkey count +1 |
| T12 | shipyard switched to ships, plank stock 12, coast site in reach | run | 12 work trips of one plank each, then the ship is complete |
| T13 | hunter whose reserved animal is gone (removed) before he arrives | arrival | no action, no kill; a ham is still carried home |
| T14 | deer walks 2 fields uphill (rise class 3) | ticks | 45 ticks for the walk (22.2 per field, carry), then a wait of 135 ticks (+1 update), then choosing |
| T15 | rabbit walks 1 field | ticks | 15 ticks (progress 0.07 x 15 = 1.05 >= 1), carry 0.05 into no next step |
| T14 | coal mine finished in tick F, miner arrives in tick F + 150, coal in reach | searches | worker request and ore search both in tick F + 100 (no miner yet: no search), next try F + 201 finds the field; none in F + 150 |
| T15 | farm sows a field, sowing ends in tick S; the field is never harvested | wither | lifetime 1 left after tick S + 2998, 0 in tick S + 2999 (marked, removal counter 1); deleted in tick S + 3018 |
| T16 | ripe field reserved by the farm's target search in tick R, the farmer starts the harvest 500 ticks later | lifetime | the lifetime does not change from tick R on |
| T17 | two hunters' huts, one animal in reach of both | searches | the first hut's search reserves it; the second hut finds no goal on that field |
| T18 | hunter's hut switched off, animal reserved | worker ready | no trip while off; trip starts in the tick after it is switched on (every-tick retry) |
| T19 | shipyard at S in ship mode, the only Coast field within 8 (walkable path) is C at distance 3, C's neighbours in directions 1 and 3 are Water | first ship | ship building field C, water field = the direction-3 neighbour |
| T20 | as T19, a ship of another player already uses C's direction-3 neighbour as water field | first ship | water field = the direction-1 neighbour |
| T21 | shipyard with a ship of its player at frame 0.5 within 8, no shipwright linked | ship check | linked to that ship, no new ship |
| T22 | ship with frame work 1.0 | 4 work trips | after trip 3: frame 1 - 168/220; after trip 4: frame 0, main 1 - 4/220 |
| T23 | ship with main work 8/440 left | one trip | main reaches 0 at the 8th decrement; "Ship constructed" in the tick the countdown is 0 (after the 56th decrement) |
| T24 | deer waiting with count 5 | updates | waits 6 updates, then choosing with ticket -1; with `top` 0 and fewer than 2 starts this tick it chooses in its 2nd choosing update |
| T25 | `top` 0; animals A, B, C enter choosing in the same tick (list order) | next two ticks | tick 1: all go from -1 to 0; tick 2: A and B choose (`started` 2), C's ticket becomes 1; tick 3: C chooses via `top < q` (`top` = 1) |
| T26 | deer (md 2, range 6) at home, every field acceptable except home itself (5.10a: the choosing animal is registered there; corrected 2026-11-13, Q-233), draw gives N = 8 | target | the 8th acceptable field of the breadth-first order from home: after the 6 neighbours (1st to 6th) and the first field of ring 2 (7th), the **second field of ring 2** in visiting order |
| T27 | deer 9 fields from home (range 6) | choosing | the search is centred on its home field (5.10: the current field is farther than the range from home); the bound stays within 6 of home |
| T28 | rabbit walks for 40 ticks | arrival | wait about 120 updates (trunc(4.0 x 3.0 x 10), 119 if the float sum of the times falls short) |
| T29 | fisher, spot chosen, hut's fish field has 1 unit | end of fishing | the field loses its good, the hut's fish field is cleared, the fisher still takes a fish home (4.5, 4.6) |
| T30 | farmer's rest ends, no ripe field, no free spot ever | behaviour | sowing step waits 50 ticks, searches, (search finishes with none) ends; decision again in the next tick (3.2a, corrected 2026-10-27 b, T49); repeat: one search per 52 ticks, he never leaves. A search that cannot finish (its flood runs empty before 11 candidates) instead waits 50 more ticks without a decision: one search per 51 ticks |
| T31 | MP_2P_Storm_Coast, the map's 261 animal records (5.13), rules 5.8 and 5.9 | number of animals that choose (draw N) per counter tick 0 .. 40 | 0, 0, 2, 1, 1, 3, 2, 2, 2, 2, 3, 2, 2, 1, 3, 2, 3, 5, 2, 3, 0, 3, 5, 3, 0, 1, 4, 3, 6, 0, 2, 3, 3, 0, 0, 3, 2, 2, 5, 2, 0; the original's total draws in the same ticks: 0, 0, 3, 1, 1, 7, 6, 8, 7, 4, 5, 8, 8, 3, 9, 7, 12, 17, 8, 11, 0, 11, 19, 6, 0, 4, 12, 6, 19, 0, 4, 11, 11, 0, 0, 6, 10, 8, 14, 6, 0 (SIM-RNG-2.5); the difference is the walks' draws (5.14), 0 where no animal chose |
| T32 | counter tick 2 of T31 | the two animals that choose (object numbers 89933 and 90042 in the map) | 2 draws for N, then 1 walk draw in all: one of the two targets is a neighbour field |
| T33 | counter ticks 20, 24, 29, 33, 34, 40 of T31 | draws | none |
| T34 | a respawn check that places a deer | draws | 2: the kind, then the wait in 0 .. 132 |
| T35 | 5.11b: an animal (idle factor 3.0) that chose in trace tick s and whose last step completed in trace tick e | wait W | (s, e): W = (13, 28): 44; (11, 26): 44; (21, 36): 44; (4, 19): 45; (2, 17): 45; (5, 20): 45; (4, 27): 68; (2, 25): 69; (2, 26): 72; (2, 27): 74; (31, 54): 68; (45, 60): 44; (45, 90): 134; (28, 95): 200 (the clock values of trace ticks s - 1 and e - 1; 3 x (e - s) would give 45, 45, 45, 45, 45, 45, 69, 69, 72, 75, 69, 45, 135, 201). Measured: the animal of record 110 of E-trace-002, choosing in 45 and arriving in 90, re-chose in trace tick 227 (W = 134); record 90, choosing in 28 and arriving in 95, re-chose in 298 (E-trace-004) and 301 (E-trace-002) (W = 200) |
| T36 | 5.11b: the clock the animals read in trace tick T (the recorded `next` column of trace tick T - 1) | float32 value | T = 2: 0.1; 8: 0.70000005; 11: 1.0000001; 69: 6.799996; 73: 7.1999955; 74: 7.2999954 |
| T37 | 5.10: a deer at its home (48, 60) (even row), N = 1, all neighbours free and acceptable | target | the first neighbour in ring order, direction 5: (48, 61) |
| T38 | 5.10: an animal 3 fields from its home (range 6) | centre of the search; bound | the current field; fields within 6 of the home field |
| T40 | 5.15: E-trace-004, the respawn's checks | ticks, cells, placements | checks every 25th trace tick from 25 on, cells (10, 9), (0, 10), (1, 10), ..., (10, 10), (0, 0), (1, 0), ...; the first 16 place nothing (wood 0, 0, ..., 2, 1, 10 trees); the 17th, trace tick 425, cell (4, 0) with 31 trees, 1 home: the sub-cell (16, 3) with 13 trees, the flood from (64, 12) finds (65, 14); the kind draw mod 3 = 1 places a deer, the wait draw mod 133 = 25; the deer chooses first in trace tick 453. The placements of the first 1000 ticks: 425, 475 (cell (6, 0), sub-cell (25, 3), field (100, 13)), 625 (cell (1, 1), (6, 5), (25, 22)), 800 (cell (8, 1), (35, 5), (140, 20)), 900 (cell (1, 2), (5, 9), (21, 38)): the count 261 -> 266 of E-trace-002 |
| T41 | 5.15 with 5.11b, the one-player game | the generator | equal to the original's (E-trace-004) through trace tick 4737, 21 placements by then |
| T42 | 5.8 / 5.9 in E-trace-005 (two-player game), the animals' states at the end of trace tick 546 (counter 545): choosing 69 (ticket 1), 234 (0), 246 (0), 96 (-1), 164 (-1); 156, 165, 113, 226 waiting with 0, 0, 1, 1 left; `top` 0 | states at the ends of trace ticks 547 .. 551 | 547: 69 starts (rule 2, top 1), 96 and 164 to 0, 234 and 246 to 1, 156 and 165 enter with -1; 548: 234 (rule 3, top 0, started 1) and 246 (rule 2, top 1) start, 96 and 164 to 1, 156 and 165 to 0, 113 and 226 enter; 549: 96 (rule 3) and 156 (rule 3, started 2) start, 164 to 2 (rule 1), 165 to 1, 113 and 226 to 0; 550: 113 and 164 start, 165 to 2, 226 to 1; 551: 165 and 216 start, 226 to 2; 552: 108, 226, 242 start (`animals.csv.gz`, ids 89914 + the number) |
| T39 | 5.10a: a rabbit that chose at trace tick 13 at (86, 140) and reached (89, 143) at trace tick 88 (E-trace-002) | the field it is registered on at trace tick 109 | (89, 143) by 5.10a, yet the original's generator at trace tick 109 is matched only with (89, 143) free (Q-326) |
| T43 | 5.11a (a): MP_2P_Storm_Coast two-player game (E-trace-008), counter 1030 (trace tick 1031): rabbit 90092 walks from (144, 57) to (140, 63); the computer player's site 90269 stands on (142, 56) (placed at counter 818), its flag on (142, 55); heights as in the original at counter 1023 (heap dump) | labels and route | label(W) 26; label(143, 57) 21, label(143, 58) 18, label(142, 57) 28 (18 without the extra on the field stepped from: (142, 57) is a neighbour of the site); at (143, 57) the kept direction 3 is not allowed, only 4, so a draw picks 4; route (143, 57) (143, 58) (142, 59) (141, 59) (141, 60) (140, 61) (141, 62) (140, 63), the steps ending in trace ticks 1045, 1059, 1073, 1088, 1102, 1116, 1131, 1145 as recorded. With the extra on the field stepped to, (142, 57) and (143, 58) would tie at 18 and the rabbit would keep west to (142, 57), with two draws fewer in its read-out |
| T44 | 5.11a (a): flat ground, a building on (10, 10), an animal that avoids buildings walks from (8, 9) to (8, 11) | labels and route | label(9, 10) 13 (it neighbours the building), label(8, 10) 3, label(8, 9) 6; the only route is (8, 9) (8, 10) (8, 11) for every starting draw. (With the extra on the field stepped to, both middle fields would have 3, label(8, 9) would be 16 and the route through (9, 10) would be possible) |
| T45 | 5.16: E-trace-010 (two-player game, computer player Weak), the animal 90072 (kind 4a9bef83, home (138, 54)) waiting on (136, 59); the computer player places its farm (large) on (136, 58) at counter 3724 | the animal | removed in counter 3724: the recorded animal list holds 275 animals in 3723 and 274 in 3724, without 90072 ((136, 59) is the farm's north-east footprint field). An experiment build that keeps it draws for it at trace tick 4237 (15 draws in that tick instead of 11); removing it moves the generator's first difference from 4237 to 4537 |
| T46 | 5.7, 5.11: E-trace-010, the animal 90061 (kind 4a9bef83, home (142, 42)) on its walk (142, 39) -> (142, 40), started at game time 474.71976 s, reserved by the hunter's hut 90244 during that walk; the hut is torn down at counter 4791 | the animal | 4791: walking again (progress 0.98); 4792: on (142, 40), waiting 135 (5.11 from the walk's start 474.71976 s, idle factor 3) (measured). An experiment build that sets a released animal to waiting 0 where it stands leaves it on (142, 39) and its draws differ from trace tick 4797 (7 there against the original's 8); keeping the walk matches, generator 4797 -> 5041 |
| T47 | 5.7: E-trace-011, the animal 90091 (kind 41797b76, home (143, 57)) arriving on (145, 55) at counter 7689 with wait 557, reserved by player 1's hunter's hut 90625 (144, 58) at 7701 with 546 left; the hut torn down at counter 7948 (its plank 90927 on the flag, remains 90928) | the animal's rows 7700 .. 7960 | hunted from 7701 to 7947 with the counter at 546 throughout; 7948: waiting, 546; 7949: 545; then one less each tick (534 at 7960), no choice and no draw (measured). Releasing it with wait 0 makes it choose at 7950 and draw from trace tick 7955 (the original then 1 draw against 8) |
| T48 | 5.10: E-trace-011, the animal 90048 (kind 41797b76, home (144, 37), range 8, move distance 4) choosing on (142, 40) at counter 9095; player 1's flag 90721 on (141, 45) (roads (145, 46) .. (141, 45) and (140, 46) .. (141, 45)) | its target | N = 58; the flood from (142, 40) passes (141, 45), not acceptable for the flag on it; the 58th acceptable field is (141, 35): the walk (142, 40) -> (142, 39) -> (142, 38) -> (141, 37) -> (141, 36) -> (141, 35), the animal on (141, 36) at counter 9154 (measured). Accepting the flag field gives (142, 35) and the walk through (142, 36), and the draws differ from trace tick 9156 (with the rule 9956) |
| T49 | 3.2a: E-trace-011, player 1's farm 90751 (139, 44), farmer 91200; in from his last sowing at trace tick 16354, the rest over at 16410; no free spot within the radius, grain field ripe and reserved by the farm's search between 16462 and 16514 | the farmer's decisions | trace ticks: 16410 decision (sowing), sowing wait 34 from 16411, its search fails at 16462 (the decision state shown in 16462), sowing again from 16463, search fails at 16514, harvest decided at 16515 (state 33), walks out at 16566 (16515 + 51). Deciding in the failed search's own tick (the old rule, an experiment build) gives the cycle 51 instead of 52 ticks: harvest at 16513, out at 16564 (settlers first differ at 16564); with the rule at 16637, generator 16610 -> 18121, buildings 17537 -> 17685 |
| T50 | 5.2 (the exploration test), `AI-construction.md` AI-CON-6.3: E-trace-014 (R4, the Weak game every tick from 20 000), player 1's hunter's hut order for cell (8, 1), its site search at counters 21170 .. 21175 | the site chosen | animal 90031 on (127, 31) (distance 7 from (134, 31), its walk to (128, 33) under way, step to (128, 32)); player 1 has explored (128, 33) but not (127, 31) (heap dumps 21 000 and 21 500: exploration map); the original's search counts it and places the hut on (134, 31) at 21179 (route 2). Our build tested the animal's own field, found no animal for (134, 31) and placed it on (136, 31). Experiment build (explored test at the walk's goal): the hut on (134, 31); the Weak game against R4 equal to settlers 22214, buildings 22730, generator 22219 (was 21180 / 21179 / 21183), the 150 000 pair's generator 21183 -> 22219, the other pairs unchanged |
| T51 | 4.1a: E-trace-014 (R4, the Weak game every tick from 20 000), player 1's fisher 92341 for the hut on (145, 57) | his rows | arrives (stands on the hut's field) in trace tick 25147; start (the trip's state, the spot draw) in 25148; first step of the walk out in 25149; at the spot (147, 52) in 25428. Second trip: in and resting from 26192, start in 27305, first step in 27306. Ordering the walk in the start tick (an experiment build without the rule) begins it in 25148; with the rule the Weak game against R4 is equal to settlers 25427 (was 25149), generator 25747 (was 25204; the 150 000 pair's too), buildings 26278 unchanged, the other pairs unchanged |
| T52 | 4.6a: E-trace-014 (R4), player 1's fisher 92341 (hut (145, 57)) | his rows after each catch | trip 1: the take (putting down) from trace tick 25761, in his hand from 25765, standing in 25769, first step home in 25770, at the flag in 26160; trip 2: 27960, 27964, 27968, 27969; trip 3: 30140, 30144, 30148, 30149 (measured). An experiment build that walks back in the tick the take ends (25769) moves the walk's draws a tick: the Weak game against R4 differs from settlers 25770, generator 25769; with the rule it is equal to settlers 25791, generator 25894 (the 150 000 pair's too), buildings 26278, the other pairs unchanged |
| T53 | 4.6 (the trip after a rest): E-trace-014 (R4, the Weak game), player 1's fisherman 92341 of the hut on (145, 57); back in the hut at trace tick 26192 (standing in it), rest | his next trip | the trip starts in trace tick 27305 (job state 36 -> 33; its draw falls in that tick: five generator draws in 27305, none in 27306), the walk out is ordered in 27306 at progress 0 and he moves from 27307 (recorded). A tick between the rest and the start (the start in 27306) moves the draw to 27306 and the walk a tick later: the Weak game against R4 then differs from generator 27305, settlers 27307; without that tick (experiment build, with ECO-DIST T22, ECO-REQ-5.4a, 5.3d T34, AI-CON-3.3 T77, TRN-CARRY-3.6 T44) settlers 29151, buildings 30330, generator 29835 (the 150 000 pair's too), the other pairs unchanged |
| T54 | 7.3 (start and period): E-trace-014 (R4, the Weak game), player 1's shipyard 92849 on (144, 41) in ship mode, finished at counter 33646, switched off by the computer player at 34317 and on again at 39028 (buildings table, column enabled); its shipwright 93488 inside from 34415 | the shipwright's states | in his mode check from 34415 to 39028; the ship step from 39029; no ship at the checks of 39130 and 39232 (the mode check for one tick each, the step again from 39131 and 39233); at the check of 39334 he has a ship and walks out (walk at progress 0 in counter 39334). Checks every 101 ticks from his arrival, whatever the switch (our build), took him out at 39264: the Weak pair's settlers differed from 39270; with the rule (experiment build) his ticks equal the recording's, settlers 39510, generator 39610 (was 39290), the 150 000 generator 39597 (was 39288); the buildings column, equal to 40 000 before, now differs at 39730, after the settlers' point; the Hard pairs unchanged |
| T55 | 7.4, 7.4a (a work trip at the ship): E-trace-014 (R4, the Weak game), the shipwright 93488 of the shipyard (144, 41) on his first trip to the new ship 94308 (building field (145, 46), water field (146, 46), started at counter 39334) | his ticks at the ship and the ship's frame work | standing on (145, 46) from counter 39535; the drop animation 39535 .. 39538, the pick-up 39539 .. 39542, standing 39543; the work animation from 39544 with the frame work 1 - 1/220 at 39544 and 1 - 56/220 at 39599 (ships table); state 37 from 39599, the walk back at progress 0 in 39600, 0.045 in 39601. Going to work in the arrival tick, without the put-down (our build), ends the trip nine ticks early: the Weak pair's settlers differ from 39600 (with the ship site of 7.3b in place, our experiment build); with the timing: settlers 39830, buildings and generator equal to 40 000, the 150 000 generator 40959; the Hard pairs unchanged |
| T56 | 7.4, SIM-START-4.2 (the carried plank's number): as T55 | the objects made in counter 39334 and the plank's end | the ship 94308, then the ware 94309 (plank, carried by the shipwright 93488) in counter 39334; 94309 gone in 39539 (objects table). Without the ware's number (our build) every later number is one lower: the donkey 94312 of counter 39355 gets 94311 and the Weak pair's settlers differ from 39830 (with 7.3b and T55 in our experiment build); with the number the Weak pair (R4, to 40 000) is equal in settlers, buildings and generator; the 150 000 generator stays at 40959; the Hard pairs unchanged |
| T57 | 7.3b (the flood's order): E-trace-014 (R4, the Weak game), shipyard 92849 on (144, 41), radius 8; Coast fields (145, 45) (state Coast only, continent 12, Water neighbours (146, 45) and (146, 46); its only walkable neighbours (144, 45) and (145, 46)) and (145, 46) (Coast, Street, building and harbour bits, Water neighbour (146, 46)) | the ship started at counter 39334 | ship 94308 with building field (145, 46) and water field (146, 46) (ships table); the shipwright walks to (145, 46) (T55). The flood in MAP-GRID-3.2 ring order gives (145, 46); in MAP-GRID direction order 0 .. 5 it gives (145, 45) (both in the experiment build) |
| T58 | 6.2 (the breeder's next cycle): E-trace-013 (R3, the Hard game), player 1's donkey breeding 91283 on (131, 59), breeder 92052, corn and water at hand | his rows and the donkeys | work (WorkLoop0, 333 ticks) from counter 32135 to 32467; 32468 standing (job state 36); 32469 waiting (state 33), donkey 94482 appears on (131, 60) walking to (131, 59) from progress 0; 32470 the next work begins (333 ticks left). The cycle before: standing 32133, donkey 94446 and waiting 32134, work from 32135. The Weak game (E-trace-014, breeder 93053 of 92529): work to 34832, standing 34833, donkey 93642 and waiting 34834, then waiting for inputs. Starting the next work in the donkey's tick makes every later donkey of a busy breeding one tick earlier: the Hard pair's settlers first differ at 32470 (donkey 94482 a tick ahead); with the rule (experiment build) at 34310 (buildings 33340 -> 33630, generator 32824 -> 34133; every other pair unchanged) |
| T59 | 5.2 (several animals on one field): E-trace-013 (R3, the Hard game), the hunter's hut (120, 61) and the field (115, 55) | the animal reserved in the search of counter 43206 | 90076 (home (115, 56)) registered on (115, 55) at counter 43155, 90073 (home (118, 54)) at 43186; the hut reserves 90076 (its state changes at 43206, its wait frozen at 217) and its hunter shoots it from trace tick 43439. Taking the lower number (90073) leaves 90076 unreserved, so its wait runs out and it chooses a new walk at counter 43425, which the original's reserved 90076 never does (5 draws in that tick instead of 1: the Hard game vs R3 differs at generator 43426). With the rule (experiment build, with TRN-CARRY T56, AI-CON T97, ECO-OUT T27): Hard settlers 43500 -> 44400, generator 43426 -> 44612 (buildings 45400 -> 45100, after the new generator divergence); every other pair unchanged |
| T60 | 7.4b (the ship finished partway through the last trip), `SHIP-ships-expeditions.md` SHIP-2.1, 2.4: E-trace-013 (R3, the Hard game), player 1's ship 94738 on (146, 46) (`ships.csv`, the main work in column build_a) | the ship's work and its search | main work 0.0954554 (42/440) from counter 45800 to 45824; one decrement of 1/440 per tick from 45825 (0.0931827 ...); 8.75e-07 at 45866, 0 at 45867; the search counter starts at 45867 (1 at 45867, 101 at 45967), so the first search try is in counter 45968: it reserves a docking place of the harbour 90366 and sails, its route's read-out drawing from the generator (12 draws in 45968 against 9 without it), and docks on (146, 50) at 45991 (recorded). Starting the ship at the end of the shipwright's step (counter 45881, the trips counted whole) puts the first try at 45983: the Hard game against R3 differs from generator 45969 (experiment build with T59, T100, ECO-REQ T41). With the rule and the ship's route of `SHIP-ships-expeditions.md` SHIP-3.1 (T13; experiment build): the Hard 150 000 pair's generator 45969 -> 46754, settlers 46100 -> 46600; the Weak 150 000 pair's generator 50113 -> 50934 (the same case with ship 94308, its work 0 at counter 50011); every other pair unchanged |
| T61 | 7.3b (a launched ship frees its water field): E-trace-018 (R5, the Weak game), player 1's shipyard 92849 (144, 41); its first ship 94308 was built on (145, 46) with the water field (146, 46), launched at counter 50011 and docked on (146, 50) since 50112 | the next ship | ship 96085 started at counter 50667 on (145, 46), water field (146, 46) again (`ships.csv`); the shipwright 93488 walks to (145, 46) by (144, 46) and stands there from trace 50869 (recorded). Keeping the launched ship's water field reserved starts the second ship on another field: he walks by (144, 45) to (145, 45), and the Weak game against R5 differs from settlers 50825; with the rule (experiment build, with T60 and SHIP T13) from 50882 (with `ECO-distribution.md` T23 the Weak 150 000 generator 50934 -> 51472; every other pair unchanged) |
| T62 | 5.7 (released while standing): a deer reserved by a hunter's hut on its walk, standing at the walk's end; the hut is removed in tick R | the deer's state per tick | R: waiting, wait 0 (in the removal's tick); R + 1: choosing, ticket -1; R + 2: ticket 0 (rule 4 with top 0, or rule 1); from R + 3 at the earliest: it chooses and walks (one draw for N and the walk's draws); with no other animal choosing, R + 3 exactly (R6: R = 6631, walk to (57, 18) at 6634) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-009 | code reading | `S2DNG.exe` | worker cycle by building type |
| E-eco-009 | code reading | | per-tick building searches: deposit target, hunter, mine |
| E-eco-009 | code reading | | mine ore search (radius, goal) and dig |
| E-eco-009 | code reading | | farm choice, harvest and sowing objects, work end per profession |
| E-eco-009 | code reading | | fisherman: retry grid, spot search, fish field, catch |
| E-eco-009 | code reading | | hunter: animal search, trip, kill |
| E-eco-009 | code reading | | take step: always the building's next product |
| E-eco-009 | code reading | | donkey farm: produce cycle, donkey carrier created at the door |
| E-eco-012 | code reading | | new carrier flag set at creation -> donkey branch: walk out to the flag (as its job), then go home (state 3) |
| E-eco-009 | code reading | | shipyard: mode, ship check, trip, work step |
| E-eco-009 | code reading | the worker's creation | which step objects a worker has |
| E-eco-013 | code reading | | ship check: existing unlinked ship, site flood and goal, ship creation, link |
| E-eco-013 | code reading | | ship trip states, work step, decrement, unlink, "Ship constructed" |
| E-ship-009 | code reading | (session 2026-11-16 day) | 7.4b: the "Ship constructed" message is made without the ship's name; the name is added only by "Ship waiting for Orders" (Q-1065) |
| E-eco-013 | code reading | | deposit lifetime count, removal counter, deletion at 20 |
| E-eco-013 | code reading | | hunter: separate worker counter, unreserved animal, switch check, animal update states |
| E-eco-013 | code reading | building creation | mine counter starts at 0 at creation |
| data | script | `buildings.lua`, `deposits.lua`, `ship.lua`, `config.lua`, `Jobs.lua` | radii, times, grain field, ship costs, fishing distance, ids |
| E-eco-014 | code reading | `S2DNG.exe` | animal states, global start counter reset per tick, ticket rules, idle wait |
| E-eco-014 | code reading | | target draw, search, range and field tests |
| E-eco-014 | code reading | | animal kind defaults and setters (idle factor at least 2.0) |
| E-pt2-001 | code reading | | farmer decision timing, fisher spot and catch, hunter ham |
| E-trace-002 | live read of the running game | `generator.csv` (draws per tick); `raw/` reads of the resources object (animal count per 1000 ticks) | draw pattern from the map's counters; animal counts |
| E-map-001 | data file | `MP_2P_Storm_Coast.s2m` animal records | 261 animals, state 0, counters 0 .. 169 |
| E-sim-002 | code reading | E-trace-002 `next` column (float32 clock) | 5.9 order, 5.10, 5.10a, 5.11, 5.11a, 5.11b, 5.12, 5.15 |
| E-sim-001 | code reading | | animals' draws and start state |
| E-eco-018 | code reading and recording | E-trace-010 `animals.csv` counters 3723, 3724 |
| E-eco-032, E-trace-014 | code reading, memory reads of the running game, recording | the players' exploration maps in E-trace-014's dumps; animals table | ECO-FOOD-5.2 (T50), `AI-construction.md` AI-CON-6.3 |
| E-eco-034, E-trace-014 | code reading, recording | E-trace-014 `settlers.csv` fisher 92341, trace ticks 25147 .. 27306 | ECO-FOOD-4.1a (T51) |
| E-eco-045, E-trace-019 | recording, experiment build | E-trace-019 `animals.csv` deer 13740 (states 4, 0, 1, 1, 2 at counters 6630 .. 6634) and `buildings.csv` hunter's hut 14615 gone at 6631; an experiment build with the release setting the wait at once: the fight game's generator equal to 6714 (was 6635), Weak, one-player and Hard games' first differences unchanged | 5.7 released while standing, T62 |
| E-sim-012 | code reading and replay | a check over E-trace-007's heap dump of counter 1023 (heights) and s2-reborn's field states; E-trace-008 `animals.csv.gz` (rabbit 90092) | 5.11a (a), T43, T44 |
| E-trace-005 | live read of the running game | `animals.csv.gz`: every animal's state, ticket / wait counter, field, step end, goal, registered field and walk-start clock in every trace tick 0 .. 6000 | ECO-FOOD-5.8, 5.9 (T42), 5.10a; Q-334, Q-337 |

## Open questions

- The launch of a finished ship (plan 4.7).
- The exact stop point of the fisherman's,
  hunter's and shipwright's walks.
- In-game check of ECO-FOOD-4.6 and 5.4 (product made without a catch).
