# ECO-SPC: Geologists and scouts sent to a flag

Status: draft (commands, trip, spot choice and results from the code; depot selection as in ECO-REQ)
Owner question(s): Q-184 (h), Q-100 (d); results of the geologist in ECO-production-chains.md
Game build: 11757

## Summary

From a flag's menu the player can call a geologist (the game's "scholar") or a scout (the "explorer").
The order becomes an ordinary request for a settler of that profession at the flag; a depot that holds
one sends it. At the flag the specialist visits ten random spots around the flag, works at each for
8 seconds, reports (geologist: a sign and a message; scout: uncovers the map) and finally walks home.

## Rules

### ECO-SPC-1 Ordering

- **ECO-SPC-1.1** The commands `send-geologist` and `send-scout` carry the flag's
  object id (UI-GAME-2.4). The handler looks the flag up; a flag that no longer exists ends the
  command. Otherwise a specialist order for the profession (geologist `good_scholar`, scout
  `good_explorer`) bound to the flag's field is created and a request for one settler of that
  profession, targeted at the flag, is queued with the player's other requests (ECO-REQ). A depot
  that holds such a settler sends him; nothing else is special about the delivery. Confidence: high.
- **ECO-SPC-1.2** Several orders at the same flag queue up independently (each is its own request).
  Confidence: medium (no counter or lock was found).

### ECO-SPC-2 The trip

- **ECO-SPC-2.1** The specialist walks to the flag (over roads like any delivery). On arrival he
  checks that he stands on the order's flag field; if not, he finishes (2.5).
- **ECO-SPC-2.2** He then visits **10 spots**. Before each spot he picks one: the candidate fields are
  collected by growing a six-neighbour frontier from the flag outwards (the flag field first,
  then each popped field's six neighbours in direction order); a field is entered when it is
  walkable (FMT-S2M-4.4 bits Blocked, Water, ElevationBlocked, MovementBlocked, DoodadBlocked all
  clear) and is a candidate when, for the geologist, it is diggable: ForMining set, Blocked,
  DepositBlocked and DoodadBlocked clear, and no flag, building, sign, deposit or layer-4 object on
  it (roads do not matter); for the scout, it is not yet explored by his player and holds no
  layer-4 object. The search STOPS at the first candidate, or when a field beyond the search
  radius (geologist 5 fields, scout 10) is examined with no candidate yet. So the spot is the
  nearest acceptable field in frontier order (the random draw is over one candidate), and the
  geologist works outward field by field because every dug field gets a sign and is no longer
  diggable. With no candidate he finishes (2.5). Confidence: high (E-pt2-001; replaces the earlier "passed the radius with at
  least one candidate" reading).
- **ECO-SPC-2.3** At the spot he works for **80 ticks** (8 s): the geologist plays the dig clip, or
  the hammering clip when the field holds no resource entry and is not buildable ground; the scout
  plays his clip (GFX-ANIM-3.8). Then the result of 3 or 4 is applied, he steps off the spot and, if
  spots remain, picks the next one (2.2).
- **ECO-SPC-2.4** If the order's flag disappears while he is between spots or working, he finishes
  at once.
- **ECO-SPC-2.5** Finishing: from his last spot he walks over the terrain (free walk) back to
  the order's flag field, and from there home to a depot as a settler returning home (TRN-CARRY)
  and is stored there again. If the order's flag is gone (2.4) he steps off the spot and the
  returning-home walk starts from where he stands. Confidence: high (E-pt2-001).

### ECO-SPC-3 Geologist result

- **ECO-SPC-3.1** The report is the field's resource layer entry: on buildable ground (ForBuilding)
  it is water with amount 1 regardless of the layer (ECO-CHAIN T18); on other fields the entry's
  good and amount (ECO-CHAIN T19); no entry is reported as "nothing" (the empty sign of 3.2 is planted, no message is sent).
- **ECO-SPC-3.2** A sign doodad is placed on the spot according to the sign table (ECO-CAT-5.2:
  empty sign for nothing, water sign, small / medium / large signs at amounts 1 / 3 / 6 for coal,
  iron ore, gold and stone): the rule with the largest threshold not above the amount; a good
  with no rule at or below its amount gets no sign (cannot happen with the shipped table, amounts
  are >= 1). Every sign lasts 10000 ticks (doodads.lua `dd_setLifeTime`). A spot that already
  holds a sign is never chosen (2.2). Confidence: high (E-pt2-001,
  signs.lua, doodads.lua).
- **ECO-SPC-3.3** The owner receives a message "Found Gold!" / "Found Coal!" / "Found Stone!" /
  "Found Iron Ore!" / "Found Water!" at the spot (message kinds: gold and coal one kind, water and
  stone another; UI-GAME-6.4). Confidence: high.

### ECO-SPC-4 Scout result

- **ECO-SPC-4.1** The scout explores the map for his player in a radius of **4 fields** around the
  spot: the RECTANGLE x - 4 .. x + 4, y - 4 .. y + 4 (clamped to the map) becomes explored for
  his player and for every player that is not his enemy (allies share the scouting), and the
  objects on the fields of the HEXAGONAL radius 4 become known to every player who sees objects
  there. Confidence: high (E-pt2-001).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| spots_per_order | 10 | spots | high | E-cmd-001 |
| geologist_search_radius | 5 | fields | high | E-cmd-001 |
| scout_search_radius | 10 | fields | high | E-cmd-001 |
| work_per_spot | 80 | ticks | high | E-cmd-001 |
| scout_reveal_radius | 4 | fields | high | E-cmd-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | own flag F connected to a depot holding a geologist | `send-geologist`(F) | a geologist leaves the depot for F; at F he visits 10 random spots within 5 fields, 80 ticks each; signs appear per ECO-SPC-3; then he returns to a depot |
| T2 | same, no geologist in any depot | `send-geologist`(F) | the request waits (no error) until a geologist exists |
| T3 | flag F at the edge of the explored area, scout available | `send-scout`(F) | 10 spots within 10 fields, each chosen among unexplored fields; after each, the fields within 4 of the spot are explored |
| T4 | geologist walking between spots | the flag is destroyed | he walks home at once |
| T5 | geologist ordered at a flag on mountain ground, all fields diggable | spots | the 10 spots are the 10 nearest diggable fields in frontier order from the flag, each getting a sign (2.2) |
| T6 | geologist digs a field with no resource entry | result | empty sign planted, no message (3.1, 3.2) |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-cmd-001 | code reading | S2DNG.exe: the command handlers, the order and its request, the specialist's tick, the spot choice, the work, the geologist's result, the scout's reveal, the 10 spots | |
| E-pt2-001 | code reading | | spot search stop rule, diggable test, sign rule, finishing, explore shape |

## Open questions

- Q-184 (h): how the specialist's request is prioritised against other requests, and whether an
  order is dropped when the player has no depot with such a settler (ECO-REQ).
