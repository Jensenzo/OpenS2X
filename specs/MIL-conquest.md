# MIL-CON: Conquest and the catapult

Status: draft (read from the binary; no in-game measurement)
Owner question(s): plan step 4.5 (chunk 2); Q-062; follows `MIL-attack-combat.md` (MIL-ATK), `TER-territory.md` (TER), `TRN-free-walk.md` (TRN-FREE-6.4)
Game build: 11757

## Summary

An attacked military building falls when none of its soldiers is left to
defend it. The first attacker to walk in takes it: an ordinary military
building changes owner (it is removed and an identical, finished building of
the attacker appears in its place, which claims land at once); a headquarters
is destroyed instead. Other attackers fill the taken building up to its wanted
number of soldiers and the rest walk home. A destroyed warehouse sends the
settlers in its stock out to find another warehouse. The catapult throws
stones at the nearest enemy military building within 12 fields; a hit kills
one soldier and a building with no soldier left burns down.

## Rules

### MIL-CON-1 When a building can be taken

- **MIL-CON-1.1 Taking condition.** An attacker can take its target when the
  target's owner is no longer its enemy (then it just enters, 3.1), or when
  both hold:
  1. no soldier linked to the target is in any state other than "on its way
     in" (just made, leaving its warehouse, or walking to the building after
     being sent, MIL-SOL-4.7). Soldiers of the target out fighting, waiting
     outside or intercepting **do** count, so every one of them must be dead
     or gone;
  2. no opponent is available (MIL-ATK-5.1).
  Confidence: high.
- **MIL-CON-1.2** The check is made by a waiting attacker every tick
  (MIL-ATK-4.4), and again when it arrives at the target's door; the first
  attacker (in settler update order) for whom it holds walks to the door
  (free walk) and, on arrival, if it still holds, takes the building.
  Several attackers (Q-062): there is no central choice. Each waiting
  attacker tests the condition in its own settler update, so in one tick they
  test in settler update order, and **every** one for whom it holds starts
  walking to the door in that tick. The first to arrive with the condition
  still holding takes the building; the others arrive at a building that now
  belongs to their own player (MIL-CON-2.2 step 6) and enter it if there is
  room (MIL-CON-3). Confidence: medium-high.

### MIL-CON-2 Taking a building

- **MIL-CON-2.1 Headquarters.** If the target is a headquarters, it is
  destroyed (MIL-CON-4 with burning remains, as for any burnt building) and
  the **attacking** player gets "Attacked Castle destroyed." at its field.
  Nothing is taken over. Confidence: high.
- **MIL-CON-2.2 Other military buildings**, in this order:
  1. soldiers out intercepting for the target (MIL-ATK-3) lose that errand
     (they walk home, MIL-ATK-3.4);
  2. the old owner gets "Lost Military Building to enemy." at its field;
  3. the building is removed **without** remains (no ruin, no fire); its land
     is released as in TER-4.1 (recomputed from the other buildings, no
     messages); people linked to it go home (MIL-CON-4.2); its stock and
     coins are lost;
  3a. **its flag is removed** as by the player (`TRN-roads-flags.md`
     TRN-ROAD-6.2): every road at the flag goes (carriers home, held wares
     destroyed) and the wares lying on the flag are destroyed. The new
     building gets a fresh flag of its own at the same place when it is
     created (step 4), with no roads;
  4. a new building of the **same type** is created for the attacker at the
     same field, finished, with the old building's tribe look; it claims its
     range and explores at once (TER-2.2), which may send "Lost Territory."
     messages (TER-3.3); its coin switch and other settings start at their
     creation defaults (MIL-PROM-3.2, MIL-SOL-4.6);
  5. the attacker gets "Military Building conquered." at the new building's
     field;
  6. all other attackers of the old building are re-linked to the new one as
     their target (MIL-CON-3.2).
  Confidence: high for the order and messages; medium for "settings at
  defaults" (follows from a fresh building).
- **MIL-CON-2.3 The taking soldier** then enters the new building if there is
  room (MIL-CON-3.1); its attack errand ends either way. Confidence: high.
  **How he gets in (2026-11-15, asked by the implementation; E-mil-013, code reading):** not at once.
  Taken in, his home is the new building and he walks in the way every settler enters a building:
  a walk to the building's flag (he stands on it already, so it is over at once), then the walk from
  the flag into the building, onto its field and the door step (`ECO-worker-cycle.md` ECO-WORK-3.8,
  3.8b), with the usual walking time; he is inside when that walk ends. If an opponent has been
  paired with him by the time the walk to the flag ends, he does not go in but stays out as the
  building's defender (MIL-ATK-5). Confidence: high for the two walks and the defender case (the
  soldier's take-over job read); medium for the walk to the flag taking no tick of its own.

### MIL-CON-3 Filling the taken building

- **MIL-CON-3.1 Room.** An own attacker reaching an own (taken) building joins
  it when `wanted - S - R - 1 >= 0` after open soldier requests have been
  cancelled one at a time while that value is below 0 (MIL-SOL-3.2 `wanted`,
  4.2 `S`, `R`). Joining: it leaves its old home building's list and becomes a
  soldier of the new building. Without room it walks back to its old home.
  Confidence: high.
- **MIL-CON-3.2** Since the new building's enemy distance is -1 until its
  first distance update, `wanted` is its full soldier limit then (MIL-SOL-3.2),
  so in practice the first attackers up to the limit move in. Confidence:
  high (follows from MIL-SOL).

### MIL-CON-4 Removing a building and its people

- **MIL-CON-4.1 Removal.** A building removed by conquest (no remains), by a
  catapult or a headquarters capture (with remains), on lost land (TER-6.2,
  with remains), or torn down by a computer player (`AI-construction.md`
  AI-CON-9.3, with remains; measured T17, E-ai-021), or demolished by the
  human player (`remove-building`, with remains: 4.1b) loses its land if it had claimed (TER-4.1), its open requests
  are cancelled, and it leaves every world list. With remains, a ruin is left
  that burns (`rm_setBurningTime`, `ECO-time-units.md`); the headquarters
  never leaves a ruin. Confidence: high for the steps; the remains contents
  are not specified here.
- **MIL-CON-4.1a The burning ruin blocks its field.** While a ruin burns, its field
  is marked like a building's field (Blocked, `ECO-building-placement.md` ECO-PLACE-5.1): no
  building, site or flag can be placed on it and the placement classes of the fields
  around it follow as for a building there; when the ruin has burnt out the mark is
  cleared. A **site** torn down with remains leaves no ruin (it takes no number for
  one) but gives back its material by MIL-CON-4.4 (the work done so far and the unused
  material). Confidence: high for a small building (measured, E-trace-011: the ruin
  90781 of the woodcutter's hut torn down at counter 5816 burns on (136, 47) until
  6149; the computer player's site search for a spring in cell (8, 2), taken at 6094,
  passes over (136, 47) and ends one step earlier than with the field free, the site
  (134, 45) placed at 6124 as recorded (an experiment build without the mark places
  it at 6125); the hunter's site 90716 torn down at 6226 in its main stage gives one
  plank (90818) and no ruin number); medium for the footprint of a larger building
  (code E-ai-022: the ruin marks the field and the footprint; not
  recorded).
  **What is left after the fire (2026-11-09 c, Q-150; `remains.lua`):** the three remains
  kinds (small, medium, large: `RuinSmall01`, `RuinMedium01`, `RuinLarge01`) all burn **333
  ticks** (the script's earlier 600 is commented out), and the "doodad after finished" line
  (`rm_setDoodadAfterFinished`, a debris doodad) is commented out for all three, so a ruin that
  has burnt out leaves **nothing**: the ruin disappears and its field is free again. Which
  removals leave a ruin is 4.1's list (the human player's demolish included, 4.1b). Confidence:
  high for the time and the empty end (script).
- **MIL-CON-4.1b The human player's demolish leaves remains (2026-11-15 day, Q-150; E-mil-014,
  code reading).** Every demolish the player gives sends the one `remove-building` command: the
  building window's and the site window's destroy button (UI-GAME-4.1, 4.2) and the destroy
  action with a building selected. That command removes the building **with remains**, exactly as
  the computer player's tear-down (AI-CON-9.3):
  - **a finished building** gives back MIL-CON-4.4's material on its flag (the frame and main
    material, each good halved and rounded up, at most what the flag has room for) and leaves a
    burning ruin for 333 ticks that blocks its field (4.1a);
  - **a site** gives back the done share halved plus its unused delivered material in full, and
    leaves no ruin (4.1a);
  - **a warehouse or harbour** sends its settlers out, except helpers, and its wares and helpers
    are lost (4.3);
  - the people linked to it go home (4.2).

  The headquarters never leaves remains (4.4), and its window has no destroy button.
  **The building's flag stays.** The command removes only the building. Its flag and the flag's
  roads remain, so the given-back wares lie on the flag and are carried off by the normal
  transport rules (remove-flag and remove-road are separate commands, UI-GAME-3.1, 3.3). The world
  drops the remains only while a game is being shut down, never in play. Confidence: high (code:
  the command's handler, the removal's remains switch, the senders). Test T17a.
- **MIL-CON-4.2 People.** Settlers linked to the removed building (worker,
  soldiers inside or on their way, settlers sent to it) lose the link; each
  then goes home by TRN-FREE-6.4 (nearest accepting warehouse by road, else
  any warehouse of the same landmass (TRN-FREE-6.4 step 3: equal stored
  continent indices) by free walk to its network, up to 51
  tries). Confidence: medium (seen for soldiers; assumed for every settler
  kind).
- **MIL-CON-4.3 Warehouse stock.** When a finished warehouse-type building
  (headquarters, warehouse, harbour) is removed **with** remains, every
  settler counted in its stock, except helpers, is made into a walking settler
  of its profession at the door, one per count, and goes home (4.2); helpers
  and all wares in the stock are lost. Confidence: high for the rule; medium
  for the walk-out details (door cooldown not traced).
- **MIL-CON-4.4 Remains: material put on the flag (Q-062).** A building
  removed **with** remains (any type except the headquarters) gives back
  part of its building material as wares put on **its flag**:
  1. frame material: `round(f x F)` of the building type's (first) frame
     material, `F` its frame count and `f` the share of frame work done
     (1 minus the frame part still to do, ECO-HQ-4.7);
  2. main material: `M = round(m x T)`, `T` the sum of all main material
     counts and `m` the share of main work done; `M` is handed out over the
     main material list in list order, each good up to its own count;
  3. each good's amount from 1 and 2 is then **halved, rounded up**
     (`(n + 1) div 2`);
  4. a building still under construction adds, in full, the materials
     delivered to its site and not yet used;
  5. the wares are put on the flag one good after another in ascending good
     id order, one ware of each good with something left per round, rounds
     repeated until all are handed out. A ware is only put down while the
     flag holds fewer than the flag package limit (6, TRN-ROAD-5.2); a ware
     that does not fit is **lost**.
  `round(x)` is `trunc(x + 0.5)`. These wares are made first (each takes its
  object number, `SIM-game-start.md` SIM-START-4.2), before the building
  leaves the lists; the ruin is made after the removal and takes the next
  number (measured T17). The wares lie at the flag like any other
  and are carried off by the normal transport rules. The burning ruin itself
  (`rm_setBurningTime`) holds nothing. Where the flag is removed in the same
  step (TRN-ROAD-6.2), these wares are destroyed with it. Confidence: high
  for the amounts; medium for the good order (key order of a sorted map).

### MIL-CON-5 The catapult

- **MIL-CON-5.1 Building.** Type `catapult` (`buildings.lua`): medium size,
  worker a helper, working radius 12, stock limit 4 (stones). The helper
  operates it; it never moves. Confidence: high.
- **MIL-CON-5.2 Cycle.** The operator idles until the catapult holds at least
  one stone and production is switched on. Then a search step runs: a counter
  rises by 1 in every update of this step; on the update where it exceeds
  300 (the 301st) it is set to 0 and a target is searched (5.3). No target:
  the step goes on (next search 301 updates later). Target found: the shot
  (5.4) runs, then the operator idles again (and the next search comes 301
  updates after the search step restarts). Confidence: high.
- **MIL-CON-5.3 Target.** Among all military buildings of players that are
  enemies of the catapult's owner (world list order): not a headquarters;
  occupied (territory applied); not having interceptors out for it while
  fewer than 2 of its soldiers are present; with `dist(catapult, building) <=
  12`. The nearest wins; on equal distance the first in the list. The aim
  field is that building's field. Confidence: high.
- **MIL-CON-5.4 The shot.**
  1. Aim: the landing field is the aim field. One number is drawn:
     `draw mod 19` (`19 = 12/2 + 1 + 12`); if it is below `6 + d` (`d` =
     distance to the aim field) the shot misses: a second draw `mod 6` gives a
     direction and the landing field moves one step that way (MAP-GRID).
  2. Turn: the catapult has a facing direction (starts at 1). The new
     direction `n` is the six-way direction from the catapult to the landing
     field (MAP-GRID-2.2). The number of turn steps is `(n - current) mod 6`,
     except that when `n > 3` it is `6 - n` (as the original computes it). The
     turn takes `(steps + 1) x 11` ticks (`cfg_setCatapultRotationTimeFactor`);
     afterwards the facing is `n`.
  3. Throw: one stone is taken from the stock (and a new one ordered); the
     flight takes `dist(catapult, landing field) x 11` ticks
     (`cfg_setCatapultFlightTimeFactor`).
  4. Impact: if the landing field equals the aim field and a building stands
     there, that building is hit (5.5). A miss does nothing.
  5. Turn back: as step 2 toward direction 1.
  Confidence: high for the formulas; medium for the exact tick counting of
  the step durations (an animation of length L ends after L updates).
- **MIL-CON-5.5 Hit.** If the building has present soldiers, the one with the
  lowest rank (first in list order on equal rank) dies: its owner gets
  "Soldier killed by Catapult" (at the catapult's field), and the catapult
  owner's kill statistic rises by 1. Then, if no present soldier is left, the
  building is removed with remains (MIL-CON-4). So a hit on a building with
  0 or 1 present soldier destroys it. Confidence: high.
- **MIL-CON-5.6 Hit chance.** `P(hit) = (13 - d) / 19` for `d` 0..12: 12/19
  at `d = 1`, 1/19 at `d = 12`. Confidence: high (follows from 5.4).

### MIL-CON-6 Messages (summary)

| Text | To | Where | When |
|---|---|---|---|
| "You are under attack!" | target's owner | target | a soldier of a duel arrives (MIL-ATK-6.1) |
| "Lost Military Building to enemy." | old owner | building | MIL-CON-2.2 step 2 |
| "Military Building conquered." | conqueror | new building | MIL-CON-2.2 step 5 |
| "Attacked Castle destroyed." | attacker | headquarters | MIL-CON-2.1 |
| "Soldier killed by Catapult" | soldier's owner | catapult | MIL-CON-5.5 |
| "Lost Territory." | previous field owner | field | land claim (TER-3.3) |

Confidence: high for texts and recipients.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| catapult radius | 12 | fields | high | `buildings.lua`, E-mil-004 |
| catapult search period | 301 | updates | high | E-mil-004 |
| miss test | draw mod 19 < 6 + d | | high | E-mil-004 |
| rotation time | (steps + 1) x 11 | ticks | high | `config.lua`, E-mil-004 |
| flight time | distance x 11 | ticks | high | `config.lua`, E-mil-004 |
| catapult stock limit | 4 | stones | high | `buildings.lua` |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | enemy barracks with 2 soldiers: 1 inside, 1 out as an interceptor sent by the barracks itself; attackers waiting | | not takeable while either lives: the inside one is an available opponent, the interceptor still counts (MIL-CON-1.1) |
| T2 | last defender dies; a 3rd soldier of the barracks is still walking in from a warehouse | next waiting-attacker check | takeable (the walking soldier does not count) |
| T3 | attacker takes enemy tower T (limit 6) at field F | conquest | old owner "Lost Military Building to enemy."; T removed without ruin; new tower at F for the attacker, claims range at once; "Military Building conquered." |
| T4 | as T3 with 8 attackers waiting | filling | the taker and the next 5 that check enter (wanted 6), the other 2 walk home |
| T5 | attacker takes an enemy headquarters | capture | HQ removed with remains; attacker gets "Attacked Castle destroyed."; the HQ's stock: 4 woodcutters, 2 privates, 30 helpers, 50 planks -> 6 settlers walk out, helpers and planks lost |
| T6 | catapult, enemy guard houses at distance 9 and 11 (both occupied), one stone | search | targets the one at 9; miss test draw mod 19 < 15; hit chance 4/19 |
| T7 | catapult facing 1, landing direction 5 | turn | steps = 6 - 5 = 1, time 22 ticks; facing 5; turn back: (1 - 5) mod 6 = 2 steps, 33 ticks |
| T8 | catapult facing 1, landing direction 3 | turn | steps (3 - 1) mod 6 = 2, 33 ticks |
| T9 | hit on a building with present soldiers ranks 2, 0, 0 | impact | the first rank-0 soldier dies; 2 present remain; building stays |
| T10 | hit on a building with 1 present soldier | impact | soldier dies, building removed with remains, land released |
| T11 | landing field at distance 10 from the catapult | flight | 110 ticks |
| T12 | finished guard house (`buildings.lua`: frame 2 planks, main 3 stones) destroyed by a catapult, empty flag | remains | frame round(1 x 2) = 2 planks, main M = 3 stones; halved up: 1 plank, 2 stones; put on the flag as plank/stone by ascending good id, one of each per round |
| T13 | guard house site, frame done (f = 1), main one third done (m = 1/3, T = 3), 1 stone delivered and unused | removed with remains | planks 2 -> 1; stones round(1.0) = 1 -> 1; plus 1 unused stone: 1 plank, 2 stones |
| T14 | remains of 5 wares, flag already holds 4 | put on flag | 2 are put down (flag at 6), 3 lost |
| T15 | 3 attackers waiting when the last defender dies | that tick | all 3 walk to the door; the first to arrive takes it; the other 2 enter the new building if it has room |
| T16 | enemy tower with roads to 2 flags, 3 wares on its flag | conquered | old flag and both roads removed, the 3 wares destroyed; the new tower has a flag with no roads |
| T17 | 4.1, 4.4, `AI-construction.md` AI-CON-9.3: E-trace-010, the computer player tears down its finished hunter's hut 90244 on (139, 47) (flag (140, 46)) at counter 4791 | wares and numbers | one plank 90644 lies at (140, 46) (the hut's frame and main planks, halved, rounded up), then the remains 90645 on (139, 47) (measured, `objects.csv`); the hut's hunter, resting inside, goes home (`TRN-free-walk.md` T28) |
| T17a | 4.1b (Q-150): the human player demolishes their finished woodcutter's hut (frame cost 1 plank, main cost 1 plank) through the building window; separately, a woodcutter's site whose frame is done (frame share 1, main share 0) with its main plank delivered and not yet used is destroyed through the site window | wares, ruin, flag | the hut: 1 plank on its flag (frame 1 plus main 1 = 2, halved rounded up), then a ruin on the hut's field burning 333 ticks; the site: 1 plank for the frame (1 halved rounded up) plus the unused plank in full = 2 planks on its flag, no ruin; in both cases the flag and its roads stay, and carriers take the planks off |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-mil-004 | code reading | `S2DNG.exe` | taking condition |
| E-mil-004 | code reading | | capture, HQ destruction, conquest, filling |
| E-mil-004 | code reading | | removal, people going home, warehouse stock walking out |
| E-mil-004 | code reading | | new building claims at creation |
| E-mil-004 | code reading | | catapult operator, search step, shot sequence |
| E-mil-004 | code reading | | catapult target |
| E-mil-004 | code reading | | aim and miss, turn, flight, stone use, impact, kill |
| E-mil-005 | code reading | | material put on the flag |
| E-mil-005 | code reading | | conquest removes the old flag |
| E-mil-005 | code reading | | each attacker tests the taking condition itself |
| E-mil-013 | code reading | (session 2026-11-15 night) | 2.3: the taking soldier's job after the takeover: walk to the flag, walk in (ECO-WORK-3.8), or stay out as the defender when paired |
| E-mil-014 | code reading | (session 2026-11-15 day) | 4.1b: the remove-building command's handler removes with remains; the senders (building and site windows, the destroy action); the remains switch cleared only at shutdown; the flag not removed |
| data | script | `buildings.lua` (catapult), `config.lua` | radius, stock, time factors |

## Open questions

- Victory messages ("You have destroyed all enemies!", "You have conquered
  the map!") belong to mission scripts and are not covered.
- The building-window command of Q-062 is not military: it is the harbour's
  `prepare-expedition` (`SHIP-ships-expeditions.md`).
