# ECO-PROD: Production rules of workshops and mines

Status: draft (rules read from the binary; values from the scripts; no in-game measurement yet)
Owner question(s): plan step 3.1; part of Q-005 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Every building that runs the workshop cycle (`ECO-worker-cycle.md`: sawmill,
mill, bakery, brewery, slaughterhouse, pig farm, well, smelting works,
metalworks, smithy, mint and the four mines) has a small production state: an
on/off switch, a product switch and a counter. Before each work action the
building checks that it is switched on, has its worker and holds its inputs.
When the work action ends it consumes the inputs and orders them again. When
the worker picks the product up, the building decides which product it was:
the first, the second (alternating, or always the second when switched), or
for the metalworks one tool chosen by the player's tool priorities. Mines eat
one unit of any food per product and take ore from the ground. This file adds
those rules to the cycle of ECO-WORK and the orders of ECO-REQ-4.

## Rules

Terms: "product slot 0 / 1" = the good of `bp_setGoods` / `bp_setSecondGoods`
(`ECO-buildings-catalog.md` ECO-CAT-3); "stock" = the wares stored in the
building; "order" = a ware order of ECO-REQ-4. A building's worker is a
**miner** when its profession is `good_miner` (59B7633E); the four mines are
the only such buildings.

### ECO-PROD-1 Production state

- **ECO-PROD-1.1** Each such building has: `enabled` (bool), `switched`
  (bool) and `count` (integer). A new building starts with enabled = true,
  switched = false, count = 0. All three are part of the saved building.
- **ECO-PROD-1.2** Command "production on/off" (`toggle-production`,
  carries the building) **toggles** `enabled`. Turning it off cancels
  the building's open input orders that have no ware assigned yet
  (ECO-PROD-6.4); turning it on files input orders at once (ECO-PROD-6.1).
- **ECO-PROD-1.3** Command "switch production" (`switch-production`)
  **toggles** `switched`. The command itself does not check
  `bp_setSwitchableProduction`; the in-game window offers it for the
  shipyard only (the only switchable building, ECO-CAT-3.3).
- **ECO-PROD-1.4** A command naming a building that no longer exists does
  nothing.

### ECO-PROD-2 Inputs: what a building consumes

- **ECO-PROD-2.1 Workshops.** A building whose worker is not a miner consumes,
  per product, the cost list of **product slot 0** (`gp_addCost` of that
  good), every entry with its amount. For the smithy the shield (slot 1) is
  paid with the sword's cost list (they are equal). For the metalworks it is
  the cost list of the group `tools` (plank 1, iron 1), whatever tool comes
  out.
- **ECO-PROD-2.2 Mines.** A building whose worker is a miner ignores the cost
  list of its product. Per product it consumes **one food unit**: the first
  good of **fish, bread, ham (flesh)**, in that order, of which it holds at
  least one. The product's cost entry "food" (ECO-CAT-5) is not used by the
  engine.
- **ECO-PROD-2.3 Outdoor workers.** The woodcutter, quarry, forester, fisher,
  hunter and farm do not run this production step at all, so their product's
  cost list is never consumed: the hunter does not need pigs although its
  product `flesh` costs one pig.
- **ECO-PROD-2.4 Stone mine.** The stone mine is a mine by its worker, so
  ECO-PROD-2.2 applies: it eats one food unit per stone, although `stone`
  has no cost list. Its ore is the stone (granite) of the ground's resource
  layer (ECO-PROD-3.3).
- **ECO-PROD-2.5 Well.** `water` has no cost list, so the well consumes
  nothing and its production check (ECO-PROD-3.1 c) always passes. The
  production step does not look at any ground water resource (`water`'s
  per-field resource flag is commented out in the shipped `Goods.lua`).
  - **ECO-PROD-2.5a Placing a well checks nothing extra** (Q-052). A well is
    placed by the rules for small buildings alone (`ECO-building-placement.md`
    ECO-PLACE-3); no rule looks at ground water, a water resource or the
    well's product. The well's building type appears in the game only in the
    worker job dispatch, never in a placement check. Confidence: high.
  - **ECO-PROD-2.5b Ground water is "buildable ground".** For water, the game
    uses the field state ForBuilding (`FMT-s2m-map.md` FMT-S2M-4.4, bit 9)
    instead of the resource layer: (1) the geologist, on a field with
    ForBuilding, reports **water, amount 1** whatever the resource layer holds
    there (message "Found Water", the water sign of `signs.lua` for amount 1);
    on other fields he reports the resource layer's good and amount (coal,
    iron ore, gold, stone; nothing if empty). (2) A water resource can only be
    put into the resource layer on a ForBuilding field; other goods need
    ForMining, fish needs a Water field that has a Coast field close by (a
    short search outward from the field). The resource
    layer's water amounts have no effect on any production. Confidence: high.
  - **ECO-PROD-2.5c** So the help text ("wells must be built over sweet
    water") has no rule behind it in this build: a well works on any site
    where it may be built, at its normal rate (ECO-PROD-3).
- **ECO-PROD-2.6** Where the workshop cycle covers a building without a
  product (none in the shipped scripts), the check of ECO-PROD-3.1 c passes
  and nothing is consumed.

### ECO-PROD-3 Starting a work action

The worker's step ECO-WORK-3.1 ("wait for inputs") is, in detail:

- **ECO-PROD-3.1** The worker begins a cycle only while `enabled` is true.
  For a mine it also needs a current ore field (ECO-PROD-3.3); if there is
  none it does not begin. Having begun, it waits (checked once per tick) until
  the building **can produce**:
  - (a) `enabled` is true, and
  - (b) the building has at least one worker, and
  - (c) for a mine: at least one unit of food (fish, bread or ham) in stock;
    for any other building: every entry of product slot 0's cost list is in
    stock in at least its amount (an empty cost list passes).
  In the first tick in which it can produce, the work action of the
  production time starts (ECO-WORK-3.2).
- **ECO-PROD-3.2** Switching production off while the worker waits stops it
  from starting (a). Switching off **during** the work action does not stop
  it: the work ends, the inputs are consumed (ECO-PROD-4.1) and the product
  is carried out as usual, but nothing is re-ordered (ECO-PROD-4.2).
- **ECO-PROD-3.3 Ore of a mine.** A mine has a current ore field (a field of
  the map's resource layer, `FMT-s2m-map.md` FMT-S2M-4.5) or none. When the
  miner begins a cycle with a current ore field, one unit is taken from that
  field at once, **before** the wait for food of 3.1: the field's amount
  becomes `min(amount - 1, 7)`; if it becomes 0, the field's good is cleared
  (empty) and the mine has no current ore field from then on. The cycle that
  took the last unit still produces. (Release maps hold 7 units per field, so
  one field gives 7 products.) How a mine finds its next ore field (every
  101 ticks, nearest in flood order within its working radius 3) and its
  "No resources found." message: `ECO-food-mining.md` ECO-FOOD-2.

### ECO-PROD-4 End of the work action

- **ECO-PROD-4.1** In the tick the work action ends (ECO-WORK-3.2) the
  building consumes its inputs: a mine one food unit (ECO-PROD-2.2), any
  other building its cost list (ECO-PROD-2.1). A mine that holds no food at
  that moment consumes nothing (cannot happen in the shipped game, since 3.1
  c required a unit and only the miner takes from the stock).
- **ECO-PROD-4.2** If `enabled` is true at that moment, every consumed unit is
  ordered again at once, one order per unit (ECO-REQ-4.3). A mine re-orders
  the food good it ate.
- **ECO-PROD-4.3** The product does not exist as a ware yet; its kind is
  decided when the worker takes it (ECO-PROD-5).

### ECO-PROD-5 Which product: taking the product

- **ECO-PROD-5.1** When the worker starts taking the product (ECO-WORK-3.3,
  the start of the drop-time action), the building decides the product, then
  adds 1 to `count`:
  - if `switched` is true: product slot 1;
  - else, if the building has a product slot 1 and `count` is odd: slot 1;
  - else: product slot 0.
  `count` is the value before the increment. So a building with two products
  alternates slot 0, slot 1, slot 0, ... starting with slot 0; switching on
  gives slot 1 every time; switching off again resumes the alternation by
  the parity of `count`.
- **ECO-PROD-5.2** Smithy: sword, shield, sword, shield, ... (it is not
  switchable in the window, ECO-PROD-1.3).
- **ECO-PROD-5.3** Shipyard: both slots are `boat`, so this step always gives
  a boat; ship building in ship mode is another path (plan step 4.7).
- **ECO-PROD-5.4** If the chosen product is the group `tools` (D5AA9BDE), the
  product is the tool chosen for the building's owner by ECO-PROD-7.
- **ECO-PROD-5.5** At the end of taking (after drop time and pickup time) the
  worker holds one ware of the decided good, which it carries to the flag
  (ECO-WORK-3.4 on). If the decided good is "none" (id -1), no ware is made
  and nothing is carried. With the clamp of ECO-PROD-7.3 this cannot happen
  for tools in the shipped game.

### ECO-PROD-6 Input orders and stock limits

Refines ECO-REQ-4.2.

- **ECO-PROD-6.1 Filing orders.** "Order up to L" for a good means: file
  `L - in stock - open orders` new orders for it, if that is positive. When
  the building files its inputs (once when it becomes active with its
  worker, ECO-REQ-4.2, and every time production is switched on), it does so
  only if `enabled` is true, as follows:
  - a mine: fish, ham, bread, each up to **2** (fixed, independent of the
    stock limit 6);
  - the catapult: stone up to its stock limit (4);
  - any other building with a product: every good of product slot 0's cost
    list up to the stock limit (`bp_setStockLimit`). A building without a
    stock limit (default -1) files none.
- **ECO-PROD-6.2** Consequently stock plus open orders of each input stays at
  the limit while the building works (ECO-PROD-4.2 re-orders what was used).
  A mine keeps up to 2 of each food; it eats them in the order fish, bread,
  ham, so with all three delivered it uses fish first.
- **ECO-PROD-6.3** Wares already in stock stay when production is switched
  off; they are used when it is switched on again.
- **ECO-PROD-6.4** Switching off removes every open input order of the
  building that has no ware assigned yet. Orders whose ware is already
  assigned (on its way) stay; such a ware is delivered and stored.
  Switching on files new orders by 6.1, which counts those orders.

### ECO-PROD-7 Tool choice (metalworks)

Each player has one tool-priority record with 12 slots. Slot order, tool and
default weight:

| Slot | Tool | Default weight |
|---|---|---|
| 0 | gripper (tongs) | 0.1 |
| 1 | axe | 0.4 |
| 2 | saw | 0.2 |
| 3 | pickaxe | 0.2 |
| 4 | hammer | 0.4 |
| 5 | shovel | 0.5 |
| 6 | pot (crucible) | 0.6 |
| 7 | fishing rod | 0.7 |
| 8 | scythe | 0.8 |
| 9 | hatchet (cleaver) | 0.9 |
| 10 | rolling pin | 0.2 |
| 11 | spear | 0.3 |

- **ECO-PROD-7.1** The record holds per slot a weight and an accumulator
  (32-bit floats; accumulators start at 0), and an **exclusive slot** (none
  at the start).
- **ECO-PROD-7.2 Choosing.** If an exclusive slot is set, the tool is that
  slot's tool; nothing else changes. Otherwise: add each slot's weight to its
  accumulator (all 12, float32 addition); take the slot with the largest
  accumulator, the lowest slot number on equal values, provided the largest
  value is above 0; set that slot's accumulator to 0; the tool is that slot's
  tool. (If no accumulator is above 0, the result is "none", ECO-PROD-5.5.)
  The choice is made per tool produced, by any metalworks of the player, in
  the order the products are taken.
- **ECO-PROD-7.3 Commands.** `set-tool-weight` (slot,
  value) sets that slot's weight to the value, or to **0.001** if the value
  is not greater than 0.001 (a weight is never 0).
  `set-only-tool` (slot) sets the exclusive slot;
  `clear-only-tool` clears it;
  `reset-tools` restores the default weights and clears the
  exclusive slot but **keeps the accumulators**.
- **ECO-PROD-7.4** Weights, accumulators and the exclusive slot are saved
  with the player.
- **ECO-PROD-7.5** In the long run each tool's share is its weight divided by
  the sum of the weights (default sum 5.3: hatchet 17 %, gripper 1.9 %).

### ECO-PROD-8 What stops production (summary)

A workshop or mine produces nothing while any of these holds:

- **ECO-PROD-8.1** production is switched off (ECO-PROD-3.1 a);
- **ECO-PROD-8.2** it has no worker (b; the worker is requested by ECO-REQ-1.4,
  and needs its tool, ECO-CAT-6);
- **ECO-PROD-8.3** an input is missing in stock (c), for a mine: no food at
  all;
- **ECO-PROD-8.4** a mine has no current ore field (ECO-PROD-3.3);
- **ECO-PROD-8.5** the flag has no room for the product: the worker waits
  with the product before leaving (ECO-WORK-3.4) and does not start the next
  cycle meanwhile.

No other stop condition was found in the traced functions: in particular
nothing checks how many finished products of the good already exist.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| initial enabled / switched / count | true / false / 0 | | high | E-eco-008 |
| product choice rule | ECO-PROD-5.1 | | high | E-eco-008 |
| mine food order | fish, bread, ham | | high | E-eco-008 |
| mine food orders | 2 each of fish, ham, bread | units | high | E-eco-008 |
| catapult input | stone up to stock limit | | high | E-eco-008 |
| ore taken per cycle | 1, amount capped at 7 | units | high | E-eco-008 |
| tool slots, default weights | ECO-PROD-7 table | | high | E-eco-008 |
| minimum weight | 0.001 | | high | E-eco-008 |
| tool choice | ECO-PROD-7.2 | | high | E-eco-008 |
| off cancels only unassigned orders | | | medium (which order field means "assigned" read from use) | E-eco-008 |
| worker begins only while enabled; mine needs ore field | | | medium (retry behaviour of a refused begin taken from ECO-WORK-3.1) | E-eco-008 |

## Test cases

Level ground, room at the flags, production time 333, rest 55 unless stated.

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | new smithy, iron and coal in stock | take 5 products | sword, shield, sword, shield, sword |
| T2 | smithy after 3 products (count 3) | toggle switch, take 2, toggle back, take 2 | shield, shield (count 3, 4), then by parity of 5, 6: shield, sword |
| T3 | new player, default weights, one metalworks | first 12 tools | hatchet, scythe, fishing rod, hatchet, pot, scythe, shovel, hatchet, fishing rod, axe, hammer, scythe |
| T4 | as T3 | 510 tools | hatchet 80, scythe 79, fishing rod 67, pot 57, shovel 49, axe 39, hammer 39, spear 30, saw 20, pickaxe 20, rolling pin 20, gripper 10 |
| T5 | as T3 after 2 tools | `set-only-tool` slot 1, take 3 tools, `clear-only-tool`, take 1 | axe, axe, axe, then fishing rod (the accumulators did not move during the exclusive tools) |
| T6 | as T3 | `set-tool-weight` slot 9 to 0, first 3 tools | weight 0.001; scythe, fishing rod, pot |
| T7 | metalworks, plank 1 and iron 1 in stock | one product | consumes 1 plank and 1 iron; re-orders 1 plank, 1 iron |
| T8 | coal mine with an ore field, stock fish 1, bread 0, ham 2 | one product | fish consumed and re-ordered; ham stays 2 |
| T9 | coal mine with an ore field, no food in stock | worker begins | the field loses one unit at once; work starts only when a food unit arrives |
| T10 | iron mine on a field of amount 1 | one cycle | field emptied (good cleared); the product is made; the next cycle does not begin until a new ore field is set |
| T11 | stone mine, ham 1 in stock | one product | ham consumed; a stone is made |
| T12 | well, nothing in stock | run | works continuously (production 333 + gap 122), never orders anything |
| T13 | hunter | catch game | no pig ordered or consumed |
| T14 | sawmill working (work action running), 3 logs in stock, 3 orders open of which 1 has a ware on its way | switch off | the 2 unassigned orders are removed; the plank is made and carried out; the log used is not re-ordered (stock 2); the assigned log is delivered (stock 3); no next work action |
| T15 | T14 | switch on | orders up to 6: 6 - 3 - 0 = 3 new orders; work starts at the next cycle begin |
| T16 | new mine (worker arrived) | input orders | 2 fish, 2 ham, 2 bread (6 orders), not 6 of each |
| T17 | catapult, finished, worker arrived | input orders | 4 stones |
| T18 | geologist on a meadow field (ForBuilding) whose resource layer holds coal 7 | search | reports water, amount 1 (water sign); the coal is not reported |
| T19 | geologist on a rock field (ForMining) with gold 4 | search | reports gold, amount 4 |
| T20 | well placed on any small-building site, no water in the resource layer | run | works as T12 |
| T21 | ECO-PROD-1.2, 6.4 (switching on counts the orders on their way): E-trace-013 (R3, the Hard game), player 1's locksmith 90896 on (135, 58) (stock limit of planks reached with one plank on its way: plank 94737 sent by the castle at counter 33924, delivered 34625); production switched off at counter 34031 and on again at 34368 by the computer player's locksmith run (`AI-construction.md` AI-CON-10.2) | the orders filed at 34368 and what the castle sends | no new plank order: the plank on its way counts as an open order; the castle (18 planks) sends no plank to the locksmith before 34625 (recorded). Counting only orders without a ware files one, and the castle sends a second plank at counter 34375 (a helper at its door): the Hard game against R3 differs from settlers 34380; with the rule settlers 36240, buildings 37880, generator 38004 (experiment build; every other pair unchanged) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-008 | code reading | `S2DNG.exe` | the production state's begin / update: enabled check, mine ore field, can-produce, work action, end of work |
| E-eco-008 | code reading | | can produce: enabled, worker count, inputs; mine: any food |
| E-eco-008 | code reading | | consume at end of work: mine food order fish, bread, ham; re-order if enabled |
| E-eco-008 | code reading | | product choice at taking; tools group; no ware for id -1 |
| E-eco-008 | code reading | | production state constructor, set enabled, set switched, save |
| E-eco-008 | code reading | | input orders (mine 2 each, catapult stone, cost list up to limit); cancel unassigned orders |
| E-eco-008 | code reading | | ore unit taken from the resource layer, cap 7, cleared at 0 |
| E-eco-008 | code reading | | tool choice, set weight (min 0.001), defaults, constructor, save |
| E-eco-008 | code reading | | command executors (toggle) and command name table |
| E-eco-008 | code reading | | first input orders when the building is active with its worker |
| E-trn-005 | code reading | | geologist result: ForBuilding field -> water amount 1; messages per good (Q-052) |
| E-trn-005 | code reading | | resource put into the layer: water needs ForBuilding, fish Water, others ForMining; cap 7 (Q-052) |
| E-trn-005 | code reading | water id A9676263 and well id C2C7C303 in code | well id only in worker dispatch; no placement use (Q-052) |
| ECO-CAT | spec | `ECO-buildings-catalog.md` | costs, stock limits, workers |

## Open questions

- Mine ore search, donkey farm, shipyard, fisher, hunter and farm cycles:
  `ECO-food-mining.md` (ship site and launch still open there).
- Exact order fields behind "assigned" in ECO-PROD-6.4 (medium confidence).
