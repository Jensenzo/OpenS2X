# AI-MIL: Computer opponent, military and expansion

Status: draft (rules read from the binary; not yet checked against the running game)
Owner question(s): Q-067 (military part), Q-070 (profile use, partly), Q-077 to Q-079; plan step 5.1 chunks 2 and 4
Game build: 11757

## Summary

The computer player keeps two coarse maps of 8 x 8-field cells: a **military
map** per player (threat and score of the buildings in each cell, from
`bp_setAiThreat` / `bp_setAiScore`) and its own **territory map** (per-cell
counters of failed military sites, attacks and catapults). From these it
places military buildings towards expansion targets, picks attack targets
every 641 ticks and sends attacks with the player's normal attack order,
builds catapults where enemy threat is high, upgrades military buildings
near enemies, and sets the player's military settings from its profile.
Soldier materials are concentrated in one warehouse. Periods are in
`AI-overview.md`; this file gives the decisions. `AI-construction.md` gives
the model (cells, needs, orders, site search).

## Rules

"Enemy distance" of a field = the distance to the nearest *occupied*
military building of an enemy player (1000 if none). "Soldiers in stock" =
soldiers of all ranks in the player's warehouses. "Strength" of a player =
sum over its soldiers in buildings and in the field of (rank + 1) plus sum
over ranks of stock x (rank + 1), ranks 0..4. S is the statistics value of
AI-construction (AI-CON-1.8).

### AI-MIL-1 Maps

- **AI-MIL-1.1 Military map.** For every player slot a grid of 8 x 8-field
  cells with, per cell: threat = sum of `bp_setAiThreat` of that player's
  buildings in the cell (sites and unoccupied buildings included, no
  occupation test); threat under construction = the same sum over the
  buildings whose construction still has progress (sites, and finished
  buildings being upgraded, `MIL-building-upgrade.md`); and score = sum of
  `bp_setAiScore` over all of them. One cell per player slot that holds a
  player is refreshed per tick, cycling (AI-2.1, Q-159). Confidence: high.
  **A type without `bp_setAiScore` scores 20** (corrected 2026-10-29, E-ai-049: the building type's default; its threat default is 0). In the shipped
  `buildings.lua` these are spring, barracks, guard house, tower, mill, shipyard,
  watch tower, farm, fortress and the construction dummy. Measured T25: player 1's
  cell (16, 7) of the Hard game holds 510 = 490 from the scored types plus 20 for
  its guard house site.
- **AI-MIL-1.2 Threat balance** of a cell = over the 3 x 3 neighbourhood of
  cells and all alive players: plus the threat of enemies, minus the threat of
  the player itself and its allies. **Weights (corrected 2026-10-29, E-ai-049):** the neighbour one column and one row further, (x + 1, y + 1),
  weighs 1; the centre and the seven other cells weigh 0.5. (The intent was
  likely the centre at 1; the original gives the 1 to (x + 1, y + 1).) Cells off
  the grid count nothing. Measured T26: the catapult pass of counter 4838 finds
  350 for cell (17, 6), the value in the scan state of heap dump 4500; centre
  weight 1 gives 340. Confidence: high.
- **AI-MIL-1.3 Territory map.** The player's own 8 x 8 grid with, per cell: a
  military-site penalty P, an attack result counter C, the number of
  catapults placed K and an attack attempt counter A. Confidence: high.

### AI-MIL-2 Military settings (every 53 ticks; only once the player has a building)

- **AI-MIL-2.1** Attack strength = the profile's attack strength and the
  intercepting (defend outside) setting = the profile's intercepting factor
  (`MIL-attack-combat.md` MIL-ATK-7.4), set every run. Confidence: high.
- **AI-MIL-2.2 Near and medium allocation.** A countdown of 50 runs gates
  the change; it starts at 0 and is decremented before the test, so the
  first run may already change the allocation (Q-166f). With m = the current medium allocation: if no soldiers are in
  stock, f = m x 8/3; near = min(1, f), medium = min(1, 0.3 f) (the values
  shrink). Else if soldiers in stock > S / 128, f = min(4, m x 3.3333 x 1.25);
  near = min(1, f), medium = min(1, 0.3 f) (they grow towards 1). Otherwise
  no change and no countdown. Confidence: high (formula), medium (that the
  base is the medium value).
  **The countdown exactly (2026-11-15 day; E-ai-067, the R6 recording and an experiment build).**
  A change sets the countdown to 50. Every run first lowers it by 1 and tests only when it is 0
  or below. So two changes are **50 runs apart**, not 51: the countdown is lowered in each of the
  49 runs after a change, and the 50th run after it tests and can change again. A run whose test
  finds no change leaves the countdown at 0 or below, so the next run tests again. Measured in R6
  (MP_4P_Fight, Hard players, a run every 53 ticks for player 1):
  - player 1's medium allocation had grown three times to 0.586;
  - its fourth growth, to 0.733 (near 1), comes in the run of counter 7997;
  - its fortress on (56, 50) (9 soldiers at most) then wants trunc(0.733 x 9 + 0.5) = 7 instead of
    5, and asks for one soldier at 8000 and one at 8021 (MIL-SOL-4).

  An experiment build with 51 runs between changes was three runs late (countdown 3 at 7997). It
  made no request there, and so sent player 1's builder out of the headquarters through the free
  door at 8003, instead of at 8062 after the two soldiers. With 50 runs the fight game's generator
  is equal to 8205 (was 8072) and its buildings to 8580 (was 8280). The Weak, one-player and Hard
  games are unchanged. Test T29.
- **AI-MIL-2.3 Defense type.** Mode Safe sets type 0, Aggressive type 1,
  Normal leaves it. Confidence: high.

### AI-MIL-3 Expansion targets (every 29 ticks)

- **AI-MIL-3.1 Kinds and sources.** An expansion target is a field position
  with a kind: 1 "towards a position" (set by the cell system for a waiting
  resource need, AI-CON-3.5, and by a failed attack, AI-MIL-5.5), 2 "crowded
  cell" (AI-CON-7.6), 3 "anywhere" (a need found no cell, AI-CON-3.3, at the
  player's home position). Only while no target is in progress (none taken
  and not yet finished or cancelled, 3.2), the oldest pending target of the
  four sources is taken (strictly older wins, sources in the order waiting
  need, crowded cell, anywhere, attack); one run later it becomes the current
  target. **The source's slot keeps the target** until the target is finished
  or cancelled (3.2): taking it clears only the expansion system's own copy
  (corrected 2026-10-26 night, E-eco-019: "its source slot
  is cleared"; measured, E-trace-011 heap dumps of 5500, 6000 and 6500: player
  1's cell-system slot still holds kind 1, (116, 48), 484.72 s, taken at counter
  5330, AI-MIL T17). So a source that makes a target only into an empty slot
  (the cell system, `AI-construction.md` AI-CON-3.5) makes no new one while
  its target is worked on. Confidence: high.
- **AI-MIL-3.2 Outcome.** A target is **cancelled** at once when its military
  building is given up (AI-MIL-4.4; also when 4.1 finds no cell for a kind 1 or
  3 target), and **finished** at once when an own building is upgraded instead
  (4.4), or later: every military-building run that makes an order
  **remembers** the current target (4.1), and the first later run that is not
  skipped (4.1: no order pending and the cooldown run down) finishes a
  remembered target. A placement leaves it remembered (and starts the 3-run
  cooldown, so three runs of 311 ticks pass in between); so does a chain of
  orders that ends for **no material** below level 0 (4.4; corrected
  2026-10-27 night, E-ai-024, measured T18); only a
  cancel or an upgrade in 4.4 forgets it. Finishing or cancelling tells the source and clears
  both the source's slot and the current target: for a waiting-need target the
  attempt counter of the target's cell coordinate is halved on success and
  raised by 1 on failure; the other sources are just cleared. That run makes no
  order (no current target is left). (Corrected 2026-10-26 night, E-eco-019: "when placed, the
  source is told"; measured: the guard house (131, 59) of the coal target
  placed at counter 5898 and the target finished in the run of 6836; the
  cell system's next target (116, 36) at 684.78 s, for the iron ore need, in
  the heap dump of 7000, AI-MIL T17.) Confidence: high.

### AI-MIL-4 Military buildings

- **AI-MIL-4.1 Creation (every 311 ticks).** Nothing while an order is pending
  or during the 3-run cooldown after a placement (a cooldown run only counts
  down). A run that gets past both first finishes the target it remembers
  (3.2: the target of the last run that made an order, unless 4.4 has ended
  it since) and then stops. Otherwise it sets the attempt count to 0, makes
  its order and remembers the current target. With a current expansion
  target: kind 1: the site is the best military-map cell of the home landmass
  (AI-MIL-4.2), order radius 12; kind 2: the target position itself, cell
  mode (the whole 16 x 16 cell); kind 3: best cell without a position, radius
  20. Confidence: high.
- **AI-MIL-4.2 Best cell.** Breadth-first search over the 8 x 8-field
  military-map cells, starting at the cell of the player's home position,
  expanding each dequeued cell's 8 neighbours in the order (-1,-1), (0,-1),
  (+1,-1), (-1,0), (+1,0), (-1,+1), (0,+1), (+1,+1) (dx, dy in cells), each
  neighbour only if its top-left field (8 cx, 8 cy) lies in the map on the
  same landmass as the home field (its stored continent index equals the
  index at the home position, `AI-construction.md` AI-CON-1.1b) and inside
  the military map, and not yet visited. Every
  reached cell (the start included) is evaluated; the search never stops
  early, so the whole connected set of home-landmass cells is scanned. A cell
  qualifies if its threat balance < 40 and (one of its 8 neighbouring cells
  holds military threat of the player, or (the player has buildings in the
  cell (score > 0), none of them under construction (threat under
  construction 0) and one of its 8 neighbouring cells holds no building of
  the player (score 0))). The neighbour tests read the player's own map
  (Q-166a; confidence high for "neighbouring cells", medium for "own map"). Its
  value = 100 / (4 x d + max(1, own threat)) / (P + 1), with d the hex
  distance from the cell's top-left field to the target field (d = 1 when
  there is no target) and P the cell's site penalty. Strictly greater wins
  (ties keep the earlier cell); the site is the winning cell's top-left
  field; no qualifying cell leaves the site at (-1, -1). Confidence: high.
- **AI-MIL-4.3 Which building (Q-078).** level = f(d) with d the enemy
  distance at the site (distance to the nearest occupied enemy military
  building, 1000 if none): d <= 18 gives -1 (creation) or 2 (upgrade check);
  d > 18 gives -2, except exactly d = 24 gives 1 (read literally from the
  code; the intent is unknown, the effect is that far from the enemy only a
  barracks is built unless resources or stone add to the level). Always: +2
  if an underground resource (not fish) lies within 4 squares (4 x 4) of the
  site, +1 if stock(stone) > 40. For the upgrade check also +1 if soldiers in
  stock > 30 and +1 if the nearest own finished **warehouse** (the game's
  warehouse list: headquarters, storehouses, harbours) whose field is on the
  site's landmass (AI-CON-1.1b) is within distance < 5 (corrected 2026-10-25
  evening, E-ai-016: not a building of any kind). The result is max(0, level) minus the
  number of failed attempts; type 0 barracks, 1 guard house, 2 tower, 3 or
  more fortress; a negative result means no building. At the first attempt
  the result is never negative, so a building is always ordered; the level
  only decides its size and how many smaller retries exist. Orders have
  weight 20 (AI-CON-5). Confidence: high (read twice from the machine code).
- **AI-MIL-4.4 Failure and success.** No site **or no material**
  (`AI-construction.md` AI-CON-5.3 (1)): the attempt count rises and a
  smaller building is ordered while the level stays >= 0 (the same order with
  the next building, made at once; corrected 2026-10-27 night: no material
  was "ends the order", E-ai-024). Below 0: after no site the AI
  tries instead to upgrade an own military building in the order's cell (or
  within the order's radius in point mode; AI-MIL-4.4a); if one is upgraded the target counts as
  done and P is halved, else the target is cancelled and P rises by 1; after
  no material nothing happens: the target stays current and remembered, and
  the next run past the cooldown finishes it (3.2, without a penalty change
  of P). Success: P halved, cooldown 3 runs. Confidence: high (T18).
  **The level is worked out again at each failure (2026-11-15, E-ai-065: code reading, the heap
  dumps and the four-player recording E-trace-019):** "the level" of a re-order is not the level
  the order was made with: at every failure the handler computes the level of AI-MIL-4.3 anew for
  the order's place (the cell's origin in cell mode, the order's point in point mode) **as things
  stand at that moment** (the nearest enemy military building's distance, an underground resource
  within 4 squares, stock(stone) > 40), and the result is that new level minus the new attempt
  count. So a target that an enemy building comes within reach of during the retries drops to a
  smaller building, or to none, faster than its first level says. When the result is below 0 the
  order's record keeps the raised attempt count with no building, and no order is pending (seen in
  the heap dump of 4500: player 1's slot holds attempts 3, no building type, the time of the
  re-order of trace tick 4245). Confidence: high (the handler's call with the attempt count; the
  heap record; building rows equal to trace tick 7030 instead of 4680 with it). Test T28.
- **AI-MIL-4.4a The upgrade instead: which building, and its tests (2026-11-17 day, E-ai-073:
  code reading, the four-player recording E-trace-019, an experiment build).**
  - **The fields, in order.** Cell mode: the 256 fields of the order's 16 x 16 cell (AI-CON-1.1),
    row by row from the cell's top-left field (y from the cell's first row, x from its first column
    within each row). Point mode: the fields around the order's point in the ring order of
    `MAP-grid-geometry.md` MAP-GRID-3 (the point first), up to **the order's own radius** (12 for a
    target towards a position, 20 for a need without a cell, AI-MIL-4.1), not 6 fields as written
    before. Fields outside the map are skipped.
  - **The tests, per field.** A building stands on the field (its own field); it is the player's own;
    its type has a territory range; it can be upgraded (`MIL-building-upgrade.md` MIL-UPG-1.2:
    finished, no upgrade running, no builder still linked, a next type, the size fits); and the next
    type's construction costs (frame goods plus building goods, **once**) are in stock (stock(g) of
    `AI-construction.md` AI-CON-1.7: every good's stock >= its amount).
  - **What is not tested, unlike the 37-tick run of AI-MIL-4.5:** the building's level against the
    upgrade level of AI-MIL-4.3, and the doubled costs. So a guard house far from any enemy (upgrade
    level 0) is still upgraded here, with 3 planks and 5 stones in stock for its tower.
  - The first field that passes is upgraded (one upgrade command, as a player's); the walk ends there.
    None passes: the target is cancelled (P + 1), as above.
  - The upgrade then runs as any other (`MIL-building-upgrade.md` MIL-UPG-2): its levelling part
    requests the leveller in the building's first update after the command.
  - Confidence: high (the two walks and their tests read from the code and compared with the 37-tick
    run's; the case of T32 measured). Test T32.
- **AI-MIL-4.5 Upgrades (every 37 ticks, one military building per run in
  list order).** An own finished non-depot military building that can be
  upgraded (`MIL-building-upgrade.md` MIL-UPG-1.2: next type in the upgrade
  table, no upgrade running, no builder still linked to its construction, the
  bigger size fits) is upgraded when its level (barracks 0, guard house 1,
  tower 2, fortress 3) is below the upgrade level of AI-MIL-4.3 at its
  position and **twice** the target type's full construction costs are in
  stock (stock(g) of `AI-construction.md` AI-CON-1.7): the list of frame goods
  and building goods is merged with itself before the test, as for the orders
  of AI-CON-5.3 (1), so a guard house needs 6 planks and 10 stones for its
  tower (corrected 2026-10-27 night, E-ai-029, measured
  T19). The upgrade itself: `MIL-building-upgrade.md` (Q-165).
  Confidence: high.
- **AI-MIL-4.5a The run's list and cursor.** The list of 4.5 is
  the **world's military building list**, the same for every player: every
  building whose type has a territory range (headquarters, barracks, guard
  house, tower, fortress), **of every player, sites included**, in the order
  the buildings were created (a building enters it when it is placed, as a
  site or finished; `MIL-soldiers.md` MIL-SOL-4.1 uses the same list). Each
  computer player keeps its own cursor, 0 at the start. A run: if the cursor
  is not below the list's length it is set to 0; the building at the cursor
  is taken and the cursor rises by 1. Only if that building is the player's
  own and not of a headquarters type are the tests of 4.5 made; otherwise the
  run does nothing. So with k buildings in the list a building is checked in
  every k-th run, and a computer player spends runs on other players'
  buildings and on castles. Confidence: high (code E-ai-006, the
  village system's building list, filled when a building is placed;
  measured, E-trace-010: player 1's guard house 90300 on (138, 57), finished
  at counter 5501, list [castle 0, castle 1, 90300]: the run of 5508 takes
  castle 0 and nothing happens; the runs of 5545 and 5582 take castle 1 and
  90300, and 90300 is upgraded at 5582 (two object numbers in 5582, the
  leveller 90750 leaving the castle at trace tick 5583, as in an experiment
  build with this rule; one that checked only its own buildings upgraded it at
  5508). T15.
- **AI-MIL-4.6 Headquarters reserve (every 613 ticks).** With d the enemy
  distance at the headquarters: reserve per rank = (50 - d) x 15 / 50
  (integer) when d < 50, else 0; set for all five ranks. Confidence: high.
- **AI-MIL-4.7 Coins and garrison (every 653 ticks; profile coin
  arrangement).** Every own finished non-depot military building with enemy
  distance < 30 gets coin delivery on and the "move soldiers out" switch
  (`MIL-soldiers.md` MIL-SOL-4.6: the building asks for fewer soldiers and
  releases the rest) off; farther buildings get coins off and the switch on.
  Confidence: high.

### AI-MIL-5 Attacks (every 641 ticks)

- **AI-MIL-5.1 Budget (Q-077).** An attack counter of -1 means unlimited; 0
  would block the attack system; a positive value would be spent one per run.
  It is set to -1 when the AI player is created, saved and loaded with the
  game, and nothing else writes it: no profile value, script function or
  command exists for it. So attacks are never limited. Confidence: high.
- **AI-MIL-5.2 Target cell.** Without a target: over all territory cells
  where some player has threat, and all alive enemy players p with score in
  the cell: value = (threat of p's buildings under construction + 2 x p's
  score - threat balance) / (C + 1 + A). The highest value above -100 becomes
  the target: that cell, player p, radius 12. Confidence: high.
  **Which cells are looked at, and in which order (corrected 2026-11-16 day b; E-ai-003 addendum,
  code reading, and the fight game measured, high).** "Where some player has threat" is narrower
  than it reads: a cell is looked at only when **one of its eight neighbouring cells** (the cells
  at x - 1 .. x + 1, y - 1 .. y + 1 **without the cell itself**; cells off the grid count nothing)
  holds a nonzero threat (AI-MIL-1.1) **of the attacking player itself**, in its own military map.
  So a computer player only aims at cells next to its own military presence; the enemy's threat
  inside the target cell plays no part in this gate. The cells are visited **column by column**
  (x from 0, and for each x every y from 0), and for each cell the players 0 to 7 in order (alive,
  an enemy, and score in the cell not 0); a value replaces the best only when strictly greater, so
  of equal values the first visited stays. Test T31.
- **AI-MIL-5.3 Candidates.** Enemy buildings of p within distance 12 of the
  cell's top-left field that can be attacked (headquarters, or soldier limit
  above 0 and occupied) and are either in the player's explored map or have
  enemy distance < 13. They are ordered by the sum, over the owner's
  buildings within 10 rings, of `bp_setAiScore` / max(1, ring - 3),
  **highest first**; a candidate whose sum equals that of an earlier one is
  dropped (the ordered set keeps one building per value). Confidence: high
  (Q-079).
  **The enemy distance test is a signed "at most 12" (2026-11-16 day b; E-ai-003 addendum, code
  reading and the R6 heap, high).** A building whose enemy distance is still -1 passes it. The
  enemy **headquarters** always holds -1 (MIL-SOL-3.1 never updates it; read in the original's heap
  at counters 1000, 8000 and 8500 for player 3's headquarters on Keyhole), so an enemy headquarters
  within distance 12 of the cell's top-left field is **always a candidate, explored or not**; the
  same holds for a military building made since the last enemy-distance update. **Build it together
  with 5.2's neighbour gate:** this rule alone (with ours' old gate) moved the fight game's first
  difference from 8884 back to 2451, because ours then took player 3's headquarters cell from its
  first attack run; with the gate as well the difference moves forward to 9311 (T31).
- **AI-MIL-5.4 Decision per candidate b.** def = 10 for a warehouse-type
  target, else the soldiers present; r = min(5, strength(owner) / max(1,
  strength(us))); want = trunc(r x def x (owner's defend-outside setting x
  0.5 + 1) x 1.5); need = max(1, want) + C of the target cell. In mode Safe
  the candidate is skipped unless want <= soldiers in stock. avail = the
  soldiers the player can send against b (MIL-ATK-1.3). Attack if avail > 0
  and (mode Aggressive or need <= avail), with count = min(need, avail)
  (Safe), min(2 need, avail) (Normal), avail (Aggressive), strong soldiers
  first; the first attack ends the run and b becomes the current target.
  Confidence: high.
  **The want, corrected (2026-11-23, E-ai-085: code reading and an experiment build; high).** The
  factor 1.5 and the truncation above hold only in mode **Safe**. Let
  `x = r x def x (owner's defend-outside setting x 0.5 + 1)`. In modes **Normal** and **Aggressive**
  `want = trunc(x + 0.5)` (x rounded to the nearest whole number, halves up; **no factor 1.5**). In
  mode **Safe** the stock test compares the unrounded `x x 1.5` with the soldiers in stock (the
  candidate is skipped when `x x 1.5` is greater), and then `want = trunc(x x 1.5 + 0.5)`. need =
  max(1, want) + C as before. Example (T34): r = 52 / 49, def 3, setting 0.5: x = 3.98, want 4 (not
  trunc(5.97) = 5), need 4, count min(8, avail 10) = 8. T33 is unchanged by it (def 2: x = 2.26, want
  2, need 2, count min(4, 3) = 3; with def 3, the soldier list of E-ai-078, x = 3.39, want 3, need 3
  <= 3, count 3 as well).
  **"The soldiers present" (2026-11-21 day, E-ai-077: the war recording, its heap dump and an
  experiment build; high for the measured case):** def counts, besides the soldiers inside, a
  soldier of the building's list who is on his step in from the building's flag (the last walk of
  `MIL-soldiers.md` MIL-SOL-4.7a, from the flag field into the building), from that step's start;
  a soldier still walking to the flag does not count. Test T33. **Answered from the code (2026-11-21
  pm, Q-1076, E-ai-078):** def is the **number of soldiers on the building's soldier list**, with no
  test of where they are; a soldier is on that list from his arrival at the building's flag (where
  his step in starts) until he leaves the building's service (sent home, dead, released), and a
  soldier sent out to attack stays on it. It is not the building's own "soldiers present": every
  other rule that asks for present soldiers (the defenders, `MIL-attack-combat.md` the sendable
  soldiers of MIL-ATK-1.3, the take-over test of `MIL-conquest.md` MIL-CON-1.2, coins and
  promotion) counts only a soldier who stands on the building's field with no step under way and
  no attack or fight; the soldier on his step in does not count there. Confidence: high (code)
  for the list and for the other rules' test; medium for the moment he joins the list (from the
  recording: counted while on his step in, not while walking to the flag).
- **AI-MIL-5.5 Results.** While the current target still exists, is not
  ours and has attackers on the way, the system waits. Target gone or taken:
  C and A of the cell are halved; attackers gone but target still enemy: C +
  1. No candidate attacked: A + 1, and (if the attack system has no pending
  expansion request) an expansion target of kind 1 at the last candidate's
  position. Then the target is cleared and a new one chosen next run.
  Confidence: high.
  **Precisions (2026-11-16 day b; E-ai-003 addendum, code reading, high):**
  - **C and A are whole numbers** (unsigned counters of the cell's territory record), and
    "halved" drops the remainder: 3 becomes 1, 1 becomes 0. They enter 5.2's value and 5.4's
    need as those whole numbers. (Keeping them as fractions, 1.5 after 3, changes the target
    choice and the need from the first halving of an odd value.)
  - **"The last candidate"** is the last candidate in 5.3's order that reached the need test
    of 5.4 and was not attacked. In mode Safe a candidate skipped because want is above the
    soldiers in stock is **not** a last candidate (it does not move the position); in modes
    Normal and Aggressive every candidate reaches the test. When no candidate reached it
    (no candidates at all, or every one skipped by Safe), **no expansion target** is set; A
    still rises by 1 in both cases.
  Test T30.

### AI-MIL-6 Soldiers, weapons, coins, catapults

- **AI-MIL-6.1 Recruitment warehouse (every 409 ticks).** Among the
  **depots** (the depot lists of the cells, AI-CON-1.1: storehouses and the
  headquarters) of the cells on the home landmass (the cell's landmass equal
  to the home landmass, `AI-construction.md` AI-CON-1.1b (b), (c); not each
  depot's own field) the finished one with the highest 2 x
  enemy distance - (S + B) is chosen (ties keep the earlier; an unfinished
  one never, and it asks nothing). S and B are **stored ware route costs**
  (`TRN-carriers-transport.md` TRN-CARRY-5.3a, kind 1, asked **from the
  depot's flag**): for every smithy and every brewery the computer player
  has recorded, **finished or still a construction site**, whose flag lies
  in the depot flag's ware network, the cost from the depot's flag to that
  building's flag is asked (cell by cell in the cell system's order, each
  cell's buildings in their order); S is the lowest smithy cost and B the
  lowest brewery cost, each starting at 100, so a cost above 100 counts as
  100 and none (or none connected) gives 100 (corrected 2026-10-27 b: before,
  a fresh road length to the nearest finished one). Because the store keeps
  every pair it is asked, this run fills it with depot-first pairs that the
  order matching later reads (ECO-REQ-5.3, T22). Then, if one was chosen, cell by cell in
  the cell system's order, every finished harbour of the cell and then every
  finished depot of the cell other than the chosen one, on any landmass, gets
  the warehouse "out" flag (`ECO-distribution.md` ECO-DIST-1.1) **set** for the
  wares of the soldier's cost list (sword, shield, beer), and last the chosen
  one gets it **cleared** for them, so these wares drain into it and it
  recruits. Each warehouse touched gets a flag record for each of the three
  goods if it has none yet (find or create, "stop" untouched, new records with
  "stop" clear): this is what gives the computer player's castle its three
  records at counter 403 of E-trace-006 (ECO-DIST-1.7c (b)), the castle being
  its only depot then. The helper of the cost list is not a ware and is
  skipped. With no finished depot on the home landmass nothing is touched
  (Q-166c, Q-167; Q-264). Confidence: high (E-sim-007; the castle's records measured); medium for the order of the
  touched warehouses (no recording with two depots).
- **AI-MIL-6.2 Weapons (every 631 ticks, not during need stage 1).**
  (Corrected 2026-10-27 b, Q-283: the readiness stop.) First every building of
  the player whose product is sword (the smithy; in any state, in the order of
  the computer player's building list) is looked at and counted. It is *ready*
  when **every need tied to it** (`AI-construction.md` AI-CON-2.5, the needs
  whose consumer it is) **has a production** and its worker is in it. A
  production exists only from its site's placement (`AI-construction.md`
  AI-CON-3.3): a need whose producer is only ordered, or whose producer's site
  is not placed yet, has none, so its consumer is not ready (corrected
  2026-10-30, T27: "a producer planned for it" was read as an ordered one); a ready smithy's productivity (ECO-OUT-3.11) is added
  to a sum. If some smithy is not ready, or a ready one has productivity < 0.1,
  the check **ends here**: when a ready one with productivity < 0.1 was met (the
  last such in list order), each input of its product (iron, coal) **of which
  that building's own input store holds none** (not the player's stock;
  corrected 2026-10-28 night, E-ai-046: read from the building's own store)
  gets a need tied to it (an existing need of that input without a consumer is
  taken, else a new one is made); otherwise nothing happens. The productivity
  is the float32 value of `ECO-outdoor-workers.md` ECO-OUT-3.11: one that has
  fallen to "0.1" by steps of 0.05 from 1.0, 0.75 or 0.5 is 0.0999998 and
  counts as below 0.1 (measured T24). Only when every
  smithy is ready with productivity >= 0.1 (or there is none) does the check go
  on: if smithies < the profile's maximum smithies and `now - t0 > 600 s x k`
  (k starts at 1 when the computer player is activated, t0 at 0): first
  `t0 = now - 600 k` and `k = k + 1`; then, when the mean productivity (the sum
  over the count; with no smithy there is no mean and only the soldier test
  counts) >= 0.9 or soldiers in stock < trunc(S / 48) / max(1, sword
  producers), `k = k + 1` and a sword need is created (a new record, tied to
  the home building). So the next pass comes 600 s after a pass without a need
  and 1200 s after one with a need (Q-166d). A smithy whose iron need never gets
  a producer (no iron smelter planned) therefore stops every later pass: no
  sword need is made after its site is placed (T20). Confidence: high (code
  E-ai-034; measured T20).
- **AI-MIL-6.3 Coins (every 647 ticks, need stage 4).** Same scheme for
  mints (inputs gold, coal; profile maximum mints), with the same readiness
  stop over the player's mints; a coin need when the mean mint productivity
  >= 0.9 (k then rises by 2 in all; no soldier test); with no mint there is no
  mean and no coin need. Confidence: high (code E-ai-034; no recording
  with a mint).
- **AI-MIL-6.4 Catapults (every 5 ticks; profile catapults).** The territory
  grid is scanned max(1, columns x rows / 600) cells per run (1 on a 256 x
  256 map: 32 x 32 = 1024 cells; set when the computer player is activated,
  Q-166e); a cell scores (2 x threat balance +
  own score) / max(0.01, near allocation) / (K + 1)^2 when the player has
  buildings there, stock(stone) >= 20 and some cell has a stone producer.
  After a full pass the best cell above 200 gets a catapult order (point
  mode, radius 8, weight 20) and all own catapults are switched on when
  stock(stone) > 20, else off. Confidence: medium-high; the Strong (Hard)
  profile's first two catapult sites are measured (T21: none for Weak, whose
  profile has catapults off).
  **No scan while a catapult order exists** (added 2026-10-29, E-ai-049,
  measured T25): from the tick the order is filed until its execution ends
  (placed or failed), the runs are skipped and the cursor stands still, so every
  later pass ends that many ticks later. Within a run the cursor first moves one
  cell on (column, then row) and then scores the cell it reached. The pass ends,
  and the order is filed, when it wraps past the last row. The best score of a pass
  starts at 200 and only a strictly higher one replaces it.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| military cell | 8 | fields | high | E-ai-003 |
| threat balance weights | 1 for (x + 1, y + 1), 0.5 for the centre and the other seven (corrected 2026-10-29) | | high | E-ai-003, E-ai-049 |
| default AI score of a building type | 20 (threat 0) | | high | E-ai-049 |
| settings countdown | 50 runs (2650 ticks) | | high | E-ai-003 |
| allocation factors | 8/3 (shrink), 3.3333 x 1.25 cap 4 (grow), medium = 0.3 near | | high | E-ai-003 |
| soldiers threshold | S / 128 | | high | E-ai-003 |
| site radius by kind | 12 / cell / 20 | fields | high | E-ai-003 |
| cell value | 100 / (4 d + max(1, threat)) / (P + 1), threat balance < 40, full scan | | high | E-ai-003, E-ai-005 |
| level base | d <= 18: -1 (creation) / 2 (upgrade); d > 18: -2; d = 24: 1 | | high | E-ai-005 |
| level bonuses | +2 resource within 4 squares, +1 stone > 40; upgrade: +1 soldiers > 30, +1 own building within < 5 | | high | E-ai-005 |
| attack budget | -1 (unlimited), never changed | | high | E-ai-005 |
| candidate order | surrounding score, highest first, equal scores dropped | | high | E-ai-005 |
| cooldown after placement | 3 runs | | high | E-ai-003 |
| headquarters reserve | (50 - d) x 15 / 50 | soldiers | high | E-ai-003 |
| coin distance | 30 | fields | high | E-ai-003 |
| attack target value | (tc + 2 s - balance) / (C + 1 + A), min -100, radius 12 | | high | E-ai-003 |
| attack need | trunc(min(5, r) x def x (o x 0.5 + 1) x 1.5), def 10 for depots | soldiers | high | E-ai-003 |
| recruit warehouse score | 2 d - (S + B); S, B the lowest stored ware costs from the depot's flag to a smithy / brewery (sites too), each from 100 | | high | E-ai-036 |
| weapons threshold | S x 0.0208333 / smithies, productivity 0.9 / 0.1, 600 s x k | | high | E-ai-003 |
| catapult score | (2 balance + score) / max(0.01, near) / (K + 1)^2 > 200; stone 20 | | medium-high | E-ai-003 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | profile Normal, medium 0.3, 5 soldiers in stock, S = 128 | settings run with countdown at 0 | 5 > 1: f = 1.25, near 1.0, medium 0.375; defense type 1 |
| T2 | no soldiers in stock, medium 0.3 | settings run | near 0.8, medium 0.24 |
| T3 | site with enemy distance 10, no resources, first attempt | creation | level -1 -> max(0, -1) - 0 = 0: barracks |
| T4 | site with enemy distance 30, underground coal within 4 squares, stone 10 | creation | -2 + 2 = 0: barracks; after one failure -1: no smaller building, an upgrade nearby is tried instead |
| T4b | site with enemy distance 24, underground coal within 4 squares, stone 10 | creation | 1 + 2 = 3: fortress; after one failure 2: tower |
| T4c | site with enemy distance 30, no resources, stone 50 | creation | -2 + 1 = -1 -> 0: barracks |
| T5 | guard house at enemy distance 12, 35 soldiers in stock, an own finished building within 4, stone 50 | upgrade check | 2 + 1 + 1 + 1 = 5 > 1: upgraded if the tower's costs are in stock |
| T13 | home cell A (balance 10, own score 5, no site under construction, some player without buildings) and neighbour B (balance 50, threat) | best cell, target 16 fields from A, 8 from B | B fails (balance >= 40); A: 100 / (4 x 16 + 1) / 1 = 1.54; A chosen although B is nearer |
| T14 | candidates with surrounding scores 30, 50, 50, 10 | ordering | visited 50, 30, 10; the second 50 is dropped |
| T6 | HQ with enemy distance 20 | castle settings | reserve 9 per rank; distance 60: 0 |
| T7 | target cell: enemy score 40, construction threat 0, balance 10, C = 1, A = 2 | selection | value (0 + 80 - 10) / 4 = 17.5 |
| T8 | mode Safe, target guard house with 2 soldiers, r = 1, owner defend-outside 1.0, 4 soldiers in stock, avail 5, C = 0 | execution | want = trunc(2 x 1.5 x 1.5) = 4 <= 4; need 4 <= 5: attack with 4 |
| T9 | mode Aggressive, same but 1 soldier in stock, avail 2 | execution | attack with 2 |
| T10 | mode Normal, need 3, avail 10 | execution | attack with 6 |
| T11 | smithy productivity 0.05, iron stock 0 | weapon run | an iron need attached to the smithy, no sword need |
| T12 | 1 smithy at 0.95, max smithies 2, S = 960, 5 soldiers in stock, 600 s passed | weapon run | trunc(20) / 1 = 20 > 5 and productivity >= 0.9: sword need created |
| T15 | AI-MIL-4.5a: E-trace-010, the world's military list [player 0's castle (29, 51), player 1's castle (139, 51), player 1's guard house 90300 (138, 57), placed at counter 1101, finished at 5501]; player 1's cursor such that its run of counter 5508 takes the first entry | upgrade runs of 5508, 5545, 5582 | 5508: player 0's castle, nothing; 5545: player 1's castle (headquarters type), nothing; 5582: 90300 is checked (level 1 below its upgrade level, costs in stock) and upgraded: its request and its leveller take object numbers 90749, 90750 in 5582, the leveller walks out of the castle's door from trace tick 5583 (measured) |
| T16 | AI-MIL-4.2, 4.1: E-trace-010, player 1's military building run of counter 5592 with the current target of kind 1 towards (116, 48) (`AI-construction.md` AI-CON-3.5); home field (139, 51), stored continent index 12; the top-left fields of the military-map cells (16, 6) (128, 48), (16, 5) (128, 40) and (16, 7) (128, 56) hold the stored indices 88, -1 and -1; the player's military threat in (17, 6) (castle) and (17, 7) (guard house 90300) | the best cell | (16, 6), (16, 5) and (16, 7) are not entered by the search; among the qualifying cells reached (16, 8) (top-left (128, 64), d = 20, value 100 / 80) is best: a guard-house order at (128, 64), radius 12, whose site (131, 59) is placed at counter 5898 (measured). Entering cells by a flood of walkable fields instead takes (16, 6) (d = 12): its point (128, 48) lies in a pocket of the coal mountain closed by ElevationBlocked fields, no candidate gets a route (AI-CON-5.4) and no site is found |
| T17 | AI-MIL-3.1, 3.2, 4.1, `AI-construction.md` AI-CON-3.4, 3.5: E-trace-011 (two-player game, player 1 Weak); the cell system's coal target (116, 48) made at 484.72 s (counter 4847), the waiting list then [iron ore need 90490, coal need 90484]; the target taken at 5330; the guard house (131, 59) for it placed at counter 5898 | the slot and the next targets | the cell system's slot keeps kind 1, (116, 48), 484.72 s and the coal need stays waiting without a cell in the heap dumps of 5000, 5500, 6000 and 6500 (no coal mine is ordered: the woodcutter (140, 61) at counter 6335 is the next building, measured); the military runs of about 5903, 6214 and 6525 count the cooldown down, the run of 6836 finishes the target; the cell system's run of 6847 takes the first waiting entry, the iron ore need: target (116, 36) at 684.78 s, the waiting list [coal, iron ore] (heap dump 7000). Clearing the slot when the target is taken (old rule) re-targets the coal need at 5376, gives it the castle's cell at 5962 and places a coal mine (131, 55) at 6330 |
| T18 | AI-MIL-3.2, 4.1, 4.4, `AI-construction.md` AI-CON-5.3 (1), 1.7: E-trace-011, player 1; the crowded target (144, 32) of 533.3 s taken after the coal target's finish (6836); the castle's planks 3 from counter 7172 (stock(plank) 3 from 7193), 2 from 7381 | the military run of 7147 and the orders after it | 7147: a guard-house order for the crowded cell (9, 2) (cell mode); taken at 7194, no material in 7195 (planks 3 < 2 x 2); at once the barracks order (attempt 1; the military order slot holds its time 719.47 s in the heap dump of 7500), taken at 7294, no material (3 < 2 x 2); level below 0: the target stays; the run of 7458 finishes it (heap dump 7500: attempt count 2, the remembered target (144, 32) cleared, the cell system's iron ore target (116, 36) taken by the expansion run of 7476, not yet current); the run of 7769 orders a guard house at the point (120, 56) (radius 12) for it, taken at 7794, no material, then the barracks (its time 779.46 s in the slot in the heap dump of 8000), no material: the target stays (heap dump 8000: current (116, 36), remembered, attempt count 2). Treating no material as the end of the order (old rule) leaves the crowded target current until its no-site chain cancels it, and with single costs and site materials counted the experiment build places a coal mine at (131, 55) in trace tick 7931 that the original never builds (buildings first differ 7931 -> 10026 with all three rules) |
| T19 | AI-MIL-4.5, MIL-UPG-1.2: E-trace-011, player 1's guard house 90785 (131, 59), finished at counter 9943 (its builder 90795 walks home from 9944, arrives 10398); the upgrade level there is 2 (enemy far -2, underground resource +2, soldiers 50 > 30 +1, stone 49 > 40 +1, no warehouse within 5); the upgrade run visits it every 185 ticks from 9984 (5 entries; 222 from 6 entries); the castle's planks (stock table) 5 at 9984, 10169, 10354, 4 at 10539, 10724, below 3 from 10911, 7 at 17495 | the upgrade runs | no upgrade at 9984 .. 17273 (planks below 6; at 9984 also the builder still linked: the dump of 9500 holds builder 90795 in the guard house's construction, the dump of 10000 none); upgrade at 17495 (7 planks >= 6, 68 stones >= 10), its leveller requested at 17533 (measured). With single costs (3 planks, 5 stones) an experiment build upgrades it at 9984 and its leveller walks from the castle at 10015, where the original makes the harbour's carrier 91073 (the settlers' first difference); with doubled costs it does not upgrade there (its later upgrade, 19049, follows its own stocks) |
| T20 | AI-MIL-6.2 (Q-283): E-trace-011, player 1 (Weak, maximum smithies 2); its smithy 90481 on (137, 55) placed at counter 3199, finished, worker in, productivity 0.95 (heap dump 12000: 0.95 after 7 decays; 12500: 1.0); its needs iron 90483 (no cell, no production) and coal 90484 (cell 90181, no production) in the heap dumps of 12000 .. 13000 | the weapon passes up to 20 000 | no sword need record at all after the stage-1 one (90465 at counter 3009; the objects table has no need from 12027 to 20 000). Without the readiness stop the pass of counter 12614 (now 1261.3 s, k = 2, t0 = 30.5 s) finds the mean 0.95 >= 0.9 and makes a sword need, whose missing iron input then gets a need without consumer (AI-CON-3.2): two numbers too many from the castle's helper of trace tick 12718 (91282 recorded); with the stop an experiment build's numbers equal the original's at every sampled tick through 15 000 |
| T21 | AI-MIL-2.1, 2.3, 2.2, 6.4, `AI-overview.md` AI-1.2 (the Strong profile, shown as Hard in the set-up): E-trace-012 (MP_2P_Storm_Coast, player 1 strength 2, profile 3), its autosave E-sav-005 of counter 18042 (`Player Military` records, FMT-SAV-3.3) and buildings table | the profile's military values as the game holds them, and its catapults | player 1: attack strength 1.0, intercepting (defend outside) 0.5, defense type 1 (mode Normal leaves the default 1; Safe would set 0), allocation near 1.0 and medium 1.0 (grown from 0.3 by 2.2), far 0.1, recruit count 10; player 0 (human, untouched): near 1.0, medium 0.3, far 0.1, attack 1.0, intercepting 1.0, type 1, recruit 10. Catapult sites (`catapult`, a1445ef3): (136, 49) at counter 4923 (R3, every tick; E-trace-012 samples every 10 there) and (133, 59) at 17276; the Weak game (E-trace-011) has none to 20 000. The coin rules (maximum mints 3, coin arrangement on) and attacks do not occur by counter 20 000 (no mint, no attack; the human's castle lies 110 fields away): they are left to the long Hard recording (TRACED plan R3) |
| T22 | AI-MIL-6.1, TRN-CARRY-5.3a: E-trace-011 (player 1, Weak) and E-trace-012 (player 1, Hard), the castle's flag (140, 50), heap dumps of the route store | the recruit runs' stored pairs and their use by the matching | Weak: the smithy 90481 (flag (138, 54)) and the brewery 90561 (flag (137, 50)): the run of counter 17990 asks castle -> (138, 54) = 48 and castle -> (137, 50) = 24 (the store at 18000 holds both, castle first); after the store was emptied, the run of 18399 asks 30 and 6 (the store at 19000 and 19500: castle -> (138, 54) 30, castle -> (137, 50) 6). The matching of 19888 compares, for the brewery's water order, the castle (cost 6, read from that pair) with the free water at (137, 52) (cost 12): the castle wins and its helper brings water (settlers point 19889). Hard: the smithy **site** whose flag is (137, 52) counts: the run of 3675 asks 39, the run of 4084 asks 8 (the store at 4500 holds castle -> (137, 52) = 8). Experiment build: Weak dense pair buildings and generator equal to 20 000 (from 19895), the 150 000 pair's generator 19895 -> 21183; counting finished buildings only leaves the Hard pair at settlers 4210 / buildings 5160 / generator 4410, counting the sites moves it to 4770 / 5593 / 5665 |
| T23 | `AI-overview.md` AI-1.2 (Strong / Hard), AI-MIL-6.2, 6.3, 6.4, the coin arrangement, MIL-ATK: E-trace-013 (R3: MP_2P_Storm_Coast, player 1 Hard, every tick from counter 0 to 150011, heap dumps every 500; the same game as E-trace-012, its generator state, object counter and draw index equal at all 20 001 ticks of E-trace-012) | the long-run profile rules | smithies (b779ab83) placed at counters 3286, 52726 and 97387; mints (bc203663) at 27327 (122, 62), 46724 (146, 34) and 84935 (139, 22); catapult sites (a1445ef3) at 4923 (136, 49), 17276 (133, 59), 80425 (113, 55), 83242 (122, 34) and 88175 (142, 15); coins (499d4c63): the first in the player's stock at 37247, in the storehouse 93353 at 37613, in the barracks 96422 at 52371 (then 94359, 98374, the harbour 90366, the castle 90180, ...); no attack and no conquest: player 0 keeps only its castle on (29, 51) to the end, player 1's expansion turns south over the sea (harbours (52, 138) at 86545 and (22, 136) at 94989) |
| T24 | AI-MIL-6.2 (the idle smithy's inputs): E-trace-014 (R4, the Weak game), player 1's smithy 90481 on (137, 55), ready, its statistics productivity decayed to "0.1" (float32 0.0999998); iron and coal 0 in the player's stock, coal in the smithy's store and no iron (so in our build, with the same deliveries; the store is not in the tables); its needs iron 90483 and coal 90484 (with productions) | the weapon pass of counter 24603 | one new need, iron 92322 for the smithy (objects table; heap dump 25000), none for coal. Reading the value as exactly 0.1 makes no need (an experiment build: the object numbers fall one behind from 24603 and the Weak pair's settlers first differ at 24967); the float with the player's stock makes needs for both (one number ahead); the float with the building's store: settlers first differ at 25149; the Hard pair and the Weak pairs to 20 000 unchanged |
| T25 | AI-MIL-6.4 (the paused scan), AI-MIL-1.1 (the default score): E-trace-013 (R3, the Hard game), player 1 (22 x 22 cells, one per run). The scan state in the heap dumps: cursor index 416 at 4500, 16 after the wrap at 5000, then exactly 100 cells per 500 ticks until 17000 (index 480), and 45 at 17500 (51 runs fewer). The orders: the first catapult order is filed at about counter 4838 and executes to 4923 (16 or 17 runs missing). The second is filed at about 17016 (order time 1701.632 game seconds) and placed at 17276, which accounts for the 51 runs missing. In that pass the best cell is (16, 7) with 210: own score 510 (brewery 60, locksmithery 100, two woodcutters 50 each, coal mine 150, donkey breeding 80, guard house site 20), threat 30, balance -150 with the weights of 1.2 | the second catapult | the catapult site on (133, 59) at counter 17276 (recorded: buildings table; heap dumps 4500 .. 17500: scan state, military map). Without the pause the pass ends at 16939. With the guard house scored 0 the cell has 190, with the centre weighted 1 it has 180, and with both 160. In each case there is no order in that pass, and the Hard game against R3 differs from buildings 17277. With the three rules: buildings 18575, generator 18919 (R3 buildings 18580, generator 18919; experiment build; the Weak pairs unchanged) |
| T26 | AI-MIL-1.2 (the weights): as T25, the first catapult pass (filed at about counter 4838) | the best cell | (17, 6) with 350: scan state of heap dump 4500 (best 350.0, cell (17, 6)). The experiment build gives 350 with the weights of 1.2 and 340 with the centre at weight 1 |
| T27 | AI-MIL-6.2 (readiness: a production only from the placement): E-trace-013 (R3, the Hard game), player 1's smithy 90597 on (136, 53), finished, worker in, productivity below 0.1 (below at the pass of 23972 already, and 0 in our build at 24603); its needs iron 90599 (production 92841), coal 90600, and the iron need 93243 made by the weapon pass of counter 23972 (the smithy's store had no iron) | the weapon pass of counter 24603 | no new need (the objects table: the smithy's next iron need is 93509 at counter 25234, the pass after). Heap dump 24500: need 93243 has cell 92155 and no production (-1); its smeltery 93384 is placed at counter 24776 (production 93385). Our build had linked 93243 to the production record of a smeltery only ordered in that cell, counted the smithy ready and made a new iron need at 24603: object numbers one ahead from there (the R3 settlers' first difference at 24720 stays, its cause is elsewhere); counting only placed productions (experiment build) keeps the numbers equal through 24711 |
| T28 | AI-MIL-4.4 (the level anew): E-trace-019 (R6), player 1 (Hard); the fortress order of trace tick 3726 for the crowded cell (3, 3) (target (48, 48), level 4: no enemy building known, the 'distance 24' case, + 2 underground + 1 stone); no site at 4022, no material at 4245 (attempt 2, a tower), then the tower taken at 4494 | the failure at 4495 | the level at the cell's origin is worked out again: player 2's guard house (57, 29) now counts at distance 19, so -2 + 2 + 1 = 1, result 1 - 3 < 0: no further order (no guard house at (57, 61) at 4675); the record keeps attempts 3 and no building; the next crowded target (48, 64) gets its fortress order from the run of 4970 (the heap dump of 5000: attempts 1 after no material at 4995); player 1's next military building is the guard house (56, 57) of trace tick 7530 |
| T29 | AI-MIL-2.2 (countdown): a computer player whose allocation grows at run r (medium m to 0.3 f), soldiers in stock above S / 128 at every later run | the runs that change the allocation | r, r + 50, r + 100, ... (each change 50 runs after the one before; the countdown 50 after a change, 49 after the next run, ..., 0 at run r + 50, which tests and changes); R6 player 1: medium 0.586 -> 0.733 at the run of counter 7997, its fortress (56, 50) then wanting 7 and asking at 8000 and 8021. With 51 runs apart the fourth change would come three runs (159 ticks) later |
| T30 | AI-MIL-5.5 precisions: (a) a cell with C = 3, A = 5 whose target is taken; (b) mode Safe, candidates in order g1 (want 6), g2 (want 2), g3 (want 7), 4 soldiers in stock, avail 1 against each, C = 0, no pending expansion; (c) the same in mode Normal; (d) a run whose target cell has no candidate | the cell's counters; the expansion target | (a) C = 1, A = 2; (b) g1 and g3 skipped, g2 tested and not attacked (need 2 > 1): A + 1, expansion target at g2 (not g3); (c) all three tested, none attacked: expansion at g3; (d) A + 1, no expansion target |
| T31 | AI-MIL-5.2 (the neighbour gate, column order), 5.3 (the headquarters' enemy distance -1), 5.5: E-trace-019 (R6, Keyhole `MP_4P_Fight`, players 1 to 3 Hard), player 2 (headquarters (52, 24)) | its attack runs (every 641 ticks) and its expansion target | no cell passes the gate at the runs of counters 630 .. 7681 (no target, no expansion); at the run of counter 8322 (trace 8323) the best cell is (4, 2) of player 3 with value -10; its only candidate is player 3's headquarters (26, 24) (enemy distance -1, not explored), not attacked (need far above the soldiers available), so A + 1 and the expansion target "towards a position" (26, 24) (the original's heap at counters 8500 and 9000: current kind 1 (26, 24) since game time 832.14 s, the attack source); the guard house (12e67603) then placed on (42, 23) at counter 8573 as in the original. Experiment: the fight game's first generator difference 8884 -> 9311; the MP_2P_Storm_Coast pairs unchanged |
| T32 | AI-MIL-4.4a: E-trace-019 (R6, Keyhole `MP_4P_Fight`, players 1 to 3 Hard), player 2; its military order for the cell (3, 1) (fields x 48 .. 63, y 16 .. 31) taken at counter 11639 fails with no site at 11669 with attempts 0; the level worked out anew at the cell's origin (48, 16) is 0 (enemy distance 19, nothing added), so 0 - 1 < 0; in the cell stands its guard house 16515 on (51, 17), finished long before (a guard house, level 1), with stock(planks) 6 and stock(stones) 36 | the failure at 11669 | the upgrade instead: the guard house is ordered up to a tower (the first qualifying field row by row; costs 3 planks + 5 stones, once); its leveller request is filed in its next update, counter 11670 (object number 18159), and the target counts as done (P halved); each 21-tick pass then cancels that request and the next update files it again (11676, 11697, ..., 11928; MIL-SOL-4.4 step 1) until a leveller takes it at 11933; the building is a tower from counter 15674 (the recording's tables). With the 37-tick run's tests here (level below the upgrade level, twice the costs) nothing is upgraded at 11669, the request queue's phase moves by one and player 1's leveller for (49, 60) leaves the headquarters at 11685 instead of 11778. Experiment build (the rule switched on, every other rule as before): the war's settlers (animation aside) first differ at 11862 instead of 11672, buildings 12225 instead of 12067, generator 12169 instead of 11964 |
| T33 | AI-MIL-5.4, def: E-trace-019 (R6, the war), player 2's attack run of counter 14091 against player 3's guard house on (21, 20): one soldier inside (18865), 18867 on his step in from the flag (21, 19) since 14075 (inside at 14103), 18874 still walking to the flag; r = 47 / 52; C = 0 (heap dump 14000); 3 soldiers available | def, want, the count sent | def 2, want trunc(0.904 x 2 x 1.25 x 1.5) = 3, need 3, count min(6, 3) = 3: 16592 and 16600 of (31, 21), 18218 of (42, 23) (recorded; 16592 intercepted at once by player 3's 16156). Counting only the soldier inside (def 1): count 2, the war's generator first differs at trace tick 14092, settlers 14100, buildings 14320; with the rule generator 14799, settlers 14110, buildings 14860 (experiment build on 0.12.3 with the other rules of 2026-11-21; every Storm Coast pair unchanged). Counting the whole list (def 3): want 5 > 3, no attack |
| T34 | AI-MIL-5.4, the want (E-ai-085): E-trace-019 (R6, the war), player 2's attack run of counter 17937 against player 1's guard house on (58, 45): soldier list 3, r = 52 / 49 (strengths), player 1's defend-outside setting 0.5, C = 0, mode Normal; sendable (MIL-ATK-1.1): (58, 36) 8 (d 9, 9 soldiers), (57, 29) 1, (56, 22) 1, (52, 24) 0: avail 10 | want, need, count, who goes | x = 1.0612 x 3 x 1.25 = 3.98, want 4, need 4, count min(8, 10) = 8: the 8 of (58, 36) only (nearest level first, MIL-ATK-2.1), as recorded. With the factor 1.5 (want 5, count 10) (57, 29) and (56, 22) send one each and the war's generator first differs at trace tick 17938; with the rule 18003, buildings 18240 -> 18930 (experiment build on top of the rules of 2026-11-22 night; every Storm Coast pair unchanged) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ai-003 | code reading | `S2DNG.exe` | military map, threat balance, territory map |
| E-ai-003 | code reading | | military settings |
| E-ai-003 | code reading | | expansion targets |
| E-ai-003 | code reading | | military building creation |
| E-ai-003 | code reading | | upgrades, headquarters reserve, coins |
| E-ai-003 | code reading | | attacks |
| E-ai-003 | code reading | | recruitment warehouse, weapons, coins, catapults |
| E-ai-005 | code reading | | best cell search (AI-MIL-4.2) |
| E-ai-005 | code reading | | level table (AI-MIL-4.3) |
| E-ai-005 | code reading | | attack budget (AI-MIL-5.1) |
| E-ai-005 | code reading | | candidate order (AI-MIL-5.3) |
| E-ai-005 | code reading | see `MIL-soldiers.md` | move-out switch (AI-MIL-4.7) |
| data | script | `ai.lua`, `buildings.lua`, `Jobs.lua` | profile values, threat/score weights, recruit costs |
| E-ai-006 | code reading | | military map sums (AI-MIL-1.1); countdown start (2.2); cell test over the 8 neighbours (4.2); upgrade stock test (4.5); recruitment storehouse and out flags (6.1); weapon-need timing (6.2); catapult scan width (6.4) |
| E-sim-007 | code reading and recording | E-trace-006 `goodflags.csv.gz` | AI-MIL-6.1, Q-264 |
| E-ai-024, E-trace-011 | code reading and recording | heap dumps 7000, 7500, 8000 of E-trace-011 (the AI core: order slot, remembered target, attempt count; construction part: the last order's state 4) | AI-MIL-3.2, 4.1, 4.4 (T18) |
| E-eco-019, E-trace-011 | code reading and recording | heap dumps 5000 .. 7000 of E-trace-011 (player 1's cell-system slot and waiting list) | the source slot kept until finish or cancel, the delayed finish (AI-MIL-3.1, 3.2, 4.1, T17) |
| E-ai-029, E-trace-011 | code reading, memory reads of the running game, recording | heap dumps 9500, 10000 (guard house 90785's builder link; upgrade cursor 3 / 4), stocks table | AI-MIL-4.5 (T19), MIL-UPG-1.2 |
| E-ai-034, E-trace-011 | code reading, memory reads of the running game, recording | heap dumps 12000, 12500, 13000 (productivity node of 90481) | AI-MIL-6.2, 6.3 (T20) |
| E-ai-036, E-trace-011, E-trace-012 | code reading, memory reads of the running game, recordings | route stores of E-trace-011 at 18000, 19000, 19500 and E-trace-012 at 4500 | AI-MIL-6.1 S and B |
| E-ai-049, E-trace-013 | code reading and memory reads of the running game | heap dumps 4500 .. 17500 of E-trace-013 (player 1's catapult scan state in its military part, military map cells (16, 7) / (17, 6) / (17, 7)) | AI-MIL-1.1, 1.2, 6.4, T25, T26 |
| E-ai-065, E-trace-019 | code reading, heap dumps of the recording, an experiment build | (session 2026-11-15 night) | 4.4: the level computed anew at each failure (the failure handler's level call with the attempt count; the military order records in the heap dumps 4000 / 4500 / 5000; an experiment build with the rule: first differing building row 4680 -> 7030, generator 4873 -> 6635) |
| E-ai-073, E-trace-019 | code reading, recording, experiment build | E-trace-019 `objects` (the leveller requests for player 2's guard house 16515 from counter 11670), `buildings` (its type from 15674), heap dumps 11500 and 12000 (player 2's upgrade cursor 4 and 17, as ours) | AI-MIL-4.4a: the order's upgrade has no level test and checks the costs once; its point-mode reach is the order's radius (T32); `MIL-building-upgrade.md` MIL-UPG-1.4 |
| E-ai-067, E-trace-019 | recording, experiment build | E-trace-019 `objects` (player 1's soldier requests 16786 at 8000 and 16798 at 8021 for the fortress 14432 on (56, 50)), `settlers` (soldiers out of player 1's headquarters at 8002 and 8032, its builder at 8062); an experiment build's allocation log (ours countdown 3 at the run of 7997, medium 0.586, wanted 5); with 50 runs between changes: fight generator 8072 -> 8205, buildings 8280 -> 8580, other games unchanged | AI-MIL-2.2 countdown, T29 |
| E-trace-013 heap 24500 | a memory read of the running game | the AI core of player 1: need 93243 (iron, consumer 90597) cell 92155, production -1; iron productions 92841 (need 90599) and 92942 (need 90899) | AI-MIL-6.2 (T27) |
| E-trace-013 | live read of the running game | 2026-10-27 01:18 .. 04:15, player 1 Hard, every tick from 0 to 150011 with about 12 % snapshot gaps, heap dumps every 500 | T23 |
| E-ai-077, E-trace-019 | recording, heap dump, saves, experiment build | (session 2026-11-21 day): R6's settlers, stocks and heap dump 14000 (player 2's territory records), the Keyhole fight saves' player military records | 5.4 def counts a soldier on his step in (T33) |
| E-ai-078 | code reading | (session 2026-11-21 pm) | 5.4 def: the soldier list's size; the presence test of the other rules (Q-1076) |
| E-ai-085, E-trace-019 | code reading, recording, experiment build | (session 2026-11-23): the attack execution's want per mode; R6's settlers around counter 17937 | 5.4 the want (T34) |

## Open questions

- None open from this file. (Q-077 to Q-079 answered in E-ai-005; the
  d = 24 case of AI-MIL-4.3 is specified as read, its intent unknown.)
