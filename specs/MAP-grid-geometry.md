# MAP-GRID: Field grid geometry

Status: draft (topology, height mapping and scale verified in the binary and against all release maps; open points listed at the end)
Owner question(s): Q-015, Q-018 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

The map is a triangular lattice. A **field** is a lattice point; every field has
up to six neighbours at equal distance, and odd rows sit half a field to the
side of even rows. The map has hard edges: nothing wraps. Terrain height is
stored on a second lattice of the same shape that is four times finer; every
field is one of its points. This spec fixes the neighbour rule, distances,
the mapping between fields, height samples and triangles, the world scale, and
the meaning of the Water and Coast field states.

## Rules

Coordinates: a field is `(x, y)` with `0 <= x < width`, `0 <= y < height`
(`FMT-s2m-map.md` FMT-S2M-1.3). `y` is the row. "Odd row" means `y` is odd.
All divisions are integer divisions that truncate toward zero.

### MAP-GRID-1 Neighbours

- **MAP-GRID-1.1** Fields in one row are one field spacing apart. Each row is
  placed `sqrt(3)/2` field spacings after the previous one, and **odd rows are
  shifted half a field spacing toward +x**. All six neighbours of a field are
  therefore exactly one field spacing away.

  ```
            x=0     x=1     x=2     x=3
  y=0        o-------o-------o-------o
              \     / \     / \     / \
  y=1          o-------o-------o-------o       (odd row: half a field to the right)
              / \     / \     / \     /
  y=2        o-------o-------o-------o
              \     / \     / \     / \
  y=3          o-------o-------o-------o

                 2     1
                  \   /
               3 -- F -- 0          directions as seen with row 0 at the top
                  /   \
                 4     5
  ```

- **MAP-GRID-1.2** The six directions are numbered 0 to 5. The neighbour of
  `(x, y)` in each direction:

  | Direction | Even row (`y` even) | Odd row (`y` odd) |
  |---|---|---|
  | 0 | `(x+1, y)` | `(x+1, y)` |
  | 1 | `(x, y-1)` | `(x+1, y-1)` |
  | 2 | `(x-1, y-1)` | `(x, y-1)` |
  | 3 | `(x-1, y)` | `(x-1, y)` |
  | 4 | `(x-1, y+1)` | `(x, y+1)` |
  | 5 | `(x, y+1)` | `(x+1, y+1)` |

  The numbers go round the field in one turning sense, starting along +x.
  Direction `(d + 3) mod 6` is the opposite of `d`.
- **MAP-GRID-1.6** Compass names. The game's own scripts call direction 3 West,
  4 North-West and 5 North-East, and pass these numbers unchanged to the
  stepping rule above. So: 0 = East, 1 = South-East, 2 = South-West, 3 = West,
  4 = North-West, 5 = North-East; **+y is north and row 0 is the southern edge
  of the map.** A building's flag is on its direction 1 (south-east)
  neighbour. The diagram above is drawn with row 0 at the top, that is with
  south up; flip it vertically for the view with north up.
- **MAP-GRID-1.3** Stepping several times in one direction applies the table once
  per step, with the row parity of the field reached so far.
- **MAP-GRID-1.4** **Edges do not wrap.** A neighbour whose coordinates fall
  outside `0 <= x < width`, `0 <= y < height` does not exist. Code that walks
  neighbours computes the coordinates and skips those outside the map; the walk
  itself continues from the out-of-map coordinates (this matters for the ring
  order, MAP-GRID-3). So a corner field has 2 or 3 neighbours and an edge field
  3, 4 or 5.
- **MAP-GRID-1.5** Two further step codes exist: 6 = `(x, y+2)` and
  7 = `(x, y-2)` (two rows straight down / up, same `x`). They are not
  neighbours (distance 2). Where they are used is not traced.

### MAP-GRID-2 Distance and direction between fields

- **MAP-GRID-2.1** The distance from field A to field B is the number of
  neighbour steps on the shortest way, ignoring obstacles. With
  `dx = B.x - A.x`, `dy = |B.y - A.y|`, `ax = |dx|`:
  1. If `dy` is odd: when B is on an even row and `dx >= 1`, or B is on an odd
     row and `dx <= -1`, reduce `ax` by 1.
  2. If `ax > dy / 2`: distance = `dy - dy / 2 + ax`. Otherwise distance = `dy`.

  The result is symmetric and equals the breadth-first step count.
- **MAP-GRID-2.2** The direction from field A toward a field B that is not a
  neighbour is found by angle. Take the offset `(B.x - A.x, B.y - A.y)`, adding
  0.5 to the x of whichever field is on an odd row; rows count as one unit
  apart here (not `sqrt(3)/2`). Let `deg` be the angle of that offset in
  degrees, measured from the +x axis toward +y, cut to a whole number toward
  zero. `bearing = (90 - deg) mod 360`, in `0..359`. If the row difference is
  zero, bearing is 90 for B to the +x side and 270 otherwise. The direction is
  taken from the 60 degree sector: bearing 0..59 gives direction 5, 60..119
  gives 0, 120..179 gives 1, 180..239 gives 2, 240..299 gives 3, 300..359
  gives 4. For a real neighbour this returns the direction of MAP-GRID-1.2.

### MAP-GRID-3 Ring order

- **MAP-GRID-3.1** Code that scans the surroundings of a field visits the centre
  first, then ring 1 (the six neighbours), then ring 2, and so on up to the
  wanted radius. Ring `k` has `6k` fields; the area up to radius `r` has
  `1 + 3r(r+1)`.
- **MAP-GRID-3.2** Order inside ring `k`: start from the centre moved `k` steps in
  direction 4; that start point is visited **last**. Then for each direction
  0, 1, 2, 3, 4, 5 in turn, take `k` steps and visit the field after each
  step. Ring 1 is therefore visited as the neighbours in directions
  5, 0, 1, 2, 3, 4.
- **MAP-GRID-3.3** Positions outside the map are stepped over without being
  visited (MAP-GRID-1.4).

### MAP-GRID-4 Height samples, fields and triangles

- **MAP-GRID-4.1** The height grid of a map (`FMT-s2m-map.md` FMT-S2M-4.2) has
  `(4 * width + 1)` columns and `(4 * height + 1)` rows of samples. A sample is
  `(c, r)`. The samples form the same kind of lattice as the fields at a
  quarter of the spacing: **odd sample rows are shifted half a sample step
  toward +x**, and the neighbour table of MAP-GRID-1.2 applies to samples
  unchanged (with the height grid's size as the bounds).
- **MAP-GRID-4.2** Field `(x, y)` is the sample `(4x + s, 4y)` with `s = 2` on odd
  field rows and `s = 0` on even ones. The **height of a field** is the value of
  that sample. One field step in direction `d` is four sample steps in
  direction `d`. Measured (E-trace-001): a settler standing on or walking from field
  (x, y) has its fine position at exactly this sample in the original's memory.
- **MAP-GRID-4.3** Sample to field, truncating: `y = r / 4`, then
  `x = (c - s) / 4` with `s` as above for that `y`. This gives the field at or
  before the sample, not the nearest one. Measured (E-trace-001): during a fine walk
  the original's field follows the sample by this rule.
- **MAP-GRID-4.4** Sample to nearest field: among the fields from two columns and
  rows before to two after the truncated result, inside the map, take the one
  whose own sample is the fewest sample steps (MAP-GRID-2.1 on the sample
  lattice) away, at most 4. If none qualifies, use the truncated result clamped
  into the map. Objects stored with a sample position (trees, stones) stand at
  most one sample step from the sample of the field they belong to.
- **MAP-GRID-4.5** **Triangles.** The terrain surface is the lattice's own
  triangulation: every triangle joins three samples that are neighbours of each
  other. Between sample rows `r` and `r + 1`, for each column `c`:
  - `r` even: `(c, r), (c+1, r), (c, r+1)` and `(c+1, r), (c, r+1), (c+1, r+1)`;
  - `r` odd: `(c, r), (c, r+1), (c+1, r+1)` and `(c, r), (c+1, r), (c+1, r+1)`.

  Three fields that are neighbours of each other span one large triangle, made
  of 16 of these small ones. Height inside a small triangle is the plane
  through its three samples.
- **MAP-GRID-4.6** The last sample column and row lie beyond the last field
  column and row and only close the surface.

### MAP-GRID-5 Scale

- **MAP-GRID-5.1** The scale is a setting, read at start from the `Map` entry of
  `data/settings/graphics.xml` (three numbers): world units per sample step
  along a row, world units per raw height unit, world units per sample row.
  Shipped values: 1.5, 0.001, 1.2990375.
- **MAP-GRID-5.2** Position of sample `(c, r)` in the map plane, relative to
  sample `(0, 0)`: `u = 1.5 * (c + h)` with `h = 0.5` on odd sample rows and 0
  otherwise, `v = 1.2990375 * r`. Elevation = `0.001 * raw height`. The game
  adds a fixed world origin to `u` and `v`: the fine grid is centred, origin
  `u0 = -1.5 (W - 1) / 2`, `v0 = -1.2990375 (H - 1) / 2` with `W x H` the fine
  grid size (`GFX-world-view.md` GFX-VIEW-1.1).
- **MAP-GRID-5.3** Hence: field spacing is 6.0 world units, field rows are
  5.19615 world units apart (`6 * sqrt(3) / 2`), and all six neighbours are 6.0
  away. Position of field `(x, y)`: `u = 6 * x + 3 * (y odd)`,
  `v = 5.19615 * y`.
- **MAP-GRID-5.4** Height: 1000 raw units are one world unit, one sixth of a
  field spacing. A height difference of 6000 between two neighbouring fields is
  a 45 degree slope. Water level is raw height 0. (The rendered water surface lies at raw -100,
  world -0.1, the bound of the water rule MAP-GRID-7: `GFX-water.md` GFX-WATER-3.3, corrected
  2026-11-09.)

### MAP-GRID-6 Height classes

- **MAP-GRID-6.1** Where the game compares two heights for steepness it reduces
  the raw difference `d` to a class: more than 2400 gives 4, more than 1200
  gives 3, more than 600 gives 2, more than 200 gives 1; less than -2400 gives
  -4, less than -1200 gives -3, less than -600 gives -2, less than -200 gives
  -1; anything else 0.
- **MAP-GRID-6.2** The mean height of an area (a field and its rings up to a
  radius, MAP-GRID-3) is the sum of the field heights in the map, each raised
  to at least the height grid's floor value, divided by the number of fields
  counted. The floor value is the number stored in front of the height grid in
  the map file (-100 in every release map), so deep water counts as -100.
- **MAP-GRID-6.3** Building-site checks compare surrounding fields with such a
  mean and reject a site when a class is 4 or -4. Which building kinds use
  which radius is not specified here (see open questions).

### MAP-GRID-7 Water and Coast field states

- **MAP-GRID-7.1** In the field state bit set (`FMT-s2m-map.md` FMT-S2M-4.4)
  **bit 2 (0x0004) is Water and bit 3 (0x0008) is Coast.**
- **MAP-GRID-7.2** A Coast field is a field that is not Water and has at least
  one Water neighbour (MAP-GRID-1.2). In the release maps this holds exactly,
  in both directions. No field is both. Every Water field has a negative
  height.
- **MAP-GRID-7.3** Adding a resource to a field (`FMT-s2m-map.md` FMT-S2M-4.5):
  - fish can only be added to a Water field that has at least one Coast
    neighbour; on any other field the request is ignored;
  - no other good can be added to a Water field;
  - the well-water good needs the `ForBuilding` state, every other good needs
    `ForMining`;
  - a field holds one kind of good; adding the same kind raises the amount,
    which is capped at 7.
- **MAP-GRID-7.4** The slope check of a building site (MAP-GRID-6.3) leaves
  Water fields out of some of its ring scans, so sea floor next to a site does
  not count as a steep drop.
- **MAP-GRID-7.5** The generator that creates objects and resources from a
  painted image places nothing on Water fields.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| neighbour_count | 6 | directions | high | E-map-004 |
| odd_row_shift | +0.5 | field spacings along +x | high | E-map-004, E-map-005 |
| map_edges | no wrap | | high | E-map-004, 200 call sites |
| samples_per_field | 4 | per direction | high | E-map-004 |
| odd_field_row_sample_offset | 2 | samples | high | E-map-004, E-map-005 |
| sample_step_x | 1.5 | world units | high (shipped setting) | E-map-006, E-map-004 |
| sample_step_row | 1.2990375 | world units | high (shipped setting) | E-map-006, E-map-004 |
| height_scale | 0.001 | world units per raw unit | high (shipped setting) | E-map-006, E-map-004 |
| field_spacing | 6.0 | world units | high (derived) | MAP-GRID-5.3 |
| field_row_spacing | 5.19615 | world units | high (derived) | MAP-GRID-5.3 |
| height_class_limits | 200, 600, 1200, 2400 | raw height units | high | E-map-004 |
| height_floor | -100 | raw height units | high (value), medium (use) | E-map-004 |
| resource_amount_cap | 7 | units per field | high | E-map-004 |
| water_bit / coast_bit | 2 / 3 | bit index | high | E-map-004, E-map-005 |
| compass names of directions | MAP-GRID-1.6 | | medium-high (script constant names; not checked on screen) | E-eco-004 |
| triangle layout | MAP-GRID-4.5 | | medium (follows from sample positions; renderer index order not traced) | E-map-004, E-map-007 |
| angle rounding in MAP-GRID-2.2 | toward zero | | medium | E-map-004 |

## Test cases

Map M1 is `tutorial01.s2m` (80 x 80) from `FMT-s2m-map.md`. "none" means the
neighbour does not exist.

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | 80 x 80 | neighbours of (10,10), directions 0..5 | (11,10), (10,9), (9,9), (9,10), (9,11), (10,11) |
| T2 | 80 x 80 | neighbours of (10,11) | (11,11), (11,10), (10,10), (9,11), (10,12), (11,12) |
| T3 | 80 x 80 | neighbours of corner (0,0) | (1,0), none, none, none, none, (0,1) |
| T4 | 80 x 80 | neighbours of edge (0,1) | (1,1), (1,0), (0,0), none, (0,2), (1,2) |
| T5 | 80 x 80 | neighbours of corner (79,0) | none, none, none, (78,0), (78,1), (79,1) |
| T6 | 80 x 80 | neighbours of edge (79,78) | none, (79,77), (78,77), (78,78), (78,79), (79,79) |
| T7 | 80 x 80 | neighbours of corner (79,79) | none, none, (79,78), (78,79), none, none |
| T8 | 80 x 80 | neighbours of corner (0,79) | (1,79), (1,78), (0,78), none, none, none |
| T9 | any | step direction d then (d+3) mod 6 from (10,10) and from (10,11), all d | back at the start |
| T10 | any | distance (10,10) to (10,10); to (13,10); to (10,14); to (12,13) | 0; 3; 4; 4 |
| T11 | any | distance (10,11) to (8,14); (0,0) to (79,79); (45,38) to (25,4) | 4; 119; 37 |
| T12 | any | distance by MAP-GRID-2.1 against breadth-first search, all fields within 9 steps of (20,20) and of (20,21), both argument orders | equal everywhere |
| T13 | any | direction from (10,10) to each neighbour by MAP-GRID-2.2 | the direction of T1, for all six |
| T14 | any | ring 1 of (10,10) in visiting order | (10,11), (11,10), (10,9), (9,9), (9,10), (9,11) |
| T15 | any | ring 2 of (10,10) in visiting order | (10,12), (11,12), (11,11), (12,10), (11,9), (11,8), (10,8), (9,8), (8,9), (8,10), (8,11), (9,12) |
| T16 | any | ring 1 of (10,11) in visiting order | (11,12), (11,11), (11,10), (10,10), (9,11), (10,12) |
| T17 | any | sample of field (45,38); (45,39); (79,79) | (180,152); (182,156); (318,316) |
| T18 | any | truncating field of sample (182,156); (181,156); (183,159) | (45,39); (44,39); (45,39) |
| T19 | M1 | height of field (45,38); (45,39); (0,0); (79,79); (10,11) | 1872; 1872; -8598; -7513; -4343 |
| T20 | M1 | plane position and elevation of field (45,39) | u 273.0, v 202.64985, elevation 1.872 |
| T21 | M1 | first deposit: field (38,61), stored sample position (154,244) | equals the field's own sample (MAP-GRID-4.2) |
| T22 | all 34 release maps | every deposit: stored sample position minus its field's sample | one of (0,0), (1,0), (-1,0), (0,-1), (-1,-1), (0,1), (-1,1): the sample itself or one of its six neighbours on an even sample row |
| T23 | height difference | class of 200; 201; 600; 601; 2400; 2401; -200; -201; -2401 | 0; 1; 1; 2; 3; 4; 0; -1; -4 |
| T24 | M1 | count fields with Coast; count fields that are not Water and have a Water neighbour | 493; 493, and they are the same fields |
| T25 | all 34 release maps | same as T24 over all maps | 40287 Coast fields, identical to the 40287 non-Water fields with a Water neighbour; 0 fields with both bits |
| T26 | all 34 release maps | every field holding fish | has Water (41471 of 41471) and at least one Coast neighbour (41471 of 41471) |
| T27 | a Water field with no Coast neighbour | add fish | nothing is added |
| T28 | a Water field | add coal | nothing is added |
| T29 | a field with 6 units of a good | add 3 of the same good | amount is 7 |

T25 and T26 fail if the row shift is applied to even rows instead of odd ones
(36477 of 40287 and 37391 of 41471), so they also guard MAP-GRID-1.2.

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | follow tables: fine positions of standing and walking settlers | MAP-GRID-4.2, 4.3 |
| E-map-004 | code reading | `S2DNG.exe` | the stepping routine: neighbour table, step codes 6 and 7, unused wrap variant |
| E-map-004 | code reading | all 200 call sites of the field and sample step calls | every call disables wrapping; callers bounds-check instead |
| E-map-004 | code reading | | distance between two fields |
| E-map-004 | code reading | a lookup table | bearing and sector to direction |
| E-map-004 | code reading | | ring visiting order |
| E-map-004 | code reading | | field to sample, sample to field (truncating and nearest) |
| E-map-004 | code reading | | scale read from settings; sample and field to world position |
| E-map-004 | code reading | | height classes, mean height with floor, where the floor is loaded |
| E-map-004 | code reading | | resource placement: Water / Coast / ForMining / ForBuilding tests, cap 7 |
| E-map-004 | code reading | | building-site check: ring scans, Water fields skipped, class limit |
| E-map-004 | code reading | | image-driven generator skips Water fields |
| E-map-005 | data survey | the 34 maps | T22, T24 to T26, with the opposite row shift as control |
| E-map-006 | settings file | `data/settings/graphics.xml`, `Map` entry | the three scale numbers |
| E-eco-004 | script + code reading | `buildings.lua` door direction constants | direction names (3 West, 4 North-West, 5 North-East) and their unchanged use as step directions; flag on direction 1 |
| E-map-007 | rendering | mesh images | mesh of M1 fields (40,34) + 10 x 8: levelled sites come out as clean hexagons; with the opposite row shift every sample row shows comb artefacts |

## Open questions

- Orientation is answered by MAP-GRID-1.6 as far as compass names go. Still
  open: that the default camera shows north at the top of the screen, the
  minimap orientation, and the world origin added in MAP-GRID-5.2. (Q-023)
- Building-site rules in full: radius, kinds and the remaining conditions of the
  site check that MAP-GRID-6.3 and 7.4 only touch. (Q-024)
- Use of step codes 6 and 7 (MAP-GRID-1.5).
- Rounding at exact sector boundaries in MAP-GRID-2.2.
- Whether the renderer splits the surface exactly as MAP-GRID-4.5 (index order
  not traced; positions are).
