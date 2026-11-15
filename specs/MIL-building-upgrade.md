# MIL-UPG: Military building upgrades

Status: draft (conditions, triggers, costs, process and completion read from the binary; what is drawn during an upgrade, cancellation and attacks during an upgrade not traced)
Owner question(s): Q-165 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

A finished barracks can be upgraded to a guard house, a guard house to a
tower and a tower to a fortress. The upgrade is ordered by the player on the
building (or by the computer player) and then runs like the construction of
the new type on the spot: the ground is levelled where the bigger footprint
needs it, the new type's building materials are delivered and built in by a
builder, and when the construction is complete the building changes type.
Meanwhile it stays what it was: it keeps its soldiers and its territory.

## Rules

### MIL-UPG-1 Chain and conditions

- **MIL-UPG-1.1** The upgrade chain is defined by the shipped `military.lua`
  (`mi_addBuildingUpgrade`): barracks -> guard house -> tower -> fortress.
  A fortress has no upgrade. Confidence: high.
- **MIL-UPG-1.2** A building **can be upgraded** when all of these hold:
  it is finished (its construction has no remaining progress), no upgrade is
  running on it, its type has a next type in the chain, and either the next
  type has the same size class as the current one or the next type's
  footprint fits at the building's position with the building itself
  ignored. One further condition: **no builder is still linked to the
  building's construction** (identified 2026-10-27 night, E-ai-029:
  the construction's builder reference must be none; earlier believed "not
  under attack"). The builder that finished the building stays linked until
  he is released at the end of his step's action (`ECO-warehouse-construction.md`
  ECO-HQ-4.7); his link is dropped in that tick, in the settlers' part, after the
  buildings' update (Q-367, answered 2026-10-27 night: guard house 90785 of
  E-trace-011 finished at counter 9938, its builder 90795 worked on to 9941 and
  was released, going home, at 9942; the heap dump of 10000 holds no builder; the
  same moment gates the type change of MIL-UPG-2.4, measured there, T8). So an
  upgrade ordered in the ticks between the completion and the release is
  refused. Confidence: high.
- **MIL-UPG-1.3** The player orders an upgrade with a command on the
  building (`upgrade-building`; the building window's upgrade button; the tooltips "Upgrade
  Running", "Upgrade no space", "Upgrade not possible" correspond to
  MIL-UPG-1.2). The command is refused when MIL-UPG-1.2 fails. **No stock is
  checked and nothing is paid at the order**: the materials are requested
  and delivered as for a site. Confidence: high.
- **MIL-UPG-1.4** The computer player orders upgrades as in `AI-military.md`
  AI-MIL-4.4 and 4.5; before ordering it checks that **twice** the target
  type's full construction costs (frame goods and building goods, the list
  merged with itself) are in stock (corrected 2026-10-27 night, E-ai-029, `AI-military.md` T19: a guard house needs 6 planks and 10 stones
  for its tower). Confidence: high.

### MIL-UPG-2 Process

- **MIL-UPG-2.1** At the start the building is marked *upgrading*, its
  blocked area is re-laid for the target type's size class, and the area
  around it (radius 4) is re-marked (confidence medium for the re-marking's
  exact effect). Two helpers are created: a construction of the target type
  and a levelling job.
- **MIL-UPG-2.2 Levelling.** If the target size class is small, medium or
  large (not a mine or harbour class) and the heights around the building
  are not level, the footprint is levelled first, as for a new site
  (`ECO-warehouse-construction.md`). **The start of the upgrade sets the
  building field's height again as a site's placement does**
  (`ECO-warehouse-construction.md` ECO-HQ-4.8a: when the six neighbours are not
  all at its height, the integer mean of the 19 fields within distance 2, spread
  like a levelling step); the leveller then brings the neighbours to that new
  value (added 2026-10-26 night, E-eco-019: the upgrade's start runs the site-height
  rule of a placement; measured, T6). Confidence: medium-high (levelling), high
  (the height).
- **MIL-UPG-2.3 Construction.** Then the construction runs as for a new
  building of the target type: its frame goods and building goods are
  requested from the warehouses and delivered to the building's flag, a
  builder is requested and works through the stages
  (`ECO-warehouse-construction.md` for the stage rules and times). With the
  game rule for instant building the construction completes at once.
  Confidence: high (same construction object and rules as a site), medium
  (that the builder's walk and animation around a standing building are
  unchanged).
- **MIL-UPG-2.2a One part per update.** The upgrade's update (each tick, in
  the buildings' part) does **one** of: the levelling part, while its progress is
  below complete; else the construction part (requests, orders, builder), while
  frame or main work is left; else the completion of 2.4. The levelling part, in
  an update that finds the footprint level (or a target size class that is not
  levelled), only marks the levelling complete; the construction part starts in
  the **next** update. So for an upgrade ordered in tick T (the computer player's
  command at the end of T, `AI-overview.md` AI-2.1b) on level ground, the update of
  T + 1 marks the levelling done and the builder request is filed and served in
  T + 2 (the material orders with it, as the construction part files them). This
  differs from a new site, whose first update levels and files in the same update
  (`ECO-requests-dispatch.md` ECO-REQ-1.3). (Added 2026-10-30, E-eco-040; measured T9.) Confidence: high for the builder request's
  tick (measured), medium for the orders' tick (code, not separately measured).
- **MIL-UPG-2.3a The builder's positions.** The four construction positions
  of each unit (`ECO-warehouse-construction.md` ECO-HQ-4.6a: offsets from the
  building flag's sample, per tribe) are those of the building's **current
  type**, the type it keeps until completion (MIL-UPG-3.1), not the target
  type's. The materials, and so the number of units, are the target
  type's. Measured: E-trace-011, guard house 90300 -> tower, builder 90825.
  All 32 work starts (8 units = the tower's 3 + 5 goods) follow the guard
  house's Roman offsets (-1, 2), (-2, 1), (1, 2), (0, 2), not the tower's
  (-1, 2), (1, 2), (-2, 0), (-1, 1) (T7). Confidence: high.
- **MIL-UPG-2.4 Completion.** The building's own update (each tick, in the
  buildings' part, before the settlers) checks its upgrade: while the levelling
  is not done it levels; while the frame or main part is left it runs the
  construction (requests, builder); once both are worked off it waits until **no
  builder is linked** to the construction any more (MIL-UPG-1.2), and in that
  update the building's type becomes the target type. Since the builder is
  released only at the end of his step's action (ECO-HQ-4.7) and drops his link
  in the settlers' part of that tick, the type changes in the **tick after the
  builder's release**, not with the completing decrement as for a new building
  (added 2026-10-27 night, E-ai-031; measured, T8). Then
  the helpers are removed and the
  building is re-registered: its flag is cleared and the claim and
  exploration run again with the new type's range (`TER-territory.md`
  TER-2.3). From then on the new type's soldier limit, coin rules and
  ranges apply (`MIL-soldiers.md`, `MIL-coins-promotion.md`); soldiers are
  requested up to the new limit by the usual rules. Confidence: high.

### MIL-UPG-3 During the upgrade

- **MIL-UPG-3.1** The building keeps its current type, its soldiers, its
  coins, its territory and its flag. Nothing moves the soldiers out or
  blocks them; no rule prevents attacks on it (not traced in the combat
  code: confidence medium).
- **MIL-UPG-3.2** For the computer players' military map
  (`AI-military.md` AI-MIL-1.1) an upgrading building counts as "under
  construction" (its upgrade construction has progress) while still
  counting its full threat and score. Confidence: high.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| upgrade chain | barracks, guard house, tower, fortress | | high | `military.lua` |
| order cost for the player | none up front; materials delivered | | high | E-ai-006 |
| AI stock test | twice the target type's frame goods + building goods | | high | E-ai-006; E-ai-029 |
| footprint test | same size class, or the bigger footprint fits ignoring the own building | | high | E-ai-006 |
| area re-marked at start | radius 4 | fields | medium | E-ai-006 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | finished barracks with 2 soldiers, level ground, planks and stones in a nearby warehouse | player orders the upgrade | accepted at once without stock check; the guard house's frame goods and building goods are requested; a builder comes; the two soldiers stay inside throughout; on completion the type is guard house, the claim runs again with the new range and more soldiers are requested up to the new limit |
| T2 | barracks still under construction; or a fortress; or a tower whose fortress footprint is blocked | order | refused (MIL-UPG-1.2) |
| T3 | tower on uneven ground where the fortress footprint is not level | order | the footprint is levelled first, then the construction starts |
| T4 | computer player, guard house with upgrade level above its own, 5 planks and 49 stones in stock (the tower costs 3 planks and 5 stones) | 37-tick upgrade run | no order: twice the costs (6 planks, 10 stones) are needed; ordered once 6 planks are in stock (`AI-military.md` T19) |
| T5 | upgrading barracks | AI military map refresh of its cell | its threat counts in both "threat" and "threat under construction", its score in "score" |
| T6 | MIL-UPG-2.2, ECO-HQ-4.8a: E-trace-011 (= E-trace-010 to 6000), player 1's guard house 90300 on (138, 57), height 7400 since its placement at counter 1101; its 19 fields within distance 2 sum to 141 884 in the heap dump of 5500 | the upgrade of counter 5582 and its levelling | the building field is 7467 (the mean) in the heap dump of 6000, with (138, 56) and (137, 57) levelled to 7467; by 6500 all six neighbours hold 7467 (measured). The leveller 90750, done at trace tick 6316, walks from (139, 56) (7467) towards (140, 56) (7646) at full speed, 180 per tick from 6320 (rise 179, class 0); levelling to 7400 instead makes that step class 1 (144 per tick, the dense trace's first settler difference) and slows the guard house (131, 59)'s construction at 7245 |
| T7 | MIL-UPG-2.3a: E-trace-011, player 1's guard house 90300 on (138, 57) upgrading to a tower (Romans); builder 90825 (made at counter 6335), material for the first unit at hand | the builder's work positions (fine coordinates of the raw settlers table, x 1000 in the s2trace rows) | unit 1: (555, 226) from trace tick 7646, (554, 225) from 7713, (557, 226) from 7802, (556, 226) from 7869; every later unit the same four (units from 7938, 8713, 9005, 9297, 9589, ...; 32 work starts in all); the tower's offsets would give (557, 226) second (an experiment build with them first differs at trace tick 7702, its builder walking east) |
| T8 | MIL-UPG-2.4: E-trace-011, player 1's guard house 90300 on (138, 57) upgrading to a tower (frame 3 planks, main 5 stones), builder 90825 on his eighth unit, last step begun at counter 10671 | the last unit | the main part falls below 0 at the unit's 213th decrement (counter 10718, the step's 48th tick; float32 as ECO-HQ-4.7); the builder works on to 10725 and is released (going home) at 10726; the building is a tower from counter 10727 (the recording's buildings table; trace tick 10728). Changing the type with the completing decrement (an experiment build) makes it a tower at 10718: the dense trace's buildings first differ at 10719, with the rule at 13256 |
| T9 | 2.2a: E-trace-013 (R3, the Hard game), player 1's barracks 93148 on (144, 54) (finished at counter 24604, its builder 93183 released and going home), upgraded to a guard house (same size class, level ground) by the computer player's upgrade run of counter 24710 (AI-MIL-4.5) | the builder for the upgrade | the builder request and the builder 93378 out of the castle in counter 24712 (the object numbers 93377 and 93378 taken in 24712; the builder walking from (139, 52) at 0.045 in 24712). Filing in the first update after the order (24711, our build) sent him a tick early: the Hard pair's settlers differed from 24720; with one part per update (experiment build) settlers 28010, buildings 29030 (was 25800), generator 28946 (was 25302), every other pair unchanged. The guard house -> tower upgrade of counter 17495 (the size class grows) takes its next two numbers (a request and its settler) in 17496 in both builds: its levelling part requests the leveller in the first update. The upgrade run's tick 24710 is our build's (the runs every 37 ticks agree with R3 at 10724, 15497 and 17495) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ai-006 | script | `data/scripts/properties/military.lua` | `mi_addBuildingUpgrade` x 3 |
| E-ai-006 | code reading | `S2DNG.exe` | can-upgrade test |
| E-ai-006 | code reading | | player command executor; computer player callers |
| E-ai-006 | code reading | | start: helpers, blocked area, area re-marking |
| E-ai-006 | code reading | | per-tick update: levelling, material requests, construction |
| E-ai-006 | code reading | | completion and type change |
| E-ai-006 | code reading | | military map counts an upgrading building as under construction |
| E-ai-029, E-trace-011 | code reading, memory reads of the running game | heap dumps 9500 and 10000 (guard house 90785) | MIL-UPG-1.2, 1.4 |
| E-ai-006 | strings | | button tooltips and the game event name |
| E-trace-011 | recording | `settlers.csv.gz` (builder 90825, counters 7645 .. 10728) | MIL-UPG-2.3a (T7) |
| E-ai-031, E-trace-011 | code reading and recording | buildings and settlers tables (90300 type at counter 10727, builder 90825 released 10726; 90785 finished 9938, builder 90795 released 9942) | MIL-UPG-1.2, 2.4 (T8) |
| E-eco-019, E-trace-011 | code reading and recording | E-trace-011 heap dumps 5500, 6000, 6500 (the height samples) | the upgrade's site height (MIL-UPG-2.2, T6) |
| E-eco-040, E-trace-013 | code reading and recording | `counter.csv.gz`, `settlers.csv.gz` (builder 93378 at 24712) | MIL-UPG-2.2a (T9) |

## Open questions

- What is drawn while an upgrade runs (the old building, a site, both).
- Whether the player can cancel a running upgrade and what happens to the
  delivered materials.
- Attacks and conquest of a building while it upgrades.
