# SHIP: Ships, harbours and expeditions

Status: draft (read from the binary; no in-game measurement)
Owner question(s): plan step 4.7; Q-062 (the command `prepare-expedition`); new Q-063 to Q-066
Game build: 11757

## Summary

A harbour is a warehouse on the coast with docking places on the water around
it. Ships are built by the shipyard (`ECO-food-mining.md` ECO-FOOD-7) and, once
finished, sail by themselves to the nearest harbour of their player. Harbours
of one player on the same sea exchange wares and settlers by ship: a harbour
that needs a good asks another harbour that can supply it, that harbour
gathers the goods, calls a ship, loads it and sends it off. An expedition is
prepared in a harbour (4 planks, 6 stones, a builder), loaded onto a free
ship, and steered by the player towards one of up to six landing places, one
per compass direction. At a landing place the player can found a new harbour,
which claims land at once.

## Rules

### SHIP-1 Harbour sites and docking places

- **SHIP-1.1 Site.** A harbour can only be placed where the building help
  says "harbour" (`ECO-building-placement.md` ECO-PLACE-3.8: harbour
  pavement, at least 3 Water fields among the 18 fields at distance exactly 3).
  Maps mark candidate landing/harbour spots per land continent
  (`FMT-s2m-map.md` FMT-S2M-4.9, the per-continent field list); these are the
  spots an expedition can land at (SHIP-6.2). Confidence: high.
- **SHIP-1.2 Docking places.** Once, when the harbour starts working, its
  docking places are made: the 18 fields at distance exactly 3 from the
  harbour's field are visited in ring order (start 3 steps in direction 4,
  then 3 steps in each of the directions 0 to 5, MAP-GRID-3). A field becomes
  a docking place when it is inside the map, is Water and not DoodadBlocked,
  does not already carry the **LandingPosition** state (FMT-S2M-4.4 bit 11),
  and **none of its six neighbours** carries it. Making a docking place sets
  LandingPosition on that field. So docking places of all harbours never
  touch each other. Confidence: high for the ring and the tests; medium for
  "sets the bit" (a field-state setter is called with the field).
- **SHIP-1.3** Each docking place holds at most one ship reserved for it and
  one ship docked at it. A harbour is **reachable by sea** from a field F when
  it has a docking place that is neither reserved nor occupied and lies on the
  same sea (water continent) as F. Confidence: high.
- **SHIP-1.4** A harbour is also a warehouse (headquarters-like stock,
  `bp_setDepot`, `bp_setSpawn`: it makes helpers like a warehouse,
  ECO-PROF) and is built like any large building (frame 4 planks, main 6
  stones). Confidence: high (script values).

### SHIP-2 The ship and its states

- **SHIP-2.1 Launch.** A ship under construction does nothing. In the first
  tick in which both its frame and main work are 0 (ECO-FOOD-7.4b) it starts
  its state machine at its water field in state **Idle**. There is no
  separate launch step. Confidence: high.
- **SHIP-2.2 States** (updated every tick, in the navy update): Idle,
  "search", "to harbour", "docked", "waiting", "sailing", "unloading",
  "making room", "returning". Confidence: high (the state list of the game's
  debug names).
- **SHIP-2.3 Idle.** In order (confidence medium for the order of the middle
  cases):
  1. cargo aboard -> **unloading** (SHIP-4.6);
  2. expedition active and no target -> message **"Ship waiting for
     Orders"** (type 4, at the ship) and **waiting** (SHIP-6);
  3. the ship has been called to a harbour (to load, SHIP-4.4, or for an
     expedition, SHIP-5.3) -> it reserves a docking place there and goes
     **to harbour**, if that harbour is reachable by sea (SHIP-1.3);
  4. expedition goods aboard (a cancelled expedition, SHIP-6.5) while docked
     -> they are put into the harbour's stock;
  5. otherwise -> **search**.
  **Correction to 4 and 5 (2026-11-21 eve, Q-1074, E-ship-017: code reading and the Hard recording;
  high):** a ship that still has its harbour link (it lies docked at that harbour) **stays idle**: in
  its update it only does 4 (expedition goods aboard into the harbour's stock) and nothing else; it
  does not search. Only an idle ship **without** a harbour link turns to search, and it keeps the
  counter it holds (SHIP-2.4). So a ship that has docked and has nothing to do lies idle at its
  harbour until a call (3), cargo (1), an expedition (2) or a push-out (SHIP-2.4a) moves it
  (recorded: the Hard game's ship 94738 docked at counter 45991, idle 45992 .. 48457 with counter 0;
  ours searched again from 45993 and docked again every 104 ticks). Test T25.
  **The order, read whole (2026-11-26 a, Q-065 answered; E-ship-027, code reading; high):** an idle
  ship's update tests, in this order:
  (1) cargo aboard: unloading;
  (2) it holds an **expedition-call link** (SHIP-5.3): if it does not lie docked at that harbour, it
  reserves a place there and goes to harbour; if it lies docked there, it stays;
  (3) it is on an expedition: its landing options, the message "Ship waiting for Orders", waiting;
  (4) it holds a **transfer-call link** (SHIP-4.4): the same as (2) for that harbour;
  (5) it lies **docked**: expedition goods aboard go into that harbour's stock, and it stays;
  (6) otherwise: search.
  The ship's four links are named in SHIP-4.4 "which link, exactly": the reserved docking place,
  the harbour it is docked at, the transfer call and the expedition call. A call sets only its
  link (and the harbour's side); the ship acts on it here, in its own update.
- **SHIP-2.4 Search.** A counter runs from 0 to 100 (one try every **101
  ticks**). At a try: any harbour link is dropped; the nearest **finished**
  harbour of the ship's player that is reachable by sea from the ship's field
  (straight-line distance, strictly smaller wins, so the first in the harbour
  list on ties) gets a docking place reserved and the ship goes **to
  harbour**. If there is none and the ship stands on a LandingPosition field,
  it goes **making room** (SHIP-3.3); otherwise it stays searching. A ship
  coming back from a cancelled expedition starts with the counter at 100, so
  it tries in its next update. Confidence: high. **Corrected for the returning ship
  (2026-11-21 day, Q-1073; E-ship-012: the Hard recording and an experiment build; high for the
  measured case):** a returning ship that has stopped (SHIP-6.5) and turns to searching makes its
  first try in its **second** update after the one in which it turned, not in the next: the
  recorded counter shows 100 in the turning update, 101 in the next, and the try (with its route's
  draws) comes in the one after. Every one of the recording's 35 tries came in the update after a
  counter of 101. How the counter's other starts map onto this (the recording's searches from
  idle show 90 at their start and also try one update after 101; ours' search from idle counts
  from 0) is Q-1074. Test T21. **The counter (answered from the code 2026-11-21 pm, Q-1074,
  E-ship-013):** in each update of a searching ship, a counter below 101 is raised by one and
  nothing else happens; an update that finds it at 101 or more makes the try, and the try sets it
  to 0 whatever its result (so a ship that keeps finding no harbour tries every **102** updates).
  The counter is set to 100 only when a returning ship turns to searching, and to 0 when a ship is
  made. **Turning to searching from idle (or docked) does not set it**: the search goes on from
  the value it holds (0 after any try; a ship that left a search without a try keeps its count).
  A ship still being built makes no update at all, so its counter stays 0 (the recording's new
  ships showing 0 for thousands of ticks). Confidence: high (code). Still open (Q-1074): why the
  recording's searches from idle show 90 at their start; the code holds no 90, so a value left
  from an earlier state of those ships is likely, not traced.
  **The counter as an implementation sees it (2026-11-22 day, the implementation's measurement of
  2026-11-21 eve and an experiment build; high for the timings, which are what must be met):** the
  recorded counter already counts in the tick a ship is finished (1 in that tick), so an
  implementation whose ship spends its first update turning from idle to searching without counting
  holds the recorded value less one at every update. What must come out are the tries' updates:
  (i) a new ship tries in the 102nd update from its finishing tick (T13); (ii) a ship turned from
  returning tries in its second update after the turn (T21); (iii) after a try that finds no harbour
  the next try comes 102 updates later; (iv) a pushed-out ship (SHIP-2.4a) tries in its twelfth
  update after the one that turns it to searching. In such a counter (try at 100, the period 101 for
  counting from 0) that is: a returning ship starts at 99, a pushed-out one at 89, and a try that
  finds nothing leaves the counter one below 0 (its next update only brings it to 0). Moving the
  try to "past 100" in such a counter delays (i) by one update and puts the Hard and Weak pairs'
  generators apart at once (the recordings' first tries, trace ticks 45 969 and 50 113): the
  implementation's measurement of the 102 read literally. (iii) and (iv) in these terms move no
  compared pair's first difference or matching ticks (experiment build with every rule of
  2026-11-21, against the push-out at 90 and a try leaving 0).
- **SHIP-2.4a Pushed out (2026-11-21 eve, Q-1074 answered, E-ship-017: code reading and the Hard
  recording; high for the rule, medium for the cases it applies in).** When a ship reserves a
  docking place at a harbour for a call (SHIP-2.3 3) or to unload, and the harbour has no free
  docking place on its sea, the harbour looks at its docking places in their order for a ship
  lying there that is finished, idle, empty (no cargo, no expedition goods, no expedition, no
  target, not called) and still linked to it. The first such ship is pushed out: its counter is
  set to **90**, its harbour link and berth are dropped. The calling ship then reserves the freed
  place. The pushed-out ship, idle without a link, turns to search in its next update with 90 and
  tries in its twelfth searching update (91 .. 101, then the try). This is the recorded "90 at the
  start of a search from idle": all 19 of the Hard recording's idle-to-search turns show 90, each
  after a stretch idle at a harbour (ship 100861: docked at 87007, idle 87011 .. 87118, searching
  from 87119 with 90, 91, 92, ...). A harbour search of SHIP-2.4 takes only harbours with a free
  place, so a search itself never pushes a ship out. Test T25. The ship sails to the docking place's water field.
  When it has stopped: if it still has its harbour link it is docked there
  (the harbour records it at that docking place), its last port position
  becomes the **harbour's own field** (the building's land field, not the
  docking place's water field; corrected 2026-11-16 night, Q-1060, SHIP-2.7),
  state **docked**; if the link is gone, **Idle**. Confidence: high.
- **SHIP-2.6 Docked** lasts one update: Idle again if it still has its
  harbour link, else search. Confidence: high.
- **SHIP-2.7 Last port position (added 2026-11-16 night, Q-1060; E-ship-005,
  code reading).** Every ship keeps one field, its last port position, or
  none. It is used only by SHIP-6.1 (which spots count as its own harbour)
  and SHIP-7.1 (where a harbour can be founded). It changes at exactly these
  moments:
  - **a ship the shipyard builds** gets, at its creation, the coast field the
    shipwright works from (ECO-FOOD-7);
  - **a ship a mission's script makes** (`createShip`, `CAM-campaign-missions.md`
    CAM-1.2) gets, at its creation, its **first land neighbour** (below);
  - **every route start** clears it: a ship that starts sailing to a docking
    place (SHIP-2.3 case 3, SHIP-2.4), to a landing field (SHIP-6.3) or to
    make room (SHIP-3.3) has none until it stops; a route whose target is
    the ship's own field also clears it;
  - **docking** (SHIP-2.5) sets the harbour building's own field;
  - **an expedition's arrival** (SHIP-6.4) sets the first land neighbour of
    the field it stopped on.
  **The first land neighbour** of a field F: the six neighbours of F are
  tested in the order of directions 4, 5, 0, 1, 2, 3 (start at the neighbour
  in direction 4, then step once in each of the directions 0 to 4; ring 1 of
  `MAP-grid-geometry.md` MAP-GRID-3 with its start field tested **first**);
  the first one that lies inside the map, is not Water and belongs to a land
  continent is the result. Nothing farther than ring 1 is looked at: when
  none of the six qualifies the ship has no last port position. Confidence:
  high.

### SHIP-3 Movement on water

- **SHIP-3.1** Ships use the settlers' step movement (`TRN-settler-movement.md`)
  with the ship speed `sh_setSpeed` 0.18 (progress per tick on level ground;
  about 0.56 s per field) and a route found by a search over **Water fields
  that are not DoodadBlocked**. A ship never leaves its sea. **The route**
  (added 2026-11-02 night, Q-064 partly answered, T13; E-ship-002): the free walk's search and read-out of `TRN-free-walk.md`
  TRN-FREE-2.3 to 3.2a, with the ship's own field test and step cost in place of
  the walker's: a field can be entered when it is Water and not DoodadBlocked
  (the ship's own field always); a step costs **1** when each of the six
  neighbours of the field stepped from lies inside the map (not on its border
  rows or columns) and is Water, not DoodadBlocked and not LandingPosition, and
  **3** otherwise (beside a coast, a harbour's landing fields or the map's
  edge). The read-out draws from the generator as the free walk's does (its
  first direction and the choices between equally low neighbours), so a ship's
  departure costs draws. Confidence: high for the cost, the field test and the
  draws (code and T13). **Other ships (2026-11-26 a, Q-064 answered; E-ship-028, code reading;
  high):** other ships never block anything. The route search's field test does not look at ships.
  The ship's step takes the next field of its route without asking whether a ship stands there.
  Entering a field only moves the ship from one field's object list to the next. Two ships may
  stand on one field, pass through each other and cross in a channel one field wide. Each arrives
  as if it were alone (no waiting, no new route, no draw). The one thing kept apart is a **docking
  place**: a ship reserves one before it sails there (SHIP-2.3 3, SHIP-2.4), and a harbour with
  none free pushes an idle ship out (SHIP-2.4a). Test T35.
- **SHIP-3.2** A ship stands on one field at a time and counts as being at
  its current field for all tests ("same sea", distances). Confidence: high.
- **SHIP-3.3 Making room.** Starting one step in direction 4 from the ship's
  field, the six fields of the ring of radius 1 are visited in ring order; a
  field qualifies if it is Water and not LandingPosition. With `k = draw mod
  6`, the ship takes the `(k + 1)`-th qualifying field in that order, or the
  last qualifying one if there are fewer. It sails there (no move if that is
  its own field), then search again. No qualifying field: it stays.
  Confidence: high for the rule; the random draw is the game's shared one
  (`ECO-outdoor-workers.md`).

- **SHIP-3.4 Exploring.** In every update in which a finished ship is moving
  and its field is not yet explored by its player, the ship explores
  **radius 3** around its field (as walking soldiers do with radius 2,
  `MIL-soldiers.md`). On arriving at an expedition target (SHIP-6.4) it
  explores **radius 6** around its field. Confidence: high for the radii;
  medium for "moving" (speed test).

### SHIP-4 Wares and settlers between harbours

All harbour work below runs in the harbour's own update, once every **101
ticks** (counter 0..100), after the expedition step (SHIP-5) which runs every
tick. Within the 101-tick pass the order is SHIP-4.2, 4.3, 4.4.
**The passes' ticks (2026-11-22 day, E-ship-018: the Hard recording; high):** a harbour's first
update is in the tick **after** the one in which it is finished, so its passes fall **101, 202,
303, ... ticks after its finishing tick** (recorded: harbour (144, 47) finished in counter 6734, its
loading pass in 54 507 = 6734 + 473 x 101; harbour (147, 136) finished in 54 100, its passes in
54 201, 54 302, 54 403, 54 504, each also the tick of its new helper, ECO-PROF-3). An
implementation that runs the new harbour's first update in its finishing tick has every pass one
tick early. Test T26.

- **SHIP-4.1 Needs.** A harbour keeps a list of goods its island needs from
  overseas. **How entries get there (2026-11-16 night, Q-063 partly answered;
  E-ship-006, code reading):** an open request is attached to one harbour when
  the land cannot serve it.
  - A building's **ware order** that finds no warehouse of its player holding
    the good and reachable over the roads (the distribution's warehouse search,
    `ECO-requests-dispatch.md`) is attached, if it is not attached yet.
  - A **settler request** that finds no warehouse able to send the settler (and
    none that can make one from a helper and a tool) is attached the same way.
    **A military building's soldier request too (2026-11-27, E-ship-030, test T36):** when no
    finished warehouse connected to the building's flag holds a soldier of any rank
    (`MIL-soldiers.md` MIL-SOL-4.7), the request is attached to a harbour in the same way, and
    the harbour's need is for the request's good (the private's). The harbour then asks
    overseas (4.2) and the supplying harbour gathers the soldier into itself (4.3). Confidence:
    high (measured, one case: the Weak game's colony).
  - **The harbour chosen** is among the player's finished harbours that are
    reachable over the roads from the requester's flag, accept the good (its
    stock settings, `ECO-warehouse-construction.md`), and have a docking place
    on a sea where the player has a finished ship. The one with the lowest
    road route cost from the requester's flag wins; on equal cost, the first in
    the world's harbour list. With no such harbour the request stays
    unattached and is tried again at its next turn.
    **"Reachable over the roads" is the ware network, for a settler request too (2026-11-22 pm,
    Q-1079; E-ship-019: code reading, the Hard recording's heap dumps and roads table, and an
    experiment build; high).** The harbour finder compares the requester's flag and the harbour's
    flag in the network made of the roads that have a carrier or donkey assigned
    (`TRN-carriers-transport.md` TRN-CARRY-5.1a). The road over which ours' settler walk would go
    is not enough. A road counts from the moment its carrier is linked to it: when he is sent
    out, while still on his way. The route cost compared is that network's. So a settler request
    whose site is joined to a new harbour only by a road still waiting for its carrier is not
    attached. It is attached on its first turn after that carrier is sent, and the harbour asks
    for it in its next pass. Test T27.
    **The cost is a stored cost, asked from the representative's flag (2026-11-21 eve, E-ship-016:
    code reading and an experiment build; medium-high).** For a ware order the route cost is the
    stored cost of the ware network (`TRN-carriers-transport.md` TRN-CARRY-5.3a), not a fresh
    search: an earlier ask of the pair gives its stored value, and this ask stores the cost of the
    roads as they are then (3L for a road whose carrier holds a ware, 4L towards a full flag) for
    every later user. It is asked from the flag of the matching's **representative order** of that
    good and network (ECO-REQ-5.3d), not from the chosen order's own flag (the chosen order is the
    one attached), for each harbour that passes the other tests (finished, its player's, reachable
    over the ware roads, accepting the good, a docking place on a sea with the player's ship), once
    per harbour, in the harbour list's order. The ask comes after the warehouse search found no
    warehouse with the good (ECO-REQ-5.3 (b)) and the order was chosen. Test T24.
  - **Attaching** adds the requester to the harbour's need entry for that good
    (the entry is made on its first requester). The need is the number of
    requesters attached. A settler request that later finds a warehouse over
    land is detached before it is served, which lowers the need.
  - Then SHIP-4.2 asks another harbour for the good.
  Confidence: medium (the attach paths and the choice read in the code; the
  exact turn at which a waiting request is tried again, and whether a ware
  order is detached the same way when its good turns up on land, were not
  followed). Test T19.
  **Retry and detach (2026-11-16 day, rest of Q-063; E-ship-007, code reading).**
  - **When:** the attach is a step of the ware order matching itself (`ECO-requests-dispatch.md`
    ECO-REQ-5.3, the pass every 11 ticks): an order the pass cannot serve from a warehouse over
    land is attached in that pass, and an unattached one is tried again at the next pass. A
    settler request is attached at its own turn in the settler queue (ECO-REQ-2.2) (high).
  - **An attached ware order is checked again at every pass** in which no warehouse over land
    can serve it: it stays attached while its harbour still accepts the good (its stock settings)
    and is still reachable over the roads from the requester's flag; otherwise it is **detached**
    in that pass, which lowers that harbour's need, and is free to be attached again (to the same
    or another harbour) at a later pass (high).
  - **A ware order served over land:** the branch that finds a warehouse with the good does not
    detach the order there; its harbour link ends with the order itself when the ware is
    assigned and the order is used up.
  **The order's end (2026-11-17 night, Q-063 end; E-ship-010, code reading, high).** An attached
  ware order counts as one requester in its harbour's need for that good from the attach until it
  is detached, and nothing else changes that count in between. It is detached, which lowers the
  need by one at once, at the first of:
  1. **a ware is assigned to it**: at the moment the order takes its ware, whether a warehouse
     sends one (ECO-REQ-5.3, any warehouse, the harbour itself included once the good is there)
     or the matching gives it a free ware. The order leaves the open orders in the same step, so
     the need drops when the ware is assigned, not when it arrives;
  2. **it is cancelled**: when its requester's open order for that good is withdrawn (a site's or
     building's requests cancelled, the building removed), the order is detached and then
     deleted;
  3. the re-check of a pass fails (above);
  4. **its harbour is removed**: when a harbour building is torn down, every ware order and every
     settler request attached to it is detached at once, and its need list is cleared. They are
     free to be attached to another harbour at their next turn. Capture was not followed (Q-066).
  A need entry is deleted when its last requester is detached and its second count is zero too
  (that count is taken to be the goods already under way for it, SHIP-4.2 to 4.6; medium).
  Test T19a: in T19, a ship has unloaded one plank into H2. At the next pass the first plank
  order is served from H2 (it takes that plank): H2's plank requesters fall from 2 to 1 in that
  pass, before the plank leaves H2. The site is then torn down: the second order is cancelled and
  detached, and the requesters fall to 0. With H2 torn down instead, both orders are detached at
  once, and with a ship on its sea they are attached to another harbour of island B at their
  next pass, if one is reachable.
- **SHIP-4.2 Asking another harbour.** For each needed good with no supplier
  yet: among the player's other **finished** harbours, in harbour list
  order, the first one that (a) has not already been asked for this good by
  this harbour, (b) does not itself need this good, and (c) can supply it
  (it accepts the good by its warehouse settings and a warehouse on its own
  road network has the good or the good is in its stock; for soldiers also
  by rank) gets a **transfer order** (good, destination = the asking
  harbour). A need whose supplier can no longer supply it is cancelled and
  re-asked later. Confidence: medium.
  **A good the asking harbour holds (2026-11-22 day, E-ship-018: code reading; high):** a need
  is skipped in the pass, neither asked nor checked, while the asking harbour's own stock holds
  at least one of its good. So a new harbour that makes a helper in the same tick as its pass
  (ECO-PROF-3, the tick above) does not ask overseas for helpers in that pass. Test T26.
- **SHIP-4.3 Gathering.** For each good in its transfer orders, a supplying
  harbour sums the amounts all orders want and orders the shortfall (wanted
  minus what is in stock for transfer minus what is already on the way) into
  itself like any warehouse order: wares by a normal ware order
  (`ECO-distribution.md`), settlers by a settler request. Confidence: medium.
  **What an order wants (2026-11-22 night, E-ship-020: code reading, the Hard recording and an
  experiment build; high):** the **destination harbour's need for the good as it counts at this
  pass** (as for the loading, SHIP-4.4), not the count the order was made with. So when the
  destination's need grows after it asked (its later pass finds more requesters; the harbour is
  not asked again, SHIP-4.2 (a)), the supplier orders the difference. Test T28.
  **When the pass runs within the tick (2026-11-22 night, E-ship-020; high):** the harbour's
  update (its expedition step and this 101-tick pass) is that building's own update in the
  buildings' part of the tick (`ECO-requests-dispatch.md` ECO-REQ-0.1, 0.2: in the building list
  order, before the request queue), so it comes **before the tick's transport jobs**: a ware
  order the pass files is matched in the same tick when that tick is a matching tick
  (ECO-REQ-5.1, `t mod 11 = 0`), and a settler request it files is tried in the same tick's
  request queue. Running the harbours after the transport jobs (as a separate navy update)
  delays such an order to the next matching, 11 ticks later. Test T28.
  **What is "already on the way" (2026-11-25, Q-1084; E-ship-024: the Weak recording E-trace-034 and an
  experiment build; high for the measured case):** the supplying harbour's **open orders** of the good
  plus its **served orders whose ware has not arrived yet**: every ware of the good on its way to the
  harbour for one of its orders (lying at a flag, in a carrier's hands, or still in the hands of a
  warehouse's helper carrying it out to the warehouse's flag). This is the same count as the
  expedition's (SHIP-5.2, T14). A ware on a **storage trip** to the harbour (an unordered ware sent there
  only to be stored, `ECO-requests-dispatch.md` ECO-REQ-5.5) does **not** count, although it will arrive;
  once an order of the harbour takes it over (it stays a free ware, ECO-REQ-5.4a, and the matching's
  free-ware step may give it to that order), it counts like any ordered ware. So the shortfall is
  wanted - stock - (open orders + ordered wares on the way), at least 0, and that many orders are filed.
  Test T30.
  **Settlers (2026-11-25 a, E-ship-025: code reading, E-trace-034 and an experiment build; high for
  the measured case):** for a settler good (a builder, a soldier rank, any job) "already on the way"
  is the harbour's **open settler requests of that job** (each counts, not only "one is open") plus
  the **settlers of that job already sent to it and walking there** (including a builder who left as
  a builder for the harbour and was turned home to it, SHIP-5.2a); soldiers: all five ranks
  summed. The shortfall is filed as **one request per missing settler**, like the ware orders. Test
  T31.
- **SHIP-4.4 Calling and loading a ship.** For each transfer order:
  - if the harbour has **no ship called** and the order's goods are ready in
    stock: the nearest free ship (finished, not on an expedition, not moving,
    no assignment, empty) that is on a sea which also reaches one of the
    destination harbour's docking places is **called**; one ship per pass;
    **which ship, exactly (2026-11-25 a, E-ship-025: code reading, E-trace-034, an experiment
    build; high for the measured case):** the harbour's docking places are tried **in their
    order** (SHIP-1), skipping a place whose sea is not the sea of one of the destination's
    docking places; for a place, the candidates are every ship **linked to this harbour** (which link: corrected below, 'which link, exactly'; lying
    at it or sailing to one of its places, whatever else it does) and every ship of the owner that
    is free as above **and linked to no harbour**, on that place's sea; the one nearest to **that
    docking place** (straight-line field distance, the first in the ship list on ties) is called,
    and the first place that has a candidate decides. Calling a ship already linked to this
    harbour only marks it called: it keeps lying there or keeps sailing there (no new route, no
    draw); any other called ship goes to the harbour as before. Test T32;
    **Which link, exactly (2026-11-26 a, Q-1095 answered, E-ship-027: code reading and an experiment
    build; high for the code):** a ship holds four separate harbour links: the docking place it
    has **reserved** (while it sails to a harbour), the harbour it lies **docked** at, the harbour
    whose **transfer called** it, and the harbour whose **expedition called** it (SHIP-5.3). "Linked
    to this harbour" in the call above means only the **transfer-call link**. That link is set by
    the call and cleared by the release after the loading (SHIP-4.5), both on the ship and on the
    harbour. Unloading, going idle and docking never set it. "Linked to no harbour" also means only
    that no transfer-call link is set. Where the ship lies docked, or which place it has reserved,
    does not matter. So the candidates for a docking place are:
    (a) every ship whose transfer-call link is this harbour, whatever it does; and
    (b) every ship of the owner on that place's sea that is finished, not on an expedition, not
    moving, has no cargo and has no transfer-call link. That includes a ship lying idle and docked
    at **another** harbour, such as the destination where it has just unloaded.
    A ship that is sailing is never a candidate under (b). A ship sailing to this harbour because of
    its own search (SHIP-2.4) is therefore not called; it becomes a candidate once it lies docked.
    **The call only sets the two links.** It never changes the ship's state, route or counter. The
    ship acts on it in its own idle update (SHIP-2.3 3): if it lies docked at the calling harbour it
    stays; otherwise it reserves a docking place there and sails. A searching ship that is called
    goes on counting until its try (medium: read in the code, not measured).
    **Consequence:** with one ship and two harbours, H1's transfers to H2 go on one after another.
    After each unloading the ship lies idle at H2 with no transfer-call link. H1's next pass with a
    ready good calls it, and it sails back to H1, docks, is loaded in a later pass and sails to H2
    again. The original does not stall. Reading the docked link as the call's link instead gives a
    single delivery, after which H1's goods wait for good. It gives none at all when the ship's
    first search takes it to H2. Test T34;
  - if the called ship is **docked here** and holds fewer than the ship
    stock limit (12, `sh_setStockLimit`) cargo entries: the order's goods are
    moved from the stock into the ship as a cargo entry for the destination
    harbour (soldiers split by rank, lowest first, at most 5 ranks).
  After the pass, if something was loaded, or a ship is called but nothing
  could be served, the ship is released to sail (SHIP-4.5). Confidence:
  medium.
  **Corrections (2026-11-22 day, E-ship-018: code reading, the Hard recording and the heap dumps
  of 54 000 and 54 500, an experiment build; high for the code, the measured case below):**
  - **A call ends the pass.** When the harbour calls a ship, its pass stops there: no further
    order is looked at, nothing is loaded and nothing released in that pass, even if the called
    ship already lies docked at this harbour. The loading comes in a later pass, once the called
    ship is docked here (it is linked to the harbour, SHIP-2.5).
  - **Only while the order is not yet shipped:** an order is looked at only while the
    destination's need entry for its good has no ship assigned to it.
  - **How many go aboard:** as many as the **destination harbour's need for the good counts at
    the moment of loading** (its requesters, plus the goods already under way for it, SHIP-4.1),
    not the count the order had when it was made; a settler request detached in between (served
    over land at the destination) is not shipped. "Ready in stock" means at least one of the good
    in the harbour's stock.
  Test T26.
- **SHIP-4.5** A released ship with cargo goes Idle -> unloading (SHIP-2.3).
  Confidence: medium.
- **SHIP-4.6 Unloading.** Each update the ship looks at its **first** cargo
  entry: if the ship is docked at that entry's destination harbour, the
  entry's goods go into that harbour's stock (soldiers by rank) and the
  requests waiting for them are told; one entry per update. Otherwise the
  ship reserves a docking place at the destination and goes **to harbour**
  (SHIP-2.5). With no cargo left it goes Idle. Confidence: medium-high.
- **SHIP-4.7** Settlers travel as cargo (counted, not walking): a settler
  ordered overseas is taken into the supplying harbour's stock, shipped as a
  count and comes out of the destination harbour's stock like any warehouse
  settler (`ECO-requests-dispatch.md`). Confidence: medium.

- **SHIP-4.8 A harbour removed; a defeated player's ships (2026-11-24 b, Q-066 partly answered;
  E-ship-022, code reading).**
  1. **No capture.** A harbour cannot be attacked (`MIL-attack-combat.md` MIL-ATK-1.3: the attack
     dialog opens on nothing but the headquarters and occupied military buildings), so it never
     changes hands. It only goes: torn down by its owner or lost with its land, both with remains
     (`MIL-conquest.md` MIL-CON-4.1, 4.3: settlers in its stock walk out, wares in it are lost).
     Confidence: high.
  2. **What goes with it.** Its own records are deleted with the building: its need list and the
     orders attached to it (4.1, end of order item 4), its transfer orders (4.2 to 4.4), its
     docking-place records and its expedition with everything set aside for it (SHIP-5.1a: the
     set-aside goods are outside the stock, so they are not part of 4.3's walk-out; they are lost).
     The removal itself does **not** visit any ship: no ship is told, re-routed or emptied at that
     moment. Confidence: high for "no ship visited" (the building's teardown read whole); medium
     that the expedition's builder and goods are lost rather than walked out.
  3. **Ships.** A ship refers to a harbour only by a link, which reads "gone" from the removal on.
     So, by the existing rules: a ship lying idle at it (2.3 correction) has no harbour link any
     more and turns to search in its next update, keeping its counter; a docked one (2.6) goes to
     search; one sailing to it (2.5) finds the link gone when it stops and goes Idle, then
     search. A search finds the nearest harbour of the owner with a free docking place on its sea
     (2.4); with none, it keeps searching every 101 ticks for the rest of the game (making room on
     its way as 2.4 says). Confidence: high (the ship's update read; the states as in 2.3 to 2.6).
  4. **Cargo for a removed harbour.** Not settled: a ship's first cargo entry whose destination
     harbour is gone is neither unloaded (4.6's dock test needs the harbour) nor dropped by the
     removal (item 2); the unloading step then tries to send the ship to the gone harbour, and
     that call has no test for a missing harbour, so unless the entry's own link drops the entry when
     the harbour goes (its callback not identified), the original would fail there (Q-1089). Until answered:
     ours' stand-in (the entry is lost, the ship goes on with the next entry) stays.
  5. **A defeated player's ships.** A player is never taken out of the game (`AI-overview.md`
     AI-2.1: the slot keeps its player object; nothing clears it on defeat), so his ships stay on
     the water and go on by these rules: with no harbour of his left, every ship searches every
     101 ticks without end; a ship in expedition mode keeps waiting for orders (SHIP-6.3). They
     are not removed, not handed to the winner and not sunk. Confidence: medium-high (no code
     path that removes a ship other than its own end was found; the defeat itself has no step that
     visits ships).
  Test T29.

### SHIP-5 Preparing an expedition (harbour side)

- **SHIP-5.1 Start.** Command `prepare-expedition` (harbour window,
  "Expedition management") switches the harbour's expedition to **Gathering
  goods** if it is not already active; otherwise nothing. Computer players
  use the same step. Confidence: high.
- **SHIP-5.1a No stop (added 2026-11-05, Q-1011; E-ship-004).** No command ends a harbour's preparation: `cancel-expedition`
  (SHIP-6.5) acts on a ship only, and the harbour window offers no stop
  (`UI-ingame.md` UI-GAME-8.1a). A preparation, once started, runs until its
  transfer to a ship (SHIP-5.5); what it has set aside stays in its store
  and out of the harbour's stock until then. Confidence: high for the
  commands and the window; what happens to the store when the harbour is
  destroyed was not read.
- **SHIP-5.2 Gathering goods.** A counter runs 0..100: one pass every **101
  ticks**. In a pass, for every material of the **harbour's own building
  cost** (frame list then main list: 4 planks, 6 stones): missing =
  required - already set aside. If missing > 0: as many as the harbour's
  stock has are taken from the stock and set aside; then the harbour's
  orders for that good are made to equal what is still missing (more orders
  placed, or surplus orders cancelled). **All its orders of the good count**,
  also the served ones whose ware is still on its way (with a warehouse's
  helper, or lying or carried towards the harbour); only orders not yet
  served are cancelled (added 2026-11-02 night, Q-385, T14; E-ship-003: the count over the building's order list,
  an order without a ware is the one cancelled). **A ware on a storage trip to
  the harbour is not one of its orders and does not count** (Q-297; E-ship-003: an order is a record the building adds to its own list,
  while a storage trip only sets the ware's destination). A ware
  delivered goes into the stock and is set aside at the next pass. Then the **builder** (`good_constructor`
  DD2DAEBE): if none is set aside, one is taken from the stock if there is
  one; else, unless one is already requested, one builder is requested. When
  nothing is missing and the builder is set aside: **Getting ship**.
  Confidence: high.
  **The requested builder's way (added 2026-11-21 pm, E-ship-015; recording, code reading and an
  experiment build):** he leaves the warehouse that serves the request as a builder going to his
  site: out of its door to its flag. There, in his arrival tick A, his walk to the "site" is refused
  (the harbour is no construction site) and he goes home from that flag, **his home being the
  harbour**: he stands in A, A + 1 and A + 2 and starts his road walk to the harbour at progress 0 in
  A + 3 (`TRN-free-walk.md` TRN-FREE-6.4 step 1, a settler with a home warehouse assigned; not the
  nearest warehouse, which is usually the one he just left), and goes into its stock, from where
  the next pass sets him aside. The request is gone once it is served, so while he walks each pass
  requests another builder (recorded: four builders for one expedition). Test T23. Confidence: high
  for the measured walk (4 of 4 recorded cases); medium for the reason (the code's refusal of the
  walk to a building that is not a site).
  **When he is in (added 2026-11-21 pm, E-ship-015):** the harbour does not send that builder out
  again for its own open builder request: in the recording 97659 is in the harbour from counter
  53186, and the request filed at 53173 is served only at 53220, from the castle. Two readings fit
  and are measured equal (Q-1077): (a) a builder taken in by a harbour that is gathering without one
  is set aside at once, never counting as its stock; (b) a harbour's own request is never served
  from that harbour. Either gives the Hard pair's settlers 53 900 (was 53 200) and its generator
  53 726 (was 53 194); (a) keeps 30 more matching generator ticks after it. Test T23.
- **SHIP-5.2a The builder step of a pass, in order (2026-11-24 c; E-ship-023, code reading, high).**
  After the materials, in the same pass:
  1. If a builder is **already set aside** (from an earlier pass): if no material was missing in
     this pass, the expedition goes to **Getting ship**; otherwise nothing more.
  2. Else, if the harbour's stock holds a builder: one is taken from the stock and set aside, and
     the harbour's **first open builder request** (in the order its open settler requests were
     filed) is **cancelled**; only that one, and the pass ends there. **Getting ship is not
     entered in this pass** even when nothing is missing: it waits for the next pass, 101 ticks
     later (step 1).
  3. Else, if the harbour has no open builder request, one builder is requested; with one open,
     nothing.
  So an expedition whose last thing to come is its builder leaves Gathering one pass after he is
  set aside, and of several builder requests open at that pass (SHIP-5.2's way: one per pass
  while he walks) one is cancelled per set-aside; a request already served is not.
  **"No material was missing in this pass" (2026-11-25 a, E-ship-026: the Weak recording E-trace-034,
  its heap dumps and an experiment build; medium-high, measured):** a material counts as missing
  when, at the start of the pass's material step, fewer of it are set aside than the expedition
  needs, **even if the harbour's stock covers the rest and this pass sets it aside**. So the pass
  that sets aside the last materials from the stock stays in Gathering, and Getting ship comes one
  pass later (as for the builder, step 2), not in the same pass. Test T33.
- **SHIP-5.3 Getting ship.** Every 101 ticks: for each docking place of the
  harbour, in order, the nearest free ship (as in SHIP-4.4) on that docking
  place's sea; the first found is called to this harbour: **Waiting for
  ship**. Confidence: high.
- **SHIP-5.4 Waiting for ship.** Each tick: when the called ship is docked at
  this harbour: **Transfer goods to ship**. Confidence: high.
- **SHIP-5.5 Transfer.** In one tick: the set-aside goods and the builder are
  moved into the ship, the ship's **expedition mode** is switched on, the
  harbour's link to the ship ends and the harbour's expedition becomes
  inactive (a new one can be prepared). The ship, Idle with expedition mode
  and no target, sends "Ship waiting for Orders" (SHIP-2.3). Confidence:
  high.

### SHIP-6 The voyage

- **SHIP-6.1 Landing options.** Whenever the ship starts waiting (after
  SHIP-5.5 and after each arrival, SHIP-6.4) it computes up to six landing
  options, one per **compass direction** 0..5:
  - candidates are the harbour spots of every **land** continent
    (SHIP-1.1);
  - a spot is skipped when it lies on the continent of the ship's last port
    position (SHIP-2.7) and within distance 3 of the ship (its own harbour).
    A ship without a last port position skips none here (medium: the
    original then reads a value from outside its continent table);
  - the spot must be a site where a harbour could be built **by nobody**
    (unowned land) **or by the ship's player** (enemy land is excluded);
  - its direction is the bearing class from the ship to the spot (the same
    6-way class as the catapult turn, `MIL-conquest.md`);
  - its landing field is the first **Water** field of the **ship's sea**
    found in the rings of radius 1 to 3 around the spot, in the ring order
    of `MAP-grid-geometry.md` MAP-GRID-3: ring 1, then 2, then 3; inside
    ring k the walk starts at the spot moved k steps in direction 4, visits
    the field after each of the k steps in each of the directions 0 to 5,
    and so visits that start field **last** (ring 1: the neighbours in
    directions 5, 0, 1, 2, 3, 4). Fields outside the map are skipped. A spot
    without such a field within radius 3 is skipped (rule restated
    2026-11-05, Q-1014, T15; E-ship-004);
  - per direction the **nearest** spot wins; on equal distance the **later**
    spot in map order replaces the earlier one.
  Directions without a spot have no option (their arrow is not lit).
  **SHIP-6.1a** The ship sails to that landing field and stops on it
  (SHIP-6.4); the landing field is always Water, so a ship never stands on
  the land field next to it. Confidence: high (the walk and its tests read
  2026-11-05); medium for "stops on it" (the route ends at its target).
- **SHIP-6.2 Command "go" (`sail-expedition`, direction d).** Accepted only while the
  ship is on an expedition and has no target: the option of direction d
  becomes the target (an empty option sets no target). Confidence: high.
- **SHIP-6.3 Waiting.** Each update: if a target is set, the ship starts
  sailing to it (**sailing**). If expedition mode has been switched off
  (SHIP-6.5), **returning**. Confidence: high.
- **SHIP-6.4 Arrival.** When the ship has stopped at the target: its last
  port position is set to the first land neighbour of its field (SHIP-2.7;
  ring 1 only, so none when no neighbour is land; corrected 2026-11-16
  night from "the land field nearest to it"), the target is
  cleared, the options are recomputed (SHIP-6.1), radius 6 is explored
  (SHIP-3.4) and the player gets "Ship
  waiting for Orders" again (waiting). Confidence: high.
- **SHIP-6.5 Cancel (`cancel-expedition`).** On a ship in expedition mode: expedition
  mode off, target and all six options cleared. Waiting or sailing ships go
  **returning**: when stopped, they search (SHIP-2.4, at once) and, docked,
  give the expedition goods back to that harbour's stock (SHIP-2.3 case 4).
  Confidence: medium-high.

### SHIP-7 Founding a harbour

- **SHIP-7.1 Possible.** The "Construct harbor" button works when the ship
  is not moving, is in expedition mode, has a last port position
  (SHIP-2.7), the spot is a harbour site for nobody or for the player, and
  the ship is **closer than 5** fields to that spot. **The spot** is taken
  from the harbour spots of the land continent of the last port position:
  the one nearest to the **ship's own field** (strictly smaller distance
  wins, so the first in the continent's list on equal distance); a
  continent without spots gives none. The ship window shows the button
  exactly while this holds (`UI-ingame.md` UI-GAME-4.5 f); a ship a
  mission's script made is no exception, since it has a last port position
  from its creation (SHIP-2.7). Confidence: high (wording of the spot
  sharpened 2026-11-16 night, E-ship-005).
- **SHIP-7.2 Command (`found-harbour`).** A harbour **construction site** of the
  player is created at the spot; it claims **range 4** and explores **radius
  6** around it at once (`TER-territory.md` TER-2.4). A builder walks out of
  the ship onto the site (created as a settler at the site's field and linked
  to it); the 4 planks and 6 stones aboard become the site's delivered
  material, so the site's own material requests are cancelled. Expedition
  mode is switched off and the options cleared; the empty ship then searches
  for a harbour (it may wait until the new harbour is finished).
  **The ship after the founding (2026-11-19 night, E-ship-011: code reading and the Hard
  recording; high).** The founding changes only the expedition: mode off, target and
  options cleared. The ship keeps its state (waiting); its next update turns it to **returning**
  (SHIP-6.3: expedition mode off), and returning, as it stands still, searches **at once** (SHIP-6.5:
  the counter at 100, SHIP-2.4), so it sails for the nearest finished harbour in the update after
  that. It is never set idle (idle would start the search's counter at 0 and delay the voyage by
  about 100 ticks). Test T20. The search's first try comes in the second update after the turn
  (SHIP-2.4 as corrected, Q-1073), so the ship sails in the third update after the founding.
  **The founded site is levelled (2026-11-21 day, E-ship-012: the Hard recording and an experiment
  build; high):** the harbour site a founding makes is levelled from its creation (the recording's
  buildings table shows it levelled in the founding tick), whatever the ground's heights: no
  leveller is ordered and its builder's work begins by the construction rules. Test T22.
  **The builder's first walk (2026-11-21 day, E-ship-012: recording and an experiment build;
  medium):** the builder made on the site's field walks from it to the site's flag, a plain
  one-field walk at progress 0 in the founding tick (23 ticks on that ground), as a builder coming
  to his site (`ECO-warehouse-construction.md` ECO-HQ-4.6b) whose walk ends on the flag; there he
  stands in his arrival tick A and in A + 1 and starts his first work step at progress 0 in A + 2.
  Making him walk so (and numbering the founding as SIM-START-4.4 (b) T16) gives the recorded walk
  and arrival; ours' arrival rules then start his first step in A + 1, one tick early (Q-1075).
  Test T22. **Why A + 2 (answered 2026-11-21 pm, Q-1075, E-ship-014; code reading and an experiment
  build):** his walk is the builder's ordinary walk to his site, a road walk to the site's flag,
  ordered here from the site's own field, which is off the roads. As every road walk that starts off
  the roads (`TRN-carriers-transport.md` TRN-CARRY-3.5a, `TRN-free-walk.md` TRN-FREE-6.4a), he
  reaches the flag in A and **stands there one tick** (A + 1) before the road part, here empty, ends
  the walk; only then does he start on the site as any builder arriving (ECO-HQ-4.5), so his first
  work step begins in A + 2. With it the Hard pair's settlers first differ at trace tick 52 600
  (was 50 700) and its buildings at 52 600 (was 50 800), **measured with `ECO-food-mining.md`
  ECO-FOOD-4.5's remembered fish field (T63) in** (2026-11-21 eve: without that rule the settlers
  first differ at 51 600, a fisherman's retry that follows the generator's difference of 51 418,
  buildings 52 600; experiment build and implementation alike).
  Confidence: high for the steps; medium for the builder's walk.
  **The builder and the load, in order (2026-11-16 day, Q-157; E-ship-008, code reading, high).**
  1. The site is made on the spot (with its claim and exploring, as above).
  2. One builder is taken off the ship's load, and **a new builder is made standing on the site's
     own field** and linked to the site as its builder. He does not walk out of the ship: he
     appears on the site in the same tick, and his work starts by the construction rules.
  3. **The rest of the load goes into the site's store, all of it**, good by good, with no test of
     what the site needs (only the store's limit, which a site does not set). The expedition's load
     is always exactly the harbour's cost (4 planks, 6 stones; SHIP-6.1), so nothing is left aboard
     and nothing is too much.
  4. Every open material order of the new site is cancelled, so no carrier brings anything more.
  5. The expedition is switched off and its options cleared.
  So ours' "delivers the material up to what the site still needs" gives the same result for the
  shipped load; a load that differs (a script's `createShip`, SHIP-2.7) would go into the store
  whole.
- **SHIP-7.3** The new harbour is built by the normal construction rules
  (`ECO-warehouse-construction.md`) and, when finished, makes its docking
  places (SHIP-1.2). Confidence: high.

### SHIP-8 Commands (summary)

| Name | Effect |
|---|---|
| `prepare-expedition` | harbour: prepare expedition (SHIP-5.1) |
| `cancel-expedition` | ship: cancel expedition (SHIP-6.5) |
| `found-harbour` | ship: found harbour (SHIP-7) |
| `sail-expedition` | ship: go in direction d (SHIP-6.2) |

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| ship speed | 0.18 | progress per tick | high | `ship.lua`, E-ship-001 |
| ship stock limit | 12 | cargo entries | high (value), medium (unit) | `ship.lua`, E-ship-001 |
| ship frame / main cost | 4 / 8 planks | | high | `ship.lua` |
| harbour pass period | 101 | ticks | high | E-ship-001 |
| ship search period | 101 | ticks | high | E-ship-001 |
| expedition pass period | 101 | ticks | high | E-ship-001 |
| expedition material | harbour cost: 4 planks, 6 stones + 1 builder | | high | E-ship-001, `buildings.lua`, tutorial text |
| docking ring | distance 3, 18 fields | | high | E-ship-001 |
| landing field search | rings 1..3 around the spot, ring start last (MAP-GRID-3) | | high | E-ship-001, E-ship-004 |
| build-harbour distance | < 5 | fields | high | E-ship-001 |
| landing claim / explore | 4 / 6 | fields | high | TER-2.4 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | ship completes in tick t, player has one finished harbour reachable by sea | ticks t.. | Idle in its first update, search; first try 101 updates later; docking place reserved, ship sails there, docked, Idle |
| T2 | finished ship of player 1, no harbour of player 1 reachable, standing on a docking place of player 2's harbour | search try | moves to a neighbour Water field without LandingPosition chosen by draw mod 6 |
| T3 | harbour whose 18-ring has Water at ring positions 1, 2, 3 (consecutive) | docking places | positions 1 and 3 only (2 touches 1) |
| T4 | harbour, prepare expedition with 2 planks, 6 stones, no builder in stock | first pass (101 ticks later) | 2 planks + 6 stones set aside, 2 plank orders open, 1 builder requested; state stays Gathering |
| T5 | as T4 when all arrived | next pass | Getting ship; next pass calls the nearest free ship on a docking place's sea |
| T6 | called ship docked | that tick | goods and builder aboard, expedition mode on, harbour idle; ship: "Ship waiting for Orders" |
| T7 | ship on expedition; spots: A at distance 10 bearing 1, B at distance 8 bearing 1, C on enemy land bearing 3 | options | direction 1 = B's landing field; direction 3 none; others none |
| T8 | waiting ship | go 1 | sails to B's landing field; on arrival message again and options recomputed |
| T9 | ship 4 fields from spot B | Construct harbor | harbour site of the player at B, land range 4 claimed, radius 6 explored; site has 4 planks and 6 stones, no material requests; builder on the site |
| T10 | ship 5 fields from the spot | Construct harbor | refused (not < 5) |
| T11 | waiting ship | cancel | expedition off; returns to the nearest reachable harbour; goods back into its stock |
| T12 | harbour H2 needs 3 planks, H1 (same sea, other island) has 3 planks in a warehouse on its network | passes | H2 asks H1; H1 orders 3 planks in; when in stock, H1 calls a ship, loads one cargo entry (3 planks for H2), ship sails to H2 and unloads them into H2's stock |
| T13 | SHIP-3.1 (the route and its draws), 2.4, `ECO-food-mining.md` ECO-FOOD-7.4b (T60): E-trace-018 (R5, the Weak game), player 1's ship 94308 on (146, 46), its main work 0 in counter 50011 (8.75e-07 in 50010); E-trace-013 (R3, the Hard game), ship 94738, work 0 in 45867 | the generator's draws at the first search try | the search counter counts 1 in the tick the work reaches 0 and 101 a hundred ticks later; the try is in the next tick (R5 50112, R3 45968): a docking place of the harbour 90366 is reserved and the ship sails, its route's read-out drawing: R5 3 draws in counter 50112 (none else that tick), R3 12 in 45968 (9 for three animals, then 3 for the ship) (`generator.csv`, recorded). Counting whole trips (the ship started after the shipwright's step) and a route search without draws (a plain flood): the Weak 150 000 pair's generator first differs at 50113, the Hard pair's at 45969. With both rules (experiment build): every draw of those ticks equal; the Weak 150 000 generator 50113 -> 50934, the Hard 150 000 generator 45969 -> 46754 (settlers 46100 -> 46600); every other pair unchanged |
| T14 | SHIP-5.2 (served orders count): E-trace-018 (R5, the Weak game), player 1's harbour 90351 (144, 47) gathering 4 planks and 6 stones; its orders filed at counter 50765; the castle sends planks at the matchings of 50820 and 50853 (helpers 96100, 96112; 96101 lies at (140, 50) from 50873) | the harbour's next pass (101 ticks after the filing one, before the matching of 50864) and what the castle sends | no new plank orders: 2 open + 2 served = 4 missing; the castle's sends: planks at 50886 and 50952, then stones from 50985 (recorded, objects table). Counting only the open orders and the wares already lying or carried (not those in a helper's hands) files 2 more plank orders at the pass: a fifth plank at 50985 (experiment build), and in the Hard game (E-trace-013, harbour 90366) two more stones at 46739 and 46750, the storehouse (130, 33) then busy at the take-out of 46753. With the rule (experiment build, with `ECO-distribution.md` T24): the Weak game's take-out draws of 51297 as recorded, the Hard game against R3 settlers 46751 -> 48154; the Weak 150 000 generator 51472 -> 51724 and the Hard 150 000 pair 46600 / 47000 / 46754 -> 48200 / 48200 / 48043 (settlers / buildings / generator) for T24 and this rule together; every other pair unchanged |
| T15 | SHIP-6.1 landing field: `tutorial04.s2m`, the harbour spot (129, 34) of land continent 36, the ship on sea continent 0 | the landing field search | the spot and ring 1 are land; ring 2 from its start (128, 36) visits (129, 36), (130, 36), (130, 35), (131, 34), (130, 33), (130, 32), (129, 32), (128, 32), (127, 33), then (127, 34), the first Water field: the landing field is (127, 34). (127, 35), lesson 4's marked field, is land; (127, 36), where lesson 4's script puts its second ship, is Water but comes after (127, 34) in ring order |
| T16 | SHIP-2.7, 7.1 (Q-1060): `tutorial04.s2m`, lesson 4's script runs `createShip(player0, 101, 63)` and `createShip(player0, 127, 36)` | the ships' last port positions; the second ship's window; `found-harbour` for it | second ship (127, 36): its neighbours in test order are (126, 37) Water, (127, 37) Water, (128, 36) land of continent 36, so its last port position is **(128, 36)**; continent 36's only spot is (129, 34), 3 fields from the ship and free, so `!Construct harbor.` is shown and the command is accepted: a harbour site of player 0 on (129, 34). First ship (101, 63): neighbours (101, 64), (102, 64), (102, 63) Water, (102, 62) land of continent 66, so its last port position is **(102, 62)**; in its landing options (SHIP-6.1) every spot of continent 66 within 3 fields of (101, 63) is skipped, among them (103, 60) at distance 3. After "go 2" the first ship has no last port position until it stops |
| T17 | SHIP-2.5, 2.7: a ship docks at a harbour on (40, 40) at a docking place 3 fields away; then it is loaded for an expedition (SHIP-5.5) | last port position after docking; options | (40, 40) (the harbour's field, land), not the docking place; the expedition's options skip the spots of the harbour's continent within 3 fields of the ship |
| T18 | SHIP-2.7, 6.4: an expedition ship stops on a Water field F whose neighbours in directions 4 and 5 are Water and in direction 0 land | arrival | its last port position is F's neighbour in direction 0. A ship stopping on a field with six Water neighbours has none, and `!Construct harbor.` is not shown there |
| T19 | SHIP-4.1 (Q-063): player 1 has island A (headquarters, harbour H1, a finished ship on the sea) and island B (harbour H2, a site needing 2 planks, no warehouse with planks on B), both harbours finished and accepting planks | the site's plank orders' turn | each order finds no warehouse with planks over B's roads and is attached to H2 (reachable from the site's flag, a ship on its sea); H2's plank need is 2; SHIP-4.2 then asks H1. With the ship removed (none on the sea) no order is attached and H2 has no plank need |
| T19a | SHIP-4.1 (Q-063 end): T19 after a ship has unloaded one plank into H2 | the next ware order pass; then the site torn down | the first plank order takes H2's plank and is detached in that step (H2's plank requesters 2 -> 1 before the plank leaves H2); the site's removal cancels and detaches the second (1 -> 0). With H2 torn down instead, both orders are detached at once |
| T20 | SHIP-7.2 (the ship after the founding): E-trace-013 (R3, the Hard game), player 1's expedition ship 94738 founds the harbour on (147, 136) in counter 50 658 (AI-misc T9) | its states (the recording's ships table, counters) | waiting with the cargo out in 50 658, returning (state 8) in 50 659, searching with the counter at 100 in 50 660 (101 in 50 661), sailing home (to harbour, its route) from 50 662; at home (146, 50) docked in 51 179, idle in 51 180. Setting it idle at the founding starts the search at 0: the voyage home and its route's draws about 100 ticks late; with the rule: not yet measured (no experiment build on 2026-11-19 night) |
| T21 | SHIP-2.4 (corrected), SHIP-7.2: E-trace-013 (R3, the Hard game), ship 94738 after the founding of counter 50 658 (T20); all 35 search ends of the recording | when the search tries | turning to search in 50 660 (counter 100), 101 in 50 661, sailing (the route's 13 draws) from 50 662. Building T20 with the try in the next update puts the Hard 150 000 generator apart at trace tick 50 662 (0.12.3: 50 663); T20 with the try in the second update: 50 765 (matching 50 887; experiment build on 0.12.3; every other pair unchanged; the war's switches on). Running every search's counter one further instead (a try after 101 also when counting from 0) puts the Hard generator apart before 49 000 |
| T22 | SHIP-7.2, the founded site: E-trace-013, the harbour site 97262 on (147, 136), founded in counter 50 658; its builder 97268 | the site's levelling, the builder | the site levelled in 50 658 (frame left 1, main left 1), its frame starting at 50 764; builder: on (147, 136) walking to (147, 135) from 50 658 (progress 0), standing there 50 681, at work 50 682, his first lattice step at progress 0 in 50 683. Levelling the site (0.12.3: levelling at trace tick 50 700, the original a frame): with the levelled site (on T20, T21) Hard 150 000 buildings 50 700 -> 50 800, generator 50 765 -> 51 418 (matching 51 673), settlers stay 50 700 (experiment build). With the builder's walk to the flag and the founding's numbers (T16) his walk, arrival and number are the recorded ones; the settlers stay at 50 700, his first work step one tick early (Q-1075) |
| T23 | 5.2 the builder's way: E-trace-013 (R3, the Hard game), player 1's harbour 90366 on (144, 47) gathering an expedition; its builder requests served from the castle (door field (139, 52)) at counters 52886, 52986, 53086, 53220 (builders 97659, 97682, 97695, 97725) | each builder's walk | out of the door from the serving tick (progress 0.045 in it), on the flag (140, 50) 45 ticks later (A = 52931 for 97659), the going-home state in A, standing to A + 2, the walk at progress 0 in A + 3 (52934) over (139, 49), (139, 48), (138, 47) to the harbour, in it at 53186; the next builder request at the pass 101 ticks after the previous one. Walking him into the harbour from the start (no turn, no standing) puts the Hard 150 000 settlers apart from trace tick 53000 (3 ticks ahead); with the rule 53200 (experiment build on 0.12.3 with the rules of 2026-11-21; every other pair unchanged) |
| T24 | SHIP-4.1, the stored cost of the harbour choice: E-trace-018 (R5, the Weak game), player 1's sawmill (144, 37), flag (145, 36), with an open wood order and no warehouse holding wood; its harbour (144, 47), flag (145, 46), with a ship on its sea; the ware roads (145, 36) .. (145, 46) of lengths 4, 6, 4, 4, 8, 7, 4 (37 in all); the carrier of the last road, (143, 45) - (145, 46), holds a ware from about counter 54680 to 54709; the ware renumbering of 54678 has emptied the store | the store's entry (145, 36) -> (145, 46), and the storage run of counter 55740 for the plank lying at (145, 36) | the harbour choice of the matching of counter 54692 stores 45 (that road at 3L; the original's heap dump of 55000 holds 45 under (145, 36)); at 55740 the plank goes to the warehouse (133, 39) (cost 44) rather than the harbour (45). With a fresh search there, the storage run of 54725 asks the pair first and stores 37 (every road free): the plank of 55740 goes to the harbour and the Weak pair's generator first differs at trace tick 56470; with the rule 56729. Asking from the chosen order's flag instead of the representative's moves the Hard pair's settlers from 54400 to 50300 (experiment build on 0.12.3 with every rule of 2026-11-21; with the rule every other pair unchanged) |
| T25 | SHIP-2.3 5 (stays idle) and SHIP-2.4a (pushed out): E-trace-013 (the Hard game), player 1's ships | the ships table | (a) ship 94738 docks at the harbour (144, 47) at counter 45991 and lies idle with counter 0 from 45992 to its expedition (state 4 from 48458), no search in between; after its return it docks at 51179 and lies idle 51180 .. 54506. (b) every turn from idle to search shows the counter 90 in its first searching row and the try after 101: 19 of 19 (first: ship 100861, idle 87011 .. 87118, searching from 87119). With (a) an experiment build's ship lies idle from 45992 (it searched and docked again every 104 ticks before), its counter 0 through the expedition as recorded (74 before); the compared first differences of every pair are unchanged (the ships' rows are not compared; experiment build on 0.12.3 with every rule of 2026-11-21) |
| T26 | SHIP-4 the pass's tick, 4.2 a good held, 4.4 the call ending the pass and the load's count: E-trace-013 (the Hard game), player 1's harbour (144, 47) (finished in counter 6734; ship 94738 lying idle and linked there since 51 180) and the new harbour (147, 136) founded by the expedition, finished in counter 54 100; on its island two builder requests (sites (147, 134), (144, 135)) and two helper requests (roads) wait for it; the founding's builder walks into it in 54 149 and is sent out to (147, 134) in 54 154 | the passes of both harbours, what is asked and loaded | (147, 136)'s passes in 54 201, 54 302, 54 403 (each with a new helper there); no helper transfer is ever asked (the heap dump of 54 500 holds the builder and plank needs only); ship 94738 is loaded in 54 507 (pass of (144, 47)) with **1 builder**, no helpers (the call itself, in an earlier pass, does not show in the ships table: the ship lies idle through 54 506). Ours before the rules: passes one tick early (54 200, 54 203), both helpers asked in 54 200, the ship called and loaded with 1 builder and 2 helpers in one pass (54 203): the Hard 150 000 generator apart at trace tick 54 206. With the four rules (experiment build with every rule of 2026-11-21): 1 builder only, loaded in 54 305 (called 54 204); Hard 150 000 generator 54 206 -> 54 308 (matching 54 256 -> 54 408), settlers 54 400 -> 54 500, buildings 55 700; Weak generator 56 729 (matching 56 854 -> 56 805); the war and every other pair unchanged. The two passes still between ours' load (54 305) and the recorded one (54 507) are Q-1079 |
| T27 | 4.1 (the network a settler request is attached over): E-trace-013 (the Hard game), player 1's new harbour (147, 136) (flag (147, 135), finished in counter 54 100, passes 54 201, 54 302, 54 403); the guard house site (144, 135) (flag (145, 134)) has a builder request no warehouse can serve; the road (145, 134)-(147, 135) stands from trace tick 53 502 without a carrier; its carrier is made by the new harbour from the helper of its pass of 54 302 and sent out in trace tick 54 305 (on the flag in 54 328); harbour (144, 47) holds 3 builders, its ship 94738 idle at its docks | the builder request's harbour and the transfer | the request is not attached before 54 305 (heap dumps: the two flags in different networks of the carrier roads at 54 000, in one at 54 500); attached at its first turn after (54 309); the harbour asks in its pass of 54 403; the builder is loaded at (144, 47) in 54 507 (recorded). Attaching over every road attaches it in 54 103, asks in 54 201 and loads in 54 305 (the Hard generator apart from 54 308); with the rule (experiment build) from 54 412 |
| T28 | SHIP-4.3 what an order wants and when the pass runs: E-trace-013 (the Hard game), player 1's harbour (144, 47) (finished in counter 6734) supplying the new harbour (147, 136) on the island with planks for the guard house site (144, 135); (147, 136)'s pass of counter 54 302 asks (144, 47) for planks (need 2); its pass of 54 403 counts 4; (144, 47)'s passes at 54 305 and 54 406 (6734 + 471 / 472 x 101); counters 54 406 and 54 307 are matching ticks | the castle's plank sends to (144, 47) | two planks from the castle (139, 51) for the first two (helpers out of its door at 54 307 and 54 340), then two more for the grown need: the first at **54 406, the tick of (144, 47)'s pass**, the second at 54 439. The castle's door is then busy at the take-out of 54 411 (ECO-DIST-1.7: no draw for it), so an animal's target that tick comes from the 7th draw: deer 89915 on (44, 65) walks to (46, 65) (start draw) in the tick of trace 54 412. Wanting the order's first count (2) sends no more planks; running the pass after the transport jobs sends the third plank at 54 417 (the next matching): the Hard 150 000 generator then differs from trace tick 54 412 (the deer walks to the neighbour (44, 66)) |
| T29a | 5.2a: a harbour gathering with every material set aside, no builder set aside, one builder in its stock and two open builder requests (filed at the two passes before) | its passes P and P + 101 | P: the stock builder set aside, the earlier-filed open builder request cancelled, the other kept, the state stays Gathering; P + 101: Getting ship (and nothing requested). With no builder in stock at P and one request open: nothing requested at P |
| T29 | 4.8: player 1 has harbours H1 and H2 on one sea and ship S idle at H1 (docked earlier, harbour link to H1, search counter 37); H1 is torn down in tick t | S from t + 1 | in t nothing happens to S; in t + 1 S has no harbour link and turns to search with counter 37 (2.3 correction, 2.4), and when the counter reaches its try it finds H2 (free place) and sails there. With H2 gone too: S searches every 101 ticks forever, stays on the water, never removed; H1's expedition goods (if it was preparing one) are gone, its stock's settlers (not helpers) walk out of its door (MIL-CON-4.3) |
| T30 | 4.3 'already on the way' (Q-1084): E-trace-034 (the Weak game, `MP_2P_Storm_Coast`, player 1 Weak), the harbour (144, 47) at its pass of counter 60 695: transfer orders for the harbour (147, 136) (finished at 60 433) wanting 4 planks; no plank in stock; no open plank order; plank 97328 (made at 60 145) on a storage trip to (144, 47) since 60 180 (at 60 695 in a carrier's hands on the road to flag (142, 43), where it lies from 60 702); no ordered plank on its way | the pass | **4** plank orders are filed (the storage trip does not count; counting it gives 3). The matching then serves them from the castle at 60 698, by 97328 itself at 60 709 (the free-ware step; in the experiment build its cost 11 against the castle's 22), from the castle at 60 731 and 60 764 (helpers 97390, 97394, 97396). At the next pass (60 796): wanted 4, stock 0, four ordered planks on their way (97328 and the three castle planks): nothing is filed, so the castle's door is free for the water helper 97401 at 60 797 and the builder 97403 (to the harbour) at 60 832, and the castle makes no take-out draw at 60 837 (6 draws, not 7). Experiment build: the Weak game's generator first differs at trace tick 60 901 instead of 60 838; Hard 150 000 (58 115) and the war (S 24 350, B 24 890, G 24 343) unchanged; `s2-app fidelity`: Weak generator matching ticks 60 947 -> 60 969, every other pair unchanged |
| T31 | 4.3 'already on the way', settlers (E-ship-025): E-trace-034 (the Weak game, player 1 Weak), the harbour (144, 47) supplying the harbour (147, 136) with one builder; at its pass of counter 60 796 one builder request is filed and served from the castle at 60 832 (builder 97403, walking to the harbour until 61 082) | the pass of counter 60 897 | wanted 1, stock 0, open requests 0, one builder of the job walking to it: **nothing** is filed (recorded: no builder leaves the storehouse (133, 39) at counter 60 900; the next new settler is the carrier 97415 at (119, 59)). Counting only "a request is open" files a second request, served from (133, 39) at 60 900. Experiment build: the Weak generator 60 901 -> 60 928 on top of main's equivalent; Hard 150 000 (58 115) and the war (24 343) unchanged |
| T32 | 4.4 'which ship' (E-ship-025): E-trace-034, the harbour (144, 47) with docking places (146, 50), (147, 48), (147, 46) in this order; ship 94308 lying at (146, 50) linked to (144, 47) since its return (57 519), ship 96085 idle at (146, 46) linked to no harbour (both with no transfer-call link, so both are candidates under T34's reading, with the same result); at the pass of counter 60 998 a transfer to (147, 136) is ready (one plank in stock) | that pass and the next | 60 998: 94308 is called (distance 0 to the first docking place) and stays where it is, no draw; 60 998 + 101 = 61 099: 94308 is docked here, so one plank and the builder go aboard (two cargo entries) and it sails at 61 100 (recorded). Calling the ship nearest to the harbour's own field (96085, distance 2 against 3) sends 96085 to the harbour (two route draws at 60 998). A ship sailing to this harbour when it is called (the Hard game, ship 94308 at counter 55 113) keeps its route. Experiment build: the Weak generator 60 999 -> 61 931 (with T31 and ECO-OUT T29 on); Hard 150 000 58 115 unchanged (calling with a new route there moves it to 55 114) |
| T33 | 5.2a 'no material was missing' (E-ship-026): E-trace-034 (the Weak game), the harbour (144, 47) gathering an expedition (builder set aside earlier; 6 stones needed, 4 set aside); the last two stones reach its stock at counters 61 729 and 61 808; its expedition passes at 61 728, 61 829, 61 930, 62 031 | the passes | 61 829: both stones are taken from the stock and set aside (recorded: the harbour's stock of stones 2 -> 0 at 61 829), the expedition stays in Gathering; 61 930: nothing to set aside, Getting ship; 62 031: the ship 94308 (lying at (147, 139)) is called (recorded: the ship has not moved by counter 62 000; the heap dump read over 61 999 .. 62 073 holds the expedition waiting for ship 94308 with 11 goods set aside). Going to Getting ship already at 61 829 calls the ship at 61 930 (the Weak generator's first difference there, 61 931). Experiment build: Weak generator 61 931 -> 63 127 (on top of T31, T32, ECO-OUT T29); Hard 150 000 (58 115) and the war (24 343) unchanged |
| T34 | 4.4 'which link' (Q-1095, E-ship-027): two harbours of player 0 on one sea, H1 on island A (a warehouse on its roads), H2 on island B; a site on B needs 4 planks and no warehouse on B has planks; one finished ship on the sea, nearer to H2 than to H1; H1's warehouse holds 1 plank at the start and gets 1 more after each unloading at H2 (4 in all) | the passes until the site has its planks | the ship's first search docks it at H2 (the nearer harbour); H1's first pass with a plank ready calls it from there (free, no transfer-call link, though docked at H2); it sails to H1, is loaded with 1 plank in a later pass, sails to H2 and unloads; this repeats: **4 deliveries**, each after a call of the ship lying idle at H2, and the site's 4 plank orders are served. With the docked link read as the call's link: **no delivery** (the ship lies at H2 and is never a candidate for H1). Experiment build: 4 deliveries (ticks 779, 1 284, 1 789, 2 395 of the test world, one every 5 passes), against 0 without the rule; the Weak generator stays at 63 127, Hard 150 000 at 58 115, the war at 25 290 / 26 590 / 26 132. The candidate test alone, with the call still resetting the ship (a new route), moves Hard 150 000 to 55 114: the ship 94308 sailing to (144, 47) there already holds its transfer-call link and must keep its route |
| T35 | 3.1 'other ships' (Q-064): a channel of Water one field wide, ships A and B of one player at its two ends, each sent to a docking place beyond the other's end; then the same with B lying still in the channel on A's route | the routes, steps and arrival ticks | each route is searched as on a sea without the other ship (the field test ignores it; the draws come from the shared generator in update order, as always); A and B pass through each other (both on one field in an update, or crossing between two fields) with no wait and no new route, each arriving after the same number of steps as alone; with B lying in the channel, A's route goes through B's field and A sails through it without stopping |
| T36 | 4.1 (a soldier request), 4.2, 4.3, `MIL-soldiers.md` MIL-SOL-4.7a: E-trace-037 (R5 again, the Weak game, every table 62 950 .. 63 300). Player 1's guard building 96956 on (144, 135), on the colony island of the harbour (147, 136) (no soldier in any warehouse there), files the soldier request 97664 at counter 62 957 | the requests and the harbour (144, 47) | the request becomes a soldier need of (147, 136), which asks (144, 47); at (144, 47)'s pass of counter 63 119 it orders one soldier (request 97689, linked to the harbour); the castle (139, 51) serves it: soldier 97690 stands on the door field point at progress 0 in counter 63 123 and steps from 63 124; the castle's door is blocked from then (door wait 28 in the memory picture of about counter 63 120, 18 at about 63 130), so the take-out pass of counter 63 126 skips it and makes 5 draws, not 6 (recorded). Without the attachment no soldier need arises and ours made 6 draws: the Weak pair's generator first differed at 63 127; with the rule 65 990; with the soldier moving in his creation tick the dense settlers differ at trace tick 63 124, with him standing 63 171 (experiment build; every other pair unchanged) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ship-001 | code reading | `S2DNG.exe` | navy update, ship state machine |
| E-ship-001 | code reading | | nearest harbour, docking reservation, reachability by sea |
| E-ship-001 | code reading | | making room, movement start, water route |
| E-ship-001 | code reading | | docking places (Village Landing Position) |
| E-ship-001 | code reading | | harbour passes: asking, gathering, calling and loading ships |
| E-ship-001 | code reading | | expedition preparation states |
| E-ship-001 | code reading | | landing options, go, cancel, build harbour |
| E-ship-001 | code reading | command registrations | the four commands of SHIP-8 |
| E-ship-001 | strings | debug name tables | state names |
| E-ship-002 | code reading and recordings | recording E-trace-018 `ships.csv.gz`, `generator.csv` (counters 50000 .. 50120), E-trace-013 the same (45800 .. 45995) | SHIP-3.1, T13 |
| E-ship-003 | code reading and recordings | recording E-trace-018 `objects.csv.gz` (helpers 96100 .. 96167, counters 50800 .. 51300) | SHIP-5.2, T14 |
| E-ship-004 | code reading | `tutorial04.s2m` continents and state layer (2026-11-05) | SHIP-6.1, 6.1a, T15 |
| E-ship-006 | code reading | (2026-11-16 night) | SHIP-4.1, T19 |
| E-ship-007 | code reading | (2026-11-16 day) | SHIP-4.1 retry and detach: the attach inside the 11-tick ware order pass; an attached order re-checked each pass (harbour accepts the good and is road-reachable, else detached) |
| E-ship-010 | code reading | (2026-11-17 night) | SHIP-4.1 the order's end: detached when it takes its ware, when cancelled, at the pass re-check, or when its harbour is torn down; T19a |
| E-ship-011, E-trace-013 | code reading, recording | (2026-11-19 night): the founding clears only the expedition's mode, target and options; R3's ships table from counter 50 658 | SHIP-7.2 the ship after the founding (T20) |
| E-ship-012, E-trace-013 | recording, experiment build | (2026-11-21 day): R3's ships table (every search run), buildings and settlers tables from counter 50 658 | SHIP-2.4 the returning ship's first try (T21, Q-1073), SHIP-7.2 the founded site levelled and its builder's walk (T22) |
| E-ship-013 | code reading | (session 2026-11-21 pm): the ship's update, its search state | SHIP-2.4 the search counter (Q-1074) |
| E-ship-014, E-trace-013 | code reading, recording, experiment build | (session 2026-11-21 pm): the builder's walk to his site (a road walk to the site's flag); R3's settlers table for builder 97268 | SHIP-7.2 the founded builder's stand on the flag (T22, Q-1075) |
| E-ship-015, E-trace-013 | recording, code reading, experiment build | (session 2026-11-21 pm): R3's settlers and objects tables (the builder requests of harbour 90366 from 52870) | SHIP-5.2 the requested builder's way (T23) |
| E-ship-016, E-trace-018, E-trace-013 | code reading, memory reads of the running game, recording, experiment build | (session 2026-11-21 eve): the harbour choice for a ship transport and the matching's call of it; the route store in R5's heap dumps of 54500 and 55000; the streets, wares and settlers tables of R5, counters 54670 .. 54726 | SHIP-4.1 the stored cost of the harbour choice (T24) |
| E-ship-017, E-trace-013 | code reading, recording, experiment build | (session 2026-11-21 eve): the ship's idle update, the docking reservation and the harbour's push-out of an idle ship; the Hard recording's ships table | SHIP-2.3 5, SHIP-2.4a (T25), Q-1074 |
| E-ship-018, E-trace-013 | code reading, memory reads of the running game, recording, experiment build | (session 2026-11-22 day): the harbour's pass, its asking, gathering and loading steps, the harbour's count for other users; the need entries, transfer orders and settler requests in R3's heap dumps of 54 000 and 54 500; the buildings, stocks and settlers tables around counter 54 100 .. 54 510 | SHIP-4 the passes' ticks, SHIP-4.2 a held good, SHIP-4.4 corrections (T26), Q-1079 |
| E-ship-019, E-trace-013 | code reading, recording, experiment build | (session 2026-11-22 pm): the harbour finder's tests (finished, road network of the carrier roads, stock settings, a docking place on a sea with a finished ship); the flags' network ids in the heap dumps of 54 000 and 54 500; the roads table's carrier of the road (145, 134)-(147, 135) | 4.1 the network (T27) |
| E-ship-020, E-trace-013 | code reading, recording, experiment build | (session 2026-11-22 night): the gathering step (what each transfer order wants), the place of the harbour's update in the tick; the recording's helpers, wares and animals around counter 54 406; the experiment build with both (T28) |
| E-ship-022 | code reading | | SHIP-4.8: the building teardown visits no ship; the harbour part deletes transfers, expedition, docks; ship states with gone links (Q-066) |
| E-ship-023 | code reading | (session 2026-11-24 c): the expedition's gathering pass, its builder branch (set aside from the stock, the cancel of the first open builder request, the request only with none open, Getting ship only from a pass that began with the builder set aside) | SHIP-5.2a, T29a (Q-1077 narrowed) |
| E-ship-024, E-trace-034 | recording, experiment build | (session 2026-11-24 night): the Weak game recorded again with every table from counter 59 800 to 62 000; the harbour's plank orders and wares, the castle's helpers 60 698 .. 60 832 | SHIP-4.3 'already on the way' (T30), Q-1084 |
| E-ship-025, E-trace-034 | code reading, recording, experiment build | (session 2026-11-25 a): the harbour's count of settlers on their way (requests of the job plus walking settlers); the call's ship choice per docking place; the Weak game 60 901 .. 61 931 | SHIP-4.3 settlers (T31), SHIP-4.4 'which ship' (T32) |
| E-ship-026, E-trace-034 | recording, heap dumps, experiment build | (session 2026-11-25 a): the expedition of (144, 47): its stones' stock 61 729 .. 61 829, its state and count in the dumps of 61 000, 61 500, 62 000; the ship 94308 not moving before 62 000 | SHIP-5.2a 'no material was missing' (T33) |
| E-ship-027 | code reading, experiment build | (session 2026-11-26 a): the transfer call's ship choice, the call and the release, the ship's links and its idle update; the clone's four-delivery test | SHIP-4.4 'which link' (T34), Q-1095 |
| E-ship-028 | code reading | (session 2026-11-26 a): the ship's mover (its next step, arrival, the object lists' move) | SHIP-3.1 'other ships' (T35), Q-064 |
| E-ship-030, E-trace-037 | recording, memory pictures, experiment build | (session 2026-11-26 night) | SHIP-4.1 a soldier request attached to a harbour; MIL-SOL-4.7a for the soldier sent (T36) |
| E-trace-037 | live read of the running game and pictures of its memory | the Weak game again to counter 63 300, every table from 62 950, memory pictures at 63 100, 63 110, 63 120, 63 130 | T36 |
| E-ship-008 | code reading | (2026-11-16 day) | SHIP-7.2: the order of the harbour founding: site, builder made on its field, the load into its store, its orders cancelled (Q-157) |
| E-ship-005 | code reading and map data | `tutorial04.s2m` continents and state layer (2026-11-16) | SHIP-2.5, 2.7, 6.4, 7.1, T16 to T18 |
| data | script | `ship.lua`, `buildings.lua` (harbor, shipYard), `dialogs.txt` dialog3_3 / dialog32_0, `tutorial.txt` T04_* | values and the player's view |

## Open questions

- Q-063: how a harbour's list of needed goods is filled (which requests on
  an island without the good become harbour needs, and when).
- Q-064: answered (SHIP-3.1 'other ships', T35).
- Q-065: answered (SHIP-2.3 'the order, read whole'; SHIP-4.4 'which link, exactly').
- Q-066: what happens to a ship's cargo and to ships docked at a harbour
  that is destroyed or captured; ships of a defeated player.
