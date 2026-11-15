# ECO-PROF: Professions, tools and new settlers

Status: draft (values from `Jobs.lua`; rules read from the binary; no in-game measurement yet)
Owner question(s): plan step 3.2
Game build: 11757

## Summary

Every settler except the helper and the donkey has a profession that is made
from a helper and, usually, one tool. A warehouse makes a professional only
when one is requested and none is in stock: it takes one helper and the tool
from its stock. Tools come from the metalworks. New helpers are not made from
anything: every headquarters, depot and harbour slowly fills its own stock up
to 100 helpers. Donkeys come only from the donkey farm.

## Rules

### ECO-PROF-1 Professions and their cost lists

- **ECO-PROF-1.1** A profession is a good of type Settler in `Jobs.lua`. Its
  cost list (`gp_addCost`) is what a warehouse consumes to make one
  (ECO-PROF-2). Its rest time (`gp_setRestTime`) is used by the work cycles
  (ECO-WORK-3.9: rest + 1 ticks). The shipped table:

  | Id | Profession | Made from | Tool | Rest | Works in |
  |---|---|---|---|---|---|
  | DD2DAEBE | builder (`good_constructor`) | helper, hammer | hammer | - | construction sites |
  | 2740000E | leveller (`good_bulldozer`) | helper, shovel | shovel | - | construction sites |
  | 2C50533E | woodcutter | helper, axe | axe | 222 | woodcutter's hut |
  | 6EBA970E | forester | helper, shovel | shovel | 222 | forester's hut |
  | 8A3AA1EE | stonemason (`good_stonecutter`) | helper, pickaxe | pickaxe | 222 | quarry |
  | AB43100E | hunter | helper, spear | spear | 222 | hunter's hut |
  | 4AC589CE | fisherman | helper, fishing rod | fishing rod | 1111 | fisherman's hut |
  | 8D0BA75E | farmer | helper, scythe | scythe | 55 | farm |
  | 2195BC53 | carpenter (`good_cabinetmaker`) | helper, saw | saw | 55 | sawmill |
  | 862797A3 | baker | helper, rolling pin | rolling pin | 55 | bakery |
  | 290E6B9E | miller | helper | - | 55 | mill |
  | 0EAFFECE | brewer | helper | - | 55 | brewery |
  | 360B16E3 | pig farmer | helper | - | 222 | pig farm |
  | 35A82813 | donkey breeder | helper | - | 222 | donkey farm |
  | F0622D7E | butcher (`good_slaughter`) | helper, hatchet | hatchet | 222 | slaughterhouse |
  | 59B7633E | miner | helper, pickaxe | pickaxe | 55 | the four mines |
  | 9C0AE313 | smelter | helper, pot | pot | 55 | smelting works |
  | E59A65CE | locksmith | helper, gripper | gripper | 55 | metalworks |
  | 03A2B9F3 | smith | helper, hammer | hammer | 55 | armoury (smithy) |
  | FDF21163 | coiner | helper, pot | pot | 55 | mint |
  | AE71FBF3 | shipwright (`good_shipBuilder`) | helper, hammer | hammer | 222 | shipyard |
  | 3355C25E | geologist (`good_scholar`) | helper, hammer | hammer | - | sent to flags |
  | A07BF6FE | scout (`good_explorer`) | helper, spear | spear | - | lookout tower |
  | 182118D3 | boat carrier (`good_shipHelper`) | helper, boat | (boat) | - | water roads |
  | 15E86843 | private (`good_soldier0`) | helper, sword, shield, beer | - | - | military buildings |
  | BA0E7F83 | private first class | private, coin | - | - | military |
  | 466C6C23 | sergeant | private first class, coin | - | - | military |
  | 906014F3 | officer | sergeant, coin | - | - | military |
  | 559DDEA3 | general | officer, coin | - | - | military |
  | 7BFF15CE | helper | helper (itself) | - | 55 | carriers, well, catapult |
  | D4157C5F | donkey | nothing | - | - | land roads |

  The well and the catapult take a plain helper as their worker (ECO-CAT).
  Confidence: high (script values).
- **ECO-PROF-1.2** A professional keeps his profession for good; he is never
  turned back into a helper and his tool is never returned. Confidence:
  medium (no such path found).

### ECO-PROF-2 Making a professional

- **ECO-PROF-2.1 Only on request.** A warehouse never converts helpers on
  its own. When a request for a profession is served (ECO-REQ-3) by a
  warehouse that has none of that profession in stock, the warehouse makes
  one at the moment it sends: one of each good of the cost list leaves its
  stock and one settler of the profession is added and sent at once
  (ECO-REQ-3.3, 3.6). A warehouse that lacks any good of the list cannot
  make one. Confidence: high (`ECO-requests-dispatch.md`).
- **ECO-PROF-2.2 Special cases.**
  - The helper's cost list names the helper itself, so a warehouse without
    helpers cannot make one this way; helpers come only from ECO-PROF-3.
  - The donkey has no cost list and cannot be made in a warehouse; donkeys
    come only from donkey farms (`ECO-food-mining.md` ECO-FOOD-6) and the
    starting stock (ECO-HQ-2.1).
  - The boat carrier uses a boat (a ware made by the shipyard) as its
    "tool".
  - A private needs a helper, a sword, a shield and a beer. Promotions do
    **not** use the "previous rank + coin" cost lists: they are done by the
    military buildings' promotion run (`MIL-coins-promotion.md`). The cost
    lists of the higher ranks only matter if a warehouse is asked to send a
    soldier of a rank it does not have (ECO-REQ-3.3).
  Confidence: high for the lists; medium for the soldier cases.
- **ECO-PROF-2.3 Tools.** The tools are wares made by the metalworks, which
  chooses the next tool by the player's tool priorities
  (`ECO-production-chains.md` ECO-PROD-7). A tool in a warehouse is used
  only through ECO-PROF-2.1; the same tool serves several professions
  (hammer: builder, smith, shipwright, geologist; pickaxe: stonemason,
  miner; shovel: leveller, forester; pot: smelter, coiner; spear: hunter,
  scout). Confidence: high.

### ECO-PROF-3 New helpers

- **ECO-PROF-3.1** Every finished building whose type has the spawn flag
  (`bp_setSpawn`: headquarters, depot, harbour) runs a counter: +1 per tick;
  when it has reached 100 it is set to 0 and the building's stock of helpers
  is checked, so **one check every 101 ticks**. Confidence: high.
- **ECO-PROF-3.2** At a check: fewer than **100** helpers in that
  building's stock: **one helper is added**; more than 100: the helpers above
  100 are removed (the stock is set to 100); exactly 100: nothing. Helpers
  outside (on roads, at work, on their way) do not count. Confidence: high.
- **ECO-PROF-3.3** So each such warehouse adds at most one helper per 101
  ticks (about 10 s) while its own stock is below 100, and a stock above 100
  (for example the headquarters' start stock at the higher start-resources
  settings, ECO-HQ-2.2: 26 x 4 = 104, 26 x 8 = 208) is cut to 100 at the
  first check. The counter starts at 0 when the building is created; the
  first check is in its 101st update. Buildings are updated every tick from
  the **first tick of the game (tick 0)**, with no start delay, so for the
  headquarters of a new game the first check is in **tick 100** and then
  every 101 ticks (tick 201, 302, ...). A building created later is updated
  from the tick after it is created (it joins the building list; same
  tick if it is created before the building pass of that tick).
  Confidence: high for tick 0 (update loop has no gate; the tick counter is
  -1 in a new map and is raised before the updates, ECO-TIME-1.5); medium
  for buildings created during a tick.
- **ECO-PROF-3.4** There is no other growth: no food, houses or other goods
  are needed for new helpers. Confidence: medium (only this producer of
  helpers was found; the helper id appears in no other adding code).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| helper check period | 101 | ticks | high | E-eco-010 |
| helper target per spawn warehouse | 100 | helpers | high | E-eco-010 |
| helpers added per check | 1 | | high | E-eco-010 |
| spawn flag | `bp_setSpawn` (a flag of the building type) | | high | E-eco-010 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | new game, start resources setting 0 (26 helpers in the HQ), no helper sent | end of tick 100 | 27 helpers in stock (26 at the end of tick 99) |
| T2 | as T1, no helpers leave | 7,474 ticks (74 checks) | 100 helpers; no more after that |
| T3 | start resources setting 3 (208 helpers) | first check | stock cut to 100 |
| T4 | HQ with no woodcutter, 1 axe, 1 helper; woodcutter's hut requests a worker | send | HQ stock: axe 0, helper 0; one woodcutter sent |
| T5 | HQ with no woodcutter and no axe | woodcutter request | not served by this HQ ("Building without worker.") |
| T6 | HQ with 0 helpers | carrier request for a new road | not served until a helper is added by ECO-PROF-3 |
| T7 | depot with 0 helpers, HQ with 100 | 101 ticks | each gains (depot 1, HQ none) |
| T8 | no donkey in any warehouse, no donkey farm | donkey request (ECO-REQ) | never served |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-010 | code reading | `S2DNG.exe`: the helper refresh task, created with every building | helper check every 101 ticks, add 1 below 100, cut above 100 |
| E-eco-010 | code reading | | the check runs in the per-tick update of finished buildings |
| E-eco-010 | code reading | | `bp_setSpawn` sets a flag of the building type |
| E-eco-010 | code reading | helper id 7BFF15CE in code (19 places) | the only place that adds helpers to a stock over time |
| E-eco-007 | code reading | | making a professional from the cost list when sending; send by settler kind |
| data | script | `Jobs.lua`, `buildings.lua` | cost lists, rest times, spawn flags |

## Open questions

- (Answered: promotion in `MIL-coins-promotion.md`; first building update in
  tick 0, ECO-PROF-3.3.)
