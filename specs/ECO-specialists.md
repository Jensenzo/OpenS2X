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
  checks that he stands on the order's flag field; if not, he **goes home at once from where he
  stands** (`TRN-free-walk.md` TRN-FREE-6.4), as in 2.2b: no walk back to the order's flag, no
  spot. (Corrected 2026-11-24 b, Q-272 / stand-ins row 13; E-trn-048, code reading, high: it read
  "he finishes (2.5)", which would walk him to the flag first.)
- **ECO-SPC-2.1a A road cut under him (Q-272; E-trn-048, code reading).** His walk to the flag is
  the road walk of `TRN-free-walk.md` TRN-FREE-6.5: a route chosen again at every flag. When a
  road on his way is removed and no route from the flag he reaches is found, the walk ends there
  (TRN-FREE-6.5 e) and counts as done; the arrival check of 2.1 then finds him off the order's
  flag and he goes home from that flag in the same update (TRN-FREE-6.4: the nearest warehouse he
  can reach, by roads or, failing that, as 6.4 says). He does not wait and does not search again.
  When no road route exists at all when he would start (the flag already cut off), the walk does
  not start and he goes home at once from the warehouse's door. The order he was sent for is not
  filed again by him or by this routine: the flag keeps no specialist until the player orders one
  again (confidence medium: no re-filing found in the routine or the order's own code read;
  ECO-SPC-1.1 files one request per command). Confidence: high for the going home.
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
  diggable. With no candidate he goes home at once from where he stands (2.2b; corrected 2026-11-16 day, was "finishes (2.5)"). Confidence: high (E-pt2-001; replaces the earlier "passed the radius with at
  least one candidate" reading).
- **ECO-SPC-2.2a The "layer-4 object" is another specialist's spot (2026-11-16 day, Q-201 a;
  E-eco-048, code reading, high).** The map keeps one more object layer beside flags, buildings,
  roads and deposits, and only the geologist and the scout use it: when one of them has chosen his
  spot, the spot is **marked with him** before he starts walking; the mark stays while he walks
  there and while he works there, and is taken off when his work at that spot is done (before he
  chooses the next one), or when he leaves the order early (the flag gone, 2.4). So "no layer-4
  object" in 2.2 means **not the current spot of any geologist or scout** (of any player): two
  specialists never pick the same field at the same time. The marks are never in a map file.
- **ECO-SPC-2.2b No spot, or a spot he cannot walk to (Q-201 c; E-eco-048, code reading,
  high).** Choosing the spot and starting the free walk to it are one step. When it fails (no
  candidate, or the walk cannot start), he **goes home at once from where he stands**, as a settler
  sent home (`TRN-free-walk.md` TRN-FREE-6.4), without walking back to the order's flag first. The
  same holds when, on his arrival at the order's flag (before the first spot), the flag is gone.
  Only after his tenth spot does he walk back to the flag (2.5).
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
  The walk home is the common going-home state of TRN-FREE-6.4, with its retries: when no
  warehouse is found he takes one wander step and tries again (its step 4), up to its limit
  (Q-201 b, 2026-11-16 day, E-eco-048: the specialist has no going-home of his own).

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
| T7 | 2.2a: two geologists ordered at two flags whose candidate fields overlap, the second sent while the first walks to his spot S | the second's spot choice | not S (S is marked); once the first has finished his work at S (the sign planted), S is no longer a candidate anyway (the sign) |
| T8 | 2.2b: a geologist ordered at a flag with no diggable field within 5 (all grass) | after he reaches the flag | he goes home at once from the flag field (TRN-FREE-6.4), no spot, no sign; with a candidate whose walk cannot start the same |
| T9 | 2.1a: a geologist ordered at flag F, his road route H -> A -> B -> F; while he walks the road A - B, the road B - F is removed (F stays, now cut off from the network) | when he reaches B | his walk ends on B; in the same update he goes home from B (TRN-FREE-6.4) and walks to the nearest reachable warehouse, where he is stored; he never comes back to F, no spot is dug, no sign appears; the flag F has no geologist on the way afterwards. Before s2-reborn's correction he stood on B and searched again every tick |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-cmd-001 | code reading | S2DNG.exe: the command handlers, the order and its request, the specialist's tick, the spot choice, the work, the geologist's result, the scout's reveal, the 10 spots | |
| E-pt2-001 | code reading | | spot search stop rule, diggable test, sign rule, finishing, explore shape |
| E-eco-048 | code reading | S2DNG.exe: the specialist's states, the spot mark layer and its writers and readers | 2.2a, 2.2b, 2.5 (Q-201) |
| E-trn-048 | code reading | | 2.1, 2.1a: the arrival check after the walk to the flag; off the flag (a walk ended short or not started) -> home at once (Q-272) |

## Open questions

- Q-184 (h): how the specialist's request is prioritised against other requests, and whether an
  order is dropped when the player has no depot with such a settler (ECO-REQ).
