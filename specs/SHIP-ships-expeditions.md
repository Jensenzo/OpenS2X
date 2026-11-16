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
- **SHIP-2.4 Search.** A counter runs from 0 to 100 (one try every **101
  ticks**). At a try: any harbour link is dropped; the nearest **finished**
  harbour of the ship's player that is reachable by sea from the ship's field
  (straight-line distance, strictly smaller wins, so the first in the harbour
  list on ties) gets a docking place reserved and the ship goes **to
  harbour**. If there is none and the ship stands on a LandingPosition field,
  it goes **making room** (SHIP-3.3); otherwise it stays searching. A ship
  coming back from a cancelled expedition starts with the counter at 100, so
  it tries in its next update. Confidence: high.
- **SHIP-2.5 To harbour.** The ship sails to the docking place's water field.
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
  draws (code and T13); not traced: what happens when another ship blocks the
  route or the target (Q-064).
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

- **SHIP-4.1 Needs.** A harbour keeps a list of goods its island needs from
  overseas. **How entries get there (2026-11-16 night, Q-063 partly answered;
  E-ship-006, code reading):** an open request is attached to one harbour when
  the land cannot serve it.
  - A building's **ware order** that finds no warehouse of its player holding
    the good and reachable over the roads (the distribution's warehouse search,
    `ECO-requests-dispatch.md`) is attached, if it is not attached yet.
  - A **settler request** that finds no warehouse able to send the settler (and
    none that can make one from a helper and a tool) is attached the same way.
  - **The harbour chosen** is among the player's finished harbours that are
    reachable over the roads from the requester's flag, accept the good (its
    stock settings, `ECO-warehouse-construction.md`), and have a docking place
    on a sea where the player has a finished ship. The one with the lowest
    road route cost from the requester's flag wins; on equal cost, the first in
    the world's harbour list. With no such harbour the request stays
    unattached and is tried again at its next turn.
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
    assigned and the order is used up (medium: the order's end was not followed, so the harbour's
    need may count it until then).
- **SHIP-4.2 Asking another harbour.** For each needed good with no supplier
  yet: among the player's other **finished** harbours, in harbour list
  order, the first one that (a) has not already been asked for this good by
  this harbour, (b) does not itself need this good, and (c) can supply it
  (it accepts the good by its warehouse settings and a warehouse on its own
  road network has the good or the good is in its stock; for soldiers also
  by rank) gets a **transfer order** (good, destination = the asking
  harbour). A need whose supplier can no longer supply it is cancelled and
  re-asked later. Confidence: medium.
- **SHIP-4.3 Gathering.** For each good in its transfer orders, a supplying
  harbour sums the amounts all orders want and orders the shortfall (wanted
  minus what is in stock for transfer minus what is already on the way) into
  itself like any warehouse order: wares by a normal ware order
  (`ECO-distribution.md`), settlers by a settler request. Confidence: medium.
- **SHIP-4.4 Calling and loading a ship.** For each transfer order:
  - if the harbour has **no ship called** and the order's goods are ready in
    stock: the nearest free ship (finished, not on an expedition, not moving,
    no assignment, empty) that is on a sea which also reaches one of the
    destination harbour's docking places is **called**; one ship per pass;
  - if the called ship is **docked here** and holds fewer than the ship
    stock limit (12, `sh_setStockLimit`) cargo entries: the order's goods are
    moved from the stock into the ship as a cargo entry for the destination
    harbour (soldiers split by rank, lowest first, at most 5 ranks).
  After the pass, if something was loaded, or a ship is called but nothing
  could be served, the ship is released to sail (SHIP-4.5). Confidence:
  medium.
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
| E-ship-008 | code reading | (2026-11-16 day) | SHIP-7.2: the order of the harbour founding: site, builder made on its field, the load into its store, its orders cancelled (Q-157) |
| E-ship-005 | code reading and map data | `tutorial04.s2m` continents and state layer (2026-11-16) | SHIP-2.5, 2.7, 6.4, 7.1, T16 to T18 |
| data | script | `ship.lua`, `buildings.lua` (harbor, shipYard), `dialogs.txt` dialog3_3 / dialog32_0, `tutorial.txt` T04_* | values and the player's view |

## Open questions

- Q-063: how a harbour's list of needed goods is filled (which requests on
  an island without the good become harbour needs, and when).
- Q-064: the ship route search: cost per water step, tie-breaking, and what
  happens when the route is blocked by another ship.
- Q-065: the exact order of the Idle cases (SHIP-2.3) and the meaning of the
  ship's four harbour links (from offsets; medium).
- Q-066: what happens to a ship's cargo and to ships docked at a harbour
  that is destroyed or captured; ships of a defeated player.
