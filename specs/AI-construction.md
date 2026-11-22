# AI-CON: Computer opponent, construction

Status: draft (decision rules read from the binary; numbers from the code and the shipped scripts; not yet checked against the running game)
Owner question(s): Q-067 (construction part), Q-068, Q-071, Q-072 to Q-076; plan step 5.1 chunks 1 and 4; new Q-081, Q-082
Game build: 11757

## Summary

The computer player cuts the map into cells of 16 x 16 fields and keeps three
kinds of planning objects: **needs** (a good it wants produced, with a
consumer), **productions** (a planned or existing producer of a good in a
cell) and its view of its own **buildings**. Needs are created from a fixed
start script and from stock checks, given a cell by the "needs arrangement",
turned into construction orders by the "needs construction", and orders are
executed one at a time by the construction system, which scans the order's
cell for the best site. Roads are built every 7 ticks from unconnected
building flags to the nearest flag of the warehouse network; two optional
optimizers add flags and shortcut roads. A destruction system tears down one
useless resource building per 41 ticks and one useless flag per tick. The
schedule (which part runs in which tick) is in `AI-overview.md`; this file
gives the decisions.

## Rules

"stock(g)" is the player's count of good g inside buildings (the count the
script function `pl_getGoodAmount` returns, AI-CON-1.7); "stock2(g)" is the
number of the player's wares of g outside buildings, lying at a flag or being
carried (AI-CON-1.7). "S" is the number of fields the player owns, as the
statistics last computed it (AI-CON-1.8). Distances are hex distances
(`MAP-grid-geometry.md`). Goods and building ids are the `Goods.lua` /
`buildings.lua` names.

### AI-CON-1 Planning model

- **AI-CON-1.1 Cells.** The map is divided into cells of 16 x 16 fields (cell
  coordinates = field coordinates / 16; the cell's **position** is a field
  inside it, AI-CON-1.1a). A cell exists for the player as soon as one of the four corner
  fields of its square is owned by the player. **The corner fields are (16 cx, 16 cy),
  (16 cx + 16, 16 cy), (16 cx, 16 cy + 16) and (16 cx + 16, 16 cy + 16)**: the cell's
  origin and the origins of the next cells to the right and below, not the square's own
  last row and column; corners off the map do not count (corrected 2026-10-29, E-ai-051, measured T82) (checked by the resource system
  for one grid position per 5 ticks: a cursor like AI-CON-7.3's, starting at
  (0, 0) and advanced before each check, cycling over the whole grid, AI-2.4b;
  the cell of a building is created when the building is added, so the home
  cell exists from the start), and is removed when its depot,
  military and ordinary building lists are empty, it has no forester and no
  corner is owned. Each cell keeps four building lists, a building being in
  exactly one of them, decided in this order (Q-160): military (soldier limit
  and territory range above 0 in the building type; the headquarters has a
  territory range but no soldier limit in the shipped `buildings.lua`, so it is
  **not** military but a depot: corrected 2026-10-24, E-sim-007, measured by AI-MIL-6.1 choosing the castle in E-trace-006),
  harbour (a harbour is a depot by type but goes to the harbour
  list), depot (storehouse), the forester (a planting building: no product,
  planting good set) as a single reference, the hunter (by type) as a single
  reference, and **everything else is an ordinary building** (farms and
  mines included). It also keeps its productions and needs, a
  "full" mark per good, and four timed
  markers: *depot wanted*, *crowded*, *clearing* and *harbour wanted*. The
  resource map is finer: one record per 4 x 4 square with, per good, kind
  (1 underground, 2 surface) and amount. Confidence: high (cell size, grid
  scan, markers, lists); medium (removal condition).
- **AI-CON-1.1c The resource map's contents (Q-365).** A square's record is
  made from **every field of the square, explored or not**: per field, a
  deposit on it (a tree or stone, TRN-FREE) gives an entry of the deposit's
  good, kind 2 (surface), whose **amount is 1 plus the number of next-deposit
  links of its type** (`dp_nextDeposit`, followed to the end): a stone pile
  `stone_06` counts 6, `stone_01` 1, a tree 1 (added 2026-10-27 b, E-ai-035, measured T57; was: 1 per deposit); a field without a deposit gives its underground
  resource (the map's resource layer, water excepted), kind 1; each entry also
  keeps the field's stored landmass index (1.1b). The squares are refreshed by
  a cursor (x fastest, wrapping), **3 per tick** (the count is a field of the
  map, 3 in every recording and save read), after all computer players have
  thought in that tick (corrected 2026-10-27 night, E-ai-030; measured: the cursor goes from (20, 16) to (19, 10) between the
  heap dumps of 9500 and 10000 of E-trace-011, 1590 squares in 500 ticks).
  Confidence: high for the contents (code E-eco-019;
  measured, Q-365: counting only explored fields leaves out the coal on
  (119, 49) and (118, 51) and moves player 1's coal target of counter 4847 from
  (116, 48) to (124, 48), the heap dump of 5500 holds (116, 48)); medium for the
  number of squares per tick (now read: 3; 8 also reproduced the recording to
  6000) and for a field with both a deposit and an underground
  resource giving only the surface entry (code, not seen in a recording).
  The **cell-coordinate resource map** (one record per 16 x 16 coordinate, the
  one 3.5 and 7.3 read) holds per good, kind and stored landmass index the sum
  of the amounts of all the coordinate's fields, explored or not: measured,
  E-trace-010 heap dump 5000, player 1, coal (kind 1): (7, 3) landmass 12 252,
  93 7, 96 14, 100 7, -1 28; (7, 5) 12 497, -1 35, 115 14, 119 14; (1, 8)
  12 182; (6, 4) 12 168; (7, 2) 12 98; (8, 3) 12 84, -1 7: each equals the sum
  of the map file's coal amounts (7 per field) over the coordinate's fields
  with that stored index (Q-278). **It is not summed when read**: it keeps
  one record per coordinate, and **one coordinate is refreshed every 16
  counts** (when 3 x the AI counter is a multiple of 16, i.e. the counter
  c = k + 1 of AI-2.1a is), from the squares as they are then, by its own
  cursor (x fastest, wrapping): a full round takes 16 x the number of
  coordinates (121 x 16 = 1936 ticks on a 170 map), so its counts lag the
  squares by up to that (added 2026-10-27 night, E-ai-030; measured T52: the cursor advanced 31 and 32
  coordinates between the dumps of 9500, 10000 and 10500). Confidence: high.
- **AI-CON-1.1a Cell position (Q-250).** Every cell keeps a position field,
  the start of the site search's routes (AI-CON-5.4) and the field whose
  landmass the cell's resource counts use (7.3). A cell made for a building
  (the building's cell did not exist when the building was added, e.g. the
  home cell for the headquarters) starts without a position; every building
  added to one of the cell's four lists (military, harbour, depot or
  ordinary; **not** the forester or the hunter, which the cell keeps as single
  references: corrected 2026-10-27 night, E-ai-026,
  measured T49) sets the position to the building's field when that field is strictly nearer
  (lattice distance) to the cell's centre field (16 cx + 8, 16 cy + 8) than
  the current position (an unset position counts as far). So the home cell's
  position is the headquarters' field until a building nearer to the centre is
  added. A cell made by the validity scan (1.1) gets the first field of its
  square's border that the player owns and that lies on the landmass of a
  neighbouring cell's position (its stored index equals the index at that
  position, 1.1b): the top row left to right, then the bottom row,
  the left column top to bottom, then the right column; the neighbouring cells
  are tried in the order cx - 1 .. cx + 1 (outer) and cy - 1 .. cy + 1 (inner),
  existing cells with cx, cy > 0 only. If no field qualifies, the check makes
  no cell (E-ai-016); so every cell has a position from the moment
  it is made or gets its first building (a new cell's position is (-1, -1)
  until then, its landmass -1). Confidence: high for the building rule
  (E-sim-002, confirmed by T23), high for the border scan's "no
  field, no cell" (E-sim-002; measured 2026-10-26 night,
  E-trace-011: player 1's coordinate (9, 4) has an owned corner from about
  counter 6313 but no qualifying border field, and the original makes no cell
  there (its object numbers run on without one); an experiment build that made
  the cell without a position took one object number too many from 6314 and
  equals the recording's numbers to 90823 once it makes none), medium for the
  scan order (not yet distinguished by a recording).
- **AI-CON-1.1b Landmass (2026-10-25 evening, E-ai-016).** Every landmass
  test of the computer player compares the map's stored continent indices
  (`FMT-s2m-map.md` FMT-S2M-4.8, used as loaded: FMT-S2M-4.9) for equality:
  two fields are on the same landmass when their stored indices are equal,
  so a field marked -1 matches every other -1 field and no other. (a) A
  **field's landmass** is its stored index. (b) A **cell's landmass** is the
  stored index at the first position the cell gets (1.1a); it is set once,
  when the cell gets a position while its landmass is still unset (stored as
  -1, so an index -1 there is set again at the next position),
  and does not follow later moves of the position. (c) The **home landmass**
  is the stored index at the home position, recorded at the activation
  (`AI-misc.md` AI-MISC-2.1) and never updated. Which landmass each rule
  uses: the border scan of 1.1a the index at the neighbouring cell's
  position; the harbour marker (1.6) the cell's; the square test of 3.5 the
  resource entries' indices against the conquered continents; 6.3 the
  site's; 7.1 and 7.3 the index at the cell's current position; the partner
  of 8.3 b (k = 1) B's field's; `AI-military.md` AI-MIL-4.2 the home field's
  index (read from the layer, the same value as (c)), AI-MIL-4.3 the site's,
  AI-MIL-6.1 and `AI-misc.md` AI-MISC-1.2 the cells' landmass (b) against
  (c); outside the computer player, the warehouse search of
  `TRN-free-walk.md` TRN-FREE-6.4 step 3 the settler's field's. Confidence:
  medium (code; the indices themselves measured, FMT-S2M-4.9).
- **AI-CON-1.2 Need.** A need has a good, a consumer building, optionally a
  cell, a resource square (4 x 4) inside that cell for resource goods, a
  production once a producer is planned, a "waiting for expansion" flag and
  optionally a linked food need. **Production**: a good, a cell, a building
  (once placed) and its need. **Building record**: the game building, its
  cell, its needs and productions and four protection flags refreshed every
  tick: military (soldier limit and territory range above 0), depot, planting
  building (no product and a planting good: the forester; **not the farm**,
  whose type has a product, grain; corrected 2026-10-27 night, E-ai-023, measured T46) and harbour. The depot flag is the building
  type's depot flag: the headquarters, storehouses **and harbours** have it,
  whichever cell list the building is filed in (AI-CON-1.1; T45).
  Confidence: high.
- **AI-CON-1.3 Per-tick update (Q-071).** The per-tick update of buildings,
  needs, productions and cells only re-resolves their references to game
  objects and to each other (dropping ids that no longer exist) and, for
  buildings, recomputes the four protection flags. No decision is taken in
  it. Confidence: high.
- **AI-CON-1.4 Producer tables (Q-074).** Two tables map a good to the
  building type that produces it, built when the AI player is created or
  loaded, from every good in good-id order: the *mine table* holds the first
  building type, in type-id order, among the types of size "mine" plus the
  fisher, whose products (the up to four product slots) include the good; the
  *normal table* holds the first type among all other types whose products
  include the good. Exception: flesh has the slaughterhouse in the normal
  table and no mine entry (the hunter is never planned as a flesh producer).
  So stone maps to the stone pit / the stone mine, fish only to the fisher
  (mine table; fish lies in the resource map as an underground good), wood to
  the woodcutter. The mine table is used when the need's resource square
  holds the good as an underground resource (kind 1), else the normal table;
  the worker check (AI-CON-4.1) looks in the normal table first, then the
  mine table. A good without entry has no producer. Confidence: high.
- **AI-CON-1.5 Goods flags.** From `Goods.lua`: *deposits* goods are wood,
  stone, corn; *fields* goods are stone, fish, coal, ironOre, gold. "Resource
  goods" below = deposits or fields goods except corn. Group goods: fish,
  flesh, bread belong to food; the tools belong to tools. Confidence: high.
- **AI-CON-1.6 Cell markers.** *Depot wanted* is set when a cell is created
  and when its depot list becomes empty, and cleared when a depot is added.
  *Crowded*: evaluated when a military, harbour or ordinary building is
  added **and when a military or ordinary building is removed** (corrected
  2026-11-01, T96: was "added" only; a removed harbour or depot does not
  evaluate it): if 5 x (military buildings in the cell) < (ordinary buildings)
  + (1 if the cell has a harbour) the marker is set, otherwise cleared.
  Setting a marker that is already set changes nothing (its time and weight
  are kept); setting a clear one gives it the current game time and weight 1,
  then 100 if the cell has a harbour and no military building. A cleared
  marker keeps its old time, unused until it is set again (E-ai-061; measured, T96). *Harbour
  wanted* (corrected 2026-10-25 evening, E-ai-016): when a cell's
  landmass is set (1.1b (b)), if the map's continent object of that index
  is land (`FMT-s2m-map.md` FMT-S2M-4.9, is-water false), each of its
  positions (the object's field cursors, the harbour spots) that lies in
  this cell sets the marker, if the cell has no harbour and the marker is not
  already set. Spots of other landmasses in the cell, and whether that
  landmass has a harbour elsewhere, do not matter. It is also set when the
  cell's harbour list becomes empty and cleared when a harbour is added.
  *Clearing* is never set (AI-CON-9.4). A *full* mark for good g on a cell means
  "no site was found for a producer of g here". It keeps the game time at
  which it was set and a count of how often it has expired (0 for a new mark;
  the count is never reset). The 37-tick check (AI-2.4) clears an active mark
  when its time + count x 600 s is less than the current game time, and
  raises its count by 1. Setting a mark that is already active changes
  nothing (its time is kept); setting an inactive one makes it active with
  the current time and leaves the count. So the **first** mark of a good in
  a cell lasts only until the next check (at most 37 ticks), the second
  600 s, the third 1200 s, and so on (corrected 2026-10-27 night: was "600 s
  the first time, then 1200 s, ..."; E-ai-031; measured, T53). Confidence: high (depot, crowded, full);
  medium (harbour).
- **AI-CON-1.7 Stock counts (Q-072).** Every player keeps two per-good
  counts, recomputed on a 22-tick cycle: at cycle tick 0, stock(g) = the sum
  of the stores of the player's **warehouses** (headquarters, storehouses,
  harbours) for g; workshop inputs and the materials at construction sites
  are **not** counted (corrected 2026-10-27 night: measured on the recorded
  player stock map, TOOL-trace `stocks` player stock_a, in E-trace-011: wood
  23 at counter 6000 with the castle's store 23 and 6 in the sawmill's inputs;
  planks 1 at 7695 with the castle's 1 and one at the upgrading guard house
  90300; the experiment build counting inputs and sites keeps a crowded
  target that the original drops, AI-MIL T18);
  at cycle tick 2, stock2(g) = the number of the player's ware objects of g,
  which are the wares outside buildings (lying at a flag or carried).
  **Which wares, and which state both counts read (corrected 2026-10-31 night,
  T91, T92):** stock2 counts every ware object of the player, whoever holds it:
  wares lying at flags; wares carried by road carriers and donkeys; a ware a
  warehouse's helper carries between its door and its flag; a product a worker
  carries to his flag or home, from the tick in which he **picks it up**, right
  after the 4 ticks of putting it down that end his work (at the workshop or at
  the work spot; not while he is still putting it down); the plank a shipwright
  carries out to the ship, from leaving with it until he has put it down there
  (it is gone in the tick his pick-up there begins; not while he works); and
  the remains of a torn-down building lying at its flag (AI-CON-9.3). Counting
  only lying wares and road carriers' wares leaves the helpers' and the workers'
  wares out (T91). Both counts read the state **as the previous tick left it**:
  the player's update is the first step of the logic tick (AI-2.1a has the AI
  system as its last step), so a count made in trace tick R sees the stores and
  ware objects as they stood at the end of trace tick R - 1, including what the
  computer players did in R - 1's think, and nothing of tick R itself (a ware made
  or a store changed in tick R is counted at the next refresh). Measured on the
  original's tables alone (E-trace-018
  40 000 .. 42 000 and E-trace-014 20 000 .. 32 000, every refresh): stock(g)
  equals the sum of the player's warehouse stores in the table of the tick before
  in all 16 748 counts (9 differ against the same tick's table), stock2(g) the
  number of the player's ware objects in the table of the tick before in all
  5 129 (51 differ against the same tick's). Confidence: high. Both
  therefore lag the game by up to 22 ticks; `pl_getGoodAmount` returns the
  lagging stock(g). **Start of the cycle (Q-332; E-sim-002):** the
  cycle counter starts at 0 and is advanced before the phase test, so the
  first sum is made in the player's 22nd update (stock2 in the 24th); until
  then every count reads 0. No need exists that early (the first need
  creation runs at counter 41, AI-2.3), so the empty start changes no
  decision in E-trace-002; the wood need behind the woodCutter of P 744
  comes from AI-CON-2.5 (Q-252; the earlier reading that the arrangement
  of k 7 made it was wrong). Confidence: high (what is counted; the
  counter), medium (that sites' materials are included: every building has
  a stock map; the player's first update is the game's first tick).
- **AI-CON-1.8 Owned fields S (Q-073).** The statistics run a 601-tick cycle;
  in its first tick they count, per player, the map fields the player owns
  (water fields and **fields without a continent** excluded: the map's
  continent layer, `FMT-s2m-map.md` FMT-S2M-4.8, gives them none, -1:
  mountains and other blocked ground; corrected 2026-10-28 night, measured T76)
  and store the count as S (the "land" statistic shows it as a percentage of all
  countable fields). S is 0 until the first count and then changes only every
  601 ticks. Confidence: high (count and period), medium (the excluded
  regions are water).

### AI-CON-2 Need creation (every 47 ticks)

t is the game time in seconds. New needs are attached to the first building
of the first cell (the headquarters) as their consumer (not so the
locksmith's tools need, AI-CON-10.2, which has no consumer). Stages advance once:

- **AI-CON-2.1** Stage 0 (t >= 0): one need for stone and max(1, start
  sawmill count of the profile) needs for plank, each a need record of its own
  (`SIM-game-start.md` SIM-START-4.4 (g); Hard: three plank records, measured
  T14 there). Confidence: high.
- **AI-CON-2.2** Stage 1 (t >= 300) and some cell has a producer of stone,
  of wood and of plank: if the profile's maximum smithies is above 0, one
  need for sword and one for beer. Confidence: high.
- **AI-CON-2.3** Stage 2 (t >= 800): one need for donkey. Stage 3 (t >=
  1200): if the profile's maximum mints is above 0, one need for coin.
  Confidence: high.
- **AI-CON-2.4 Stock checks.** With a counter k starting at 1 and doubling
  each time it fires: when t > 600 x k: if stock(plank) < trunc(S x 0.03125)
  / max(1, P) a need for plank is created; if stock(stone) < trunc(S x 0.0625)
  / max(1, Q) a need for stone. P and Q count **needs, not producers**: the
  needs of plank (of stone) that stand in a cell's need list, summed over all
  cells, each need counted when its good is the good or the good's group is
  (the count of AI-CON-3.3). A producer that came without such a need (placed
  by another part of the player) does not count, and a need in a cell counts
  whether its producer is placed yet or not (corrected 2026-10-30, T86; was
  "the number of plank producers": E-ai-055).
  Integer division; S as in AI-CON-1.8. Each such need is a new need record (an
  object with a number) even when the headquarters already has a need of that good
  (`SIM-game-start.md` SIM-START-4.4 (g); measured 2026-10-27 night, plank need
  91252 at counter 12026 of E-trace-011). Confidence: high.
- **AI-CON-2.5a No production for a hunter's hut.** Every building whose
  type has a product gets a production of that good in its cell when it is
  added (`SIM-game-start.md` SIM-START-4.4 b for its number), **except the
  hunter's hut**: it is only its cell's hunter reference (1.1) and supplies no
  good to the counts of 1.4 and 3.2. Confidence: high (E-trace-010's object
  table: no production record for the huts 90190, 90244, 90716, while every
  other producer's appears in its placement counter; SIM-START T12; Q-365).
- **AI-CON-2.5 Needs of a new building (Q-252).** When a building of the
  computer player is announced (AI-MISC-2.2, so also a construction site in
  the tick it is placed), needs are tied to it as their consumer, outside
  the 47-tick schedule:
  - if its type has a product and it is not a hunter: one need for every
    input of the product (`gp_addCost` list of the product good);
  - if its type is of the mine class and its product is stone (the granite
    mine): one need for food;
  - if it is a planting building (no product, planting good set,
    AI-CON-1.2): one need for a good its type names for this (taken to be
    the planting good; not checked).
  For each such good, the player's first need of that good that has no
  consumer building yet is taken and tied to the new building; only if
  there is none is a new need created (no cell, created now) and tied to
  it. There is no stock test (unlike AI-CON-3.2). So a sawmill site makes a
  wood need at once, a woodcutter or stone pit site makes none (wood and
  stone have no inputs), the headquarters makes none (no product). The new
  need gets its cell from the next needs arrangement (AI-CON-3) and can then
  win the needs construction (AI-CON-4). Measured: E-trace-002 site 5 (T25).
  Confidence: high for the first case (E-ai-008; measured T25), medium for the mine and planting
  cases (code only; the planting good is read from the type, not checked
  against a trace).

- **AI-CON-2.5b A cell ties needs and productions (2026-11-16 night, item 5 of the attack line; E-ai-068,
  code reading, and the R6 recording's heap dumps).** A need and a production of one player are tied to
  each other (the need's production, the production's need) through their common cell, from either side:
  - **A need given a cell** (3.3, 3.4) is tied to the first production in that cell's production list
    whose good is the need's good, or belongs to it as its group (a fish, bread or meat production for a
    food need), and that has no need yet, **placed or not**. (3.3 said "an unplaced production": too
    narrow.)
  - **A production given a cell** is tied to the first need in that cell's need list whose good is the
    production's good, or whose good's group is that good, and that has no production yet. Every producer's
    production is given its building's cell when the building is announced (2.5, SIM-START-4.4 b),
    **whatever order placed it**: the cell system's own woodcutter or stone pit, a military producer.
    **Also a site placed by a needs order (4.3)** (stated 2026-11-21 pm, E-ai-079, measured T108): the
    need the order remembered is not the one tied when an earlier-celled need of the good waits in the
    cell; the production goes to the first need in the cell's list, and the order's need stays
    waiting (and may get a producer of its own later).
  - A cell's need list holds its needs in the order they were given that cell. Its production list holds
    its productions in the order they were added.
    **The order is the order of giving, not of making (2026-11-17 night; E-ai-069, the R6 heap dumps
    and measured, high).** A need that loses its cell (its producer torn down, 9.3) leaves its cell's
    list, and when the needs arrangement gives it a cell again it goes to the **end** of that cell's
    list, behind needs made after it, even when it is the same cell. So the first need a new
    production is tied to is the earliest one *given the cell* among the waiting needs of its good,
    which can be a younger need than the oldest waiting one. Test T102a.
  - So a producer that no need ordered still becomes the producer of a waiting need of its good in its
    cell. That decides, for one, which building the route optimizer takes as the need's producer
    (8.3 b).
  - Confidence: high for the two ties and their tests (code); medium for the list order, read from the
    append calls. Whether a needs-construction order's own need (4.3) is tied first, before the cell's
    first matching need, was not read. Test T102.

### AI-CON-3 Needs arrangement (every 13 ticks): giving a need a cell

Two passes, each stopping at the first need that is changed.

- **AI-CON-3.1 Food sub-needs (corrected 2026-10-25 evening, Q-178,
  E-ai-018).** A need of the **food group good** (`food`) is kept in a list of
  its own, apart from all other needs (when a need is made, the food good goes to the
  second list). Pass 1 runs over that list only (in need-id order): a food need
  without a linked concrete need gets one, drawn: fish with probability 0.7,
  bread 0.2, flesh 0.1 (the new need has no consumer), and the pass stops. A food
  need whose linked need is a fish need without a cell that is waiting for
  expansion (no fishing square found) gets it replaced: the fish need is deleted
  and a bread need (draw mod 6 below 5) or flesh need is linked instead. Other
  needs, mines' included, never get a sub-need here, so nothing grows without
  end. Food needs themselves never get a cell (pass 2 does not see their list);
  they come from 3.2 (a missing food input) and from 2.5 (a stone mine). A food
  need takes an object number like any need (SIM-START-4.4). Measured
  (E-trace-010): the iron ore need made at counter 3205, a food need at 3218 (object
  90491, absent from the recorder's need table, which reads the other list), and
  at 3231 one draw and a bread need, then its flour input need (3.2) in the same
  run; an experiment build with the old stand-in (a sub-need for every mine need)
  drew one number too many at 3205. Confidence: high (code and generator).
- **AI-CON-3.2 Inputs first.** Pass 2 over needs without a cell (in need-id
  order): an input good of the need's good (cost list of the good) is
  *missing* when no cell has a producer of it and stock(input) <= 10. A
  producer is a production in a cell whose good is the input or has the input
  as its group good (1.4; so a bread, fish or meat production supplies the
  group good food), counting only productions **with a placed building**: a
  hunter's hut has no production at all (2.5a), and a needs-construction order
  (4.3) adds none before its site is placed. The stock is read for the input's
  own id (there is never a stock of `food`). (Corrected 2026-10-26 night, Q-365,
  E-eco-019: "food is always missing" was read without the group
  count; measured, E-trace-010 player 1: the coal need of counter 3198 waits on
  its food input until the bakery 90650 is placed at 4833 (bread production
  90651), is then arranged, finds no coal square and waits for expansion (3.4),
  and the cell system's run of 4847 makes the coal target of 3.5 (time 484.72 s
  in the heap dumps of 5500 and 6000); with the hunters' productions counted
  the target comes at 3238, with the bakery's order counted at 4365.) If any input is missing, the
  need gets **no cell**: for the first missing input that has no need without a
  consumer, a need is created (no consumer) and the pass stops; if every missing
  input already has such a need, this need is passed over and the pass goes on
  with the next need (corrected 2026-10-25 evening, E-ai-018: "unplaced" is "without a consumer", not "without a cell",
  and a need with missing inputs is not given a cell; measured: player 1's iron
  need of counter 3198 stays without a cell at 3218 while the coal need after it
  creates the food need). Only a need without missing inputs goes on to 3.3 / 3.4.
  Confidence: high.
- **AI-CON-3.3 Plain goods and corn.** Among the cells that accept the need
  (fewer than 20 **needs** in the cell, the needs that have it as their cell,
  or the good is coal, stone, ironOre or gold; and no full mark for the good)
  the cell with the lowest cost is chosen: cost = 50 x (**needs of the good
  already in the cell**) + sum over the inputs of the supplier distance + the
  consumer distance (both counts are of needs, not productions: corrected
  2026-10-28 night, Q-370, E-ai-044 on the candidate
  cell; measured T71), **combined as
  follows** (corrected 2026-10-27 b, E-ai-037): with S = 50 x
  producers + the supplier distances and C = the consumer distance, the cost
  is S + C when both are below 100000, S when only S is, and C otherwise (a
  part of 100000 or more is left out when the other is below it). **All
  distances of this rule are lattice distances between the cells'
  positions** (AI-CON-1.1a: the stored field, the headquarters or the
  building nearest to the centre field, or the validity scan's border field),
  not between their centre fields (corrected 2026-10-27 b: the cell's stored position is used; measured T59). Supplier
  distance of good G from a cell = over the player's cells that hold a
  production of G (or of a member of G's group), the distance from the cell's
  position to that cell's position, **halved (rounded down) when that cell
  holds a free production of G**: one without a need, or whose need has no
  consumer building; the smallest; 100000 if none. Consumer distance of good
  g: a member of the food group is looked up as food (no other group stands
  in for its members); over the goods h that have it in their cost list,
  **except the settler goods** (good type Settler: the professions and soldiers;
  the ware groups count, so tools, made from planks and iron, is a consumer of
  planks; corrected 2026-10-29, Q-371, E-ai-047: the skipped type value
  is 1, which is Settler in `Goods.lua`, WareGroup being 2; measured T77), and
  over the player's cells that hold a production of h: the distance (halved
  when the cell holds a free production of h) x (**the needs of g, or of its
  group, in that consumer cell** + 1); the minimum; 100000 if none. **Which
  needs that factor counts** (corrected 2026-10-30, T85; E-ai-054): the
  needs of the consumer cell whose good is the looked-up good (food for a member
  of the food group) **or whose good's group is that good**: for a fish need the
  cell's food, fish, bread and meat needs, not only its fish and food needs. (Corrected
  2026-10-28 night, E-ai-044: "halved" was read as "only planned", and the factor as the
  producers of g; measured T70, T71). A production is made when its site is
  placed, not when the order is issued (the recordings' count of productions
  rises at the placements: E-trace-014 counters 22429, 22731, 23032; an
  experiment build that leaves unplaced productions out of these distances
  changes no pair). **The cells are taken in the
  player's cell list order (creation order, as in 7.1) and among equal costs the
  first wins** (strictly lower replaces; corrected 2026-10-25 evening, measured:
  with no producers yet all of player 1's cells cost 300000 for the sword need of
  E-trace-010's counter 3011, and the home cell (8, 3), made first, gets it: the
  smithy is placed on (137, 55) in trace tick 3199; taking the cells by position
  row by row gave (8, 2) and a smithy on (137, 45) in 3182). The same list order
  holds for 3.4 and the other passes over the player's cells. The need gets the cell and
  is linked to a free production of its good there if one exists, **placed or not**
(2.5b, corrected 2026-11-16 night; a need placed again after its producer's
tear-down is the case AI-CON-9.3, T84 measured). If no
  cell accepts it and this source's own target slot is empty (whatever the
  other sources hold or the expansion system works on; corrected 2026-10-26
  night, E-eco-019: "no expansion target is pending"), an expansion
  target of kind 3 at the player's home position is set (`AI-military.md`
  AI-MIL-3.1; the slot stays set until that target is finished or cancelled).
  Confidence: medium-high.
- **AI-CON-3.4 Resource goods.** For wood, stone, fish, coal, ironOre, gold
  (needs not waiting): in each accepting cell the best 4 x 4 resource square
  is looked for: for underground goods the square with the largest amount
  whose four corner fields include at least two owned by the player and not
  Border; for surface goods any square with an owned non-Border corner. Cost
  = supplier distance of the first input (food for mines) + consumer
  distance, combined and measured between the cells' positions as in 3.3;
  the cheapest cell wins; the need gets the cell and the square. If
  no cell has a square, the need is put at the **end** of the waiting list and
  flagged "waiting for expansion"; the cell system then drives expansion
  towards the resource (AI-CON-3.5). Putting a need on the waiting list is
  **not a change**: pass 2 goes on with the next need in the same run, so
  several needs can go on the list in one run (corrected 2026-10-26 night,
  E-eco-019: the resource branch ends with "no change" after
  listing the need; measured, E-trace-011 heap dumps: the list is empty at
  4500 and holds [iron ore need 90490, coal need 90484] from 5000 on, so the
  iron ore need went on before the coal need, taken at 4847, came back; a pass
  that stops after listing the coal need puts the iron ore need on after it
  and the cell system takes the coal need again, `AI-military.md` T17).
  **The square test, precisely (E-sim-002; confirmed 2026-10-28, E-ai-041, measured T65):** the
  squares of a cell are its 4 x 4 grid squares (sx, sy)
  whose fields are (4 sx .. 4 sx + 3, 4 sy .. 4 sy + 3), visited sx outer, sy
  inner; a square is only
  looked at when the cell's resource entries for the good sum to more than
  0 and the square's slot is empty. **The slot is taken by the need that is
  given the square, at that arrangement**, whether or not a production exists
  for it yet, and stays taken while that need keeps the square: a second need
  of the same good never shares a square (three wood needs of the Hard game
  get three squares; a second coal need with the only coal square taken goes
  on the waiting list, T65). Its "corner fields" are
  (4 sx, 4 sy), (4 sx + 4, 4 sy), (4 sx, 4 sy + 4) and (4 sx + 4, 4 sy + 4),
  each clamped into the map (so the far corners lie one field outside the
  square), and a corner counts when its owner is the player and its Border
  state bit is clear. An underground entry (kind 1) needs at least two
  counting corners and the largest amount among such squares wins; a
  surface entry (kind 2) takes the first square (sx then sy order) with one
  counting corner.
  **Both kinds in one scan (stone; clarified 2026-10-31, E-ai-057; measured T89):** the squares of a cell are scanned once, and each
  square's entries of the good are looked at in turn. An underground entry
  (two counting corners) only updates the best underground square. A surface
  entry with one counting corner ends the scan at once with that square. So a
  cell offers its first usable surface square if it has one, else its best
  underground square, and the cells then compete on the cost alone, whatever
  the kind: a cell that offers only an underground square wins over a cell
  with a surface square when it is cheaper, or equally cheap and earlier in
  the cell list. The need keeps the square's kind, and the producer follows it:
  stone pit for surface, stone mine for underground. Looking for surface
  squares in every cell first, and for underground squares only when no cell
  has one, is wrong. Confidence: high for the scan and the comparison (code,
  T89); medium for the producer following the kind (the measured order and
  the needs' kinds agree). The flag and the list entry are cleared by the cell
  system when it takes the need (AI-CON-3.5), whether or not it finds a
  square; needs arrangement then treats the need as new again (Q-161b).
  Confidence: medium-high.
- **AI-CON-3.5 Expansion for a waiting need (every 23 ticks).** The first
  entry of the waiting list (the need put on it longest ago, 3.4; not the first
  waiting need in need-id order) is taken (and un-waited, 3.4). Over every 16 x 16 cell
  coordinate of the map (x outer, y inner), every entry of the
  **cell-coordinate resource map** (1.1c: per good, kind and stored landmass
  index, the sum over all the coordinate's fields) for the need's good with
  amount > 2 whose landmass is one of the player's
  *conquered continents* is scored (corrected 2026-10-25 evening, E-ai-016: the resource map keeps one entry per good,
  kind and stored index of the fields holding it, 1.1b (a); the conquered
  continents are the set of the landmasses (1.1b (b)) of all the player's
  cells, cleared and rebuilt only by the 1323-tick part of `AI-overview.md`
  AI-2.3, so the set is empty and no entry is scored before that part's first
  run; medium, code): `(amount + V) /
  (A + 1)`, A being the cell coordinate's **attempt counter**: one counter per
  16 x 16 cell coordinate of the map, 0 at the start, halved (integer) when a
  waiting-need target in that coordinate is finished and raised by 1 when one
  is cancelled (`AI-military.md` AI-MIL-3.2); it does not count productions
  (corrected 2026-10-26 night, E-eco-019: the divisor and the counter the outcome changes are the
  same entry; measured, E-trace-011 heap dumps: player 1's 121 counters (11 x 11
  coordinates) are all 0 at 6500 while its cells hold many productions, some 1
  at 10 000; with a production count the score of a coordinate with productions
  and a negative numerator rises, which made an experiment build send the iron
  ore target of 6847 to (8, 2) instead of (7, 2)), where V is computed for the
  cell coordinate: let C be the player's **nearest existing cell** to the
  coordinate's top-left field, measured from each cell's top-left field
  (16 cx, 16 cy), the first in the player's cell list on equal distance;
  S is a square **of the coordinate itself**:
  among its 16 squares (x outer, y inner) whose entries hold the need's good
  (amount above 0) and that carry no production yet (no need with a linked
  production holds the square: the square's slot is set when a production is
  linked to a need on that square and cleared when the link is cut, 9.3;
  medium), the one whose top-left
  field is nearest (hex distance) to the top-left field of the square holding
  C's position (AI-CON-1.1a), the first on equal distance; `P = max(1, 6 x
  threat balance at S + 30 x that distance)`; if the coordinate is one of the
  player's cells, `P = 2P + 720`; `V = 1200 - P` (V = -1000 when the player
  has no cell or no such square exists). So V falls with the distance from the
  player's nearest cell, and a far coordinate with more of the good loses to a
  near one (corrected 2026-10-26 night, Q-278, E-eco-019: "S the best square in C" made V the same for every coordinate;
  measured, heap dump 5000 of E-trace-010, player 1's coal entries on landmass
  12: (7, 3) 252, (7, 5) 497, (1, 8) 182, (6, 4) 168, (7, 2) 98, (8, 3) 84,
  and the target of 4847 lies in (7, 3)'s block). The highest score wins
  (strictly greater). Then, over the squares (sx, sy) with sx from 4 cx to
  4 cx + 4 and sy from 4 cy to 4 cy + 4 (both bounds included, so one row and
  one column of the next cells too), **x outer, y inner**, the first square
  whose entries for the need's good (all kinds) sum above 0 gets an expansion
  target of kind 1 at its top-left field. The run is made only while the cell
  system's target slot is **empty**: a target made stays with its time until
  the expansion system **finishes or cancels** it (`AI-military.md` AI-MIL-3.1,
  3.2; taking it does not empty the slot) (corrected 2026-10-26 night, Q-365, E-eco-019; measured: player 1's coal
  target (116, 48), made at 484.72 s (counter 4847), taken at 5330 with that time,
  still in the slot at 6500, finished at 6836, the iron ore target (116, 36) at
  684.78 s; AI-MIL T16, T17). Confidence:
  medium-high (Q-161c); the conquered-continents test medium (code; by
  AI-2.2 player 1's set is first filled at game counter 1317, so player 1
  makes no expansion target here before it; not yet checked in a recording).
  Confidence: medium (score terms).

### AI-CON-4 Needs construction (every 17 ticks): what to build next

Skipped while a construction order from this part is pending.

- **AI-CON-4.1 Candidates.** D(g) = number of needs for good g that have a
  cell; A(g) = number of productions of g. A good is considered only if
  production control allows it (AI-CON-10.1: limited goods must be below
  their stock limit) and a worker is available: the producer type's worker
  settler is in stock, or one of the tools in that settler's cost list is in
  stock, or the settler needs no tool. Confidence: high.
- **AI-CON-4.2 Score.** score(g) = (D(g) - A(g)) x aiPriority(g)
  x max(0.001, (100 - max(1, stock(g) + stock2(g))) / 100) / max(0.2, A(g))
  x max(0.1, avgProd(g)), where aiPriority is `gp_setAiPriority` and avgProd
  is the mean productivity of the existing producers of g (1.0 if none): the
  value of `ECO-outdoor-workers.md` ECO-OUT-3.11 as corrected 2026-10-28 (made
  at 0.5 when the producer's site is placed, a sowing farm not raised; T67). The
  good with the highest score (ties: first in good-id order, strict greater
  wins) is chosen. **Good-id order is the id as a signed 32-bit number**
  (ids with the top bit set first; corrected 2026-10-25 evening, measured: corn
  0x17626a03 and water 0xa9676263 tie at 346.5 for player 1 at counter 3208 and
  water wins, the well (136, 53) placed at 3443 before the farm; unsigned order
  ordered the farm first, buildings 3425 -> 3725 when corrected; code E-ai-018: the goods are walked in the key order of a map of the needs with a
  cell). Confidence: high.
  **Which producers avgProd counts (corrected 2026-10-29, T78):** the computer
  player's productions of g that have a building, **sites included**. A site has
  its productivity entry from the moment it is placed, at 0.5 (ECO-OUT-3.11), so
  a new farm still levelling pulls the mean of the farms down at once. (Code: the
  mean walks the player's production records of the good and reads each one's
  building value, with no test for a finished building; E-ai-048.)
  **Only production records (2026-11-17 day, E-ai-075: code reading, the war recording E-trace-019's
  heap dump and an experiment build; high):** the mean runs over exactly the records that A(g) counts,
  each with its own building's value. A building of the player whose type makes g but which has no
  production record does not count; that is the hunter's hut (2.5a): meat with only hunters' huts has
  avgProd 1.0, not their values. Test T107.
  Leaving sites out gives corn 33.6 against water 30.7 at tick 15481 of the
  Hard game, and the farm is chosen there instead of the well. Confidence: high
  that sites count (code and T78). Not measured: a production whose building is
  not placed yet. It exists only while its own order is pending, and this part
  does not score then.
- **AI-CON-4.3 Order.** For the first need of the chosen good that has a cell
  and no production: the producer type is taken from the mine table if the
  need's square holds the good as kind 1, else from the normal table; a
  fields-only good without a square is refused. A construction order {the
  need's cell, type, weight 10} is issued and the need remembered. On "no
  site" the cell gets a full mark for the good and the need loses its cell
  (it is arranged again); on success nothing more. Confidence: high.

### AI-CON-5 Construction orders, budget and site search (where)

- **AI-CON-5.1 Sources.** Nine order slots feed the construction system: cell
  system (unused in this chunk), needs construction (weight 10), military
  building (20), forester (50), catapult, depot (15), harbour (16), ship
  (shipyard), hunter (10). An order = cell, building type, weight, time (the
  game-seconds clock at creation, ECO-TIME). A slot holds one order; it keeps
  it while it is pending or being executed (so no second hunter order is made
  while the first is unfinished) and is cleared by the order's failure or
  success handler. Confidence: high.
- **AI-CON-5.2 Budget.** Two counters, limit L and count C, both 0 at the
  start and reset to 0 every 600 ticks (phase AI-2.2; the reset comes before
  the 50-tick step of the same tick). Every 50 ticks while no order is being
  executed: if C < L then C = C + 1, else the pending order with the
  largest (now - order time) x weight is taken, now and order time in game
  seconds; the value must be **greater than 0**, so an order made in this
  very tick (age 0) is not taken, and ties keep the earlier source in the
  order needs construction, cell system, military, forester, catapult,
  depot, harbour, ship, hunter. Taking an order starts its execution (state
  1 of AI-CON-5.3a); nothing else happens in that tick. When a building is
  placed, L = 2 L + 2; **C is not reset by a placement**, only by the
  600-tick reset. So after the first placement of a 600-tick window (L 0 ->
  2) two idle 50-tick steps raise C to 2 and the third takes an order; after
  the second (L 6) four more steps; a 50-tick step with no order of age > 0
  changes nothing. Confidence: high (code; the ten sites of E-trace-002 fit
  it, T21).
- **AI-CON-5.3 Execution.** (1) **Twice** the type's construction cost (frame
  and main parts summed per good) must be in stock: every cost good's
  stock(g) (AI-CON-1.7) >= 2 x its amount (the check's list is the frame part
  plus the frame part again plus the main part twice; corrected 2026-10-27
  night, E-ai-024; measured, `AI-military.md` AI-MIL
  T18: a guard house (planks 2, stones 3) fails with 3 planks, a hunter's hut
  or barracks (planks 1 + 1) with 2 or 3), else the order fails with "no
  material"; the order's cell must exist, else
  "no site". (2) Site search over the cell (AI-CON-5.4), one step per tick,
  up to 10 fields per step. (3) The best field is placed with the player's
  normal placement command if the field's class still fits the size
  (`ECO-building-placement.md` ECO-PLACE-4: small needs class 2..4, medium
  3..4, large 4, mine 5, harbour 6) and, for a military type, the military
  clause of AI-CON-5.4a holds; else the order restarts at (1) with a new
  search. (Corrected 2026-10-25: the clause is the same as in the search, not
  "no finished military building of the player".) Each source has its own failure and
  success handlers (needs: AI-CON-4.3; military and ship: later files).
  Confidence: high.
- **AI-CON-5.3a Timeline.** The execution advances one state per tick, in
  the construction system's place in the think (AI-2.3, every tick), **after
  the system's inner parts** (`AI-overview.md` AI-2.4a: the street parts,
  forester, ship and hunter runs of that tick come first): the
  order is taken in tick P (50-tick step); in P + 1 the stock and cell checks
  run and the search is set up, no field tested yet; in P + 2 .. P + 1 + n
  the search takes one step per tick (n steps, AI-CON-5.4); in P + 2 + n the
  step finds the search finished and takes the result (state 3, or failure
  when no field qualified); in **P + 3 + n** the field is checked and the
  site is placed, inside that tick (AI-2.1a), and L doubles. A failure in
  (1) ends the order in P + 1, one with no field in P + 2 + n; the system is
  idle again from the next tick. Confidence: high (code); the n of a given
  search is high only by measurement (T21).
- **AI-CON-5.4 Site search (cell mode).** The fields of the 16 x 16 cell are
  scanned from the cell's origin (16 cx, 16 cy) row by row, x fastest. A
  field is a candidate if: it is in the map, fits the size (class rule
  above), a flag can be set on one of its six neighbours, its site value
  (AI-CON-6) is above the best **score** so far, and a route of length len <
  16 exists from the cell's **position** (AI-CON-1.1a, not the cell's centre)
  to it (len = 0 for the position itself; a free-walk path, no road, counted
  in steps as AI-CON-5.4b says); its
  score is value x (16 - len) / 16
  and the best score wins (strictly greater). **Step budget:** a step tests
  at most 10 fields and **ends early after the first field that passes the
  in-map, size and flag tests** (the costly value and route then run for
  it), whether or not it becomes the best. So a cell of fields that all fail
  cheaply takes 26 steps (256 / 10), and every field that passes the three
  cheap tests costs a step of its own with the cheap fields before it; the
  first hunter's cell in E-trace-002 took 80 steps. Point mode (military
  orders): the given field first, then a spiral outwards to radius R, with
  2R in place of 16 and the same step budget. Confidence: high (code),
  medium for the route being a free-walk path.
- **AI-CON-5.4a The cheap tests in detail (2026-10-25, E-ai-013).**
  - *Size* is the class test of 5.3 (3), including, for a type with a
    territory range (a military type), the **military clause**: no building or
    construction site of any player whose type has a territory range (the
    headquarters included) stands on a field within distance 4 of the
    candidate (rings 0 to 4; each building counted on its own field, as in the
    buildings layer of ECO-FOOD-5.10). A field failing it is a cheap failure, so
    it does not end the step. Measured (T34): without the clause in the search,
    the guard-house order of trace tick 1045 takes 68 steps instead of the
    recorded 53, the fields next to the castle each costing a step of their own
    (their site value is 0 by AI-CON-6.3 anyway). Confidence: high for the
    headquarters (measured), medium for sites and other players' buildings
    (code).
  - *Flag:* not the six neighbours, but five fields reached by stepping one
    copy of p through the directions 0, 1, 2, 3, 4 in turn (p + d0, then + d1,
    + d2, + d3, + d4; the same walk without reset as ECO-HQ-4.8's ring 3): the
    east neighbour, three fields at distance 2 and the direction-2 neighbour; the
    building's own flag field (direction 1) is not among them. The test passes when
    one of them is in the map, owned by the player, and carries none of Border,
    Water, ElevationBlocked, DoodadBlocked, Blocked, DepositBlocked and
    MovementBlocked (an existing flag does not make it fail). Confidence: medium
    (code; no recorded placement differs by it so far).
- **AI-CON-5.4b The route length (Q-281, 2026-10-27 b, E-ai-033).** For a
  field p that passed the cheap tests and whose site value v is above the best
  score so far (otherwise no route is computed), len is found when that field
  is tested, every time (not once per search):
  - p = the start (the cell's position, or the point in point mode): len = 0.
  - p a neighbour of the start (lattice distance 1): len = 1, no search, even
    when p or the start is not walkable.
  - Otherwise the free walk's search of the direct variant of AI-CON-8.3 c
    runs once (its first try only: flood from p towards the start, the
    walker's step costs, a field passable only if walkable or the start itself;
    no second try with every field passable, no draws). No path found: p is
    not a candidate (len counts as 1 000 000).
  - A found path's len is a **step count**, not the cost label: from the start,
    step to a neighbour until p is reached, at every field choosing among the
    neighbours whose label is the lowest of those below the field's own label,
    and among those the **lowest direction number** (0 to 5). There is no
    kept direction: unlike the read-out of AI-CON-8.3 c (and TRN-FREE-3.1 /
    3.2a), the previous direction does not count when it is still allowed.
    (Only if a field had no lower neighbour, which a found path does not have,
    would the previous step's direction be taken again.)
  - The candidate counts only if len < 2R (16 in cell mode); its score is
    ((2R - len) / 2R) x v in single precision, and it becomes the best only if
    strictly above the best score.
  Confidence: high (code). The guard house of T56 is decided by len; the
  difference from the kept-direction read-out changes no placement of the
  recorded games so far (all 349 site routes of the Weak 20 000-tick game and
  507 of 508 of the Hard one count the same both ways in an experiment build),
  so the direction rule rests on the code: T56 (a) is measured, T56 (b) is a
  case taken from s2-reborn's own game for the step count.

### AI-CON-6 Site value of a building type T at field p

Multiplicative factors, starting at 1; v = 0 means unusable.

- **AI-CON-6.1 Pavement rule (Q-075).** Not a harbour type: the rings 0..1
  around p (0..2 for a large type) are walked outward, p itself first; if a
  field with the ForHarbor state bit (`ECO-building-placement.md`
  ECO-PLACE-1: harbour pavement) is found, the first such field must have an
  own **harbour** within 3 fields (4 for a large type), else v = 0. No pavement
  field within the rings: no constraint.
  **The building must be a harbour (corrected 2026-11-17 night, E-ai-070, code reading and
  measured; was "an own building", the list being unidentified).** The test looks through the
  game's list of harbours, which holds every building whose type is a harbour (in any state: a
  harbour enters the list when it is placed, so a harbour's building site counts). It takes the
  player's own harbours from it. Any other building near the pavement does not count: a guard
  house, a woodcutter or the headquarters. So the computer player keeps harbour pavement and the
  fields next to it free (v = 0) until it has a harbour there. Measured (T103): with every own building
  counted, R6's player 3 puts the woodcutter of counter 9772 on (19, 30) instead of the recorded
  (28, 24), and the fight game's generator parts at 9801. With harbours only, the site is the
  recorded one, and the generator stays equal to 9909; the Storm Coast pairs are unchanged (Weak
  51724, Hard 49218). Confidence: high.
- **AI-CON-6.2 Base value.** Type without product, shipyard or hunter: if T
  has a planting good (forester): v = the supplier value of that good (for
  the forester's wood: the woodcutter). Type with product g: deposits good and
  not a mine: v = number of deposits of g (trees, stones) within the working
  radius, **counted over a flood from p** (corrected 2026-10-29, E-ai-050; measured T81): starting on p, the flood steps to a
  neighbour only if that field is within the working radius of p and either holds a
  deposit or is walkable. A deposit that no such chain of fields reaches does not
  count, even within the radius (for example one beyond water). A deposit being
  removed does not count (the code tests a field of the deposit, read as its removal,
  medium). A tree reserved by a woodcutter still counts: leaving reserved trees out
  differs from the Weak game at buildings 17685. **A sapling counts as well** (corrected 2026-10-31
  night, T90; E-ai-058): the count tests only the deposit's good and its removal mark, never its
  growth, so a tree counts from the moment the forester creates it on the chosen spot (`ECO-outdoor-workers.md`
  ECO-OUT-4.5), before it grows; "saplings are not tested" was read as "left out". Fields good: v = **the sum of the resource amounts of g on all
  fields within the working radius**, explored or not (corrected 2026-10-28
  night, E-ai-044; was read as the number of fields;
  measured T72); otherwise, if g has inputs, v = sum over the inputs of g of their
  supplier values; a product without inputs (well) keeps v = 1 (Q-161d).
  **Supplier value** of a good g seen from p: if a building type produces g, d
  = the distance to the nearest own building of that type **in any state,
  building sites included**; v = max(1, 15 - d), 0.1 if there is none within
  1000 (no warehouse then). Only if no building type produces g: d = the
  distance to the nearest own finished warehouse on p's landmass (1.1b), v =
  max(1, 15 - d), 0.1 if none. (Corrected 2026-10-25 evening: the planting
  good's producer was "finished, else the nearest warehouse"; E-ai-019:
  the nearest-building search with its finished-only argument off, the warehouse search in mode 1.
  The sites of the inputs' producers are measured in T39; the forester's case
  is read from the code only, no forester was placed in E-trace-010 by trace
  tick 4792.) Confidence: high (structure, inputs), medium (the forester's
  case, the exact counting of deposits).
- **AI-CON-6.3 Special types.** Shipyard: v = max(1, 100 / d), d = distance
  to the nearest own harbour. Hunter: 1 if game animals are within the
  working radius, else 0 (the animals are the hunter's own search's,
  `ECO-food-mining.md` ECO-FOOD-5.2: an animal some hut has reserved does not
  count; corrected 2026-10-25 evening, T41, E-ai-021; the player must
  have explored the field the animal's walk ends on, not its own field,
  ECO-FOOD T50, 2026-10-28). Farm: v += the number of fields on the **ring at
  lattice distance exactly 2** around p (its 12 fields, walked as in 6.4; those
  outside the map skipped; p and its six neighbours are not looked at) that pass
  the grain planting test of `ECO-outdoor-workers.md` ECO-OUT-4.3: no flag on
  it, none of the bits Blocked, Street, Water, Coast, ElevationBlocked,
  DepositBlocked, MovementBlocked, DoodadBlocked set (so a tree or stone on the
  field fails it), terrain neither for mining nor harbour ground, terrain for
  planting; the radius and the Blocked-neighbour conditions of 4.3 are not
  applied. At most 12 (corrected 2026-10-25 evening, T38; the earlier "plantable
  fields within radius 2", up to 19, put the farm of trace tick 3725 on the
  wrong field; E-ai-019). Confidence: high. Military type (territory range > 0): v = 0 if an own military building
  (the game's list of soldier-holding buildings, the headquarters included,
  Q-161e) is within distance < 5, else (fields within the territory range on the same
  landmass not owned by the player) + min(100, distance to the nearest own
  military building); v = 0 when no field counts. **Landmass** here is the
  map's stored continent index of the field (`FMT-s2m-map.md` FMT-S2M-4.8), used
  as loaded: a field counts when its index equals the site's (1.1b). Land the
  map marks -1 (mountains and other blocked ground, doodad-blocked water)
  never counts for a site on a landmass (a site on a -1 field would count the
  other -1 fields; not seen). Nothing else is tested: blocked fields, deposits
  and stones with the site's index count (E-ai-016; corrected
  2026-10-25 evening, the values of T34); buildings, flags and trees placed later do not
  change the indices (measured 2026-10-25, T34: with every non-water field as
  one landmass the guard house of trace tick 1101 goes to (135, 48), with the
  map's indices to the recorded (138, 57); E-ai-013). The fields are walked in
  ring order (MAP-GRID-3) out to the range, skipping those outside the map. Harbour: 0 if an own harbour is within < 5 fields,
  else 1. Depot: 0 if any own warehouse is within < 11 fields, else 1.
  **The warehouses in any state (corrected 2026-11-17 night, T105, E-ai-072; code and measured).**
  The depot test looks through the game's list of warehouses: every building whose type is a
  depot (headquarters, storehouse, harbour). A building enters that list when it is placed, as
  the harbour list of 6.1 does. So **a warehouse's or harbour's building site counts** as soon as
  it is placed; there is no test of the building's state. (The supplier value of 6.2, by
  contrast, wants a finished warehouse.) Confidence: high (all of 6.3).
- **AI-CON-6.4 Penalties.** Not a mine or harbour: an own farm **in any
  state (a farm's building site counts)** within distance 4 (3 for a farm)
  makes v = 0 for non-military types and halves v for military types
  (corrected 2026-10-25 evening, T39: "a finished farm" put the brewery of
  trace tick 3987 next to the farm's site; E-ai-019). Not military, mine or harbour: a Border field on the
  ring at lattice distance exactly 2 around p makes v x 0.4 (the 12 fields of
  that ring, walked from p's direction-4 field two steps out, two steps in each
  direction 0 to 5; p, its six neighbours and fields farther than 2 are not
  looked at; fields outside the map are skipped; Q-333, E-ai-009; measured: T27). Field class 3 for a small type, or class 4 for a
  small or medium type: v x 0.5 (large spots are kept for large buildings).
  Confidence: high.

### AI-CON-7 Cell-driven orders

- **AI-CON-7.1 Forester (every 3 ticks, one cell per run).** The run does
  nothing while its own order is pending. It keeps an index into the player's
  cell list (creation order): the cell at the index is handled and the index
  advanced after it (back to 0 when it is at or past the end before the run).
  limit = 10 if stock(plank) + stock(wood) >= 40, else 40; trees = surface wood
  in the cell on its landmass (7.3; the stored index at the cell's current
  position, 1.1b), **as the cell-coordinate resource map holds it** (1.1c: it
  lags the trees by up to a full round of its refresh, T52). If the cell has a forester that is
  finished: trees > limit -> its production is switched off (if on); trees <=
  limit -> on (if off). If the cell has no forester, trees < limit and the cell
  has a plank producer (sawmill): order forester, weight 50. **"Has a plank
  producer"** (clarified 2026-10-28, E-ai-041;
  measured T66): the cell's production list holds a production of plank (or of
  a good whose group good is plank). A needs order (AI-CON-4) makes no
  production: the order remembers only the need, and the production record
  comes with the placed site (SIM-START-4.4 b). So a sawmill that is only
  ordered, not yet placed, does not count. Confidence: high.
- **AI-CON-7.2 Hunter (every 3 ticks, one cell per run).** The same own-order
  test and the same kind of list index as 7.1 (its own index). A cell without a
  hunter whose animal count is above 0 gets a hunter order, weight 10. **The
  count is the respawn record's per-cell count** (`ECO-food-mining.md`
  ECO-FOOD-5.15): the living animals whose **home** field lies in the cell, where
  they stand and whether the player has explored anything there do not matter
  (corrected 2026-10-25 evening, Q-363, T42: was read as the animals seen in the
  cell; E-ai-022). Confidence: high. **Own order pending**
  (Q-376, E-ai-052): the run looks at its slot's
  building type; the slot keeps the order from its filing until the end handler
  of AI-CON-5.1 clears it, and that handler runs after the hunter run of its tick
  (`AI-overview.md` AI-2.4a), so after an order ends in tick k the next cell is
  looked at in the first run after k (T83). The index moves on by one at every
  run that finds the slot free, whether the cell gets an order or not.
- **AI-CON-7.3 Stone pit and woodcutter (every 53 ticks, one grid position per
  run).** The run does nothing while its own order is pending. Unlike 7.1 and
  7.2 it walks the **cell grid**, not the cell list: a cursor over the
  ceil(width / 16) x ceil(height / 16) grid positions, x fastest, wrapping in
  both directions, starting at (0, 0) and **advanced before** each run, so the
  first run looks at (1, 0) and (0, 0) is only looked at after a full round.
  The run tests the cell at that position if the player has one; no cell there
  -> nothing this run. Counts: producers of a good = the cell's productions
  whose good is that good or has it as its group good (`gp_setGroupGoodId`:
  bread, fish and meat count for food; corrected 2026-10-26 night, E-eco-019: the field read is the good's group, not the building type's
  product); stones (trees) =
  the sum of the surface records (kind 2) of stone (wood) in the shared
  low-resolution resource square of the cell (AI-CON-1.1) on the landmass of
  the cell's position (the stored index at its current position, not the
  cell's landmass of 1.1b (b); E-ai-016). If the cell has fewer than 2 stone producers and 5 x
  (stone producers) < stones: order stone pit, weight 10; else if fewer than 2
  wood producers and 5 x (wood producers) + 5 < trees: order woodcutter, weight
  10. Confidence: high (E-sim-002; Q-238). The validity scan of AI-CON-1.1
  uses a cursor of the same kind, with its own position.
- **AI-CON-7.4 Depot (every 1423 ticks).** Among cells with the depot-wanted
  marker and no clearing marker, the oldest marker gets a depot order (weight
  15) and its time is reset. Confidence: high.
- **AI-CON-7.5 Harbour (every 997 ticks).** If no order is pending, the cell
  with the oldest harbour-wanted marker (1.6: a harbour spot of the cell's
  landmass in the cell, no harbour in it) gets a harbour order, weight
  16. The "harbours enabled" switch is set when the computer player is
  created and nothing clears it during a game (it is only saved and loaded),
  so harbours are always enabled (Q-169). Confidence: high.
- **AI-CON-7.6 Crowded cells (every 19 ticks).** If the crowded source's own
  target slot is empty (other sources' targets and the target in progress do
  not matter; the slot stays set until its target is finished or cancelled,
  `AI-military.md` AI-MIL-3.1, 3.2), the cell with the largest (now - marker
  time) x weight among crowded cells (game seconds; strictly greater, above 0,
  so of equal values the first in the player's cell list, creation order;
  E-ai-061, the product in single precision)
  sets an expansion target of kind 2 at its position and its time is reset.
  (Corrected 2026-10-26 night, E-eco-019: "if no expansion target is
  pending"; measured, E-trace-011 heap dump 7500: player 1's expansion system
  holds the crowded target (144, 32) of 533.3 s (counter 5333, made while the
  coal target of 3.5 was in progress) as its last current target, already finished or
  cancelled, then takes the
  iron ore target of 684.78 s; an experiment build with the old condition never
  made the 5333 target and took the iron ore target first.) Confidence: high.

### AI-CON-8 Roads

- **AI-CON-8.1 Street construction (every 7 ticks).** The player's flags are
  visited in list order, one flag per run (the cursor restarts at the end). A
  flag without a building is skipped (next flag). For a building flag F the
  nearest own flag T is searched (AI-CON-8.4) with F's own road network as
  the excluded network and the flags given up for F as excluded positions.
  No T: next flag. If F has roads and T has none: T is given up (8.1c). Else
  if T is new: T is remembered and the run ends. Else (same T as last run):
  the attempt counter rises; attempts 1 to 5 each measure one strategy with
  the walk pathfinder (a "walk path" a -> b, AI-CON-8.1e: it counts when it
  exists and its length in steps is at most 2 x distance(a, b)), attempt 6
  executes the best one:
  a. Strategies. Best length L and secondary length L2 start at 1000.
     1: walk path F -> T; length < L: L = it, best = 1.
     2: for every inner waypoint w of every road at T where a flag can be
        set: walk path F -> w; length < L: L = it, best = 2, remember w.
     3: a *road-stop search*: the walk path F -> T is searched again (road
        fields block, as always), but the first field reached that carries a
        **road R attached to T** (one of R's ends is T) is noted and may be
        walked through; the search counts when a path exists and its field
        count - 1 <= 2 x distance(F, T). P = the other end of R. **Its length
        is the path's field count, F and T both included** (steps + 1; corrected
        2026-10-24 night, E-ai-010: the first difference at trace tick 646), while every walk path's
        length is in steps. If that length <= L and the walk path T -> P (with
        R's fields walkable, 8.1e) has length < L2: L = the field count, L2 =
        the second length, best = 3. So 3 replaces 1 or 2 only with a path
        **one step shorter** or more; at equal steps the walk path stays best
        (T30).
     4: as 3 with the second leg F -> P: best = 4.
     5: as 3; for every field w of the found F -> T path where a flag can be
        set: walk path P -> w with length < L2: L = the field count of the
        road-stop path, L2 = it, best = 5, remember w. **In practice strategy 5
        never counts**: every w is a field of the road-stop path, an excluded
        position of the second legs (8.1e, T74), and a walk path to an
        excluded field finds nothing (added 2026-10-28 night, E-ai-045: the walks P -> w on the same search object as the road-stop search;
        measured T75: no `ai_street` row with best 5 in E-trace-011 .. 014).
  b. Execution (attempt 6) builds with the road planner, which accepts a
     road when its field count - 1 <= 2 x the straight distance of its ends:
     1: road F -> T. 2: road F -> w. 3: the road R (attached to T) is
     removed, then roads F -> T and T -> P. 4: R removed, roads F -> T and F
     -> P (F sits between T and P). 5: R removed, road F -> T and a road
     from P to w (w on the new road gets a flag). In 3 to 5 the search of
     attempt 3 is repeated first and nothing is done if it fails. Every
     variant rebuilds a connection for P, so the network is only torn when
     the second road is refused by the planner. The attempt's result is that
     of the last road built; success moves on to the next flag.
  c. Giving up T: T's position joins F's excluded list; F's give-up counter
     rises; above 5 the next flag is taken, else the attempt counter, best
     strategy and remembered T are reset. The excluded list starts with F's
     own position, put in when F is taken (and again if F is handled with the
     list empty), so it holds give-ups + 1 entries (measured, E-trace-010
     `ai_street`: every one of 1325 rows with a flag; the run of 1485 in T35
     shows give-ups 1, excluded 2). F lies in the excluded network anyway, so
     the extra entry changes no search (E-ai-017, 2026-10-25
     evening). A failed attempt 6 leaves the
     attempt counter at 6, so T is given up at the next run.
  Confidence: high (search, attempts 1 and 2, acceptance rule, give-up,
  and, after Q-168: that R is attached to T, P is its other end, and the
  last road of 5 starts at P; the lengths' units, measured: T30, T31);
  medium (the road-stop search's exact path through the noted field).
- **AI-CON-8.1f The cursor is a list index (2026-10-27 b, E-ai-038;
  measured T60).** Each run takes the flag at the cursor's position in the
  flag list (after the restart of 8.1d when the cursor lies at or past the
  end). The flag being worked on (its T, attempts and give-ups) is
  identified by its object number: the work goes on only while that flag is
  still the one at the cursor's position. When a flag earlier in the list is
  removed, the list closes up, so the cursor names the next flag, or lies past
  the end and the run restarts with the first flag: the work on the old flag
  is dropped and the flag now at the cursor is taken as a new F (the old flag
  is reached again only on the list's next round). **And the cursor moves on
  at once** (corrected 2026-11-01, Q-380, T97): when the flag at the cursor is
  not the one worked on while that work is still held (its excluded list is
  not empty, which it never is once a flag was taken, or it has give-ups),
  the cursor moves one place on in the same run, before the flag at the old
  place is handled; that flag is handled in this run as a new F (no T: next
  flag; a new T: remembered, the run ends), and the next run looks at the
  place after it, so its work too ends after one run (a lone run that
  remembers a T and is never followed by attempts). When the cursor lies at
  or past the end it restarts first (8.1d) and the work is cleared, so the
  first flag is handled normally. A flag at the cursor with a lower object
  number than the one worked on makes the run do nothing, the work kept (code,
  not observed). Confidence: high (E-ai-062; measured T60,
  T97).
  **"Next flag" steps the cursor on from where it is (2026-11-17 day, E-ai-074: the Weak recording
  E-trace-018's street table and an experiment build; high):** whenever a run passes on to the next
  flag (no T found, a flag without a building, the give-ups used up, a road built), the cursor rises
  by one from its current value; it is not set to the handled flag's place plus one. The two agree
  in a normal run. After the move of the T97 case they do not: the cursor already stands one place
  past the flag handled as a new F, so a run in which that F finds no T leaves the cursor **two**
  places past the old place, and the flag right after F is not looked at in that round. Test T106.
- **AI-CON-8.1e The walk path (2026-10-24 night, E-ai-010; measured, T30,
  T31).** A walk path a -> b of 8.1 and 8.3 is not the settlers' free walk:
  it is the computer player's labelled search of `AI-overview.md` AI-2.7 b
  (the road-stop search) **with the hit note switched off**:
  - the search floods from b and stops the moment a gets a label; the bound
    is 4 x distance(a, b) measured from b; the excluded positions (F's
    given-up flags in 8.1, none in 8.3), the player's fields and walkability
    apply as in AI-2.7 b; the goal a is exempt from the excluded positions
    and the flag and road tests (the bound and the usability test of the next
    item apply to it too; read before 2026-10-31 as exempt from walkability,
    which makes no difference for a flag or road field);
  - **only fields usable by the player are entered** (corrected 2026-10-31
    night, T93; E-ai-059): inside the map, owned
    by the player, **not a Border field**, not Water, not ElevationBlocked, not
    DoodadBlocked (the inner land of `TER-territory.md` TER-6.1), and not Blocked,
    DepositBlocked or MovementBlocked. The test comes first, for every field and
    the goal a too (a flag or road field always passes it). So no walk path and no
    road-stop path runs along the player's Border ring (the outermost fields of its
    land, where no flag or road can stand either, TER-6.4), and in the road-stop
    search, as in the walk path, a field with a tree or stone pile is closed: the
    free walk's higher cost for such a field never applies to the computer player's
    searches (corrected: AI-2.7 b had it). Confidence: high (code; measured T93);
  - a field with a flag blocks unless it is a; a road field blocks unless it
    is a or lies on the road R noted by the same attempt's road-stop search:
    in strategies 3 to 5 the second leg (T -> P, F -> P, P -> w) may walk over
    R's fields; in 1, 2 and 8.3 no road field but a is enterable;
  - **in strategies 3 to 5 the fields of the road-stop path found by the same
    attempt are excluded positions of the second leg** (a exempt as always):
    it goes around them (added 2026-10-28 night, measured T74; the code not
    read: medium-high);
  - **a field with a tree or a stone pile blocks** unless it is a: a field
    with the DepositBlocked state (`ECO-building-placement.md`), as a flag
    does; the search does not walk through it at a higher cost (added
    2026-10-28, measured T68, T64; E-ai-039: a field holding an
    object of the field-object layer 0 is refused before its step cost is
    worked out);
  - **No settler blocks a walk path (corrected 2026-11-17 night, T104, E-ai-071;
    measured).** A field is not refused because a settler stands on it, not even
    R's own carrier waiting on R's idle field. The settler test written below
    (2026-10-27 to 2026-10-31) is **withdrawn**. None of its cases needs it:
    - T61's blocked field was a field of the same attempt's road-stop path, which
      the second leg may not enter anyway (the exclusion added later, T74);
    - T64's field also held the tree, and a tree field is closed;
    - T79's hunter was walking;
    - in T80 and T95, R's carrier stood on R's idle field and the second leg went
      over it, read then as a donkey's field and as a cleared mark;
    - in T104 R's carrier had stood on its idle field for over 200 ticks, nobody had
      crossed it, and the second leg still went over it.

    With no settler test, every recorded pair still reproduces those cases. The
    fields closed to a walk path are those of the usability test, the excluded
    positions, flags, roads but R, and trees and stone piles. Confidence:
    medium-high (measured on all pairs; the code's field-object test was read for
    flags, trees and stones, not for settlers).
    *Withdrawn text, kept for the cases' history:* a field on which a settler stands blocks unless it is a (not one in
    the middle of a step: a carrier waiting on his idle field, a worker in a
    timed action on his work spot; a settler walking through does not block; nor does a
    worker on his walk back from a work spot off the field points to his own field,
    `TRN-free-walk.md` TRN-FREE-5.6: he is in a step until he stands on that field;
    added 2026-10-29, measured T79; nor does a field on which a road's **donkey**
    stands idle, even with the road's carrier standing there too: measured T80, where
    the donkey stopped there about 100 ticks after the carrier; whether the order of
    the two matters and how the original marks the field is not read, medium),
    on R's fields (added 2026-10-27 b, Q-368, T61: R's carrier on its idle
    field) and, taken as the same layer-0 test, on any other field alike
    (2026-10-28, T64: a woodcutter felling a tree beside the second leg's way;
    that field also holds the tree, so T64 alone does not separate the two:
    medium for fields off R); only settlers out in the world count (not one
    that has gone into a building). **What counts as standing there is the
    field's settler mark** (added 2026-10-31 night, Q-379, T95): a settler that
    comes to stand on a field marks it, and any settler leaving a field clears its
    mark, whoever set it; so a settler standing on a field that another settler
    has since walked over no longer blocks, until it moves itself (consistent with
    T79 and T80: a donkey stopping on its carrier's field takes the mark, and an
    idle donkey does not block). Confidence: medium (one deciding case, T95; the
    cases T61, T64, T79, T80 unchanged by it; the mark itself not read in the
    code). The road-stop
    search itself (AI-2.7 b, the hit note on) is not blocked by settlers: T61's
    search goes through the field R's carrier stands on (E-ai-039:
    the road test reads the field's road reference; medium for the mechanism,
    measured T61, T64);
  - its length is the number of steps from a down the labels to b, each step
    to the neighbour with the lowest label below the current one (the lowest
    direction number among equal labels); a and b next to each other: 1
    without a search; no draws;
  - the pairs: strategy 1 F -> T, 2 F -> w, the second legs T -> P (3), F -> P
    (4), P -> w (5), and 8.3's route variant Fn -> F0, a being the first named.
  Confidence: high (the code, and every street state of E-trace-006 at
  counters 344 .. 386 and 603 .. 645 reproduced with it in an experiment
  build; with the settlers' free walk the sawmill flag's attempt 1 at 351
  finds a 2-step path through a road field and the run differs).
- **AI-CON-8.1d A new flag is seen one tick later (E-trace-002; E-sim-002).** The flag list the street
  construction walks is the player's flag list as it stands when the run
  starts; a flag created in tick k (by the computer player's own placement
  in the same update or by anyone else) is not in it before tick k + 1. The
  cursor's restart at the list's end happens at the start of a run, and that
  run handles the first flag (no run is spent on the restart). With one flag
  (the castle's) every run finds no T and ends; with two connected flags the
  runs alternate castle, hunter, castle, .., each ending with no T. Measured
  (T24): the hunter's flag (141, 48) is created in tick 127, a run tick, and
  the run of 127 still handles the castle flag alone (no T); the run of 134
  takes the hunter's flag as F with the castle's flag as T, attempts 1 to 6
  follow at 141 .. 176 and the road built at 176 runs from the hunter's flag
  to the castle's (its first flag is F: "141,48 140,48 139,49 140,50",
  TRN-CARRY-1.3 T20). The sawmill's flag (139, 48) is created in tick 323,
  again a run tick: the run of 323 handles the castle flag (no T), 330 the
  hunter's flag with T = the sawmill's flag (the hunter's flag has a road, T
  none: given up), 337 the hunter's flag again (no T), 344 the sawmill's
  flag (T remembered), 351 and 358 attempts 1 and 2 (no draws), 365, 372 and
  379 attempts 3 to 5 (the road-stop search draws 2, 3 and 3 times more than
  the animals alone, AI-2.7 b), 386 attempt 6 builds strategy 4: the road
  (141, 48) - (140, 50) is removed and "139,48 139,49 140,50" and "139,48
  140,48 141,48" are built. Seeing the flag in its creation tick instead
  makes the castle's flag F at 127 (the road then starts at the castle's
  flag) and puts every later run one flag behind, so the sawmill's attempts
  start two runs late (E-trace-002 trace ticks 278 and 366). Confidence:
  high (two creation ticks measured against the runs; the code's cursor and
  restart).
- **AI-CON-8.2 Street optimizer (every 67 ticks; profile switch).** One road
  per run (list order): a flag is set on the first waypoint, from the road's
  second field on, where a flag fits and no object stands. Confidence: high.
  Made exact 2026-10-25 night (Q-275, E-sim-011): a run does
  nothing at all unless the player's profile enables street optimizing
  (`ai_enableStreetOptimizing`: off for Weak, on for Normal and Strong, AI-1.2). Else
  its cursor (0 at the start) indexes the player's list of roads; a cursor at or past
  the list's length becomes 0; that road is handled (its stored fields in order, the
  first where a flag can be placed gets the flag command) and the cursor rises by 1
  after every run, with or without a flag. **The cursor is only compared with the
  list's length when it is read**: it is not wrapped after the run, so when roads were
  added since, it goes on to them instead of starting again at 0 (measured T58). The
  list is the player's roads in creation order (a removed road taken out, new roads
  appended; so the order of the roads' object numbers, never a storage order in which
  a new road takes the place of a removed one, T62). Measured: the recorded game's Weak computer player never runs it (cursor
  0 at counters 523 and 1023, no flag in 6000 ticks that is not a building's); the Hard
  one (E-trace-012) runs at counters 195 + 67 k (T58). Confidence: high.
- **AI-CON-8.3 Route optimizer (every 37 ticks; profile switch).** State: a
  building cursor, a partner index k, a route-flag index i, a try counter t,
  a waypoint index w and a list X of tried flag positions.
  a. Cursor. The building record with the smallest id >= cursor is taken;
     none: cursor restarts and the state is cleared. A planting building
     (AI-CON-1.2: no product, a planting good; a farm is not one, T46) is
     skipped (cursor moves past it, state cleared). A record with an id above
     the cursor only moves the cursor to it (handled next run). The current
     building B stays current until it is "done". At the game's start the
     cursor holds -1 (measured, E-trace-007 heap dump in counter 26) and
     counts as below every id: the first run (counter 31) moves it to the
     smallest id (T33; an unsigned -1 would make that run a restart and does
     not give the measured cursors of T33).
     **The state goes with the task, not with the building (2026-11-15 day; E-ai-066, the R6
     recording and an experiment build).** The state (k, i, t, w, X) is cleared only when the
     current building is done (finishing it, or b's "done"), when the cursor restarts, and when a
     planting building is skipped. If the current building's record goes while it is current (the
     building torn down, burnt or taken), the next run finds the next record with a higher id and
     only moves the cursor to it, as above, and the run after that takes it up as B **with the
     state the removed building left**: k, the route-flag index, the tries, the waypoint index and
     the tried positions X are not reset. So the new B's first run can finish a shortcut begun for
     the old one. In the direct variant this asks a road from the new Fn to the flag at X's last
     entry. Measured in R6 (MP_4P_Fight, Hard players):
     - Player 2's route task works on its hunter's hut (counter 6600: k 0, i 3, t 1; 6612: t 2, X
       ending (51, 20)).
     - The hut is torn down at 6631.
     - The run of 6649 moves the cursor to the next record, building 14678 (on (58, 24)), with
       i 3, t 2 and X kept.
     - The run of 6686 builds the road from flag (58, 23) to the flag on (51, 20) over (57, 21)
       and (52, 18), 14 points, and finishes S (k 1, X cleared).

     An experiment build that cleared the state when taking up the new building built no road
     there. That also left those two fields free for player 2's guard house of 7020, and sent no
     carrier from player 2's headquarters at 6707 (whose door wait leaves out a take-out draw at
     6713, `ECO-distribution.md` ECO-DIST-1.7c). With the state kept, the fight game's generator
     is equal to 8072 (was 6714) and its buildings to 8280 (was 7030). The Weak, one-player and
     Hard games are unchanged. Test T101.
  b. Partner S by k. (Corrected 2026-10-25 night, E-ai-011.) The candidates
     of k = 0 and k = 1 are **warehouses only**: the game's list of
     warehouses (headquarters, storehouses and harbours, in the order they were
     added), finished (both construction parts at 0). "Route" here is the same
     measure as in c: from B's flag to the candidate's flag, a candidate counts
     only if both flags are in one *ware* network (TRN-CARRY-5.1a), and its
     route is the cost of the cheapest ware route (TRN-CARRY-5.3). The smallest
     wins; on equal values the first in the list. B unfinished: k = 0: among
     the finished warehouses whose store holds wood (count > 0), the one with
     the cheapest route; none: the same search for stone; none: done. k >= 1:
     done. **B being upgraded counts as unfinished** while its upgrade's
     construction has work left: the test reads the upgrade's construction in
     place of the building's own, which stays finished (`MIL-building-upgrade.md`
     MIL-UPG-3; E-ai-011; measured, E-trace-010: the guard house 90300,
     upgrading from counter 5582, current B from 5618; after the direct variant
     with the castle ends at 5803 (k = 1) the run of 5840 finds B unfinished with
     k = 1: done, cursor on to 90301, as recorded; an experiment build with this
     rule equals the recorded route task run for run to 6000, one that tests the
     building's own construction takes k = 1's nearest warehouse there; T44).
     This includes an upgrade ordered **earlier in the same think**: the
     military system's upgrade run (AI-MIL-4.5) runs before the construction
     system (AI-2.3), and its upgrade takes effect at once (AI-2.1a), so a
     route run in that tick already finds B unfinished (added 2026-10-28
     night, E-ai-043; measured, E-trace-014, T69).
     B finished and not a depot: k = 0: among all finished warehouses,
     the one with the cheapest route (no good condition); none: k = 1. k = 1:
     the own finished warehouse on B's landmass with the smallest field
     distance from B's field to its field (not a route); none: k = 2.
     k >= 2: the (k - 2)-th entry of the list made of the producer buildings
     of B's needs followed by the consumer buildings of B's productions
     (unplaced ones skipped: a need counts only when it has a production and
     that production a building; a production only when it has a need and
     that need a consumer building, so the locksmithery's tools production,
     whose need has none (AI-CON-10.2), adds no partner; T54); none: done. **B's needs are taken in the order of
     B's own list, not by need id** (added 2026-10-27 night, E-ai-028, measured T51): a need joins the end of
     the list when it is tied to B. When B is added, the inputs of its product
     are tied first, in the order of the product's cost list (`gp_addCost`),
     each being the first existing need of that good without a consumer or
     else a new need; then a stone producer's food need, then a planting
     building's need, the same way; needs tied to B later (the need creation's
     needs on the headquarters, AI-CON-3) join at the end then. So an older
     need tied to a new building can stand behind a newer one. B a depot (by the type's depot
     flag, AI-CON-1.2: the headquarters, a storehouse or a **harbour**, though
     a harbour is filed in its cell's harbour list; Q-279, T45): the 3 x 3 neighbouring
     cells are numbered 0, 1, 2, 4, 5, 6, 8, 9, 10 in scan order (columns
     outer, rows inner); the first cell with number >= k that exists, is not
     B's cell and has a depot gives S = its first depot and k = its number
     (the cell's depot list only: the headquarters and storehouses; a
     neighbour cell's harbour, in its harbour list, does not count; E-ai-023);
     none: done.
  c. Shortcut with S. (Corrected 2026-10-24 night, E-ai-010.) Everything in
     c works on the two buildings' **flags** Bf and Sf. "One network" is one
     *ware* network (`TRN-carriers-transport.md` TRN-CARRY-5.1a: roads with a
     carrier or donkey assigned), and every route length of c is the cost of
     the cheapest ware route (TRN-CARRY-5.3: L, 3L or 4L per road), not a step
     count. Bf and Sf not in one ware network: next partner. Else with r =
     cost(Bf -> Sf) and d = distance(Bf, Sf). **Every cost of c is a stored
     cost** (`TRN-carriers-transport.md` TRN-CARRY-5.3a, ware network, kind 1;
     2026-10-25, Q-362, E-ai-015, E-ai-017): a
     pair asked before (in either direction, by this task or by the economy's
     ware order matching, ECO-REQ-5.3) gives its stored value, else the search
     runs and its result is stored; a flag to itself costs 0. The store is
     emptied only when one of the player's ware networks is renumbered
     (SIM-START-4.3 (b), (c)). Measured (E-trace-008/009, building 90269,
     partner the castle): r = 10 (L of road 90277) at counters 1030, 1067,
     1104, 1141 and 1178, although that road's carrier held a ware at 1067 and
     1178 (3L = 30 then) and a road was built at 1163; road 90311 (built 1163)
     got its carrier at 1185. With the store as in 5.3a an experiment build
     reproduces the recorded route task run for run (cursor, k, i, t, w, X)
     from counter 993 to 2584 (E-trace-010; T36, T37), while "every road or
     flag change" parts at 1178, "removals only" at 919, "never" before 1067,
     and a store of the route task's own costs only (no economy entries) at
     2140 (T37). Then:
     - r > 4 d (*direct variant*): the free walk's path (one try, no draws)
       from Bf to Sf, its fields listed from Bf, **Bf itself being field 0**
       (none: next partner); w past its end: next partner. (Made exact
       2026-10-25 night, E-ai-011.) The search is the free walk's first try
       exactly (TRN-FREE-2.3 to 2.6: flood from Sf, h towards Bf, the
       walker's step cost, a field passable only if walkable or Bf itself; no
       second try, no building-and-flag shortcut of TRN-FREE-2.2, no flag,
       road, owner or bound tests); Bf and Sf neighbours: the path is [Bf, Sf].
       The read-out is TRN-FREE-3.1 / 3.2a with both draws replaced: the
       current direction starts at 0, and when it is not among the allowed
       directions of a field the lowest allowed direction becomes the current
       one. Else Fn = the own
       flag nearest (AI-CON-8.4, no exclusions) to the w-th field. If Fn
       exists and X is not empty and Fn is not X's last entry: with G the flag
       at X's last entry (if it exists), if cost(G -> Fn) > 4 x distance(Fn,
       G) the road Fn -> G is asked of the street construction's road builder
       (the one of 8.1: the route builder from Fn's field to G, built only when
       it completes a path whose step count is at most 2 x distance(Fn, G))
       and S is finished (next partner) **whether or not a road was built**
       (corrected 2026-10-25 night, Q-273: the builder's answer is not used;
       E-trace-008 counter 808: the sawmill (138, 49), Fn = the castle's flag,
       G = the sawmill's flag, cost 12 > 8, the two flags already joined, no
       road built, k = 1 and X empty after the run); otherwise X becomes just
       Fn's position. **G and Fn without a ware route between them** (the
       stored cost has none, for example G a lone flag) do not count as too long:
       no road is asked and X becomes just Fn (added 2026-11-01 night, T99,
       E-ai-064: the stored cost reports no route as -2, and the test is
       4 x distance < cost on signed values). This holds also for an X left by the
       route variant when the run turns to the direct variant (X is not cleared
       then). (Only the last Fn is kept; corrected 2026-10-25, measured:
       E-trace-009 `ai_route`, building 90300, counters 1289 to 1401: X holds
       one entry while it goes (139, 56), (141, 56), (142, 55)). Fn equal to X's
       last entry leaves X as it is. w rises by 1. One field per run. (The route
       variant appends to X, as below.)
     - r <= 4 d (*route variant*): the flags of the cheapest ware route are
       listed **from Bf (entry 0) to Sf (the last)**, by the costs L and 4L
       (the 3L of an unfree carrier is not counted for the list). **The search
       for this list runs from Sf towards Bf**: a road's cost is 4L when the
       flag it leads to in that direction (away from Sf) is full, so a full
       flag at Sf itself never adds 4L, while a full flag between them or at
       Bf does (corrected 2026-10-25 evening, T40: the list was searched from
       Bf; E-ai-020: the search starts at Sf's flag). **Of equally
       cheap routes** the list is read out from Bf as a ware's next flag is
       (`TRN-carriers-transport.md` TRN-CARRY-5.4): at each flag, starting at
       Bf, the first road in the flag's road list whose other end lies on a
       cheapest route (added 2026-10-27 night, E-ai-020;
       measured T55). The list is made afresh in every run, so a flag filling
       or emptying can change it between runs. i past the
       second-last: next partner. F0 = the i-th listed flag. Fn = the own flag
       nearest to F0 whose position is neither one of the first i + 2 listed
       flags nor in X; none: t = 0, X cleared, i + 1. Else Fn joins X; **if Fn
       is not in Sf's ware network the run ends here** (no try counted). Else,
       if the walk path Fn -> F0 (8.1e) counts and cost(Fn -> Sf) + its length
       + cost(F0 -> Bf) < r (its **length here is not its step count** but the
       road length L of the path as for a road, TRN-CARRY-5.2: trunc of the sum
       over its steps of 1/f(p -> q) + 1/f(q -> p), both directions; corrected
       2026-10-25 evening, E-ai-017, measured T36: the
       earlier reading took one direction only; the step count is only used for
       "counts"), the road Fn -> F0 is asked of the same road
       builder as in the direct variant and S is finished (whatever the
       builder answers). **The two stored costs are asked only when the walk
       path counts** (cost(Fn -> Sf) first, then cost(F0 -> Bf)); a try whose
       walk path does not count asks neither, so it leaves no pair in the store
       (TRN-CARRY-5.3a; added 2026-10-27 b, E-ai-040, measured T63). Else
       t + 1; above 2: t = 0, X cleared, i + 1. With entry 0 being Bf, the
       first tries (i = 0) take F0 = Bf, so cost(F0 -> Bf) = 0, and a flag Fn
       near Bf whose cost to Sf plus the new road beats r is joined to Bf. Fn
       can be Sf itself (a try at i = 0 then compares just the new road's L
       with r, T36).
     Finishing S: k + 1, X cleared, i = t = w = 0. The state (k, i, t, w, X)
     is one for both variants, so a run in one variant continues the indices
     left by the other when r crosses 4 d between runs.
  Confidence: high for the flags, the list's order, the ware costs and the
  untried Fn (the code's arguments, E-ai-010; in an experiment build, with
  TRN-CARRY-5.3's free carrier as corrected, the generator's first
  difference moved from 708 to 848); high for b's warehouse candidates and
  ware routes and for the run timing (Q-353, T32, T33): the original's
  cursor read from E-trace-007's heap dumps is 90181 in counter 523 (the
  castle done at 512) and 90269 in 1023, and an experiment build with b as
  corrected has the same cursor in both runs and builds the road "140,50
  141,50 141,49 141,48" at 660 as the original does (with "any finished
  building over all roads" the unfinished sawmill took the castle as its
  partner at 401, the shortcut then found no ware network, and the extra run
  put the road at 697); the whole run timeline of T33 is measured run for run
  in E-trace-008's `ai_route` table (2026-10-25: cursor, k, i, t, w and X after
  every run from counter 0 to 6002, `traces/original-dense-6000-ai_route.csv.gz`);
  medium for the direct variant's pathfinder, being read from the code only, and
  the list's cost (the 4L-only search).
- **AI-CON-8.4 Flag search.** (Start field and neighbour order corrected
  2026-10-24, Q-269.) AI-CON-8.1 starts it at F's own field (the flag's field,
  not the building's). Each field taken from the queue (first in, first out)
  has its six neighbours tested in the order **5, 0, 1, 2, 3, 4**
  (the ring 1 order of `MAP-grid-geometry.md` MAP-GRID-3.2), not 0 to 5: the search
  steps once in direction 4 and then once
  in each of the directions 0 .. 5, testing the field reached after each step, so
  the first field tested is the direction 5 neighbour. A neighbour is tested as a
  target when it is reached (before it is queued), so among targets at the same
  depth the one reached through the earlier queued field and the earlier
  direction wins. Confidence: high (E-ai-005: the flag's field is the
  start; the step in direction 4 before the loop over 0 .. 5, the
  queue taken from the front). Breadth-first, **with no limit on the number of
  steps** (corrected 2026-10-25, Q-360), over the fields within distance 10 of
  the start that are walkable (TRN-FREE-2.5) and do not carry DepositBlocked
  (trees, stones); the start itself is tested first and always admitted. So a
  flag at distance 10 can be found over a detour of more than 10 steps (T35).
  Confidence: high (code E-ai-014: the admission test is
  the distance, the DepositBlocked bit and the walkable test only; measured T35). The first own flag found is the result, unless its position
  is in the caller's excluded list or (when the caller gives a network) it
  belongs to that same road network. Confidence: high.

### AI-CON-9 Demolition (Q-068)

- **AI-CON-9.1 Protected.** A building whose record has any of the four flags
  (military, depot, planting building, harbour) is never torn down by this
  part. Confidence: high.
- **AI-CON-9.2 Candidates (every 41 ticks, next building in id order).** Only
  buildings whose first product is a resource good (wood, stone, fish, coal,
  ironOre, gold) and hunters are tested. The test needs the building to be
  unfinished or to have **one of its settlers standing (not walking) on the
  building's own field**, i.e. its worker at home (corrected 2026-10-25 evening:
  "present or on the way" was read too wide; a hunter out hunting, or a worker
  walking out or in, does not count; T41, E-ai-021). Then the site value
  (AI-CON-6) of the building's type at its position is computed: v > 0 keeps
  the building, except a fisher whose productivity record is 0 (no catch),
  which is a candidate; v <= 0 is a candidate. A woodcutter candidate is kept
  only if its cell **has a forester** (the cell's forester reference, AI-CON-7.1)
  and exactly one wood production (the cell's productions whose good is wood
  or has wood as its group good); without a forester in the cell it is torn
  down (corrected 2026-10-26 night: "kept if the only wood producer" was read
  without the forester test; E-eco-019; measured,
  E-trace-010: the woodcutter's hut 90383 on (136, 47), cell (8, 2) with one
  wood production and no forester (heap dump 5500: the cell's forester
  reference unset), v = 0 at counter 5816: torn down with remains, plank 90780
  on its flag and ruin 90781; T43). The wood count is a plain count: a cell with
  a forester and five wood productions does not keep its woodcutter, which then
  stands only while v > 0, a sapling within reach being enough (T90). Confidence:
  high (conditions), medium (meaning of "worker present").
- **AI-CON-9.3 Tear-down.** The productions of the building are detached from
  their needs' squares and relinked, then the player's demolish command is
  sent (`TRN-roads-flags.md` TRN-ROAD-6 for what happens). The command removes
  the building **with remains** (`MIL-conquest.md` MIL-CON-4.1, 4.4): the
  material wares are put on its flag first (each taking its object number),
  then the building is removed and its ruin made (one more number); T41.
  Confidence: high.
  **What happens to the needs** (2026-10-30, T84; E-ai-053): when the building is gone, each of its
  productions is removed, and a need that was linked to one loses the
  production, its square (the square's slot is free again) **and its cell**: it
  is a new need again, and the next needs arrangement (AI-CON-3) places it as
  any new need, choosing its cell afresh (for a resource good the cheapest cell
  with a free square, AI-CON-3.4) and linking it to a free production of its
  good in that cell, **placed or not** (one without a need; T84's woodcutter
  production had stood since counter 9628). Confidence: high for the outcome
  (measured T84), medium for the steps between the removal and the next
  arrangement.
- **AI-CON-9.4 Clearing (every 59 ticks).** In every cell with the clearing
  marker and ordinary buildings, the first building whose size equals the
  marker's size is torn down and the marker cleared (one per cell per run).
  Confidence: high. **This never happens in a game (Q-1000 a, 2026-11-04, E-sav-008):**
  no code sets the clearing marker or a non-zero size, and the task named for it (`AI CellClearing`)
  has only save and load, no run. It could fire only from a save that holds the marker set, and no
  sample does. Keep it as a no-op; do not invent a size. Confidence: high (code search); the earlier
  "set by the military part" was a guess.
- **AI-CON-9.5 Flags (every tick that is not a multiple of 41 or 59).** The
  next flag of the player (cursor over the player's flag list in creation
  order, the list `TER-territory.md` TER-6.3 walks; a removed flag leaves the
  order of the others unchanged; AI-CON-8.1 uses the same list, Q-163): if it has no building, no
  wares and no road it is removed; if it has exactly one road (and no building
  and no wares), it is removed unless that road's carrier or donkey is
  **carrying a ware** at that moment (it holds a ware that is not lying; an
  assigned carrier walking empty does not keep the flag; corrected 2026-10-27
  night, E-ai-025, measured T47). Removal uses the
  player's flag removal (TRN-ROAD-6). Confidence: high.
  **The cursor** (added 2026-10-27 night, E-ai-027, measured T50):
  the run reads the list as it is at that moment, which includes the flags
  the computer player made earlier in the same think (its construction
  part runs before; its commands act at once, AI-2.1a), such as the flag of
  a site it has just placed. A cursor at or past the end of the list starts
  again at the first flag (it is not taken modulo the list's length); the
  flag at the cursor is tested, then the cursor moves on by one, whether or
  not the flag was removed. An empty list leaves the cursor unchanged.
  Confidence: high.

### AI-CON-10 Production control, tools, distribution

- **AI-CON-10.1 Production control (every 67 ticks).** Goods are visited in
  id order **read as signed 32-bit numbers** (ids with the top bit set first,
  as in AI-CON-4.2; corrected 2026-10-28 night, measured T73), one qualifying
  good per run (cursor). Qualifying: group goods and
  deposit/fields goods only if they are donkey, corn or fish; other wares all
  but coin, plank and boat. Stock limit of g: stock(g) + stock2(g) < 40 +
  (S x 60) / 9216 (integer division; donkeys: stock of the settler kind
  `good_donkey` only). Every finished producer of g is switched on when below
  the limit and off when at or above it. The same test gates AI-CON-4.1.
  Confidence: high.
- **AI-CON-10.2 Locksmith (every 337 ticks, not during need stage 1).** Tool
  goods with stock(t) + stock2(t) = 0 are "missing"; the next missing tool
  after the last one handled (cyclic) is chosen. If none is missing: the
  tools need's building, if finished and on, is switched off. Else: if no
  tools need exists one is created; otherwise its producer (if finished) is
  switched on and the player's 12 tool sliders are set to 1.0 for the chosen
  tool and 0 (floor 0.001) for all others. The tools need is made with no
  consumer building and is not tied to the headquarters (unlike the needs of
  AI-CON-2; added 2026-10-27 night, E-ai-032, which creates it with the new-need step
  alone (no tie to a building); heap dump 10500 of E-trace-011: the tools need's consumer
  reference is none; T54). Confidence: high.
- **AI-CON-10.3 Food distribution (every 643 ticks).** In the food table:
  stone mine 1.0 if stock(stone) < 5 else 0.5. Then gold mine, coal mine and
  iron mine, each with its ore (gold, coal, iron ore): **0.3** if stock(ore) > 4;
  otherwise **1.0** if some need of that ore (`AI-construction.md` needs) has a
  consumer building that is *ready* and *starved*: every need tied to that
  building has a production (a placed producer, as in `AI-military.md`
  AI-MIL-6.2), it is finished with its worker in it, the ore is in its type's
  input list and its own input store holds none of it (an input never delivered
  counts as none); otherwise **0.5**. **"Its worker in it"** (clarified
  2026-11-01, T98): one of the building's workers stands, not walking, on the
  building's own field at the moment of the run; a worker out carrying a
  product to the flag or walking back in does not count, so the building is
  passed over in that run (E-ai-063: each worker's movement standing
  and its position equal to the building's). The needs are looked at in need-map order,
  the first such consumer decides. (Corrected 2026-10-30, T87: the 1.0 branch was
  read as unreachable; E-ai-056.)
  Confidence: high (code; measured T87).
- **AI-CON-10.4 Goods arrangement (every 643 ticks; Q-076).** A ten-step
  cycle, one step per run, that sets the per-warehouse good flags
  (`ECO-distribution.md` ECO-DIST-1.1: *stop* and *out*) of the player's
  finished storehouses and harbours (the cells' depot and harbour lists; the
  headquarters is in a depot list, AI-CON-1.1, and is treated as a storehouse:
  in E-trace-006 the run of counter 1280 gave the computer player's castle, then
  its only depot, flag records for coin, stone and plank, all flags clear;
  corrected 2026-10-24, before that this text said the headquarters is never
  touched):
  - Step 0, for plank, stone and coin: over the finished storehouses (not
    harbours) the smallest and the largest own stock of the good are found,
    remembering the building with the smallest and the building with the
    largest (the **first** found in each case: a later one must be strictly
    smaller or larger). If smallest + 40 < largest: every finished storehouse
    and harbour gets *stop* for that good, then the building with the largest
    stock gets *out* set and **the building with the smallest stock gets *stop*
    cleared** (corrected 2026-10-31 night, T94; E-ai-060; read before as both on the fullest). So for the next 643 ticks
    the fullest storehouse sends its units out and only the emptiest one stores
    the good (besides its consumers). **"First found"** follows the player's
    cell list in its order (creation order, AI-CON-7.1) and, in each cell, the
    cell's depot list in the order its buildings were added (the headquarters
    is in its cell's depot list); a storehouse whose construction parts are not
    both done is skipped (added 2026-11-02 night, T100; E-trn-029). Not by field order. **What the run leaves**, warehouse by
    warehouse: *stop* set on every one except the emptiest, *stop* clear on the
    emptiest, *out* set on the fullest, the other *out* flags as they were. (A
    build that compares each of the three steps with the flags as they were
    before the run, and skips the steps that seem to change nothing, leaves an
    emptiest storehouse with a clear record stopped: T100.)
  - Step 1: plank, stone, coin: *stop* and *out* cleared in all finished
    storehouses and harbours.
  - Steps 2 and 7, for the five soldier ranks: a finished storehouse or
    harbour is "at the front" when its cell's military list holds a finished
    military building with enemy distance below 36 (`MIL-attack-combat.md`
    enemy distance of the building). If at least one warehouse is at the
    front: front warehouses get *out* cleared, all others get *out* set for
    the five soldier goods (soldiers gather at the front). If none is at the
    front nothing changes.
  - Steps 3 and 8: *out* cleared for the five soldier goods in all finished
    storehouses and harbours.
  - Steps 4, 5, 6, 9: nothing.
  Confidence: high (steps, goods, flags); medium (the enemy distance field of
  a military building is the one `MIL-attack-combat.md` uses).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| cell size | 16 | fields | high | E-ai-002 |
| resource square | 4 | fields | high | E-ai-002 |
| need stages | 0, 300, 800, 1200 | s | high | E-ai-002 |
| stock check period | 600 x 2^n | s | high | E-ai-002 |
| plank / stone scale | 0.03125 / 0.0625 x S | | high | E-ai-002 |
| food sub-need odds | fish 7/10, bread 2/10, flesh 1/10; replacement bread 5/6 | | high | E-ai-002 |
| input stock threshold | <= 10 | wares | high | E-ai-002 |
| cell capacity | 20 productions | | high | E-ai-002 |
| cell cost per producer | 50 | | high | E-ai-002 |
| score constants | 100, 0.01, 0.001, 0.2, 0.1 | | high | E-ai-002 |
| budget reset / step / growth | 600 / 50 ticks / 2L+2 | | high | E-ai-002 |
| search per tick / route limit | 10 fields / 16 (2R) | | high | E-ai-002 |
| order weights | needs 10, forester 50, hunter 10, pit/woodcutter 10, depot 15, harbour 16, military 20 | | high | E-ai-002 |
| forester limit | 40, 10 when plank+wood >= 40 | | high | E-ai-002 |
| full-mark lifetime | 600 x (expiries so far): 0 (until the next 37-tick check), 600, 1200, ... | s | high | E-ai-002, E-ai-031; T53 |
| destruction / clearing / flag periods | 41 / 59 / other ticks | ticks | high | E-ai-002 |
| stock limit | 40 + 60 S / 9216 | wares | high | E-ai-002 |
| road acceptance | length - 1 <= 2 x distance; search radius 10 | | high | E-ai-002 |
| shortcut variants | route > 4 x distance: direct; else route | | high | E-ai-005 |
| route optimizer tries per route flag | 3 | | high | E-ai-005 |
| street strategies / attempts / give-ups per flag | 5 / 6 / 6 | | high | E-ai-005 |
| stock refresh cycle | 22 (stock at 0, stock2 at 2) | ticks | high | E-ai-005 |
| S refresh cycle | 601 | ticks | high | E-ai-005 |
| goods arrangement spread | smallest + 40 < largest | wares | high | E-ai-005 |
| goods arrangement front distance | enemy distance < 36 | fields | high | E-ai-005 |
| goods arrangement cycle | 10 steps (0, 1, 2, 3, 7, 8 act) | runs | high | E-ai-005 |
| pavement rule | rings 1 (2 large), building within 3 (4) | fields | medium | E-ai-005 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | profile Strong (3 sawmills), t = 0 | first need creation | 1 stone need and 3 plank needs attached to the headquarters |
| T2 | profile Weak (max smithies 2, max mints 0), t >= 300, one cell with quarry, woodcutter and sawmill | need creation at t = 300 and t = 1200 | stage 2: one sword and one beer need; at t >= 1200 stage 4 without a coin need |
| T3 | S = 1536, 1 plank producer, stock(plank) 40 | t crosses 600 | trunc(1536 x 0.03125) = 48 > 40: a plank need is created; next check at t > 1200 |
| T4 | needs: bread D=2, A=0, priority 70, stock 0; coal D=1, A=1, priority 100, stock 3 | needs construction | bread score = 2 x 70 x 0.99 / 0.2 x 1 = 693; coal = 0 x ... = 0; bread chosen |
| T5 | one cell with 21 productions, need for plank | arrangement | the cell is not accepted (>= 20) unless the good were coal/stone/ore/gold |
| T6 | idle construction system, L = 0, C = 0, orders pending | tick 50 | an order is taken at once; after placement L = 2; the next order starts after two idle 50-tick steps |
| T7 | cell with 60 trees, no forester, a sawmill, stock plank+wood 10 | forester run on that cell | no order (60 >= 40); with 30 trees: forester order weight 50 |
| T8 | cell with 1 quarry and 12 stones | stone/wood run | 5 x 1 < 12: stone pit order |
| T9 | fisher, finished, worker inside, productivity 0, fish in radius | destruction run on it | torn down (v > 0 but productivity 0) |
| T10 | woodcutter, trees gone (v = 0), only wood producer in its cell, the cell has a forester (9.2) | destruction run | kept; the same without a forester in the cell: torn down (T43) |
| T11 | flag with no building, no wares, one road without carrier | flag check tick | flag removed |
| T12 | S = 9216, stock(fish) 100, stock2 0 | production control on fish | 100 >= 40 + 60: fishers switched off |
| T13 | tools: saw missing, others in stock | locksmith run | tools need created; next run (producer finished): slider saw 1.0, others 0.001, producer on |
| T14 | cell with 1 military building (the HQ), 6 ordinary buildings, no harbour | a 7th ordinary building is added | 5 x 1 < 7: crowded marker set, weight 1 |
| T15 | cell with a harbour, no military building, 1 ordinary building | ordinary building added | 5 x 0 < 2 + 1: crowded set, weight 100 |
| T16 | storehouses A (plank 50), B (plank 5), harbour H (plank 20); step 0 due | goods arrangement | 5 + 40 < 50: stop(plank) on A, B, H; then A: out on, stop off; B and H keep stop; next run (step 1) clears all |
| T17 | storehouse A in a cell with a tower at enemy distance 20, storehouse B in a cell without military building; step 2 due | goods arrangement | soldier0..4: out off in A, out on in B; step 3 (next run): out off in both |
| T18 | fish need, resource square with fish (kind 1) | needs construction | producer type = fisher (mine table); flesh need in a cell: slaughterhouse |
| T19 | finished smithy B, partner S in the same network, route 30, distance 5 (30 > 20) | route optimizer, 3 runs | direct variant: one path field per run; a road is built only between two found flags whose route is > 4 x their distance |
| T20 | player with 3 storehouses holding 10 planks each and 7 planks lying at flags, tick 44 | stock counts | stock(plank) = 30 (from cycle tick 0), stock2(plank) = 7 (cycle tick 2); unchanged until tick 66 |
| T21 | MP_2P_Storm_Coast, default set-up, player 1 computer (E-trace-002; game counters k, site in that tick's state) | the first ten sites | hunter (140, 49) k 127 (taken at P 44, the only order; n 80); sawMill (138, 49) 323 (P 244 after C 1, 2 at 144, 194; n 76); hunter (139, 47) 596 (P 544, C reached 6 at 494 .. the 494 step took nothing lasting; n 49; the 594 reset falls in the search, placement sets L 2); woodCutter (142, 56) 818 (P 744; n 71); guardHouse (138, 57) 1100 (point mode; P 1044, n 53, or P 994, n 103); stonePit (140, 57) 1308 (P 1244, n 61, or P 1194 the reset tick, n 111); harbor (144, 47) 1474 (P 1444; n 27); woodCutter (136, 47) 1838 (P 1794, the reset tick, idle: L 0 C 0 takes at once; n 41); stonePit (139, 55) 2703 (P 2644; n 56); smithy (137, 55) 3198 (P 3144; n 51). Each site tick = P + 3 + n with P = 44 (mod 50); n follows from AI-CON-5.4 on the map state |
| T22 | MP_2P_Storm_Coast (170 x 170: grid 11 x 11), player 1 computer (v = k + 6; E-trace-002) | stone pit / woodcutter cursor (7.3) | runs at k = 47, 100, 153, 206, 259, 312 look at grid positions (1, 0) .. (6, 0): the player has no cell there, so no order; the home cell (8, 3) is first looked at in run 41 (v 2173, k 2167), the cells (7, 2) .. (9, 2) in runs 29 .. 31 (k 1531, 1584, 1637), (0, 0) in run 121 (k 6407). At the pick of k 294 the only order is the sawmill's plank need; the stone pit (140, 57) of k 1308 (T21) is a needs order, not a 7.3 order |
| T23 | MP_2P_Storm_Coast, player 1, home cell (8, 3), centre (136, 56), headquarters on (139, 51) (distance 5 to the centre), hunter site (140, 49) and sawmill (138, 49) (7 each) | cell position; sawmill site of T21 | the position stays (139, 51); at the sawmill search the medium fields (138, 49) and (137, 51) both have route length 2 from it, so they tie and the first in scan order, (138, 49) (row 49), wins; measured from the cell's centre they would be 7 and 5 and (137, 51) would win (Q-250) |
| T24 | E-trace-002, player 1: flags castle (140, 50) id 90182, hunter (141, 48) created tick 127, sawmill (139, 48) created tick 323; street runs at k = 1 mod 7 | the runs' flags and the roads' ticks | 127 castle (no T); 134 hunter, T castle remembered; 141 .. 169 attempts 1 .. 5; 176 road "141,48 140,48 139,49 140,50"; 183 .. 323 castle / hunter alternating, no T; 330 hunter gives up the sawmill's flag; 337 hunter no T; 344 sawmill, T remembered; 351, 358 attempts 1, 2 without draws; 365, 372, 379 attempts 3 .. 5 with 2, 3, 3 draws; 386 attempt 6: road 90199 removed, roads "139,48 139,49 140,50" and "139,48 140,48 141,48" built (AI-CON-8.1d) |
| T25 | E-trace-002, player 1, start resources Medium (castle stock 24 wood, 44 plank, 68 stone) | the wood need and site 5 | the sawmill site (138, 49) placed at k 323 is announced and gets a new wood need (AI-CON-2.5; no wood need existed: stock(wood) 24 > 10, so AI-CON-3.2 made none); the arrangement of k 332 gives it a cell; the needs construction of k 334 scores (AI-CON-4.2, stock + stock2, no producer of any of the three, avgProd 1): wood D 1, A 0, priority 400: 400 x 0.76 / 0.2 = 1520; stone D 1, A 0, priority 300: 300 x 0.32 / 0.2 = 480; plank D 2, A 1 (the sawmill site), priority 300: 300 x 0.56 / 1 = 168 -> woodCutter order (weight 10, time of k 334), taken at P 744 -> woodCutter at k 818 (trace tick 819; field (142, 56) in the original, Q-333); the stone pit follows at P 1244 (k 1308). Measured in a clone: without 2.5 the order at 334 is the stone pit (480 wins), with it the woodcutter, in the original's tick |
| T26 | computer player, no wood need, a sawmill site is placed | announcement | one new wood need with the sawmill site as consumer and no cell; a second sawmill site placed while that need still exists makes another wood need (the first already has a consumer); with an existing wood need that has no consumer (made by AI-CON-3.2), that need is tied to the sawmill site and no new need is made; a woodcutter, stone pit, hunter or the headquarters: no need (AI-CON-2.5) |
| T27 | E-trace-002, player 1, the woodcutter's site search of k 744 .. 818 (cell (8, 3), routes from the castle (139, 51), working radius 6) | site 5 | (140, 53): 18 trees, no Border field on its distance-2 ring, class 3 x 0.5 -> 9, route 3 -> 9 x 13 / 16 = 7.3125; (142, 56): 37 trees, nearest Border field 4 away (none on the ring), class 4 x 0.5 -> 18.5, route 6 -> 11.5625; (142, 57): 41 trees, class 4 -> 20.5, route 7 -> 11.53125; the best is (142, 56), placed at k 818 (trace tick 819) as in the original. With the old reading (any Border field within 6) (142, 56) and (142, 57) take x 0.4 and (140, 53) wins (s2-reborn before Q-333). Tree counts as s2-reborn counts them (AI-CON-6.2); the chosen field measured in an experiment build |
| T28 | site value of a non-military type at p | border factor | a Border field at distance 2 from p: x 0.4; Border fields only at distance 1 (none at 2) or only at 3 .. 6: no factor; a Border field on p itself and none at distance 2: no factor |
| T29 | E-trace-002, player 1, the street run of counter 344: F = the sawmill's flag (139, 48), roads "141,48 140,48 139,49 140,50" (hunter's flag to castle's flag) | flag search from F (AI-CON-8.4) | neighbours of F tested (139, 49), (140, 48), (139, 47), (138, 47), (138, 48), (138, 49); (139, 49) is taken from the queue first and its direction 5 neighbour (140, 50), the castle's flag, is the first target: T = the castle's flag, so strategy 4 builds F -> castle first (the original's road 90224, carrier requests 90232 at position 0, 90233 at position 1, counter 387). With the order 0 .. 5 the hunter's flag (141, 48), reached through (140, 48), would come first (Q-269) |
| T30 | E-trace-006 (and E-trace-002), player 1: flag F (140, 46) of the hunter site (139, 47), created counter 598, T = the hunter's flag (141, 48); roads "139,48 139,49 140,50", "139,48 140,48 141,48" (R, attached to T) | street runs of counters 603 .. 645 | 603 T remembered; 610 attempt 1: walk path F -> T 2 steps (through (140, 47)) -> L 2, best 1; 617 attempt 2: nothing shorter; 624, 631, 638 attempts 3 .. 5: the road-stop path "140,47 141,48" is 3 fields with F, 3 > L 2: L 2, L2 1000, best 1 stay (each attempt still reads out and draws its start direction); 645 attempt 6: strategy 1, road "140,46 140,47 141,48" built, no draw, R stays. Counting the road-stop path in steps (2 <= 2) takes strategy 3 at 624 (L2 2), repeats the search at 645 (one more draw, trace tick 646) and removes R (a carrier standing at (139, 48) at trace tick 682): the old first differences |
| T31 | E-trace-006, player 1: F = the sawmill's flag (139, 48), T = the castle's flag (140, 50), the road R "141,48 140,48 139,49 140,50" (hunter's flag to T) | street runs 351 .. 386 | 351 attempt 1: no walk path F -> T counts (the 2-step way passes (139, 49), a field of R; the experiment build's search finds no other within 4 steps): L 1000; 358 attempt 2: nothing; 365 attempt 3: road-stop path "139,49 140,50", 3 fields -> L 3; second leg T -> P = (141, 48) over R's fields 3 steps -> L2 3, best 3; 372 attempt 4: F -> P 2 steps through (140, 48), a field of R -> L2 2, best 4; 379 attempt 5: nothing below L2 2; 386: strategy 4 (T24). Every value equals E-trace-006's `ai_street` rows (L, L2, best) |
| T32 | E-trace-006, player 1: the hunter (140, 49) with flag (141, 48), roads "139,48 139,49 140,50" (carrier 90234) and "139,48 140,48 141,48" (carrier 90237), "140,46 140,47 141,48" (no carrier yet); route optimizer runs at counters k with k + 6 = 0 (mod 37) | the road "140,50 141,50 141,49 141,48" | measured: the original builds it from the castle's flag to the hunter's flag in counter 660 (its carrier request appears in 662; snapshots 660, 661 missing; only the street construction and the route optimizer build roads, and the street task is idle then). Experiment build with 8.1e, 8.3 c and TRN-CARRY-5.3 as corrected: B = the hunter, S = the castle, cost(Bf -> Sf) = 6 + 4 = 10 at 660 (the 2-step road (141, 48) .. (139, 48) is L 6 on its slopes: the original's heights there are 5002, 6320, 7219, and the experiment build computes the same L) > 4 x 2: direct variant; field 2 of the path (139, 49) has the castle's flag as nearest flag, X's last entry is the hunter's flag, cost > 8: road from the castle's flag to the hunter's flag, the same road, but at 697 (the hunter handled from 623). The run timing is Q-353, answered by T33: with 8.3 b as corrected (2026-10-25) the hunter is handled from 586, the direct variant's w = 0, 1, 2 at 586, 623, 660, and the road is built at 660 |
| T33 | E-trace-006 / 007, player 1: the route optimizer from counter 31 (runs at counters k with k + 6 = 0 mod 37); records: castle 90180 (finished), hunter 90190 (placed 127), sawmill 90211 (placed 323), hunter 90244 (placed 596) | the cursor and the building each run handles | 31 cursor to 90180; 68 castle (a depot, no neighbour cell with a depot) done, cursor 90181; 105 no record >= 90181: restart; 142 cursor 90180; 179 castle done; 216 cursor 90190; 253, 290, 327 hunter (unfinished: k = 0 partner the castle, route variant; then k = 1 done); 364 cursor 90211; 401 sawmill (unfinished): the castle is not in its flag's ware network, so no wood or stone warehouse is found and the sawmill is done in this run; 438 restart; 475 cursor 90180; 512 castle done, **cursor 90181** (measured: 90181, k = i = t = w = 0, X empty in counter 523, E-trace-007 heap dump 500); 549 cursor 90190; 586, 623, 660 hunter, direct variant w = 0, 1, 2, the road "140,50 141,50 141,49 141,48" at 660 (measured, E-trace-006); after 993 the cursor is **90269** (measured in counter 1023, heap dump 1000). Measured in every run (E-trace-008 `ai_route`, the state after each run): 31 90180; 68 90181; 105 0 (restart); 142 90180; 179 90181; 216 90190; 253 i = 1; 290 k = 1; 327 90191; 364 90211; 401 90212; 438 0; 475 90180; 512 90181; 549 90190; 586 w = 1, X = [(141, 48)]; 623 w = 2; 660 k = 1, X empty (the road); 697 90191; 734 90211; 771 w = 1, X = [(139, 48)]; 808 k = 1; 845 90212; 882 90244; 919 k = 1; 956 90245; 993 90269 |
| T34 | AI-CON-5.4a, 6.3: E-trace-008 (two-player game, computer player Weak), the military order for a guard house in cell (8, 3) (cell mode, from the crowded-cell target (128, 48)), taken at trace tick 1045 | search and site | 53 search steps, site (138, 57) placed in trace tick 1101 (measured): value 45.5 ((85 fields + 6) x 0.5 for its large class), route length 7, score 45.5 x 9 / 16 = 25.59375, above (135, 56) with 50.5 x 8 / 16 = 25.25 and (136, 48) with 32 x 11 / 16 = 22.0 (values from the code, medium: corrected 2026-10-25 evening; the earlier 41 and 46 came from an experiment build whose landmass left out the 9 blocked stone fields (138, 61), (140, 62), (138 .. 139, 62 .. 64), (140, 64), which the stored index 12 includes); the fields within 4 of the castle (139, 51) fail the size test cheaply. With every non-water field as one landmass the mountains west of the castle count and (135, 48) wins with 55.5 x 10 / 16; with the castle's fields as candidates the search takes 68 steps (placement in 1116). Then the stone pit (140, 57) in 1309 |
| T35 | AI-CON-8.1, 8.4: E-trace-008, the street construction's run of counter 1485 on the stone pit's flag F = (141, 56) (its network: every flag of the player but the harbour's flag (145, 46), made at 1474 and without roads) | flag search and run | T = (145, 46) at distance 10 (no way of 10 steps exists around the trees, a longer one does); F has roads, T none: T given up (the street task shows give-ups 1); the next run (1492) moves on, the harbour's flag is handled from 1499 with attempts 1 .. 5 at 1506 .. 1534 and its road built at 1541 (the leveller then leaves the castle at trace tick 1543). With a 10-step limit F finds no T, everything comes one run (7 ticks) earlier |
| T36 | AI-CON-8.3 c, TRN-CARRY-5.2, 5.3a: E-trace-010 (two-player game, computer player Weak), route task on building 90322 (flag Bf (141, 56)), partner the castle (flag Sf (140, 50)), d = 6, stored r = 14 (route variant); the route's flags [(141, 56), (142, 55), (140, 50)], i = 0, F0 = Bf; X after the runs of counters 1548 (direct variant: X = [(141, 56)], w = 1) and 1585 (Fn (139, 56), t = 1) | the run of counter 1622 | Fn = Sf (140, 50) (not among the first two listed flags, not in X); the walk path (140, 50) -> (141, 56) is found with 9 steps, "140,50 139,50 138,50 137,51 138,52 138,53 139,54 139,55 140,55 141,56" (9 <= 2 x 6: counts); its road length L = 20; cost(Sf -> Sf) = 0, cost(Bf -> Bf) = 0; 0 + 20 + 0 = 20 is not < 14: no road, t = 2, X = [(141, 56), (139, 56), (140, 50)] (measured: ai_route and ai_walk; with the one-way length 10 an experiment build asks for the road here). Then 1659: Fn (141, 48), walk path none, t = 3 > 2: t = 0, X cleared, i = 1; 1696: Fn (139, 56), 7 steps; 1733: Fn (141, 48), none; 1770: Fn (139, 48), 18 steps (> 2 x 7, does not count), i = 2; 1807: k = 1 |
| T37 | TRN-CARRY-5.3a, ECO-REQ-5.3, AI-CON-8.3 c: E-trace-010, the computer player's woodcutter site 90383 at (136, 47) (placed 1838, flag (137, 46)); road (137, 46) .. (139, 48) made 1968, its carrier assigned 1999 (ware renumbering: the store emptied); the site's ware orders matched every 11 ticks from counter 2002, the castle's flag (140, 50) holding fewer than 6 wares then; at 2140 it holds 6 (5 lying, one being carried out) | the route task's run of counter 2140 on building 90383, partner the castle | r = the stored cost 10 (6 for (137, 46) -> (139, 48), 4 for (139, 48) -> (140, 50), stored by the ware order matching from the site's flag towards the castle's flag after the 1999 renumbering: at 2002 in an experiment build), at most 4 x 5 = 20: route variant, Fn (140, 46), t = 1, X = [(140, 46)] (measured). A fresh search at 2140 would give 6 + 4 x 4 = 22 (the castle's flag full), above 20: the direct variant, which the original did not take |
| T38 | AI-CON-6.3, 6.4, 7.1: E-trace-010 (two-player game, computer player Weak), the farm order (needs order taken at counter 3695, cell (8, 3), cell mode, routes up to 16) | the farm's site | candidate (135, 57): ring-2 fields (134, 55), (134, 56), (133, 57) hold trees (DepositBlocked), the other 9 pass: v = (0 + 9) x 0.4 (Border on the ring) = 3.6, route length 3, score 3.6 x 13 / 16 = 2.925; candidate (136, 58): all 12 pass, v = 12 x 0.4 = 4.8, route length 3, score 3.9, the best: the farm is placed at (136, 58) in trace tick 3725 (measured). Counting the 19 fields within radius 2 by terrain alone gives both 19 x 0.4 = 7.6, the tie keeps the first, (135, 57) |
| T39 | AI-CON-6.2, 6.4, 7.1: E-trace-010, player 1, the brewery order (needs order taken at counter 3945, cell (8, 3), cell mode, routes up to 16); own buildings then: the well's site (136, 53) (frame), the farm's site (136, 58) (levelling, placed 3725, T38) | the brewery's site | (136, 56) is within 4 of the farm's site: v = 0. (136, 51): supplier values water 15 - 2 = 13 (the well's site), grain 15 - 7 = 8 (the farm's site), v = 21, route length 4, score 21 x 12 / 16 = 15.75, the best ((137, 51) ties later, the first stays): the brewery is placed at (136, 51) in trace tick 3987 (measured). Counting finished farms only, (136, 56) has v = 12 + 13 = 25, route length 2, score 21.875 and wins |
| T40 | AI-CON-8.3 c: E-trace-010, player 1, the route task on the hunter 90190 (flag Bf (141, 48)), partner the castle (flag Sf (140, 50)), route variant (stored r = 8, d = 2); roads (141, 48) .. (140, 50) "140,50 141,50 141,49 141,48" (direct), (141, 48) .. (139, 48) and (139, 48) .. (140, 50); the castle's flag full | the runs of counters 2584 and 2621 | searched from Sf, the castle's flag adds no 4L: the direct road is the cheapest, the list is [(141, 48), (140, 50)]; i = 0, F0 = Bf; 2584: Fn = (140, 46) (walk path 3 steps, no road), X = [(140, 46)]; 2621: Fn = (139, 48), X = [(140, 46), (139, 48)], no walk path (measured, ai_route and ai_walk). Searched from Bf, the road into the full castle flag costs 4L on both ways and the list becomes [(141, 48), (139, 48), (140, 50)]: (139, 48) is then excluded and 2621 takes Fn = (140, 50); the route task leaves the recording from there (k, i, X), and a road command its run at 3028 (an extra object number from it shifts every later settler number by one: carrier 90502 of the road (139, 56) .. (138, 54) on its 20-tick grid one tick early, trace tick 4218) |
| T41 | AI-2.6, AI-CON-9.2, 9.3, 6.3: E-trace-010, player 1, the destruction runs (counters 35 mod 41) | the cursor and the hunter's hut 90244 on (139, 47) | the cursor read from the heap dumps (the AI player's destruction part, last handled id + 1): 90212 in 3500 (90211 handled at 3479), 90181 in 4000 (the castle at 3971, after the reset run 3930), 90539 in 4500 (the farm 90538 at 4463); 4094: the hut is tested, its hunter is out: not a candidate; 4791: the hunter rests inside, no unreserved animal within 8 (the animal 90061 on (142, 39) is reserved by this hut and released at its removal): v = 0, torn down with remains (plank 90644 on its flag (140, 46), remains 90645, measured). Without the reset run the experiment build tests 90351 at 4791; with "worker present or on the way" it tears the hut down at 4094; counting reserved animals keeps it at 4791 |
| T42 | AI-CON-7.2, 5.2, 5.3: E-trace-010, player 1 after the hunter's hut 90244 is torn down (counter 4791) | the construction system's orders 4500 .. 5440 | read from the heap dumps (the AI player's construction part: budget limit 2, counter 2 at about 4516 and 5018; the hunter order holder): a hunter order for cell (9, 3) pending at 4516 (placed (144, 58) at 4574); a hunter order for cell (9, 2) made at game time 458.12 s (counter about 4581; three animals have their home (144, 37) in that cell, and in the experiment build none of them stands on a field the player has explored), taken and failed with no site (the experiment build: taken 4995, no field at 5023); then the needs order of the locksmithery, site (135, 48) at 5082; the hunter order for cell (8, 2) at 5345, site (141, 46) at 5389; the sawmill (136, 45) at 5435 (all measured). Counting the animals standing in explored fields of the cell instead makes no order for (9, 2) and orders (8, 2)'s hunter at 4995, site (139, 47) on the ruin's field at 5038 |
| T43 | AI-CON-9.2: E-trace-010, player 1, the destruction run of counter 5816 (35 mod 41) on the woodcutter's hut 90383 (136, 47), its woodcutter at home; its cell (8, 2) holds the productions wood (90383) and planks (sawmill 90734) and no forester | the candidate test | v = 0 (no tree in reach); one wood production, but no forester in the cell: torn down with remains (plank 90780 on the flag (137, 46), ruin 90781, numbers as recorded), the woodcutter going home from trace tick 5821. With "kept while the only wood producer" the hut stands to 6000 |
| T44 | AI-CON-8.3 b: E-trace-010, player 1's route task with B = the guard house 90300 (138, 57), upgraded from counter 5582 (MIL-UPG), k = 1 after the run of 5803 | the run of 5840 | B unfinished (its upgrade's construction not done) and k = 1: done; cursor 90301, k = 0 (measured: ai_route); then 90322 at 5877 |
| T45 | AI-CON-8.3 b, AI-CON-1.2 (Q-279): E-trace-011, player 1's route task on the harbour 90351 (144, 47), cell (9, 2), finished at trace tick 6459; the castle 90180 (139, 51) in cell (8, 3) | the runs of counters 6765 .. 7061 | 6765 cursor to 90351; 6802: B is a depot (harbour): the neighbour cells 0 (8, 1), 1 (8, 2) have no depot, 2 (8, 3) holds the castle: S = the castle, k = 2, direct variant, w = 1, X = [(145, 46)]; 6839 .. 6950 w = 2 .. 5, X last (145, 46), (145, 46), (141, 45), (141, 45); 6987: the road builder (141, 48) -> (141, 45) (numbers 90887, 90888), k = 3; 7024: no further neighbour cell with a depot: done, cursor 90352; 7061: cursor 90436 (all measured, ai_route). Taking the non-depot branch (k = 0 the castle, then k = 1) gives the same w and X, but the task stays on the harbour to 7246 and the road builder call of 90436 at 7172 (number 90899) comes later |
| T46 | AI-CON-1.2, 8.3 a: E-trace-011, player 1's farm 90538 (136, 58) (type: product grain, planting good grain) | the route task's runs of counters 7838 .. 7949 | 7838 done with 90510, cursor 90511; 7875 cursor 90538 (the farm's record, not skipped); 7912 B = the farm, w = 1, X = [(136, 57)]; 7949 w = 2 (measured, ai_route). Counting the farm as a planting building skips it: cursor 90539 at 7875 |
| T47 | AI-CON-9.5: E-trace-011, player 1's hunter's hut 90625 (144, 58) torn down at counter 7948, its flag 90626 (144, 57) left with one road 90634 to flag 90271, carrier 90641; the hut's plank 90927 lying on 90626 | the flag runs after 7948 | 7959 .. 8083: 90641 holds the plank (picked up at 90626, carried to 90271, put down there at 8084): the flag stays; from 8084 he walks back empty: at the next visit of the flag run, counter 8104, the flag and its road are removed (the ware networks renumbered, netid 90940), and 90641 goes home from trace tick 8106 (measured). Keeping a flag while its road has a carrier at all leaves it standing (experiment build: settlers first differ at 8115, the carrier idle on (143, 55); with the rule 9602) |
| T49 | AI-CON-1.1a, 5.4: E-trace-011, player 1's cell (9, 3) (centre (152, 56)), made by the validity scan with the position (144, 48); its only building the hunter's hut (144, 58) (placed counter 4574, torn down 7948) | the cell's position and the stone pit order of about 10 000 | the position stays (144, 48) in the heap dumps of 7500, 8000 and 9500 (the hut, a hunter, is a single reference and does not move it although (144, 58) is nearer to the centre, 9 against 12); the stone pit's site search from (144, 48) places it at (144, 56) at counter 10028 (measured). With the hut setting the position the routes start at (144, 58) and the site is (144, 58) (experiment build: buildings first differ at 10028; with the rule 10232) |
| T50 | AI-CON-9.5, the cursor: E-trace-011, player 1. Counter 127: the list holds only the castle flag (140, 50), the cursor is 1; the same think places the first site (hunter's hut 90190 on (140, 49), its flag (141, 48)) | the flag runs of 127 and after | 127 tests the new flag (index 1, the list now 2 long), the cursor becomes 2; 128 starts again at the castle flag. The cursor read from the heap dumps (the AI player's destruction part) at trace ticks 500, 1000, ..., 8000: 1, 4, 5, 2, 5, 7, 5, 8, 15, 11, 19, 18, 13, 13, 12, 12; the run of counter 8104 tests (144, 57), index 15 of the 24 flags, and removes it (T47). Leaving the new site's flag out of the run of 127 (or taking the cursor modulo the length) puts every later run one place behind: (144, 57) at 8105, each dump value one tick late (experiment build, 16 of 16 dumps match with the rule) |
| T51 | AI-CON-8.3 b (k >= 2, the order of B's needs): E-trace-011, player 1's route task on the bakery 90650 (136, 49), flag Bf (137, 48); bread's cost list is water, flour; when the bakery was added the water need 90652 was made and the flour need 90495 (older, without a consumer) tied after it: the bakery's list is [90652, 90495] (heap dump 10000) with the producers spring 90809 (flag (135, 44)) and mill 90590 (flag (141, 52)) | the runs of counters 9910, 9947, 9984 (k = 2, direct variant) | the partner is the spring: 9910 w 0, X [(137, 48)]; 9947: field 1 (136, 47), Fn = (137, 48) = X's last: w 2, no road-builder call (no object number taken); 9984: field 2 (136, 46), Fn (137, 46): the road builder is asked (137, 46) -> (137, 48) (ai_plan 9984) and k = 3 (measured, ai_route). In id order the partner is the mill: 9947 takes field (138, 48), Fn (139, 48), asks (139, 48) -> (137, 48) (an extra object number) and goes to k = 3 one run early |
| T52 | AI-CON-1.1c, 7.1: E-trace-011, player 1's cell (8, 3) with the sawmill 90211 and no forester; plank + wood in stock 34 (limit 40); its trees on landmass 12: 41 until tree 76962 (141, 61) is cut down (gone at counter 9479) and tree 77166 (142, 62) (gone at 9969) | the forester runs (one cell per run, every 3 ticks) | the cell-coordinate map holds 41 surface wood for (8, 3) in the heap dumps of 9500 and 10000 (refreshed last before 9479), 39 in 10500 (refreshed about 10320, its cursor between (10, 1) at 10000 and (9, 4) at 10500); the forester order (weight 50) is made at game time 1035.19 s (counter about 10352; heap dump 10500: the forester slot's order and the current order). Counting the trees as they stand (or summing the squares when read) orders it at 9963 and places the forester site on (140, 59) at trace tick 10232 instead of 10442 (an experiment build: with 1.1c's rates the order comes at 10353; buildings first differ 10232 -> 10382, generator 10313 -> 10434) |
| T53 | AI-CON-1.6, `AI-misc.md` AI-MISC-1.1: E-trace-011, player 1 (computer, Weak); its harbour 90351 on (144, 47) in cell (9, 2), finished at counter 6458; shipyard orders (weight 10) on (9, 2) made by the 673-tick part at counters 6724, 7397, 8070 and 8743 (order times 672.38, 739.67, 806.95, 874.23 s in the order slot of the heap dumps of 7000, 7500, 8500 and 9000) | the orders run (AI-CON-5.2, 5.3) | the order of 6724 finds no site (its search ends at 6772): the cell's ship mark is set at 677.18 s, and in the heap dump of 7000 it is already inactive with count 1; so the orders of 7397 and 8070 are made (taken at the 50-tick steps of 7444 and 8194, both fail for lack of material: no mark). The order of 8743 is taken at the step of 8994, its value (now - 874.23 s) x 10 = 251.7 beating the needs order of 874.83 s (245.7), which stays pending; it finds no site at 9022: mark set at 902.13 s with count 1, active in the dumps of 9000 (read up to counter 9053) and 9500, so the part makes no order at 9416. The mark map of the autosave E-sav-005 fits the same rule (inactive marks with count 1 or 2, active ones with count 1). With 600 s for the first mark (an experiment build) no ship order follows 6772, the needs order is taken at 8994 instead, the hunter orders run 50 ticks early and the buildings of the dense trace first differ at 10382 (10719 with the rule) |
| T54 | AI-CON-10.2, 8.3 b: E-trace-011, player 1's locksmithery 90685 on (135, 48) (flag (135, 47)), finished; its needs: plank (produced by the sawmill's production) and iron (no production); its production of tools, whose need (the locksmith part's tools need 90470, in cell (8, 3)) has no consumer building (heap dump 10500) | the route task on 90685 | k = 0 and 1 the castle, k = 2 the plank producer, the sawmill 90734 (passes of 10650 .. 10724), and at the run of counter 10761 k = 3 has no partner: done, the cursor goes on to 90686 (recorded ai_route). With the headquarters as the tools need's consumer (an experiment build) k = 3 is the castle: three more passes, the cursor reaches 90734 three runs early and the road (135, 47) - (137, 46), which the original never builds, is asked for at 11427 (a carrier leaves the castle at 11449; settlers first differ at 11449, the generator at 11453; with the rule 11634 and 11838) |
| T55 | AI-CON-8.3 c (route variant, equal routes): E-trace-011, player 1's route task on the sawmill 90734 (flag Bf (137, 44)) with its k = 2 partner (flag Sf (141, 60)), r = 48; the same 32 roads in the runs of 12426 and 12463; two routes from Sf to Bf of road length 36: over (141, 58), (141, 56), (142, 55), (141, 52), (140, 50), (139, 48), (137, 46) (9 flags) and over (141, 58), (141, 56), (139, 56), (138, 54), (137, 52), (137, 50), (137, 48), (137, 46) (10 flags); at (137, 46) the road to (139, 48) is older than the one to (137, 48) | the runs of 12426 and 12463 with i = 8 | 12426: the castle's flag (140, 50) holds 6 wares (full), the 9-flag route costs more, the list has 10 flags and i = 8 gets its try (recorded ai_route); 12463: it holds 5, the routes tie, the read-out at (137, 46) takes the road to (139, 48): 9 flags, i = 8 is past the second-last and the task goes on to k = 3 (recorded). Taking the other route on the tie (an experiment build) keeps 10 flags and tries once more: settlers first differ at 12614 (the road (135, 47) - (135, 44) of 12611 is not built), the generator at 12615; with the rule 12933 and 12983 |
| T56 | AI-CON-5.4b (Q-281): (a) E-trace-011, player 1's military order of point (128, 32), radius 12 (limit 24), taken at counter 14394; candidates (137, 38) (site value 65) and (137, 37) (value 70.5); (b) the step count at a tie: a found path from the start (120, 56) to p = (123, 61) whose labels run (120, 56) 22, (121, 56) 19, (121, 57) 16, (122, 58) 13, (122, 59) 10; at (122, 59) the neighbours in direction 0, (123, 59), and in direction 5, (123, 60), both carry the lowest lower label 7; (123, 60) neighbours p (label 0), (123, 59) reaches p only over (124, 60) (label 4) (s2-reborn's own Hard game, --ai 1=3, world tick 19206, at 1a2cbd7) | the site search's route length | (a) (137, 38) len 12, score 65 x 12 / 24 = 32.5; (137, 37) len 13, score 70.5 x 11 / 24 = 32.3125: guard house 91373 on (137, 38) at counter 14449 (recorded); a breadth-first length 12 for both would put it on (137, 37). (b) len = 7: at (122, 59) the lowest direction, 0, is taken: (123, 59), (124, 60), (123, 61); the kept direction 5 of the previous steps (the read-out of AI-CON-8.3 c) would give 6 |
| T57 | AI-CON-1.1c, 7.3: E-trace-011, player 1, the cell-coordinate record of (9, 3) (fields x 144 .. 159, y 48 .. 63): stone piles (deposits table) at 2999: (144, 53) stone_05, (144, 54) stone_04, (144, 49) stone_04; at 9999: stone_05, stone_01, stone_04; at 16999: (144, 49) stone_04 only | the stone (kind 2, landmass 12) and wood amounts in the heap dumps, and the 7.3 run of counter 17378 on cell (9, 3) (one stone producer, no wood producer) | stone 13 at 3000 (5 + 4 + 4) and 13 at 10000 (the lagging record), 6 at 17000 and 17500; wood 11 at 17000. The run orders a stone pit (5 x 1 < 6), placed on (144, 58) at counter 17684 (site 91662, production 91663). Counting 1 per pile (an experiment build) gives 3, 3, 2: 5 x 1 < 2 fails and a woodcutter is ordered and placed on (145, 57) (buildings first differ at 17685); with the rule the buildings of the dense 20 000 pair equal the recording to the end, generator 18121 -> 18509 |
| T58 | AI-CON-8.2: E-trace-012, player 1 (Hard: street optimizing on); its roads in creation order: 90201 (until counter 386), then 90226 "139,48 139,49 140,50" and 90230 "139,48 140,48 141,48" (386), 90254 "140,46 140,47 141,48" and 90260 "140,50 141,50 141,49 141,48" (by 664), 90279 "142,55 142,54 141,53 141,52 140,51 140,50" (the woodcutter site of counter 818) | the runs of 195, 262, 329 (one road), 396, 463, 530, 597 (two), 664, 731, 798 (four), 865 (five) | cursor read / road: 0/90201, 1->0/90201, 1->0/90201, 1/90230, 2->0/90226, 1/90230, 2->0/90226, 1/90230, 2/90254, 3/90260, 4/90279: at 865 the flag goes on (141, 53) of road 90279 (flag 90286, its halves 90290 and 90293 in the objects snapshot of 869, recorded). Wrapping the cursor modulo the list's length after each run (an experiment build) reaches 90279 only at 932: settlers first differ at 1003, generator 1520, buildings 1610; with the rule 2330, 4410, 4190 |
| T59 | AI-CON-3.3, 1.1a: E-trace-012 (player 1, Hard), heap dumps 5000 .. 6500 (the AI core's cells), objects table | the flour need 90715 (made at counter 4669 with the bread need 90714) and its cell | cells and positions at 5000: (8, 3) (136, 57), (8, 2) (137, 44) (the farm 90720's field), (9, 2) (144, 47), (9, 3) (144, 48); the only grain producer is the farm in (8, 2). The need has cell (8, 2) at 5000; the mill finds no site there (full mark for flour on (8, 2) set at 542.4 s); re-arranged, (8, 2) marked: distances from the grain cell's position (137, 44): (8, 3) 13, (9, 2) 9, (9, 3) 9 ((9, 2) made earlier, so it wins the tie); the need has cell (9, 2) at 5500, (9, 3) at 6000 and (8, 3) at 6500, the mill placed on (136, 59) at 6639 (production 90962). With centre fields (16 cx + 8, 16 cy + 8) (8, 3) and (9, 2) tie at 16 and the first in the list, (8, 3), wins: the mill on (135, 58) at 5592. Experiment build: Hard pair buildings 5593 -> 6031, generator 5665 -> 5685; the other pairs unchanged (the combination rule changes no pair: no cost here mixed a part of 100000 with a finite one) |
| T60 | AI-CON-8.1f, 8.1d: E-trace-012 (player 1, Hard), `ai_street` and `ai_plan` tables, flags table | the street runs around counter 5629 | run 5629: F the flag (145, 56) at position 23 (the last), T (142, 55) remembered; flag 90467 (137, 46), earlier in the list, is removed between counters 5633 and 5634, so the list is one shorter; run 5636: the cursor lies past the end, the run restarts with the castle's flag (140, 50) (no usable T: give-ups 1, excluded 2), and runs go on through the list; (145, 56) comes back at position 22 in 5895, attempts 1 to 5 at 5902 .. 5930 and the road (145, 56) -> (142, 55) is built at 5937 (`ai_plan`: "145,56 144,55 143,55 142,55"). Keeping the worked flag by identity instead builds that road at 5671 and lets the hunter's hut site on (144, 57) start its frame at 6031 instead of 6588. Experiment build: Hard pair settlers 5679 -> 6167, buildings 6031 -> 6876, generator 5685 -> 6259; the other pairs unchanged |
| T61 | AI-CON-8.1e (Q-368): E-trace-012 (player 1, Hard), street work on F (135, 57) (flag 90900), T (137, 56); R = road 90803 "135,55 136,56 137,56", its carrier 90807 standing on (136, 56) from before counter 6340 to 6378 (E-trace-013, every tick) | the attempts' lengths and the execution | `ai_street`: attempt 1 L 4, best 1 (walk path F -> T, 4 steps around R); attempt 3 changes nothing (T -> P cannot pass (136, 56)); attempt 4 L 3, L2 4, best 4 (the road-stop path F, (136, 56), T is 3 fields; F -> P 4 steps over (134, 57), (134, 56), (134, 55)); at 6378 R is removed and the roads (135, 57)-(136, 56)-(137, 56) and (135, 57)-(134, 57)-(134, 56)-(134, 55)-(135, 55) are built (`ai_plan`). Walking over the occupied field gives T -> P 2 (strategy 3) and a refused second road. Experiment build (s2-reborn 0cf1665 + 8.1f + this rule): Hard pair settlers 6419 -> 7054, buildings 6876 -> 7680, generator 6462 -> 7435; counting walking carriers too breaks the Weak pair at 5503; blocking R's fields always breaks every pair early |
| T62 | AI-CON-8.2 (list order): E-trace-012 (player 1, Hard), the street optimizer's run of counter 7364 (195 + 67 x 107), the player's 34 roads in object-number order (90226, 90230, 90260, ..., 90994, 91001, 91012, 91043, 91046; the last two are the halves of a split made after 91012) | the road handled and its flag | cursor 31 -> road 91012 "141,60 141,59 142,58 141,57 141,56" (made at 7355): a flag on it, the road split, the new half's carrier request (object 91083 at 7365), carrier 91020 walking to his new idle field at progress 0 in 7365. A storage order that put the split's halves into freed places makes index 31 the road "137,58 137,57 137,56" (no field takes a flag), no split. Experiment build: Hard pair settlers (view-free) 7367 -> 7592, generator 7435 -> 7668; the other pairs unchanged |
| T63 | AI-CON-8.3 c (when the costs are asked), TRN-CARRY-5.3a: E-trace-012 (player 1, Hard), the route task on the harbour 90366 (k 2, partner the castle), try t 2 at counter 7616 with Fn = the flag (142, 47) (`ai_route`: X (143, 45), (142, 47)), the store emptied last at 7529 (objects table: k1 renumbering) | the stored pair (142, 47) -> castle (140, 50) and the storage of water 91124 | the try's walk path does not count, so no cost is asked; the pair is first asked by the storage run of 7780 for water 91124 lying at (142, 47), with the castle's flag full: 28 (4L into it), the harbour (145, 46) 25 (heap dump 8000: "(142, 47) -> (140, 50) 28", "(142, 47) -> (145, 46) 25"): the water goes to the harbour, next flag (141, 48), then (140, 46), where carrier 90403 takes it at 7917. Asking cost(Fn -> Sf) before the walk-path test stores 14 at 7616 (the castle's flag not full then) and sends the water to the castle. Experiment build: Hard pair view-free settlers 7919 -> 8810 (generator 8866 -> 8843: the next point, a builder whose site is removed at 8810, comes into view first) |
| T64 | AI-CON-8.1e (a standing settler off the road): E-trace-013 / E-trace-012 (player 1, Hard), street work on F (132, 58) (flag 91287, the donkey farm 91283's) with T (135, 57); R = road 91043 "135,57 134,57 134,56", P (134, 56) the flag of woodcutter hut 91097 on (133, 57); the woodcutter 91251 stands on (132, 57) felling tree 76066 (a timed action, progress -1) at counter 9633 | attempts 3 to 6 | `ai_street`: attempts 1 (9612) L 6 best 1, 3 (9626) and 4 (9633) change nothing; at 9647 (attempt 6) the road F -> T over (133, 58), (134, 58), (134, 59), (135, 59), (136, 58) is planned (`ai_plan`, cost 37) with no draw. Our build: attempt 4's second leg F -> P (132, 57), (133, 56), (134, 56) = 3 steps makes strategy 4 best (L 5, L2 3) and its execution draws the road-stop read-out again at 9647. Experiment build (a standing settler blocks every walk path field but a; not the road-stop search; the settlers counted: road carriers and donkeys and the settlers out in the world, each not in a step; a carrier that has retired into a warehouse is none, though its last field stays on record: counting it blocked (143, 44) at 9661 and broke the next street work): Hard pair settlers 9653 -> 9781, buildings 9780 unchanged, generator 9648 -> 9784, the other pairs unchanged; blocking in the road-stop search too breaks the Hard pair at 630 |
| T65 | AI-CON-3.4 (the square test): E-trace-013 / E-trace-012 (player 1, Hard), cell (8, 3); the needs' squares in the heap dump of counter 9000 | each resource need's square | the stone need 90188 (35, 12); the wood needs 90215 (33, 12), 90438 (33, 13), 90519 (33, 14) (arranged at counters 332, 1853, 2542 in the experiment build, equal to the recording there: each the first free square in sx, sy order), 90955 (32, 14) (6598); coal need 90600 (32, 13) at 7248, no production until its mine site at 9133; the second coal need 90973 (no consumer) at 7261 finds no square and waits through 11 000 at least. Our build shares squares (all three wood needs on (33, 13), both coal needs on (32, 14)), takes the largest surface amount and the corners at +3, so need 90973's twin gets a coal mine at counter 9779 (132, 56). Experiment build with the slot taken at the arrangement, surface first, corners at +4 clamped: every square above equal; together with T66 Hard pair settlers 9781 -> 10112, buildings 9780 -> 10175, generator 9784 -> 10466, the other pairs unchanged (T65 without T66: buildings 9629, the forester of T66 comes first) |
| T66 | AI-CON-7.1 (the plank producer): as T65, cell (8, 4) | the forester order | the needs order for a sawmill in cell (8, 4) is filed at game time 932.6 s (the needs slot at the dump of 9500); no forester order follows (the forester slot keeps its order of 648.6 s through 10 000). Our build makes the order's production at once and orders a forester for (8, 4) at 932.7 s, taken at counter 9544. Experiment build (only placed producers count): with T65 as above |
| T67 | AI-CON-4.2 (avgProd): as T65, the needs construction run at counter 10024 (after the spring order for cell (8, 4) failed at 10022) | the good chosen | water 63.7, plank (D 4, A 3, priority 300, stock 17, productivity 1.0) 83, corn (D 3, A 1, priority 70, stock 0) 2 x 70 x 0.99 x avgProd: the farm 90720 reads 0.4 in the original (T26 of ECO-outdoor-workers), 55.4, so plank wins and the sawmill order for cell (8, 2) fails at 10171 (plank full mark at 1017.1 s, heap 10 500); our build read 0.7 (its sowings counted, its first one setting 0.5), 97.0, and placed a farm on (143, 59) at 10174. Experiment build (ECO-OUT-3.11 as corrected): Hard pair buildings 10175 -> 10278, settlers 10291 -> 10492, generator 10466 -> 10494, the other pairs unchanged |
| T68 | AI-CON-8.1e (a tree on the way): E-trace-014 (R4, the Weak game), street work on F (130, 54) (the flag of the coal mine site 92002 on (129, 55), placed at counter 22087) with T (132, 58); trees on (131, 56), (131, 57), (132, 57), (133, 57) (DepositBlocked; heap dump 22 000), road fields on (129 .. 133, 58) | attempts 1 to 6 | `ai_street`: attempt 1 (22177) finds no walk path F -> T (T is reached only over those fields); attempt 2 (22184) L 4 best 2: the waypoint (130, 58) on the road 91490 at T over (130, 55), (130, 56), (129, 57); at 22212 the road F -> (130, 58) is planned and built (`ai_plan`; roads 92014, 92026 and the site's builder sent from the castle at 22213). Our build walked through the trees (F, (130, 55), (131, 56), (131, 57), T, L 4 best 1) and lost T at attempt 6. Experiment build (DepositBlocked fields block the walk paths): the Weak game against R4 equal to settlers 22493, buildings 22933, generator 22604 (was 22214 / 22730 / 22219), the 150 000 pair's generator 22219 -> 22604; the Hard pair the same with or without the standing-settler rule of T64 |
| T69 | AI-CON-8.3 b, AI-2.1a (an upgrade ordered in the same think): E-trace-014 (R4, the Weak game), player 1's guard house 91705 (132, 31), finished at counter 22242 (buildings table), the route task's B since before 22301 (warehouse partners while unfinished), k = 1 after the run of 22490 (ai_route); in 22527 the upgrade run (AI-MIL-4.5: level 1, upgrade level 2, twice the costs in stock) takes it | the route run of 22527 | B unfinished (the heap dump 22499 .. 22558 holds its upgrade's construction, both parts 1.0) and k = 1: done, cursor 91706, then 91939 (the hunter's hut (134, 31)) from 22564, k = 1 after 22601, which builds the road (133, 30) .. (135, 30) (measured, ai_route). Reading B as finished in 22527 (the upgrade not yet in effect): k = 1 gives the castle (139, 51) (distance 20; the harbour (144, 47) also 20, later in the list), the task stays on 91705 to 22564 and reaches the hut one run late (22638): an experiment build differs from 22604 (settlers, generator); with the rule it equals R4 to 23023 (settlers) and 23024 (generator), the Hard pair and the Weak pair to 20 000 unchanged |
| T70 | AI-CON-3.4, 3.3 (free productions, the consumer distance): E-trace-014 (R4, the Weak game), player 1's fish need 92083 (the food link of the iron mine's food need), made at counter 22432, arranged at 22458; cell positions (9, 2) (144, 39), (9, 3) (144, 56), (8, 3) (137, 55), (8, 2) (137, 38); fish is looked up as food, whose consumers are the coal mine's site 92002 (129, 55) in (8, 3) (production 92003, its need 90484 for the smelter 90481: not free) and the iron mine's site 92075 (130, 36) in (8, 2) (production 92076, its need 90490 without consumer: free); (8, 3) holds one food-group need (bread 90494), (8, 2) none | the fish need's cell | (9, 2): coal 16 x 2, iron ore 8 / 2 = 4 -> 4; (9, 3): coal 7 x 2 = 14, iron ore 18 / 2 = 9 -> 9; (9, 2) wins with the square (36, 11) (fish 28; heap dump 22500: cell 90197, res (36, 11)); its fisher order of counter 22740 finds no site (full mark fish (9, 2) at 2292.085 s), the need goes to (9, 3), square (36, 12), order at 2292.586 s, the fisher placed on (145, 57) at 23032 (T72). With the old reading ((9, 2) 8, (9, 3) 7) the need takes (9, 3) at once and the fisher is placed at 22932: an experiment build moves the Weak game's first difference vs R4 from 22933 to 23033 (buildings), 23023 -> 23232 (settlers), 23024 -> 23319 (generator) |
| T71 | AI-CON-3.3 (Q-370): E-trace-013 (R3, the Hard game), player 1's water need 91055 for the bakery 91053 (138, 59), re-arranged at counter 10030 after its spring order for (8, 4) failed at 10022; water's consumers are the brewery 90786 (production 90787, need 90560 for the castle: not free) and the bakery (production 91054, need 90714 without consumer: free), both in (8, 3) (136, 57), which holds one water need (90563); (8, 2) holds the water need 91286 (for 91283); cell positions (8, 2) (137, 44), (9, 2) (144, 47), (9, 3) (144, 48), (9, 4) (144, 65) (heap dump 10000) | the need's cell | distances 13, 13, 12, 12, halved by the bakery: 6 x (1 + 1) = 12 for all four; (8, 2) adds 50 x 1: 62; (9, 2) is the first of the three at 12 in the cell list and gets the need (heap dump 10000: need 91055 in cell 90199), its spring order fails there (full mark water (9, 2) at 1027.096 s; experiment: no site at 10272). With the old reading ((9, 3) and (9, 4) at 12, (8, 2) and (9, 2) at 13) the need took (9, 3); with the halving alone (not the needs count) (8, 2). Experiment build: the Hard pair's first difference 10278 -> 11393 (buildings), 10492 -> 10838 (settlers), 10494 -> 11202 (generator) |
| T72 | AI-CON-6.2 (a fields good): E-trace-014 (R4), the fisher order for (9, 3) (2292.586 s), routes from the cell position (144, 56), working radius 8, every fish field holding 7, a small building on Medium ground (x 0.5) | the site | (144, 54): 13 fish fields within 8, v = 91 x 0.5 = 45.5, route 2, score 39.8; (145, 57): 20 fish fields (8 of them unexplored), v = 140 x 0.5 = 70, route 2, score 61.25: the best (the heap dump 22999 .. 23057 holds the search's best field (145, 57) with score 61.25); the fisher is placed on (145, 57) at counter 23032 (measured). Counting the explored fields (6.5 and 6) takes (144, 54). Experiment build: the Weak game vs R4 buildings 23033 -> 24438, generator 23319 -> 23493 |
| T73 | AI-CON-10.1 (the order): E-trace-014 (R4, the Weak game), player 1's production control, 24 qualifying goods, one per 67-tick run, a cycle of 1608 ticks; the springs 90510 (136, 53) and 90809 (134, 45) | the run that visits water | in signed id order water is the 4th qualifying good (after iron 8198bf23, bread 90ab8923, a8483903): the runs of 61, 128, 195 take the first three, water from 262 and every 1608 ticks; at counter 22774 water counts 43 + 3 = 46, the limit is 45: both springs are switched off (buildings table, enabled 1 -> 0 at 22774; on again 25990, off 32422, on 34030 ...). In unsigned order water is the 16th (runs at 1066 + 1608 k) and the run of 22774 takes the donkey: the springs stay on (an experiment build; the spring's worker starts a cycle at 22887 that the original's does not: settlers first differ at 23232, with the rule at 23461, buildings 24438 -> 24978; the Hard pair and the Weak pairs to 20 000 unchanged) |
| T74 | AI-CON-8.1e (the second leg avoids the road-stop path): E-trace-014 (R4, the Weak game), street work on F (132, 25) (the flag of the stone pit's site 92177 on (132, 26)) with T (135, 30); the road-stop search of attempts 3 and 4 finds the path (133, 26), (133, 27), (134, 28), (134, 29), T, hitting R (133, 30) - (133, 29) - (134, 29) - (135, 30) at (133, 29), P = (133, 30) | the second legs | `ai_walk`: attempt 3 (counter 23367) T -> P 13 steps around the north (135, 29), (135, 28), (134, 27), (134, 26), (133, 25), (133, 24), (132, 24), (131, 25), (131, 26), (131, 27), (132, 28), (132, 29), P; attempt 4 (23374) F -> P 6 steps over (131, 25), (131, 26), (131, 27), (132, 28), (132, 29) (`ai_street`: L2 6), where the free fields (133, 26), (133, 27), (133, 28), (132, 29) give 5. An experiment build with the rule gives L2 6 at 23374; with it alone the pairs do not move, since attempt 5 decides there (T75) |
| T75 | AI-CON-8.1 a (strategy 5): as T74, attempt 5 at counter 23381; and every `ai_street` table recorded (E-trace-011, E-trace-012, E-trace-013 to 150 000, E-trace-014) | the best strategy | 23381: no waypoint, best stays 4 (L 6, L2 6); at 23388 R is removed and the roads F -> T and F -> P (133, 30) are built (`ai_plan`); the recordings hold best 1, 2, 3 and 4 (E-trace-013: 10093, 118, 127, 175 rows) and never 5. Our build found w (134, 28) (P -> w 2 steps over R's field (133, 29)), placed a flag there and built three roads, one more carrier left the castle before the stone pit's builder. Experiment build (T74 and strategy 5 never counting): the Weak game vs R4 settlers 23461 -> 23591, generator 23493 -> 24256 (the 150 000 pair's too); the Hard pair and the Weak pairs to 20 000 unchanged |
| T76 | AI-CON-1.8 (which fields count), AI-CON-10.1: E-trace-014 (R4, the Weak game), player 1 after the guard house (132, 31) is manned (soldier 92053 in at counter 22921); the heap dump 23999 .. 24050: 991 fields owned (the territory map), 54 of them water (region 0), 98 without a continent (-1), 828 on continent 12, 11 on small ones | S from the counts of 23439 and 24040, and production control's water run of 24382 (water 43 + 2 = 45) | S = 991 - 54 - 98 = 839, the limit 40 + 839 x 60 / 9216 = 45: the springs stay off (enabled 0 from 22774 to 25990, buildings table). Counting only the water out (S = 937, limit 46) switches them on at 24382 and the spring's worker works from then (an experiment build: the Weak pair's settlers first differ at 24729; with the rule at 24967, buildings 25702 -> 26278, generator 25152 -> 25204, the 150 000 pair's too; the Hard pair and the Weak pairs to 20 000 unchanged) |
| T77 | AI-CON-3.3 (the consumer distance counts ware-group consumers, Q-371): E-trace-015 / E-trace-013 (the Hard game), player 1's plank need 90898 (for 90896) loses its cell (8, 2) when the sawmill order there finds no site at counter 10171 (full mark planks on (8, 2) at 1017.099 s); cell positions (8, 2) (137, 44), (8, 3) (136, 57), (8, 4) (137, 64), (9, 2) (144, 47), (9, 3) (144, 48), (9, 4) (144, 65); wood suppliers: the free woodcutter production 91311 (no need) in (8, 2) and the woodcutters of (8, 3) (needs with consumers); the only plank consumer is the tools production 90897 in (8, 3) (the metalworks; its need 90564 has no consumer: free); (8, 3) holds the plank needs 90189, 90190, 90191 | the need's cell | supplier part S: (9, 2) 9 / 2 = 4, (9, 3) 4, (8, 4) 7, (9, 4) 10, (8, 3) 50 x 3 + 0 = 150; consumer part C = (distance to (8, 3)) / 2 x (3 + 1): (9, 2) 6 x 4 = 24, (9, 3) 24, (8, 4) 3 x 4 = 12, (9, 4) 24, (8, 3) 0; so (8, 4) with 7 + 12 = 19 (against 28, 28, 34, 150; (8, 2) refused by its mark). Measured: the need is in (8, 2) in the heap dump of counters 10159 .. 10162 and in (8, 4) (cell 90793) in 10169 .. 10174 (window dumps every 10 ticks); its later moves ((8, 4) fails at 10970, then (9, 3), where the sawmill 91546 is placed, trace tick 11526) follow in the experiment build. Skipping the ware groups (no consumer, S only) gives (9, 2) at 4. Experiment build: the Hard pair's buildings 11526 -> 14327, generator 11824 -> 14064, settlers without the animation column to 12691; R3 settlers 11820 -> 12700, buildings 11530 -> 14440, generator 11824 -> 14064; the Weak pairs and the one-player pair unchanged |
| T78 | AI-CON-4.2 (sites count in avgProd): E-trace-013 (R3, the Hard game), player 1 at the needs construction run that files the order (order time 1547.87 game seconds; tick 15481 in the experiment build). Corn and water each have D 3 and A 2 (cell (8, 2): the corn need for the consumer at (131, 59) without a production, and its water need likewise). Stocks are corn 4 and water 10, priority 70 each. The farms are (137, 44), finished at 1.0, and the site (138, 38), placed at 14983 and still levelling; the water mean is 0.975 | the order | water 30.71 against corn 0.96 x 70 x 0.5 x 0.75 = 25.20: a spring order for cell (8, 2), its site placed on (142, 37) at counter 15539 (recorded: heap dump 15500, the needs order slot executing type spring, cell (8, 2), order time 1547.87 s; buildings table). Without the site in the mean, corn scores 33.6 and the farm goes on (141, 37) at 15525: the Hard game against R3 differs from buildings 15526. With the rule, buildings 17030, generator 15787, R3 settlers 15810 (experiment build; the Weak pairs unchanged) |
| T79 | AI-CON-8.1e (a worker walking back from his spot): E-trace-013 (R3, the Hard game), player 1's street work on F (143, 36) (flag 92046, the spring site's) with T (140, 39). The hunter 91949 is on his walk back from his spot to his field (142, 36) (trace px 568639, 142360 in tick 15751, walking) | attempt 1 at counter 15751, and what is built | the walk (143, 36) (142, 36) (141, 37) (141, 38) (140, 39), 4 steps, through the hunter's field; attempts 3 to 5 do not beat it (5 fields); at 15786 the road of strategy 1 is built with those fields (recorded: ai_street best length 4, strategy 1; ai_walk 15751; ai_plan and streets 15786/15787). Treating the hunter as standing on (142, 36) gives a 6-step walk, strategy 3 wins, and road 92009 is torn down under its carrier: the Hard game against R3 differs from settlers 15792. With the rule: buildings 17277, generator 17172, R3 settlers 17170 (experiment build; the Weak pairs unchanged) |
| T80 | AI-CON-8.1e (a donkey's field): E-trace-014 (R4, the Weak game), player 1's street work on F (141, 54) (flag 93354) with T (142, 55); R = road 90601 "142,55 142,54 141,53 141,52" (P (141, 52)). Its carrier 90282 (standing since about counter 32838) and its donkey 91112 (since about 32941) both stand on R's idle field (141, 53) | attempt 4 at counter 32971, and what is built | the second leg F -> P is the walk (141, 54) (141, 53) (141, 52), 2 steps, through the idle field: strategy 4 with lengths 3 / 2. At 32985 R is removed and the roads (141, 54) - (141, 52) and F - T are built (recorded: ai_street, ai_walk 32971, ai_plan 32985, objects 32985 / 32986). Blocking (141, 53) for the standing carrier finds no second leg and builds nothing: the Weak game against R4 differs from generator 32990 (settlers 33000). With the donkey's field free: settlers 33550, buildings 33850, generator 33650 (experiment build; Hard unchanged) |
| T81 | AI-CON-6.2 (the flood count): E-trace-013 (R3, the Hard game), player 1's woodcutter order for cell (9, 3), taken at counter 18544. The two best fields are (145, 59) and (146, 60). Within radius 6, (145, 59) has 10 trees and (146, 60) has 11; the extra one is the fir 78028 on (145, 66), at distance exactly 6 from (146, 60) and not reached by the flood | the site | (145, 59) at 18575 (recorded). The flood gives 10 and 10 growing trees (reserved ones included; after the border factor 0.4 and the size factor 0.5: 2.0 each), and the tie goes to (145, 59), first in the search. Counting by radius alone gives 2.0 against 2.2 and the site (146, 60), which differs from the Hard game against R3 at buildings 18575. With the rule: buildings 19651, generator 19702, R3 settlers 18710, R3 buildings 19660 (experiment build; the Weak pairs unchanged) |
| T82 | AI-CON-1.1 (the corner fields): E-trace-013 (R3, the Hard game), player 1's cell (7, 2) (fields 112 .. 127 x 32 .. 47), checked by the grid scan at counter 18294 | the cell | made at 18294 with position (127, 33) (recorded: objects table, cell 92432). None of (112, 32), (127, 32), (112, 47) and (127, 47) is the player's; one of the fields 16 on, (128, 32), (112, 48) or (128, 48), is. With the square's own corners the cell is not made, the object counter falls one behind from 18294, and the donkeys' numbers shift (R3 settlers first differ at 18710). With the rule: R3 settlers 19510. The Hard buildings then differ earlier, at 19325 (the hunter site (146, 42) comes 150 ticks late: Q-376), against 19651 before (experiment build) |
| T83 | AI-2.4a, AI-CON-7.2 (Q-376): E-trace-013 (R3, the Hard game) with T82, player 1, ten cells in the order (8, 3), (8, 2), (9, 2), (9, 3), (8, 4), (9, 4), (7, 3), (7, 4), (8, 1), (7, 2) from counter 18294; the hunter order for (8, 2) ends with its site (132, 34) placed in counter 18291 (a hunter run tick: (18291 + 6) mod 3 = 0) | the hunter source's next orders | the run of 18291 still finds the (8, 2) order; the runs from 18294 look at (9, 2) .. (9, 4), (7, 3) (has a hunter) and order (7, 4) in 18309 (game clock 1830.6 s in the original's slot, heap dump 18500); later orders (7, 2) at 1897.18 s (dump 19000, slot already empty, index 1); after the respawn of the animal 92625 with home (144, 37) in counter 19249 the run on (9, 2) orders its hunter, site (146, 42) placed in 19325. Running the executing step before the hunter run (the run of 18291 sees the free slot) orders (7, 4) in 18306, is one 50-tick step early from 18844 on ((7, 2) at 1892.19 s) and looks at (9, 2) at 19230, before the respawn: the hunter site comes in 19475. Experiment build with T82 and the rule: the Hard 20 000 pair equal in buildings and generator to its end (19325 / 19469 without the rule), R3 settlers 19510 -> 20080, buildings 19330 -> 22280, generator 19469 -> 22107; the Weak pairs unchanged. (The experiment build's game clock reads 0.1 s above the original's slot times on every source, military and depot orders too: a constant offset of the clock read-out, not a phase) |
| T84 | AI-CON-9.3, 3.4 (a need whose producer is torn down): E-trace-013 (R3, the Hard game), player 1's wood need 90519 (for the sawmill 90517), linked to the production 90987 of the woodcutter 90986 on (140, 61) in cell (8, 3), square (33, 14); the woodcutter is torn down at counter 20207 (production 90987 gone); cell (8, 2) holds the free wood production 91311 (woodcutter 91310, since 9628) | the need in the heap dumps, and the iron ore needs after it | dump 20000: 90519 cell (8, 3), square (33, 14), production 90987; dump 20500: cell (8, 2), square (32, 8), production 91311. The first iron ore need 90606 then gets square (32, 9) in (8, 2) (dump 21000), so the second, 92943 (for the smeltery site 92941 on (133, 32), made 21931), finds no free square in (8, 2) and goes to cell (7, 2) (dump 22000: the needs order slot holds an iron mine for (7, 2)); the mine site 93024 is placed on (127, 35) in 22275. Keeping the need's cell and square (the experiment build without the rule) puts the first iron ore need on (32, 8) and the second in (8, 2): its mine comes on (131, 36) in 22282; the R3 buildings first differ at 22280; with the rule at 22930 (settlers 22380 -> 23130, generator 22277 -> 23129; every other pair unchanged) |
| T85 | AI-CON-3.3, 3.4 (the consumer factor's count): E-trace-013 (R3, the Hard game), player 1's fish need 93032, made at counter 22276 by the food link of the iron mine 93024's food need (AI-2.7 a); cells with a fish square: (9, 2), (9, 3), (8, 4); food consumers: the coal mine (130, 56) in (8, 3) (that cell also holds the bread need of the food link of 22263's coal mine), the iron mines (129, 36) in (8, 2) and (127, 35) in (7, 2), none of them in a cell with another food-group need | the cell of the fish need | (9, 2) (the objects table: cell 90199 at creation; the fisherman's hut site 93115 on (147, 41) placed at 22924). Distances from the cells' positions to the coal mine's cell: (9, 2) 17, (9, 3) 9, (8, 4) 7, to the iron mines' cells (9, 2) 11 and 19, (9, 3) 21 and 29, (8, 4) 25 and 29; with the bread need counted the coal mine's terms double (34, 18, 14): costs (9, 2) 11, (9, 3) 18, (8, 4) 14. Counting only fish and food needs gives (8, 4) 7: its fisher order finds no site (22922) and the R3 buildings first differ at 22930; with the rule at 25800 (generator 23129 -> 25302, settlers 23130 -> 23360; every other pair unchanged) |
| T86 | AI-CON-2.4 (the divisor counts needs in cells): E-trace-013 (R3, the Hard game), player 1's third stock check at counter 24011 (t = 2401.19 s > 600 x 4): S = 883, stock(stone) = 39, stock(plank) = 23; three stone productions (90339, 90570, 91442) but one stone need in a cell; five plank productions and five plank needs in cells (values from our build, whose tables equal R3's to this counter) | the needs made by the check | one stone need for the castle 90180 (objects table: need 93251, counter 24011) and no plank need: trunc(883 x 0.0625) / 1 = 55 > 39, trunc(883 x 0.03125) / 5 = 5 < 23. Dividing by the productions (55 / 3 = 18) makes no stone need and shifts every later number by one: R3 settlers first differ at 24640 (carrier 93262 vs 93263); with the rule at 24720 (experiment build), buildings 25800 and generator 25302 unchanged, every other pair unchanged. The earlier checks agree either way: 6010 none (S 330), 12026 a plank need (S 454, plank stock 2, 14 / 4 = 3) |
| T87 | AI-CON-10.3 (the 1.0 weight): E-trace-013 (R3, the Hard game), player 1; stock(iron ore) 0; the smeltery 92840's (134, 52) iron ore need with a placed iron mine production, finished, worker in, no iron ore ever delivered | the food table and the fish 93771's destination | heap dumps (food table of player 1): from about counter 24428 (the run of 24428; at 24500 the iron accumulator is 9.5 ahead of the stone mine's) to 28000 stone 0.5, coal 0.3, **iron 1.0**, gold 0.5; at the matching of 27423 the free fish 93771 goes to the iron mine 92808 (129, 36) (wares table), and the donkey 93852 of the road (142, 43) - (141, 41) claims it at 28008. With iron at 0.5 (our build) the pick gives the coal mine 91254 (130, 56): the Hard pair's settlers differed from 28010; with the rule (experiment build) settlers 30880, buildings 32190 (was 29030), generator 31949 (was 28946) |
| T88 | AI-CON-3.3 (supplier distance), 7.1 (a needs order makes no production): E-trace-013 (R3, the Hard game), player 1 (its AI core). The bakery 94114 is placed in cell (9, 3) and its flour need (94117) is arranged in counter 29647, while the farm order for the corn need 93549 in (9, 3) waits in the needs slot (filed 2963.04 s, taken at 30044, no site) | the flour need's cell | (7, 3): heap dump 30000 has 94117 in cell 92155 = (7, 3), and 93549 still without a production (production -1, as at 29500). Costs with only placed productions: (9, 3) 21 (supplier 21, consumer 0), (7, 2) 16, (9, 4) 30, (7, 3) 12 (supplier 0, the placed farm 92205's production), the lowest. Counting a production for the ordered farm in (9, 3) gives (9, 3) supplier 0, cost 0, and the flour need there. The mill orders then fail in other cells (full marks for flour: R3 (7, 3) at 3042.918 with count 1, (7, 1) 3077.852, (7, 2) 3092.866) and the first mill is placed on (146, 34) at 33629 instead of (138, 31) at 33669. Experiment build (the ordered, unplaced productions left out of the supplier distance): Hard buildings 33630 -> 34130; settlers 34310 and generator 34133 unchanged, every other pair unchanged |
| T89 | AI-CON-3.4 (stone, both kinds): E-trace-013 (R3, the Hard game), player 1; heap dump 34000 (player 1's AI core) | the stone needs and the order | both stone needs (90188, 93251, consumer the castle 90180) are in cell (8, 3) with underground squares (32, 12) and (34, 15); the needs order slot holds a stone mine (6222a09e) for (8, 3), filed at 3368.035 s; the mine's site appears on (132, 53) at counter 34129. Searching surface squares in every cell first gives the castle's stone need the only surface square, (33, 5) in cell (8, 1) (counter 33209), and a stone pit order for (8, 3), which finds no site (34128). Experiment build (one scan per cell, cells compared by cost whatever the kind): Hard settlers 34310 -> 34380, buildings 34130 -> 36490, generator 34133 -> 34385; every other pair unchanged |
| T90 | AI-CON-6.2 (a sapling counts), 9.2: E-trace-018 (R5, the Weak game), player 1's woodcutter's hut 93352 on (140, 55) (placed counter 32731), cell (8, 3): forester reference set (heap dumps 40000 and 40500: 91116), 16 productions, 5 of them wood | the destruction test of the hut at counter 40297 (35 mod 41; the experiment build's timing), worker at home | within the working radius 6 the only wood deposit is the sapling 94419 on (138, 61), distance 6, made by the forester of (140, 59) at counter 40193 and not growing yet (besides the grain field (137, 60)): v = 1, the hut is kept; it stands to 60 000 (measured). Leaving the sapling out gives v = 0, and with 5 wood productions (not 1) the hut is torn down at 40297; the forester of (144, 60) then finds its field free in the spot search of counter 40935 (11 candidates instead of 10) and plants on (145, 54) instead of the original's (138, 59), and the Weak game's generator first differs at trace tick 40959; with the sapling counted at 41399 (experiment build; every other pair unchanged) |
| T91 | AI-CON-1.7 (stock2), 10.1: E-trace-018 (R5, the Weak game), player 1's production control run on water (a9676263) at counter 40462 (every 1608 ticks, T73); the springs 90510 (136, 53), 90809 (134, 45), 94201 (146, 62) | the run | stock(water) 45 (the castle's store), stock2(water) 3: the ware 94400 lying at the flag 90737, and 94457 and 94460 carried from the castle's door to its flag by its helpers 94456 and 94459 (94460 taken out at counter 40458): 48, at the limit 48: all three springs are switched off (buildings table: enabled 1 -> 0 at counter 40462, on again at 42070). Counting only lying wares and road carriers' wares gives 46 < 48: the springs stay on, and the spring 90809's worker carries another water out at trace tick 41024 where the original's stands: settlers first differ at 41025 (experiment build); with the rule settlers are equal to 42023 and the Weak game's generator first differs at 41890 instead of 41399 (every other pair unchanged) |
| T92 | AI-CON-1.7 (which wares, which state): E-trace-018 and E-trace-014, player 1 | stock2 at its refreshes (counters 1 mod 22 in these games; all ticks here are game counters, trace tick = counter + 1) | (a) the worker 93343 (good 7068dcd3) works to counter 40124 and puts his product down in 40125 .. 40128; its ware object 94403 is made in 40129, the first tick of his pick-up: the refresh of 40129 counts none of that good (the state at the end of 40128), the refresh of 40151 counts it. (b) The shipwright 93488's plank 94425 is a ware object from counter 40242 (he leaves with it) to 40447 (putting it down at the ship on (145, 46) in 40443 .. 40446, the plank gone as he picks up 40447 .. 40450, one tick standing, 55 ticks of work): counted by the refreshes 40261 .. 40437, not by 40459. (c) The plank 92491 of the building torn down in the think of counter 26070 (lying on its flag 90438) is counted by the refresh of 26071 (stock2(plank) 10 -> 11). (d) The helpers' wares of T91. An experiment build counting this way equals the recorded stock2 in all 1 817 refreshes of E-trace-014 (0 .. 40 000) but two, (c) and one more torn-down plank (in the experiment build a tear-down's wares appear only in the next tick), and in all 85 of E-trace-018 40 000 .. 41 880; its stock(g), read from the previous tick, equals the recorded stock in all 79 893 counts of E-trace-014 and 4 250 of E-trace-018 (76 differ when read from the same tick) |
| T93 | AI-CON-8.1e, `AI-overview.md` AI-2.7 b (usable fields): E-trace-018 (R5, the Weak game), player 1's street work on F (149, 30) (flag 94647) with T (142, 34), after (144, 33) was given up; attempts 1, 2, 3 at counters 41875, 41882, 41889 | the lengths and the draws | R5 `ai_street`: L stays 1000 after attempts 1 and 2 (best 0), and attempt 3 draws nothing (the generator: only the animals' draw in trace tick 41890). The 11-step walk F -> T (148, 29), (147, 29), (146, 29), (145, 29), (145, 30), (144, 30), (143, 30), (142, 31), (142, 32), (141, 33) uses four Border fields (state 0x220 in heap dump 41500): closed; the road-stop path (148, 30), (147, 30), ..., (142, 33) (the hit, road 93801), (142, 34) crosses the tree 71213 on (147, 30) (from the map's start to 56243): closed, no path within the bound that the hit note needs. Entering Border fields gives attempt 1 L 11 (and a road F -> T at attempt 6); entering the tree's field at a higher cost reads out a road-stop path (3 draws in 41890): the Weak game's generator first differs at 41890, with both closed at 45998, the settlers equal to R5 to 45674 and the buildings to 46000 (experiment build; every other pair unchanged) |
| T94 | AI-CON-10.4 step 0: E-trace-018 (R5, the Weak game), player 1, the goods arrangement run of counter 45647 (step 0); finished storehouses the castle 90180 (139, 51), 90351 (144, 47), 94385 (133, 39); stone 42, 2, 0 | the stone flags after the run (`goodflags.csv`, stop:out) | 0 + 40 < 42: the castle 1:1 (stop from the loop, out as the fullest), 90351 1:0, 94385 0:0 (its record made, stop set by the loop and cleared as the emptiest); plank and coin unchanged. The ware 95220 (stone) bound for 94385 is put down on (125, 27) at counter 45660 and goes on toward (127, 29) (its carrier 93607 called at counter 45673). Clearing stop on the fullest instead leaves 94385 stopped: the ware loses its storage trip at 45661, the carrier stays idle and the settlers first differ at 45675 (experiment build); with the rule the Weak game's generator first differs at 50113 instead of 45998, the settlers equal to R5 to 47932, the buildings to 50000 (every other pair unchanged) |
| T95 | AI-CON-8.1e (the settler mark): E-trace-013 (R3, the Hard game), player 1's street work on F (121, 65) (flag 95084) with T (121, 67); R = road 93491 "121,67 122,66 122,65 123,64 124,64", P = (124, 64); R's carrier 93505 came to stand on his idle field (122, 65) at counter 34099; the constructor 93994 walked over (122, 65) at 34620 .. 34640; (122, 64) is a building's field | attempt 4 (counter 36219) and the execution | the second leg F -> P is 3 steps over (122, 65) (the carrier no longer blocks): L 3, L2 3, best 4 (`ai_street`); at 36233 R is removed and the roads "121,65 122,66 121,67" and "121,65 122,65 123,64 124,64" are built; the released carrier finds the new road under him and road-walks home over (123, 64). Counting him as standing gives no leg (L2 6, best 3), only the first road is built, and the carrier free-walks home the other way: the Hard game against R3 differs from settlers 36240; with the mark from 36290 (the next point is TRN-CARRY T55's; experiment build, every other pair unchanged) |
| T96 | AI-CON-1.6 (crowded re-evaluated on removal), 7.6: E-trace-013 (R3, the Hard game), player 1's cells (9, 1) and (8, 1) | the crowded markers, the crowded target of counter 38578 | R3's heap dumps (the cells' crowded markers): (9, 1) set at 3483.4 s (an ordinary building, no military one), (8, 1) at 3499.3 s, both still set at 36000 and clear at 36500, their times kept; in between player 1 removes the site 94809 on (139, 27) in (8, 1) (last row counter 36196: 1 military, 6 ordinary -> 5, 5 x 1 < 5 false) and the site 94915 on (144, 30) in (9, 1) (last row 36278: 0 military, 1 ordinary -> 0). So at 3857.8 s the oldest set marker is (7, 2)'s (3578.2 s), and the crowded target goes to (7, 2) (dump 39000: last target (112, 32) at 3857.81 s; the next, (7, 3), at 3899.65 s). Evaluating only on additions keeps (9, 1) set and takes it at counter 38564: the Hard game vs R3 differs at buildings 38990. With the rule (experiment build): Hard settlers 39510 -> 41900, buildings 38990 -> 44400, generator 39502 -> 43016; every other pair unchanged |
| T97 | AI-CON-8.1f (the cursor moves on when the list closes up; Q-380): E-trace-013 (R3, the Hard game), player 1's street task (`ai_street`) | the runs of counters 42953 .. 42974 | 42953 and 42960: F = flag 95991 (135, 25) at cursor 132 (T (137, 26) remembered, attempt 1). In 42963 an earlier flag of player 1 is removed (134 -> 133 flags), so place 132 now holds 96047 (147, 26), site 96045's unconnected flag. 42967: cursor 133 and F = 96047 with T (144, 29) remembered (give-ups 0, excluded 1); 42974: the cursor past the end, restart (the first flag; cursor 1 after it); no attempts on 96047. Its road "147,26 146,26 145,27 145,28 144,29" comes only on a later round, at 44293, and site 96045's builder request (96051, filed 42791) is served at 44294. Keeping the cursor on 96047 does attempts 1 .. 5 at 42974 .. 43002 and builds the road at 43009; its builder leaves storehouse (144, 47) at 43010 and the take-out draw of 43015 misses it (Hard game vs R3: generator 43016, buildings 44400). With the rule (experiment build, with TRN-CARRY T56): the road at 44290, buildings 44400 -> 44700, generator 43016 -> 43124; every other pair unchanged |
| T98 | AI-CON-10.3 (the worker in the building): E-trace-013 (R3, the Hard game), player 1's food distribution run of counter 43718; its smithy 90597 on (136, 53) holds no coal from 43655 (its stock: iron 2, no coal entry) and its smith 90875, out with a product from 43656, walks back in and stands on the smithy's field from trace tick 43721 | the coal mines' weight | 0.5 kept (the coal mines' food orders are chosen every 55 ticks before and after, R3 heap dumps 43500 and 44000: order stamps 43604, 43659, 43714, 43769, 43824, 43879, 43934, 43989). Counting the smithy as starved because it has a worker sets 1.0, the coal mines' orders are chosen more often and the fish 96183 goes to mine (123, 55) instead of 95024 (119, 55) at 43879 (Q-383; Hard settlers 44286). With the rule (experiment build, with TRN-CARRY T56, AI-CON T97, ECO-OUT T27, ECO-FOOD T59): Hard settlers 44400 -> 45700, buildings 45100 -> 46700, generator 44612 -> 45865; every other pair unchanged |
| T99 | 8.3c (the direct variant, G without a route): E-trace-019 (R6, MP_4P_Fight), player 1's route task on the building 15189 (47, 62) (flag (47, 61)), partner the castle (flag (52, 55)). After the route variant's runs X holds (50, 61), (53, 57), (53, 61) (counter 3509; `ai_route`); in 3546 the route cost is above 4 x 8 and the run takes the direct variant at w = 0: Fn = (47, 61); G = (53, 61), flag 15625, a lone flag then (no roads, its own ware network) | the run of 3546 and the next | 3546: no road, X = [(47, 61)], w 1; 3583: w 2; 3620: the road (50, 61) - (47, 61) (15675, the plan cost 18) and k = 1 (recorded). Counting no route as too long asks the road (47, 61) -> (53, 61) at 3546 and takes k = 1 there: object numbers shift from 3620 and the Fight game against R6 differs from settlers 3663. With the rule (experiment build, with the Fight rules before it): settlers 3663 -> 4452, buildings 4125 -> 4676, generator 3725 -> 4698; every pair of the fidelity set unchanged |
| T100 | 10.4 step 0 (the order of "first found", the end state): E-trace-013 (R3, the Hard game), player 1, the goods arrangement run of counter 45647; finished storehouses with planks: the castle 90180 (139, 51) 45, 92510 (130, 33) 1, 93353 (120, 59), 94007 (138, 26) and 94359 (146, 36) none (`stocks.csv`); every one has a plank record with both flags clear from step 1 | the plank flags after the run (`goodflags.csv`, stop:out) | 0 + 40 < 45: the castle 1:1, the harbour 90366 and 92510, 93353, 94007 1:0, 94359 0:0: the first storehouse with the smallest stock in cell-list order is 94359 (in the experiment build's cell list the storehouses' cells come as (8, 3), (8, 2), (9, 2), (7, 3), (8, 1)). Sorting by field (row, then column) takes 94007 (138, 26); skipping the "clear stop" step because the record was already clear leaves every storehouse stopped, so the plank of T59 in `TRN-carriers-transport.md` finds no warehouse: the Hard game against R3 differs from settlers 45670 (experiment build); with both fixed and T59, settlers 45723, buildings 46985 |
| T101 | 8.3 a (state kept, E-ai-066): R6 player 2, route task on its hunter's hut (k 0, i 3, t 2, X ending on (51, 20)); the hut torn down at counter 6631; next record building 14678 on (58, 24), flag on (58, 23) | the runs of 6649 and 6686 | 6649: cursor moved to 14678, nothing else changed (i 3, t 2, X kept); 6686: 14678 taken up with that state; the direct variant asks the road from (58, 23) to the flag on (51, 20) and it is built: "58,23 58,22 57,21 57,20 56,19 56,18 55,17 55,16 54,16 53,16 52,17 52,18 51,19 51,20"; after the run k 1, i 0, t 0, X empty. Clearing the state at the take-up builds no road there |
| T102 | 2.5b, 8.3 b: E-trace-019 (R6, the fight game), player 3. Its sawmill site (25, 22) of counter 322 makes a wood need (2.5), which gets the home cell and the square (4, 4) before counter 500 and no production (heap 500: need 14258, production none). The cell system's own woodcutter order places a woodcutter on (30, 22) at counter 622 | the woodcutter's announcement; the route optimizer's run of 6829 on the sawmill | the woodcutter's production is tied to the sawmill's wood need (heap 1000: need 14258, production 14315 of the woodcutter 14314). The sawmill's k = 2 partner at 6829 is that woodcutter, flag (30, 21), not the later woodcutter (26, 19) of counter 1786; the route task's rows from 6792 to 6903 follow (E-ai-067's emulation). Experiment build with both ties: the fight game's generator is equal to 8340 (was 8205); the Hard game's generator to 49 218 (was 48 043), its settlers to 49 500 and buildings to 50 700 (were 48 200); the Weak, one-player and two-player 40 000 games unchanged (no regression) |
| T102a | 2.5b (the list order), 9.3, 8.3 b: E-trace-019 (R6, the fight game), player 2. Its wood needs 15177 (sawmill (55, 24)) and 15245 (sawmill (54, 22)) are made in that order; 15177 is tied to the woodcutter (56, 22), which is torn down at counter 5810 (heap 5500: 15177 production 14908, square (12, 5); heap 6000: no production, square (14, 4)), so 15177 is given its cell again and goes to the end of the cell's need list (heap 8500, the cell's list: ... 14266, 15245, ..., 15862, 15480, 15177, 16800). The cell system's woodcutter (59, 30) is placed at counter 8824 | which wood need its production is tied to; the route optimizer | the production of (59, 30) is tied to 15245, the first waiting wood need in the list (heap 9000: 15245 production 17058; 15177 still none); so the sawmill (54, 22)'s partner list (8.3 b) becomes [the woodcutter (59, 30), the headquarters]: at counter 8832 its k = 2 run takes the woodcutter and steps on to k = 3 (the recorded route table: k 3 at 8832, the road (52, 23) -> (54, 24) at 9387). Tying by making order (to 15177) keeps the headquarters as k = 2, builds that road at 9276 and makes the fight game's generator differ from trace tick 9311; with the rule it moves to 9371 (experiment build) |
| T103 | 6.1 (the harbour list): E-trace-019 (R6, Keyhole, three Hard computer players), player 3's woodcutter order searched from its cell's position (24, 24) between counters 9737 and 9772; (18, 30) is harbour pavement, and the pavement runs over (15 .. 18, 30), (14 .. 17, 31), (16 .. 17, 32); player 3's only building within 3 of (18, 30) is a finished guard house on (21, 30); it has no harbour | the values of (18, 30) and (19, 30) and the site | both 0 (the first pavement field in their rings is (18, 30), with no own harbour within 3); the best stays (28, 24) (value 4, route 5, score 4 x 11 / 16 = 2.75), placed in counter 9772 as recorded. Counting the guard house, (19, 30) would win with 10 x 8 / 16 = 5.0 |
| T104 | 8.1e (no settler blocks), 8.1 b: E-trace-019 (R6, Keyhole, three Hard computer players), player 2's street work on F (57, 26) with T (58, 28); R = road "58,28 57,27 58,26 58,25", P = (58, 25); R's carrier has stood on its idle field (58, 26) since counter 9678 and nobody has crossed it; (57, 25) is an inner field of another road at P; the road-stop path of attempts 3 to 5 is F, (57, 27), T | attempts 1 to 6 (counters 9873 .. 9908) and the execution | attempts 1, 2, 3 count nothing; attempt 4: the second leg F -> P is 2 steps over (58, 26): L 3, L2 2, best 4 (`ai_street`); attempt 5 changes nothing; attempt 6 (9908) repeats the road-stop search (two draws), removes R and builds "57,26 57,27 58,28" and "57,26 58,26 58,25" (streets table). Counting the carrier as standing leaves no second leg, attempt 6 does nothing and draws nothing, and the fight game's generator differs from 9909 |
| T105 | 6.3 (the depot test counts sites): E-trace-019 (R6, Keyhole, three Hard computer players), player 1's storehouse order searched from its cell's position (54, 47) between counters 10412 and 10456; player 1's headquarters stands on (52, 56) and its harbour is a building site on (62, 49) (placed, levelling under way) | the values and the site | with the harbour site counted: (55, 45) (distance 9 from it), (56, 44) (9), (55, 43) (10) and the fields east of them are worth 0, (54, 44) (distance 11) is worth 1 with route 3: score 13 / 16 = 0.8125, and the storehouse site goes on (54, 44) in counter 10456, as recorded; counting only finished warehouses, (55, 45) is worth 1 with route 2 (score 0.875) and wins |
| T106 | 8.1f (next flag after the T97 move): E-trace-018 (R5, the Weak game), player 1's street construction (a run every 7 ticks); after the run of counter 51416 the cursor is 88 with work on the flag (121, 31); a flag earlier in the list is removed before the run of 51423 (the list 119 -> 118 flags) | the runs from 51423 | 51423: the flag at 88 is now (121, 52), not the worked one: the cursor moves to 89, (121, 52) is handled as a new F and finds no T, the cursor moves on to 90 (recorded: cursor 90 after 51423; 91 after 51430, 92 after 51437; work on (122, 35) remembered at 51444 and on (114, 35) at 51675, its road "114,35 115,35 116,34 117,34" built at 51717 and the castle's builder for the site (114, 36) made at 51721). Setting the cursor to the handled flag's place plus one (89) puts every later run one place behind: the work on (114, 35) begins only at 51682 and is dropped at the next run (another T97), the road is never built there, and the Weak game's generator differs from 51724; with the rule (experiment build, the tick-exact branch's head) from 52420, every other pair unchanged |
| T107 | 4.2 (avgProd over production records only): E-trace-019 (R6, the war: MP_4P_Fight, players 1 to 3 Hard), player 1's needs construction at counter 12080 (the needs order of 12044 placed at 12075); open needs with a cell and a producer type: meat D 1 A 0 stock 12 priority 50; donkeys D 1 A 0 stock 0 priority 40; others lower; player 1's only meat-making building is the hunter's hut on (43, 60) (placed by the hunter part, value 0.5); no meat production record (heap dump of 12000: 18 records, none of meat) | the chosen good | meat: 1 x 50 x 0.88 / 0.2 x 1.0 = 220 against donkeys 1 x 40 x 0.99 / 0.2 x 1.0 = 198; the slaughterhouse order for the cell (3, 3), its site (48, 63) placed at counter 12224 (recorded). With the hut's 0.5 counted, meat scores 110, the donkey farm is ordered and fails for material, and the war's buildings differ from 12225; with the rule (experiment build, the tick-exact branch's head) settlers 12227 -> 12486, buildings 12225 -> 12705, generator 12476 -> 12485, every Storm Coast pair unchanged |
| T108 | 2.5b with a needs order: E-trace-013 (R3, the Hard game), player 1's stone needs in cell (8, 3): 90188 (made at counter 41, given the cell after 93251), 93251 (made at 24011); the needs order for 90188 places the granite mine 94759 on (132, 53) in counter 34129 | which need gets its production 94760 | 93251 (heap dumps 34000 / 34500: 93251 production -1 -> 94760; 90188 stays without one, and gets its own mine 97572 on (133, 49) at 52427, whose food need draws the bread need 97581 at 52436). Tying the production to the order's need (90188) leaves 93251 waiting, the mine of 52427 is not placed and the bread need's draw is missing: the Hard 150 000 generator first differs at trace tick 52437; with the rule 53194, settlers 52600 -> 53000, buildings 52600 -> 54000 (experiment build on 0.12.3 with the rules of 2026-11-21; every other pair unchanged; the only placement in 53 000 ticks whose tie changes) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ai-002 | code reading | `S2DNG.exe` | need creation |
| E-ai-002 | code reading | | needs arrangement and cell costs |
| E-ai-002 | code reading | | needs construction, score, order, failure |
| E-ai-002 | code reading | | construction system, budget, site search |
| E-ai-002 | code reading | with its helpers | site value |
| E-ai-002 | code reading | | forester, hunter, pit/woodcutter, depot, harbour, crowded, cell validator |
| E-ai-002 | code reading | | cell markers and full marks |
| E-ai-002 | code reading | | roads |
| E-ai-002 | code reading | | demolition and protection flags |
| E-ai-002 | code reading | | production control, locksmith, food, goods |
| E-ai-005 | code reading | | cell building lists and markers (corrected roles) |
| E-ai-005 | code reading | | stock and stock2 (AI-CON-1.7) |
| E-ai-005 | code reading | | S = owned fields (AI-CON-1.8) |
| E-ai-005 | code reading | | producer tables (AI-CON-1.4) |
| E-ai-005 | code reading | | pavement rule (AI-CON-6.1) |
| E-ai-005 | code reading | | street construction strategies, flag search (AI-CON-8.1, 8.4) |
| E-ai-005 | code reading | | route optimizer (AI-CON-8.3) |
| E-ai-005 | code reading | | goods arrangement (AI-CON-10.4) |
| E-ai-002 | the program's data | the property setter table and the setters' member writes | profile, building and good property fields |
| data | script | `Goods.lua`, `buildings.lua`, `ai.lua` | flags, priorities, profile values |
| E-ai-006 | code reading | | building classification (AI-CON-1.1); food links (3.1); waiting need and expansion score (3.4, 3.5); site value (6.2, 6.3); harbour switch (7.5); road-stop search and strategies 3 to 5 (8.1); flag list order (9.5) |
| E-ai-007 | code reading | | budget, pick order and age rule (5.1, 5.2), execution timeline (5.3a), scan order and step budget (5.4), hunter order (7.2) |
| E-ai-008 | code reading | | AI-CON-2.5, T25 (Q-252) |
| E-ai-009 | code reading | the site value's border loop: a ring iterator, start = p stepped 2 in direction 4, 2 steps per direction 0 .. 5, state bit 5 tested, the factor a constant; not the land leveller (ECO-HQ-4.3a) | AI-CON-6.4, T27, T28 |
| E-sim-002 | code reading | | grid cursor and counts of the stone pit / woodcutter run (7.3), list index of 7.1 / 7.2, validity scan cursor (1.1); cell position (1.1a) |
| E-trace-002 | recording | `buildings.csv.gz`, counters 0 .. 5000 every tick | the first ten sites of the computer player (T21) |
| E-sim-007 | code reading and data | `data/scripts/properties/buildings.lua` (castle: depot 1, territory range 9, no soldier limit); E-trace-006 `goodflags.csv.gz` (castle records at 403 and 1280) | AI-CON-1.1, 10.4: the headquarters is a depot |
| E-ai-010 | code reading | | AI-CON-8.1, 8.1e, 8.3; T30 .. T32; E-trace-006 `ai_street`, `streets`, `objects`, `heights` |
| E-ai-011 | code reading | E-trace-007 heap dumps 500, 1000 (the construction system's route task) | AI-CON-8.3 b, c; T32, T33 |
| E-ai-013 | code reading and experiment | E-trace-008 heap dump of trace tick 1000: continent layer = the map file's continent index per field, 28 900 of 28 900; experiment build (febd6f5 + Q-356 + both rules): guard house (138, 57) in 1101, stone pit (140, 57) in 1309 | AI-CON-5.3, 5.4a, 6.3, T34 |
| E-ai-016 | code reading | scan of every read of the map's continent layer: the border scan, the cell landmass at the first position, the harbour marker, the resource entries and conquered continents, the military value, the cell counts, the home landmass, the warehouse search in mode 1 (nearest finished warehouse), the cells' landmass, the free-walk search in mode 1 (TRN-FREE-6.4 step 3) | AI-CON-1.1a, 1.1b, 1.6, 3.5, 6.3, 7.1, 7.3, T34; AI-MIL-4.2, 4.3, 6.1; AI-MISC-1.2, 2.1; TRN-FREE-6.4 |
| E-ai-017 | code reading and experiment | the ware order matching (from D's flag to each warehouse's flag, kind 1; (a) cost(H's flag, D's flag) against cost(D's flag, Wf's flag)); experiment exp4: route task equal to E-trace-010's `ai_route` run for run from counter 993 to 2584 | AI-CON-8.3 c, T36, T37; TRN-CARRY-5.3a; ECO-REQ-5.3 |
| E-ai-018 | code reading and recording | E-trace-010 `objects`, `counter`, generator | AI-CON-3.1, 3.2, 3.3, 4.2 |
| E-ai-019 | code reading and experiment | experiment build exp4 with the corrected count: farm at (136, 58) in trace tick 3725, buildings 3725 -> 3987; the supplier value's warehouse search in mode 1 (nearest finished warehouse on the field's continent index); experiment: brewery at (136, 51) in trace tick 3987, buildings 3987 -> 4792 |
| E-ai-023, E-trace-011 | code reading and recording | `ai_route.csv.gz` of E-trace-011 | AI-CON-1.2, 8.3 a, b (T45, T46; Q-279) |
| E-ai-027, E-trace-011 | code reading and memory reads of the running game | heap dumps 500 .. 8000 of E-trace-011 (the destruction part's cursor; the player's flag vector: 24 flags in id order at 8000) | AI-CON-9.5 cursor (T50) |
| E-ai-028, E-trace-011 | code reading, a memory read of the running game, recording | heap dump 10000 of E-trace-011 (the bakery's record: need list 90652, 90495); `ai_route`, `ai_plan`, `counter` tables of E-trace-011 | AI-CON-8.3 b need order (T51) |
| E-ai-030, E-trace-011 | code reading and memory reads of the running game | heap dumps 9500, 10000, 10500 of E-trace-011 (3 squares per call, cursors; cell (8, 3) wood 41, 41, 39; the forester slot) | AI-CON-1.1c, 7.1 (T52) |
| E-ai-031, E-trace-011, E-sav-005 | code reading, memory reads of the running game, save | the full mark's setting and its 37-tick expiry, its setters (needs order and shipyard order, no site) and readers (shipyard part, needs arrangement), the execution states (4 no material, 5 no site); heap dumps 7000, 9000, 9500 of E-trace-011 (cell (9, 2)'s ship mark: active, time, count; the nine order slots); the marks of E-sav-005 (AI CellFullElement) | AI-CON-1.6 (T53) |
| E-ai-032, E-trace-011 | code reading, a memory read of the running game, recording | heap dump 10500 (need 90470: consumer reference none; 90685's production 90686 -> need 90470); ai_route (counters 10724, 10761) | AI-CON-10.2, 8.3 b (T54) |
| E-ai-033 | code reading | | AI-CON-5.4b, T56 |
| E-ai-035, E-trace-011 | code reading, memory reads of the running game, recording | heap dumps 3000, 10000, 17000, 17500 (player 1's AI core); deposits table | AI-CON-1.1c (T57) |
| E-ai-037, E-trace-012 | code reading, memory reads of the running game, recording | E-trace-012 heap dumps 5000 .. 6500 (cells, needs, full marks) and objects table | AI-CON-3.3 distances and combination (T59) |
| E-ai-038, E-trace-012 | code reading and recording | `ai_street`, `ai_plan` and flags tables of E-trace-012 | AI-CON-8.1f (T60) |
| E-ai-039, E-trace-012, E-trace-013 | code reading and recordings | `ai_street`, `ai_plan` of E-trace-012, settlers table of E-trace-013 (carrier 90807) | AI-CON-8.1e occupied R field (T61) |
| E-ai-040, E-trace-012 | code reading, memory reads of the running game, recording | route stores of E-trace-012 at 7500 and 8000, `ai_route` and objects tables | AI-CON-8.3 c cost order (T63) |
| E-ai-041, E-trace-013 | code reading and recording | heap dumps 9000 .. 11 000 of E-trace-013 (need map, order slots) | AI-CON-3.4 (T65), AI-CON-7.1 (T66) |
| E-ai-043, E-trace-014 | code reading, memory reads of the running game, recording | E-trace-014 heap dump 22500 (the warehouse list: 90175, 90180, 90351; the upgrade construction of 91705), `ai_route` and buildings tables | AI-CON-8.3 b, AI-2.1a (T69) |
| E-ai-044, E-trace-013, E-trace-014 | code reading, memory reads of the running game, recordings | E-trace-014 heap dumps 22500, 23000 (needs, productions, cells, squares, full marks, the search state), E-trace-013 heap dumps 9500, 10000 | AI-CON-3.3 (T70, T71), AI-CON-6.2 (T72) |
| E-ai-045, E-trace-011 .. 014 | code reading and recordings | `ai_street`, `ai_walk`, `ai_plan` tables | AI-CON-8.1 a, 8.1e (T74, T75) |
| E-ai-047, E-trace-015 | code reading and recording | E-trace-015 window heap dumps 10100 .. 10600 every 10 | AI-CON-3.3 (T77) |
| E-ai-057, E-trace-013 | code reading and memory reads of the running game | heap dump 34000 of E-trace-013 (stone needs, the needs order slot) | AI-CON-3.4 (T89), T88 |
| E-ai-058, E-trace-018 | code reading and recording | E-trace-018 `objects.csv.gz` (deposit 94419 new at 40193), `deposits.csv.gz`, heap dumps 40000 / 40500 | AI-CON-6.2, 9.2 (T90) |
| E-trace-014, E-trace-018 (stock counts) | recordings | E-trace-014 and E-trace-018: `stocks.csv.gz` (player stock_a = stock, stock_b = stock2; building stock_b = a warehouse's store), `wares.csv.gz` (every ware object per tick: holder, flag, lying), `objects.csv.gz` (ware made / gone), `buildings.csv.gz` (enabled) | AI-CON-1.7 (T91, T92) |
| E-ai-059, E-trace-018 | code reading and recording | E-trace-018 `ai_street.csv.gz`, `generator.csv`, heap dump 41500, `objects.csv.gz` (deposit 71213) | AI-CON-8.1e, AI-2.7 b (T93) |
| E-ai-060, E-trace-018 | code reading and recording | E-trace-018 `goodflags.csv.gz`, `stocks.csv.gz`, `wares.csv.gz` (ware 95220) | AI-CON-10.4 (T94) |
| E-ai-061, E-trace-013 | code reading and recording | R3 heap dumps 33000 .. 39000, `buildings.csv.gz` (sites 94809, 94915) | AI-CON-1.6, 7.6, T96 |
| E-ai-062, E-trace-013 | code reading and recording | R3 `ai_street`, `flags` (42953 .. 42974), `streets` (44293) | AI-CON-8.1f, T97 |
| E-trn-029, E-trace-013 | code reading and recording | E-trace-013 `goodflags.csv.gz`, `stocks.csv.gz` (counter 45647) | AI-CON-10.4, T100 |
| E-ai-064, E-trace-019 | code reading and recording | E-trace-019 `ai_route.csv.gz`, `ai_plan.csv.gz`, `flags.csv.gz` (player 1, counters 3450 .. 3620) | AI-CON-8.3c, T99 |
| E-ai-066, E-trace-019 | recording, experiment build | E-trace-019 `ai_route` (player 2's runs 6600, 6612, 6649, 6686), `streets` (road 16452 at 6686), `buildings` (hunter's hut 14615 gone at 6631), `objects` (the carrier request 16453 at 6687, served 6707); an experiment build keeping the state at the take-up: fight generator 6714 -> 8072, buildings 7030 -> 8280, other games unchanged | 8.3 a, T101 |
| E-ai-063, E-trace-013 | code reading and recording | R3 `stocks` (smithy 90597), settlers (smith 90875), heap order stamps (dumps 43500, 44000) | AI-CON-10.3, T98 |
| E-ai-048, E-trace-013 | code reading, a memory read of the running game and recording | heap dump 15500 of E-trace-013 (the AI core: needs order slot and need map) | AI-CON-4.2 (sites count), T78 |
| E-ai-050, E-trace-013 | code reading and recording | `E-trace-013` `objects.csv.gz` (deposits alive at counter 18563), `buildings.csv.gz` | AI-CON-6.2 (the flood), T81 |
| E-ai-051, E-trace-013 | code reading and recording | `E-trace-013` `objects.csv.gz` (cell 92432 at counter 18294), `counter.csv.gz` (next id 92432 -> 92433) | AI-CON-1.1 (the corners), T82 |
| E-ai-052, E-trace-013 | code reading and recording | heap dumps 17500 .. 19500 of E-trace-013 (the construction part's hunter object: order, index; the cells' hunter references), `animals.csv.gz` (animal 92625) | AI-2.4a, AI-CON-7.2, T83 |
| E-ai-053, E-trace-013 | code reading and recording | heap dumps 20000 .. 22000 of E-trace-013 (each need's cell, production and square), `objects.csv.gz` (production 90987 gone at 20207) | AI-CON-9.3, T84 |
| E-ai-055, E-trace-013 | code reading and recording | E-trace-013 `objects.csv.gz` (need 93251 for 90180 at counter 24011; the stone productions) | AI-CON-2.4, T86 |
| E-ai-056, E-trace-013 | code reading and memory reads of the running game | E-trace-013's heap dumps 20500 .. 28000 (player 1's food table) | AI-CON-10.3, T87 |
| E-ai-054, E-trace-013 | code reading and recording | E-trace-013 `objects.csv.gz` (need 93032, cell 90199), `buildings.csv.gz` (93115) | AI-CON-3.3, T85 |
| E-ai-025, E-trace-011 | code reading and recording | E-trace-011 `flags.csv.gz`, `streets.csv.gz`, `wares.csv.gz` (flag 90626, road 90634, plank 90927) | AI-CON-9.5 (T47) |
| E-ai-026, E-trace-011 | code reading and memory reads of the running game | heap dumps 7500, 8000, 9500, 10000 (cell (9, 3)'s position) | AI-CON-1.1a (T49) |
| E-ai-069 | code reading, heap dumps and measured | the R6 heap dumps 4500 .. 9000 (player 2's need map and its home cell's need list), the recorded route table, an experiment build | AI-CON-2.5b list order, T102a |
| E-ai-070 | code reading and measured | the building system's list registration (the harbour list filled by the type's harbour flag), the pavement test's lookup, the R6 heap dump of 9500 (field state bits around (18, 30)) and buildings table of 9762, an experiment build | AI-CON-6.1, T103 |
| E-ai-071 | recorded tables and measured | R6's `ai_street`, streets and settler rows around counters 9678 .. 9910, the field state bits of the heap dump 9500, the street task's field test read for its road and field-object checks, an experiment build without the settler test (fight generator 9909 -> 10698, Weak 51724 and Hard 49218 unchanged) | AI-CON-8.1e, T104 |
| E-ai-072 | code reading and measured | the site value's depot test (it walks the building system's depot list, filled at placement, and tests owner and distance only), R6's buildings table at 10440, an experiment build (fight generator 10698 -> 11964, buildings 10457 -> 12067, settlers to 11686; Weak 51724 and Hard 49218 unchanged) | AI-CON-6.3, T105 |
| E-ai-074, E-trace-018 | recording and experiment build | (session 2026-11-17 day): R5's `ai_street` table (cursor and work per run, counters 51000 .. 51730) and flags table; the experiment build's runs | 8.1f next flag (T106) |
| E-ai-075, E-trace-019 | code reading, heap dump, experiment build | (session 2026-11-17 day): the needs score's mean over the production records; player 1's production records in R6's dump of 12000 | 4.2 avgProd records only (T107) |
| E-ai-079, E-trace-013 | memory reads of the running game, recording, experiment build | (session 2026-11-21 pm): R3's heap dumps 34000, 34500, 52000, 52500 (player 1's need map), objects and buildings tables | 2.5b the production of a site placed by a needs order (T108) |
| E-ai-020 | code reading and experiment | experiment build exp4: the route task equals E-trace-010 run for run to counter 3472 (from 2584), dense settlers 4218 -> 4308 |
| E-ai-021 | code reading, memory reads of the running game and experiment | E-trace-010 heap dumps 3500, 4000, 4500 (the AI player's destruction cursor) |
| E-ai-022 | code reading, memory reads of the running game and experiment | E-trace-010 heap dumps 4500, 5000, 5500 (the AI player's construction part: budget, state, hunter holder; cell (8, 2)'s hunter id) |

## Open questions

- Q-081: AI-CON-8.1 strategies 3 to 5: which end of the road found by the
  reverse search is P (the code compares a road field pair with the target
  flag's building reference), where the last road of strategy 5 starts, and
  whether the game really removes the road in the way. Needs an
  in-game observation of an AI connecting a building across an existing road.
- Q-082: AI-CON-6.1: which building list of the building system (its third
  list) the pavement rule checks; and whether construction sites count in
  stock(g) (AI-CON-1.7).
