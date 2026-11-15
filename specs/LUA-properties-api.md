# LUA-PROP: The `properties` script API

Status: draft (function list verified in the binary; meanings mostly from names and script usage)
Owner question(s): Q-008 in `OPEN_QUESTIONS.md` (API part), supports Q-005, Q-007
Game build: 11757

## Summary

All game rule tables (goods, professions, buildings, terrain types, deposits,
doodads, animals, sounds, AI profiles, global settings) are set up at start by
Lua 5.0 scripts in `data/scripts/properties/`. The scripts call functions of one
global table named `properties`. The engine registers 148 functions in it; the
shipped scripts call 136 of them. An implementation that runs the original
scripts must provide these functions with the argument shapes below. This file
describes what each call means; it deliberately does not list the values the
scripts set.

## Rules

### LUA-PROP-1 Environment

- **LUA-PROP-1.1** Language: Lua 5.0 (5.0.2). The scripts are plain source text
  with CRLF line ends, Latin-1 bytes in comments, stored in the `rc00` container.
  They use only assignments, function definitions, calls, `if`/`else`, `nil`
  tests and string concatenation. No standard library function is called.
- **LUA-PROP-1.2** Entry point: `data/scripts/properties/data.lua`. It defines a
  helper that calls `Default.CallScript(path)` and uses it to run the other
  files in this order: doodads, config, patterns, goods, jobs, deposits,
  remains, buildings, streets, signs, military, ship, animals, sounds, ai.
- **LUA-PROP-1.3** `Default.CallScript(path)` runs another script file in the same
  global environment. The path uses backslashes and is relative to the game
  directory (`data\scripts\properties\goods.lua`). File names are matched without
  regard to case: the script asks for `goods.lua` and `jobs.lua`, the files are
  `Goods.lua` and `Jobs.lua`.
- **LUA-PROP-1.4** `Default.WriteToLog(text)` writes a line to the game log. (Used
  by `data/scripts/graphic_main.lua` only.)
- **LUA-PROP-1.5** All files share one global namespace and depend on the load
  order: a later file uses variables assigned by an earlier one (buildings use
  goods, professions and remains; deposits use goods and patterns; signs use
  goods and doodads; config uses a doodad).
- **LUA-PROP-1.6** The engine defines no enumeration constants. Every named
  constant used as an argument is an ordinary global assigned by the scripts
  themselves. The numeric values in LUA-PROP-3 are therefore part of the contract
  between scripts and engine.
- **LUA-PROP-1.7** Functions return nothing except the `*_create` functions.
  Calls that the engine cannot apply (unknown handle) are not expected in the
  shipped scripts; error behaviour was not examined.

### LUA-PROP-2 Objects and handles

- **LUA-PROP-2.1** `xx_create("<hex>")` registers a new object of the family `xx`
  and returns a handle. The argument is a string of 1 to 8 hexadecimal digits:
  the object's permanent 32-bit id. This id is what map and save files store
  (`FMT-s2m-map.md` FMT-S2M-1.7) and what the per-object info scripts are named
  after (`data/scripts/info/b_<id>.lua` for buildings, `i_<id>.lua` for goods).
- **LUA-PROP-2.1a** How the string is read: characters are taken from the start
  until the first character that is not a hex digit (or the end); that prefix
  is the id, most significant digit first. Digits `0`-`9`, `a`-`f`, `A`-`F` have
  their usual value. (The characters `*` to `/` and `:` to `?`, codes 0x2A to
  0x3F, are accepted as digits with the value of their low four bits; no
  shipped script uses them.) A string that does not start with a hex digit
  gives id 0. More than 8 digits overflow in an unspecified way. So
  `"0xcea2bcc3"` (the one such call in `sounds.lua`) is read as id **0**, not
  as 0xCEA2BCC3; the create succeeds and that sound gets id 0. Measured
  (2026-10-25, E-heap-003): the running game holds that sound (`locksmithery
  sawing`) under id 0 and holds no object 0xCEA2BCC3.
- **LUA-PROP-2.2** A handle is a Lua number. Scripts store it in a global and pass
  it as the first argument of that family's setters and as a reference argument
  of other families (a building's worker is a good handle, and so on). Scripts
  pass `-1` where "no object" is meant. Whether the handle equals the id or is an
  index is not established; an implementation may choose, as long as `-1` never
  collides with a valid handle. What the object keeps is the referenced object's
  **id**, not a handle or an index (measured 2026-10-25, E-heap-003: a building's
  worker, remains and planting good, a deposit's good and next deposit, a good's
  group, 124 references in all, each held as the referenced object's id).
- **LUA-PROP-2.5a Measured (2026-10-25, E-heap-003).** In the running original
  every object of the eight families with objects (`bp` 33, `gp` 67, `dp` 38,
  `dd` 112, `an` 3, `rm` 3, `sp` 1, `sn` 104: 361 objects) is held with the
  shipped scripts' values as 2.1, 2.1a and 2.5 predict: every name, file list,
  number, flag and reference that a one-value setter writes equals the script's
  last call (`add*` lists in call order), at trace ticks 1 and 6000 of a free game.
  Cost lists, goods slots, positions and the other multi-value setters were not
  compared, nor the values of setters a script does not call (the defaults). Per
  item: the heap dumps of E-trace-007 at trace ticks 1 and 6000. Confidence: high.
- **LUA-PROP-2.3** `ai_create` is the exception: its string is a small decimal
  number ("1", "2", "3"), the profile number.
- **LUA-PROP-2.4** Setters without a handle argument (`cfg_*`, `mi_*`, `sh_*`,
  `si_*`) change one global record.
- **LUA-PROP-2.5** A setter may be called more than once for the same object; the
  last call wins. `add*` functions append to a list instead.
- **LUA-PROP-2.6** Families: `gp` goods (wares, settler professions and ware
  groups are all "goods"), `bp` buildings, `pa` terrain patterns, `dp` deposits
  (things standing on a field that yield a good: trees, stone piles, grain
  fields), `dd` doodads (decoration, signs), `rm` building remains, `an` animals,
  `sp` street types, `sn` sounds, `ai` computer player profiles; global records
  `cfg` settings, `mi` military, `sh` ship, `si` geologist signs.

### LUA-PROP-3 Constants defined by the scripts

Argument types below refer to these by name.

| Name | Values |
|---|---|
| good type | -1 invalid, 0 ware, 1 settler (profession), 2 ware group |
| building size | 0 small, 1 medium, 2 large, 3 mine, 4 harbor |
| depot kind | -1 none, 0 local, 1 resources, 2 stuff |
| door direction | 3 west, 4 north-west, 5 north-east |
| tribe | 0 Romans, 1 Africans, 2 Chinese |
| tree set | 0 Caribbean, 1 coniferous, 2 black forest, 3 lava |
| doodad type | 0 landscape, 1 sign |
| sound type | 0 ambient, 1 music, 2 GUI, 3 sound effect |
| AI military mode | 0 safe, 1 normal, 2 aggressive |

Time units: "ticks" are logic ticks of 0.1 s; see `ECO-time-units.md`.
"Fields" are map fields. Types: `handle(xx)` = handle of family `xx`; `int`,
`number`, `bool`, `string` as in Lua; `count` = non-negative int.

### LUA-PROP-4 Global settings: `cfg_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `cfg_setGameSpeed` | number seconds | length of one logic tick. Not called by shipped scripts (default 0.1). | high |
| `cfg_setSettlerSpeed` | number | walking progress per tick along one field step (1.0 = one field) | medium |
| `cfg_setPickupTime` | number ticks | time a carrier needs to pick a ware up at a flag | medium |
| `cfg_setDropTime` | number ticks | time to put a ware down | medium |
| `cfg_setBulldozeTime` | number ticks | time to level a building site | medium |
| `cfg_setConstructTime` | number ticks | length of one construction work step of a builder | high |
| `cfg_setFlagPackageLimit` | count | how many wares can lie at one flag | medium |
| `cfg_setFishingDistance` | count fields | how far from his spot a fisher reaches for fish | low |
| `cfg_setCatapultRotationTimeFactor` | number | scales how long a catapult takes to turn | low |
| `cfg_setCatapultFlightTimeFactor` | number | scales how long a stone flies | low |
| `cfg_setSettlerDyingTime` | int ticks | how long a dying settler remains before removal | medium |
| `cfg_setSettlerSkeletonDoodad` | handle(dd) | doodad left where a settler died | medium |
| `cfg_setBorderPattern` | handle(pa) | terrain pattern used for the map border (called from the pattern script) | medium |

### LUA-PROP-5 Goods and professions: `gp_`

Used by the goods script (wares, groups) and the jobs script (professions).
A good whose script never calls `gp_setType` has type 0, **ware** (the default
of a new good; the scripts define only `Settler = 1` and `WareGroup = 2`).
Transport priority (`gp_setTransportPriority`): smaller numbers are carried
first (`TRN-carriers-transport.md`).

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `gp_create` | string id | new good; returns handle | high |
| `gp_setName` | handle, string | display / debug name | high |
| `gp_setShortName` | handle, string | short code used in debug output and statistics | medium |
| `gp_setIconXY` | handle, int column, int row | cell of the good's icon in the icon atlas | medium |
| `gp_setType` | handle, good type | ware, settler or group | high |
| `gp_setSort` | handle, int | position in sorted UI lists | medium |
| `gp_setTransportPriority` | handle, int | default rank in the transport priority list | medium |
| `gp_setAiPriority` | handle, int | weight the computer player gives this good | low |
| `gp_setGroupGoodId` | handle member, handle(gp) group | makes the good a member of a ware group (food, tools, weapons); a building that accepts the group accepts any member | medium |
| `gp_addCost` | handle product, handle(gp) input, count | adds an input to what is consumed to make one unit. For a profession: the base settler plus the tool that turn a helper into that profession. | medium |
| `gp_setRestTime` | handle profession, number ticks | pause of a worker of this profession between two work cycles | medium |
| `gp_setPlantable` | handle, bool | the good can be planted (forester, farmer) | medium |
| `gp_enableDeposits` | handle, int flag (1) | the good is gathered from deposits standing on the map (wood, stone, grain) | medium |
| `gp_enableFields` | handle, int flag (1) | the good exists as a per-field resource amount in the map's resource layer (fish, coal, iron ore, gold, granite, water) | medium |
| `gp_setValue` | handle, number | not called by shipped scripts | - |

### LUA-PROP-6 Buildings: `bp_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `bp_create` | string id | new building type; returns handle. One extra type stands for "construction site". | high |
| `bp_setName` | handle, string | name | high |
| `bp_setGfxName` | handle, string | key of the building's graphics set (model folders are derived from it per tribe) | medium |
| `bp_setIconXY` | handle, int, int | icon cell | medium |
| `bp_setSort` | handle, int | position in the build menu / lists | medium |
| `bp_setSize` | handle, building size | plot size class; decides where it may be built | high |
| `bp_addFrameCost` | handle, handle(gp), count | wares needed for the first building phase (the frame) | medium |
| `bp_addCost` | handle, handle(gp), count | wares needed for the second phase. Total cost is frame cost plus cost. | medium |
| `bp_setConstructionTime` | handle, number ticks | special construction time; -1 = none (only the headquarters, which is never built) | low |
| `bp_setConstructionPosition` | handle, tribe, int index, number x, number y | spot number `index` (0-based) where a builder stands while working on this building, as an offset from the building in field units, per tribe | medium |
| `bp_setDoorOffsetDirection` | handle, tribe, door direction | side on which the door is, where it differs from the default for that tribe's model | medium |
| `bp_setWorker` | handle, handle(gp) profession | profession that operates the building | high |
| `bp_setWorkerLimit` | handle, count | number of workers; 0 = none needed | medium |
| `bp_setGoods` | handle, handle(gp), count | product and amount per cycle | high |
| `bp_setSecondGoods` | handle, handle(gp), count | alternative product (buildings that alternate or can be switched) | medium |
| `bp_setAlternativeGoods` | handle, handle(gp), count | same function as `bp_setSecondGoods` under another name; not called | high |
| `bp_setSwitchableProduction` | handle, bool | player may choose which of the two products is made | medium |
| `bp_setProductionTime` | handle, number ticks | length of one production cycle | medium |
| `bp_setStockLimit` | handle, count | how many units of each input the building stores | medium |
| `bp_setWorkingRadius` | handle, count fields | radius in which the worker looks for trees, stone, fish, game, planting spots | medium |
| `bp_setPlantingGood` | handle, handle(gp) | good the worker plants (forester, farm) | medium |
| `bp_setDepot` | handle, int flag (1) | building stores wares like a warehouse | medium |
| `bp_setSpawn` | handle, int flag (1) | new settlers appear here | low |
| `bp_setHarbor` | handle, bool | building is a harbor | high |
| `bp_setCatapult` | handle, bool | building is a catapult | high |
| `bp_setSoldierLimit` | handle, count | soldiers a military building holds | medium |
| `bp_setTerritoryRange` | handle, count fields | radius of land the building claims | medium |
| `bp_setExploreRadius` | handle, count fields | radius it uncovers on the map | medium |
| `bp_setRemains` | handle, handle(rm) | ruin left when it is destroyed | medium |
| `bp_setAiScore` | handle, int | value for the computer player's planning | low |
| `bp_setAiThreat` | handle, int | how dangerous the computer player rates it | low |
| `bp_setHealth` | handle, number | not called by shipped scripts | - |

How inputs of production are defined: there is no "input" setter on buildings.
Inputs follow from `gp_addCost` of the product (LUA-PROP-5). (Hypothesis from the
API shape; to be confirmed under Q-005.)

### LUA-PROP-7 Terrain patterns: `pa_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `pa_create` | string id | new terrain type; returns handle | high |
| `pa_setName` | handle, string | editor name | high |
| `pa_setGfxName` | handle, string | key into the pattern list of `data/settings/terrain_static_data.xml`, which gives the texture file (not always `<name>.dds`), alias and texture scale (`GFX-terrain-patterns.md`, Q-126); also the key in `properties_filterlist.ini` | high |
| `pa_setBlocked` | handle, bool | nothing can walk or build there | medium |
| `pa_setForBuilding` | handle, bool | buildings allowed | high |
| `pa_setForPlanting` | handle, bool | trees / grain can be planted | medium |
| `pa_setForMining` | handle, bool | mines allowed | high |
| `pa_setForHarbor` | handle, bool | harbor allowed | high |
| `pa_setTreeType` | handle, tree set | which tree family a forester plants here | medium |
| `pa_setMiniMapColor` | handle, int R, int G, int B | colour on the overview map, 0..255 each | high |

The pattern script wraps these in its own Lua helper functions with optional
(`nil`) arguments; an implementation only sees the `properties` calls.

### LUA-PROP-8 Deposits: `dp_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `dp_create` | string id | new deposit type; returns handle | high |
| `dp_setName` | handle, string | name | high |
| `dp_setGfxName` | handle, string [, int] | model name; optional variant number | medium |
| `dp_setGood` | handle, handle(gp) | good obtained from it | high |
| `dp_setBlocking` | handle, bool | its field is blocked for walking and building | medium |
| `dp_nextDeposit` | handle, handle(dp) | deposit type it turns into after one unit was taken (a stone pile shrinks through a chain of types) | medium |
| `dp_setElevationSize` | handle, int | footprint radius on the height grid | low |
| `dp_addUndergroundPattern` | handle, handle(pa) | terrain pattern put under the deposit (field for grain, bare ground under stones); may be called several times | low |
| `dp_setRandomFinePosOffsetState` | handle, bool | whether new instances get a random offset inside their field | medium |
| `dp_setNeedForPlantablePattern` | handle, bool | may only be placed on terrain that allows planting | medium |
| `dp_setGrowingSpeed` | handle, number | size gained per tick (1.0 = full size) | medium |
| `dp_setGrowingStartSize` | handle, number | size when planted | medium |
| `dp_setLifeTime` | handle, int ticks | how long it stays once fully grown before it withers | low |
| `dp_setDoodadAfterDestruction` | handle, handle(dd) | not called by shipped scripts | - |

Class rule (2026-10-24): every `dp_create` entry of `deposits.lua` is a deposit type
made of these setters, drawn by GFX-OBJ-1.3. Confidence: medium (the lowest of its main
setters). Covers (inventory): *deposit type* from `scripts/properties/deposits.lua`.

### LUA-PROP-9 Doodads, remains, signs: `dd_`, `rm_`, `si_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `dd_create` | string id | new doodad type; returns handle | high |
| `dd_setName` | handle, string | name | high |
| `dd_setGfxName` | handle, string | model name | medium |
| `dd_setBlocking` | handle, bool | blocks its field (stored in the map's blocking doodad list) | high |
| `dd_setType` | handle, doodad type | landscape decoration or geologist sign | medium |
| `dd_setLifeTime` | handle, int ticks | removed after this time (signs) | medium |
| `dd_setDeleteByStreet` | handle, bool | whether building a road over it removes it (default on; removal is permanent, `GFX-objects.md` GFX-OBJ-1.4a) | high |
| `dd_setOnWater` | handle, bool | not called by shipped scripts | - |
| `rm_create` | string id | new ruin type; returns handle | high |
| `rm_setName` | handle, string | name | high |
| `rm_setGfxName` | handle, string | model name | medium |
| `rm_setBurningTime` | handle, int ticks | how long the ruin burns before it disappears | medium |
| `rm_setDoodadAfterFinished` | handle, handle(dd) | not called by shipped scripts | - |
| `si_addSignDoodadId` | handle(gp) or -1, count threshold, handle(dd) | sign a geologist plants for that resource when the amount found is at least the threshold; several thresholds per good give small / medium / large signs; good -1 is the "nothing found" sign | medium |

Class rule (2026-10-24): every `dd_create` entry of `doodads.lua` is a doodad type made
of these setters, drawn by GFX-OBJ-1.3. Confidence: medium (the lowest of its main
setters). Covers (inventory): *doodad type* from `scripts/properties/doodads.lua`.

### LUA-PROP-10 Military and ship: `mi_`, `sh_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `mi_setFightingTime` | number ticks | length of one round of a duel | low |
| `mi_setSoldierId` | int rank (0..4), handle(gp) | profession that represents soldiers of that rank | high |
| `mi_setSoldierMaxHealth` | int rank, int | hit points of that rank | medium |
| `mi_addBuildingUpgrade` | handle(bp) from, handle(bp) to | military building `from` can be upgraded to `to` | medium |
| `sh_setSpeed` | number | sailing progress per tick | medium |
| `sh_addFrameCost` | handle(gp), count | wares for the first building phase of a ship | medium |
| `sh_addCost` | handle(gp), count | wares for the second phase | medium |
| `sh_setConstructTime` | number ticks | length of one ship building work step | medium |
| `sh_setSize` | int | size class of the ship | low |
| `sh_setStockLimit` | count | cargo capacity | medium |

### LUA-PROP-11 Animals, streets, sounds, AI: `an_`, `sp_`, `sn_`, `ai_`

| Function | Arguments | Meaning | Conf. |
|---|---|---|---|
| `an_create` | string id | new animal type; returns handle | high |
| `an_setName` / `an_setGfxName` | handle, string | name / model | high |
| `an_setMovementSpeed` | handle, number | walking progress per tick | medium |
| `an_setMoveDistance` | handle, count fields | how far one walk goes | low |
| `an_setRange` | handle, count fields | how far it strays from where it was placed | low |
| `an_setAvoidBuildings` | handle, bool | keeps away from buildings | medium |
| `an_setIdleTimeFactor` | handle, number | not called by shipped scripts | - |
| `sp_create` | string id | new street type; returns handle (one exists) | high |
| `sp_setName` / `sp_setShortName` / `sp_setMesh` | handle, string | name, short code, model | medium |
| `sp_setHeight`, `sp_setLength`, `sp_addCost` | - | not called by shipped scripts | - |
| `sn_create` | string id | new sound; returns handle | high |
| `sn_setName` | handle, string | name | high |
| `sn_addFileName` | handle, string path | adds a sound file; several files = random variants | medium |
| `sn_setType` | handle, sound type | ambient, music, GUI or effect | high |
| `sn_setLooping` | handle, bool | loops | high |
| `sn_setVolume` | handle, int 0..255 | volume | medium |
| `sn_setPriority` | handle, int | channel priority | medium |
| `sn_set3DSound` | handle, bool | positioned in the world or not | medium |
| `sn_set3DMinDistance` | handle, number | distance at which it starts to fade | medium |
| `sn_set3DMaxDistance` | handle, number | not called by shipped scripts | - |
| `sn_set2DDistance` | handle, number | audible distance for non-positional ambient sounds | low |
| `sn_setMusicTribe` | handle, tribe | tribe whose music this is | medium |
| `sn_setPercussion` | handle, bool | percussion layer of the tribe music | low |
| `ai_create` | string number | new computer player profile; returns handle | high |
| `ai_setName` | handle, string | profile name (weak / normal / strong) | high |
| `ai_setMilitaryMode` | handle, AI military mode | how eagerly it attacks | medium |
| `ai_setAttackStrength` | handle, number 0..1 | share of available soldiers it sends | low |
| `ai_setInterceptingFactor` | handle, number | how strongly it reinforces against attacks | low |
| `ai_setStartSawmillCount` | handle, count | sawmills it builds first | medium |
| `ai_setMaximumSmithies`, `ai_setMaximumMints` | handle, count | upper limits for those buildings | medium |
| `ai_enableStreetOptimizing`, `ai_enableRouteOptimizing`, `ai_enableCatapults`, `ai_enableCoinArrangement` | handle, bool | feature switches | medium |

Sounds and AI profiles do not affect simulation rules for a human-only game, but
ambient sound ids are referenced by map files and must at least be accepted.

### LUA-PROP-12 Help pages and info scripts (Q-011; 2026-10-24, E-sim-007)

- **LUA-PROP-12.1 Which page.** The help pane of a window shows one of three pages: a
  dialog's help text, the text `dialog<id>_<page>` of `dialogs.txt` (UI-GAME-1.3); a
  good's page, made by running `data/scripts/info/i_<id>.lua`; or a building type's page,
  made by running `data/scripts/info/b_<id>.lua`, where `<id>` is the object's id
  (LUA-PROP-2.1) in lower-case hex **without leading zeros** (`b_43185f3.lua` for
  0x043185f3). If the script file does not exist the page shows the plain text
  `Database file b_<id> not found!` (or `i_`) in font `light_system`, in the 285 x 360
  box at (55, 41). Confidence: high (E-sim-007).
- **LUA-PROP-12.2 The script's functions.** The script draws the page with four calls of
  the `ui` table; coordinates and sizes are pixels of the 1024 x 768 reference space
  (UI-GAME-1.1), placed in the help pane:
  - `ui.infoLabel2(x, y, key)`: a one-line label, its text looked up as a text key,
    in orange (255, 200, 0) (the page's title);
  - `ui.infoLabel(x, y, text)`: the same label in the default colour; with a fourth
    argument (the shipped scripts pass 1 with texts such as `x3`) the text is set a
    second way (shown as given);
  - `ui.infoText(x, y, w, h, key)`: a text box of size w x h in font `light_system`,
    the text looked up as a key and wrapped (the description);
  - `ui.infoPicture(x, y, atlas, sx, sy, sw, sh, w, h)`: the rectangle (sx, sy, sw, sh)
    of the named atlas drawn at (x, y) with size w x h (cost icons: 48 x 48 cells of
    `uiElements1` drawn 50 x 50).
  The 124 shipped scripts use only these (124 infoLabel2, 124 infoText, 96 infoLabel,
  64 infoPicture). Their keys are the `dlg_*` entries of `language/<lang>/info.txt`:
  197 of its 199 entries; `dlg_CastleBuilding` and `dlg_CastleBuildingInfo` are named by
  no script (the castle has none). Confidence: high for the calls and arguments
  (registration, infoText, infoLabel,
  infoLabel2, infoPicture); medium for the effect of infoLabel's fourth
  argument. Covers (inventory): *script* from `data/scripts/info/*`. Covers
  (inventory): *UI text* from `language/gb/info.txt` except `dlg_CastleBuilding`,
  `dlg_CastleBuildingInfo`.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| registered functions | 148 names, 147 handlers | | high | E-fmt-002, recounted E-trn-001 |
| functions called by shipped scripts | 136 (12 never called) | | high | decoded scripts, recounted |
| table name | `properties` | | high | E-fmt-002 |
| script files run by `data.lua` | 15 | | high | decoded `data.lua` |
| objects created by shipped scripts | gp 67, bp 33, pa 73, dp 38, dd 112, rm 3, an 3, sp 1, sn 104, ai 3 | | high | decoded scripts |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | all 148 functions provided, recording calls | run `data.lua` with a Lua 5.0-compatible interpreter | finishes without error; no call to an unknown function |
| T2 | as T1 | count `*_create` calls | gp 67, bp 33, pa 73, dp 38, dd 112, rm 3, an 3, sp 1, sn 104, ai 3 |
| T3 | as T1 | collect all ids | all distinct within a family; every id fits in 32 bits; at least one pattern id has fewer than 8 hex digits |
| T3a | id strings | read `"13374e4"`, `"0xcea2bcc3"`, `"zz"`, `"ff00ff00"` | 0x013374E4, 0, 0, 0xFF00FF00 |
| T3b | goods script | type of `wood` (no `gp_setType` call) | 0, ware |
| T4 | as T1 | load order | the first `properties` call is `dd_create`; `cfg_setSettlerSkeletonDoodad` receives a handle that `dd_create` returned earlier |
| T5 | as T1, on a case-sensitive file system | run | `goods.lua` / `jobs.lua` resolve to `Goods.lua` / `Jobs.lua` |
| T6 | as T1 | check never-called set | none of the 12 functions marked "not called" is called |
| T7 | T1 + a decoded map | resolve ids | every pattern, deposit, doodad, animal, good and sound id in `data/maps/campaignMaps/campaign01.s2m` was created by a script |
| T8 | as T1 | `si_addSignDoodadId` | called 14 times; exactly one call has -1 as its first argument |
| T9 | as T1 | `pa_setMiniMapColor` | called once per pattern set up through the helper functions, always with three numbers (never `nil`) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-fmt-002 | code reading | `S2DNG.exe` | registration of the `properties` table: all 148 names; `bp_setSecondGoods` and `bp_setAlternativeGoods` share one handler |
| E-trn-001 | code reading | | reading the id string (LUA-PROP-2.1a) |
| E-trn-001 | code reading | | new good: type field 0; `gp_setType` writes that field |
| E-fmt-002 | code reading | `cfg_*` handlers | settings slots and built-in defaults |
| E-lua-001 | script survey | decoded `data/scripts/properties/*.lua` | call counts, argument shapes, constants, load order (`data.lua`) |
| E-lua-002 | map data | `FMT-s2m-map.md` | ids in maps resolve to script-created objects |

## Open questions

- Is a handle the id itself? (LUA-PROP-2.2)
- Exact semantics of the "low" confidence rows; each needs its consumer traced
  when the corresponding system is specified (Q-004, Q-005, Q-007).
- Behaviour on bad arguments (unknown handle, wrong type).
- The map scripts use other tables (`menu.*`, and callbacks `onLoaded`,
  `onStarted`, `onLogicTick`, `onTriggerIsActive`) and the menu configuration
  uses more; those APIs are not covered here. (Q-020)
- The per-object info scripts in `data/scripts/info/` and their API. (Q-011)
