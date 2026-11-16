# FMT-S2M: The `.s2m` map file

Status: draft (layout verified, several field meanings open)
Owner question(s): Q-002 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

A `.s2m` file is the complete starting state of a game world: map header (name,
size, player slots, start positions), a height grid, per-field terrain and state
layers, and the lists of objects placed on the map (trees and stones, decorative
doodads, animals, ambient sound emitters, script trigger regions). It is the same
kind of stream the game uses for saving its world, written before the first
logic tick. On disk the file is wrapped in the `rc00` container
(`FMT-rc00-container.md`, fixed key); everything below describes the contents
after unwrapping.

Verification status: a reader written from these rules consumes all 34 maps in
`data/maps/` to the last byte, with every marker, count and size check passing.
Revision of 2026-10-13: every version in FMT-S2M-1.6 and every "always",
"never" and range statement was re-counted over the 34 maps with
a survey script and the marker versions were read from the binary; the
statements that were wrong are corrected and listed under "Corrections".
The two demo maps in `data/map/` use an older header version and are out of scope
(FMT-S2M-1.6).

## Rules

All integers are little-endian. Sizes are in bytes.

### FMT-S2M-0 Which files

- **FMT-S2M-0.1 Shipped maps (2026-10-24).** Every `.s2m` under `maps/` is a map of this
  format: 20 free-game maps, 10 campaign maps and the 4 tutorial maps; a reader written
  from these rules consumes all 34 (see the verification status above). Confidence:
  high. Covers (inventory): *free game map* from `maps/freeGameMaps/*.s2m`. Covers
  (inventory): *campaign mission* from `maps/campaignMaps/*.s2m`. Covers (inventory): *tutorial mission* from `maps/tutorialMaps/*.s2m`.

### FMT-S2M-1 Stream primitives

- **FMT-S2M-1.1** `u32` / `i32`: 4 bytes. `u64`: 8 bytes. `f32`: 4 bytes IEEE
  single. `bool`: 4 bytes, value 0 or 1. `string`: `u32` length followed by that
  many bytes, no terminator (Latin-1 / ASCII).
- **FMT-S2M-1.2** A **marker** opens every object. It is 12 bytes: `u32` version of
  that object's layout, `u32` CRC-32 (zlib polynomial) of the object's section
  name, `u32` length of the section name. Example: `MapInfo` gives CRC
  `0xC835B682` and length 7. A reader must check CRC and length and use the
  version to select the layout.
- **FMT-S2M-1.3** A **field cursor** is an object: marker `PatternCursor`
  (version 0) then `u32 x`, `u32 y`. It either addresses one map field,
  `0 <= x < width`, `0 <= y < height`, or is the **none cursor**: both values
  all bits set (`0xFFFFFFFF`, `0xFFFFFFFF`). None is the value a cursor has
  before it is first set; the game does not range-check cursors on load. A
  reader must accept none wherever the rules below say so, and must reject any
  other out-of-range value. In the release maps none occurs in exactly one
  place, the third cursor of an animal's movement state (FMT-S2M-5.4): 3927 of
  the 12852 cursors read there. All other 56,911 field cursors are in range.
  (In a saved game, where settlers have movement state too, expect none in
  every movement cursor that has not been used yet.)
- **FMT-S2M-1.4** A **fine cursor** is an object: marker `ElevationCursor`
  (version 0) then `u32 x`, `u32 y`. It addresses one point of the height grid
  (FMT-S2M-4.2), `0 <= x <= 4 * width`, `0 <= y <= 4 * height`; all 261,799 in
  the release maps are in range and none is the none value. The grid has four
  steps per field. Field `(x, y)` is the fine
  position `(4x, 4y)` on even rows and `(4x + 2, 4y)` on odd rows; the reverse
  mapping is in `MAP-grid-geometry.md` MAP-GRID-4.3 and 4.4. (It is not simply
  `(x div 4, y div 4)`.)
- **FMT-S2M-1.5** An **object id** is an object: marker `logic UniqueId`
  (version 0) then `u64`. All bits set (`0xFFFFFFFFFFFFFFFF`) means "none". Ids
  are unique across all object kinds in one file and smaller than
  `next_unique_id` (FMT-S2M-3.1).
- **FMT-S2M-1.6** Versions. The table gives the version every one of the 34
  release maps carries (counted at every occurrence) and the version the game
  itself writes. These are the only versions this spec defines; any other
  version must be rejected as unsupported.

  | Section | In all release maps | Game writes |
  |---|---|---|
  | `MapInfo` | 7 | 9 |
  | `Core UUID` | 0 | 0 |
  | `LogicSystem` | **7** | 7 |
  | `TriggerSystem` | 0 | 0 |
  | `TriggerObject` | 1 | 1 |
  | `MapSystem` | 0 | 0 |
  | `ElevationMap` | 1 | 1 |
  | `PatternMap` | 0 | 0 |
  | `GridStatesMap` | 0 | 0 |
  | `Map Resources` | 0 | 0 |
  | `Map Territory` | 0 | 0 |
  | `Map Exploration` | 0 | 1 |
  | `Map Continents` | 1 | 1 |
  | `Map Continent` | 3 | 3 |
  | `resources` | **4** | 4 |
  | `deposit` | 1 | 1 |
  | `Resources Animal` | 2 | 2 |
  | `Navy Movement` | 1 | 1 |
  | `Resources Path` | 0 | 0 |
  | `Movement Path Base` | 1 | 1 |
  | `Movement Base` | 0 | 0 |
  | `Movement Interpolator` | 0 | 0 |
  | `Resources AnimalRespawn` | 0 | 0 |
  | `DoodadsSystem` | 0 | 0 |
  | `DoodadsObject` | 1 | 1 |
  | `Logic Ambients` | 0 | 0 |
  | `PatternCursor`, `ElevationCursor`, `logic UniqueId` | 0 | 0 |

  The maps are older than the game in two sections (`MapInfo`,
  `Map Exploration`); the game still reads those versions, and a saved game
  will carry the newer ones, which this spec does not define. The layouts in
  FMT-S2M-3 are those of `LogicSystem` 7 and the layouts in FMT-S2M-5 those of
  `resources` 4; the loader's version tests are given there.
  (`data/map/Presentation.s2m` and `X_MediterranDemo.s2m` have `MapInfo` 1 and are
  not loadable with these rules.)
- **FMT-S2M-1.7** Type ids. Wherever an object refers to a kind of thing (terrain
  pattern, good, deposit, doodad, animal, sound) it stores the 32-bit id that the
  property scripts pass, as a hexadecimal string, to the matching `*_create`
  function (`LUA-properties-api.md`). The string may have fewer than 8 digits
  (`"13374e4"` is `0x013374E4`).
- **FMT-S2M-1.8** After the last object the file is padded with zero bytes to a
  multiple of 128. Padding is 0..127 bytes. Non-zero bytes there are an error.

### FMT-S2M-2 MapInfo (header)

In order:

- **FMT-S2M-2.1** Marker `MapInfo`.
- **FMT-S2M-2.2** `u32` count, then that many field cursors: the **start
  positions**, one per player slot the map supports (2 for `MP_2P_*`, 6 for
  `MP_6P_*`, 1 for a solo tutorial; 1, 2, 3, 4 and 6 occur). Entry *i* belongs to player index *i*. Every
  start position in the release maps lies on a field with the `ForBuilding` state.
  (The original keeps at most the first 6.)
- **FMT-S2M-2.3** `string` map name. Equals the file name without extension in all
  release maps.
- **FMT-S2M-2.4** `u32 width`, `u32 height` in fields. Release maps are square,
  80 to 240.
- **FMT-S2M-2.5 (corrected 2026-11-04, Q-1007, E-cam-003)** 8 x `i32` **player kind**, one per
  player index: 0 = no player, 1 = computer, 2 = human. Observed: 0 in all multiplayer and most
  free-game maps, 2 for the human player and 1 for computer players in campaign maps. A campaign
  game takes it from here (`CAM-campaign-missions.md` CAM-4.3a). Confidence high.
- **FMT-S2M-2.6 (corrected 2026-11-04, Q-1007, E-cam-003)** 8 records of 4 x `i32`, one per
  player index. The earlier readings were wrong: field 1 is the tribe and field 4 the strength.
  - field 1: **tribe**: 0 Romans, 1 Africans, 2 Chinese.
  - field 2: **colour**. 0..7, equal to the player index by default (254 of 272 records); two
    players of one map may share a colour (campaigns 5 and 7).
  - field 3: **team**: -1 no team (where the player is not preset), else the team number.
  - field 4: **computer strength**: 0 Weak, 1 Normal, 2 Strong, selecting AI profile 1, 2, 3
    (UI-SETUP-2.5). Ignored on a human player (campaign 10's human holds 2). Read only in map
    versions from 6 (all shipped maps).
  Confidence high (E-cam-003: the map loader reads them into the setup fields). The same record is in a save (FMT-SAV).
- **FMT-S2M-2.7** 3 x `i32`. First: 0 or 4; the game picks a different
  end-of-game screen flow when it is 4 (campaign maps 1 to 3 and
  `sp07_good_old_times`). Second: always 0. Third: 10 in 31 maps, 1 in campaign
  maps 1 to 3.
  Meanings unknown beyond that.
- **FMT-S2M-2.8** Object `Core UUID`: marker, `bool` valid (1), 16 raw bytes. A
  unique id of the map, used to match maps between players and saves
  (inferred from the name and from being unique per file).

### FMT-S2M-3 LogicSystem and triggers

- **FMT-S2M-3.1** Marker `LogicSystem` (version 7), then `u64 next_unique_id`,
  `bool` (1), then 3 x `f32` clock values (frame step, time of next tick,
  elapsed logic time: all 0.0 in a map). `next_unique_id` is greater than every
  id in the file; it is exactly the largest id plus one in 25 of the 34 maps
  and larger in the other 9 (ids of objects deleted in the editor are not
  reused).
- **FMT-S2M-3.1a** Version tests of the loader, for reference: a version from 1
  to 5 has one more `u32` after the `bool` (ignored); the three clock values
  exist from version 4; the trigger system (FMT-S2M-3.2) from version 5; the
  tick counter (FMT-S2M-3.4) from version 7. Only version 7 has to be read.
- **FMT-S2M-3.1b** After reading the clock values the game corrects them: if the
  elapsed logic time is not smaller than the time of the next tick, it is set
  to that time minus 0.01. For a map (all three 0.0) the game therefore starts
  with elapsed time -0.01.
- **FMT-S2M-3.2** Object `TriggerSystem`: marker, `bool` (1), `u32` count, then
  that many trigger objects.
- **FMT-S2M-3.3** Trigger object: marker `TriggerObject`, `bool` (1), object id,
  `u32` kind (2 or 3 observed), field cursor (position), `u32` (0), `bool` (0),
  `string` name, `u32` (0 or 2), `f32` (0.1 .. 5.0, differs per trigger).
  **The fields' meaning (2026-11-14, Q-187 a; E-cam-009, code reading):** the `bool` after the
  first `u32` is the trigger's **active flag** (`CAM-campaign-missions.md` CAM-2.2, false in every
  map); the `u32` after the name is the **player** whose exploration or territory the trigger
  tests (CAM-2.1; 2 for campaign05's `RegionVictory__1` .. `__3`, 0 elsewhere); the `f32` is the
  trigger's **timer**, the seconds already gathered toward its next 5-second check (read only when
  the record's version is 1 or more, else 0); the first `u32` (0 in every map) is kept but no
  trigger rule reads it. (The earlier "probably an animation phase" is withdrawn.) The name is what map scripts receive in their
  `onTriggerIsActive(name)` callback, e.g. `Region_K`, `RegionVictory__0`. Only
  campaign maps contain triggers. Radius or shape of a region is not stored here
  and must come from the kind; unknown.
- **FMT-S2M-3.4** `i32` logic tick counter. -1 in every map: no tick has run. The
  first tick of a game is tick 0. (See `ECO-time-units.md`.)

### FMT-S2M-4 Map layers

"Field index" below means `y * width + x`; all per-field arrays are stored in that
order (row by row, `x` fastest).

- **FMT-S2M-4.1** Marker `MapSystem`, `bool` (1), `u32 width`, `u32 height`
  (repeats FMT-S2M-2.4; must match).
- **FMT-S2M-4.2** **Height grid.** Marker `ElevationMap`, `bool` (1), `i32` height
  floor (-100 in all maps; heights below it count as this value when the game
  averages the height of an area, `MAP-grid-geometry.md` MAP-GRID-6.2),
  `u32 ew`, `u32 eh`, then `ew * eh`
  x `i32` heights, row by row. `ew = 4 * width + 1`, `eh = 4 * height + 1`: four
  height samples per field in each direction plus the closing edge. Heights are
  signed fixed-scale values; observed range -29717 .. +50587 over all maps, open
  sea floor around -7000 .. -10000, start positions between about 300 and 10000.
  Water level is 0: fields with the `Water` state have negative heights. 1000
  height units are one world unit, one sixth of the distance between two
  neighbouring fields. The samples form a triangular lattice with odd rows
  shifted half a step; see `MAP-grid-geometry.md` MAP-GRID-4 and 5.
- **FMT-S2M-4.3** **Terrain.** Marker `PatternMap`, `bool` (1), `u32` count
  (= width * height), then count x `u32` pattern type id (FMT-S2M-1.7, created by
  `pa_create`). 71 distinct patterns occur in the release maps.
- **FMT-S2M-4.4** **Field state.** Marker `GridStatesMap`, `bool` (1), `u32` count
  (= width * height), then count x `u32` bit set:

  | Bit | Value | Name | Observed meaning | Confidence |
  |---|---|---|---|---|
  | 0 | 0x0001 | Blocked | generally unusable (swamp, snow, lava, some decor) | medium |
  | 1 | 0x0002 | Street | never set in a map | name only |
  | 2 | 0x0004 | Water | sea fields; no deposits ever; all heights negative | high (code) |
  | 3 | 0x0008 | Coast | exactly the non-Water fields with a Water neighbour | high (code) |
  | 4 | 0x0010 | ForMining | rock patterns; most underground resources | high |
  | 5 | 0x0020 | Border | never set in a map | name only |
  | 6 | 0x0040 | ElevationBlocked | steep ground | medium |
  | 7 | 0x0080 | DepositBlocked | exactly the fields holding a deposit (tree, stone) | high |
  | 8 | 0x0100 | MovementBlocked | never set in a map | name only |
  | 9 | 0x0200 | ForBuilding | buildable ground; all start positions | high |
  | 10 | 0x0400 | DoodadBlocked | exactly the fields holding a blocking doodad | high |
  | 11 | 0x0800 | LandingPosition | never set in a map | name only |
  | 12 | 0x1000 | ForHarbor | fields with the harbor pavement pattern | high |

  The names come from the map editor's list of field conditions. Bits were
  assigned by correlating each bit with the other layers over all maps. Bits 2
  and 3 are confirmed by the code that reads them (`MAP-grid-geometry.md`
  MAP-GRID-7); the editor's name list has these two in the opposite order and
  is not a bit order there. A state of 0 ("free") occurs on beach and
  shallow sea fields. The layer is stored, not recomputed, so a loader can use it
  as is; whether the game recomputes it on load is not established.
- **FMT-S2M-4.5** **Underground / fish resources.** Marker `Map Resources`,
  `bool` (1), `u32 width`, `u32 height`, then width * height pairs of
  (`u32` amount, `u32` good id). Empty: amount 0, good `0xFFFFFFFF`. Non-empty in
  release maps: amount is always 7; goods are fish (on water), coal, gold, iron
  ore, stone (granite) and water. About 19% of all fields carry a resource.
- **FMT-S2M-4.6** **Territory.** Marker `Map Territory`, `bool` (1), width, height,
  then width * height x `i32` owner player index. -1 everywhere in a map.
- **FMT-S2M-4.7** **Exploration.** Marker `Map Exploration`, `bool` (1), width,
  height, then 8 blocks (one per player index) of width * height x `i32`. All 0
  in a map (nothing explored). Version 0 (maps) stores stream `bool`s per cell,
  version 1 (saved games, FMT-SAV-2.2) the exploration state `u32` (0 or 1);
  both are 4 bytes per cell. (Confirmed 2026-11-12, Q-1041 b, E-sav-016: the loader branches on the
  version; a version-0 cell is explored when its `bool` is not 0, a version-1 cell keeps the low byte
  of its `u32`.)
- **FMT-S2M-4.8** **Continents.** Marker `Map Continents`, `bool` (1), width,
  height, then width * height x `i32` continent index per field (-1 = none),
  then `u32` count and that many continent objects, then one `u32` (2482 ..
  22157 observed): the number of fields on land continents (on all 34 shipped
  maps it equals the summed field counts of the continents that are not water;
  2026-11-16 day b, Q-1066 b, data). The player statistics' land series divides
  by it (`UI-ingame.md` UI-GAME-8.5a).
- **FMT-S2M-4.9** Continent object: marker `Map Continent`, `u32` number of
  fields, `bool` is-water, `u32` index (equals its position in the list), 4 x
  `i32` bounding box (min x, min y, max x, max y), `u32` count + that many field
  cursors (positions on that continent, empty for most; probably landing or
  harbor spots), `u32` count + that many `i32` (indices of other continents;
  probably neighbours). Continent 0 is the surrounding sea in 33 of the 34 maps;
  in `sp05_the_forest` it is a single land field at (0,0), so do not rely on
  it. 130 of the 5161 continents have positions and 2601 have neighbours. A
  continent is a connected region of walkable land or of water; these serve
  reachability checks. **The game uses the per-field indices of 4.8 as stored**
  (the running original's continent layer equals the file's, field for field:
  28 900 of 28 900 on MP_2P_Storm_Coast, heap dump of E-trace-008), and the
  computer player's landmass tests read it (`AI-construction.md` AI-CON-6.3).
  Recomputing them from the field states is close but not the same: on
  MP_2P_Storm_Coast 212 deposit fields (Blocked + DepositBlocked in the running
  game) are land in the file, and 179 doodad-blocked water fields are -1, not
  sea. So load them; do not recompute. (Corrected 2026-10-25; was "can be
  recomputed and need not be trusted".)
- **FMT-S2M-4.10** There is no road, flag or building layer in a map file. Roads
  and buildings exist only as objects in a running game.

### FMT-S2M-5 Deposits and animals

- **FMT-S2M-5.1** Marker `resources` (version 4), `bool` (1), `u32` count, then
  that many deposit records. Version tests of the loader, for reference: the
  animal list (FMT-S2M-5.3) exists from version 1, the respawn object
  (FMT-S2M-5.5) from version 3, the two closing values (FMT-S2M-5.6) from
  version 4. Only version 4 has to be read.
- **FMT-S2M-5.2** Deposit record: `u32` deposit type id (created by `dp_create`:
  trees `tree_*`, stone piles `stone_*` / `med_stone_*`; fields of grain are the
  same kind but do not occur in maps), then object `deposit`: marker, object id,
  field cursor (the field it stands on), object id (none), fine cursor (exact
  position; it is the fine position of the field itself or one of that
  position's six neighbours on the height lattice), `f32` size (1.0 = fully grown in all maps), `i32` 0, `i32` -1.
- **FMT-S2M-5.3** `u32` count, then that many animal records.
- **FMT-S2M-5.4** Animal record: `u32` animal type id (`an_create`), then object
  `Resources Animal`: marker, object id, `f32` (0.0), field cursor **home**
  (the field the animal was placed on; it roams around it and can be more than
  9 fields away), then nested movement state:
  - marker `Navy Movement`, marker `Resources Path`, object `Movement Path Base`
    (marker, `bool` has-path; if 1: `u32` count + field cursors, `bool` (1 in
    all maps), `u32` index, `bool`). The path is stored **destination first**:
    entry 0 is the destination, entry `index` is the field of the step in
    progress, entry `index + 1` is the field the animal is on. Counts 2 to 7
    occur.
  - field cursor **position**: the field the animal is on (always equal to
    cursor A below),
  - marker `Movement Base`, 3 field cursors:
    - A, current field;
    - B, destination of the walk (equal to A when not walking);
    - C, target of the step in progress or of the last step taken. **C is the
      none cursor (FMT-S2M-1.3) for an animal that has never taken a step.**
  - object `Movement Interpolator`: marker, 3 x `f32`: progress of the step
    (-1.0 when not walking), carry-over into the next step, progress per tick
    of the current or last step (0.0 if it never took one; otherwise the
    animal's speed, 0.045 or 0.18 in the maps). See `TRN-settler-movement.md`
    TRN-MOVE-2 for how these are used.
  - `i32` the animal's state (0 waiting, 1 choosing, 2 walking; ECO-FOOD-5.7),
    `i32` its counter (the wait left in state 0, the ticket in state 1;
    0 .. several hundred), object id (none). A game starts the animal in exactly
    this state with this counter (ECO-FOOD-5.13).

  The three states found in the release maps (4284 animals):

  | State | Count | has-path | A, B, C | progress | home |
  |---|---|---|---|---|---|
  | never moved | 3927 | 0 | A = B, C = none | -1.0, speed 0.0 | = A |
  | walked and stopped | 267 | 0 | A = B = C | -1.0, speed kept | differs from A in 265 |
  | walking | 90 (2.1 %) | 1 | C is a neighbour of A, B = path entry 0 | 0 or more | differs from A |
- **FMT-S2M-5.5** Object `Resources AnimalRespawn`: marker, `bool` (1), 2 x `u32`,
  then a nested timer without a marker of its own, 2 x `i32`: four values in
  all: the respawn accumulator, the respawn rate per tick, and the cell cursor's
  x and y (ECO-FOOD-5.15; 0, 40, 9, 9 on MP_2P_Storm_Coast). The rate grows with
  map area (8 for 80x80, 16 for 100x100, 21 for 125x125, 40 for 170x170, 75 for
  240x240).
- **FMT-S2M-5.6** 2 x `u32` that close the `resources` section (not part of the
  respawn object; present from `resources` version 4). Both 0 in every map.
  Meaning unknown.

### FMT-S2M-6 Doodads and ambients

- **FMT-S2M-6.1** Marker `DoodadsSystem`, `bool` (1), then three lists. Each list:
  `u32` count, then that many doodad records.
  - list 1: non-blocking decoration (bushes, flowers, small stones, ...),
  - list 2: empty in every release map (purpose unknown),
  - list 3: blocking doodads (large rocks, gates); each makes its field
    `DoodadBlocked`. In all release maps a blocking doodad sits exactly on the
    fine position of its field (FMT-S2M-1.4), so "fine position div 4" finds
    the field.
- **FMT-S2M-6.2** Doodad record: `u32` doodad type id (`dd_create`), then object
  `DoodadsObject`: marker, object id, fine cursor. (Types with a limited life
  time carry one more `i32` after the cursor; no such type occurs in a map.)
- **FMT-S2M-6.3** Marker `Logic Ambients`, `bool` (1), `u32` count, then that many
  records of `u32` sound id (`sn_create`, the `ambient_*` sounds) + field cursor.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| marker_size | 12 | bytes | high | E-fmt-002 |
| marker_hash | CRC-32 of section name | | high | data + E-fmt-002 |
| bool_size | 4 | bytes | high | data |
| elevation_steps_per_field | 4 | samples | high | E-fmt-002 |
| elevation_floor | -100 | height units | high (value), medium (use) | data, E-map-004 |
| height_units_per_world_unit | 1000 | | high (shipped setting) | `MAP-grid-geometry.md` |
| max_players | 8 | player indices | high | E-fmt-002 |
| max_start_positions_kept | 6 | | medium | E-fmt-002 |
| resource_amount_in_maps | 7 | units per field | high (observed) | data |
| file_padding | to multiple of 128 | bytes | high (observed) | data |
| none_id | `0xFFFFFFFFFFFFFFFF` | | high | data |
| empty_good | `0xFFFFFFFF` | | high | data |

## Test cases

Input: the file from the install, unwrapped per `FMT-rc00-container.md`. The
SHA-256 is of the unwrapped contents. Positions are (x, y).

| ID | Map file | Unwrapped size | Unwrapped SHA-256 |
|---|---|---|---|
| M1 | `data/maps/tutorialMaps/tutorial01.s2m` | 940544 | `8cb3678b9a7b8f0fd474e13aa4f1547b9de63722519b28639d402358cf918fc0` |
| M2 | `data/maps/campaignMaps/campaign01.s2m` | 1488768 | `a5403060eba0b55483d0f38661c68c46534e021684c70055d67ea7ace0349b6b` |
| M3 | `data/maps/freeGameMaps/MP_4P_Fight.s2m` | 839808 | `1e3e58292d55181aef8a845442519a89d9c471711ee06d93b1e2f1538d8435c8` |
| M4 | `data/maps/freeGameMaps/MP_6P_Paradise_Island.s2m` | 9002240 | `fa5f5de39ba223d5cc8b5e9b425af51b80a8fd7f8002cd4f6989948fdd67b54d` |

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | M1 | read header | name `tutorial01`, 80 x 80, start positions [(45,38)], player types all 0, third info value 10, next id 3340, tick -1 |
| T2 | M1 | height grid | 321 x 321; first value -8598, last value -480; value at fine (180,152) = 1872; min -22890, max 24015 |
| T3 | M1 | field (0,0) | pattern `0xBABEB00B` (sea ground), state `0x0004` |
| T4 | M1 | field (45,38) | pattern `0x4545FAC4`, state `0x0200`, no resource, territory -1, continent 1 |
| T5 | M1 | resources | first non-empty field in index order is (25,4): amount 7, good `0x4012E5A3` (fish) |
| T6 | M1 | object counts | 771 deposits, 6 animals, doodad lists 1416 / 0 / 40, 10 ambients, 0 triggers, 51 continents, 106 padding bytes |
| T7 | M1 | first deposit | type `0x5D936A9E`, id 12, field (38,61), fine position (154,244), size 1.0 |
| T8 | M1 | state layer | 2925 fields have Water, 2415 have ForBuilding; number of fields with DepositBlocked = 771 |
| T9 | M2 | read header | name `campaign01`, 100 x 100, start [(33,33)], player types (2,0,0,0,0,0,0,0), info values (4,0,1), next id 56747 |
| T10 | M2 | triggers | 2: `Region_K` kind 2 at (55,51); `RegionVictory__0` kind 3 at (64,70) |
| T11 | M2 | height grid | 401 x 401; first -9081, last -6976; at fine (132,132) = 503 |
| T12 | M2 | object counts | 1122 deposits, 68 animals, doodads 2427 / 0 / 130, 14 ambients, 67 continents; continent 0 is water, 4385 fields, bounds (0,0,99,99) |
| T13 | M3 | read header | name `MP_4P_Fight`, 80 x 80, starts [(27,57),(52,56),(52,24),(26,24)], next id 14153 |
| T14 | M3 | height grid | first and last value -10000; at fine (108,228) = 347 |
| T15 | M3 | object counts | 180 deposits, 56 animals, doodads 546 / 0 / 56, 14 ambients, 19 continents, 57 padding bytes |
| T16 | M4 | read header | name `MP_6P_Paradise_Island`, 240 x 240, starts [(54,160),(116,202),(185,160),(185,86),(115,43),(53,85)] |
| T17 | M4 | height grid | 961 x 961; first -6980, last -7149; min -10188, max 29537 |
| T18 | M4 | object counts | 2563 deposits, 809 animals, doodads 26426 / 0 / 1906, 418 ambients, 196 continents, 7 padding bytes |
| T19 | any of M1..M4 | consistency | every start position has ForBuilding; every deposit's field has DepositBlocked; every list-3 doodad's field (fine position div 4) has DoodadBlocked; sum of continent sizes <= width * height |
| T20 | `data/map/Presentation.s2m` | parse | rejected: unsupported `MapInfo` version 1 |
| T21 | all 34 files under `data/maps/` | parse | all succeed with only zero padding left (< 128 bytes) |
| T22 | all 34 files | marker versions | exactly those of FMT-S2M-1.6, at every occurrence: `LogicSystem` 7 (34 times), `resources` 4 (34), `MapInfo` 7 (34), `Map Exploration` 0 (34), `Map Continent` 3 (5161), `deposit` 1 (45961), `Resources Animal` 2 (4284), `DoodadsObject` 1 (215838) |
| T23 | M1 with the `LogicSystem` or the `resources` version word changed to 0 | parse | rejected as unsupported version |
| T24 | M1 | animals | 6 animals: 2 walking (has-path 1), 4 stopped (cursors A = B = C), none with a none cursor |
| T25 | M2, M3, M4 | animals | all 68, 56 and 809 animals have never moved: cursor C is the none cursor, home = position = A = B, interpolator (-1.0, 0.0, 0.0) |
| T26 | all 34 files | none cursors | 3927 in total, all of them cursor C of an animal; every other field cursor (start positions, triggers, continents, deposits, animal home / position / A / B, path entries, ambients) and every fine cursor is in range |
| T27 | all 34 files | walking animals | 90; for each: path entry 0 = B, entry `index` = C, entry `index + 1` = A, and C is a neighbour of A (`MAP-grid-geometry.md` MAP-GRID-1.2) |
| T28 | a file where a start position or a deposit field is (`0xFFFFFFFF`, `0xFFFFFFFF`), or where any cursor has x = width | parse | rejected |
| T29 | all 34 files | `next_unique_id` | greater than every object id; equal to the largest id + 1 in 25 files |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-fmt-002 | code reading | `S2DNG.exe` | header write / read: field order, version conditions |
| E-fmt-002 | code reading | | order of header, logic state, trigger system |
| E-fmt-002 | code reading | | map system: order of layers, 4x+1 rule |
| E-fmt-002 | code reading | | layer writers |
| E-fmt-002 | code reading | | deposits, animals, respawn, doodads, ambients |
| E-fmt-002 | code reading | | triggers, cursors, ids, UUID |
| E-fmt-002 | code reading | | names the saved tick counter and logic time |
| E-fmt-003 | code reading | the section markers' static initialisers | the version each section is written with (FMT-S2M-1.6, right column) |
| E-fmt-003 | code reading | | `LogicSystem` loader: version tests, clock correction |
| E-fmt-003 | code reading | | `resources` loader: version tests, the two closing values; respawn object has four values |
| E-fmt-003 | code reading | | `Map Exploration` is read in two versions |
| E-fmt-003 | code reading | | `Movement Base`: meaning of the three cursors, when each is written, the none cursor |
| E-map-008 | data survey | the 34 maps | every marker version, cursor and bool counted at the place it is read |
| E-map-001 | data survey | decoded `data/maps` | strict parse of 34 maps; value statistics |
| E-map-002 | editor strings | `S2DNGEditor.exe` | names of the field state conditions |
| E-map-003 | rendering | images drawn from the map layers | height and terrain layers give matching coherent islands |
| E-cam-009 | code reading, data | (session 2026-11-14; the shipped campaign maps) | 3.3 (Q-187 a): the trigger record's load order and which fields the trigger check reads |

## Corrections

Found by the loader implementation (Q-025, Q-026) and by the re-count that
followed. Each was a statement written from a sample instead of from a count.

| Rule | Was | Is |
|---|---|---|
| FMT-S2M-1.6 | `LogicSystem` version 0 | 7 |
| FMT-S2M-1.6 | `resources` version 0 | 4 |
| FMT-S2M-1.3 | a field cursor is always in range | it may be the none cursor; where, see FMT-S2M-5.4 |
| FMT-S2M-5.4 | first cursor of an animal is its current field | it is its home field; the cursor after the path is the current field |
| FMT-S2M-5.4 | three cursors "from, to, next", equal to the current field when idle | current, destination, step target; step target is none until the first step |
| FMT-S2M-5.4 | about 5 % of animals are walking | 90 of 4284 (2.1 %); 267 more have walked and stopped |
| FMT-S2M-5.5 | respawn object has 6 values | 4; the last 2 belong to `resources` (FMT-S2M-5.6) |
| FMT-S2M-3.1 | `next_unique_id` is the largest id + 1 | greater than every id; equal to largest + 1 in 25 of 34 maps |
| FMT-S2M-2.6 | player field 1 is always 0 | 0, 1 or 2 |
| FMT-S2M-4.2 | heights within -30000 .. +50000 | -29717 .. +50587 |
| FMT-S2M-4.9 | continent 0 is the sea in all maps | in 33 of 34 |
| FMT-S2M-1.4 (earlier revision) | field of a fine position is (x div 4, y div 4) | see FMT-S2M-1.4 |

## Open questions

- Geometry is answered in `MAP-grid-geometry.md` (Q-015).
- Meaning of the per-player header fields and the three info values
  (FMT-S2M-2.5 to 2.7). (Q-016)
- Trigger kinds 2 and 3 and their region size. (Q-017)
- Water / Coast bit order is confirmed in `MAP-grid-geometry.md` MAP-GRID-7
  (Q-018).
- Does the game recompute field states and continents on load, or trust the
  file?
- Purpose of doodad list 2 and of the trailing value after the continent list.
- Older `MapInfo` versions (demo maps) are not specified, nor the newer versions
  the game writes into saved games (`MapInfo` 9, `Map Exploration` 1). (Q-009)
- The loaders of the other sections were not re-read for version tests in this
  revision; only `LogicSystem`, `resources`, `Map Exploration` and
  `Resources AnimalRespawn` were.
- Meaning of the leading `bool` of most sections. (Q-027)
- Meaning of player field 1 and of the two closing values of `resources`.
