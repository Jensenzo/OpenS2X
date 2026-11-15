# TER: Territory and borders

Status: draft (read from the binary; values from the scripts; no in-game measurement)
Owner question(s): Q-059 (border markers); plan step 4.1; builds on Q-034 and Q-044 (`ECO-warehouse-construction.md` ECO-HQ-3)
Game build: 11757

## Summary

A military building claims the land around it once it is occupied by its
first soldier. Each field has at most one owner. Where claims meet, the
strength rule of ECO-HQ-3.5 decides. When an occupied military building
disappears, its area is shared out again among the remaining military
buildings. After every change the game recomputes the Border state, removes
roads that run over Border fields or foreign land, and in its regular
updates removes buildings and flags that no longer stand on their owner's
inner land.

## Rules

### TER-1 Ranges

- **TER-1.1** A building type claims land only if its territory range
  (`bp_setTerritoryRange`) is above 0. Range and explore radius
  (`bp_setExploreRadius`) per type:

  | Building | Territory range | Explore radius | Soldiers (`bp_setSoldierLimit`) |
  |---|---|---|---|
  | headquarters (castle) | 9 | 12 | - (keeps soldiers in stock, MIL-SOL-6) |
  | barracks | 8 | 11 | 2 |
  | guard house | 9 | 12 | 3 |
  | tower | 10 | 13 | 6 |
  | fortress | 11 | 14 | 9 |
  | lookout tower (watchTower) | 0 | 20 | - (worker: explorer) |
  | harbour, warehouse, all others | 0 | 0 | - |

  Confidence: high (scripts).
- **TER-1.2** Distances are lattice distances (MAP-GRID-2.1); "within r"
  means distance <= r, the building's own field included. Confidence: high.

### TER-2 When land is claimed

- **TER-2.1 Occupation.** Every building has a one-time "territory applied"
  flag, initially off. In each building update (`ECO-requests-dispatch.md`
  ECO-REQ-0.2 order: after the construction parts) a finished building whose
  flag is off checks whether it is **active**:
  - a type with a soldier limit is active when at least one of its soldiers
    is **present** (inside: linked to the building, not walking, standing on
    the building's door field, not fighting; `MIL-soldiers.md` MIL-SOL-2.4);
    a building whose "move soldiers out" switch is on (MIL-SOL-4.6) and that
    is already applied also counts as active;
  - in addition, a type with a worker (worker limit above 0) needs its worker.
  When active, in that same tick: the player gets the message **"Military
  building occupied!"** (any type with a territory range other than the
  headquarters) or **"Watchtower occupied!"** (the lookout tower); the
  building claims with its range (TER-3) and explores its radius
  (ECO-HQ-3.3); the flag is set. This happens **once**: further soldiers do
  not trigger it again. Confidence: high.
- **TER-2.2 Buildings created already finished** (the headquarters at game
  start, other game-created buildings, ECO-HQ-1.2) claim and explore at
  creation; their flag is set at once. Confidence: high.
- **TER-2.3 Change of type.** When a building's type changes (a finished
  upgrade, `mi_addBuildingUpgrade`: barracks -> guard house -> tower ->
  fortress) and its flag is set, the flag is cleared and the claim and
  exploration run again at once with the new type's range and radius.
  Confidence: high for the mechanism; the upgrade sequence itself is not
  specified here.
- **TER-2.4 Landing.** A harbour site placed by a landing expedition claims
  range 4 and explores radius 6 around its position at once
  (`SHIP-ships-expeditions.md` SHIP-7.2).
  Confidence: high for the numbers.
- **TER-2.5** A construction site does not claim. Confidence: high.

### TER-3 Computing a claim

- **TER-3.1 Claimants.** For a claim by building B at field P with range r:
  B itself comes first, then every other building within distance 3 x r of P
  whose type has range > 0 and whose territory flag is set, in the world's
  building list order (creation order, medium), excluding any building
  standing on P. Confidence: high (order of the others: medium).
- **TER-3.2 Fields.** Every field within r of P is visited (ring order,
  MAP-GRID-3), and gets the owner given by ECO-HQ-3.5 over these claimants:
  the first claimant in TER-3.1 order at distance 0, 1 or 2 takes it outright
  (so B always takes the fields within 2 of itself); otherwise strength
  `range / d^2` (range doubled when d < 5, test `d <= range` with the doubled
  value), the highest per player, greatest player wins, ties to the lower
  player number, no claimant -> no owner. Confidence: high.
- **TER-3.3 Message.** Whenever a visited field had an owner and gets a
  different owner (another player, or none), its previous owner gets the
  message **"Lost Territory."** (an existing message of that text for that
  player is refreshed instead of repeated, as ECO-REQ-3.8). Confidence: high.
- **TER-3.4** Fields outside r are not touched, even if B is stronger there.
  Confidence: high.

### TER-4 Losing a building

- **TER-4.1** When a building whose territory flag is set is deleted (burnt,
  destroyed, removed under TER-6), every field within its range r is
  recomputed with TER-3.2 over the **other** claimants only (those of
  TER-3.1 without the deleted building). No "Lost Territory." messages are
  sent for this. Fields that no claimant reaches lose their owner.
  Confidence: high.
- **TER-4.2** A military building's exploration is not undone. Confidence:
  high.

### TER-5 After a claim or a release

In this order, around the building's field P, with its range r:

- **TER-5.1 Border.** For every field in the square of fields with
  `|dx| <= r + 2` and `|dy| <= r + 2` around P (coordinates, not distance),
  the Border state (bit 5, FMT-S2M-4.4) is set when the field has an owner
  and at least one of its six neighbours is outside the map or has a
  different owner (another player or none); otherwise it is cleared.
  Confidence: high.
- **TER-5.2 Building help.** The per-player "which building fits here" data
  (ECO-PLACE-4) is recomputed for every field within r + 4. Confidence: high
  for the area.
- **TER-5.3 Roads.** For every field within r + 4 that carries a road: if the
  field is a Border field, or its owner is not the road's player, that road
  is removed exactly as when its player removes it (`TRN-roads-flags.md`
  TRN-ROAD-6.1: carrier home, held ware destroyed, end flags stay).
  Confidence: high.
- **TER-5.4 Contact (claims only).** For every field within r + 1 owned by a
  player other than B's, the two players are marked as having met (used by
  the military and AI parts, not specified here). Confidence: high for the
  marking; its use is not traced.

### TER-6 Buildings and flags on lost land

- **TER-6.1 Inner land.** A field is inner land of player X when it is
  inside the map, is owned by X, is **not** a Border field, is not Water, not
  ElevationBlocked and not DoodadBlocked. Confidence: high.
- **TER-6.2 Buildings.** In every tick, right after a building's own update
  (ECO-REQ-0.2), the building is removed if its field is not inner land of
  its player. It is replaced by its remains (`bp_setRemains`) as on
  destruction, and TER-4 runs if its flag was set. So a building whose field
  is lost, or whose field becomes a Border field, disappears in the same
  tick if it comes later in the building list than the building that caused
  the change, else in the next tick. Confidence: high for the test and the
  moment; medium for the remains (the removal path creates them for types
  that have one, except the headquarters).
- **TER-6.3 Flags.** In every tick, in the roads update, for each player
  0..7 and each of his flags: a flag whose field is not his inner land is
  removed as by the player (`TRN-roads-flags.md` TRN-ROAD-6.2): its roads,
  then the building behind it with remains, then the flag with the wares on
  it (destroyed). So a building whose own field is still inner land goes
  with its flag when only the flag field is lost. Confidence: high.
- **TER-6.4** So buildings, flags and roads are never kept on Border fields:
  the Border ring is where the land ends for building purposes.
  Confidence: high.

### TER-7 Border markers (presentation)

- **TER-7.1** Border fields are shown with marker objects in the owner's
  player colour, a stone (`data\map\objects\limes.GR2`) on land and a buoy
  (`data\map\objects\buoy.GR2`) on water, drawn under the fog of war like
  other objects. The player can hide them ("Show/Hide Borders").
  Confidence: medium for the fog of war (shader).
- **TER-7.2 Placement (Q-059).** For each of the 8 player slots, every field
  owned by that player whose Border state is set (TER-5.1) gets exactly one
  marker, standing at the field's own position (its terrain point, not
  between fields). A field with the Water state gets a buoy, any other a
  stone. Markers have no per-marker rotation or spacing rule: neighbouring
  Border fields simply each carry one. Markers of all players are built, not
  only the viewer's; the view only culls them by visibility. Confidence: high.
- **TER-7.3** A buoy bobs: its height is the water surface plus
  `0.05 x sin(0.3 x c x t + x + z)` (world x and z of the field, `t` the render
  clock, `c` a renderer constant; same clock question as Q-055). Stones stand
  still at the terrain height. Confidence: medium.
- **TER-7.4** The lists of all players are rebuilt from scratch when the map
  view is told that territory changed; the exact trigger is not traced.
  Markers are therefore always the current Border fields. Confidence: medium.

### TER-8 When territory changes (summary)

- **TER-8.1** Ownership changes only at: a building's occupation or creation
  (TER-2.1, 2.2), a type change (2.3), a landing (2.4), and the deletion of
  an applied building (TER-4); conquest is plan 4.4. There is no periodic
  recomputation. Border state, building help and roads are recomputed right
  after each of these (TER-5); buildings and flags are checked every tick
  (TER-6). Confidence: high.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| territory ranges | castle 9, barracks 8, guard house 9, tower 10, fortress 11 | fields | high | `buildings.lua` |
| claimant search radius | 3 x range | fields | high | E-ter-001 |
| outright-win distance | 0..2 | fields | high | E-ter-001 |
| doubled range below distance | 5 | fields | high | E-ter-001 |
| border recompute square | range + 2 | fields | high | E-ter-001 |
| road / building-help recompute radius | range + 4 | fields | high | E-ter-001 |
| contact radius | range + 1 | fields | high | E-ter-001 |
| landing claim / explore | 4 / 6 | fields | high | E-ter-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | own barracks finished in tick F, first soldier present in tick S > F | territory | no change before S; in tick S "Military building occupied!", fields within 8 claimed, fields within 11 explored |
| T2 | as T1, second soldier arrives later | territory | no new claim, no message |
| T3 | player 0 guard house G0 (range 9) occupied; player 1 barracks B1 (range 8) occupied at distance 12 from G0 | field F at distance 6 from both | strengths 9/36 = 0.25 vs 8/36 = 0.222: F stays player 0's (B1's claim visits it) and player 0 gets no message |
| T4 | as T3, field at distance 2 from B1 and 10 from G0 | B1's claim | the field goes to player 1 (outright); player 0 gets "Lost Territory." if it was player 0's |
| T5 | only one military building, range 8, occupied; it burns down | release | every field within 8 loses its owner; no messages; Border recomputed in the 21 x 21 square |
| T6 | two own barracks A and B at distance 6, both occupied; A is destroyed | release | fields within 8 of A that are within reach of B (strength rule) stay owned, the rest lose their owner |
| T7 | own woodcutter whose field is at distance 3 from the new border after a release | next building update | the woodcutter is removed when its field is no longer owned or is a Border field |
| T8 | own flag on a field that becomes a Border field | roads update that tick | the flag and its roads are removed |
| T9 | own road crossing a field that becomes foreign land | claim by the enemy | the road is removed during the claim |
| T10 | field A owned by player 0 whose neighbour is unowned | Border | A has the Border bit; an owned field whose six neighbours are all owned by player 0 and inside the map has not |
| T11 | owned field on the map edge (x = 0) | Border | Border bit set (a neighbour is outside the map) |
| T12 | occupied barracks upgraded to a guard house | upgrade finishes | claim runs again with range 9 at once |
| T13 | player 1 owns a row of 5 Border fields, the middle one Water | markers | 4 stones and 1 buoy, one on each field's own point |
| T14 | player 2's border is in an area player 1 has not explored | markers | still built for player 2 (drawing under fog of war only) |
| T15 | a woodcutter hut whose field stays inner land but whose flag field becomes Border after an enemy claim; 2 wares on the flag | roads update of that tick | flag removed with its roads, the hut removed with remains, the 2 wares and the remains destroyed (TRN-ROAD-6.2) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ter-001 | code reading | `S2DNG.exe` | occupation check, active test, claim + explore + flag (in the building update; the "territory applied" flag) |
| E-ter-001 | code reading | | claims at creation, on type change, at a landing |
| E-trn-006 | code reading | | what removing a road or flag does (TER-5.3, 6.3) |
| E-ter-001 | code reading | | claim: claimant list, ring walk, owner rule, "Lost Territory." |
| E-ter-001 | code reading | | release when an applied building is deleted |
| E-ter-001 | code reading | | Border recompute and rule |
| E-ter-001 | code reading | | building-help recompute, road removal test |
| E-ter-001 | code reading | | per-tick building removal on lost land, remains |
| E-ter-001 | code reading | | per-tick flag removal |
| E-ter-001 | code reading | strings limes.GR2, buoy.GR2, "Show/Hide Borders." | border marker meshes |
| E-ter-002 | code reading | | per-player marker lists, placement, buoy bob, drawing, rebuild |
| data | script | `buildings.lua`, `military.lua` | ranges, radii, soldier limits, upgrades |

## Open questions

- What exactly triggers the marker rebuild (TER-7.4).
- Order of the other claimants (building list order assumed).
- What the "players have met" marks are used for.
