# ECO-PLACE: Where buildings and flags may be placed

Status: draft (rules read from the binary; test values computed from the rules on the release maps, not yet checked in the game)
Owner question(s): Q-024 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Whether a building of a given size fits on a field is decided from the field
state bits of `FMT-s2m-map.md` FMT-S2M-4.4, the heights of the surrounding
fields, the player's territory and the objects already standing (buildings,
flags, roads). Every building has its flag on the neighbouring field to its
south-east, and that flag must exist or be placeable. Buildings may not touch
each other: a field next to a building can hold only roads and flags. The
state bits come from the terrain types and are kept up to date when objects
are placed or removed; the game does not recompute them from scratch during
play.

## Rules

Fields, directions (0 E, 1 SE, 2 SW, 3 W, 4 NW, 5 NE), rings and distances are
as in `MAP-grid-geometry.md`; height classes as in MAP-GRID-6.1. "Radius r"
means the field and all fields at distance 1..r. Building sizes are the
`bp_setSize` values of `buildings.lua`: small 0, medium 1, large 2, mine 3,
harbour 4. The headquarters (`castle`) is large.

### ECO-PLACE-1 Field state bits relevant to placement

- **ECO-PLACE-1.1** From the terrain type (`patterns.lua`: `pa_setBlocked`,
  `pa_setForBuilding`, `pa_setForMining`, `pa_setForHarbor`) the game derives:
  - ForMining (bit 4) = the terrain is for mining;
  - ForBuilding (bit 9) = the terrain is for building and is neither for mining
    nor blocked;
  - ForHarbor (bit 12) = as ForBuilding, and the terrain is harbour pavement;
  - Blocked (bit 0) = the terrain is blocked (swamp, snow, lava), **or a
    building stands on the field** (ECO-PLACE-5.1). A map may also carry Blocked
    on other fields (6337 ForBuilding fields in the release maps have it); a
    loader keeps the stored value.
- **ECO-PLACE-1.2** Water (bit 2) = the field's height (MAP-GRID-4.2) is below
  the height floor of the map file (-100). Coast (bit 3) = not Water and at
  least one Water neighbour (MAP-GRID-7.2). Holds for every field of every
  release map.
- **ECO-PLACE-1.3** ElevationBlocked (bit 6): of any two neighbouring fields that
  are both not Water and differ in height by more than 2400 (class 4), one is
  marked ElevationBlocked, chosen so that the fewest usable fields within
  radius 2 are lost. Never on Water. The exact choice is only needed when
  heights change during play; maps carry the layer (FMT-S2M-4.4).
- **ECO-PLACE-1.4** Border (bit 5) = the field belongs to a player and a
  neighbour is outside the map or belongs to someone else (or no one).
- **ECO-PLACE-1.5** Object bits, set and cleared with the object: DepositBlocked
  (bit 7, tree or stone pile), DoodadBlocked (bit 10, blocking doodad),
  MovementBlocked (bit 8, every field of a building's footprint), Street
  (bit 1, the fields of a road between its flags), LandingPosition (bit 11,
  harbour landing spots).
- **ECO-PLACE-1.6** Recalculation. Terrain-derived bits change only when a
  field's terrain type changes; Water, Coast and ElevationBlocked only when
  heights change (Water and Coast are then redone for the fields within 2 of
  the changed height sample); Border when territory changes. No recalculation
  happens on load; the whole-map recalculation exists only as a debug action.

### ECO-PLACE-2 Flag sites

- **ECO-PLACE-2.1** A field is a **general site** for player P when it is inside
  the map, belongs to P, and has none of Border, Water, ElevationBlocked,
  DoodadBlocked, Blocked, DepositBlocked, MovementBlocked.
- **ECO-PLACE-2.2** A flag may be placed on a general site that has no flag on
  itself or on any of its six neighbours. (Street is allowed: placing a flag on
  a road splits the road, `TRN-roads-flags.md` TRN-ROAD-4.)
- **ECO-PLACE-2.3** ForBuilding is not required for a flag: a flag can stand on
  sand, sea-ground beach or rock if the general-site rule holds.

### ECO-PLACE-3 Building sites

The building's own field is `B`; its flag field is `F` = the direction-1
(south-east) neighbour of `B`. A building of size `s` fits when all of the
following hold, in this order:

- **ECO-PLACE-3.1** `B` belongs to the player.
- **ECO-PLACE-3.2** `F` is inside the map and has ForBuilding (ForMining for a
  mine). If a flag stands on `F` it must be the player's; otherwise a flag
  must be placeable on `F` (ECO-PLACE-2.2).
- **ECO-PLACE-3.3** Neither `B` nor any of its six neighbours is Blocked (fields
  outside the map are ignored). Because a building makes its own field Blocked,
  **no building may stand next to another building**.
- **ECO-PLACE-3.4** Small and mine: slope. Take the mean height of the area of
  radius 2 (small) or 1 (mine) around `B` (MAP-GRID-6.2: heights below -100
  count as -100, integer mean). For each neighbour of `B` that is inside the
  map and not Water, the class of `height - mean` must be between -3 and 3
  (difference at most 2400).
- **ECO-PLACE-3.5** Mine: then `B` must be a general site (ECO-PLACE-2.1), not
  Street, and ForMining. Nothing further.
- **ECO-PLACE-3.6** Footprint. Small and medium: `B` alone. Large and harbour: `B`
  and its neighbours in directions 3 (W), 4 (NW) and 5 (NE). Every footprint
  field must be inside the map, carry no flag, belong to the player, have none
  of Border, Water, ElevationBlocked, DoodadBlocked, Blocked, DepositBlocked,
  MovementBlocked, Street, and have ForBuilding; for a harbour also ForHarbor.
- **ECO-PLACE-3.7** Small: done.
- **ECO-PLACE-3.8** Harbour: among the 18 fields at distance exactly 3 from `B`,
  at least 3 must be inside the map, Water, not DoodadBlocked and not
  LandingPosition.
- **ECO-PLACE-3.9** Large and harbour: every field within radius 2 of `B`
  (including `B`) must be inside the map and must not be the field of another
  building.
- **ECO-PLACE-3.10** Medium, large, harbour: slope. Mean height of radius 2
  around `B` as in ECO-PLACE-3.4; every field at distance 1 or 2 must be
  inside the map and the class of `height - mean` between -3 and 3. For a
  harbour, Water fields are skipped; for medium and large they are not.

### ECO-PLACE-4 Building class of a field

- **ECO-PLACE-4.1** The game keeps, per player and field, the best thing that
  can be built there, trying in this order and taking the first that fits:
  harbour, large, medium, small, mine, then flag (ECO-PLACE-2.2), else nothing.
  A field that fits large also fits medium and small; the class is what the
  build menu offers.
- **ECO-PLACE-4.2** The class of the fields around a change must be recomputed
  when a building, flag or road is placed or removed, or territory changes.

### ECO-PLACE-5 Placing a building

- **ECO-PLACE-5.1** Placing a building sets Blocked on `B`, sets MovementBlocked
  on every footprint field and places the player's flag on `F` if none is
  there. Removing it clears them again (Blocked returns to the terrain's
  value). Animals registered on the footprint fields are removed when the
  building is created (`ECO-food-mining.md` ECO-FOOD-5.16).
- **ECO-PLACE-5.2** Construction-site levelling of the terrain is not covered
  here (open question).
- **ECO-PLACE-5.3 Decorations on a site (Q-293 a, 2026-10-30).** The map's
  decoration objects (the doodad layer: grass, ferns, small stones, `SIM-game-start.md`
  SIM-START-4.5) that stand on the site's footprint fields and on its flag field are
  removed from the world (they leave the doodad lists, so they are not drawn any
  more). A decoration stands on the field whose sample point (fine (4 x + 2 (y odd),
  4 y)) is nearest to its position. When: with the placement for a site that needs
  no levelling; for a site that is levelled, between the placement and the first
  levelling step, in the measured case as its leveller arrives. Measured (T16):
  E-trace-014, the farm site 90538 on (136, 58), placed in counter 3729: the
  decorations at fine (544, 234), (540, 234), (541, 235), (543, 236) (fields (136, 58),
  (135, 58), (135, 59): the building field and two of its footprint fields) are gone
  between the snapshots of 3749 and 3801 (its leveller 90500 arrives at 3789; the
  levelling steps from 4329), the ones at (549, 233), (548, 237), (539, 237), (543,
  226), (541, 227) stay; the small site 90785 on (131, 59), placed by 5901 without
  levelling: (527, 236), (525, 237) (its field) and (529, 232), (528, 232), (529, 233)
  (its flag field (132, 58)) gone at 5901, (525, 232), (523, 235), (530, 230) stay.
  Confidence: medium (two sites; the moment for a levelled site is known only to
  within those snapshots; the decoration's field by nearest sample point and the
  footprint's fourth field are not measured).
  **The start castles do the same (Q-295 answered 2026-11-01, T17).** The
  castles placed finished at the start (`SIM-game-start.md`) remove the
  decorations on their footprint fields and their flag field in the first
  update (counter 0, trace tick 1). **A decoration exactly halfway between two
  sample points stands on the field with the higher x** (fine x / 4 rounded
  half up on even rows). Measured on E-trace-006 (every tick): Storm Coast,
  player 0's castle (29, 51): the decorations at fine (114, 208) and (115, 208),
  field (29, 52) (its north-west footprint field; 114 is halfway between
  fields 28 and 29), gone at counter 0. Player 1's castle (139, 51): (561, 200)
  on its flag field (140, 50) and (561, 208) on its north-east footprint field
  (140, 52), gone at counter 0; (562, 208), halfway between (140, 52) and
  (141, 52) and so on (141, 52), stays until counter 864. Confidence: high for
  the castles; medium for the halfway rule (two cases). One more decoration,
  (136, 207), also goes at counter 0, away from both castles; not explained.

### ECO-PLACE-6 Military clearance (2026-11-16 day, E-eco-047)

- **ECO-PLACE-6.1 What counts as military.** A building type is military here when its
  territory range is above 0: the headquarters (9), the barracks (8), the guard house (9), the
  watchtower (10) and the stronghold (11). The lookout tower, the catapult, the storehouse, the
  harbour and every workshop have no territory range and never count, on either side of the test.
  Confidence: high (code reading and the building properties).
- **ECO-PLACE-6.2 The test.** A military type may be placed on `B` only when no building of a
  military type stands on any field within distance 4 of `B` (MAP-GRID-2.1; `B` itself
  included, 61 fields when none is outside the map). It counts:
  - buildings of **every player**, the player's own and the enemies' alike;
  - **construction sites** as well as finished buildings (a barracks site counts as soon as it
    is placed);
  - each building on **its own field** only (the field it was placed on; a large building's
    other footprint fields do not count).

  Fields outside the map are skipped (the map does not wrap, MAP-GRID-1.4), so a site near the
  edge is not refused because of the edge. Confidence: high (code reading).
- **ECO-PLACE-6.3 Where it applies.**
  - The `place-building` command of a player checks 6.2 **first**, before ECO-PLACE-3: a
    military type that fails it is refused (no site, nothing else happens).
  - The build menu (`UI-ingame.md` UI-GAME-3.2) runs the same test on the menu's field for each
    military cell and disables a failing cell with the tooltip `!Too close to other military
    buildings.` ("Too close to other military buildings"). This test comes before the
    `!Building not allowed` test, so a military type that is both too close and locked shows
    "Too close".
  - The computer player's site search uses the same clause (`AI-construction.md` AI-CON-5.4a).
  - A **military upgrade** (`MIL-building-upgrade.md`) does not run it: the building's own
    field is at distance 0 and would always fail, so the upgrade's own fit test leaves the
    clearance out.
  - The building class of a field (ECO-PLACE-4) does not include it: the menu's tabs and the
    build-help marks are the same near a military building; only the military cells are greyed.
  Confidence: high (code reading).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| military clearance (ECO-PLACE-6.2) | no building or site with a territory range within 4 (distance 0..4, every player) | fields | high | E-eco-047 |
| flag offset from building | direction 1 (south-east) | | high | E-eco-005, E-eco-004 |
| no-building radius (Blocked check) | 1 | fields | high | E-eco-005 |
| large / harbour footprint | B + W + NW + NE | | high | E-eco-005 |
| small slope area | mean radius 2, check ring 1, Water skipped | | high | E-eco-005 |
| mine slope area | mean radius 1, check ring 1, Water skipped | | high | E-eco-005 |
| medium / large slope area | mean radius 2, check radius 2, all in map | | high | E-eco-005 |
| slope limit | class -3 .. 3 (difference to the mean at most 2400) | height units | high | E-eco-005 |
| large / harbour: no other building within | 2 | fields | high | E-eco-005 |
| harbour water fields at distance 3 | at least 3 | fields | high | E-eco-005 |
| class order | harbour, large, medium, small, mine, flag | | high | E-eco-005 |
| Water rule | height < -100 | height units | high (code and all 34 maps) | E-eco-005 |

## Test cases

Setup "T01" = `tutorial01.s2m`; "Fight" = `MP_4P_Fight.s2m`. Unless stated,
player 0 is assumed to own every field (territory is not modelled), nothing
is built, and the class of ECO-PLACE-4.1 is given as 6 harbour, 4 large,
3 medium, 2 small, 5 mine, 1 flag only, 0 nothing. Values computed from the
rules with the investigation's placement script.

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | all 34 release maps | class of every start position | 4 (large) for all 97 start positions |
| T2 | T01, empty | class of (45,38), (44,38), (43,38) | 4, 4, 4 |
| T3 | T01, empty | class of (47,38), (46,40), (49,36), (42,41) | 3, 3, 3, 3 |
| T4 | T01, empty | class of (51,44), (50,44), (40,40), (39,34), (38,32) | 5, 1, 1, 0, 4 |
| T5 | T01, headquarters (large) at the start (45,38) | its flag | at (45,37) |
| T6 | as T5 | class of (45,38), (44,38) | 0, 0 (building field and footprint) |
| T7 | as T5 | class of (43,38) | 3 (large refused: building within 2) |
| T8 | as T5 | class of (47,38), (46,40), (49,36), (51,44), (50,44) | 3, 3, 3, 5, 1 (unchanged) |
| T9 | Fight, empty | class of (40,12); its footprint | 6; (40,12), (39,12), (39,13), (40,13) |
| T10 | Fight, empty | class of (9,9), (68,9), (9,10), (3,11), (10,6), (0,0) | 2, 3, 4, 5, 1, 0 |
| T11 | Fight, empty | (9,9): small, medium, large, mine, harbour | fits, no, no, no, no |
| T12 | Fight, headquarters on all four start positions | class of the 13 x 11 fields from (21,52) to (33,62) | rows below, x = 21..33 |
| T13 | all 34 maps, empty | number of fields per class | tutorial01: 0: 4065, 1: 989, 2: 136, 3: 311, 4: 825, 5: 74. MP_4P_Fight: 0: 4072, 1: 717, 2: 166, 3: 312, 4: 653, 5: 451, 6: 29 |
| T14 | any | a building at B; try any building on a neighbour of B | refused (ECO-PLACE-3.3); a flag or road there is still possible |
| T15 | any | a flag at F; try a flag on any neighbour of F | refused |
| T16 | 5.3: E-trace-014 (R4), player 1's farm site 90538 on (136, 58) (placed 3729, levelled from 4329) and site 90785 on (131, 59) (placed by 5901, no levelling) | the map's decorations around them | gone by 3801: fine (544, 234), (540, 234), (541, 235), (543, 236); by 5901: (527, 236), (525, 237), (529, 232), (528, 232), (529, 233); kept: (549, 233), (548, 237), (539, 237), (525, 232), (523, 235); the look comparison's farm picture of 6000 shows the farm site bare (`look-compare-auto/2026-10-30` weakfogoff-006000-farm) |
| T17 | 5.3 (start castles, Q-295): MP_2P_Storm_Coast, the default start, E-trace-006 (`specs/traces/original-dense-6000-objects.csv.gz`) | the map's decorations on and around the two castles at counters -1 and 0 | gone at counter 0: (114, 208), (115, 208) (field (29, 52), player 0's castle (29, 51)), (561, 200) (flag field (140, 50)), (561, 208) (field (140, 52), player 1's castle (139, 51)); kept: (562, 208) (halfway, so field (141, 52)) until 864. A build that removes none for the start castles keeps four decorations drawn under the castles |

| T18 | ECO-PLACE-6: any map with player 0 owning every field used; player 0's headquarters on (20, 20), nothing else | open the build menu (or send `place-building`) on (24, 20) (distance 4), (25, 20) (distance 5), (20, 24) (distance 4), (20, 25) (distance 5), each a field where the size fits | (24, 20) and (20, 24): the barracks, guard house, watchtower and stronghold cells are greyed with "Too close to other military buildings" and `place-building` of any of them is refused; every workshop, the lookout tower, the storehouse and the catapult stay enabled and are accepted. (25, 20) and (20, 25): all enabled, a barracks is accepted |
| T19 | ECO-PLACE-6: as T18, then player 1's guard house **site** on (40, 20) (on player 1's land, just placed; player 0 owns (35, 20) and (36, 20)), and a lookout tower of player 0 on (60, 20) | the military cells on (36, 20), (35, 20) and (62, 20) | (36, 20) (distance 4 from the enemy's site): greyed, "Too close to other military buildings"; (35, 20): enabled; (62, 20) (distance 2 from the lookout tower, no military building within 4): enabled, a guard house is accepted |
| T20 | ECO-PLACE-6.3: as T18, player 0's finished barracks on (25, 20) | upgrade the barracks | not refused by the clearance (the upgrade does not run it, though the barracks itself stands at distance 0 and the headquarters at 5) |

T12, headquarters at (27,57), (52,56), (52,24), (26,24) of Fight; odd rows are
drawn half a field to the right:

```
y=52   2 4 4 0 3 1 5 5 5 5 5 1 5
y=53    3 4 1 4 4 5 5 5 5 5 5 0 5
y=54   1 3 4 4 4 1 5 5 5 5 5 1 5
y=55    3 4 4 4 4 1 0 0 0 5 5 5 1
y=56   1 3 3 3 3 3 0 0 0 5 0 0 0
y=57    1 0 3 0 3 0 0 0 1 0 1 1 1
y=58   5 1 4 1 4 1 0 0 0 1 1 1 1
y=59    0 3 4 4 4 1 1 1 4 4 4 4 1
y=60   1 3 4 4 4 4 3 3 4 4 4 4 4
y=61    0 3 4 4 4 4 0 3 3 3 4 4 4
y=62   0 3 4 4 4 4 1 3 3 0 3 4 4
```

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-005 | code reading | `S2DNG.exe` | building site check, order of ECO-PLACE-3 |
| E-eco-005 | code reading | | flag field, footprint, mine, flag site, general site |
| E-eco-005 | code reading | | footprint walk |
| E-eco-005 | code reading | | ring walks; harbour water ring |
| E-eco-005 | code reading | | class cache and its order |
| E-eco-005 | code reading | | terrain-derived bits |
| E-eco-005 | code reading | | Water, Coast and when they are redone |
| E-eco-005 | code reading | | ElevationBlocked |
| E-eco-005 | code reading | | Border |
| E-eco-005 | code reading | | object bits; building placement |
| E-eco-005 | code reading | | whole-map recalculation only from a debug action |
| E-eco-047 | code reading and the building properties | `S2DNG.exe`; the scripts' territory ranges | ECO-PLACE-6: the build menu's military cells, the player's place command and the upgrade's exemption share one test of rings 0 to 4 |
| E-map-009 | data survey | the investigation's placement script over the 34 maps | T1, T13; Water = height < -100 on all 565,100 fields; ForBuilding never with ForMining; ForHarbor always with ForBuilding |

## Open questions

- Confirmation in the game of T5 to T8 and T12 (Q-031).
- Territory: which fields a player owns at the start and after military
  buildings (needed before ECO-PLACE-3.1 can be used for real games).
- Levelling of the ground for a construction site (height changes, and hence
  the recalculation of ECO-PLACE-1.6).
- Exact tie rule of ElevationBlocked (ECO-PLACE-1.3).
