# ECO-CAT: Catalog of buildings, goods and professions

Status: draft (values read from the shipped property scripts; meanings as in `LUA-properties-api.md`; production rules in `ECO-production-chains.md`)
Owner question(s): plan step 3.1; part of Q-005, Q-008 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

The game has 32 building types plus one construction-site type, 36 wares and
ware groups, and 31 professions, all set up by the property scripts. This file
lists, for every building, what it costs, how big it is, who works in it with
which tool, what it consumes and makes, and its work times, and for every good
and profession the values the economy needs. It is a reference table: rules on
how production runs are in `ECO-production-chains.md`, `ECO-worker-cycle.md`
and `ECO-outdoor-workers.md`.

## Rules

### ECO-CAT-1 How the table is read from the scripts

- **ECO-CAT-1.1** Every value here comes from one `properties.*` call in
  `data/scripts/properties/` (buildings, Goods, Jobs, military, ship, config).
  An implementation that runs the original scripts gets them itself; an
  implementation with its own tables must reproduce them.
- **ECO-CAT-1.2** A setter called twice keeps the last value (LUA-PROP-2.5).
  This matters once: the woodcutter's working radius is set to 5, then to 6;
  it is **6**. `bp_addFrameCost` / `bp_addCost` / `gp_addCost` append; the
  catapult's main cost is plank 1 **and** stone 2.
- **ECO-CAT-1.3** A building's **inputs** are the cost list (`gp_addCost`) of
  the good it makes (ECO-REQ-4.2). A good that is a ware group (tools,
  weapons) has a cost list of its own, which is what the building that makes
  the group consumes. The cost entry "food" of the mine products is not
  used: mines eat any one of fish, bread, ham by their own rule
  (ECO-PROD-2.2). Outdoor workers never consume a cost list (ECO-PROD-2.3).
- **ECO-CAT-1.4** A building's **tool** is the tool in its worker
  profession's cost list (the helper plus a tool make the profession,
  LUA-PROP-5). Four professions need no tool (miller, brewer, pig farmer,
  donkey breeder); the helper itself needs none.
- **ECO-CAT-1.5** Unset values have these defaults: worker limit 1
  (ECO-REQ-1.4), stock limit none (no input orders), construction time 500
  (only its sign matters, ECO-REQ-4.1), working radius 4 (shown as "(4)" in ECO-CAT-3;
  whether the workshops use a radius at all is not traced), no second
  product, not switchable.
- **ECO-CAT-1.6** Construction work ticks (column "work" in ECO-CAT-2) follow
  from the material counts by ECO-HQ-4.6 / 4.7 (float32 counting) and exclude
  the builder's walking between positions and the levelling. They are
  computed by the investigation's construction-time script, which reproduces the three
  examples of ECO-HQ-4.7 (882, 442, 1542).
- **ECO-CAT-1.7** Every building's product amount per cycle is 1
  (`bp_setGoods` count 1 everywhere).

### ECO-CAT-2 Buildings: identity and construction

Size: 0 small, 1 medium, 2 large, 3 mine, 4 harbour (LUA-PROP-3). Costs are
"frame / main"; P = plank, S = stone. Levelling (ECO-HQ-4.2) applies to sizes
1, 2 and 4 only. "Sort" is the position in its build-menu tab.

| Id | Script name | English name | Size | Frame | Main | Total | Work ticks | Remains | Sort |
|---|---|---|---|---|---|---|---|---|---|
| F6E26CB3 | castle | Headquarters | 2 | - | - | 0 | not built (construction time -1) | large | -1 |
| 5A926FA3 | woodCutter | Woodcutter's Hut | 0 | 1 P | 1 P | 2 P | 442 | small | 0 |
| 3FF43D23 | forester | Forester's Hut | 0 | 1 P | 1 P | 2 P | 442 | small | 1 |
| 043185F3 | stonePit | Quarry | 0 | 1 P | 1 P | 2 P | 442 | small | 2 |
| 3EF3BC43 | fisher | Fisherman's Hut | 0 | 1 P | 1 P | 2 P | 442 | small | 3 |
| 3C10E223 | hunter | Hunter's Hut | 0 | 1 P | 1 P | 2 P | 442 | small | 4 |
| C2C7C303 | spring | Well | 0 | 1 P | 1 P | 2 P | 442 | small | 6 |
| 281F0783 | watchTower | Lookout Tower | 0 | 2 P | 2 P | 4 P | 882 | small | 5 |
| A7BBC573 | barracks | Barracks | 0 | 1 P | 1 P | 2 P | 442 | small | 7 |
| 12E67603 | guardHouse | Guard House | 0 | 2 P | 3 S | 2 P 3 S | 1101 | small | 8 |
| 7D00B493 | tower | Watchtower | 1 | 3 P | 5 S | 3 P 5 S | 1760 | medium | 12 |
| 8C137E93 | fortress | Stronghold | 2 | 4 P | 7 S | 4 P 7 S | 2419 | large | 4 |
| 18282BD3 | ironMine | Iron Mine | 3 | 2 P | 2 P | 4 P | 882 | small | 0 |
| 154C1DAE | goldMine | Gold Mine | 3 | 2 P | 2 P | 4 P | 882 | small | 1 |
| 9B027DAE | coalMine | Coal Mine | 3 | 2 P | 2 P | 4 P | 882 | small | 2 |
| 6222A09E | stoneMine | Stone Mine | 3 | 2 P | 2 P | 4 P | 882 | small | 3 |
| 918FF373 | sawMill | Sawmill | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 0 |
| F7E2ED93 | mill | Mill | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 2 |
| 0AF7BB13 | bakery | Bakery | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 3 |
| 5C5E9743 | slaughterHouse | Slaughterhouse | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 1 |
| 154551B3 | smeltery | Iron Smelting Works | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 4 |
| FAADA31E | locksmithery | Metalworks | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 5 |
| B779AB83 | smithy | Smithy | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 6 |
| BC203663 | mint | Mint | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 7 |
| FBCA3A8E | brewery | Brewery | 1 | 2 P | 2 S | 2 P 2 S | 882 | medium | 9 |
| 78F8184E | shipYard | Shipyard | 1 | 2 P | 3 S | 2 P 3 S | 1101 | medium | 8 |
| B3965083 | depot | Storehouse | 1 | 4 P | 3 S | 4 P 3 S | 1542 | medium | 10 |
| A1445EF3 | catapult | Catapult | 1 | 3 P | 1 P + 2 S | 4 P 2 S | 1322 | medium | 11 |
| 07F63873 | farm | Farm | 2 | 3 P | 3 S | 3 P 3 S | 1322 | large | 0 |
| A6C72EDE | piggery | Pig Farm | 2 | 3 P | 3 S | 3 P 3 S | 1322 | large | 1 |
| 1F0FCD1E | donkeyBreeding | Donkey Farm | 2 | 3 P | 3 S | 3 P 3 S | 1322 | large | 2 |
| CF526FF3 | harbor | Harbour | 4 | 4 P | 6 S | 4 P 6 S | 2200 | large | 3 |
| 4A2CE4DE | construction | (construction site) | - | - | - | - | - | - | - |

Notes:
- **ECO-CAT-2.1** The construction-site type (4A2CE4DE) has only a name, a
  graphics key and an icon; it is the stand-in object for any site.
- **ECO-CAT-2.2** Within the main part the catapult's builder uses the plank
  first when it is on site (ECO-HQ-4.5).
- **ECO-CAT-2.3** Mines are size 3 and small and mines are never levelled;
  the guard house is small although it costs stone.
- **ECO-CAT-2.4** Door offset direction (ECO-WORK-3.8): north-west (4) for the
  headquarters, farm and donkey farm of every tribe and for the pig farm of
  the Africans only. No other building has one.

### ECO-CAT-3 Buildings: work

"Cycle" names the spec of the worker's cycle: WORK = `ECO-worker-cycle.md`,
OUT = `ECO-outdoor-workers.md`, own = `ECO-food-mining.md` (ECO-FOOD). Times in ticks of 0.1 s. Rest = the profession's rest time
(`gp_setRestTime`), lasting rest + 1 ticks (ECO-WORK-3.9). Stock = stock
limit per input (`bp_setStockLimit`).

| Building | Worker | Tool | Product | Second product | Inputs (per unit) | Stock | Prod. time | Rest | Radius | Cycle |
|---|---|---|---|---|---|---|---|---|---|---|
| Woodcutter's Hut | woodcutter | axe | wood | - | - | - | 55 | 222 | 6 | OUT |
| Forester's Hut | forester | shovel | (plants wood) | - | - | - | 55 | 222 | 6 | OUT |
| Quarry | stonecutter | pickaxe | stone | - | - | - | 55 | 222 | 8 | OUT |
| Fisherman's Hut | fisher | fishing rod | fish | - | - | - | 333 | 1111 | 8 | own |
| Hunter's Hut | hunter | spear | flesh (ham) | - | - (not pig, ECO-PROD-2.3) | - | 55 | 222 | 8 | own |
| Well | helper | - | water | - | - | 6 | 333 | 55 | (4) | WORK |
| Farm | farmer | scythe | corn | - | - | - | 83 | 55 | 2 | own |
| Sawmill | carpenter (`good_cabinetmaker`) | saw | plank | - | wood 1 | 6 | 333 | 55 | (4) | WORK |
| Mill | miller | - | flour | - | corn 1 | 6 | 333 | 55 | (4) | WORK |
| Bakery | baker | rolling pin | bread | - | water 1, flour 1 | 6 | 333 | 55 | (4) | WORK |
| Brewery | brewer | - | beer | - | water 1, corn 1 | 6 | 333 | 55 | (4) | WORK |
| Pig Farm | pig farmer | - | pig | - | corn 1, water 1 | 6 | 333 | 222 | (4) | WORK |
| Slaughterhouse | slaughterer (`good_slaughter`) | hatchet | flesh (ham) | - | pig 1 | 6 | 333 | 222 | (4) | WORK |
| Donkey Farm | donkey breeder | - | donkey (ware 2448356E) | - | corn 1, water 1 | 6 | 333 | 222 | (4) | own |
| Iron Smelting Works | smelter | pot | iron | - | iron ore 1, coal 1 | 6 | 333 | 55 | (4) | WORK |
| Mint | coiner | pot | coin | - | gold 1, coal 1 | 6 | 333 | 55 | (4) | WORK |
| Metalworks | locksmith | gripper | tools (group) | - | plank 1, iron 1 | 6 | 333 | 55 | (4) | WORK |
| Smithy | smith | hammer | sword | shield | iron 1, coal 1 | 6 | 333 | 55 | (4) | WORK |
| Shipyard | ship builder | hammer | boat | boat (ship mode) | plank 1 (boat) | 6 | 333 | 222 | 8 | own |
| Iron Mine | miner | pickaxe | iron ore | - | 1 of fish / bread / ham | 6 | 333 | 55 | 3 | WORK |
| Gold Mine | miner | pickaxe | gold | - | 1 of fish / bread / ham | 6 | 333 | 55 | 3 | WORK |
| Coal Mine | miner | pickaxe | coal | - | 1 of fish / bread / ham | 6 | 333 | 55 | 3 | WORK |
| Stone Mine | miner | pickaxe | stone | - | 1 of fish / bread / ham (ECO-PROD-2.4) | 6 | 333 | 55 | 3 | WORK |
| Lookout Tower | scout (`good_explorer`) | spear | - | - | - | - | - | - | - | own |
| Catapult | helper | - | - | - | stones (ECO-PROD-6.1) | 4 | - | 55 | 12 | own |
| Headquarters, Storehouse, Harbour | none (worker limit 0) | - | - | - | - | - | - | - | - | - |
| Barracks, Guard House, Watchtower, Stronghold | none (worker limit 0) | - | - | - | (coins) | 1 / 2 / 4 / 6 | - | - | - | own |

Notes:
- **ECO-CAT-3.1** The well, the catapult and the four tool-less professions
  mean that a building can be manned from helpers alone (ECO-CAT-1.4).
- **ECO-CAT-3.2** The farm's and forester's planting good is corn and wood
  (`bp_setPlantingGood`); the farm's product time 83 and radius 2 are its own
  cycle's values (Q-005).
- **ECO-CAT-3.3** The shipyard's script gives `boat` as both product and
  second product, with switchable production; the in-game help says the
  switch is "from boats to ships". The ship itself is set up by the ship
  record: frame 4 planks, main 8 planks, construct time 55 per step, stock
  limit 12, size 1, speed 0.18 (`ship.lua`). Boat and ship mode:
  `ECO-food-mining.md` ECO-FOOD-7 (ship site and launch: plan step 4.7).
- **ECO-CAT-3.4** The donkey farm makes the good `donkey` (2448356E), which
  the scripts type as a ware group; the settler that pulls on roads is the
  profession `good_donkey` (D4157C5F, Q-108). The farm creates the settler
  directly at its door after each production; the ware is never made
  (`ECO-food-mining.md` ECO-FOOD-6).

### ECO-CAT-4 Buildings: military and storage

| Building | Soldiers | Territory | Explore | Coin stock | Depot | Spawn | Upgrades to |
|---|---|---|---|---|---|---|---|
| Headquarters | - | 9 | 12 | - | yes | yes | - |
| Storehouse | - | - | - | - | yes | yes | - |
| Harbour | - | - | - | - | yes | yes | - |
| Barracks | 2 | 8 | 11 | 1 | - | - | Guard House |
| Guard House | 3 | 9 | 12 | 2 | - | - | Watchtower |
| Watchtower | 6 | 10 | 13 | 4 | - | - | Stronghold |
| Stronghold | 9 | 11 | 14 | 6 | - | - | - |
| Lookout Tower | - | - | 20 | - | - | - | - |

- **ECO-CAT-4.1** Territory and explore radius are in fields. The coin stock
  is the military building's stock limit; that it holds coins comes from the
  in-game help ("the current amount of gold coins here, and the maximum
  amount possible"). Military rules are plan steps 4.1 to 4.6.

### ECO-CAT-5 Goods

Type: W ware, G ware group, S settler (LUA-PROP-3). Priority: transport
priority, smaller is carried first (LUA-PROP-5); for weapons and tools the
script sets it twice and the second value counts. "Res" = gathered from
deposits (D) or the per-field resource layer (F).

| Id | Name | Type | Group | Cost (inputs) | Priority | Res | Made by | Consumed by |
|---|---|---|---|---|---|---|---|---|
| CD6EC133 | wood | W | - | - | 9 | D | woodcutter | sawmill |
| C5C945A3 | plank | W | - | wood 1 | 7 | - | sawmill | metalworks, shipyard, construction |
| 55E952D3 | stone | W | - | - | 8 | D, F | quarry, stone mine | construction, catapult |
| BB5D0683 | pig | W | - | corn 1, water 1 | 11 | - | pig farm | slaughterhouse |
| 17626A03 | corn | W | - | - | 11 | D | farm | mill, brewery, pig farm, donkey farm |
| E70C72D3 | flour | W | - | corn 1 | 11 | - | mill | bakery |
| 4012E5A3 | fish | W | food | - | 11 | F | fisher | mines |
| AD0D6C63 | flesh (ham) | W | food | pig 1 | 11 | - | slaughterhouse, hunter | mines |
| 90AB8923 | bread | W | food | water 1, flour 1 | 11 | - | bakery | mines |
| A9676263 | water | W | - | - | 10 | - (see ECO-PROD-2.5) | well | bakery, brewery, pig farm, donkey farm |
| 58647DA3 | beer | W | - | water 1, corn 1 | 2 | - | brewery | soldier recruiting |
| 7068DCD3 | coal | W | - | food 1 | 6 | F | coal mine | smelting works, mint, smithy |
| EC5020BE | iron ore | W | - | food 1 | 5 | F | iron mine | smelting works |
| 8198BF23 | iron | W | - | iron ore 1, coal 1 | 3 | - | smelting works | metalworks, smithy |
| 4F41C633 | gold | W | - | food 1 | 4 | F | gold mine | mint |
| 499D4C63 | coin | W | - | gold 1, coal 1 | 0 | - | mint | military buildings |
| 6CC2D2D3 | gripper (tongs) | W | tools | - | 12 | - | metalworks | locksmith |
| E1281613 | axe | W | tools | - | 12 | - | metalworks | woodcutter |
| 3788FFC3 | saw | W | tools | - | 12 | - | metalworks | carpenter |
| 45D899E3 | pickaxe | W | tools | - | 12 | - | metalworks | stonecutter, miner |
| 69C15033 | hammer | W | tools | - | 12 | - | metalworks | builder, smith, ship builder, scholar |
| 67C04E73 | shovel | W | tools | - | 12 | - | metalworks | leveller, forester |
| ED76AE73 | pot (crucible) | W | tools | - | 12 | - | metalworks | smelter, coiner |
| F3F3DCC3 | fishing rod | W | tools | - | 12 | - | metalworks | fisher |
| 1CEACD43 | scythe | W | tools | - | 12 | - | metalworks | farmer |
| F2B7453E | hatchet (cleaver) | W | tools | - | 12 | - | metalworks | slaughterer |
| C2049323 | rolling pin | W | tools | - | 12 | - | metalworks | baker |
| 79F34393 | spear | W | tools | - | 12 | - | metalworks | hunter, scout |
| A8483903 | sword | W | weapons | iron 1, coal 1 | 1 | - | smithy | soldier recruiting |
| 1EEBCF03 | shield | W | weapons | iron 1, coal 1 | 1 | - | smithy | soldier recruiting |
| F63DE553 | boat | W | - | plank 1 | 13 | - | shipyard | boat carrier |
| 2448356E | donkey | G | - | corn 1, water 1 | 14 | - | donkey farm | (becomes road donkey, ECO-CAT-3.4) |
| 65CD3AFE | food | G | - | - | 11 | - | - | mines |
| D5AA9BDE | tools | G | - | plank 1, iron 1 | 12 | - | metalworks | - |
| 0897D1E3 | weapons | G | - | coal 1, iron 1 | 1 | - | - | - |
| 1D66B4AE | ship | G | - | - | 11 | - | (shipyard, ship mode) | - |

- **ECO-CAT-5.1** "Sort" values (UI order) are 0..30 for the wares in the
  order wood, plank, stone, pig, corn, flour, fish, flesh, bread, water, beer,
  coal, iron ore, gold, iron, coin, gripper, axe, saw, pickaxe, hammer,
  shovel, pot, fishing rod, scythe, hatchet, rolling pin, spear, sword,
  shield, boat. The groups have none.
- **ECO-CAT-5.2** Geologist signs exist for water (threshold 1), and for coal,
  iron ore, gold and stone (thresholds 1, 3, 6: small, medium, large), plus
  the "nothing" sign (`signs.lua`).

### ECO-CAT-6 Professions

All have type settler (1) except the boat carrier (`good_shipHelper`, typed
ware group in the script). Cost = what turns a helper into the profession.

| Id | Script name | Name | Tool | Rest | Works in |
|---|---|---|---|---|---|
| 7BFF15CE | good_helper | Helper | - | 55 | carriers, well, catapult, raw material of all others |
| DD2DAEBE | good_constructor | Builder | hammer | - | construction sites |
| 2740000E | good_bulldozer | Leveller | shovel | - | construction sites |
| 2C50533E | good_woodcutter | Woodcutter | axe | 222 | woodcutter's hut |
| 6EBA970E | good_forester | Forester | shovel | 222 | forester's hut |
| 8A3AA1EE | good_stonecutter | Stonecutter | pickaxe | 222 | quarry |
| 4AC589CE | good_fisher | Fisher | fishing rod | 1111 | fisherman's hut |
| AB43100E | good_hunter | Hunter | spear | 222 | hunter's hut |
| 2195BC53 | good_cabinetmaker | Carpenter | saw | 55 | sawmill |
| 8D0BA75E | good_farmer | Farmer | scythe | 55 | farm |
| 290E6B9E | good_miller | Miller | - | 55 | mill |
| 862797A3 | good_baker | Baker | rolling pin | 55 | bakery |
| F0622D7E | good_slaughter | Butcher | hatchet | 222 | slaughterhouse |
| 0EAFFECE | good_brewer | Brewer | - | 55 | brewery |
| 59B7633E | good_miner | Miner | pickaxe | 55 | the four mines |
| 9C0AE313 | good_smelter | Smelter | pot | 55 | iron smelting works |
| 03A2B9F3 | good_smith | Weaponsmith | hammer | 55 | smithy |
| FDF21163 | good_coiner | Coiner | pot | 55 | mint |
| E59A65CE | good_locksmith | Toolsmith | gripper | 55 | metalworks |
| 360B16E3 | good_pigfarmer | Pig farmer | - | 222 | pig farm |
| 35A82813 | good_donkeyBreeder | Donkey breeder | - | 222 | donkey farm |
| AE71FBF3 | good_shipBuilder | Ship builder | hammer | 222 | shipyard |
| 3355C25E | good_scholar | Scholar (presumably the geologist) | hammer | - | (sent to flags) |
| A07BF6FE | good_explorer | Scout | spear | - | lookout tower, (sent to flags) |
| D4157C5F | good_donkey | Donkey | (no cost) | - | roads |
| 182118D3 | good_shipHelper | Boat carrier | boat | - | waterways |
| 15E86843 | good_soldier0 | Private | sword, shield, beer | - | military |
| BA0E7F83 | good_soldier1 | Private first class | soldier0 + coin | - | military |
| 466C6C23 | good_soldier2 | Sergeant | soldier1 + coin | - | military |
| 906014F3 | good_soldier3 | Officer | soldier2 + coin | - | military |
| 559DDEA3 | good_soldier4 | General | soldier3 + coin | - | military |

- **ECO-CAT-6.1** Soldier health by rank 0..4: 3, 4, 5, 6, 7; upgrade chain
  barracks, guard house, watchtower, stronghold (`military.lua`).
- **ECO-CAT-6.2** The helper's cost list names the helper itself; how new
  helpers appear is plan step 3.2.
- **ECO-CAT-6.3** English profession names in this table are descriptive,
  not the game's display strings (which were not looked up).

### ECO-CAT-7 The chains

Derived from ECO-CAT-3 and ECO-CAT-5 (arrows: good -> building -> good).

- **ECO-CAT-7.1 Wood:** trees -> woodcutter -> wood -> sawmill -> plank.
  Forester replants trees.
- **ECO-CAT-7.2 Stone:** stone piles -> quarry -> stone; granite in
  mountains -> stone mine -> stone.
- **ECO-CAT-7.3 Grain:** farm -> corn -> mill -> flour; corn + water ->
  brewery -> beer; corn + water -> pig farm -> pig -> slaughterhouse -> ham;
  corn + water -> donkey farm -> donkey; water + flour -> bakery -> bread.
- **ECO-CAT-7.4 Food:** fish (fisher), ham (hunter, slaughterhouse), bread
  (bakery) form the group food; each of the iron, gold and coal mine eats one
  food unit per product (the stone mine: ECO-PROD-2.4).
- **ECO-CAT-7.5 Metal:** iron ore + coal -> smelting works -> iron;
  plank + iron -> metalworks -> one tool; iron + coal -> smithy -> sword or
  shield; gold + coal -> mint -> coin.
- **ECO-CAT-7.6 Military:** helper + sword + shield + beer -> private;
  private + coin -> next rank, up to general.
- **ECO-CAT-7.7 Ships:** plank -> shipyard -> boat (boat carriers on
  waterways) or ship (ship mode).

## Constants

Confidence of a value read from a script is **high** for the value itself.
The meaning columns carry the confidence of the setter in `LUA-properties-api.md`
unless listed here.

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| building types | 32 + construction site | | high | scripts, LUA-PROP T2 (bp 33) |
| sizes, costs, worker, product, second product, stock limit, production time | ECO-CAT-2, 3 | | high (values and meaning) | scripts; meaning traced in ECO-WORK, ECO-REQ, ECO-HQ |
| construction work ticks | ECO-CAT-2 | ticks | high (derived; verified against ECO-HQ-4.7 examples) | |
| inputs = product's cost list | ECO-CAT-3 | | high (workshops: slot 0's cost list; mines: food rule; outdoor workers: none), ECO-PROD-2 | ECO-REQ-4.2, E-eco-008 |
| working radius | ECO-CAT-3 | fields | high (woodcutter, quarry, forester: ECO-OUT); medium (others) | scripts |
| rest time | ECO-CAT-6 | ticks (+1) | high | ECO-WORK-3.9 |
| woodcutter radius | 6 (second call) | fields | high | scripts, LUA-PROP-2.5, ECO-OUT |
| tool per profession | ECO-CAT-6 | | medium (cost list read; use in recruiting is plan step 3.2) | `Jobs.lua` |
| soldiers, territory, explore radius, coin stock | ECO-CAT-4 | | medium | scripts; help texts |
| transport priority | ECO-CAT-5 | rank | high (value), medium (second call wins for tools/weapons) | `Goods.lua` |
| door offset buildings | ECO-CAT-2.4 | | high | scripts |
| ship record | frame 4 P, main 8 P, step 55, stock 12 | | high (value), medium (meaning) | `ship.lua` |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | run the property scripts | list building types | 33 created; 32 with a size; construction site 4A2CE4DE has none |
| T2 | as T1 | woodcutter working radius | 6 |
| T3 | as T1 | catapult cost | frame plank 3; main plank 1 and stone 2 (total 6 units) |
| T4 | as T1 | inputs of the bakery | water 1 and flour 1, stock limit 6 each |
| T5 | as T1 | inputs of the metalworks | plank 1 and iron 1 (the cost list of the group `tools`) |
| T6 | ECO-HQ-4.6 / 4.7 counting | work ticks of the stronghold (4 + 7) | 2419; frame done at unit 4, decrement 218; complete at unit 11, decrement 209 |
| T7 | as T6 | work ticks of guard house, harbour, catapult | 1101, 2200, 1322 |
| T8 | as T1 | tool of the miller, brewer, pig farmer, donkey breeder | none (cost list is the helper only) |
| T9 | as T1 | transport priority of weapons and of tools | 1 and 12 |
| T10 | as T1 | which buildings have a door offset, for tribe 1 | headquarters, farm, donkey farm, pig farm |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-lua-001 | script survey | `data/scripts/properties/*.lua` | every value in ECO-CAT-2..6 |
| E-lua-003 | game text | `data/language/gb/dialogs.txt`, `dlg_*Building`, `dlg_*BuildingInfo`, `dlg_LocalProductionInfo*` | English names; help texts (ship mode, coins in military buildings, wells over water) |
| E-lua-003 | UI scripts | `data/scripts/info/b_<id>.lua` | which info text each building's window shows |
| ECO-REQ-4.2 | spec | `ECO-requests-dispatch.md` (binary evidence there, E-eco-007) | inputs from the product's cost list |
| ECO-HQ-4.6, 4.7 | spec | `ECO-warehouse-construction.md` (binary evidence there, E-eco-006, E-trn-002) | construction counting used for the work ticks |

## Open questions

- Production rules (what a building checks before it works, which product it
  makes, food choice in mines, the stone mine and the well) are in
  `ECO-production-chains.md` (ECO-PROD).
- Cycles of farm, fisher, hunter, donkey farm, shipyard, catapult, lookout
  tower: plan step 3.3 (Q-005).
- Display names of professions (ECO-CAT-6.3).
