# FMT-SAV: The `.sav` save game

Status: draft (every section verified against three original saves, which the reference parser
reads to the last byte, one of them with an active computer player; the fight records are
unverified)
Owner question(s): plan 7.1 / 7.4; leftovers in Q-100
Game build: 11757

## Summary

A save game is a map file (FMT-S2M) with a newer header version, followed by one "game" block that
holds the complete simulation state: players, buildings, settlers, transport, military, ships,
roads and flags, computer players, statistics and script variables. Every object is written with
the same marker scheme as the map file, so the file is self-describing down to the class and
version of every object. Object references are stored as object ids and resolved after loading.

## Rules

### FMT-SAV-1 Files and folders

- **FMT-SAV-1.1** Base folder: the user's Documents folder + `S2`. Single-player saves are
  `<Documents>\S2\saves\<name>.sav`, multiplayer saves `<Documents>\S2\mp_saves\<name>.mps`, user
  maps `<Documents>\S2\maps\<name>.s2m`. A missing save is also looked for in the user maps folder.
- **FMT-SAV-1.2** `.sav` and `.mps` use the rc00 container of FMT-RC00 with the key derived from
  the file name, exactly like `.s2m`. The decoded body is FMT-SAV-2 onward.
- **FMT-SAV-1.3** The save dialog proposes the name `DD-MM-YYYY HH-MM-SS <map name>` from the
  local clock; the user may replace it. The dialog lists every `*.sav` of the saves folder without
  the extension.
- **FMT-SAV-1.4** Header cache: the game keeps `<LocalAppData>\S2\<name>.info` (saves),
  `.usermapinfo` (user maps) and `.mpsaveinfo` (multiplayer saves). An info file is a plain,
  unwrapped copy of the `MapInfo` block (FMT-SAV-2) of the file. When a list of saves is shown, an
  info file newer than its save is read instead of the save; otherwise the save's header is read and
  the info file (re)written. An implementation may ignore the cache (it is derived data).
- **FMT-SAV-1.5** Multiplayer saves are found by their `Core UUID`: all `*.sav` of the saves folder
  are scanned for the header whose UUID equals the one the host announces.
- **FMT-SAV-1.6** A `.bmp` next to a multiplayer save (same base name) is copied along with it.
  (Presentation only, not needed to load.)

### FMT-SAV-2 Header and map part

- **FMT-SAV-2.1** The file starts with the `MapInfo` object of FMT-S2M-2 written as **version 9**
  (maps are written as version 7). After the `Core UUID` (version 7 field) follow: version 8: `bool`
  and 8 x `string` player names (observed: `default` for the human, `AI` for the others); version 9:
  `u32` (observed 0 in both saves; meaning open, Q-100).
- **FMT-SAV-2.2** Then `LogicSystem` (version 7), the map layers, deposits and animals, doodads and
  ambients exactly as FMT-S2M-3 to FMT-S2M-6, with the live values: the tick counter is the current
  tick, the clocks are the current elapsed time, territory, exploration and continents hold the
  current state, animals are mid-walk. Nothing in the map part has to be recomputed on load.
  **The object counter (answered 2026-11-06, Q-1025; E-sav-009).**
  `LogicSystem`'s `u64 next_unique_id` (`FMT-s2m-map.md` FMT-S2M-3.1) is the game's one global
  object counter (`SIM-game-start.md` SIM-START-4.1) as it stood at saving: the next number a new
  object takes. Loading sets the counter from it, and every object keeps the id it was saved with.
  The Fight autosave: 24858, its largest id (a package) plus one. Confidence: high.
  **Exploration version (Q-195 a, 2026-10-17).** The map part of a save carries
  `Map Exploration` as **version 1**: the same bytes as version 0 (FMT-S2M-4.7:
  `bool` 1, width, height, 8 blocks of width x height x 4 bytes, one block per
  player index), but each cell value is the player's exploration state as a
  `u32` whose low byte counts: 0 unexplored, 1 explored (the only values seen;
  the game stores the low byte). Version 0 cells are stream `bool`s (0 / 1,
  also 4 bytes). A reader accepts both; a writer uses version 1. Confidence:
  high (E-sav-003; both sample saves).

- **FMT-SAV-2.3** The `MapInfo` field "map kind" (the third of the three closing `i32`,
  FMT-S2M-2.7) is kept from the map the game was started from. If it is 1 (campaign) or 2
  (tutorial) the loader reads the map's Lua script again from `data\maps\campaignMaps\<name>.lua`
  or `data\maps\tutorialMaps\<name>.lua` after the map part (see the campaign spec); script
  variables come from FMT-SAV-3.12.
- **FMT-SAV-2.4** Reading the player records: only the first six start positions are kept (the
  loader reads all but stores six).

### FMT-SAV-3 The game block

All objects use the marker of FMT-S2M-1.2 (version, crc32 of the class name, name length).
"object" below means: a marker of the named class followed by that class's fields. Lists are
`u32 count` followed by count elements. Object ids are `logic UniqueId` objects (FMT-S2M-1.5);
the value `0xFFFFFFFFFFFFFFFF` is "none". Class names and versions are listed in FMT-SAV-4.

- **FMT-SAV-3.1** Object `Game File Logic` (version 2) directly after `Logic Ambients`. Contents
  in order: 3.2 (version >= 1), 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 3.12 (version >= 2).
  After it the file is zero-padded (FMT-S2M-1.8).
- **FMT-SAV-3.2** `Logic Random` (v0): `bool`, 8 raw bytes: the state of the game's random
  generator. Restoring it makes the continuation deterministic; an implementation with another
  generator seeds from these bytes (medium).
- **FMT-SAV-3.3** `PlayerSystem` (v0): `bool`, then 8 slots: `i32` index; `-1` = empty slot and
  nothing follows; else the index equals the slot number and a `PlayerObject` follows.
  `PlayerObject` (v5): `bool` active; `string` name; `u32` player index; `u32`; `u32`; `u32`;
  `PlayerLocksmith` (v1: 12 x (`f32`, `f32`) tool production settings, `u32`); `Good Priorities`
  (v0: `bool` present; if present a list of `Good Priority` (v0: `u32` good id, `u32` priority),
  the transport priority order); `Good Arrangement` (v0: `bool`, list of `Player
  GoodArrangementGroup` (v0: `bool`, `Player ArrangementBase` (v0: list of `Player
  ArrangementObject` (v0: `f32`, `f32`, `u32`)), `u32`), the distribution settings); `Player
  Military` (v0: 5 x `f32`, `u32`, `f32`: the military settings sliders); `Messages` (v0: list of
  `Message` (v2: `f32` game time, field cursor, `string` title key, `string` text key, `u32`,
  `u32`, `uniqueId` (v0: `u64` id of the object the message is about), `string` (building name
  key))); if active: `Stock` (3.3a); `u32`; `Player ShipNames` (v0: `u32` next ship name index);
  `u32`; 8 x (`bool`, `bool`) per other player (diplomacy flags, meaning open); `Stock` (3.3a).
- **FMT-SAV-3.3a** `Stock` (v0): `bool`, `i32`, `u32 n`, n x (`u32` good id, `u32` amount).
  Good ids are the property ids of LUA-PROP (for example 0x8198bf23). Used for warehouse stocks,
  construction material, expedition cargo and the player's two global stocks.
- **FMT-SAV-3.4** `VillageSystem` (v0): `bool`; list of buildings: (`u32` building type id,
  `u32` owner player index, `VillageBuilding`); list of remains: (`u32`, `u32`, `VillageRemains`
  (v1: object id, field cursor, `u32`, `Blocking`, `u32`; 3.4b)); `Order System` (v2: `bool`, list of
  `Village Order` (v3), `u32`).
- **FMT-SAV-3.4a** `VillageBuilding` (v6), in order: object id; field cursor (the building's
  field); `ticker` (v0: `u32`, `u32`); `VillageDepot` (v0: `Stock` (held wares), `Stock`,
  `Need Good System` (v0: list of (`u32` good id, object id) = open requests), `Returning Good
  System` (v0: list of objects)); `VillageWorkers` (v0: `u32`, worker object ids ...);
  `VillageConstruction` (v2: `f32`, `f32`, object, object, `u32`); `Stock`;
  `VillageProduction` (v1: `bool` enabled, `u32`, `bool`); `Blocking` (v0: `bool`, field cursor,
  `u32`); object id; `Village SettlerSpawn` (v0: `u32`); field cursor (door); `Village Territory
  Updater` (v25: `bool`); `VillageBulldozing` (v1: `f32`, object id, `u32`, `u32`); `Order
  Container` (v0: list of objects); `bool`; only for a type with soldiers: `Village Military` (v2:
  object, object, `u32`, `f32`, `bool`, `u32`, rank counters (`u32` each, count = number of ranks,
  Q-100), object, `bool`), `VillageSoldiers`, `Village Attackers`, `Village Interceptors` (each v0:
  a `Settlers Container` (v0: list of object ids)); `Village CarrierRefresh` (v0: `u32`);
  `VillageGoodFlags` (v0: list of (`u32` good id, `bool`, `bool`): the stop / take-out flags of a
  storehouse); `u32`; `u32`; only for a catapult: `Village Catapult` (v0: 2 field cursors, 2 x
  `f32`, 2 x `u32`); only for a harbour: `Village Harbor` (v6: `bool`, list of `Village Landing
  Position` (v0: object, object, field cursor), list of objects, `Village Expedition` (v0: `u32`,
  `Stock`, object id, `u32`), `Order Container`, list of objects, object id, `Settlers Container`,
  `u32`, 3 x `Village HarborNeedsTransfer` (v5, no fields)); `VillageUpgrade` (v0: `bool`; if set
  `u32`, object, object). The two "u32" pairs and the exact object classes at the positions marked
  "object" are in the investigation note; they parse but are not yet named (Q-100).
- **FMT-SAV-3.4b Building sub-objects named (Q-195 b, 2026-10-17).** The
  positions 3.4a marks "object", in the order they are written, hold these
  classes (observed in both sample saves; `ref` = a `logic UniqueId` object,
  value all bits set = none):
  1. `ref` the worker's reserved outdoor target (deposit, tree or animal; none
     when idle);
  2. `ref` a second reservation (none in every sample);
  3. `VillageConstruction` (v2): `f32` remaining work of the **main** part, then `f32` of the
     **frame** (ECO-HQ-4.1; 0, 0 when finished, 1, 1 for a fresh site; answered 2026-11-04, Q-1009,
     E-sav-008), `ref` the builder at work (none otherwise), `Stock` **the
     materials lying at the site now**: one is taken out as each work unit begins (E-sav-008), so a unit under way has already used its material, `u32` the building type id. The
     first object inside `VillageUpgrade` is this same record, same order (E-sav-008);
  4. `VillageProduction` (v1): `bool` production switch, `u32` tick counter of
     the running production, `bool` (false in the samples);
  5. `Blocking` (v0): `bool` true, field cursor = the building's field, `u32`
     size class (0 small, 1 medium, 2 large, 3 mine, 4 harbour);
  6. the two plain `u32` (0 in every sample);
  7. `ref` the door flag's id (always set); `Village SettlerSpawn` `u32` (0);
  8. field cursor: a mine's current ore field (none for other types);
  9. `Village Territory Updater` (v0): `bool` territory claimed;
  10. `VillageBulldozing` (v1): `f32` levelling progress (1.0 = done), `ref`
      the leveller, `u32` 2, `u32` the building type id;
  11. `Order Container` (v0): the building's open orders (`Village Order`
      objects, see below); `bool` (not identified);
  12. `Village Military` (v2, military types): `VillageSoldiers` = ids of the
      soldiers inside, `Village Attackers` = ids, `u32` (1000, or none), `f32`
      (0 .. 0.06), `bool` (coin supply, medium), `u32` 0, one `u32` per rank,
      `Village Interceptors` = ids, `bool` (send soldiers out, medium);
  13. `Village CarrierRefresh` `u32` (0 .. 100); `VillageGoodFlags` (the
      depot's per-good lock / send-out flags); `u32` target-search counter
      (0 .. 100) and `u32` mine-search counter (0 .. 100);
  14. `Village Catapult`, `Village Harbor` (v6: `bool`; `Village Landing
      Position` list, each `ref`, `ref`, field cursor; an empty list; `Village
      Expedition` (`u32`, `Stock`, `ref`, `u32`); `Order Container`; an empty
      list; `ref`; `Settlers Container`; `u32`; 3 x `Village
      HarborNeedsTransfer`); `VillageUpgrade` (v0: `bool` running; if set `u32`
      target building type id, then a `VillageConstruction` (v2, as item 3:
      the upgrade's own construction work, builder and materials) and a
      `VillageBulldozing` (v1, as item 10: its levelling), **not two `ref`s**
      (Q-259, 2026-10-21: the 2023 sample has no upgrade running, the
      autosave of 2026-10-20 one, a guard house of the computer player; a
      reader expecting a `ref` there stops at its `VillageConstruction`
      marker, decoded offset 5035265 of that file; E-sav-004).
  `Village Order` (v3): `ref` own id, `u32` kind or good id, `bool`, `ref` the
  ordering building, 3 x `ref` (assigned settler / package / source, none in
  the samples). `VillageRemains` (v1, Q-260 a, E-sim-006 save and
  load): `ref` own id, field cursor (the building's field), `u32` (not
  identified; 130 in the 2026-10-20 autosave), a `Blocking` object (v0: `bool`
  active, field cursor, `u32` size class, 0 = small), `u32` (written from
  version 1 on; a version-0 record ends after the `Blocking`). The earlier reading
  "`ref`, `u32`" for the last two was never exercised (no remains in the 2023
  samples); the autosave's only remains record reads as above, and the object after
  the remains list is the `Order System` (12-character name, crc32 0x0f7f7f52). The meanings marked medium or "not identified" do not affect parsing.
  Confidence: high for the classes and field order (E-sav-003, the investigation's parser
  on both saves), medium for the meanings as noted.
- **FMT-SAV-3.5** `SettlersSystem` (v0): `bool`; six lists in the order workers, constructors,
  carriers, bulldozers, soldiers, specialists; each element is (`u32` **owner's player index**,
  settler object; corrected 2026-11-04, Q-299, E-sav-007: it was called the settler type
  id, for every kind it is the owner). The settler object's class is given by its own marker: `SettlersWorker` (v1),
  `SettlersConstructor` (v0), `SettlersCarrier` (v0), `SettlersBulldozer` (v0), `SettlersSoldier`
  (v3), `SettlersSpecialist` (v0). Road carriers are saved in this list like every other settler.
  Each class writes its own fields (3.5b to 3.5g) and then the common `Settlers Settler` (3.5a).
- **FMT-SAV-3.5a** Marker-less values. Inside the settler classes most state is written as plain
  values without markers. The notation below: `ref` = a `logic UniqueId` object (marker + `u64`,
  none = all ones); `good` = (`u32` valid flag, `u32` good id; `0, 0xFFFFFFFF` when empty);
  `step` = one `u32` (a state's step counter; every state object ends with one). The common part
  `Settlers Settler` (v0): `ref` own id; the movement object (3.5h); `SettlersAnimation` (v1:
  `f32`, `f32`, `u32`); `ref`; `u32` **profession id** (a good id of `Jobs.lua`; a soldier's rank,
  3.5c); `u32` current state number; 4 x `u32`; `u32`; `ref` (Q-299, E-sav-007).
  **The `ref` after the animation (named 2026-11-07, Q-257; E-sav-010, read on both
  saves)** is the **package the settler has claimed or carries**: none or a `Transport Package` id in
  every record (Fight autosave: 33 carriers and 2 workers; 2023 sample: 249 carriers and 3
  workers). A package in hands (3.6, lying 0) whose carrier is a worker is a worker taking his
  product out: the Fight autosave's two such packages are held by workers in state 36, one
  walking from his building's door to its flag (rough step toward the flag field, the package's
  flag ref = his building's flag), one on the fine lattice coming back from outside work with the
  package not yet given a flag. Put the package in that settler's hands. Confidence: high (both
  samples).
  **The state number (Q-257 b, c, e; 2026-11-07, both saves).** Values seen, with TOOL-trace's
  table: 3 **going home** (walking to a warehouse to be taken in), 4 **going to work** (to the
  building, site or flag its own `ref` names), 12 soldier inside his building, 24 .. 31 the road
  carrier's states, 32 constructor **at his site**, 33 .. 38 a worker's **job states** (their
  meaning belongs to the building type's work routine, ECO-WORK; 33 is the main work state, 36
  taking the product out). The twelve worker state objects of 3.5b keep their counters between
  uses, so several can be non-zero at once; the running one is the state number, not the first
  non-zero object.
  - (b) A **named worker** (one a building's worker list names) has his building in his own first
    `ref` (3.5b) and is in state 4 (on his way), 33 .. 38 (at work), or rarely 3. He is not always
    inside: outdoor workers stand or walk on the land in states 33 and 36 (the movement, 3.5h,
    says where).
    **The job states on the land (2026-11-08 b, E-trace-017's per-tick rows; medium):** the numbers
    are the routine's phases, the same for every job of a kind. A harvesting worker (woodcutter,
    stonecutter, hunter; the indoor producers use 33 and 36 too): **33** = resting inside, walking
    out to the work spot (rough walk, then the fine lattice) and working there (the timed action:
    the woodcutter's cut is animation 4 over 55 ticks); **36** = picking the product up, carrying
    it back over the fine lattice to the building's flag, setting it down there and walking in.
    A planting worker (farmer sowing): **34** = resting, walking out and sowing (animation 5, 83
    ticks); **37** = walking back in. So a loaded worker stands where the movement says (3.5h): in
    33 / 34 with an animation running he is working on that spot for the animation's remaining
    ticks (`SettlersAnimation`: time, total, code), with a walk under way he is on his way out to
    the walk's target; in 36 with a package (the common `ref`) he carries it to his flag; in 36
    without one, or in 37, he walks back in. An implementation restores him in the matching
    phase of its own cycle (ECO-WORK, ECO-OUTDOOR) at that place, instead of seating him inside.
    **The rest (2026-11-09, Q-257; E-sav-011: the animation record's save and the worker's state routines,
    read on the Fight autosave of 2026-11-01):**
    - **The animation record** `SettlersAnimation` is saved as (`f32` **ticks left**, `f32` total,
      `u32` code): code 0 none, 2 put down, 3 pick up, 4 / 5 / 6 work loops 0 / 1 / 2, 7 hit
      (`TOOL-trace.md`). The ticks left go down by 1 each tick; at 0 the request is cleared (code
      0), and the total stays from the last request (so (0, 55, 0) or (0, 3.3, 0) mean "nothing
      running"). Samples (all of the Fight autosave's settlers in 33 / 34 with a code set, counted
      again 2026-11-09; **professions corrected 2026-11-09 c** from each settler's profession id,
      `Jobs.lua`): a **forester planting** (2, 55, 4) on (207, 234), 2 ticks of the planting left,
      his sapling the not-yet-growing deposit 24822 (size -1, owner his hut 15623) at (207, 233);
      a **woodcutter cutting** (52, 55, 4) on (200, 78), his tree deposit 23837 at (199, 79); both
      with the fine flag set; **five** indoor producers in 33 at (323, 333, 4), (312, 333, 4),
      (291, 333, 4), (112, 333, 4), (103, 333, 4): producing inside, that many ticks left of the
      333-tick work; a **shipwright** in 34 at (43, 55, 4) on rough field (62, 52), working at his
      ship (his shipyard (59, 48)). So the forester's planting runs in 33 (his walk back in 37) and
      the shipwright's work in 34; the woodcutter's cut in 33. (Before: the (2, 55, 4) worker was
      called the woodcutter and the 34 worker was not named.) A loaded
      worker with code 4 / 5 and ticks left t finishes that action after t more ticks and then
      goes on as his routine does at the action's end.
    - **State 33 of an indoor producer (corrected 2026-11-11, Q-1040; E-sav-014; high).** The
      item of 2026-11-10 ("step by step": a wait of 100 updates, a walk out, a free walk, a door
      animation, a scaled work time) read the wrong state object, one that belongs to another
      job, and is **withdrawn**. An indoor producer's 33 is his production step and nothing else:
      `ECO-production-chains.md` ECO-PROD-3.1 .. 3.3, `ECO-worker-cycle.md` ECO-WORK-3.2 and
      ECO-PROD-4.1. (1) It begins only while the building is enabled; otherwise he is in 38 and
      tries again every update (ECO-WORK-3.1b). A miner also needs a current ore field and takes
      its unit as 33 begins (ECO-PROD-3.3). (2) Every update until the building can produce
      (ECO-PROD-3.1 a .. c): nothing. (3) In the first update that can produce, the work action
      starts: animation code 4, total = the building's production time (ECO-WORK-3.2, 333 ticks
      for the sampled producers), no factor. (4) In the update in which that action has run out,
      the building consumes its inputs and the product is decided as ECO-PROD-4 says, and 33 is
      over: 36 follows with ECO-WORK-3.3's take (drop then pickup time) and the rest of
      ECO-WORK-3. No input is taken before the work, there is no walk, no wait of 100 updates and
      no door animation in 33. Its state object (3.5b) is a single `bool`: **1 = waiting to begin
      the work** (steps 1 .. 2), **0 = the work action requested** (step 3 on). Samples, every
      indoor producer in 33 of both saves: 2023 sample 29 with 0 and a work loop running (code 4,
      total 333), 41 with 1 and no action running (the last request's total 3.3 or 333 left over),
      no other combination (the Fight autosave's five producers in 33, above, are working: code 4). So a loaded
      producer in 33 with 1 waits for his inputs (3.1) from the next tick; with 0 and t ticks left
      he works on for t ticks, then the building consumes the inputs **from the saved stock**
      (they are still in it) and makes the product, then 36.
    - **A farmer going out to harvest (2026-11-11, Q-1040; E-sav-014; high).** The farmer's 33 is
      his harvest step, `ECO-outdoor-workers.md` ECO-OUT-3.2 .. 3.5, with its own state object (3.5b:
      `u32` walk-out leg, `u32`, `f32` countdown, `step`): step **7** waiting inside for a target (the
      `f32` is ECO-OUT-3.2's countdown left), **8** walking out (ECO-OUT-3.3), **9** walking to the
      target (ECO-OUT-3.4: a free path with k = 1, then the fine approach), **10** working (ECO-OUT-3.5).
      The walk out of step 8 is one or two plain walks (k = 0): a farm has a door offset
      (ECO-WORK-3.8), so a farmer who starts on the door field first steps onto the farm's own field
      (**leg 1**, ECO-WORK-3.8b) and then walks on to the flag (**leg 0**); one who starts on the
      farm's field walks to the flag at once (leg 0). **The two walkers of the 2023 Coastal Storm
      sample that look like "walking in" are of this kind:** farmers 100504 (farm 98995 on (26, 38))
      and 298689 (farm 282373 on (83, 134)), each in step 8 with leg 1, stepping from their door
      field ((25, 39) and (82, 135)) onto the farm's field. They are going **out**, not in. Others
      in the same sample: farmers 107387 and 111474 in step 8, leg 0 (farm field to flag), 103786 and
      125984 in step 9 (k = 1), 110877, 203134, 211736, 216019, 256071 in step 7 (countdowns 27, 34,
      4, 45, 35), 91842 in step 10. Each farm with a farmer in step 8 or 9 has its reserved target
      set (3.4b 1). A loaded farmer in step 8 finishes the saved step, then (leg 1) walks from the
      farm's field to the flag, starting at progress 0 (ECO-WORK-3.8b), then goes on with ECO-OUT-3.4
      to the farm's reserved target and harvests it (ECO-OUT-3.5, 3.6), then 36.
    - **Step 7, and woodcutters and stonemasons (2026-11-12, E-sav-016, code reading and the four
      samples).** The harvest state's four saved values are, in order: the walk out's **leg**
      (step 8), the walk to the target's **part** (step 9: 0 on the rough free walk, 1 on the fine
      approach, ECO-OUT-3.4), the wait's **countdown** (step 7, a whole number 0 .. 50 stored as
      `f32`), and the step. Values of a step not running are left over from the last cycle (a
      woodcutter waiting in step 7 may show part 0 or 1): read them only for the running step.
      **A loaded worker in step 7** stays inside, invisible, and goes on with ECO-OUT-3.2 from the
      saved countdown c: each tick, while c > 0, c falls by 1; at 0 the building's reserved target
      is checked (leave when there is one, else c = 50 again and the "No resources found."
      message rule). So his first check comes in the (c + 1)-th tick after the load (c = 0: the
      first tick), not after a fresh 51. The samples: 2023 Coastal Storm farmers c = 27, 34, 4,
      45, 35 and woodcutters 104042, 228464, 253582, 271583 with c = 24, 28, 9, 35; the Fight
      autosave a farmer (20269) c = 7 and woodcutters 19739, 20180, 22192, 24719 with c = 32, 33,
      22, 12; the Hard autosave a farmer (91236) c = 44.
      **Woodcutters and stonemasons** run the very same harvest state as the farmer (the
      woodcutter's hut and the quarry share one routine, the farm has its own, all three use this
      state object), so steps 7 .. 10 and their saved values mean the same for them, and a loaded
      woodcutter or stonemason in step 8 is restored as the farmer above: finish the saved step,
      then (leg 1) the second walk to the flag from progress 0, else (leg 0) on from the flag with
      ECO-OUT-3.4 to the building's reserved target. Leg 1 only arises where the first step out is
      not the plain step to the flag (the farm's door offset, ECO-WORK-3.8); a woodcutter or
      stonemason starts that walk on his building's field, so expect leg 0. None of the four
      samples has a woodcutter or stonemason in step 8 (woodcutters: 8 in step 7, 6 in step 9, 2 in
      step 10; no stonemason in 33 at all), so this part is code only. Confidence: high for the
      countdown order (the wait's update read) and for the shared state (the routine dispatch by
      building type read); medium for "expect leg 0".
    - **His fine position (3.5h):** a worker out on the land has the movement's fine flag set; the
      four fine cursors are, as the rough ones, from, goes-to, current and target, and the closing
      fine cursor is where he stands. Working at a spot: no fine step (progress -1), current =
      from, goes-to = target = the point he works at (the forester above stands on (207, 234)
      facing his sapling's point (207, 233); the woodcutter on (200, 78) facing (199, 79)). Walking: the fine path list holds the walk's points **from the target
      back to the start**, and its index names the point the running step goes to; the walk then
      takes the points before it down to index 0 (the target). Example: points (221, 259), (220,
      259), (219, 259), (219, 260), (218, 260), index 3: stepping (218, 260) -> (219, 260) at
      progress 0.18, then (219, 259), (220, 259), (221, 259). Fine steps advance 0.09 a tick (rough
      steps 0.045 or the slope's value).
    - **The rough part of a walk out (2026-11-09 c, E-sav-012; the
      Fight autosave):** the saved movement **does** hold it. `Movement Rough`'s `Path` has kind
      (its `u32`) **1 = free path** (2 street path, 0 none), its field cursor is the walk's target,
      and the walk's field list is in `Movement Free Path`'s **`Movement Path Base`**, whose own
      `bool` active is 1. (The `bool` of `Movement Free Path` itself, before the base, is 0 in every
      sample, walking or not: it is not the active flag.) The list runs **from the target back to
      the start**, as the fine path's: point 0 = the target = the fourth rough cursor, point
      `index` = the field the running step goes to (the second cursor), point `index + 1` = the
      field it started from (the first). All 11 active free paths of the Fight autosave fit. The
      walk takes the points before `index` down to 0, the step under way at the interpolator's
      progress and rate. **The `u32` after the interpolator is the walk's stop-short count `k` of
      `TRN-free-walk.md` TRN-FREE-4.2** (the mover's own field, written there): 1 for a worker
      walking out to his object's field (he stops on the field before it, ECO-OUT-3), 2 on the
      hunter's lattice walk, 0 otherwise. Samples: **forester 21064** (state 33, hut 20118) on
      (40, 67) stepping to (39, 67) at progress 0.77 (rate 0.045), list (37, 67), (38, 67), (39,
      67), (40, 67), (41, 67), (42, 67), (43, 66), (44, 66), index 2, k = 1: he walks on to (38,
      67), one field short of (37, 67); his spot is already reserved: the not-yet-growing deposit
      24826 (size -1, owner 20118) on (37, 67) at the fine point (149, 267). **Woodcutter 15985**
      (state 33, building 15268) on (53, 63) stepping to (54, 63), list from (51, 63) to (56, 65),
      index 3, k = 1; his tree deposit 24564 (owner 15268) on (56, 65) at (227, 260). A loaded
      worker in 33 / 34 with an active free path and no fine flag is therefore on the rough part
      of his walk out: continue that walk from the saved step, and when it ends go on as the
      routine does after the walk out's free walk (the straight fine approach to the reserved
      object's point, then the work), with the object the save already ties to his building (the
      building's reserved target, 3.4b 1, or the deposit he owns).
    - **A hunter out at work (2026-11-09 c, code only, no sample: no saved game holds one; E-sav-012; medium):** the hunter's
      cycle (`ECO-outdoor-workers.md` ECO-OUT-8.1) runs in job state **33**, its taking and walk
      back in 36 as for the woodcutter. His state object (3.5b, the last of the twelve: `u32`,
      `u32`, `f32`, `step`) is (the walk out's own step, **the walk to the animal's step**, the
      wait's countdown, **the hunt step**). Hunt step **7** waiting inside for a reserved animal (the
      `f32` is the countdown left); **8** walking out (door to flag); **9** walking to the animal:
      walk step 0 = the free walk to the animal's walk-goal field (k = 1, a free path as above),
      walk step 1 = the lattice walk to the animal's position (k = 2, the fine path); when the
      lattice walk ends the animal is checked and, if the hut's reservation still holds, shot: the
      animal's state becomes **5** with its counter 0, and hunt step **10** starts the 55-tick work
      (animation code 4); **10** working: the animation's ticks left t say how much of the 55 is
      left. The animal he hunts is the hut's reserved target (3.4b 1). In the map part (`FMT-s2m-
      map.md` FMT-S2M-5.4) a reserved animal is in state **4** (hunted; its counter kept, 5.7) and a
      shot one in state **5**, whose counter **goes up by 1 each tick** from the shot (so, for a
      hunter in step 10, about 55 - t). When step 10 ends the job state passes to 36 (take the meat,
      walk back), as after the woodcutter's cut. Restore him in that phase at his saved place;
      leave the animal in its saved state (4 or 5), not roaming.
    - **State 38 is waiting:** in every routine that has it, the worker's main job state (33, or
      the planting state) could not start (nothing to do yet: no input, no spot, output full), so
      he stays in 38 and **tries to start it again in each update**; no state object runs and no
      animation. The Fight autosave's two are indoor workers standing at their building's field.
      Restore him idle at his place, retrying his job each tick.
  - (c) **Every settler that no building, street or soldier list names is in state 3**, going
    home: 2023 sample 142 workers, 218 of 219 such carriers (one in 23), 21 constructors, 7
    bulldozers, 16 specialists, 44 soldiers; Fight autosave 1 worker, 1 carrier, 2 constructors.
    Their own building `ref` is none; the walk's target (3.5h, fourth rough cursor) is where they
    are going. The 2023 sample's 396 settlers of owner 1 are all such: player 1 there owns no
    building, flag or road (a beaten player), so his settlers wander home with no home. Their
    going-home record ends with the wander failure count (`u32`, 14 for 3 of them and 15 .. 22 for the other 393 there; corrected 2026-11-09) before the
    record's own step (`TRN-free-walk.md` TRN-FREE-6.4 step 4; Q-1032 c, 2026-11-08 c).
  - (e) A **constructor** names his site in his `ref` (3.5e; named by the site's construction
    record in all 13 of the Fight autosave): state 4 walking to it, 32 at it. His work state (3.5e,
    the second group) is (`u32`, 1 or 2 in the sample, not named; `u32` c, the number of
    construction positions ordered so far in the running unit; `u32` r, the ticks **left** of the
    current 55-tick hammering step; `f32` work done per tick; `step`: 13 while he approaches the
    site on the fine lattice, 10 while he works). **c and r (corrected 2026-11-08 c, Q-1033;
    E-trn-032):** when a step has run out, the unit's update orders
    the walk to position c (TRN-FREE-5.5, 0-based), sets r to the construct time (55) and c to
    c + 1; with c = 4 it makes the extra decrement instead and the unit ends (ECO-HQ-4.6a). While r
    is at least 1 and no walk runs, each tick starts the hammering action if none runs, lowers r
    by one and makes one decrement. So he is at, or walking to, position **c - 1**: r = 55 walking
    there (no decrement yet), 0 < r < 55 working there with r decrements left (55 - r made),
    r = 0 the step is over and the next update orders the next walk or ends the unit. c = 0 only
    between a unit's start and the first walk's order. The Fight autosave's (c, r) pairs (2, 55),
    (4, 27), (3, 55), (4, 38), (4, 12), (1, 55), (3, 55), (1, 4) all fit: c 1 .. 4, three at
    position 3 mid-step. A bulldozer (leveller) likewise names his site; the samples hold none at work (the
    2023 sample's 7 are going home).
    **Mapped onto the builder of ECO-HQ-4.6b (2026-11-08 b; medium):** `step` 13 is the approach
    (4.6b step 2): he walks on to his saved walk's target and then chooses a unit (step 3).
    `step` 10 is a running unit: its material is already taken and its work per tick is the
    saved `f32` (do not recompute it). With r below 55 (and above 0) he is in the timed action at
    construction position c - 1: r decrements remain, the first in the first tick after the load,
    then the walk to position c (after position 3, c = 4: the extra decrement and the unit's end,
    4.6a). With r = 55 he is walking to position c - 1 (the movement says where; his first
    decrement there comes in the walk's arrival tick). With r = 0 the next update orders the walk
    to position c (or ends the unit when c = 4). The site's remaining work values (3.4b) already
    hold the decrements done. (Corrected 2026-11-08 c: the earlier mapping read the second
    number as ticks done and the first as the position itself.)
    **The approach, step 13 (2026-11-09, Q-257; Fight autosave):** the work state is (1 or 2, c = 0,
    r = 0, work 0.0, 13): no unit has started yet. He walks on the fine lattice (3.5h, flag set) to
    the fine path's target (the fourth fine cursor), the path read as for a worker above; e.g.
    constructor 24454: points (92, 192), (92, 193), (93, 194), (93, 195), index 2, stepping (93, 195)
    -> (93, 194) at progress 0.81. His animation is empty (0, 0, 0). **The first number is the
    approach's leg** (corrected 2026-11-09 c, Q-1037, E-sav-012): **1** = on the way out,
    walking from the flag field's sample to the point 3 lattice steps in direction 5 from it;
    **2** = on the way back to the flag field's sample (a working constructor keeps 2). With 2,
    when the saved walk ends the approach is done and he chooses his first unit in that tick
    (ECO-HQ-4.6b step 3). With 1, when the saved walk ends he first starts the walk back to his
    flag field's sample (the leg becomes 2), and chooses only when that walk ends. The four
    samples: 24454 (2, walking to (92, 192), the flag field (23, 48)'s sample) and 24416 (2, to
    (86, 196), flag (21, 49)) are on the way back; 24602 (1, to (227, 79), 3 steps from the flag
    sample (226, 76)) and 24675 (1, to (89, 163), from (88, 160)) are on the way out. (Before: "when
    the walk ends he chooses his first unit" for both.) **A check for a working constructor:** his animation's
    ticks left are r + 1 while the hammering runs (samples (28, 55, 4) with r = 27, (39, 55, 4) with
    38, (13, 55, 4) with 12).
  Confidence: high for the counts and the refs (both samples), medium for the meanings of 24 ..
  31 and of the constructor's work fields (sample values against the work step, E-pt5-001, and the site job's steps 13 approach / 10 work, E-trn-005).
- **FMT-SAV-3.5b** `SettlersWorker` (v1): `ref`, `ref`, then twelve states: (`u32`, `step`);
  (`u32`, `u32`, `f32`, `step`); (field cursor, `u32`, `step`); (`u32`, `u32`, `step`, `step`, `u32`,
  `step`, `good`, `step`, `step`); (`u32`, `u32`, `step`, `u32`, `u32`, `step`, `good`, `step`);
  (`step`, `u32`, `u32`, `step`, `step`); (`f32`, field cursor, `u32`, `u32`, `step`); (`bool`);
  (`u32`); (`step`); (`u32`, `u32`, `u32`, `u32`, `f32`, `step`); (`u32`, `u32`, `f32`, `step`).
  The twelve states are, in this order: enter building, harvest, fisher, bring home, shipyard
  carry, flag and rest, plant, produce flag, (unnamed), catapult shot, ship mode, hunter. The
  `good` values are the good the worker handles (for example plank); the rest are the work
  cycle's counters (ECO-WORK). An implementation that keeps its own cycle may write zeros and
  must only keep the field count.
- **FMT-SAV-3.5c** `SettlersSoldier` (v3): (`u32`, `step`); `ref`; `ref`; `ref`; `u32`; (`step`);
  (`step`, `step`); (`step`, `step`); (`u32`, `step`); (`step`, `step`); `ref`; `bool`;
  `SettlersLivePoints` (v0: `f32` health, `u32` healing counter).
  **The four `ref`s (2026-11-12, E-sav-016, code reading of the link setters):** the three after the
  first state are the soldier's **home building**, the **building he is sent to attack**
  (MIL-ATK-2.3) and his **fight opponent** (a soldier, MIL-ATK-5.1 / 3.2); the one before the
  `bool` is the **building he went out to defend** as an interceptor (MIL-ATK-3.1). None = not set.
  A fight in progress is these links plus the records of 3.7a.
  **Rank and health (answered 2026-11-04, Q-299, E-sav-007, read back from two
  saves).** A soldier's rank is the profession id of its common part (3.5a): 15E86843 private,
  BA0E7F83 private first class, 466C6C23 sergeant, 906014F3 officer, 559DDEA3 general
  (`MIL-soldiers.md` MIL-SOL-1.1). The health `f32` is on that rank's own scale, 1 up to the rank's
  maximum (3, 4, 5, 6, 7), a whole number in every sample; the counter is the one that heals a point
  when it passes 600. No other field holds the rank. Confidence: high.
- **FMT-SAV-3.5d** `SettlersCarrier` (v0): `u32`; `u32`; `u32`; (`step`); (`u32`, `step`); (`step`);
  `good` (the carried ware); `bool`; `ref`.
- **FMT-SAV-3.5e** `SettlersConstructor` (v0): (`step`); (`u32`, `u32`, `u32`, `f32`, `step`); `ref`.
  `SettlersBulldozer` (v0): (`step`); (`step`); `ref`.
- **FMT-SAV-3.5f** `SettlersSpecialist` (v0): field cursor; `u32`; `ref`.
- **FMT-SAV-3.5g** (reserved)
- **FMT-SAV-3.5h** Movement, class `MovementMovement` (v1): `Movement Rough` (v0: `bool`, 4 field
  cursors, `Path` (v0: `u32`, field cursor, `Movement Free Path` (v0: `bool`, `Movement Path Base`
  (v1: `bool` active; if active: cursor list, `bool`, `u32` index, `bool`)), `Movement Street Path`
  (v0: `bool`, `bool`, `bool`, `u32`, field cursor, cursor list, `ref`, `bool`, `bool`))); then
  without marker: `bool`, 4 fine cursors, `Movement Fine Path` (v0: `bool`, fine cursor list,
  `bool`, `u32`); then an optional `Movement Interpolator` (v0: 3 x `f32`; present when the mover
  has one, which every sample settler had); `u32`; fine cursor; `bool`; field cursor. The street
  and fine path fields are always present (not conditional on their first flag).
  **Where a settler stands (answered 2026-11-06, Q-257 a; E-sav-009, checked on the 359 movers of the Fight autosave).** The closing **field cursor** is the
  settler's current field and the closing **fine cursor** its current fine position. `Movement
  Rough`'s four field cursors are, in order: the field the current (or last) step **started from**,
  the field it **goes to**, the **current field** (the start field while the step runs, the goal
  field once it ends), and the **walk's target** (the current field when standing). The
  interpolator's three `f32` are the step's **progress** (-1 when no step runs, else 0 .. 1), the
  carried excess and the progress per tick. The `u32` after it is the walk's **stop-short count** k of
  `TRN-free-walk.md` TRN-FREE-4.2 (corrected 2026-11-09 c, E-sav-012: the mover's field
  written there is the one the hunter's walks set to 1 and 2; it was called the
  "stop at the next waypoint" flag). A free path's list is in its `Movement Path Base` (3.5a (b)). Sample: 275 standing (current = goes-to = target, started-from = the previous field), 58
  in a step (current = started-from, progress 0 .. 1), 26 walking on the fine lattice (fine flag
  set; their field follows the fine cursor). Confidence: high for the current field and the step
  pair (code and sample), medium for the target (sample).
- **FMT-SAV-3.6** `Transport System` (v0): `bool`; `Package System` (v0: `bool`, list of (`u32`,
  `Transport Package` (v1: `ref` id, `bool`, `bool`, `ref`, `ref`, `ref`, `u32` good id, field
  cursor, `bool`, `u32`, `u32`, 2 field cursors, `ref`))); `Transport PackageNeedSystem` (v0:
  `bool`, list of `Transport PackageNeedTarget` (v0: `ref`, `ref`)); `Transport BuildingNeedSystem`
  (v0: `bool`, list of `Transport BuildingNeedGood` (v1: `ref` building, `u32` good id, `ref`,
  `u32`, `ref`)). `Package Container` (v0: `bool`, list of objects) is the package list of a flag.
  **The package's fields (answered 2026-11-06, Q-1022; E-sav-009).** In order: id; `bool` **has a target** (set exactly when a destination is
  set: an order or a warehouse took it); `bool` **has a next flag**; `ref` **destination building**;
  `ref` **carrier** (the carrier who claimed it at its flag and comes for it, or who carries it now);
  `ref` **flag** (the flag in whose ware list it is: where it lies, or, while carried, the flag it is
  being carried to and where it holds its place; TRN-CARRY-3.2's reservation); `u32` good; field
  cursor **next flag** (TRN-CARRY-5); `bool` **lying** (1: on the ground at that flag; 0: in a
  carrier's hands); `u32` the **recheck counter** of ECO-DIST (the 200 limit of the periodic recheck);
  `u32` the **wait score** (TRN-CARRY-3.2); two field cursors and a `ref` that are reset to none
  whenever the destination or the next flag changes (none in every sample; write none). So where a
  package is: lying = 1 -> at its flag (claimed if the carrier ref is set); lying = 0 -> in the
  carrier's hands, walking toward its flag ref if one is set (with the next flag as its goal), else
  carrying it toward the next flag / into the destination building. **A flag's `Package Container`
  also lists the packages carried to it**, not only the ones lying there.
  Fight autosave (47 packages): 21 lying at flags (10 of them claimed), 21 in carriers' hands with a
  place at the flag ahead, 5 in carriers' hands with no flag (4 with a destination); 44 with a
  destination. `Transport PackageNeedTarget` (8 in the sample) pairs a **package with the warehouse
  it is being taken to for storage** (second `ref`; none while no warehouse is found: the one lying
  package without a destination). `Transport BuildingNeedGood` is a building's **order**
  (`ECO-requests-dispatch.md`): building, good, `ref` the **package serving it** (37 of 327 in the
  sample; always a package whose destination is that building), `u32` the **tick it was filed**
  (its stamp; 0 in 7), `ref` none in every sample. So the sample's 327 orders are 37 served and 290
  open; the 44 packages with a destination are the 37 order packages and 7 warehouse packages.
  Confidence: high for the order and the roles of id, destination, carrier, flag, good, next flag,
  lying, wait score and recheck counter (code and sample); medium for the target flag, the order's
  stamp and the warehouse pairing (sample values and one setter each).
- **FMT-SAV-3.7** `MilitarySystem` (v0): `bool`; `Military Recruting` (v0: `u32`); `Military
  Distances` (v0: `u32`); `Military Allocation` (v0: `u32`); `Military Fight` (v1: three lists,
  each a `u32` count and then its records, every record written with its own marker; all empty in
  the samples, so a writer may write three zero counts). The lists and their records (2026-10-28,
  FMT-SAV-3.7a; E-sav-006):
  1. the attacks whose soldiers are on their way or waiting, records `Military Fighters` (v2);
  2. the attacks whose two soldiers have both arrived and are fighting, records `Military
     Fighters` (v2) as well;
  3. the defenders (soldiers come out to meet an attacker), records `Military Defender` (v0).
  `Military Attack` (v0: no fields); `Military Training` (v0: `u32`).
- **FMT-SAV-3.7a The fight records.** `Military FightConnections` (v0) is no list of its own. It is
  the link record that both kinds of fight record carry: `bool` (in use); three object
  references (`ref`): the attacking soldier, his opponent, the building under attack.
  `Military Fighters` (v2): `bool` (active); `Military FightConnections`; `u32` (the round
  counter of the duel); `bool` (the attacker has arrived); `bool` (the opponent has arrived);
  field cursor (the field where the attacker waits; none: -1); `bool` (not named); `bool` (a hit
  pending in the current round). `Military Defender` (v0): `bool` (active); `Military
  FightConnections`; `u32` (1 when the defender's own home is the building attacked, 0 for one
  sent from another building); field cursor (the meeting field). Confidence: high for the order
  and the types (the three save functions read, E-sav-006; markers from the static initialisers), medium for
  the meanings (from the fight code's use of the fields, `MIL-attack-combat.md`), unverified
  against a save with a fight in progress.
  **Reading a save with a fight in progress (2026-11-12, E-sav-016, code reading of the loaders
  against the savers).** The three loaders read exactly what the savers write, in the same order,
  so the layout above holds for non-empty lists. A loader:
  1. reads `Military Fight`'s first two counted lists as `Military Fighters` records and the third
     as `Military Defender` records; the third list is present only when the `Military Fight`
     block's version is 1 or more (every save of this build writes 1; a version-0 block ends after
     the second list);
  2. restores each record into the matching state of `MIL-attack-combat.md`:
     - **`Military Fighters`** = one attacker's pairing (MIL-ATK-2.3 .. 6): `bool` active (1); the
       link: **attacker** (the soldier sent, MIL-ATK-2.3), **opponent** (none until he is paired,
       MIL-ATK-5.1, or with an interceptor, MIL-ATK-3.2), **target** (the building attacked);
       `u32` the **round counter** of MIL-ATK-6.2 (0 or 1 when sent, +1 per round); `bool`
       **attacker arrived**; `bool` **opponent arrived** (MIL-ATK-6.1: the duel runs while both
       are set); field cursor the attacker's **waiting field** (MIL-ATK-4.3; none = -1); `bool`
       always 0 (set to 0 at creation, never set or read: write 0, ignore on load); `bool` **hit
       pending**: the previous round scored a hit that takes its point at the next round's start
       (MIL-ATK-6.3 step 1). List 1 holds the pairings not yet fighting (walking, waiting, or one
       of the two arrived), list 2 the fighting ones (both arrived), each in its list's order, which
       is the order MIL-ATK-6.3 runs the duels.
     - **`Military Defender`** = one interceptor (MIL-ATK-3): `bool` active (1); the link:
       **attacker** he is paired with (none until paired, MIL-ATK-3.2), **opponent** = the
       interceptor himself, **target** = the building attacked that he defends; `u32` 1 when that
       building is his own home, else 0 (set when he is sent; the fight never reads it: keep it for
       writing); field cursor his **meeting field** (MIL-ATK-3.3; none = -1).
  3. restores the soldiers' own links from their records (3.5c: home, attack target, opponent,
     defended building) and the buildings' `Village Attackers` / `Village Interceptors` lists from
     the building records (3.4 item 12). Every link is an id; the original rebuilds nothing else
     after reading (no post-load step for fights), so a loaded fight goes on in the next military
     update exactly where it stood: a fighting pair runs its next round (with the saved counter
     and pending hit), a waiting attacker resumes MIL-ATK-4.4, a walking soldier walks on with
     his saved state (3.5c).
  **Older record versions** (the loader accepts them; this build writes v2): `Military Fighters`
  v0 has the three `ref`s bare (attacker, opponent, target, no `Military FightConnections` marker
  and no in-use `bool`) where v1 and v2 have the link record; v0 and v1 lack the always-0 `bool`
  after the field cursor. `Military FightConnections` and `Military Defender` have one layout.
  Confidence: high for the layout, the versions and the list rule (loaders and savers read side by
  side); medium for the two flags' roles where named only from the fight code.
  **Checked against two saves with fights in progress (2026-11-13, E-sav-017, saves made by the
  original).** A free game on `MP_4P_Fight` (slot 0 human, idle; slots 1 .. 3 computer, strength
  Hard), saved by the game's own Save Game at counters 12854 and 13351, while player 2's first attack
  on player 3's building at (31, 21) was running. Both files read to the last byte with the layout
  above (`Military Fighters` v2, `Military Defender` v0, `Military FightConnections` v0); lists
  2 / 2 / 1 and 1 / 2 / 0 records. What the samples show, for a loader:
  - **The links agree both ways.** In every list-2 record the attacker's opponent link (3.5c) is the
    record's opponent and the opponent's is the attacker; the attacker's attack-target link is the
    record's target; a list-1 attacker has no opponent link (all ones) in the soldier and the record.
  - **The soldier's state is the common part's state number** (3.5a), not the first pair of 3.5c
    (that pair read (1, 3) on every soldier involved): 18 (attack) for an attacker in a duel and for
    one whose duel has ended and who walks to a new waiting field; 20 (wait) for an attacker standing
    at his waiting field without an opponent; 17 (defend) for the target's own soldiers fighting at
    its door. A loader sets each soldier's fight state from that number with the links.
  - **The sub-state objects of 3.5c** carried, in order (the values are the steps of each object):
    the walk to the target `22` in every attacker (done) and `0` in the defenders; the attack object
    (`step`, `step`) `27, 27` in a duel, `14, 27` in the attacker between duels, `0, 0` in one not yet
    paired; the defend object `27, 27` in a defending soldier, `0, 0` otherwise; the walk out and the
    return objects `0`. An interceptor of the target's own (list 3, home = target, its `u32` 1) has
    the target in the `ref` before the `bool` and the `bool` set; the target's other defenders have
    neither. Step 27 is the duel's "both arrived, stay" and 14 the walk to a new waiting field
    (`MIL-attack-combat.md` MIL-ATK-6.1, 4.3). Medium for these meanings (two samples, step values
    matched to the fight code's states), high for the values.
  - **An interceptor pair is in two lists:** its attacker's pairing in list 2 and the interceptor's
    record in list 3 (same link); the list-3 meeting field (32, 22) differed from the list-2 record's
    waiting field (31, 19).
  - **Where a fighting pair stands (2026-11-14, Q-1044; E-mil-010).** Both soldiers of a list-2
    pair are on the record's waiting field, the defender (or interceptor) on its lattice point and
    the attacker one lattice step off it (`MIL-attack-combat.md` MIL-ATK-6.1a): attacker 16579 on
    sample (129, 85) beside the sample (130, 84) of (32, 21) (`MAP-grid-geometry.md` MAP-GRID-4.2),
    16600 on (129, 77) beside (130, 76) of (32, 19), 17868 on (126, 77), later (127, 76), beside
    (126, 76) of (31, 19). The stored field (3.5h's closing field cursor) is the truncating field of
    that sample, so it reads (31, 21) and (31, 19) for the first two. A loader restores each soldier
    on his saved sample (3.5h's closing fine cursor) and keeps the pair's field the record's waiting field; it does not move the
    attacker onto the point. The interceptor's duel is on the list-2 waiting field; the list-3
    meeting field is only where he went first (MIL-ATK-6.1a).
  - **The round counter is not only 0 or 1 outside a duel:** the list-1 attacker 16592, never hit
    (health 3), had 9 at counter 12854 (state 18, walking on after a duel) and 21 at 13351 (state 20,
    waiting): the count is not reset when an attacker's duel ends and he takes on the next opponent.
    Keep the saved value. In a duel the counters were 5, 12 (first save) and 11 (hit pending), 14
    (second).
  - **The waiting field is set in every record**, list 1 included (never -1 in the samples).
  - Health (3.5c `SettlersLivePoints`): 1, 2 or 3 among the privates involved; the pending hit was
    set once (counter 11, the attacker at 2 against a defender at 1).
  Test T17 (the two samples). Confidence: high for the layout and the values read; the files are
  kept with the other samples, outside the repository.
- **FMT-SAV-3.8** `Navy System` (v2): `bool`; list of (`u32` owner, `Navy Ship` (v3)). `Navy Ship` (the object list at its end is a list of `ref`s, none in the samples; its `u32` before the last field cursor runs 0 .. 17 = one per ship; the first `u32` after the stock is 1 for a ship, 4 for one ship of the sample; Q-195 b):
  `ref` id; `NavyConstruction` (v1: `f32`, `f32`, `ref`, field cursor); `Navy Movement` (v1: `Navy
  Path` (v0: a `Movement Path Base`), `bool`, `Movement Base` (v0: 3 field cursors, `Movement
  Interpolator`)); `Stock` (the expedition goods, see below); `u32`; `ref`; `ref`; `u32`; `Navy Expedition` (v0: `ref`, field
  cursor, `bool`, 6 field cursors); `ref`; list of objects; field cursor; `u32`.
  **Fields named (answered 2026-11-06, Q-1024; E-sav-009).**
  `NavyConstruction`: `f32` remaining **main** work, `f32` remaining
  **frame** work (the frame is worked off first, then the main; the ship is finished when both are
  0), `ref` the **shipwright** working on it, field cursor the **coast field** it is built from
  (`ECO-food-mining.md` ECO-FOOD-7). `Navy Movement`'s `Movement Base` cursors: the ship's **water
  field** first. After the cargo `Stock`: `u32` the **state** (0 idle, 1 searching the nearest
  harbour, 2 moving to it, 3 docking, 4 waiting for expedition orders, 5 moving to the expedition
  spot, 6 transferring goods, 7 moving to a free place, 8 expedition cancelled; only updated once
  finished); `ref` the **harbour it is docked at**; `ref` the harbour it is sent to; `u32` the
  search counter; `Navy Expedition`; `ref` a further harbour link; the list is the cargo entries;
  the field cursor is the **last port position**; the closing `u32` the ship's number (its name
  index). Owner: the `u32` before the ship in `Navy System`. The Fight autosave's one ship (owner 1)
  is **still being built**: main 0.193 left, frame 0, shipwright 20692, coast (62, 52), water (63,
  52), state 1 (not yet in use), no harbour links. Confidence: high for the construction fields
  (code); medium for the harbour refs and the state (the ship update's use, not a sample with a
  sailing ship).
  **Finished ships (2026-11-07, the 2023 sample's 17; E-sav-010).**
  The `Stock` after `Navy Movement` is the ship's **expedition goods store**
  (the boards and stones loaded for an expedition), not its cargo; the cargo of a goods transfer is
  the object list near the end (entries of good, count and destination harbour; empty in every
  sample). The `u32` before each ship in `Navy System` is its player's index (the same numbering
  as the settler lists). A finished ship has main and frame 0; its place is `Movement Base`'s first
  cursor (all three equal while it stands, interpolator progress -1); the `u32` after the harbour
  refs is the counter of state 1 (counts up to 101 between two searches for a docking place);
  `Navy Expedition`'s `bool` is the expedition flag. The 2023 sample's 17 finished ships all belong
  to player 1, who owns no building, flag or road there (no harbour at all): 16 in state 1 with
  counters 5 .. 72 and no harbour refs, one in state 4 (waiting for expedition orders, expedition
  flag set, store 4 + 1 + 6 goods), numbers 0 .. 16; the 18th (number 17) is being built (main 1.0,
  frame 0.236, no shipwright, coast (145, 46)). Restore each as a finished ship of its player on its
  water field with its state, counter, store, flag, last port position and number; with no harbour
  of its player, state 1 keeps finding none and the ship moves to a free place and back to state 1
  (`SHIP-ships-expeditions.md`; whether the ship update skips a beaten player's ships was not
  read). Confidence:
  high for the field roles (code) and the counts; medium for the behaviour without a harbour.
- **FMT-SAV-3.9** `Net System` (v1): `bool`; `u32` total flag count, then per flag (`u32` owner,
  `Flag`); `u32` total street count, then per street (`u32` owner, `u32` street type id, `NetStreet`);
  then for each of the 8 players three tables, each `u32 n` nodes of (`ref` key, `u32 m`, m x (`ref`,
  `u32`)) (routing tables; empty for players without a net; may be written empty and are rebuilt on
  load, medium). `Flag` (v1): `ref` id; `ref`; field cursor; `u32 n`, n x `Net FlagLink` (v0: `ref`
  street, `ref` flag); 3 x `ref`; `Package Container`; `Net Specialist` (v0: a `Settlers
  Container`); `Order Container`; `i32`. `NetStreet` (v1): `ref` id; cursor list (the fields of the
  road); `u32`; 4 x `ref` (end flags and carriers); `Net Street Map Updater` (v0: no fields); `Order
  Container`; `u32`; `u32`; `f32`; `bool`; `bool`; 3 x `bool`. On load every street whose end flag
  id is not valid is removed.
- **FMT-SAV-3.10** `AI System` (v2): `bool`; 8 x (`AI Player`, followed by an `Ai Military Map` only
  when the slot's player exists); `u32` think counter; `Ai ResourceMap` (v1); `Ai LowResResourceMap`
  (v0); `bool` enabled. `AI Player` (v6): `bool` active; `i32` profile (-1 when none; 3 for a
  "Hard" computer player); only if active the body of FMT-SAV-3.10a (verified on E-sav-005, high).
  `Ai Military Map` (v0): `bool`, `u32` w, `u32` h, w x h `AI Military Map
  Element` (v0: 3 x `u32`), 2 x `u32`; w = h = ceil(map size / 8) (22 for 170). `Ai ResourceMap`
  (v1): `bool`, `u32` w, `u32` h (ceil(map size / 4) = 43), w x h `AI ResourceMapElements` (v0:
  list of `AI ResourceMapElement` (v0: `u32` good id (wood, coal, gold, stone, iron ore, fish),
  `u32` kind, `u32` amount, `i32`)), then 2 x `u32`, 2 x `u32`, `u32`. `Ai LowResResourceMap` (v0):
  `bool`, `u32` w, `u32` h (ceil(map size / 16) = 11), cells as above, then 2 x `u32`.
- **FMT-SAV-3.10a Active computer player.** The body has no length field: a loader must read it
  record by record to reach the `Ai Military Map` that follows. In order: field cursor (the home
  position); `Ai ConstructionSystem` (3.10c); `AI NeedsArrangement` (v0: `bool`, `Ai
  ExpansionTarget`); `AI NeedsConstruction` (v0: `bool`, `Ai ConstructionOrder`, `ref`); `Ai
  ResourceMapAdds` (v0: `bool`, `u32 n`, n x `Ai ResourceMapAddsElement` (v0: `ref`, `bool`): one
  per 4 x 4 square of the resource map, n = 43 x 43 on a 170 map); `Ai LowResResourceMapAdds`
  (v0: `bool`, `u32 n`, n x `Ai LowResResourceMapAddsElement` (v0: `u32` the expansion attempts
  counted for that 16 x 16 cell, AI-CON), n = 11 x 11); four lists, each `u32 n` then n records:
  the computer player's buildings (`Ai Building`), its needs (`Ai Need`), its food needs (`Ai
  Need`; medium for the role), its productions (`Ai Production`), each list in ascending id order;
  then twelve subsystems in this order: `Ai TerritoryMap`, `Ai CellSystem` (3.10e), `Ai
  ExpansionSystem`, `Ai MilitarySystem` (3.10f), `Ai ResourceSystem`, `Ai DestructionSystem`,
  `AI NeedCreation`, `AI LockSmith`, `AI FoodArrangement`, `AI GoodsArrangement`, `AI
  ProductionControl`, `Ai ConqueredContinents` (3.10g). Confidence: high (layout), medium (the
  roles named in 3.10a to 3.10g, from the computer player's code, `AI-*.md`).
- **FMT-SAV-3.10b Records shared by the computer player's classes.** The computer player's own
  records (buildings, needs, productions, cells) carry ids from the game's unique-id counter
  (`LogicSystem`, FMT-SAV-2.2); an `Ai Building`'s own id is the id of its building. A *pair* is two `u32`
  without a marker: a cell coordinate (column, row) of the 16 x 16-field cell grid, in `Ai
  Need` the coordinate of a 4 x 4 resource square; (-1, -1) is none. `Ai ConstructionOrder`
  (v0): `u32` mode; pair (the cell); field cursor (the point, (-1, -1) none); `u32` radius;
  `i32` building type id (-1 none); `u32` weight; `f32` time (game seconds when it was made).
  `Ai ExpansionTarget` (v0): `u32` kind (0 none); field cursor; `f32` time. A cell *marker* is
  written as `Ai Cell` version 1 (not the cell record, version 5): `bool` set; `f32` time; `f32`
  weight. `Ai ReferncesNeed`, `Ai ReferncesProduction`, `Ai ReferncesBuilding` (v0, names as
  written): `u32 n`, n x `ref`.
- **FMT-SAV-3.10c Construction.** `Ai ConstructionSystem` (v3): `bool`; `Ai ConstructionOrder`
  (the order being carried out); `u32` state; then seven parts: `AI BuildingConstruction` (v0:
  `bool`; `u32` state; `Ai ConstructionOrder`; `AI SearchConstructionPlace` (v0: `i32`; `bool`;
  `AI CellPositionItertator` (v0: pair, field cursor, 2 x `u32`); 4 x `u32`; 3 field cursors;
  `u32`; `f32` best score; `u32`); field cursor); `AI StreetConstruction` (v2: `bool`; `u32`
  state; `u64` id; `u32`; `u32 n`, n field cursors (the route); `u32 m`, m x (`u32`, record);
  field cursor; 4 x `u32`; field cursor); `AI ForesterConstruction` (v1: `bool`, `Ai
  ConstructionOrder`, `u32`); `AI StreetOptimizer` (v1: `bool`, `u32`); `AI
  StreetRouteOptimizer` (v1: `bool`; `u64` id; `u32 n`, n field cursors; 4 x `u32`); `AI
  ShipConstruction` (v0: `bool`, `Ai ConstructionOrder`, `ref`); `AI ConstructionHunter` (v0:
  `bool`, `Ai ConstructionOrder`, `u32` the cell cursor of the hunter search).
- **FMT-SAV-3.10d Buildings, needs, productions.** `Ai Building` (v1): `bool`; `ref` own id;
  `ref` building; `ref` cell; `Ai ReferncesNeed`; `Ai ReferncesProduction`. `Ai Need` (v0):
  `bool`; `ref` own id; `u32` good (property id); `ref` cell; `ref` production; `ref` consumer
  building; pair (the resource square, (-1, -1) none); `bool` waiting for an expansion; `ref`
  linked food need; `ref` need. `Ai Production` (v0): `bool`; `ref` own id; `u32` good;
  `ref` cell; `ref` building; `ref` need.
- **FMT-SAV-3.10e Territory map and cells.** `Ai TerritoryMap` (v0): `bool`, `u32` w, `u32` h
  (the military map's 8-field grid, 22 x 22 on a 170 map), w x h `AI TerritoryMap Element` (v2:
  `ref` building; `u32` military-site penalty; `u32` attack results; `u32` catapults placed;
  `u32` attack attempts). `Ai CellSystem` (v3): `bool`; `Ai CellDepotCreation` (v0: `bool`, `Ai
  ConstructionOrder`); `Ai CellExpansion` (v0: `bool`, `Ai ExpansionTarget`); `u32 n`, n x `Ai
  Cell` (v5) in creation order; `AI CellValidator` (v0: `bool`, pair (the scan cursor), pair (the
  grid size, 11 x 11)); `AI CellClearing` (v0: `bool`, pair, pair (grid size), `Ai
  ConstructionOrder`); `Ai HarborCreation` (v1: `bool`, `Ai ConstructionOrder`, `bool`). `Ai Cell`
  (v5): `bool`; `ref` own id; pair (the cell); field cursor (the centre); `Ai ReferncesBuilding`
  x 3 (depots, ordinary buildings, military buildings with the headquarters); `Ai
  ReferncesProduction`; `Ai ReferncesNeed`; marker (crowded); marker (depot wanted); `AI
  CellFull` (v0: `bool`, `u32 n`, n x (`u32` good, `AI CellFullElement` (v0: marker, `u32`
  count))); marker (clearing); `u32` size to clear; `ref` forester; `Ai ReferncesBuilding`
  (harbours); `u32` landmass; marker (harbour wanted); `ref` hunter.
- **FMT-SAV-3.10f Military.** `Ai MilitarySystem` (v8): `bool`; then ten parts: `Ai
  MilitaryBuildingCreation` (v0: `bool`; `Ai ConstructionOrder` (the military order, AI-MIL);
  `Ai ExpansionTarget` (the remembered target); 2 x `u32`); `AI MilitarySettings` (v0: `bool`,
  `u32`); `Ai AttackSystem` (v1: `bool`; `Ai AttackTargetSelection` (v0: `bool`, `Ai
  AttackTarget` (v0: pair, `u32`, `f32`, `u32`)); `Ai AttackExecution` (v1: `bool`, field
  cursor, `Ai ExpansionTarget`); `u32`); `AI SoldierCreation` (v0: `bool`); `AI
  CatapultConstruction` (v0: `bool`, pair, `f32`, pair, `u32`, `Ai ConstructionOrder`); `AI
  CastleSettings` (v0: `bool`); `AI WeaponProduction` (v0: `bool`, `u32`, `f32`); `AI
  CoinProduction` (v0: `bool`, `u32`, `f32`); `AI CoinArrangement` (v0: `bool`); `AI
  MilitaryUpgrade` (v1: `bool`, `u32`).
- **FMT-SAV-3.10g The other subsystems.** `Ai ExpansionSystem` (v1): `bool`; `Ai
  ExpansionTarget` (current); `Ai ExpansionTarget`; `u32`; `AI Expedition` (v0: `bool`). `Ai
  ResourceSystem` (v0): `bool`; `Ai ReferncesNeed` (the waiting list of needs without a cell, in
  insertion order); `Ai ExpansionTarget` (the waiting need's target); `u32` good. `Ai
  DestructionSystem` (v2): `bool`, `u32`, `u64` id. `AI NeedCreation` (v0): `bool`, 2 x `u32`.
  `AI LockSmith` (v1): `bool`, `ref`, `u32` good. `AI FoodArrangement` (v0): `bool`. `AI
  GoodsArrangement` (v0): `bool`, `u32`. `AI ProductionControl` (v0): `bool`, `u32` (a property id, 3.10h). `Ai ConqueredContinents` (v0): `bool`, `u32 n`, n x `u32` landmass index.
- **FMT-SAV-3.10h The unnamed values of 3.10e to 3.10g (answered 2026-11-04, Q-1000; E-sav-008, the
  save functions against each run function; values read back from the Fight autosave).**

  | Record | Value | Meaning | Sample |
  |---|---|---|---|
  | `Ai Cell` | `u32` size to clear | always 0: see AI-CON-9.4, nothing ever sets it | 0 |
  | `Ai MilitaryBuildingCreation` | first `u32` | the 3-run cooldown of AI-MIL-4.1 | 0 |
  | | second `u32` | the attempt count of AI-MIL-4.4 | 2 |
  | `AI MilitarySettings` | `u32` | the countdown of AI-MIL-2.2 (from 50) | 31 |
  | `AI WeaponProduction`, `AI CoinProduction` | `u32`, `f32` | (k, t0) of AI-MIL-6.2, 6.3 | 4, 186.4 |
  | `AI MilitaryUpgrade` | `u32` | the cursor of AI-MIL-4.5a | 9 |
  | `AI NeedCreation` | first `u32` | the stage of AI-CON-2 | 4 |
  | | second `u32` | n of the doubling stock check (every 600 x 2^n s) | 8 |
  | `AI ForesterConstruction` | `u32` | the run's list index of AI-CON-7.1 | 3 |
  | `Ai ExpansionSystem` | `u32` | the source of the target in progress: 0 none, 1 waiting need, 2 crowded cell, 3 anywhere, 4 attack (AI-MIL-3.1) | 1, 4 |
  | `Ai DestructionSystem` | `u32` | the flag cursor of AI-CON-9.5 | 71 |
  | | `u64` | the building cursor of AI-CON-9.2: the last building's id | 23236 |
  | `AI GoodsArrangement` | `u32` | the step of AI-CON-10.4 | 6 |
  | `AI ProductionControl` | `u32` | the last entry visited in AI-CON-10.1: a key of the property table of goods and professions, in key order, so it may name a profession | F0622D7E (the butcher) |
  | `AI LockSmith` | `ref` | the tools need of AI-CON-10.2 | |
  | | `u32` good | the last tool handled | 6CC2D2D3 (gripper) |
  | `Ai ResourceMapAdds` element | `ref` | the need placed on this square: on all three computer players of the sample every set `ref` (22, 7, 14) is the need whose own square this is, and no need with a square is missing, so it can be rebuilt from the needs | |
  | | `bool` | not named; false on all 1200 elements | false |

  A pending military order's level is not stored: it is worked out from the attempt count and the
  order's position each time an order is made (E-sav-008); its cell and source are in the
  record's `Ai ConstructionOrder` and `Ai ExpansionTarget`. Confidence: high for the roles read in
  the code, medium for the `Ai ResourceMapAdds` `ref` (one save) and for the sample column's meaning of
  "last" in the cursors.
- **FMT-SAV-3.11** `LogicStatistics` (v0): `u32`; list of (`ref` building id, `u32`, `f32`
  productivity, `u32` tick); 8 x `LogicPlayerStatistics` (v2: 8 series then 14 series, each `u32 n`
  followed by n x `u32` sample; then `u32`, `u32`, `u32`). The 2023 sample holds 960 samples per
  series for the human player and empty series for the others.
  **Fields named (answered 2026-11-06, Q-1026; E-sav-009,
  checked on the Fight autosave of 2026-11-01).** The first `u32` is the tick within the current
  600-tick **productivity** cycle (59 at tick 36058; corrected 2026-11-07, E-ui-020: it times the 0.05 decay of the building productivity entries listed after
  it, not UI-GAME-8.5a's sampling). **On load the original takes it from the save as it is** (59,
  not the tick modulo 600); the next decay pass comes when it reaches 600. The sampling of
  UI-GAME-8.5a runs on a second counter of its own (601 ticks per cycle, `AI-construction.md`
  AI-CON-1.8) that the save does not hold: a load makes a new statistics object, so that counter
  starts at 0 and the five sample steps run in the first five ticks after the load, the
  cycle's end (history push, per-cycle counters) in its 601st tick. Confidence: high for the read
  and the two counters (code); medium for "a load makes a new statistics object" (the game start
  sequence makes a new logic, E-ui-020; the load path through it was not
  followed to the end).
  The 8 series are in UI-GAME-8.5a's index order (0 land %, 1 buildings, 2 inhabitants, 3 goods, 4
  military, 5 coins produced, 6 productivity, 7 frags) and the 14 in UI-GAME-8.5e's counter order:
  the game writes each series by that index. The three closing `u32` are (1) the **coins produced so
  far** (the total that series 5 samples, raised by each mint cycle), (2) the **frags so far** (the
  total that series 7 samples, raised by each kill), (3) the player's **owned field count** at the
  last land sample (the count behind series 0's percentage, refreshed every cycle at its step 0).
  Sample: player 1 of the Fight autosave, series 5 last 3 and coins 3, series 7 last 28 and frags
  28, 873 fields; player 2 frags 69 against series 7's last 68 (a kill after the last sample).
  Confidence: high.
- **FMT-SAV-3.12** `GameScript` (v0): `bool`; `string` script path; list of (`string` name, `u32`
  value): the mission script's persistent variables (campaign spec). Empty in free games.

### FMT-SAV-4 Classes and versions

- **FMT-SAV-4.1 The classes and their versions (table added 2026-11-11, E-sav-015).** Every object
  in a save, a map or the player's profile starts with the marker of FMT-S2M-1.2 (version, CRC-32 of
  the class name, name length). Build 11757 knows the 206 classes below and **writes each with the
  version given**; a loader of the player's own files must accept exactly these (a save always
  carries the written versions, and its map part the game's own versions of FMT-S2M-1.6, e.g.
  `MapInfo` 9 and `Map Exploration` 1). Classes marked "yes" in the last column have a loader that
  tests the version and reads fewer fields for the older versions listed (in brackets, the versions
  it tests against); they occur in release maps (FMT-S2M-1.6) or in older saves. Every other loader
  reads one layout whatever the version says, so a loader may reject any other version as
  unsupported. "In": where the class occurs (map = the map part of a map file and of a save; profile
  = `users\<name>.cfg`, plain, not rc00-wrapped). Counts: occurrences in four sample saves, in
  order the 2023 Coastal Storm save, the four-player Fight autosave of 2026-11-01, the Hard game's
  autosave of 2026-10-27 and an older free-game autosave; 0 everywhere = the class occurs only in
  situations the samples do not have (fights, harbour receivers, ...). Two different classes share
  the name `Ai Cell` (versions 5 and 1); the place in the AI system's records tells them apart
  (FMT-SAV-3.10). Three classes occur in no sample and no profile (`GameCamera`, `GameSystem`,
  `SelectionSystem`).

  | Class (marker name) | Version written | In | Count in the samples (2023 / Fight / Hard / older) | Loader also reads older versions |
  |---|---|---|---|---|
  | `UserMapState` | 0 | profile | (profile) | - |
  | `UserObject` | 5 | profile | (profile) | yes (4) |
  | `Core UUID` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `Game File Logic` | 2 | save | 1 / 1 / 1 / 1 | yes (1, 2) |
  | `GameScript` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Navy System` | 2 | save | 1 / 1 / 1 / 1 | - |
  | `Navy Ship` | 3 | save | 18 / 1 / 0 / 0 | yes (1, 2, 3) |
  | `Navy ShipReceiver` | 1 | save (no sample) | 0 / 0 / 0 / 0 | - |
  | `Navy Expedition` | 0 | save | 18 / 1 / 0 / 0 | - |
  | `NavyConstruction` | 1 | save | 18 / 1 / 0 / 0 | yes (1) |
  | `Navy Movement` | 1 | map, save | 144 / 11 / 281 / 282 | - |
  | `Navy Path` | 0 | save | 18 / 1 / 0 / 0 | - |
  | `PlayerObject` | 5 | save | 2 / 4 / 2 / 1 | yes (1, 2, 3, 5) |
  | `PlayerSystem` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Message` | 2 | save | 1264 / 265 / 47 / 1 | yes (1, 2) |
  | `Player ShipNames` | 0 | save | 2 / 4 / 2 / 1 | - |
  | `Messages` | 0 | save | 2 / 4 / 2 / 1 | - |
  | `Good Arrangement` | 0 | save | 2 / 4 / 2 / 1 | - |
  | `Good Priority` | 0 | save | 74 / 148 / 74 / 37 | - |
  | `Good Priorities` | 0 | save | 2 / 4 / 2 / 1 | - |
  | `PlayerLocksmith` | 1 | save | 2 / 4 / 2 / 1 | yes (1) |
  | `Player Military` | 0 | save | 2 / 4 / 2 / 1 | - |
  | `Player GoodArrangementGroup` | 0 | save | 12 / 24 / 12 / 6 | - |
  | `Player ArrangementObject` | 0 | save | 40 / 80 / 40 / 20 | - |
  | `Player ArrangementBase` | 0 | save | 12 / 24 / 12 / 6 | - |
  | `LogicSystem` | 7 | map, save | 1 / 1 / 1 / 1 | yes (1, 4, 5, 6) |
  | `MapInfo` | 9 | map, save | 1 / 1 / 1 / 1 | yes (1 .. 9: every version, 4.1a) |
  | `Logic Random` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Stock` | 0 | save | 1058 / 349 / 139 / 8 | - |
  | `logic UniqueId` | 0 | map, save | 46045 / 11698 / 31817 / 26886 | - |
  | `LogicStatistics` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Logic Ambients` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `Settlers Container` | 0 | save | 1473 / 257 / 82 / 7 | - |
  | `LogicPlayerStatistics` | 2 | save | 8 / 8 / 8 / 8 | - |
  | `Transport System` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Transport BuildingNeedSystem` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Transport Package` | 1 | save | 303 / 47 / 53 / 1 | - |
  | `Package Container` | 0 | save | 947 / 172 / 57 / 4 | - |
  | `Package System` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Transport PackageNeedSystem` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Transport BuildingNeedGood` | 1 | save | 390 / 327 / 70 / 1 | - |
  | `Transport PackageNeedTarget` | 0 | save | 74 / 8 / 12 / 0 | - |
  | `SettlersSystem` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `SettlersSoldier` | 3 | save | 72 / 49 / 2 / 1 | - |
  | `SettlersCarrier` | 0 | save | 1539 / 225 / 70 / 3 | - |
  | `SettlersWorker` | 1 | save | 296 / 70 / 27 / 1 | - |
  | `SettlersConstructor` | 0 | save | 22 / 15 / 8 / 0 | - |
  | `SettlersBulldozer` | 0 | save | 7 / 0 / 3 / 0 | - |
  | `SettlersSpecialist` | 0 | save | 16 / 0 / 0 / 0 | - |
  | `Settlers Settler` | 0 | save | 1952 / 359 / 110 / 5 | - |
  | `SettlersLivePoints` | 0 | save | 72 / 49 / 2 / 1 | - |
  | `SettlersAnimation` | 1 | save | 1952 / 359 / 110 / 5 | - |
  | `Net System` | 1 | save | 1 / 1 / 1 / 1 | - |
  | `NetStreet` | 1 | save | 984 / 215 / 68 / 3 | - |
  | `Flag` | 1 | save | 947 / 172 / 57 / 4 | - |
  | `Net Street Map Updater` | 0 | save | 984 / 215 / 68 / 3 | - |
  | `Net FlagLink` | 0 | save | 1968 / 430 / 136 / 6 | - |
  | `Net Specialist` | 0 | save | 947 / 172 / 57 / 4 | - |
  | `ElevationCursor` | 0 | map, save | 23613 / 2431 / 23277 / 23027 | - |
  | `PatternCursor` | 0 | map, save | 24933 / 4760 / 6628 / 5136 | - |
  | `Map Exploration` | 1 | map, save | 1 / 1 / 1 / 1 | yes (1: version 0 cells are stream `bool`s, FMT-S2M-4.7) |
  | `Map Continent` | 3 | map, save | 490 / 19 / 490 / 490 | - |
  | `MapSystem` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `ElevationMap` | 1 | map, save | 1 / 1 / 1 / 1 | - |
  | `PatternMap` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `Map Continents` | 1 | map, save | 1 / 1 / 1 / 1 | - |
  | `Map Territory` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `Map Resources` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `GridStatesMap` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `SceneSettings` | 1 | profile | (profile) | - |
  | `SelectionSystem` | 0 | none seen | 0 / 0 / 0 / 0 | - |
  | `Movement Base` | 0 | map, save | 144 / 11 / 281 / 282 | - |
  | `Movement Path Base` | 1 | map, save | 2096 / 370 / 391 / 287 | yes |
  | `Movement Rough` | 0 | save | 1952 / 359 / 110 / 5 | - |
  | `Movement Street Path` | 0 | save | 1952 / 359 / 110 / 5 | - |
  | `MovementMovement` | 1 | save | 1952 / 359 / 110 / 5 | - |
  | `Movement Fine` | 0 | save (no sample) | 0 / 0 / 0 / 0 | - |
  | `Movement Interpolator` | 0 | map, save | 2096 / 370 / 391 / 287 | - |
  | `Path` | 0 | save | 1952 / 359 / 110 / 5 | - |
  | `Movement Fine Path` | 0 | save | 1952 / 359 / 110 / 5 | - |
  | `Movement Free Path` | 0 | save | 1952 / 359 / 110 / 5 | - |
  | `resources` | 4 | map, save | 1 / 1 / 1 / 1 | yes (1) |
  | `Resources AnimalRespawn` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `Resources AnimalAreas` | 0 | save (no sample) | 0 / 0 / 0 / 0 | - |
  | `Resources Animal` | 2 | map, save | 126 / 10 / 281 / 282 | - |
  | `deposit` | 1 | map, save | 258 / 64 / 3131 / 3212 | - |
  | `Navy Movement` | 1 | map, save | 144 / 11 / 281 / 282 | - |
  | `Resources Path` | 0 | map, save | 126 / 10 / 281 / 282 | - |
  | `DoodadsSystem` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `DoodadsObject` | 1 | map, save | 13578 / 526 / 19573 / 19790 | - |
  | `VillageSystem` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `VillageDepot` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `Village Harbor` | 6 | save | 1 / 1 / 1 / 0 | - |
  | `Need Good System` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `Order System` | 2 | save | 1 / 1 / 1 / 1 | - |
  | `VillageWorkers` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `VillageConstruction` | 2 | save | 345 / 113 / 46 / 2 | - |
  | `Returning Good System` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `Order Container` | 0 | save | 2277 / 501 / 170 / 9 | - |
  | `VillageBulldozing` | 1 | save | 345 / 113 / 46 / 2 | yes (1) |
  | `Village Military` | 2 | save | 175 / 28 / 8 / 1 | - |
  | `VillageSoldiers` | 0 | save | 175 / 28 / 8 / 1 | - |
  | `VillageUpgrade` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `VillageGoodFlags` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `Village Attackers` | 0 | save | 175 / 28 / 8 / 1 | - |
  | `Village SettlerSpawn` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `Village Catapult` | 0 | save | 0 / 3 / 2 / 0 | - |
  | `VillageProduction` | 1 | save | 345 / 113 / 44 / 2 | yes (1) |
  | `VillageBuilding` | 6 | save | 345 / 113 / 44 / 2 | - |
  | `Village HarborNeedsTransfer` | 5 | save | 3 / 3 / 3 / 0 | - |
  | `VillageRemains` | 1 | save | 0 / 1 / 0 / 0 | - |
  | `Village HarborNeedsTransfer` | 5 | save | 3 / 3 / 3 / 0 | - |
  | `Village HarborNeedsTransfer` | 5 | save | 3 / 3 / 3 / 0 | - |
  | `Village Expedition` | 0 | save | 1 / 1 / 1 / 0 | - |
  | `Village HarborNeeds` | 1 | save (no sample) | 0 / 0 / 0 / 0 | yes (1) |
  | `Village Landing Position` | 0 | save | 3 / 3 / 3 / 0 | - |
  | `Village HarborReceiver` | 0 | save (no sample) | 0 / 0 / 0 / 0 | - |
  | `Village Order` | 3 | save | 5 / 29 / 1 / 0 | - |
  | `Village Interceptors` | 0 | save | 175 / 28 / 8 / 1 | - |
  | `Blocking` | 0 | save | 345 / 114 / 44 / 2 | - |
  | `Village Territory Updater` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `Village CarrierRefresh` | 0 | save | 345 / 113 / 44 / 2 | - |
  | `MilitarySystem` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Military Fight` | 1 | save | 1 / 1 / 1 / 1 | yes (1) |
  | `Military Attack` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Military Fighters` | 2 | save (no sample) | 0 / 0 / 0 / 0 | yes (1, 2) |
  | `Military Training` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Military Allocation` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Military Distances` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Military Recruting` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Military FightConnections` | 0 | save (no sample) | 0 / 0 / 0 / 0 | - |
  | `Military Defender` | 0 | save (no sample) | 0 / 0 / 0 / 0 | - |
  | `uniqueId` | 0 | save | 1264 / 265 / 47 / 1 | - |
  | `AI System` | 2 | save | 1 / 1 / 1 / 1 | yes (1, 2) |
  | `AI Player` | 6 | save | 8 / 8 / 8 / 8 | yes (1, 2, 3, 6) |
  | `Ai LowResResourceMapAddsElement` | 0 | save | 0 / 75 / 121 / 0 | - |
  | `Ai LowResResourceMapAdds` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ResourceMapAddsElement` | 0 | save | 0 / 1200 / 1849 / 0 | - |
  | `Ai ResourceMapAdds` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ResourceMap` | 1 | save | 1 / 1 / 1 / 1 | - |
  | `AI TerritoryMap Element` | 2 | save | 0 / 300 / 484 / 0 | - |
  | `Ai TerritoryMap` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI Military Map Element` | 0 | save | 968 / 400 / 968 / 484 | - |
  | `Ai Military Map` | 0 | save | 2 / 4 / 2 / 1 | - |
  | `AI ResourceMapElement` | 0 | save | 1600 / 395 / 2505 / 2515 | - |
  | `AI ResourceMapElements` | 0 | save | 1970 / 425 / 1970 / 1970 | - |
  | `Ai LowResResourceMap` | 0 | save | 1 / 1 / 1 / 1 | - |
  | `Ai ConqueredContinents` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI ProductionControl` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI GoodsArrangement` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI FoodArrangement` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI LockSmith` | 1 | save | 0 / 3 / 1 / 0 | yes (1) |
  | `AI NeedCreation` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai DestructionSystem` | 2 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ConstructionSystem` | 3 | save | 0 / 3 / 1 / 0 | yes (1, 2) |
  | `Ai MilitarySystem` | 8 | save | 0 / 3 / 1 / 0 | yes (3, 4, 5, 7) |
  | `Ai ExpansionSystem` | 1 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ResourceSystem` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai CellSystem` | 3 | save | 0 / 3 / 1 / 0 | yes (1, 2, 3) |
  | `AI NeedsConstruction` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI NeedsArrangement` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai Building` | 1 | save | 0 / 112 / 43 / 0 | - |
  | `Ai Need` | 0 | save | 0 / 127 / 33 / 0 | - |
  | `Ai Production` | 0 | save | 0 / 72 / 30 / 0 | - |
  | `Ai Cell` | 5 | save | 0 / 171 / 57 / 0 (both classes) | yes (1, 3, 5) |
  | `AI ConstructionHunter` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI ShipConstruction` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI StreetRouteOptimizer` | 1 | save | 0 / 3 / 1 / 0 | yes (1) |
  | `AI StreetOptimizer` | 1 | save | 0 / 3 / 1 / 0 | - |
  | `AI ForesterConstruction` | 1 | save | 0 / 3 / 1 / 0 | - |
  | `AI StreetConstruction` | 2 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ConstructionOrder` | 0 | save | 0 / 33 / 11 / 0 | - |
  | `AI BuildingConstruction` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai HarborCreation` | 1 | save | 0 / 3 / 1 / 0 | yes (1) |
  | `AI CellClearing` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI CatapultConstruction` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai MilitaryBuildingCreation` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai CellDepotCreation` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI CoinArrangement` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI CoinProduction` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI WeaponProduction` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI CastleSettings` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI SoldierCreation` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai AttackSystem` | 1 | save | 0 / 3 / 1 / 0 | yes (1) |
  | `AI MilitarySettings` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI MilitaryUpgrade` | 1 | save | 0 / 3 / 1 / 0 | yes (1) |
  | `AI Expedition` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ExpansionTarget` | 0 | save | 0 / 21 / 7 / 0 | - |
  | `Ai AttackExecution` | 1 | save | 0 / 3 / 1 / 0 | yes (1) |
  | `Ai CellExpansion` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai ReferncesNeed` | 0 | save | 0 / 140 / 52 / 0 | - |
  | `AI CellValidator` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI CellFullElement` | 0 | save | 0 / 46 / 17 / 0 | - |
  | `AI CellFull` | 0 | save | 0 / 25 / 8 / 0 | - |
  | `Ai ReferncesProduction` | 0 | save | 0 / 137 / 51 / 0 | - |
  | `Ai Cell` | 1 | save | 0 / 171 / 57 / 0 (both classes) | yes (1) |
  | `Ai ReferncesBuilding` | 0 | save | 0 / 100 / 32 / 0 | - |
  | `AI SearchConstructionPlace` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `AI CellPositionItertator` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai AttackTargetSelection` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `Ai AttackTarget` | 0 | save | 0 / 3 / 1 / 0 | - |
  | `GameCamera` | 0 | none seen | 0 / 0 / 0 / 0 | - |
  | `GameSystem` | 0 | none seen | 0 / 0 / 0 / 0 | - |
  | `TriggerSystem` | 0 | map, save | 1 / 1 / 1 / 1 | - |
  | `TriggerObject` | 1 | map, save | 0 / 0 / 0 / 0 | yes (1) |
  | `ticker` | 0 | save | 345 / 113 / 44 / 2 | - |

  Confidence: high for the names and versions (each read from its marker's construction in the
  program; all 191 classes found in the samples carry exactly these versions, as do the map parts of
  the 34 release maps for the classes of FMT-S2M-1.6); medium for the last column (each loader
  scanned for comparisons of the version it reads; a version read handed to a helper is not
  followed). Correction: `Village Territory Updater` is written with version **0** (FMT-SAV-3.4a's
  item 9 had it right); an earlier private table gave 25, its name's length.
  **The last column is a lower bound (2026-11-12, Q-1041, E-sav-016).** The scan missed a loader
  that keeps the version in a stack slot and compares it later: `MapInfo` tests every version from
  1 to 9 (not only 1, 3, 4, 6), and `Map Exploration` does branch on its version (FMT-S2M-4.7 holds:
  one byte layout, two cell types). Both rows are corrected above; `Military Fight`, `Military
  Fighters` (FMT-SAV-3.7a) and `Military FightConnections` / `Military Defender` were read by hand
  and are right. Any other "-" may still hide a branch for an older version.
- **FMT-SAV-4.1a `MapInfo` by version (2026-11-12, Q-1041 a, E-sav-016, code reading).** The
  loader reads, in the order of FMT-S2M-2, each field only from the version given (each test is
  "at least"): the start positions (`u32` count, then that many field cursors; only the first 6
  are kept, the rest read and dropped) and the map name, always; width and height from **2**; the
  8 player kinds, always; the 8 per-player records from **3**, with 3 `i32` each up to version 5
  and 4 from **6**; the closing `i32`s: the first from **1**, the second from **4**, the third
  (map kind) from **5**; the `Core UUID` from **7** (older: none, not valid); the `bool` and the 8
  player names from **8**; the `u32` of FMT-SAV-2.1 from **9** (older: 0). So a version 8 is read
  with the names and without the `u32`; a version above 9 is read like 9. The release maps carry 7
  and the game writes 9 (FMT-S2M-1.6), so 7 and 9 are the only versions a player's files of this
  build hold.
- **FMT-SAV-4.1b Older versions are older builds' files (2026-11-12, Q-1041 c).** Every save of
  build 11757 carries exactly the written versions of 4.1, and every map it ships the versions of
  FMT-S2M-1.6; an older version can only come from a file written by an earlier build of the game
  (none of the four samples is one). A reader of this build's files may refuse any other version
  (FMT-S2M-1.6), as the program does now; the older layouts of the classes read by hand are given
  where they are (4.1a, 3.7a), the rest are not specified.
- **FMT-SAV-4.2** Loading in the original is a two-pass process: pass 1 reads all objects and
  stores references as ids; pass 2 ("post-load") walks every subsystem in the order villages, net,
  transport, settlers, resources, players, AI, map info, doodads, military, navy, map system,
  ambients and lets every object turn its ids into references and rebuild derived state (per-player
  map views, the navy's map, street endpoints). An implementation must therefore not require
  objects to be defined before they are referenced.

### FMT-SAV-5 Autosave

- **FMT-SAV-5.1** The autosave file is `<saves>\Autosave.sav` (single player) or
  `<mp_saves>\Autosave.mps` (multiplayer); it is overwritten each time.
- **FMT-SAV-5.2** Periodic autosave: the settings hold an interval in seconds of game time
  (0 = off; stored value and default: settings spec, chunk 3). The first check after a game starts
  only records the current game time. Afterwards, whenever `game time - last > interval`, the game
  posts the message `!Autosave...` (message kind 7) to the human player, saves, and sets `last` to
  the current game time. Game time is the elapsed-seconds clock truncated to whole seconds.
- **FMT-SAV-5.3 (corrected 2026-11-04, Q-298 e, E-ui-016)** Leaving a running game through the
  leave dialog (`UI-menus.md` UI-MENU-3.3) also writes the autosave when that dialog's `!Autosave...`
  box is on (setting 22, SET-2). This has nothing to do with the autosave interval of 5.2: with the
  interval Off it is still written if the box is on, and with an interval set it is not written
  on leaving if the box is off.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| mapinfo_version_in_save | 9 | | high | E-sav-001, both saves |
| game_block_version | 2 | | high | E-sav-001, both saves |
| player_slots | 8 | | high | E-sav-001 |
| tool_priority_entries | 12 | | high (data) | both saves |
| autosave_message_kind | 7 | | high | E-sav-001 |
| autosave_name | `Autosave` | | high | E-sav-001, file observed |
| info_cache_folder | `<LocalAppData>\S2` | | high | E-sav-001, folder observed |
| save_folder | `<Documents>\S2\saves` | | high | E-sav-001, folder observed |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | `Autosave.sav` (2026-10-13, map MP_2P_Storm_Coast 170x170) | rc00-decode with the name key | 0x4eb680 bytes; `MapInfo` version 9; player names `default`, 7 x `AI`; UUID 5796f44f75b56a4b9759583b507ee161 |
| T2 | same | parse map part | ends at 0x4ccb5a where `Game File Logic` v2 starts; `LogicSystem` tick counter > 0 |
| T3 | same | parse `PlayerSystem` | slot 0 = active player `default`, index 0, 12 tool priorities, 35 goods in the first stock; slot 1 = -1 (the computer player is slot 1 in the setup but written as -1: inactive players are not written, Q-100) |
| T4 | same | parse `VillageSystem` | every building parses; a building record begins with its object id and field, then ticker, depot (two stocks), workers |
| T5 | `17-03-2023 16-40-11 Coastal Storm.sav` | parse whole file | 0x6b6180 bytes, 77 bytes zero padding; a harbour building with 3 landing positions, an expedition stock and 3 empty `Village HarborNeedsTransfer` objects; 345 buildings; settler lists 296 / 22 / 1539 / 7 / 72 / 16; 303 packages, 74 package-need targets, 390 building needs; 18 ships; 947 flags, 984 streets (street type id 0x43d79823), player 0's routing tables 0 / 114 / 27 nodes; no fights; no active computer player; 960 statistic samples per series |
| T5b | Autosave of the Fight game (2026-11-01, 4 players), 49 soldiers; Coastal Storm save (2023), 72 soldiers | read rank and health by 3.5c | Fight: owners 0: 1, 1: 19, 2: 3, 3: 26; 47 privates (46 at 3, 1 at 1), 2 privates first class at 4. Coastal Storm: 2 privates at 3; 70 generals (68 at 7, 1 at 6, 1 at 5). Every health within its rank's scale |
| T5a | `Autosave.sav` | parse whole file | 103 bytes padding; settlers 1 worker, 3 carriers, 1 soldier; 1 package (wood) and 1 building need; 4 flags with 2, 1, 1, 2 links; 3 streets; AI think counter 4157; resource map 43 x 43, low-resolution map 11 x 11, military map 22 x 22 for slot 0 only |
| T5b | any sample | round trip | writing the parsed structure back byte for byte reproduces the decoded file |
| T6 | any save | file name | `<saves>\<name>.sav`; a save with a `.bmp` of the same name keeps it |
| T7 | interval 300, game time passes 301 s since the last autosave | tick | message `!Autosave...` to the human player, `Autosave.sav` rewritten |
| T7a | FMT-SAV-3.4b, the autosave of 2026-10-20 20:49 (E-sav-004) | parse `VillageSystem` | 21 buildings parse; the sixth (guard house 12e67603 of player 1 at (138, 57), id 90300) has `VillageUpgrade` running with target 0x7d00b493 (watchtower; Q-260 b: 0x92fe4e80 was a slip and occurs nowhere in the file) followed by a `VillageConstruction` and a `VillageBulldozing`; the other 20 have it not running |
| T9 | FMT-SAV-3.10a to 3.10g: E-sav-005 (autosave of the E-trace-012 game, Storm Coast, player 1 a "Hard" computer player; 1 855 355 bytes, decoded 5 431 424) | parse whole file | 41 bytes padding; think counter 18 043; slot 1 active with profile 3, home (139, 51); its body runs from 0x4f4e5b to 0x51084b, where its `Ai Military Map` starts; 43 buildings (each record's own id = its building's id), 32 needs, 1 food need, 30 productions; 8 cells, the first (8, 3) centred on (136, 57), landmass 12; 1 849 square records, 121 cell attempt counters (one of them 1); territory map 22 x 22; the construction order being carried out: mode 1, cell (9, 2), point (144, 48), radius 20, type -1, weight 20; the hunter order: cell (7, 4), type 0x3c10e223 (hunter), weight 10, cursor 8; conquered continents [12] |
| T8 | a save of a campaign map | load | the map's Lua script is loaded again and the saved script variables restored before play continues |
| T10 | FMT-SAV-3.5a (Q-257): the Fight autosave of 2026-11-01 | the 2 packages in hands whose carrier is no road's carrier | package 24852 in the hands of worker 15264 (state 36, his building 14345, on the fine lattice at (219, 250)); package 24857 in the hands of worker 18675 (state 36, building 18165, stepping from (61, 55) to (61, 54), progress 0.108, walking to (60, 50), the package's flag 18167); both workers' common `ref` after the animation names the package |
| T11 | FMT-SAV-3.5a (Q-257 c): the 2023 sample | settlers named by no building, street or soldier list | all in state 3 except one carrier in state 23; owner 1 for all 396 of player 1 |
| T12 | FMT-SAV-3.8: the 2023 sample's navy | ships | 18 of player 1: numbers 0 .. 16 finished (16 in state 1, number 3 in state 4 with expedition flag and store 4 + 1 + 6), number 17 being built (main 1.0, frame 0.236); none has a harbour ref |
| T13 | FMT-SAV-3.11: load the Fight autosave (productivity counter 59) | the counters after the load | productivity counter 59 (not 58); the next productivity decay 541 ticks after the load; sampling counter 0: the five samples in the first five ticks, the history push in the 601st |
| T14 | FMT-SAV-3.5a (b), 3.5h (2026-11-09 c): the Fight autosave of 2026-11-01 | the workers out on the land and their objects | professions: (2, 55, 4) on (207, 234) = forester (deposit 24822 at (207, 233), size -1); (52, 55, 4) on (200, 78) = woodcutter (deposit 23837 at (199, 79)); the 34 at (43, 55, 4) = shipwright. Rough walks out: forester 21064 (k = 1) steps (40, 67) -> (39, 67) at 0.77, then (38, 67), and stops there (one short of (37, 67), deposit 24826 at (149, 267)); woodcutter 15985 (k = 1) steps (53, 63) -> (54, 63), then (55, 63), (56, 64) and stops (one short of (56, 65), deposit 24564 at (227, 260)). All 11 active free paths: point index = the second rough cursor, point 0 = the fourth |
| T15 | FMT-SAV-3.5a (b) step 7: the 2023 sample, farmer 211736 (countdown 4) and woodcutter 104042 (countdown 24) | ticks after the load | 211736 checks his farm's reserved target in tick 5, 104042 his hut's in tick 25 (not 51); one without a target sets 50 and checks again 51 ticks later |
| T16 | FMT-SAV-3.7a: a `Military Fight` block (v1) with one list-2 record (both arrived, counter 3, hit pending 1) and one list-3 record | read, then one military update | the reader accepts the non-empty lists and reads on to `Military Attack` and `Military Training`; in the next update the pair's round starts with the pending hit taking 1 health from the striker of counter 3 (the defender, odd), then counter 4 |
| T17 | FMT-SAV-3.7a samples (E-sav-017): load the original's save at counter 12854 (fights 2 / 2 / 1) and run 497 ticks | the fight lists at counter 13351 against the original's second save | list 1: attacker 16592 alone (state 20), unpaired, waiting field (33, 20), round counter 21; list 2: 16600 against 18545 (counter 11, hit pending) and 17868 against 18560 (counter 14); list 3 empty; 16579 and the interceptor 16370 gone; reading alone: both files read to the last byte, every soldier's links agree with the records |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-sim-006 | code reading | VillageRemains save / load, Blocking save; the decoded autosave of 2026-10-20 20:49: 0x7d00b493 at offset 5035261 | FMT-SAV-3.4, 3.4b (Q-260), T7a |
| E-sav-001 | code reading | S2DNG.exe | save / load paths, folders, header, section order, record layouts |
| E-sav-002 | data + code reading | `Documents\S2\saves\Autosave.sav`, `17-03-2023 16-40-11 Coastal Storm.sav`; `AppData\Local\S2\MP_2P_Storm_Coast.info`; the investigation's parser and field-tree tool; S2DNG.exe | strict parse of both files to the last byte; settler, transport, military, navy, net, AI and statistics layouts; info cache content |
| E-sav-004 | data | `<Documents>\S2\saves\Autosave.sav` of 2026-10-20 20:49 (1 835 993 bytes, decoded 5 329 536; copies outside the repo) | the running-upgrade record (3.4b), T7a |
| E-sav-005 | data + code reading | `Autosave-2026-10-27-0905.sav` (SHA-256 24dc3f53...a39a); the computer player's save functions; the investigation's parser | the active computer player's body (3.10a to 3.10g), T9 |
| E-sav-006 | code reading | the saves of `Military Fight` (three counted lists, each record by its own save), `Military Fighters`, `Military Defender`, `Military FightConnections`; their static initialisers (names and versions 2, 0, 0) | FMT-SAV-3.7, 3.7a |
| E-sav-003 | the program's data | the section markers' static initialisers | class names and versions |
| E-sav-007 | code reading | the settler lists, the soldier's and the settler common part's save; sample tables | a soldier's rank and health |
| E-sav-008 | code reading | the site record; the computer player's save and load functions | Q-1000, Q-1009 |
| E-sav-009 | code reading + save | (= E-gfx-039, session 2026-11-06): package, object counter, statistics, ship and movement records; the Fight autosave of 2026-11-01 read with the investigation's parser | FMT-SAV-2.2, 3.5h, 3.6, 3.8, 3.11 |
| E-sav-012 | code reading and sample | (session 2026-11-09 c; also holds E-gfx-047): the builder's approach legs, a step on the field lattice, a field's sample point; the Fight autosave of 2026-11-01 read with a sample script (the four step-13 constructors' first number 2, 2, 1, 1 and their fine paths) | FMT-SAV-3.5a (e); `ECO-warehouse-construction.md` ECO-HQ-4.6b, T26 |
| E-sav-013 | code reading | (session 2026-11-10): the indoor routine; a job read as state 33's (it is another job's, see E-sav-014) | (withdrawn item of FMT-SAV-3.5a (b)) |
| E-sav-015 | code reading and save data | (session 2026-11-11, no bundle): every class marker's construction (name, version) re-read in the program; each loader scanned for version tests; the markers counted in four sample saves, the player's profile and the 34 release maps | FMT-SAV-4.1 |
| E-sav-016 | code reading and save data | (session 2026-11-12) the loaders of `Military Fight`, `Military Fighters` (versions 0 .. 2), `Military FightConnections` and `Military Defender` against their savers; the defender's creation (its `u32`); the soldier's link setters (attack, opponent, interception); the harvest state's save order, the wait's update, the walk to the target's part, the routine dispatch by building type; the four save samples scanned for workers in state 33 | 3.5a (b), 3.5c, 3.7a, T15, T16 |
| E-sav-017 | save data (two saves made by the original, kept outside the repository) | (session 2026-11-13) a free game on `MP_4P_Fight`, three Hard computer players, saved with the game's own Save Game at counters 12854 and 13351 during player 2's first attack; the fight lists and the soldiers involved read with the investigation's reader | 3.7a, 3.5c, T17 |
| E-sav-014 | code reading and save data | (session 2026-11-11): which state object each routine runs in 33 (the worker's constructor order checked); the indoor production job (start, update, its one saved `bool`, the miner's ore taken at the start), the farmer's harvest job and its walk out (two legs); both saves read with the investigation's parser | FMT-SAV-3.5a (b) "State 33 of an indoor producer", "A farmer going out to harvest" |
| E-sav-011 | code reading and sample | (session 2026-11-09): the animation record's save, the worker's state routines (state 38 = the main job state could not start, retried each update), the carrier states; the Fight autosave of 2026-11-01 read with the investigation's parser (workers in 33, 34, 38, constructors at step 13 and 10) | FMT-SAV-3.5a b, e; Q-257 |
| E-sav-010 | code reading | (= E-gfx-040, session 2026-11-07): the settler common part's, the worker's, the ship's and the navy's saves; the Fight autosave of 2026-11-01 and the 2023 sample read with the investigation's parser | FMT-SAV-3.5a, 3.8, T10 .. T12 |
| E-trn-032 | code reading | (= E-gfx-042, session 2026-11-08 c): the constructor's positions, ticks left and work per tick; the going-home record's failure count | FMT-SAV-3.5a (c), (e) |
| E-ui-020 | code reading | the same bundle: the statistics load, its two counters (both 0 at construction), the new-logic sequence | FMT-SAV-3.11, T13 |

## Open questions

- Q-100 (remaining): the meaning of the unnamed `u32` / `step` fields of the player, building and
  settler records (they parse; an implementation writes its own values), the meaning of the
  unnamed fields of the computer player's records (FMT-SAV-3.10b to 3.10g), the classes of
  the flag's package elements (the fight lists' records: FMT-SAV-3.7a, 2026-10-28) (never non-empty in the samples), the version-9 `MapInfo` field,
  which setting the autosave-on-leave flag mirrors, and why the computer player's slot is written
  as -1 in the autosave.
