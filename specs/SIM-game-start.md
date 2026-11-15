# SIM-START: Start of a free game

Status: verified (the measured parts), draft (the random start positions)
Owner question(s): Q-312; the start sequence of the tick-for-tick comparison (2026-10-19)
Game build: 11757

## Summary

What the world holds when a free game begins, in which order it is made, and what the
first update changes. A reimplementation's trace must match the original's from trace
tick 1 (TOOL-trace TRACE-4.5): both castles, both flags and one garrison soldier per
castle standing on the castle's door field, the simulation generator at its seed.

## Rules

### SIM-START-1 Order of the start sequence

- **SIM-START-1.1** The world object exists before any start object. Its tick
  counter reads -1 until the first update. The start sequence runs while the counter
  reads -1: first the simulation generator is reset to its seed (SIM-RNG-2.2), then the
  players are created in slot order, each with its headquarters (ECO-HQ-1.1) on its
  start field, the headquarters' flag on the field in direction 1 (south-east,
  MAP-GRID-1.2) and the start stock (ECO-HQ-2); a computer player is activated after
  its headquarters exists. Then the first update runs (counter 0, trace tick 1).
  Confidence: high for the order reset -> players and for the counter (measured, live
  trace 2: a snapshot 50 ms before the first update held player 0's castle only; the
  generator stood at its seed through the first three ticks); medium for the
  activation order (code).
- **SIM-START-1.2** Object numbers come from one counter shared by every game object
  and are given in creation order. On MP_2P_Storm_Coast with the default set-up the
  castles are 90175 (player 0) and 90180 (player 1), the flags 90176 and 90182, the
  garrison soldiers 90186 and 90187; the numbers between them belong to parts of the
  flags and of the player's village record. Confidence: high for the numbers (measured);
  medium for what the gaps are. Traces compare settlers by place, not by number.
- **SIM-START-1.3** The garrison soldiers do not exist in the start sequence. They are
  taken out of the headquarters' stock by the first update (MIL-SOL-6.2: one soldier
  of each rank present; the medium start stock holds privates only, so one private per
  castle) and stand on the castle's door field from the end of the first update on
  (MIL-SOL-6.5). Confidence: high (measured: absent at counter -1, present with state
  "present", Stand, at the end of counter 0, trace tick 1).

### SIM-START-2 The world at trace tick 1

- **SIM-START-2.1** At the end of the first update (trace tick 1) a default free game
  on MP_2P_Storm_Coast holds exactly: two finished castles, two flags, two garrison
  soldiers, the map's deposits and animals (ECO-FOOD-5.13), no road, no ware on a
  flag, no settler outside the castles but the garrison soldiers. The castles'
  construction state is finished with progress 1000. Confidence: high (measured).
- **SIM-START-2.2** The generator has drawn nothing by the end of trace ticks 0, 1 and
  2 (state = seed 0x5432ACE0, draw index 0). The first draws are the animals' in
  trace tick 3 (ECO-FOOD-5.14; SIM-RNG-2.5). Confidence: high (measured, both live
  traces).
- **SIM-START-2.3** Trace tick 0 is not a complete world (TRACE-3.8): a trace of the
  original holds what the recorder caught while the start sequence ran. A
  reimplementation writes its complete set-up world as trace tick 0; comparisons
  start at trace tick 1. Confidence: high.

### SIM-START-3 Start fields

- **SIM-START-3.1** Each player's start field is taken from the map's start positions
  (FMT-S2M-2.2) in slot order: player 0 gets the first, player 1 the second. The
  generator is not used. Confidence: high (measured on MP_2P_Storm_Coast: player 0 at
  (29, 51), player 1 at (139, 51), no draw).
- **SIM-START-3.2** With the set-up option that shuffles start positions (a flag of the
  game set-up; whether the free-game screen of this build offers it is not known),
  each player in slot order draws from the simulation generator: `draw mod
  (candidates left)` picks one of the start positions not yet taken, which is then
  removed from the list. Because the reset (1.1) comes first, the choice is the same
  in every game with the same set-up. Confidence: medium (code; the option was not
  seen in play, Q-312).

### SIM-START-4 Object numbers

- **SIM-START-4.1** Every simulation object takes its number from one global counter
  when it is created (TOOL-trace TRACE-3.2), so the numbers are the creation order,
  and objects the tables do not show take numbers too. Observed on MP_2P_Storm_Coast
  (E-trace-005; the numbers are the low words, the counter had reached 90175 after the
  map's 261 animals 89914 .. 90174): castle 0 = 90175, its flag 90176 and three hidden
  objects 90177 .. 90179 (the flag's link objects, three per flag, every flag), 90181
  hidden, castle 1 = 90180, its flag 90182 (+ 90183 .. 90185), the garrison soldiers
  90186 and 90187. Later a building and its flag are consecutive (hunter 90190, flag
  90191; sawmill 90211, flag 90214 with 90212 .. 90213 before it), a road is one number
  (90199, 90224, 90228, ...), each settler one number, usually with one hidden number
  before it (builder 90202, carriers 90204, 90207, 90209, leveller 90231, carriers
  90234, 90237, ...), a respawned animal one number (90236, 90239, 90250, ...). The
  carrier number of TRN-CARRY-2.1 is this number, so an implementation must give its
  settlers the same numbers; which hidden objects take the numbers in between is Q-338.
  Confidence: high for the numbers seen (measured, E-trace-005 and E-trace-002, the
  same in both); the hidden objects are open.
  **In a save (answered 2026-11-06, Q-1025; E-sav-009).** The
  counter is a 64-bit number saved as `LogicSystem`'s `next_unique_id` (`FMT-savegame.md`
  FMT-SAV-2.2, `FMT-s2m-map.md` FMT-S2M-3.1) and set from it on load; a loaded game numbers its
  next new object from there, and its saved objects keep their saved numbers. A map file carries
  the same field (its largest id plus one, or more). Confidence: high.
- **SIM-START-4.2 Which creations take a number (Q-338, 2026-10-21).** Each of these
  takes the next number when it is made, in the order the creations run: a building
  (castles included); a flag: its own number, then three more for its three network
  entries (the all-roads, ware and land networks, SIM-START-4.3); a road; a settler;
  a request filed with a player's request queue (worker, builder, leveller, carrier,
  specialist and each unit of construction material alike); a ware entering transport
  (made at a warehouse's door or produced); an animal; a deposit; a doodad (sign,
  remains); a building's remains; a ship; a script trigger; and the computer player's
  own records: a need, the production record of a producing building, and a cell (made
  when the player's first building in that 16 x 16 cell is added, so the computer
  player's castle is followed by one number, the human's by none). Nothing else
  takes numbers. Confidence: high for the list (E-sim-004: nineteen constructors call
  the counter); medium for the moments at which requests, wares and the computer
  player's records are made, which the tables do not show (Q-340).
- **SIM-START-4.3 Network renumbering (measured E-trace-006, 2026-10-23).** Each flag
  entry of SIM-START-4.2 is the id of the road network the flag belongs to for that
  transport kind. Which roads each entry spans (renamed 2026-10-25, Q-352; every road
  carries three marks, set when it is made, E-ai-012, followed by the
  renumbering's flood): **entry 0, the all-roads network**: every road and
  every waterway; **entry 1, the ware network**: the roads with a carrier or donkey
  assigned (`TRN-carriers-transport.md` TRN-CARRY-5.1a; marked when the road gets its
  carrier); **entry 2, the land network**: land roads only, never a waterway. Walking
  settlers use entry 2, a boat carrier entry 0 (TRN-FREE-6.1, 6.4), ware routing and
  the computer player's tests entry 1. (The earlier names "carrier", "donkey" and "boat"
  network for entries 0, 1 and 2 did not fit what they span.) A renumbering takes one
  fresh number and writes it into that entry of every flag of the network (the flags
  reached over roads of that network from the given flag). They happen, in this order
  within one event:
  (a) a road is made: if the two end flags' land ids (entry 2) differ, the land network
  of the first end is renumbered; then if their all-roads ids (entry 0) differ, the
  all-roads network (for a waterway the land renumbering takes its number but joins
  nothing, the waterway not being part of it);
  (b) a road gets its carrier (the carrier request is served and the carrier is made
  at the warehouse door, SIM-START-4.4 (e)): if the two ends' ware ids (entry 1) differ,
  the ware network is renumbered (90205 at counter 207, 90235 at 417, 90238 at 447);
  (c) a road is removed: if it had a carrier, the ware network of each of its two
  ends is renumbered (two numbers, first end first: 90219, 90220 at 386); then the
  land and the all-roads network of one end are renumbered unconditionally (two
  numbers, 90221, 90222), so that the two halves get distinct ids.
  (d) a **road swap** (`TRN-carriers-transport.md` TRN-CARRY-3.6, carriers; not read
  for donkeys): the two roads lose their carriers, first the road the waiting carrier
  comes off (R1), then the other (R2); a road left without its carrier that has no
  donkey has the ware network of each of its ends renumbered (two numbers, as in (c));
  then R2 gets the waiting carrier and R1 the other, each as in (b) (one number when the
  road's ends' ware ids differ at that moment). Measured T15: five numbers in one swap
  (added 2026-10-29, E-sim-013).
  So a road joining two networks takes its own number and then two more (land, then
  all-roads); a road inside one network only its own; a removed road with a carrier
  takes four. A number written into an entry that a later renumbering of the same
  tick overwrites is hidden at the tick's end (386: 90221, 90222, 90225, 90226).
  Every renumbering of an entry also empties that player's stored route costs
  of that network kind (`TRN-carriers-transport.md` TRN-CARRY-5.3a; E-ai-017).
  Confidence: high (E-sim-004; E-sim-005; measured T7, T9).
- **SIM-START-4.4 When the hidden objects are made (Q-340; E-trace-006, 2026-10-23).**
  The moments, and the order within a tick, of the creations the tables do not show:
  (a) **Flag**: its own number, then its three network ids (carrier, donkey, boat).
  (b) **Building**: its number; then, for a computer player, the records its
  building-added hook makes (the cell record if this is the player's first building
  in that 16 x 16 cell; the production record of a producing building, then the need
  record of its input good: sawmill 90211 -> production (plank) 90212, need (wood)
  90213; castle 90180 -> cell 90181; a hunter's hut makes none); then its flag with
  its three ids. Which buildings make a production record (Q-263 a, E-sim-007):
  every building whose type has a product (its first product slot is set), except the
  hunter's hut; the record is followed by the need records of that product's inputs
  (AI-CON-2.5). So a woodcutter, a stone pit, a fisher, a sawmill each make one; the
  hunter's hut, the castle, storehouses and military buildings none (hunter's hut
  90716 at counter 5388 of E-trace-006: no record). If the building's flag falls on a
  road, the flag's numbers are those of a split, (j). The building's first request (its leveller if the site must be
  levelled, else its builder) is filed **in the next counter** (hut placed at
  counter 127 -> builder request 90195 at 128; sawmill placed at 323 -> leveller
  request 90218, seen at 325 after a two-tick recorder gap; hut 90244 placed at 596
  -> builder request 90249, seen at 598 after a gap).
  (c) **Road of a computer player**: a one-field planning stub of the road class at
  the start flag's field takes a number, then the road itself, then the
  renumberings of 4.3 (a) (counter 176: stub 90198, road 90199, land 90200, all-roads
  90201). The stub is made at **every** call of the computer player's road builder,
  whether or not a road is then built: the builder makes the stub, searches the path,
  and removes the stub again without a number, building nothing when no path is found
  or the path is longer than 2 x the distance (AI-CON-8.1), else building the road as
  above. The builder is called by the street construction (8.1) and by both variants of
  the route optimizer (8.3 c), so each of those calls takes one number (2026-10-25 night,
  Q-274: E-trace-006 / 008 counter 808, the direct variant's call for the castle's flag
  to the sawmill's flag, already joined: the counter rises by one and no object
  appears; E-sim-005). The path search of
  every such call also evens the ground along the path it finds, built or refused
  (`TRN-roads-flags.md` TRN-ROAD-3.3a). A human player's road takes no stub. The road's carrier request is filed
  in the next counter (90203 at 177; 90232 and 90233 at 387 for the two roads of 386;
  90256 at 646; 90259 for the road of 660, seen at 662 after a gap).
  (d) **Request**: numbered when filed. The only requests of this game's first 700
  counters are builders (dd2daebe), levellers (2740000e) and carriers (7bff15ce);
  construction material files no request (the "2 material requests" of the earlier
  T7 do not exist).
  (e) **Settler**: numbered when made at the warehouse door, in the tick its request
  is served (the request is gone in the same tick); a carrier's ware renumbering
  4.3 (b) follows in that tick. A waterway's boat carrier is given its road by the same
  step as a land carrier and joins the ware networks of the waterway's two flags in
  the same way (Q-263 c; E-sim-007: every road request served by a settler
  other than a donkey; not recorded, no waterway in the recordings, medium).
  (f') **Produced ware** (Q-263 d): numbered in the first tick of its worker's pick-up
  action after the work, the tick the worker takes it in hand (the put-down action of 4
  ticks before it, then the pick-up): flesh 90412 at 2202 (hunter 90379), wood 90416 at
  2314 and 90455 at 2821 (woodcutter 90392), plank 90520 at 3505 (sawyer 90431, inside
  the sawmill, before his walk to the flag at 3511), stone 90525 at 3593 (stonecutter
  90480); not when it is put on the flag. Confidence: high (E-trace-006, every case of
  the first 3600 counters). **Settlers on a ship (Q-345, 2026-10-24, from the code):**
  a ship carries settlers as counts in its store, not as objects, and takes them from
  the harbour's stock (a settler that walks into the harbour becomes a count there and
  its object goes), so a settler has no number on board. The expedition's
  builder gets a new number when the harbour is founded (the expedition's build
  command): the harbour site and its numbers first (a new site, (b)), then one builder
  is taken off the ship's store and made at the site (one settler number), then the
  ship's store moves into the site's stock and the site's material requests are
  cancelled (`SHIP-ships-expeditions.md`). Confidence: high for the order site, builder,
  stock (E-ship-001); medium for the site's own numbers in that tick and for settlers carried
  between harbours (counts into the destination's stock; the unloading was not read);
  no recording has a ship yet.
  (f) **Ware and its helper**: a material unit leaving a warehouse is a helper made at
  the door (one number) and then the ware (the next number): 90207 + 90208 at 242,
  90209 + 90210 at 275, 90240 + 90241 at 484, 90242 + 90243 at 517 (planks to the
  hunter's hut). No request precedes them.
  (g) **Computer player's needs**: every need the computer player creates is a
  record of its own (one object number per need). At counter 41 its stage-0 needs
  (`AI-construction.md` AI-CON-2.1) refer to its castle: stone (90188), then
  max(1, start sawmill count) planks, one record each: one (90189) for the Weak
  profile (count 1), three (90189, 90190, 90191, in E-trace-012's object snapshot of counter 49,
  which samples every 10 counters there) for the Hard one (profile 3, count 3) (corrected 2026-10-27 b: "one record per good,
  not per unit" held only because the Weak profile makes one plank need; T14).
  A **later** creation is a record of its own even when the consumer already has a
  need of that good: the stock check of `AI-construction.md` AI-CON-2.4 at counter
  12026 of E-trace-011 (game time just past 1200 s, plank stock 0 against 503 / 32 / 2)
  made need 91252, plank, on the castle, which still had its plank need 90189 (both
  in the heap dump of 12000; added 2026-10-27 night). Counting it as no new record
  (an experiment build) leaves every later number one lower, so carrier 91280 of road
  91278 becomes 91279 and his 20-tick work grid (`TRN-carriers-transport.md`
  TRN-CARRY-2.1) moves: settlers first differ at 13983 instead of 14530. Confidence:
  high for the stock check's needs, medium for other creators.
  (h) **Computer player's cells**: besides (b), a cell record is made when the site
  search first scans a cell: (8, 2) = 90196 at counter 149, (9, 2) = 90197 at 154,
  (9, 3) = 90206 at 209.
  (i) **Animal**: a respawned animal (90236 at 424, 90239 at 474, 90250 at 624).
  (j) **A flag put on a road** (a player's flag command, or a building whose flag falls
  on a road; Q-263 b, E-sim-007): the road is split by removing it and making
  two new roads, never by keeping its number. In this order: the old road's removal
  (4.3 (c): the ware ids of both ends if it had a carrier and no donkey, then the land
  and all-roads ids of one end); the new flag and its three ids; the first half, from the
  old road's first flag to the new flag, and its join renumberings (4.3 (a)); the second
  half, from the old road's second flag to the new flag (its field list runs from that
  flag), and its join renumberings; then the old carrier is given to the half holding
  its position, with that half's ware renumbering (4.3 (b)) when its two flags
  differ. The carrier keeps its own number and its work (it never notices the split,
  `TRN-carriers-transport.md` TRN-CARRY-7.1a does not apply). The other half files its
  carrier request in the next counter; a human player's split takes no stubs. Measured
  in E-trace-006: counter 1308 .. 1310 (building 90322, production 90323, donkey 90324
  and 90325, boat and carrier 90326, 90327, flag 90328 + 90329 .. 90331, road 90332 +
  90333, 90334, road 90335 + boat 90336 + carrier 90337, donkey 90338 for carrier 90315;
  then builder request 90339 and carrier request 90340: 19 numbers), 4334 .. 4336
  (building 90590 with production and need, 21 numbers with its leveller request and
  leveller), 5388 (hunter's hut 90716, no record, 14 numbers; then builder request,
  builder and carrier request in 5389). Confidence: high.
  Checked over the whole recording (counters 0 .. 5999): this rule accounts for every
  number (the road-rebuild counters 386, 1310, 1543, 4336 and 5388 included: 386 and
  1541 .. 1543 are a removal and two new roads of the computer player, 4.3; the other
  three are splits by a building's flag, (j); Q-342) except one number in each of 23
  counters (808, 1439, 1927,
  2031, 2312, 2441, 2580, 2917, 3218, 3511, 3589, 3694, 4063, 4129, 4399, 4893, 5188,
  5322, 5389, 5544, 5582, 5803, 5989; Q-341). Confidence: high for (a), (c), (e),
  (f), (g), (h), (i) (every tick recorded, the makers read: E-sim-005); medium for the next-counter filing of
  (b) and (c) (3 buildings and 5 roads, 4 of them seen through recorder gaps) and for
  the order of (b)'s records before the flag (the castle and the sawmill only).
- **SIM-START-4.5 The counter at the start (Q-261; E-trace-006, 2026-10-23).** The
  counter is 0 at the main menu and stays 0 while the map's deposits, doodads and
  animals are loaded: they keep the ids stored in the map file (Storm Coast:
  deposits 11 .. 89913, doodads 196 .. 89876, the 261 animals 89914 .. 90174) and
  take no number. Then, still during loading, the counter is set to the map file's
  own next id (FMT-S2M-3.1: 90175 on Storm Coast) and the first castle takes that
  number. So for any map the first castle's number is the map file's next id, and
  nothing takes a number before the animals. Confidence: high (E-trace-006
  `counter-menu.csv`: 0 at the menu, 0 -> 90175 -> 90176 -> 90180 -> 90181 -> 90186
  during loading; a map reader reads 90175 from the file).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| counter before the first update | -1 | ticks | high | E-trace-002 |
| draws before trace tick 3 | 0 | draws | high | E-trace-002 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | MP_2P_Storm_Coast, default set-up (player 0 human, player 1 computer, medium) | world after the first update (trace tick 1) | castle of player 0 at (29, 51), of player 1 at (139, 51), both finished, progress 1000; flags at (30, 50) and (140, 50); one good_soldier0 per castle, state present, animation Stand, at (29, 52) fine (116000, 208000) and (139, 52) fine (556000, 208000); no other settler, building, road or ware |
| T2 | as T1 | generator at the end of trace ticks 0, 1, 2, 3 | state 1412607200 (the seed), draw index 0, 0, 0; tick 3: 3 draws, state 1054465454 |
| T3 | as T1 | object numbers | castle 0 < flag 0 < castle 1 < flag 1 < soldier 0 < soldier 1 (90175, 90176, 90180, 90182, 90186, 90187 in the original) |
| T4 | a map with 4 start positions, 2 players, start positions in order | start fields | positions 0 and 1 |
| T5 | as T4 with shuffled start positions | start fields | player 0: `draw mod 4`-th position; player 1: `draw mod 3`-th of the remaining three (draws 1 and 2 of the game) |
| T7 | SIM-START-4.2, 4.3 on the first 600 numbers of E-trace-005 (every id of the settlers, buildings, flags, roads and animals tables) | hidden numbers between the visible ones | every one of the 22 flags is followed by >= 3 hidden numbers; of the 28 roads the 20 that join two networks are followed by >= 2 hidden numbers (14 by exactly 2), the 8 inside one network by >= 0; castle 90180 (computer player) by 1 (90181, its cell), castle 90175 by 0; the hunter's hut 90190 by its flag and 7 hidden = 3 network ids + builder request + 2 cells + road stub; the sawmill 90211 by 2 hidden (production, need), its flag 90214 and 9 hidden = 3 network ids + leveller request + 2 ware + land + all-roads renumberings + road stub; the first road 90199 by 90200, 90201 (land, all-roads) and the builder 90202 |
| T6 | SIM-START-4.1, 4.4, MP_2P_Storm_Coast default set-up (E-trace-005; every hidden number named by E-trace-006, the same game) | object numbers in creation order (low words), with the counter of their creation | 90175 castle 0, 90176 flag 0, 90177 .. 90179 its all-roads / ware / land ids, 90180 castle 1, 90181 cell (8, 3), 90182 flag 1, 90183 .. 90185 its ids, 90186 soldier 0, 90187 soldier 1 (all counter -1 / 0); 90188 need stone, 90189 need plank (41); 90190 hunter site (127), 90191 its flag, 90192 .. 90194 its ids (127), 90195 builder request (128), 90196 cell (8, 2) (149), 90197 cell (9, 2) (154), 90198 road stub, 90199 road, 90200 land, 90201 all-roads renumbering (176), 90202 builder, 90203 carrier request (177), 90204 carrier, 90205 ware renumbering (207), 90206 cell (9, 3) (209), 90207 helper, 90208 ware plank (242), 90209 helper, 90210 ware plank (275), 90211 sawmill site, 90212 production plank, 90213 need wood, 90214 its flag, 90215 .. 90217 its ids (323), 90218 leveller request (324, in a gap), 90219 ware (flag 90191), 90220 ware (flag 90182), 90221 land, 90222 all-roads (one end, overwritten), 90223 road stub, 90224 road, 90225 land, 90226 all-roads (overwritten), 90227 road stub, 90228 road, 90229 land, 90230 all-roads renumbering (386), 90231 builder, 90232 carrier request (road 90224), 90233 carrier request (road 90228) (387), 90234 carrier, 90235 ware renumbering (417), 90236 animal (424), 90237 carrier, 90238 ware renumbering (447), 90239 animal (474), 90240 helper, 90241 ware plank (484), 90242 helper, 90243 ware plank (517), 90244 hunter site, 90245 its flag, 90246 .. 90248 its ids (596), 90249 builder request (597), 90250 animal (624), 90251 road stub, 90252 road, 90253 land, 90254 all-roads renumbering (645), 90255 builder, 90256 carrier request (646), 90257 road stub, 90258 road (660), 90259 carrier request (661) |
| T8 | SIM-START-4.5, MP_2P_Storm_Coast | the counter at the main menu, during loading and at the first castle | 0 at the menu and while the deposits (ids 11 .. 89913), doodads (196 .. 89876) and animals (89914 .. 90174) are loaded; then 90175 = the map file's next id; the first castle is 90175 |
| T9 | SIM-START-4.3, 4.4 (c): the computer player removes road 90199 (flags 90191 - 90182, carrier 90204) and builds roads 90214 - 90182 and 90214 - 90191 in one counter (386) | the numbers taken in that counter, in order | 12: ware id of flag 90191, ware id of flag 90182, land and all-roads id of one end, stub, road 90224, land, all-roads, stub, road 90228, land, all-roads; at the end of the tick only 90219, 90220, 90229, 90230 remain as flag entries |
| T10 | SIM-START-4.4 (g), (h): computer player on MP_2P_Storm_Coast | its need and cell records in the first 210 counters | counter 41: need stone 90188, need plank 90189 (both for the castle); cells (8, 3) = 90181 at the castle, (8, 2) = 90196 at 149, (9, 2) = 90197 at 154, (9, 3) = 90206 at 209; no other record until the sawmill (production 90212, need 90213 at 323) |
| T11 | SIM-START-4.4 (j), E-trace-006 counters 1308 .. 1310: the computer player places building 90322 (production good stone) whose flag field (141,56) lies on its road 90311 (90301 - 90271, "139,56 140,56 141,56 141,55 142,55", carrier 90315, no donkey) | the numbers 90322 .. 90340 and the roads | building 90322, production 90323, ware ids 90324 (stays on flag 90301) and 90325, land and all-roads 90326, 90327, flag 90328 and 90329 .. 90331, road 90332 (90301 - 90328, "139,56 140,56 141,56") with 90333, 90334, road 90335 (90271 - 90328, "142,55 141,55 141,56") with land 90336 and all-roads 90337 (on all 7 flags), ware 90338 (6 flags, not 90301) as carrier 90315 goes to 90335; next counter builder request 90339, carrier request 90340 for 90332; counter 90341 after |
| T12 | SIM-START-4.4 (b), (f'), E-trace-006 | production records and produced wares | hunter's hut 90716 (counter 5388) makes no production record; the sawmill's plank 90520 is numbered at counter 3505, the first tick of the sawyer's pick-up, 6 ticks before his walk to the flag |
| T13 | SIM-START-4.4 (b), (j), 4.2: E-trace-010 (two-player game, computer player Weak), counter 4334: the computer player places a building whose flag (141, 52) falls on its road 90277 (with carrier) | the numbers of that counter and the next | building 90590, its production record 90591, its need 90592, then the split: 90593 .. 90596 (the removed road's renumberings, 90594 a ware-network id), flag 90597 and its three ids, the halves 90601 and 90604 with their renumberings, 90605 .. 90607 (network ids); counter 4335: 90608 .. 90610 (requests; 90610 the carrier request of 90604); the settler made at trace tick 4336 is 90609 (measured, `counter.csv` and `objects.csv`). Numbering the two records in the player's next update instead puts them after 4335's numbers and makes that settler 90607; every later settler number then differs by two, which moves carriers' 20-tick grids (`TRN-carriers-transport.md` TRN-CARRY-2.1). The farm site of counter 3724 shows the same order (building 90538, production record 90539, flag 90540) |
| T14 | SIM-START-4.4 (g), AI-CON-2.1: MP_2P_Storm_Coast, player 1 a computer player of the Hard profile (profile 3, start sawmill count 3; E-trace-012, `specs/traces/original-hard-dense-20000*`) | the need records of its stage 0 | in the object snapshot of counter 49 (the first after 41; snapshots every 10 counters before 5500): need 90188 stone, then 90189, 90190, 90191 plank, all for the castle 90180; the next number is the castle flag's network ids 90194 .. 90196 and request 90197 at counter 129 (the Weak game: 90189 the only plank need, request 90195). Counting one record for the three plank needs leaves every later number two lower: the carrier of (139, 49) becomes 90204 instead of 90206 and takes his 20-tick look 2 ticks off (settlers first differ at trace tick 300; with the rule an experiment build matches to 402) |
| T15 | SIM-START-4.3 (d): E-trace-013 (R3, the Hard game), counter 12690: the road swap of `TRN-carriers-transport.md` T44 (90510 waiting at the full flag (137, 56) with the corn for (137, 58), 91018 idle on the road (137, 56) - (137, 58), neither road with a donkey) | the object counter | 91665 -> 91670 in that counter: five numbers (R1 loses its carrier: 2; R2: 2; R2 gets 90510: 1, its ends now apart; R1 gets 91018: none, its ends still joined over other roads); the recorder's network table: ware network 91660 gone, 91669 new (counter.csv, objects.csv). Without the swap's renumbering every later number is 5 lower: the donkey 91701 instead of 91706 phases its idle checks 5 ticks late (TRN-CARRY-2.1) and the Hard pair's settlers differ from 13056; with it the numbers equal R3's to 14100 (experiment build) and the Hard pair's settlers (without the animation column) differ from 13395, buildings 14327 -> 14433, generator 14064 -> 15534 (R3 13060 -> 13410, generator 15534), the other pairs unchanged |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-002 | live read of the running game | `buildings`, `flags`, `settlers`, `players` tables at counters -1 and 0; `generator.csv`; `original.s2trace.gz` ticks 0 to 3 | start objects, their fields, the soldiers' door field, the generator |
| E-trace-001 | live read of the running game | | the same start in another session: identical |
| E-sim-001 | code reading | | start sequence order; start position choice in order or by draw |
| E-trace-005 | live read of the running game | every table every tick; the first tick each object number appears | SIM-START-4.1, T6, T7 |
| E-sim-004 | code reading | the callers of the object counter | SIM-START-4.2, 4.3 |
| TOOL-trace TRACE-3.8 | spec | `TOOL-trace.md` | the half-made world at counter -1 |
| E-trace-006 | live read of the running game | the same game as E-trace-005: generator equal in all 6001 ticks; the object tables `objects`, `counter`, `goodflags` every tick, `counter-menu.csv` at the menu and during loading; published as `specs/traces/original-dense-6000-objects.csv.gz` and siblings | SIM-START-4.3, 4.4, 4.5, T6, T8, T9, T10 |
| E-sim-005 | code reading | road building (computer player with its stub, human without), the road constructor, network joins, detaches and renumbering | SIM-START-4.3, 4.4 |
| E-sim-007 | code reading and recording | production records, the road split, road requests; E-trace-006 `objects.csv.gz`, `counter.csv.gz`, `settlers.csv.gz` | SIM-START-4.4 (b), (e), (f'), (j); T11, T12 |
| E-sim-013, E-trace-013 | code reading and recording | the waiting carrier's checks (the road swap), a road losing or getting its carrier; E-trace-013 `counter.csv.gz`, `objects.csv.gz` (counter 12690) | SIM-START-4.3 (d) (T15) |
| E-sav-009 | code reading | (= E-gfx-039): taking a number, saving and loading the object counter | SIM-START-4.1 |

## Open questions

- Q-341: one number of a kind the recorder does not read in each of 23 counters
  (SIM-START-4.4); Q-345: the number of a builder leaving a ship at a landing.

- Q-312: whether the free-game set-up screen can switch the shuffled start positions on
  (SIM-START-3.2); if it can, play a 4-start map twice with it and compare the castles.
