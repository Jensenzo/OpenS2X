# ECO-DIST: Warehouses, distribution and transport priority settings

Status: draft (read from the binary and shipped scripts; no in-game measurement)
Owner question(s): plan step 3.4; Q-043 (order visit order) in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

The player steers the flow of wares with three kinds of settings. **Per
warehouse and per good** two flags: "do not store" (the warehouse takes no new
units of that good) and "take out" (the warehouse also sends its units of that
good away, one at a time). **Per player and per scarce good** a distribution
table (grain, iron, water, planks, food, coal) with one weight per consumer
building type; when several buildings wait for the same good, the type to serve
next is picked by a weighted round robin. **Per player** a transport priority
rank for every good, which sets the order in which carriers pick up wares and
the order in which the order-matching job visits goods. All three start at
fixed defaults and change only through player commands (and computer players).

This spec extends `ECO-requests-dispatch.md` ECO-REQ-5 (matching orders to
wares) and uses its terms: order, free ware, warehouse with stock, route cost.

## Rules

### ECO-DIST-1 Warehouse good flags

- **ECO-DIST-1.1** Every warehouse (headquarters, storehouse, harbour) has, for
  every good including settler professions, two flags: **stop** ("no longer
  stored here") and **out** ("take out of this warehouse"). Both are off by
  default. Confidence: high.
- **ECO-DIST-1.2** A warehouse **accepts** a good when neither flag is set for
  it. Confidence: high.
- **ECO-DIST-1.3** Player commands (each checks that the building still exists;
  otherwise it does nothing):
  - **cycle** (`cycle-ware-flags`; clicking a good in the warehouse window): off/off -> stop;
    stop -> out (stop cleared); out -> off/off. A state with both flags on (only
    reachable through the set commands) goes to off/off.
  - **toggle stop**, **toggle out** (`toggle-stop-flag`, `toggle-out-flag`): flip one flag, the other unchanged.
  - **set stop**, **set out** (`set-stop-flag`, `set-out-flag`): set one flag to the given value.
  - **all** (`set-all-ware-flags`; the button in the window), with a mode and a page (wares or
    settlers): for every good of that page (every good of type ware, or every
    good of type settler): mode 0 clears both flags; mode 1 sets stop and
    clears out; mode 2 clears stop and sets out.
  Confidence: high (executors read); the mapping of UI buttons to commands
  follows the shipped help text (medium).
- **ECO-DIST-1.4** The flags do **not** affect supplying orders: a warehouse
  with the good in stock can serve an order (ECO-REQ-5.3, the nearest
  warehouse with stock) whatever its flags. Confidence: high.
- **ECO-DIST-1.5** **Storage destination.** A ware without destination is sent
  to the finished warehouse of its road network that **accepts** its good and
  has the lowest route cost from the ware's flag (ECO-REQ-5.5; ties: first in
  the player's warehouse list). Warehouses still under construction never
  count. If no warehouse accepts the good, the ware stays without destination
  and is tried again at the next storage run. Confidence: high.
- **ECO-DIST-1.6** **Re-check of wares at flags** (the transport job on ticks with
  `t mod 3 = 0`, ECO-REQ-5.1; revised 2026-10-24, E-sim-007). Every ware that lies at a
  flag (no carrier holds it) and has a destination (an order's building or a storage
  warehouse) is checked: its next flag (`TRN-carriers-transport.md` TRN-CARRY-5.4) still
  exists, is in the same ware network as **the destination's flag** (TRN-CARRY-5.1a;
  corrected 2026-10-28, T22: not the ware's own flag), a road joins the ware's flag and
  the next flag, and the destination still accepts the good (ECO-DIST-1.2; a building
  without flag records accepts everything). So a ware whose destination has left its
  network (a road on the way lost its carrier, or a split left a half without one)
  loses its next flag at its next re-check, even though its next flag is still joined
  to its own. Confidence: high (E-eco-036: the network test takes the
  destination's flag and the next flag; measured, T22). If not, **or** when the per-ware counter
  has already reached 200, the counter is set to 0 and the next flag is computed
  again; otherwise the counter rises by 1 (the test comes before the rise). **Every
  computation of the next flag starts the counter at 0**, also the one when the ware
  is put down at a flag (corrected 2026-10-25 evening, measured E-trace-010: the
  stone 90433 put down on (141, 48) at counter 3019 with next flag (145, 46), counted
  1 .. 200 by the re-checks of 3021 .. 3618, computed again at 3621 to (140, 46), its
  count 8 at 3647; a counter kept from earlier flags, or tested after the rise, made
  an experiment build compute it at 3375 or 3618 and keep (145, 46)); the search finds none when the destination is out of the ware
  network or no longer accepts the good. A ware left without a next flag then loses its
  destination in the next storage run (storage trips, `ECO-requests-dispatch.md`
  ECO-REQ-5.7 (c)) or the next matching (orders, ECO-REQ-5.7 (a)) and is placed again.
  A ware a carrier has claimed (walking to fetch it) is not re-checked
  (E-eco-019); a redirect by the matching releases the claim, so such a
  ware is re-checked again from the matching's own tick (`ECO-requests-dispatch.md`
  ECO-REQ-5.3e, T21).
  Hence setting "stop" on a warehouse redirects wares lying at flags on their way there
  within 3 + 5 ticks, and every route of a lying ware is computed again at least every 603 ticks (201 re-checks). Confidence: high for which wares are checked and for the tests (E-sim-007:
  the re-check runs over the transport's whole ware list); the storage case is not
  measured.
- **ECO-DIST-1.7** **Taking out** (the transport job on ticks with
  `t mod 7 = 0`, until now unspecified in ECO-REQ-5.1). For every warehouse of
  every player, in warehouse list order, skipping warehouses that are being
  destroyed:
  1. Let `n` be the number of goods for which this warehouse has a flag record
     (a record exists for a good once any flag command has touched it, ECO-DIST-1.3;
     "all" touches every good of its page). If `n = 0`, skip the warehouse; no
     random number is drawn.
  2. Draw one number from the game's random generator (the shared Lehmer
     generator, Q-116) and set `r = draw mod n`.
  3. Walk the warehouse's records in ascending good id order (32-bit signed
     comparison). For each record with **out** set: `r = r - 1`; then if the
     warehouse has at least one unit of the good in stock and `r < 1`, send one
     unit out (step 4) and stop the walk. Records without "out" are passed over
     without changing `r`.
  4. Sending a ware: only if the warehouse can provide a helper (has one or can
     make one, ECO-REQ-3.3) and its flag holds fewer wares than the flag limit
     (6): a helper carries one unit out to the warehouse's flag, exactly as when
     a warehouse sends a ware for an order (ECO-REQ-5.3 (b), the same code): the
     helper is taken from the stock (or made from its cost list) and appears at
     the door, **the door's wait becomes 30** (1.7d), one unit leaves the stock
     and is given to the helper at the door, and the helper walks out with it
     and puts it down at the flag (ECO-REQ-5.3c's timeline). But the ware has
     **no destination**; the storage rule (ECO-DIST-1.5) then sends it to the
     nearest warehouse that accepts it (never this one, since "out" means it
     does not accept). Sending a settler (a profession or
     soldier rank with "out" set): one settler of that kind leaves the
     warehouse (same sending as for a settler request, ECO-REQ-3.6) and goes to
     another warehouse that accepts it (medium).
  So a warehouse sends at most one unit every 7 ticks in total, whatever the
  number of goods marked "out", and the random start spreads the sending over
  the marked goods. Confidence: high for steps 1 to 3 and the 7-tick rhythm;
  medium for the settler case.
- **ECO-DIST-1.7a Settler taken out (Q-057).** The settler is made at the
  warehouse's door by the same per-kind sending as for a request (a helper as
  a carrier, a soldier rank as a soldier, any other profession as a worker;
  made from its cost list if none is in stock; the door then waits 30 ticks,
  ECO-REQ-3.3). It gets **no job and no destination**. Like the new donkey
  (`ECO-food-mining.md` ECO-FOOD-6.3) it walks out to the warehouse's flag
  and then goes home (`TRN-free-walk.md` TRN-FREE-6.4): to the warehouse with
  the lowest route cost that accepts it, which cannot be the sending one.
  The door's 30-tick wait is set by every settler sent this way (2026-10-24): the
  take-out job sends through the same per-kind sending as the requests (builder,
  leveller, worker, carrier, soldier ranks), and each of these sets the door's wait
  to 30 before the settler is made, whatever called it; the take-out job's own door
  test (1.7c (a)) reads that wait. Confidence: high for the creation without job and
  for the wait (one shared sending path, E-sim-007; the wait it sets is measured for the request path,
  E-trace-006, 1.7c; no take-out happened in E-trace-006, no record had "out" set);
  medium for "goes home" (the carrier case is traced, ECO-FOOD-6.3; the worker and
  soldier cases are assumed to behave alike).
- **ECO-DIST-1.7d A ware taken out blocks the door too (Q-265, 2026-10-24).** The
  ware branch of step 4 sends through the warehouse's helper sending, the same one the
  order matching uses for a ware (ECO-REQ-5.3 (b)) and the requests use for a carrier
  (ECO-REQ-3.6), and that sending sets the door's wait to 30 before the helper is
  made. So after a ware is taken out, the job skips this warehouse (1.7c (a), no draw)
  until the wait has run out: a warehouse that only takes wares out sends at most one
  unit every 35 ticks (the wait ends 30 ticks later, the job runs on the next tick with
  `t mod 7 = 0`), and a ware or settler sent for an order in between restarts the wait.
  The ware never appears at the flag by itself: it is made in the helper's hands at the
  door and lies at the flag only when the helper has put it down. Confidence: high
  (E-sim-007: flag limit test, then the helper sending,
  which sets the wait unless a byte of the door object (
  not identified; it also skips the stock handling) is set, stock decrease, ware made and given to the helper; the job's door test
  reads that wait; the order matching reaches the same sending). The 35-tick rhythm itself is not recorded (no take-out happened in
  E-trace-006).
- **ECO-DIST-1.7b Measured (E-trace-005, 2026-10-21; E-trace-006, 2026-10-23).** In
  the two-player game the draw of step 2 is the one extra draw before the animals of
  trace tick 484 and the only draw of trace ticks 554, 561 and 582 (counters 483,
  553, 560, 581). The computer player's castle holds records from counter 403 on
  (ECO-DIST-1.7c (b)); the draw is skipped on the 7th counters 404 .. 476 and 490 ..
  546 by the door rule of 1.7c (a). The draw is made whether or not a unit leaves
  (step 3 may send nothing). Confidence: high.
- **ECO-DIST-1.7c The door and the records' makers (Q-339; E-trace-006, E-sim-005).**
  (a) Step 1 also skips a warehouse whose door is blocked: the job tests the door
  state that a settler leaving sets (a settler made at the door blocks it for 30
  ticks, ECO-DIST-1.7a, ECO-REQ-3.3), and while it is blocked no number is drawn.
  Measured on the computer player's castle: settlers left at counters 387, 417 and
  447 (its builder and two carriers) and 484 and 517 (helpers with planks); the door
  was blocked at 476 (447 + 30 = 477) and at 490 .. 546, free at 483 and from 553:
  exactly the draws seen. Clone check (experiment clone, dummy draws before the
  animals at counters 483, 553, 560, 567, 574, 581, 588, 595): the generator's first
  differing trace tick moves from 554 to 603 (counter 602, the next 7th counter, not
  in the list).
  (b) The computer player's recruitment step (the 409-tick cycle; its first run is in
  counter 403) creates, by find-or-create, a record for each of the three recruiting
  goods (sword a8483903, shield 1eebcf03, beer 58647da3) with "stop" and "out" both
  clear; the castle's map holds these 3 records from 403 (flags all 0 through counter
  1279). Which warehouses get them (Q-264, `AI-military.md` AI-MIL-6.1): every finished
  depot (castle or storehouse) and harbour of the computer player once one depot on its
  home landmass is chosen; the chosen one with "out" clear, all others with "out" set.
  Its goods arrangement (643-tick cycle) adds coin 499d4c63, stone 55e952d3 and
  plank c5c945a3 at 1280 (6 records, flags 0). A record with clear flags counts for
  step 1's `n`, so from 403 on the castle draws on every free 7th counter. The human
  castle has no record in 6000 counters (no flag command was given). Confidence: high
  for the ticks and the records (recorded every tick, E-sim-005);
  medium for the identification of the tested field with the 30-tick door wait (one
  warehouse field is tested; the wait fits every measured tick).
- **ECO-DIST-1.8** An order filed by a warehouse-type building (orders that
  warehouses themselves place, for example harbour expedition material) is
  served only while that warehouse accepts the good. Confidence: medium.

### ECO-DIST-2 Distribution of scarce goods

- **ECO-DIST-2.1** Each player has a distribution table for six goods. Each
  table is an ordered list of (consumer key, weight, accumulator). Defaults
  (order as listed, accumulators 0):

  | good | entries (key: weight) |
  |---|---|
  | grain (`corn`) | mill 0.5, piggery 0.3, donkey farm 0.2, brewery 0.3 |
  | iron | armory (`smithy`) 0.8, metalworks 0.4 |
  | water | bakery 0.6, brewery 0.3, piggery 0.2, donkey farm 0.3 |
  | planks | **construction** 1.0, metalworks 0.4, shipyard 0.2 |
  | food | stone mine 0.3, coal mine 0.5, iron mine 0.7, gold mine 1.0 |
  | coal | armory 0.8, smelter 0.6, mint 1.0 |

  The food table serves orders for fish, meat (`flesh`) and bread alike: an
  order for any of the three looks up the food table. "Construction" is the
  key of every building still under construction (whatever its type); a
  finished building uses its type. Confidence: high.
- **ECO-DIST-2.2** Commands: **set weight** (`set-distribution-weight`; good, entry index, value) stores
  the value, but at least 0.001 (anything at or below 0.001 becomes 0.001); the
  accumulator is kept. **reset** (`reset-distribution`; good) rebuilds that good's table with the
  defaults and accumulators 0. Confidence: high.
- **ECO-DIST-2.3** **Pick.** When an order for good G must be chosen among
  several and G has a table (ECO-DIST-3.3):
  1. Mark every entry "absent". For each open order of G whose building's flag
     is in the same road network as the place the ware would come from, mark
     the entry of the order's key "present".
  2. Add each entry's weight to its accumulator (**all** entries, present or
     not; float32).
  3. Among the present entries, take the one with the highest accumulator
     (first in list order on ties) and set its accumulator to 0. If no entry is
     present, the first entry's accumulator is set to 0 instead and no order is
     chosen.
  4. Among the open orders of G with that key in that network, choose the one
     that has waited longest (lowest order stamp: the tick it was filed or last
     chosen; the chosen order's stamp is set to the current tick).
  Entries never present keep accumulating, but they cannot win until they
  have an order. Confidence: high for 1 to 3; medium for the stamp's initial
  value.
- **ECO-DIST-2.4** The pick ignores distance entirely; distance only decides
  where the ware comes from (ECO-REQ-5.3). The long-run shares are **not**
  proportional to the weights (see T2, T3). Confidence: high.
- **ECO-DIST-2.5** Computer players adjust their own food table (gold, coal,
  iron and stone mine weights) from their stock; human players' tables change
  only by command. The rule: `AI-construction.md` AI-CON-10.3 (T87). Confidence: high.

### ECO-DIST-3 Order matching order (extends ECO-REQ-5.2)

- **ECO-DIST-3.1** On each matching tick (`t mod 11 = 0`), players are visited
  0 to 7. Within a player, the open orders are kept per good in two lists
  (visited first list 0, then list 1; measured for one good, T23: a finished
  warehouse-type building's order is served before an older order of list 1;
  whether all goods of list 0 come before list 1 or each good takes its list 0
  then its list 1 is not measured; which orders go to which list:
  ECO-DIST-3.3a, Q-043). Within a list, goods are visited in **transport priority
  order**: lowest rank first (ECO-DIST-4), goods with equal rank in their list
  order. Confidence: high (sorting by the priority lookup is read), medium on
  the two lists.
- **ECO-DIST-3.1a List order of goods (Q-364).** A good's place in a list is
  the place of its group, made when the good gets its first order in that
  list; a group stays (empty) when its orders are gone, so the place is kept
  for the rest of the game. The rank sort is redone for each player at the
  start of the player's turn in every matching run, ties in group order, not
  by good id. Within a group the orders keep the order in which they were
  filed (a served or dropped order is taken out without reordering the rest).
  Confidence: high (code E-eco-019;
  measured 2026-10-28, `ECO-requests-dispatch.md` T30: E-trace-014's coal
  mine files fish, meat, bread (ECO-REQ-4.2), and the castle serves its meat
  orders before its bread orders, both rank 11, although bread's good id is
  the lower; equal ranks by good id serve bread first and the Weak game
  differs from settlers 25427).
- **ECO-DIST-3.2** For each good, its open orders are grouped by road network:
  one representative order per network (the first order found in that
  network). The good is processed **once per network** per matching tick:
  at most one order of that good in that network gets a ware (a free ware
  redirected, or one sent from a warehouse) per 11 ticks. Confidence: high.
- **ECO-DIST-3.3** Which order of the good (in that network) is served:
  - if the good has a distribution table (ECO-DIST-2.1; fish, meat, bread via
    food): the pick of ECO-DIST-2.3;
  - coins: the military building rule (chunk on soldiers, not in this spec);
  - any other good: the order that has waited longest (lowest stamp), among
    those reachable; an order from a warehouse-type building only while it
    accepts the good (ECO-DIST-1.8).
  Then ECO-REQ-5.3 decides where the ware comes from (nearest free ware or the
  nearest warehouse with stock). The choice is made only at the points of
  ECO-REQ-5.3d, up to twice per network and run, and not at all while the
  warehouse cannot send and no free ware lies in the network. Confidence:
  high.
- **ECO-DIST-3.3a Two order lists (Q-057).** A player's open orders are kept
  in two lists, each grouped by good: list 0 holds the orders of **finished
  warehouse-type buildings** (`bp_setDepot`: headquarters, warehouses,
  harbours), list 1 all other orders, including those of warehouse-type
  buildings still under construction. Orders of list 0 are always chosen by
  the plain "longest waiting" rule above, also for goods with a distribution
  table and for coins; the distribution table and the coin rule apply to
  list 1 only. Confidence: high.
- **ECO-DIST-3.3b Order stamp (Q-057).** A new order's stamp is **0**, so it
  counts as having waited longer than every order that has already been
  chosen once (a chosen order's stamp is set to the current tick). The
  "longest waiting" pick takes the strictly lowest stamp, comparing as
  unsigned numbers; on equal stamps the order earlier in the list wins.
  A served order keeps its stamp and can come back into the list at the
  delivery of its ware (`ECO-requests-dispatch.md` ECO-REQ-5.3f).
  The stamp is the tick counter of the choice. An order is marked at every
  choice of ECO-REQ-5.3d, also when nothing is then sent to it (no free ware
  for it and no warehouse, or the warehouse nearer but not able to send);
  the clone tests of 2026-10-25 (stamp 0 regressing) marked at every
  matching run instead, which is not the original's rule. Confidence: high
  (code E-eco-013, E-eco-019; measured,
  E-trace-010 counter 4939, T21).

### ECO-DIST-4 Transport priority

- **ECO-DIST-4.1** Each player has a rank for every good of type ware and every
  ware group (food, tools, weapons, ship, donkey). Defaults are the
  `gp_setTransportPriority` values of `Goods.lua` (table in
  `TRN-carriers-transport.md`): coin 0; sword, shield, weapons 1; beer 2;
  iron 3; gold 4; iron ore 5; coal 6; planks 7; stone 8; wood 9; water 10;
  pig, grain, flour, fish, meat, bread, food, ship 11; every tool and tools
  12; boat 13; donkey 14. Ranks need not be distinct. Lower rank = served
  first. Confidence: high.
- **ECO-DIST-4.2** **Move** command (`move-priority`; from rank `a`, to rank `b`): every good
  with rank `a` gets rank `b`; if `a < b` every good with a rank in `a+1 .. b`
  goes down by 1, if `a > b` every good with a rank in `b .. a-1` goes up by 1.
  (So with duplicate ranks a whole rank class moves together.) **Reset**
  (`reset-priorities`) rebuilds the list from the defaults. Confidence: high.
- **ECO-DIST-4.3** The rank is used (a) by carriers choosing which ware to
  pick up (TRN-CARRY) and (b) by order matching (ECO-DIST-3.1). Confidence:
  high.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| warehouse flags default | stop off, out off | | high | E-eco-011 |
| take-out period | t mod 7 = 0 | ticks | high | E-eco-011 |
| units taken out per warehouse per period | at most 1 | | high | E-eco-011 |
| storage re-check period | t mod 3 = 0 | ticks | high | E-eco-011 |
| storage re-check limit | 200 checks | | high | E-eco-011 |
| minimum distribution weight | 0.001 | | high | E-eco-011 |
| distribution defaults | table ECO-DIST-2.1 | | high | E-eco-011 |
| matching: orders served per good per network per run | 1 | | high | E-eco-011 |
| transport ranks | Goods.lua | | high | E-eco-011 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | grain table at defaults; open grain orders from a mill, a piggery, a donkey farm and a brewery in one network, always refilled | 13 picks | mill, piggery, mill, brewery, mill, piggery, donkey farm, mill, brewery, piggery, mill, donkey farm, brewery |
| T2 | as T1 | 1300 picks | mill 473, piggery 296, brewery 295, donkey farm 236 |
| T3 | food table at defaults, orders from all four mine types | 13 picks; and counts over 2500 | gold, iron, gold, coal, iron, gold, stone, iron, gold, coal, iron, gold, stone; gold 834, iron 833, coal 417, stone 416 |
| T4 | food table at defaults, only a coal mine and a gold mine have orders | 8 picks | gold, coal, gold, coal, gold, coal, gold, coal |
| T5 | coal table at defaults, all three consumers | 12 picks | mint, armory, mint, smelter, armory, mint, smelter, armory, mint, smelter, armory, mint |
| T6 | set weight of the mill entry to 0 | read back | 0.001 |
| T7 | after T1, reset grain | next pick | mill (accumulators 0 again) |
| T8 | an iron mine waits for bread, a coal mine for fish | pick | both looked up in the food table (iron mine 0.7 beats coal mine 0.5 on the first pick) |
| T9 | default ranks | move rank 9 to rank 0 | wood 0; coin 1; sword, shield, weapons 2; beer 3; ... stone 9; water 10 unchanged |
| T10 | warehouse A nearer than B to a log at a flag; A has "stop" for wood | storage run | the log goes to B |
| T11 | as T10, log already travelling to A when "stop" is set | next tick with t mod 3 = 0 | the log loses its destination; at the next t mod 5 = 0 it is sent to B |
| T12 | HQ with 5 planks, "out" set on planks only (one flag record, so `r = 0`), a storehouse accepting planks, carriers keep the HQ flag below 6, helpers in stock, no orders | the ticks with t mod 7 = 0 from t0 | a helper with one plank at the door in t0, then at t0 + 35, t0 + 70, t0 + 105, t0 + 140 (5 in all; the door's 30-tick wait, 1.7d, makes the job skip t0 + 7 .. t0 + 28 and every 7th tick in between without a draw: one draw per plank); they are stored in the storehouse. (Corrected 2026-10-24, Q-265: was one plank on each of five consecutive 7th ticks) |
| T13 | warehouse whose flags were never touched | t mod 7 = 0 | no random number drawn for it |
| T14 | click a good three times | flags after each click | stop; out; none |
| T15 | three sawmills in one network each with open wood orders, HQ full of wood, no free logs | one matching run | exactly one wood order is served (and only if the HQ cooldown is 0, ECO-REQ-5.3) |
| T16 | wood order in network 1 and in network 2 (two HQs or a separate warehouse), both with stock | one matching run | one order served in each network |
| T17 | warehouse with "stop" set for planks, a building orders planks and this warehouse is nearest with stock | matching | the warehouse sends the plank (flags do not affect supply) |
| T18 | bread orders: a bakery-fed mine (list 1, table food) and a finished warehouse W that orders bread (list 0) | matching | W's order is chosen by lowest stamp, never by the food table |
| T19 | three new plank orders filed in ticks 100, 200, 300 (stamps 0), none served yet | first plank matching | the first in the list wins (all stamps 0); its stamp becomes the current tick, so the next pick goes to the second |
| T20 | warehouse with 'out' set for woodcutters, 1 woodcutter in stock, another warehouse accepts woodcutters | take-out job | a woodcutter leaves by the door, walks to the flag and goes to the other warehouse; door blocked 30 ticks |
| T21 | 3.3b, `ECO-requests-dispatch.md` ECO-REQ-5.3d: E-trace-010, player 1, planks, one ware network: the castle (139, 51) with planks, its send cooldown running at counters 4917, 4928 and 4939; open plank orders, in list order: the hunter site 90625 (144, 58), one order chosen before without being served (stamp 4752 in the experiment build), then two new orders of the site 90590 (140, 53) (stamp 0); the free plank 90660 lying at (139, 48) since 4910 (storage-bound) | matchings of 4917, 4928, 4939 | 4917: step 1 chooses 90590's first order (now stamped 4917); the plank's stored cost to its flag (16) is higher than the castle's (4) and the castle cannot send: nothing. 4928: 90590's second order, the same. 4939: the lowest stamp is now 90625's: plank cost 28 against the castle's 36, so 90660 gets destination 90625 (measured: redirected in 4939; its carrier 90234 then waits at the full castle flag, `TRN-carriers-transport.md` T28). Marking at every matching run, or keeping the filing tick as stamp, serves 90590 or the farm site (136, 58) instead |
| T22 | 1.6 (the network test): E-trace-014 (R4, the Weak game), player 1's stones 92284 (lying at (139, 43), next flag (137, 37)) and 92289 (at (137, 44), next (137, 37)), both for the catapult 91705 on (132, 31); at counter 25782 the computer player splits the road (137, 37)-(135, 30) at (136, 33), the half (135, 30)-(136, 33) without a carrier until 25803 (the carrier 92453 made at the castle's door) | the stones' next flags and destinations | the re-check of counter 25785 clears both next flags (their next flag is still joined to their own flag by a road with a carrier, but the catapult's flag is in another ware network); the storage run of 25795 sends 92284 to the storehouse 90351 and 92289 to the castle; at 25806, the networks joined again, the matching gives 92289 to the catapult (measured, `wares.csv`). An experiment build with the test on the ware's own flag kept both next flags and the catapult destination; with the test on the destination's flag both lose them at 25785 and are stored at 25795 as recorded; Hard, dense and single-player pairs unchanged, the Weak pair then differs at the matching of 25806 (Q-373) |
| T23 | 3.1 (list 0 before list 1), ECO-REQ-5.3: E-trace-018 (R5, the Weak game), player 1, the matching of counter 50820: plank orders of the harbour 90351 (144, 47) (finished, list 0) and of the site at (116, 35) (list 1), the castle (139, 51) the warehouse H for both with its cooldown at 0 | which order the castle sends to | the plank 96101 leaves the castle with the helper 96100 for the harbour (recorded); the site's order waits (the castle's 30 ticks). Taking the good's orders in filing order, both lists together, sends it to the site (experiment build): the plank's carrier 90260 stays idle at 50881 and the Weak game against R5 differs from settlers 50882; with list 0 first, from 51466 (buildings 52147 -> 52290; with `ECO-food-mining.md` T61 the Weak 150 000 generator 50934 -> 51472, every other pair unchanged) |
| T24 | 3.3a (list 0 by the plain rule for a good with a table): E-trace-013 (R3, the Hard game), player 1, the harbour 90366 (144, 47) files 4 plank and 6 stone orders before the matching of counter 46541 (finished, list 0); planks have a distribution table; free planks lie in its network, among them 96630 on (141, 45) | the harbour's plank orders | 46541: 96630 (storage-bound for 94359) gets the harbour as destination (recorded), and at 46552 96624 on (140, 46) as well. Choosing the harbour's orders by the plank table (the harbour is no consumer in it) chooses none, so no plank is matched to it: the donkey 92756 of the road (140, 46) - (145, 46) stays idle at 46565 and the Hard game against R3 differs from settlers 46566; with the plain rule for list 0 (experiment build, with T23) from 46751, buildings 46985 -> 48034 (the Hard 150 000 pair: settlers 46600 -> 46800, buildings 47000 -> 48200). In the Weak game the rule makes the castle send planks to the harbour 90351 at 50820 and 50853 as recorded (it sent stones there before), and so uncovers a later point: our harbour has a fifth plank order at 50985 where R5 sends a stone (Q-385); the Weak 150 000 generator's first difference moves 51472 -> 51297 from that, and with that point settled (`SHIP-ships-expeditions.md` T14) to 51724. Every other pair unchanged |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-sim-008 | code reading | | ECO-DIST-1.7 step 4, 1.7d, T12 |
| E-eco-036, E-trace-014 | code reading and recording | E-trace-014 `wares.csv`, `streets.csv` counters 25782 .. 25806 | ECO-DIST-1.6 (T22) |
| E-eco-011 | code reading | `S2DNG.exe` | warehouse flag command executors (all, toggle stop, toggle out, cycle, set stop, set out) |
| E-eco-011 | code reading | | flag record (create with defaults), accepts test, set for all goods of a type, load |
| E-eco-011 | code reading | | nearest accepting warehouse (finished, connected, accepts) / nearest with stock (no flag test) |
| E-eco-011 | code reading | | transport jobs by tick; take-out job; send ware out; send settler out |
| E-eco-011 | code reading | | storage trip re-check, 200 limit |
| E-eco-011 | code reading | | distribution: reset / set weight executors, table lookup (fish, bread, meat -> food), defaults, clamp |
| E-eco-011 | code reading | | order choice: table pick (weighted round robin, construction key), oldest order, coin rule |
| E-eco-011 | code reading | | matching loop: players, two lists, goods sorted by transport rank, one representative order per network |
| E-eco-011 | code reading | | transport rank lookup, move, move executor, defaults from goods of type ware / group |
| E-eco-011 | code reading | | computer player food table adjustments |
| data | script | `Goods.lua` `gp_setTransportPriority`; `buildings.lua` ids | defaults |
| data | text | `data/language/gb/dialogs.txt` dialog3_0, dialog19_0, dialog20_0 | UI meaning of the settings |
| E-eco-013 | code reading | | order list index (finished depot type = 0) stored per good group, list 0 bypasses table and coin rule; lowest stamp strict, unsigned |
| E-eco-013 | code reading | | new order stamp |
| E-eco-013 | code reading | | settler sent out by kind, no job |

## Open questions

- Destination of a settler taken out of a warehouse (ECO-DIST-1.7a: medium,
  by analogy with the new donkey).
