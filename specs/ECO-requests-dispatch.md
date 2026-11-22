# ECO-REQ: Settler requests, ware orders and warehouse dispatch

Status: draft (who requests, queue processing, warehouse choice, send cooldown, order filing and ware matching read from the binary; some branches marked as not traced)
Owner question(s): Q-038, Q-101, Q-004 (part) in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Roads, construction sites and buildings file **requests** for settlers
(carrier, donkey, leveller, builder, worker) into one queue per player. Each
tick the game tries up to four of them in turn. A request is served by the
nearest warehouse (by road route cost) that has that settler, or that can make
one from a helper and the right tool. A warehouse sends at most one settler or
ware every 30 ticks. Construction sites and workshops file **orders** for wares:
a site orders all its materials once levelling is done; a workshop keeps
stock plus orders at its stock limit (6) and re-orders every unit it uses.
Every 11 ticks orders are matched to the nearest free ware or warehouse stock;
every 5 ticks wares nobody ordered are sent to the nearest warehouse.

## Rules

Ticks are 0.1 s. "Route cost" between two flags is the cost of the cheapest
route over the road network as used for ware routing (`TRN-carriers-transport.md`
TRN-CARRY-5); only flags of the same network (land, or waterway for boats) can
be connected. Distances are field distances (MAP-GRID-2.1).

### ECO-REQ-0 Order within a tick

- **ECO-REQ-0.1** Of the per-tick updates, these run in this order: the
  buildings of **all players in one pass** (each building's update, including
  construction and the timers below), then the request queue, **one for all
  players** (ECO-REQ-2.1); later the roads (ECO-REQ-1.1); later the settlers' and
  resources' updates. So a request filed by a building can be served in the same
  tick; a request filed by a road is served at the earliest in the next tick.
  **Corrected 2026-11-01 night** (E-eco-042; T39): there is no
  per-player pass. The pass goes through the game's one building list, which
  holds every player's buildings and sites in creation order (a new one at the
  end, a removed one taken out; E-trace-019 records the list in object-number
  order across the four players). Confidence: high for the single pass and the
  single queue (code and T39); high for the creation order within the pass
  (code, recording and T40: two requests filed in one tick by two players'
  buildings take their places in the queue in the order of the buildings'
  creation, whatever the players).
- **ECO-REQ-0.2** Full order of one logic tick (E-mil-001): raise
  the tick counter `t` (a new map stores -1, so the first tick is `t = 0`);
  the players' queued commands; two global updates (not traced); all
  buildings of all players (each building's update, in the list order of 0.1)
  and then the request queue (one for all players, 2.1); roads
  (ECO-REQ-1.1); one further global update (not traced); settlers (**six lists
  updated one after the other: workers, constructors, carriers and helpers
  (with donkeys and boat carriers), levellers, soldiers, specialists**; each list in
  its order, a settler that leaves the world taken out where it stands, keeping
  the order, one added during the pass first updated in the next tick; E-trn-013, added 2026-10-25 evening: so a worker who waits for
  inputs sees a ware a carrier hands over in the same tick only in the next one,
  measured: the sawmill's worker 90431 of E-trace-010 starts its first work in
  counter 3168, the carrier 90234 having handed the wood over in 3167); the
  transport jobs (ECO-REQ-5.1, by `t`); resources (deposits, trees,
  animals); doodads; the military update (coin and promotion,
  `MIL-coins-promotion.md`); then triggers/scripts, productivity decay and
  two more updates (not traced). Confidence: high for the order of the
  calls; the untraced items are named by their position only.
- **ECO-REQ-0.2a Which of the six lists a new settler joins (answered 2026-11-11 eve, Q-287;
  E-eco-044).** The list is the settler's class, chosen when the settler is made at a warehouse's
  door, by one of two rules:
  1. **Sent for a request: by the request's kind**, not by the profession: a building's worker
     request makes a worker (workers' list) whatever the profession, so the well's helper of
     E-trace-012 is a worker; a road's carrier request (helper, donkey or boat carrier,
     ECO-REQ-1.1) makes a carrier; a builder request a constructor, a leveller request a
     leveller, a specialist order (geologist, scout) a specialist, a soldier request a soldier.
  2. **Sent without a request: by the profession.** A settler taken out of stock without a job
     (ECO-DIST-1.7a: moved to another warehouse or a harbour) and every settler walking out of a
     burning warehouse (MIL-CON; every settler-type stock entry but the helpers, which are lost
     with the wares): `good_constructor` (builder) -> constructors; `good_bulldozer` (leveller)
     -> levellers; `good_helper` and `good_donkey` -> carriers; `good_explorer` (scout) and
     `good_scholar` (geologist) -> specialists; the five soldier ranks -> soldiers; **every other
     profession -> workers**.
  A new donkey from a donkey farm (`ECO-food-mining.md` ECO-FOOD-6.3) is made as a carrier. A
  settler is appended to its list's end when it is made, so each list stays in object-number
  order. Confidence: high for 2 and the appends (code read: the door's sending by profession, the
  six list appends, the burning warehouse's loop); high for 1's switch on the request kind, medium
  for which request kind each road request has (the boat carrier's taken from the request's
  warehouse choice, which treats it as a road carrier).

### ECO-REQ-1 Who files settler requests

- **ECO-REQ-1.1 Roads**, every tick, for every road whose two end flags exist:
  - land road with no carrier and no open request: request a **helper**
    (`good_helper`) as its carrier;
  - else, land road that wants a donkey (traffic rule, TRN-CARRY donkey
    trigger), has no donkey and no open request: request a **donkey**;
  - waterway with no boat carrier and no open request: request a **boat
    helper** (`good_shipHelper`).
  A road has at most one open request at a time.
- **ECO-REQ-1.2 Leveller.** Each tick, a construction site of size medium,
  large or harbour whose levelling is not complete, that has no leveller and
  whose six neighbours do not all have the site field's height, requests a
  **leveller** (`good_bulldozer`) unless it already has an open leveller
  request. A site that needs no levelling (small, mine, or already flat) is
  marked levelled at once (`ECO-warehouse-construction.md` ECO-HQ-4.2) and goes
  on to ECO-REQ-4.1.
- **ECO-REQ-1.3 Builder.** Each tick, a site whose levelling is complete, with
  building work remaining and no builder, requests a **builder**
  (`good_constructor`) unless it already has an open builder request. The
  builder is requested as soon as levelling is done, in parallel with the
  material orders (ECO-REQ-4.1). **"As soon as" is the site's next update
  after the levelling is complete** (2026-10-25 evening, measured): the leveller
  is a settler, updated after the buildings and the request queues (ECO-REQ-0.2),
  so a levelling his last step completes in tick T is seen by the site's update
  in T + 1, which files the builder request, and the request
  queue right after serves it (the material orders are filed already in T,
  ECO-REQ-4.1, T29): the builder appears at the warehouse door in
  T + 1 (E-trace-010: the harbour site (144, 47) levelled in trace tick 2312, its
  builder 90415 out of the castle in 2313; (137, 55) levelled 4063, builder 4064).
  A site that needs no leveller is levelled by its own first update and files in
  that same update (site (141, 46) made in 5389, levelled and its builder out in
  5390). An experiment build that let the site's update see the leveller's last
  step at once sent the builder in 2312 (dense settlers 2312 -> 2347, buildings
  2962 -> 3182 when corrected).
- **ECO-REQ-1.4 Worker.** A finished building has a timer that fires once every
  101 ticks (counter 0..100, as ECO-OUT-2.1), counted only while the building
  is finished. When it fires and the building's number of workers differs from
  its worker limit (`bp_setWorkerLimit`, default 1; 0 for headquarters,
  warehouse, military buildings and harbour) and it has no open worker request,
  it requests one settler of its worker profession (`bp_setWorker`). **The
  first worker request is made 101 ticks after the building is finished.**
- **ECO-REQ-1.5** Military buildings request soldiers on their own timer (every
  21 ticks) up to their soldier limit; not specified here.

### ECO-REQ-2 The request queue

- **ECO-REQ-2.1** **One queue holds the settler requests of all players**, in
  filing order; new requests are added at the end. (Corrected 2026-11-01 night,
  T39: "one queue per player" was the earlier reading. With a single active
  player the two readings agree, so only a game with several computer players
  shows it.)
- **ECO-REQ-2.2** Each tick the queue is processed from a cursor that persists
  between ticks: up to **4 requests** (all of them if fewer), each in turn; the
  cursor wraps to the start when it passes the end. The 4 are counted over the
  one queue, whatever players the requests belong to: with 10 requests open, a
  player's request is tried about every third tick, not every tick.
- **ECO-REQ-2.3** A request whose requester (road, site or building) no longer
  exists is deleted.
- **ECO-REQ-2.3a** The same holds for a request whose link to its requester
  was cancelled (`MIL-soldiers.md` MIL-SOL-4.4 step 1): it stays in the queue,
  unlinked, until its turn comes and is then deleted unserved (E-trace-011:
  builder request 90823, cancelled in counter 6320, gone in 6321; MIL T18).
  **The carrier and donkey requests of a road that is removed** (`TRN-roads-flags.md`
  TRN-ROAD-6.1), **also by a split** (TRN-ROAD-4.2: the old road goes, its
  halves are new roads that file their own requests), are cancelled the same way:
  they stay in the queue, unlinked, in their places, until their turn, and so
  still count for the queue's length and the cursor's rotation (added
  2026-10-30, measured T38; deleting them at the removal shifts every later
  turn of the queue by one).
  Confidence: high for the deletion on the next turn. Medium (code only,
  E-mil-006) for the bookkeeping: the deletion takes one of the
  tick's 4 turns, and the cursor does not move, so the next request takes
  its place.
- **ECO-REQ-2.4** A request is tried (ECO-REQ-3). If it is served it is deleted;
  otherwise it stays and the cursor moves past it.
- **ECO-REQ-2.5** Within one tick, a request is not tried (and stays) if the
  request tried just before it (of any player) failed and had the same
  requester and the same profession.
  **Corrected 2026-10-29 (E-eco-038; measured T36):** the
  key is not the requester. It is the profession together with **the network of
  the requester's flag**: the land road network (the boat network for a ship
  helper), as identified by the flag's network number (`SIM-game-start.md`
  SIM-START-4.3). For a building or site that is the flag of the building; for a
  road, one of its flags (both are in the same network). So after a failed
  request, every later request of the tick for the same profession whose flag is
  in the same network is skipped as well, even for another requester. A request
  whose flag is not connected yet has a network of its own and is tried. The key
  is remembered from the last **failed** try of the tick: a success in between
  does not clear it (code; medium, not separately measured). A request linked
  through the request's second reference is keyed by that object itself; which
  requests use that reference is not traced (soldier requests are a guess).
  Effect: while the warehouse chosen for one carrier request is cooling down,
  the carrier requests of other roads behind it in the same network wait too,
  although another warehouse could serve them.

### ECO-REQ-3 Serving a settler request

- **ECO-REQ-3.1** Destination: the requester's flag (for a building or site,
  the flag on its south-east neighbour; for a road, its **first flag**, the
  flag at entry 0 of its field list, TRN-CARRY-1.1).
- **ECO-REQ-3.2 Warehouse with stock (A).** Among the player's warehouses
  (headquarters, depots, harbours) that are finished and whose flag is in the
  same network as the destination flag, the one with at least one settler of
  the requested profession in stock and the **lowest route cost** to the
  destination flag. Ties: the warehouse that comes first in the player's list
  (order of creation). The costs are stored costs (`TRN-carriers-transport.md`
  TRN-CARRY-5.3a) of the land network (kind 2; for the boat carrier the
  all-roads network, kind 0), from the destination flag to the warehouse's flag
  (2026-10-25 evening, E-ai-017; medium, code). In these
  networks every road costs its length L: no 3L for a road without a free
  carrier, no 4L for a full flag (`TRN-carriers-transport.md` TRN-CARRY-5.3;
  corrected 2026-10-27 night, E-trn-017, measured T18).
- **ECO-REQ-3.3 Warehouse that can make one (B).** If the profession has a cost
  list (`gp_addCost` in `Jobs.lua`: one helper plus one tool, for example
  woodcutter = helper + axe, builder = hammer + helper, leveller = helper +
  shovel), the warehouse, chosen as in ECO-REQ-3.2, that has every good of the
  list in stock.
- **ECO-REQ-3.4 Choice.** If only one of A and B exists, it is chosen. If both
  exist and differ, B is chosen only if it is strictly nearer (field distance)
  to the destination than A; otherwise A. **The distance is measured from each
  warehouse's own field** (the building's position, not its flag) to the
  destination flag (corrected 2026-10-30, T37; E-eco-039:
  measured from the flags, the castle would win in T37 by 20 to 21).
  Confidence: high (code; measured T37).
- **ECO-REQ-3.5** The chosen warehouse sends only if its **send cooldown** is 0
  and it can still provide the settler (has one, or can make one). Otherwise
  the request fails this time.
- **ECO-REQ-3.5a Counting.** A settler counts for its requester **from the
  moment it is sent**: a worker is linked into one of the building's worker
  places (up to 10) and the building's worker count goes up at once; a builder
  or leveller is assigned to the site at once. So the checks of 1.2 to 1.4
  ("no leveller", "no builder", worker count below the limit) see settlers
  still on their way. (The building's inputs, ECO-REQ-4.2, wait until a worker
  stands at the building's door position.)
- **ECO-REQ-3.6 Sending.** If the warehouse has none of the profession in
  stock, it first makes one: one of each good in the profession's cost list is
  removed from its stock and one settler of the profession added. Then one is
  removed from stock, the settler appears at the warehouse's door, and the send
  cooldown is set to **30**. The settler is assigned to the requester: worker
  to its building, builder or leveller to its site, helper / donkey / boat
  helper to its road, soldier to its military building. (Profession table, where
  helpers and donkeys come from: `ECO-professions.md` ECO-PROF.)
- **ECO-REQ-3.7 Cooldown.** The send cooldown falls by 1 per tick while the
  warehouse is finished (ECO-REQ-0.1: before the queue is processed). So a
  warehouse that sends in tick N can send again in tick N + 30. Wares sent out
  of a warehouse use the same cooldown (ECO-REQ-5.4).
- **ECO-REQ-3.8 Nothing available.** If no warehouse can serve the request it
  stays in the queue and is tried again on later turns. For a building's worker
  the player gets the message "Building without worker." A message is not
  added again while the player already has a message of the same type and text
  for the same field; instead that message's timer is reset to 600 timer
  units (`UI-ingame.md` UI-GAME-11.2: seconds of frame time, 36 000 ticks at
  the fixed 1/60 s per tick; Q-222) (so an unserved request keeps one message alive rather than repeating it). Further fallbacks (re-using a settler already out in the
  world, the scholar and explorer cases) are not traced.
- **ECO-REQ-3.9** The sent settler steps out to the warehouse's flag and walks
  along roads to its destination's flag (`TRN-free-walk.md` TRN-FREE-6);
  `ECO-warehouse-construction.md` covers levelling and building once there.

### ECO-REQ-4 Who files ware orders

An order is a request for one unit of a good, delivered to a building's flag
and handed into the building.

- **ECO-REQ-4.1 Construction site.** When levelling is complete (at once for
  sites that need none), the site files **one order per material unit, all at
  once**, **in the leveller's last step** (the settlers' part of tick T, so the
  matching of T, ECO-REQ-5.1, already sees them; the builder request waits for
  the site's update in T + 1, 1.3; added 2026-10-28 night, measured T29): every frame material and every main material (sawmill: 2 planks and 2
  stones = 4 orders). So no material is sent to a site that is still being
  levelled. A building type whose construction time (`bp_setConstructionTime`,
  default 500, only the sign matters) is 0 or less is completed at once
  instead (the headquarters).
- **ECO-REQ-4.2 Workshop inputs.** Once a finished building is active (for a
  building with a worker: its worker has arrived), it files orders once for each
  input good: the inputs are the cost list of its product good (`gp_addCost` in
  `Goods.lua`; plank needs wood, bread needs water and flour). For each input:
  orders = stock limit (`bp_setStockLimit`, 6 for workshops and mines; unset =
  none) minus the amount in stock minus the orders already open. Mines order
  fish, meat and bread, 2 of each, **filed in that order**: fish, fish, meat,
  meat, bread, bread (not the consumption order fish, bread, meat of 4.3).
  The filing order decides whose group comes first in the player's order
  list (`ECO-distribution.md` ECO-DIST-3.1a) when a good gets its first
  order, and so which of two foods of equal rank a warehouse serves first
  (T30). Confidence: high (E-eco-035; measured, T30).
  **When** (2026-10-27 b, T25): the orders are filed by the building's own
  update, in the buildings part of a tick (`ECO-time-units.md` ECO-TIME-1.5:
  buildings before settlers before transport), the first time it finds itself
  active with inputs to order. A worker arrives in the settlers part, so his
  building files its orders in the **tick after** his arrival tick; the
  matching of the arrival tick (ECO-REQ-5.1, transport part) does not see them
  yet. Confidence: high (code E-eco-030;
  measured T25).
- **ECO-REQ-4.3 Re-ordering.** Each time the building produces (ECO-WORK-3.2),
  every input unit it consumes is ordered again at once: one order per unit. A
  mine consumes one food unit, the first available of fish, bread, meat, and
  re-orders that one. So stock + open orders stays at the stock limit.
- **ECO-REQ-4.4** Military buildings order coins on their own timer; the
  catapult orders stones. Not specified here.

### ECO-REQ-5 Matching orders to wares (ware destination)

- **ECO-REQ-5.1** A transport job runs on the logic tick number `t`
  (`ECO-time-units.md`): order matching when `t mod 11 = 0`; storage of
  unclaimed wares when `t mod 5 = 0`; two further jobs at `t mod 3 = 0`
  (re-check of wares lying at flags, `ECO-distribution.md` ECO-DIST-1.6) and
  `t mod 7 = 0` (taking wares out of warehouses, ECO-DIST-1.7). In a tick where several
  are due they run in this order: matching (11), re-check (3), storage (5), taking out
  (7). Confidence: high (E-sim-007).
- **ECO-REQ-5.2** Matching goes through the orders player by player (0 to 7);
  within a player, grouped by good, goods in transport priority order, and
  per good once per road network; which order is served follows the
  distribution table or the longest-waiting rule (`ECO-distribution.md`
  ECO-DIST-3).
- **ECO-REQ-5.3** For an order of good G for building D, let H be the warehouse
  in D's network with G in stock and the lowest route cost to D's flag (as
  ECO-REQ-3.2), if any. Every route cost of 5.3 is a stored cost of the ware
  network (`TRN-carriers-transport.md` TRN-CARRY-5.3a; 2026-10-25 evening,
  E-ai-017): H's is the cost from D's flag to the warehouse's flag, and (a)
  compares the stored cost from D's flag to Wf's flag with the stored cost of
  the pair (H's flag, D's flag), the same entry as H's. So a cost stored
  while a flag on the way was not full or a road's carrier was free is used
  until the next ware renumbering (measured through the computer player,
  `AI-construction.md` T37).
  - **(a)** If G is not coins and a **free ware** of G lies in D's network:
    take the free ware Wf whose flag is nearest (field distance) to **D's
    building field** (not its flag; corrected 2026-10-28 night, E-eco-033: the distance runs from the building's position to the ware's
    flag) and is not D's own flag, among the free wares lying at a flag of D's
    ware network; the first of the good's free wares in their list (5.4)
    wins a tie (strictly nearer replaces; measured T28, the list's order
    measured T31 .. T33, Q-373). If there is no H, or the route cost from Wf's flag to
    D's flag is not higher than H's, Wf gets destination D. Otherwise H sends G
    as in (b).
    **No H: no cost is asked (2026-11-21 eve, E-eco-052: code reading and an experiment build;
    high).** When D's network has no warehouse with G in stock, Wf gets destination D at once and
    neither stored cost of (a) is asked, so nothing is stored for the pair (D's flag, Wf's flag)
    then. The two stored costs are asked only when an H exists. A later first ask of that pair (by
    this matching with an H, by the economy's other searches or by the computer player's route
    optimizer, `AI-construction.md` AI-CON-8.3 c) stores the cost of the roads as they are at that
    later tick (T44).
  - **(b)** Otherwise H sends one G if: H's send cooldown is 0, H can provide a
    helper to carry it out, and H's flag holds fewer wares than the flag limit
    (`cfg_setFlagPackageLimit`, 6). Then, as for sending a settler (3.6), a
    helper is taken from H's stock (or made, 3.3) and appears at H's door, H's
    cooldown becomes 30, one G leaves H's stock and is given to that helper
    with destination D. The helper carries it out to H's flag, puts it there
    and goes back in (TRN-FREE-6.4). The ticks until the ware lies at the
    flag are not traced (Q-050).
  - If neither works the order stays and is tried again 11 ticks later.
- **ECO-REQ-5.3d Which order, chosen when (Q-364).** The matching handles a
  good once per network through that network's representative order R (the
  first of the good's orders found in it, `ECO-distribution.md` ECO-DIST-3.2).
  The order D that 5.3 serves is **chosen** (ECO-DIST-3.3) only at the points
  below, among the good's orders in R's network; every choice marks the chosen
  order as just chosen (its stamp becomes the current tick, ECO-DIST-3.3b),
  whether or not it is then served:
  1. If G is not coins and a free ware of G lies at any flag of R's network
     (D's own flag included): choose D1. If a free ware Wf for D1 exists as in
     (a) (nearest to D1's building field, not on its flag), (a) runs for D1 with H found for
     D1's flag: Wf gets destination D1, or H sends G for D1 when its cost is
     lower and H's cooldown is 0, it has a helper and its flag has room (else
     nothing is sent). Either way the good is done in this network for this
     run. If no such Wf exists, go on with 2 (D1 stays marked).
  2. Find H for R's flag. **No H:** choose an order (it is marked; nothing is
     sent). **H with cooldown 0, a helper and fewer than 6 wares counted at
     its flag** (TRN-CARRY-4.3): choose D2 and H sends G for D2 as in (b), when
     the warehouse nearest to D2's flag passes the same three tests (with one
     warehouse, always). **Otherwise** (cooldown running, no helper, flag full)
     no order is chosen and none is marked.
  So an order chosen but not served goes behind the good's other orders in
  its network, and the next choice (in 2 of the same run, or in a later run)
  takes the next one; while H's cooldown runs, the choice stays where it is
  unless a free ware lies in the network. The warehouse nearest to D2 is looked
  for among the warehouses other than D2's own building. Confidence:
  high (code E-eco-019;
  measured, E-trace-010 counter 4939, ECO-DIST T21; H, not the warehouse nearest
  to D2, sends: measured T34). Not specified: the harbour step after a choice
  with no H (ship transport, Q-198).
- **ECO-REQ-5.3f A served order comes back at the delivery.** An order that is
  served (its ware is given the building as destination by 5.3 (a), or a
  warehouse's helper brings it, (b)) leaves the open orders but stays filed with
  its ware. When that ware is **delivered** into the building, the order is open
  again: it joins the **end** of its good's open orders (the list of
  `ECO-distribution.md` ECO-DIST-3.1) with the **stamp it got when it was
  chosen** (ECO-DIST-3.3b). Then the building drops one open order of that good:
  the one of its orders that was **made first** (orders keep the order in which
  they were made; a served order keeps its place in that order). So when the
  served order is the building's oldest, it is the one dropped and the open
  orders are as before the delivery; when the building has an older open order,
  that one is dropped and the served order stays open, with its newer stamp and
  at the end of the list. Delivered means put into the building by the carrier,
  or handed over by the matching when the ware lies at the building's own flag.
  Confidence: high (added 2026-10-27 night, E-eco-023: the order of the
  delivered ware filed again, appended; the building's first open order of the good in the
  transport's list of all orders, the made-first order, dropped; measured T20).
- **ECO-REQ-5.3h A destroyed ware's order comes back as it was** (added 2026-11-21 pm,
  E-eco-050). When a ware that serves an order is destroyed (in a released carrier's or a
  going-home helper's hands, or lying on a removed flag, `TRN-roads-flags.md` TRN-ROAD-6.3, 6.4;
  5.3g), the order filed with it is open again **as it was**: it joins the end of its good's
  open orders with the stamp it got when it was chosen (ECO-DIST-3.3b) and its place in the
  order the building's orders were made, exactly as the served order at a delivery (5.3f), but
  **no** order of the building is dropped (nothing was delivered). It is **not** a new order:
  a new order would have the stamp 0 and so be chosen before every order chosen before
  (ECO-DIST-3.3b). A carrier's claim on the ware is dropped with it. Confidence: high (code
  reading: the destruction looks up the served order that holds the ware and files that same
  record open again; measured T43).
- **ECO-REQ-5.3e A redirected free ware's next flag and claim.** When (a)
  gives a free ware a new destination (an order's building in place of
  storage or another order), its next flag (`TRN-carriers-transport.md`
  TRN-CARRY-5.4) is **cleared**, not computed at once, and a carrier's claim
  on it is **released at once, always** (whether or not the new route still
  goes over the claiming carrier's road; corrected 2026-10-27 b, Q-282: the
  claim was said to last until the carrier's arrival). The ware is then like
  any unclaimed lying ware: the next re-check (`ECO-distribution.md`
  ECO-DIST-1.6) computes its next flag (in the same tick when it is a re-check
  tick, t mod 3 = 0, the re-check running after the matching), and from then
  any carrier of the road to that flag may claim it (TRN-CARRY-2); until then
  no carrier takes it (TRN-CARRY-2.4 needs a next flag). The carrier whose
  claim was released does not notice: he walks on to the flag, finds no ware
  claimed by him there, **stands in the arrival tick** and chooses work again in
  the next tick (TRN-CARRY-2.5): the same ware when it is still unclaimed and
  its next flag is his road's other end (T17), else other work or his walk to
  his idle field (T21). The same release applies to a ware a helper had taken
  on for its building (the second kind of carrier link). A claiming carrier
  already **in his pick-up action** at the flag (the ware still lies there until
  the drop half ends, TRN-CARRY-3.1) is released too: when his drop half ends he
  takes the ware only if it is still linked to him, so he takes nothing, still plays
  the pick-up half and goes on as a carrier without a ware (TRN-CARRY-2.2); the
  ware stays at the flag for the carrier its re-checked next flag names, who
  claims and takes it normally (Q-285; E-eco-027; code,
  and measured for a plain pick-up in T26). The same holds for a ware bound
  for an **exchange** (TRN-CARRY-3.7): redirected while its carrier is in his
  put-down action at that flag, it is not taken in the pick-up half; he puts his
  own ware down, stands, and then chooses work as any carrier on his road's flag,
  which may take the same ware afresh (measured T24). Confidence: high
  (code E-eco-019: setting the new destination clears the next flag and its cache,
  the re-check skips claimed wares, only wares with
  a next flag are taken; E-eco-025: the free-ware branch of the matching unlinks the ware's carrier link, both
  sides, before the new destination is linked; measured: T17,
  E-trace-010 wood 92 (90758) at (137, 46), claimed by carrier 90399 from
  counter 5761, redirected to the sawmill (138, 49) by the matching of 5764,
  claim and next flag gone at 5764, next flag (139, 48) from the re-check of
  5766, 90399 on the flag standing in trace tick 5807, claiming it again at
  counter 5807 and picking it up; T21, E-trace-011 plank 91337 at 14520).
- **ECO-REQ-5.3c Helper timeline (Q-249).** Measured on the computer player's
  castle in E-trace-002 (logic ticks t = the recorder's counter; the trace tick
  is t + 1). The matching of 5.1 runs when t mod 11 = 0, and the helper of (b)
  exists at the warehouse's door field in that same tick t, standing (carrier
  state 1); it stands in t + 1 too, and its walk to the warehouse's field is
  ordered in t + 2 at progress 0 (advancing from t + 3; a builder or road
  carrier sent for a request walks in its first tick instead, with its first
  step already ordered). The walk to the flag is a chain of walks (ECO-WORK-3.8c):
  door -> building field (22 steps), building field -> flag (22 steps, ordered
  on arrival at progress 0). On the arrival tick at the flag (t + 48) the helper
  begins its put-down action at once (state 31, animation PutDown, 4 ticks), then a
  pick-up action (animation Pickup, 4 ticks: nothing is picked up when no ware
  waits for the way back), then stands 4 ticks in the going-home state (state 3),
  and in the 13th tick after arriving (t + 60) its walk back is ordered at
  progress 0; it reaches the building's field 23 ticks later and the door 23
  ticks after that (t + 106, state 2) and is gone in the next tick (at a **harbour**,
  whose door is its own field, the step from the field to the door is a walk with no
  step and still takes a tick, `TRN-free-walk.md` TRN-FREE-6.4c: on the field in his
  arrival tick, at the door, state 2, in the next, gone in the one after; added
  2026-10-27 b, T23). The ware lies
  at the flag from the tick the put-down ends (t + 52): an idle road carrier at
  the road's middle is given it in the next tick (t + 53) and its walk to the flag
  is ordered in the tick after (t + 54, ECO-TIME-1.5). The warehouse's cooldown
  of 30 (5.3 b) makes the next helper appear at the next multiple of 11 at or
  after t + 30, i.e. 33 ticks later while orders wait. Confidence: high (helpers
  90207 and 90209 of E-trace-002, T16); medium for the carrier being given the
  ware in t + 53 rather than t + 52 (the recorder sees only the carrier's walk).
  **The walk waits for the helper's entry check** (`TRN-carriers-transport.md`
  TRN-CARRY-4.4, with the ware's score 0): its first check, in t + 1, raises the
  flag's bar to 0, and the check of t + 2 admits the ware (its place at the flag
  counts from then, TRN-CARRY-4.2a, 4.3) and orders the walk. When a ware leaves
  the flag in t + 2 before the helper's check (a road carrier picking it up),
  the bar falls back to -1, that check only raises it again, and the ware is
  admitted and the walk ordered in t + 3; each further reset delays it a tick
  more (added 2026-10-27 night, measured T19). Confidence: high.
  **The bar already at 0** (added 2026-10-29, measured T35): the bar can be left at
  0 by another ware's check that was not followed by its admission, for example a
  road carrier walking in whose first check raised it and whose next check bound an
  exchange ware (TRN-CARRY-3.7, no bid). Then the helper's first check, in t + 1,
  admits the ware and resets the bar, and the walk is ordered **in t + 1**. There
  is no fixed two-tick wait at the door: the walk always starts in the tick of the
  admission. Confidence: high (T35, with the bar protocol of TRN-CARRY-4.4).
- **ECO-REQ-5.3g A helper whose warehouse goes while he is out (Q-284).** The
  helper of (b) / 5.3c checks only his warehouse, at three points, and never the
  order's building:
  - *At the door* (before his entry check admits the ware): when the warehouse's
    flag is gone, he goes home at once.
  - *Walking out* (door -> building field -> flag): the walk is not cut; on his
    arrival at the flag field, when the ware is still in his hands and a
    building still stands on the warehouse's field, he puts it down as in 5.3c;
    otherwise he goes home at once, without the put-down and pick-up actions.
  - *Putting down and picking up*: in each tick of these actions, when no
    building stands on the warehouse's field any more, he goes home at once.
  Going home is that of any settler whose building is gone (`TRN-free-walk.md`
  TRN-FREE-6; his warehouse if it still stands), and a ware still in his hands
  is destroyed when he starts (`TRN-roads-flags.md` TRN-ROAD-6.3, 6.4: its
  order is open again); it is never put down where no flag is. A flag removed
  while he walks out only takes the ware he holds off its ware list (the
  flag's lying wares are destroyed, TRN-ROAD-6.2; his is not lying). When only
  the order's building goes, the helper notices nothing: he puts the ware down
  on the warehouse's flag, without a destination, and 5.5 stores it. Confidence:
  medium (code, E-eco-026;
  no recording).
- **ECO-REQ-5.4** A free ware is a ware of the good's waiting list that
  **lies at a flag** at that moment (a ware stops lying at a flag in the tick a
  carrier's drop-time action ends, TRN-CARRY-3.1).
- **ECO-REQ-5.4a The free-ware list (Q-373, 2026-10-29; E-eco-037).** Each
  player has one waiting list per good, an ordered list. A ware is **appended at
  its end**:
  1. when it is put down at a flag without a destination and is not on the list
     yet (a worker's product that no order took, a ware whose building went away
     while it was carried), or made at a flag without one;
  2. when the order that holds it is dropped while it lies at a flag: by the
     pre-pass of 5.7 (a) and when the order's building goes away (5.5, last
     sentence). The pre-pass looks at the player's orders in the order they were
     made (the player's list of all orders, a served order keeping its place,
     ECO-REQ-5.3f) and collects the wares that lost their way; it then drops them
     **last collected first**, so in one pre-pass the wares join in the reverse
     order of their orders (T31, T32). A building that goes away drops its orders
     front to back (medium: read, not measured).
  A ware leaves the list when 5.3 (a) gives it an order, when it goes into a
  warehouse or is used, and when it is destroyed; the others keep their order.
  Getting a storage destination (5.5) does **not** take a ware off the list, and
  neither does a carrier picking it up: a ware on a storage trip keeps its place
  and is free again whenever it lies at a flag (it is how 5.6's storage trip is
  redirectable). Losing a storage destination (5.7 c) leaves the place unchanged.
  Not the order of creation (ware number) and not the order of joining a flag:
  a ware that joined the list early keeps its place over a later one wherever
  each lies (T33). Confidence: high (code E-eco-037; measured T31 .. T33).
- **ECO-REQ-5.5 Storage.** When `t mod 5 = 0`, each ware without a destination
  gets as destination the warehouse with the lowest route cost from its flag
  (the **stored** cost of the ware network, `TRN-carriers-transport.md`
  TRN-CARRY-5.3a: a pair asked before is not computed again, and a pair asked
  here is stored for the other users; 2026-10-25 evening, TRN-CARRY T26),
  among finished warehouses of its network that accept that good (warehouse
  good settings; ties: first in the player's list). If the ware already lies at
  that warehouse's flag it goes straight into the warehouse's stock, **also when
  a road carrier has claimed it and is on his way to fetch it** (clarified
  2026-10-28, measured T27, E-eco-007: the run takes in every lying
  ware without a destination, claimed or not); that carrier walks on to the
  flag, finds no ware there and walks back as for any claimed ware that is
  gone. A ware whose destination building has disappeared loses its destination
  and is handled by this rule again. **When it loses it (2026-11-17 day, E-eco-049: the war
  recording E-trace-019 and an experiment build; high):** in the tick the building is removed, as part
  of the removal: a ware lying at a flag for it is from then on a free ware without destination (on
  the free-ware list, 5.4a). **A carrier's claim on it stays** (the recording's row of the removal
  tick still names the claiming carrier; it goes only when the ware is given a new destination). So
  the next matching (5.3) can give it to another order of its good before any storage run looks at
  it; only a ware no order takes gets its warehouse here. Measured as: freed (destination, order and
  next flag cleared, put on the free-ware list, claim kept) at the transport step that follows the
  removal, before that step's matching and storage runs: the war moves and every Storm Coast pair is
  unchanged. Freeing it inside every removal and dropping the claim as well was measured wrong (the
  long Storm Coast pairs part from about trace tick 17 300 and 29 800). Confidence: high for the
  case measured (a building torn down by the computer player at the end of a tick); medium for a
  building removed in the middle of a tick (burnt, captured, land lost). Test T42.
- **ECO-REQ-5.6** So a ware put down by a producer (`ECO-worker-cycle.md`,
  `ECO-outdoor-workers.md`) either goes to a building that ordered it (if
  matched at the next multiple of 11 before or after it got a storage
  destination, the storage trip being redirectable) or to the nearest
  warehouse.
- **ECO-REQ-5.7 Wares that lost their way (E-sim-007, 2026-10-24).** "Network" in 5.2,
  5.3 and 5.5 is the ware network of `TRN-carriers-transport.md` TRN-CARRY-5.1a (flags
  joined by roads with an assigned carrier or donkey). A ware keeps a next flag
  (TRN-CARRY-5.4), computed when it arrives at a flag and again by the re-check of
  `ECO-distribution.md` ECO-DIST-1.6; a ware whose destination is not in its ware
  network has none. Three steps use it:
  - **(a) Before each matching** (`t mod 11 = 0`, first, over all players' orders): every order
    that has a ware assigned which no carrier holds (it lies at a flag) is looked at.
    If that ware has **no next flag**, the order loses it: the order is open again and
    the ware has no destination, lying at its flag as a free ware. The matching of 5.3
    follows in the same tick, so the order can get the same ware back only if it can
    reach it, or another ware, or a warehouse's stock. The orders are looked at in the
    order they were made and the lost wares released last first: that is the order in
    which they join the free-ware list (5.4a, T31).
  - **(b)** In the same step, a ware with a next flag that already lies at the flag of
    the building it is assigned to is put into that building at once (no carrier).
    Confidence: medium (read, not measured).
  - **(c) In the storage run** (`t mod 5 = 0`), before choosing destinations: a ware on
    a storage trip that no carrier holds and that has no next flag loses its storage
    destination; the choice of 5.5 is made for it **in the next storage run**,
    5 ticks later, not in the same one (corrected 2026-11-02 night, Q-384, T41;
    E-eco-043: in one pass over the storage list a ware that loses its
    destination is passed over, and only wares without a destination get one; a
    ware whose destination building is gone has no destination there and so gets
    its warehouse in the same pass, Q-296). A
    ware put down at the flag of the warehouse it is bound for, after that
    warehouse stopped taking the good, lies there with that destination and **no
    next flag** (lying at its own destination's flag counts as none) until this
    step (`TRN-carriers-transport.md` TRN-CARRY-3.3, T59).
  Wares in a carrier's hands are never touched by these steps; they are carried to the
  flag they were going to and checked there. Confidence: high for (a) and (c) as the
  only path the code has for such a ware and for the measured outcome (E-trace-006: a
  plank whose road went lost its order and was back in the castle at counter 400, its
  order was served again, `TRN-roads-flags.md` TRN-ROAD-6.6, T21); medium for the tick
  of (a) in that case (396, not recorded).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| requests tried per tick | 4 | requests | high | E-eco-007 |
| warehouse send cooldown | 30 | ticks | high | E-eco-007 |
| worker request timer | 101 | ticks | high (period), medium (first firing) | E-eco-007 |
| default worker limit | 1 | settlers | high | E-eco-007 |
| default stock limit | unset (-1) | | high | E-eco-007 |
| order matching / storage periods | t mod 11 / t mod 5 | ticks | high | E-eco-007 |
| mine food orders | 2 each of fish, meat, bread | | high | E-eco-007 |
| mine food consumption order | fish, bread, meat | | high | E-eco-007 |
| construction material orders | all at once, after levelling | | high | E-eco-007 |
| warehouse choice | lowest route cost, ties first created | | high (rule), medium (cost = ware route cost) | E-eco-007 |
| make-from-tool choice | B only if strictly nearer | | medium | E-eco-007 |
| free ware vs warehouse | free ware if its route cost <= warehouse's | | medium | E-eco-007 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | headquarters only, no other open requests; road built in tick N connecting two new flags to the HQ flag network | carrier | road files its helper request in tick N; the HQ sends a helper in tick N+1 |
| T2 | as T1 but two such roads built in tick N | carriers | first helper sent in tick N+1, second in tick N+31 |
| T3 | sawmill site placed on uneven ground, connected | order of events | leveller requested; **no** plank or stone leaves the HQ until levelling is complete; then builder request and 4 material orders at once |
| T4 | woodcutter site (small), connected | events | site levelled at once; builder request and 2 plank orders filed at once (frame 1 + main 1) |
| T5 | woodcutter hut finished at tick F, HQ has a woodcutter | worker | worker request at tick F+101 (medium); sent the same tick if the HQ cooldown is 0 |
| T6 | as T5 but HQ has no woodcutter, but an axe and a helper | worker | the HQ loses one axe and one helper and sends a woodcutter |
| T7 | as T6 with no axe | worker | not sent; "Building without worker."; request stays |
| T8 | sawmill finished, worker arrived, HQ has wood | orders | 6 wood ordered (stock limit 6, stock 0) |
| T9 | sawmill producing | after each plank | one new wood order |
| T10 | iron mine, all three foods in stock | one production | one fish consumed and one fish re-ordered |
| T11 | woodcutter puts a log at its flag; no building has ordered wood | storage | at the next tick with t mod 5 = 0 the log gets the nearest warehouse (route cost) as destination |
| T12 | sawmill with an open wood order; a log lies at a flag 2 fields from it; the HQ (with wood) is farther by route cost | matching at t mod 11 = 0 | the log gets the sawmill as destination; the HQ sends nothing |
| T13 | 6 open settler requests, warehouse cooldowns 0 | one tick | exactly 4 requests tried; the next tick continues with the 5th |
| T14 | two warehouses with builders at equal route cost | builder request | the one created first |
| T15 | HQ flag already holds 6 wares | ware order from the HQ | the HQ does not send; order retried 11 ticks later |
| T16 | E-trace-002, computer player's castle (door (139, 52), field (139, 51), flag (140, 50)), hunter site's plank orders, logic ticks | helper 90207 | matched at t 242 (242 mod 11 = 0): at the door in state 1 in 242 and 243; walk to (139, 51) at progress 0 in 244, 0.045 in 245; on (139, 51) with the next step at 0 in 267; on the flag in 290: PutDown 290 .. 293 (animation left 3.3, 2.3, 1.3, 0.3), Pickup 294 .. 297, standing in state 3 298 .. 301, walk back at 0 in 302, on (139, 51) in 325, at the door in state 2 in 348, gone in 349. Road carrier 90204 idle at (139, 49): walk to the flag at progress 0 in 296, 0.036 in 297. Second helper 90209: created 275 (= 242 + 33), flag 323, PutDown 323 .. 326, Pickup 327 .. 330, stand 331 .. 334, back 335, door 381 |
| T16a | ECO-REQ-5.7 (a): order O of a site D for a plank; its plank W lies at flag F, assigned to O; the only road between F and D's flag loses its carrier (released, no donkey) at tick 100; no other road with a carrier joins them | ticks 102 .. 115 | the re-check at 102 finds W without a next flag; the matching at 110 (110 mod 11 = 0) takes W from O (O open, W without destination) and finds no ware or stock for O; the storage run later in tick 110 (it runs after the matching, 5.1) sends W to the nearest accepting warehouse of F's ware network, or into that warehouse at once if F is its flag |
| T17 | 5.3e: E-trace-010, carrier 90399 of road (137, 46) .. (139, 48) idle on (138, 48); wood 92 lying at (137, 46) bound for storage (the castle) | the carrier's fetch from counter 5761 and the matching that redirects the wood to the sawmill 90211 (138, 49) while he walks | on the flag (137, 46) in trace tick 5807 he stands (no pick-up); the pick-up (drop-time action) starts in 5808, the walk with the wood in 5816 (measured). Keeping the next flag through the redirection makes him pick up in 5807 |
| T18 | ECO-REQ-3.2, TRN-CARRY-5.3: E-trace-011, player 1, counter 9983: the computer player's road (142, 43) .. (143, 45) ends on road 90725 (145, 46) .. (141, 45) (carrier 90371), whose split leaves the halves (145, 46) .. (143, 45) (L 4, keeps the carrier) and (141, 45) .. (143, 45) (L 7, no carrier); the castle's way to (141, 45): (141, 45) .. (141, 48) L 7 and (141, 48) .. (140, 50) L 8; the castle in its send cooldown (a builder sent at 9984) | the carrier requests of the split half (91065, destination (141, 45), filed first) and of the new road (91066) at counter 9985 | land-network costs: harbour 7 + 4 = 11, castle 7 + 8 = 15: the harbour sends carrier 91069 to the split half at 9985 (measured: streets table; the carrier walks from (144, 47) over (143, 45) to the half's middle (142, 45), standing there from 10125); 91066 waits for the harbour's cooldown and is served at 10015 (carrier 91073). Counting 3L for the carrier-less half (harbour 25) picks the castle, which cannot send: an experiment build then serves the new road first (settlers first differ at 10016; with plain L at 10162, generator 10032 -> 10193) |
| T19 | ECO-REQ-5.3c, TRN-CARRY-4.4: E-trace-011, player 1's castle (139, 51), flag (140, 50) holding 5 wares, one of them (91224) claimed by the carrier 90234 of the road to (139, 48) | helpers made at counters t = 11704, 11737, 11803, 11836 | the helpers of 11704, 11737 and 11836 have their ware's place from t + 2 (the recorded wares table: flag set) and walk from t + 2 (trace tick t + 3); 91224 leaves the flag in 11805 (picked up), so the helper 91237 of 11803 gets its place and walks only from 11806 (trace tick 11807). Admitting the helper's ware after a fixed two ticks (an experiment build) walks it at 11805: settlers first differ at 11807; with the entry check 12614 |
| T20 | ECO-REQ-5.3f: E-trace-011, player 1's guard house site 91022 (142, 44) with two plank orders (made in that order; stamps 10010 and 9999 in the heap dump of 10000); site 91075 (144, 56) with two (stamps 10296, 10285 in the dump of 11000) | the matching of 10175 chooses 91022's 9999 order (stamp 10175) and gives it the free plank 91092, delivered at 10635; the castle's helper of 11308 serves 91075's 10285 order (stamp 11308), its plank delivered at 11819 | dump 10500: 91022's open orders = only the one stamped 10010; dump 11000: only the served one, stamped 10175, at the end of the list (the older order dropped at the delivery); dump 11500: 91075's open order the one stamped 11528 (the served plank still on its way); dump 12000: the served order back, last, the other gone. With it the castle's wood goes to the sawmill 90211 at 12221, 12254 and 12496 and to 90734 at 12463, 12716, 12749, and plank 91267 to 91075 at 12452, as recorded (an experiment build that drops the served order at its choice sends the wood of 12221 to 90734 and that plank to the forester site 91116: dense trace buildings first differ at 13221; with the rule 13394) |
| T21 | ECO-REQ-5.3e (Q-282): E-trace-011, player 1, plank 91337 lying at (137, 46) (flag 90385) for storage (the castle 90180), next flag (139, 48); carrier 90399 of the road (137, 46) - (139, 48) claims it at counter 14511 and walks from (139, 48); carrier 91072 of the road (137, 46) - (136, 47) - (137, 48) idle on (136, 47) | the matching of counter 14520 (14520 mod 11 = 0, also a re-check tick) gives the plank to the site 91364 on (126, 64), whose route goes over (137, 48) | at 14520 the plank has no claim, its next flag is (137, 48) and its re-check counter is 0 (the re-check of the same tick computed it); 91072 claims it at counter 14528 (its 20-tick look) and walks from trace tick 14529; 90399 walks on, stands on (137, 46) in trace tick 14579 and walks back towards his idle field from 14580. Keeping the claim until 90399's arrival leaves 91072 idle (settlers first differ at 14529); with the rule an experiment build matches to 14661, and the dense 6000 pair (T17) stays equal |
| T22 | ECO-REQ-5.3g (Q-284): a warehouse helper with a plank for a site walks out; his warehouse and its flag are removed (conquest) while he is between the building field and the flag; the site is removed too | his walk and the following ticks | he walks on to the flag field (no flag there any more); in his arrival tick he goes home at once: no put-down or pick-up action, the plank is destroyed as he starts (not put on the field), and he walks to the nearest warehouse as any settler whose building is gone. With only the site removed (warehouse and flag stand), he puts the plank down on the flag as usual and it gets a warehouse by 5.5 |
| T23 | 5.3c: E-trace-011, player 1's harbour 90351 (144, 47) sends its one plank for the site 91705 at the matching of counter 18139 (helper 91725) | the helper's way back in | at the flag (145, 46) put-down from trace tick 18165, pick-up 18169, standing 18173 .. 18176, walk back from 18177, on the harbour's field 18200 (standing), at the door 18201 (state 2), gone from 18202. Going in from the field at once (an experiment build) removes him in 18201 (settlers, without the view columns, first differ at 18201; with the rule at 18460) |
| T24 | 5.3e (Q-285, an exchange): E-trace-011, player 1, carrier 91107 of the road (142, 55) - (144, 55) carries a ware to (144, 55) (flag 91077); stone 91739 lies there for the castle, next flag (142, 55), bound to him for the exchange from counter 18565; his put-down action at (144, 55) from trace tick 18611; the matching of counter 18612 gives the stone to the guard house 90785 | his actions at the flag | the claim is gone at 18612; his pick-up half (18615 .. 18618) takes nothing, he stands in 18619, claims the stone again at counter 18619 (next flag still (142, 55)) and takes it in a pick-up of its own (PutDown 18620, Pickup 18624), carrying from 18628. Taking the exchange ware regardless (an experiment build) walks back carrying from 18619 (settlers, view columns left out, first differ at 18620; with the rule at 18817) |
| T25 | 4.2 (when), 5.1: E-trace-013 (Hard game, every tick; the same game as E-trace-012), player 1's sawmill 90436 on (139, 55), finished long before, worker 90763 | the wood order and the free wood 90718 at (141, 53) (bound for the castle, next (140, 50)) | the worker stands on the sawmill's field in counter 5170 (arrival; his job state changes in 5171); the matching of 5170 (5170 mod 11 = 0) leaves the wood as it is; the matching of 5181 gives it to the sawmill (wares table: destination 90180 -> 90436, next flag (140, 54) in 5181) and carrier 90634 of (141, 53)-(140, 54) walks to fetch it in 5186 ((90634 + 5186) mod 20 = 0). Orders filed in the arrival tick make the matching of 5170 take it (carrier at 5180): experiment build Hard pair settlers 5190 -> 5599 with the next-tick filing; the other pairs unchanged |
| T26 | 5.3e (Q-285, a plain pick-up): E-trace-012 (player 1, Hard), carrier 90305 of the road (141, 53) - (140, 50) | his actions at (141, 53) | he claims stone 90870 (bound for the castle, next flag (140, 50)) at counter 6158 standing on the flag; his drop half 6159 .. 6162; the matching of 6160 gives the stone to the building 90786 (claim gone, next flag (142, 55) from 6162): he takes nothing, plays the pick-up half 6163 .. 6166, stands in 6167 and claims the plank 90888 at the castle's flag, walking there without a ware from 6168 (progress 0 in the trace tick 6168). Measured: this raises "what the empty carrier then does" to high for this case |
| T27 | 5.5 (a claimed ware at the warehouse's own flag): E-trace-013 / E-trace-012 (Hard), ware 91214 (good c5c945a3) lying at the castle's flag (140, 50) for the hunter's hut site 91149 on (142, 44), claimed from counter 8804 by carrier 90236 of the road (139, 48)-(140, 50), who walks to fetch it; the site is torn down at counter 8809 | the ware and the carrier | 8809: the ware has no destination; 8810 (t mod 5 = 0): it is gone (into the castle's stock) although claimed; the carrier walks on, stands on the flag in counter 8827 (trace tick 8828), walks back empty from 8828 (progress 0, moving from 8829). Our build (claimed wares left at the flag) let him pick it up. Experiment build (taken in claimed or not): his rows equal the recording; Hard pair settlers 8830 -> 9653, generator 9220 -> 9648, the other pairs unchanged |
| T28 | 5.3 (a), the distance: E-trace-013 (R3, the Hard game), the sawmill 90436 on (139, 55) (flag (140, 54)) orders wood; free woods for the castle: 91455 lying at (141, 53) since counter 10903 (bound to carrier 90305's exchange), 91469 at (141, 56) since 11077 (claimed by the fetching carrier 90332) | the matching of counter 11099 | from the building's field (139, 55): (141, 53) 3, (141, 56) 2: 91469 gets the sawmill (destination 90436 from 11101 in the wares table); 91455 stays for the castle and goes there in 90305's exchange (on the flag at 11115). From the flag (140, 54) both are 2 and the first wins (91455): an experiment build then undoes 90305's exchange; the Hard pair's settlers first differ at 11102, with the rule at 11819 (generator 11215 -> 11824). E-trace-011 (the Weak game) at counter 8261: the guard house's site 90785 on (131, 59) and the free stones 90934 ((141, 56), put down at 8217) and 90944 ((140, 54), at 8244): 11 from the building's field for both, the first (90934) gets it (measured), as with the old reading |
| T29 | 4.1 (when the orders are filed): E-trace-014 (R4, the Weak game), the smelter's site 92123 on (131, 61), its levelling completed by the leveller's step of counter 23914 (frame from trace tick 23915); a free stone 91934 lies at (137, 50) for the castle, claimed by carrier 90956 who has started picking it up | the matching of 23914 (23914 mod 11 = 0) | it gives 91934 to the site (destination 92123 from 23914 in the wares table); 90956's take finds no claim (5.3e), he stands at 23920 and takes the wood 92119 instead (pick-up 23921 .. 23928, walk from 23929) (measured). Filing the orders in the site's update of 23915 (an experiment build) leaves the order to the matching of 23925 and 90956 carries the stone: the Weak pair's settlers first differ at 23921, with the rule at 24351, buildings 24978 -> 25678, generator 24256 -> 24361; the Hard pair and the Weak pairs to 20 000 unchanged |
| T31 | 5.4a (the pre-pass's order) with `ECO-distribution.md` ECO-DIST-1.6 T22: E-trace-014 (R4, the Weak game), player 1. The stones 92284 at (139, 43) and 92289 at (137, 44), going to the catapult's site 91705 on (132, 31), lose their next flag in the re-check of counter 25785; the player's list of all orders holds 92284's order at position 34 and 92289's at 35 (heap dump 25500) | the pre-pass of 25795, the storage run of 25795, the matching of 25806 | the pre-pass drops 92289 first, then 92284, so the stone list holds 92289 before 92284; the storage run sends 92289 to the castle 90180 and 92284 to 90351; at 25806 the catapult's site orders a stone, both lie 13 from its field and 92289 gets it (destination 91705 from 25806), 92284 stays storage-bound (measured, `wares.csv`). Taking the first by ware number gives it to 92284, whose stored cost (89) loses to the castle's (88) while the castle cannot send: nothing is sent, the Weak game against R4 differs from settlers 25816 |
| T32 | 5.4a, five boards at one flag: R4, player 1, the boards (good c5c945a3) 92131, 92235, 92238, 92241 and 92270 lying at flag 91374 (137, 37); their orders at positions 16 (92131, for the building 92075 on (130, 36)), 28 (92270, for 92217 on (127, 30)), 29, 30, 31 (92238, 92241, 92235, for the catapult's site 91705); all lose their next flag by 25791 | the pre-pass of 25795 and the matchings of 25806 .. 25850 | the board list after the pre-pass: 92235, 92241, 92238, 92270, 92131 (the reverse of their orders); all lie at the same flag, so every matching takes the first one left: 25806 the catapult's site 92235, 25817 the catapult's site 92241, 25828 the catapult's site 92238, 25839 the site 92217 92270, 25850 the building 92075 92131 (measured, `wares.csv`). An experiment build with this list and T22: the Weak game against R4 from settlers 25926 to 26973, buildings 26891 to 28426, generator 26083 to 26979 (the 150 000 pair's generator 26083 to 26979); the dense 6000 and 20 000 pairs, the Hard pairs and the one-player pair unchanged. The list alone (without T22) changes no measure |
| T33 | 5.4a, the counter-case (joining the list, not the flag): E-trace-011 (the Weak game), player 1, the guard house's site 90785 on (131, 59) orders a stone; free stones 91561 at (141, 52) (put down by the quarryman without a destination at 16591, so on the list since then; on its way to the castle) and 91720 at (142, 55) (put down without a destination at 18300; claimed by the fetching carrier 91112), both 13 from the site's field | the matching of 18468 | 91561 is first on the list (it joined at 16591, 91720 at 18300) and is the one compared with the castle; neither is redirected (the guard house is served by 91672 at 18513, measured). Joining the flag would put 91720 first (it reached 90271 at 18386, 91561 reached 90597 at 18446): taking it redirects it and breaks the Weak pair at 18490 (experiment of 2026-10-28) |
| T34 | 5.3d step 2 (H is the representative's warehouse, and H sends): E-trace-014 (R4, the Weak game), player 1, the matching of counter 26972, stone; two open stone orders in one network: the representative R is the site's order with flag (142, 61), the chosen D2 the order of the site 92035 (144, 39) (flag (145, 38)); stored costs: from (142, 61) castle 90180 28, harbour 90351 50; from (145, 38) castle 52, harbour 33 (the original's store in the heap dump 27000 holds 52 and 33); the harbour has one stone, both warehouses can send | the send | the castle (H for R) sends the stone to 92035 (ware 92621, helper 92620 at (139, 52), measured); the harbour, nearest to D2, only passes the three tests. Sending from the warehouse nearest to D2 sends from the harbour: the Weak game against R4 differs from settlers 26973. Experiment build (with ECO-DIST T22, 5.4a and AI-CON-3.3 T77): settlers 26973 -> 27307, generator 26979 -> 27305 (the 150 000 pair's too), buildings 28426 unchanged; the other pairs unchanged |
| T35 | 5.3c (the bar already at 0): E-trace-014 (R4, the Weak game), player 1's castle (139, 51), flag (140, 50). The carrier 90611 of the road from (141, 52) walks in with the ware 92990; its entry check of counter 30929 raises the bar to 0. At 30930 it binds the stone 93117, laid down at the flag at 30929 with its next flag (141, 52), as its exchange ware, and makes no further check. The helper 93123 is made at counter t = 30943 with the ware 93124 | the helper's walk | ordered at counter 30944 = t + 1: trace tick 30945 at progress 0 (recorded). The helpers of the same castle before and after walk from t + 2 (93121: t = 30910) and t + 3 (93125: t = 30976, a ware left the flag in between), as before. A fixed wait to t + 2 gives a Weak vs R4 difference at settlers 30946. With the rule: 33000 (experiment build; the other pairs unchanged) |
| T36 | 2.5 (the network key): E-trace-013 (R3, the Hard game), player 1's queue. Carrier request 92254 for road 92250 (first flag (128, 62), chosen warehouse the castle, cooling down until 17207) is in the queue just before carrier request 92263 for road 92260 ((142, 39) - (144, 39), new at counter 17166, chosen warehouse the harbour (144, 47), cooldown 0, 95 helpers in stock). Both flags are in the land network | 92263 | not tried from 17167 to 17206. At 17207 the castle serves 92254 (carrier 92269) and in the same tick the harbour serves 92263 (carrier 92271 from (144, 47), trace tick 17208) (recorded: objects, streets and stocks tables). Keyed by the requester, 92263 is served at 17167: the Hard game against R3 differs from settlers 17168. With the network key: generator 17333, R3 settlers 17310 (experiment build; keying by the profession alone, without the network, gives a difference from 5150). Buildings unchanged (17277, R3 17280) and the Weak pairs unchanged |
| T37 | 3.4 (the positions of the distance): E-trace-013 (R3, the Hard game), player 1's builder request 92909 for the hunter's hut site 92904 on (131, 31) (flag (132, 30), reachable from counter 22080); A = the storehouse on (144, 47) (builders in stock, the lower route cost), B = the castle on (139, 51) (builders in stock too, and it can make one) | which warehouse serves it | field distances to (132, 30): (139, 51) 21, (144, 47) 21: B is not strictly nearer, A serves: the storehouse, whose cooldown runs from its carriers of 22080, 22110 and 22140, sends the builder 93013 at 22170 (measured). Measured from the flags ((140, 50) 20, (145, 46) 21) the castle serves it at 22104 when its cooldown ends (the experiment build: R3 settlers first differ at 22110 (the builder 93007 on (139, 52)), generator 22107; with the rule 22380 and 22277, buildings 22280, every other pair unchanged) |
| T38 | 2.3a, 2.2 (a split road's request): E-trace-014 (R4, the Weak game), player 1; road 91898 "143,41 .. 142,37" has an open donkey request 94175 (queue place 10) when the computer player's flag 94187 on (142, 39) splits it at counter 38648; the halves 94191 and 94194 and the new road 94182 file carrier requests 94198 and 94199 at 38649 (places 11 and 12; 13 requests) | the queue and the storehouse (144, 47)'s carriers | 94175 is deleted at counter 38650 on its turn, unserved, with no link left (objects table); the storehouse sends the settler 94200 (for the site 94105) at 38650, the carrier 94208 for 94194 at 38681 and the carrier 94215 for 94182 at 38712 (31 ticks apart each). Deleting 94175 at the split (our build) leaves 10 requests at 38649 and sends 94215 at 38711, one tick early: the Weak pair's settlers differed from 38720; keeping it unlinked (experiment build): settlers 39270, buildings equal to 40 000 (was 39510), generator 39290 (was 38810), the 150 000 generator 39288 (was 38801); the Hard pairs unchanged |
| T30 | 4.2 (the mine's filing order) with `ECO-distribution.md` ECO-DIST-3.1a: E-trace-014 (R4, the Weak game), player 1's coal mine 92002 on (129, 55), the castle 90180 (139, 51) the only warehouse with food (from counter 25002: fish 4, bread 13, meat 28) | the castle's sends to the mine | fish at counters 25047 and 25080, meat at 25179 and 25212 (stock 28 -> 27), bread only at 25740 and 25773, although bread is in stock throughout; between them it sends at 25113 and 25146 to other buildings (measured, `wares.csv`, `stocks.csv`). An experiment build that files fish, bread, meat and visits equal ranks by good id sends bread at 25179 and 25212; the meat order is served at 25410 by a free meat ware lying at (141, 52), whose carrier sets off at 25426: the Weak game against R4 differs from settlers 25427. With 4.2's order and ECO-DIST-3.1a it is equal to settlers 25770, generator 25769 (was 25427 / 25747; the 150 000 pair's generator too), buildings 26278, the other pairs unchanged; either change alone moves nothing |
| T39 | 0.1, 2.1, 2.2 (one queue for all players, Q-382): E-trace-019 (R6, MP_4P_Fight, three Hard computer players). At counter 783 the queue holds, in filing order: 0 builder for player 1's site 14304 (47, 59); 1 builder for player 3's site 14314 (30, 22); 2, 3 carriers for player 1's roads 14322, 14325; 4 builder for player 2's site 14338 (49, 22) (request 14344, filed at 673); 5 builder for player 1's site 14345 (53, 60); 6, 7 carriers for player 3's roads 14372, 14375; 8 carrier for player 2's road 14381 (request 14384, filed at 732); 9 carrier for player 1's road 14386. The cursor is at 3 (782 tried 9, 0, 1, 2). Player 2's castle (52, 24) can send again from 783 (`MIL-soldiers.md` MIL-SOL-6.2) | the requests served | 783: 3 fails, 4 is served (the builder leaves player 2's castle; request 14344 gone, recorded), then the former 5 and 6 (now 4 and 5, the cursor staying on 4) are tried; 784 tries player 2's carrier request (now at 7), which fails (the castle's 30 ticks); 14384 is served at 814 (recorded), 14351 (player 1's builder) at 788. One queue per player (player 2's two requests tried every tick) sends the carrier at 783 and the builder at 813: the Fight game against R6 differs from settlers 784. With one queue (experiment build, with MIL-SOL-6.2): settlers 784 -> 1665, buildings 1156 -> 2373, generator 1660 -> 1899; every pair of the fidelity set unchanged |
| T40 | 0.1 (one building pass in creation order): E-trace-019 (R6, MP_4P_Fight), counter 4378: player 3's finished building 15576 (25, 27) files its worker request 15932 (a helper) and player 1's new site 15928 (54, 57) its leveller request 15933, in this order (object numbers; positions 2 and 3, recorded) | the queue's order and the requests served from 4416 to 4444 | 15933 is served at 4416 from position 1; with 4 open requests the cursor then stands on position 3 at the start of 4444, so player 2's carrier request 15959 (road 15958, filed at 4430, position 3) is served at 4444 before 15955 (road 15951, filed at 4414, position 2; served at 4534). Updating player by player (player 1's site before player 3's building) puts 15933 first: served at 4416 from position 0, the cursor's phase one place earlier, 15955 served at 4444, the site 15861 (49, 25) on that road in a ware network a run early and a plank sent to it at 4444: the Fight game against R6 differs from settlers 4452. With the rule (experiment build, with T39 and the later Fight rules): settlers 4452 -> 4871, generator 4698 -> 4873; every pair of the fidelity set unchanged |
| T41 | 5.7 (c) (the new choice in the next run; Q-384): E-trace-013 (R3, the Hard game), player 1's plank 96452, put down at the harbour 90366's flag 90367 (145, 46) at counter 45686 after the harbour stopped taking planks (`TRN-carriers-transport.md` T59); the storehouse 94359 takes planks throughout | the plank and the carriers of the road (145, 46) - (143, 45) | destination dropped at the storage run of 45690, 94359 chosen at 45695 (next flag (143, 45)); the donkey 95957 stands in 45691 and walks to its idle field empty from 45692; the carrier 90406, after his put-down at (143, 45), claims the plank in 45699 and fetches it (recorded). Choosing in the same run (45690) makes the plank ready for the donkey's look in 45692: he takes it back, and the Hard game against R3 differs from settlers 45693; keeping the next flag "none" through the 3-tick re-check is needed as well (else the plank keeps the harbour and the carrier stays idle: 45723). With the rule (experiment build, with T59 and AI-CON T100): settlers 46075 (the Hard 150 000 pair at its 100th-tick rows: settlers 45800 -> 46100; every other pair unchanged) |
| T42 | 5.5 (a ware whose building is removed): E-trace-019 (R6, the war), player 1's hunter's hut site on (43, 60) torn down by the computer player in counter 12704 (AI-CON-9.2); plank 18331, bound for it, lies at the flag (50, 61) (next flag (47, 61)) with a carrier's claim; the site on (49, 60) has an open plank order; 12705 is a matching tick and a storage tick | 18331's destination | none from counter 12704 (recorded: the wares table's row of 12704); in counter 12705 the matching gives it to the site on (49, 60) and its carrier fetches it (recorded: the carrier of the road at (50, 60) walks from trace tick 12708), while the site's own plank, put on its flag (43, 59) by the tear-down (no order left for it), gets the headquarters in the storage run of 12705. Keeping the destination until the storage run sends 18331 to a warehouse and the war's buildings differ from trace tick 12943 (the site on (49, 60) waits for its plank); with the rule (experiment build, the tick-exact branch's head) from 13450, every Storm Coast pair unchanged |
| T43 | 5.3h: E-trace-019 (R6, the war), player 3's plank 18665, matched at counter 13398 from the flag (29, 18) to the site on (18, 34) (its order chosen then), destroyed in its carrier's hands at the take-over of 13448; player 3's site on (24, 24) has an open plank order last chosen in 12551; the matching of 13519 finds the free plank 18803 at the flag (23, 32) | which order gets the plank of 13519, and when (18, 34) is served again | (24, 24)'s order (its stamp 12551 is lower than the reopened order's 13398): the plank goes to (24, 24); (18, 34) gets its plank only at 14388 (from the flag (25, 21)). Filing a new order (stamp 0) for (18, 34) instead sends the plank of 13519 there, and the carrier of the road (24, 26)-(26, 26) takes it as an exchange ware at (26, 26) in 14101 where the original walks back empty: the war's settlers first differ at trace tick 14110, buildings 14860, generator 14799; with the rule settlers 14580, buildings 15650, generator 15181 (experiment build on 0.12.3 with the rules of 2026-11-21; every Storm Coast pair unchanged) |
| T44 | 5.3 (a), no H: E-trace-013 (the Hard game), player 1's free ware 97738 (good 7068dcd3, lying at flag (121, 55)) matched in counter 53416 to the order of the building on (122, 60) (flag (122, 59)); no warehouse of the network holds that good; the flags joined by the roads (121, 55) - (121, 58) (length 6) and (121, 58) - (122, 59) (length 4) | the store's entry for the pair (121, 55) / (122, 59) | not filled at 53416 (the ware gets the order without a cost asked). The computer player's route optimizer run of counter 53496 on building 93606 (flag (121, 55), partner with flag (122, 59), d = 4, k = 2) asks it first, from (121, 55): 22 stored (the heap dump of 53500; absent from the dump of 53000), above 4 x 4, so the direct variant (w 1, X ending (121, 55)); that variant's road (122, 59) - (121, 55) is begun and cancelled (no path) at 53570, its stub taking an object number. Asking and storing the cost at the matching (10, roads free then) takes the route variant at 53496: no stub, every later object number one lower, the Hard pair's generator first differs at trace tick 53726 (settlers 53900, buildings 54000); with the rule generator 54206, settlers 54400, buildings 55700 (experiment build on 0.12.3 with every rule of 2026-11-21; every other pair unchanged) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-007 | code reading | `S2DNG.exe` | order of the per-tick updates |
| E-eco-007 | code reading | | player update: buildings, then request queue |
| E-eco-007 | code reading | | road update: helper, donkey and boat helper requests |
| E-eco-007 | code reading | | building update; leveller and builder requests |
| E-eco-007 | code reading | | worker request on a 101-tick timer |
| E-eco-007 | code reading | | request creation |
| E-eco-007 | code reading | | queue processing: 4 per tick, cursor, deletion |
| E-eco-007 | code reading | | serving a request; warehouse choice; networks; route cost |
| E-eco-007 | code reading | | sending, making from tools, cooldown, settler kinds |
| E-eco-007 | code reading | | construction material orders after levelling |
| E-eco-007 | code reading | | input orders, re-orders on production |
| E-eco-007 | code reading | | order registration with the transport manager |
| E-eco-007 | code reading | | transport jobs: matching, free wares, storage |
| E-eco-007 | code reading | | building type defaults (worker limit 1, stock limit unset, radius 4, construction time 500) |
| E-eco-007 | script | `Jobs.lua`, `Goods.lua`, `buildings.lua` | profession cost lists, product cost lists, limits |
| E-sim-007 | code reading and recording | E-trace-006 `objects.csv.gz` | 5.7: matching pre-pass, storage run, ware networks |
| E-trn-002 | code reading | | request destination: road's first flag |
| E-trn-002 | code reading | | message dedupe by type, text and field; display time reset to 600 |
| E-trn-002 | code reading | | ware stops lying at a flag when picked up |
| E-trn-003 | code reading | | worker counted at dispatch; worker present at the door |
| E-trn-003 | code reading | | ware sent out with a helper; cooldown |
| E-trace-002 | recording | `settlers.csv.gz`, carriers 90207, 90209, 90204 | helper timeline, matching tick, cooldown (5.3c, T16) |
| E-trn-017, E-trace-011 | code reading and recording | streets and objects tables of E-trace-011 | ECO-REQ-3.2 costs (T18) |
| E-trace-011 | recording | `wares.csv.gz` and `objects.csv.gz` (helpers 91232, 91235, 91237, 91240 and their wares, counters 11700 .. 11840) | ECO-REQ-5.3c (T19) |
| E-eco-038, E-trace-013 | code reading and recording | `E-trace-013` `objects.csv.gz` (requests 92254, 92263), `streets.csv.gz`, `stocks.csv.gz` (counters 17100 .. 17210) | 2.5 (corrected), T36 |
| E-eco-023, E-trace-011 | code reading, memory reads of the running game, recording | heap dumps 10000, 10500, 11000, 11500, 12000, 12500 (order: building, ware, stamp); objects and wares tables (plank 91092 delivered 10635, plank 91206 delivered 11819) | ECO-REQ-5.3f (T20) |
| E-eco-025, E-trace-011 | code reading, recording | wares table rows of plank 91337, counters 14500 .. 14540, settler rows of 90399 and 91072 | ECO-REQ-5.3e (T21) |
| E-eco-026 | code reading | | ECO-REQ-5.3g (T22) |
| E-eco-027 | code reading | | ECO-REQ-5.3e (Q-285) |
| E-eco-030, E-trace-013 | code reading and recording | the tick parts of the logic tick (E-ai-007); settlers and wares tables of E-trace-013 | ECO-REQ-4.2 when (T25) |
| E-eco-033, E-trace-011, E-trace-013 | code reading and recordings | wares and flags tables of E-trace-011 (8200 .. 8275) and E-trace-013 (10700 .. 11120) | ECO-REQ-5.3 (a) (T28) |
| E-eco-052, E-trace-013 | code reading, memory reads of the running game, recording, experiment build | (session 2026-11-21 eve): the order matching's case (a); the route store in the heap dumps of 53000 and 53500; the computer player's route task and road builder tables, counters 53311 .. 53570 | ECO-REQ-5.3 (a) without H (T44) |
| E-eco-037, E-trace-014, E-trace-011 | code reading, a memory read of the running game, recordings | heap dump 25500 of E-trace-014 (the player's order list); `wares.csv` of E-trace-014 (25770 .. 25860) and E-trace-011 (16458 .. 18528) | ECO-REQ-5.4a (T31 .. T33) |
| E-eco-043, E-trace-013 | code reading and recording | E-trace-013 `wares.csv.gz` (counters 45640 .. 45700) | ECO-REQ-5.7 (c), T41 |
| E-eco-044 | code reading | (session 2026-11-11 eve): the warehouse door's sending of a settler by profession, the six per-class list appends of the settlers' system, the burning warehouse's evacuation loop, `Jobs.lua` (the profession ids, the boat carrier's type) | 0.2a (Q-287) |
| E-eco-042, E-trace-019 | code reading and recording | E-trace-019 `objects.csv.gz` (request rows: positions across players 1 to 3 in one list), `buildings.csv.gz` (the list in object-number order) | ECO-REQ-0.1, 2.1, 2.2, T39 |
| E-eco-039, E-trace-013 | code reading and recording | E-trace-013 `objects.csv.gz` (request 92909), `settlers.csv.gz` (93002, 93008, 93011, 93013 out of (144, 47)) | ECO-REQ-3.4, T37 |
| E-eco-035, E-trace-014 | code reading and recording | E-trace-014 `wares.csv`, `stocks.csv` counters 25002 .. 25773 | ECO-REQ-4.2 (T30), `ECO-distribution.md` ECO-DIST-3.1a |
| E-eco-049, E-trace-019 | recording and experiment build | (session 2026-11-17 day): R6's wares table for the planks 18331 and 18515 at counters 12704 .. 12707 | 5.5 the moment a removed building's ware is free (T42) |
| E-eco-050, E-trace-019 | code reading, recording, experiment build | (session 2026-11-21 pm): R6's wares table for the planks 18665, 18803 and 18994 at counters 13398 .. 14388 | 5.3h a destroyed ware's order (T43) |

## Open questions

- Q-043 (rest): the order in which ware orders are visited within a player.
  Which wares are on a good's waiting list, and in which order: 5.4a (Q-373).
- Q-046: what a sent settler does when its road walk cannot start.
- Q-050: timing of a ware carried out of a warehouse by a helper (ECO-REQ-5.3 b).
