# AI-MISC: Computer opponent, ships, start and re-initialisation, profiles, per-tick update

Status: draft (rules read from the binary; not yet checked against the running game)
Owner question(s): Q-067 (ships part), Q-069, Q-070, Q-071, Q-080; plan step 5.1 chunks 3 and 4
Game build: 11757

## Summary

Three leftovers of the computer player: how it uses shipyards, ships and
expeditions; how an AI player is switched on at game creation and what the
"re-initialisation" of `AI-overview.md` AI-2.1 really is (a complete reset
that switches the player off when it has lost every cell); which decision
reads which profile value and script weight; and what the per-tick update
of its planning objects does. Models and orders are in `AI-construction.md`
(AI-CON-1, AI-CON-5), military rules in `AI-military.md`.

## Rules

### AI-MISC-1 Ships and expeditions

- **AI-MISC-1.1 Shipyard (every 673 ticks).** If the AI has no shipyard: the
  first cell (in cell order) that contains a finished harbour and has no
  "full" mark for the good `ship` gets a shipyard order, weight 10, if the
  part's order slot is empty. An order that ends with **no site** (its
  search found no field) marks that cell full for `ship`; one that fails for
  lack of material leaves no mark. By `AI-construction.md` AI-CON-1.6 the
  first such mark lasts only until the next 37-tick check, so the cell gets
  the order again 673 ticks later; the second lasts 600 s (clarified
  2026-10-27 night, E-ai-031; measured,
  `AI-construction.md` T53). With a finished shipyard: its mode
  is switched to ships if it is in boat mode; production is off while
  stock(plank) < 12; otherwise production is on while the number of own
  finished ships is below 2 x the number of own harbours (over all cells),
  else off. Confidence: high (Q-080: the per-cell list is the harbour list,
  `AI-construction.md` AI-CON-1.1).
- **AI-MISC-1.2 Starting an expedition (every 2111 ticks).** If the player
  has no expedition ship and a harbour's construction costs are in stock:
  over the cells whose landmass is the home landmass (`AI-construction.md`
  AI-CON-1.1b (b), (c)), the harbours are checked in order; if
  one is already preparing an expedition nothing happens; the first harbour
  with an own idle ship at it (not moving, no expedition, no target, empty
  cargo, same water) gets the "prepare expedition" command
  (`SHIP-ships-expeditions.md` SHIP-5.1). Confidence: medium-high.
- **AI-MISC-1.3 Sailing (every 2111 ticks).** With an expedition ship: if it
  stands still, its expedition is active, a landing spot is known, valid and
  within distance 5, the "build harbour" command is given (SHIP-7). Else if
  the ship stands still, one of the compass directions that has a landing
  option is chosen uniformly at random and the ship is sent there (SHIP-6).
  Confidence: high.
  **The build test is the founding test (2026-11-19 night, E-ai-076: code reading and the Hard
  recording; high).** "A landing spot is known, valid and within distance
  5" is exactly the test of the ship window's "Construct harbor" button (`SHIP-ships-expeditions.md`
  SHIP-7.1): the ship not moving, expedition mode on, a **last port position** (SHIP-2.7), and the
  harbour spot of that position's land continent nearest to the ship a harbour site for nobody or
  the player and **closer than 5** to the ship. It does **not** read the ship's target, which the
  arrival has cleared (SHIP-6.4); so a ship that has sailed to a landing field and stopped there is
  told to found the harbour at the next run. When the test holds, the run gives `found-harbour`
  and nothing else: no draw. Only when it fails and the ship stands still is the direction drawn
  (one draw, the count of directions with an option) and `sail-expedition` given. Test T9.

### AI-MISC-2 Activation and re-initialisation (Q-069)

- **AI-MISC-2.1 Activation.** When the game creates its players, every
  computer player is activated with its profile ("1" weak, "2" normal, "3"
  strong, from the setup strength 0/1/2): the AI records the home position
  and its landmass (the map's stored continent index at that position,
  `AI-construction.md` AI-CON-1.1b (c); kept for the whole game), loads the profile, resets all its parts, creates its map
  views and becomes active. Every building the player already has is then
  announced to it. Confidence: high.
- **AI-MISC-2.2 Buildings announced.** Every new building of an active AI
  player creates a building record, puts it into the cell at position / 16,
  creating that cell if needed (so the headquarters creates the first cell
  at once), and a catapult raises the catapult count of its territory cell;
  a removed building is dropped (catapult: count halved). Confidence: high.
- **AI-MISC-2.3 Re-initialisation.** In the per-tick think the AI is reset
  instead of thinking when it has no cells at all. The reset deletes every
  building record, need, production and cell, resets every part and map,
  clears the profile and switches the player **off**; it is not switched on
  again by the game (only activation or loading a save does that). Cells are
  removed only when empty and unowned, so this happens when the player has
  lost all its land. A loaded save re-initialises an active AI before reading
  its state. Confidence: high (mechanics), medium (never re-activated: no
  other writer of the active flag was found).
- **AI-MISC-2.4 Global switch (Q-080).** The AI system has one enable flag
  for all computer players. It is set on when the system is created, saved
  and loaded with the game, and nothing else writes it; so computer players
  always think. Confidence: high.

### AI-MISC-3 Profile and script weights (Q-070)

- **AI-MISC-3.1 Profile values.** military mode: defense type (AI-MIL-2.3) and
  the attack decision and count (AI-MIL-5.4); start sawmill count: the plank
  needs at t = 0 (AI-CON-2.1); maximum smithies: sword and beer needs exist
  only if above 0 (AI-CON-2.2) and the smithy count limit (AI-MIL-6.2);
  maximum mints: the coin need (AI-CON-2.3) and the mint limit (AI-MIL-6.3);
  attack strength and intercepting factor: the player's settings every 53
  ticks (AI-MIL-2.1); street / route optimizing: AI-CON-8.2 / 8.3; catapults:
  AI-MIL-6.4; coin arrangement: AI-MIL-4.7. No other reads exist. Confidence:
  high.
- **AI-MISC-3.2 Script weights.** `bp_setAiThreat` and `bp_setAiScore` fill
  the military map (AI-MIL-1.1) and through it the attack target value, the
  attack candidate ranking (AI-MIL-5.2, 5.3), the military site choice
  (AI-MIL-4.2) and the catapult score (AI-MIL-6.4). `gp_setAiPriority`
  weights only the needs construction score (AI-CON-4.2). Confidence: high.

### AI-MISC-4 Per-tick update (Q-071)

- **AI-MISC-4.1** Once per tick, for every active AI player, every building
  record, need, production and cell re-resolves its references (dropping
  ids of objects that no longer exist) and every building record recomputes
  its four protection flags (AI-CON-1.2). The waiting list does the same. No
  decision is taken; the think (AI-overview AI-2) is the only place where
  the AI acts, so the order of update and think within a tick does not
  change behaviour. The update's caller is an indirect call that was not
  traced. Confidence: high (content), medium (caller).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| shipyard plank minimum | 12 | wares | high | E-ai-004 |
| ships wanted | 2 x harbours | ships | high | E-ai-004, E-ai-005 |
| harbour build distance | < 5 | fields | high | E-ai-004 |
| profiles | setup strength 0/1/2 -> "1"/"2"/"3" | | high | E-ai-004 |
| re-init condition | no cells | | high | E-ai-004 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | AI with 2 harbours and 1 military building, finished shipyard in boat mode, 20 planks, 3 ships | shipyard run | mode switched to ships; 3 < 4: production on; with 4 ships: off |
| T2 | same, 10 planks | shipyard run | production off |
| T3 | AI without expedition ship, harbour costs in stock, harbour with an idle own ship | expedition run | `prepare-expedition` command for that harbour |
| T4 | expedition ship stopped, landing spot valid at distance 3 | expedition run | `found-harbour` command |
| T5 | expedition ship stopped, no spot, options in 2 of 6 directions | expedition run | one of the two directions chosen with probability 1/2 each |
| T6 | computer player at game creation | creation | profile set, home recorded, headquarters announced: one cell; the headquarters is a depot (AI-CON-1.1, corrected 2026-10-24), so it goes into the cell's depot list and the cell's depot-wanted marker set at the cell's creation is cleared again (AI-CON-1.6; before 2026-10-24 this row said the cell keeps the marker) |
| T7 | active AI that lost every cell | next tick | all records deleted, profile cleared, player inactive; no further thinks |
| T8 | AI-MISC-1.1 with AI-CON-1.6: E-trace-011, player 1's harbour in cell (9, 2), no shipyard | shipyard part every 673 ticks | orders at counters 6724 (no site, mark expires at the next check), 7397 and 8070 (no material, no mark), 8743 (no site at 9022, mark for 600 s): no order at 9416 (`AI-construction.md` T53) |
| T9 | AI-MISC-1.3 (the build test): E-trace-013 (R3, the Hard game), player 1's expedition ship 94738 leaves its harbour (144, 47) after the expedition run of counter 48 547 (sailing from counter 48 548) and stops on (149, 136) in counter 49 064; its last port position is then (148, 137) (the recording's ships table); the harbour spot nearest to it, (147, 136), is free | the next expedition run, counter 50 658 | `found-harbour`: the cargo is out in counter 50 658, the harbour site on (147, 136) stands from then (the B row of trace tick 50 700: a frame), the empty ship sails home from 50 662; no draw in that run (trace tick 50 659's draws are the animals' only). Testing the ship's target instead (cleared at the arrival) sends it on: one direction draw and the 13 draws of its sea route, the Hard pair's generator apart from trace tick 50 659; with the rule: not yet measured (no experiment build on 2026-11-19 night) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ai-004 | code reading | `S2DNG.exe` | shipyard |
| E-ai-004 | code reading | | expedition |
| E-ai-004 | code reading | | activation, building announced / removed |
| E-ai-004 | code reading | | think, re-init condition, reset, load |
| E-ai-004 | code reading | | per-tick update |
| E-ai-002, E-ai-003 | code reading | see `AI-construction.md`, `AI-military.md` | profile reads |
| E-ai-005 | code reading | | harbour list; global switch |
| E-ai-031, E-trace-011 | code reading, memory reads of the running game | heap dumps 7000 .. 9500 | AI-MISC-1.1 (T8) |
| E-ai-076, E-trace-013 | code reading, recording | (session 2026-11-19 night): the expedition run's two branches (the founding test first, the direction draw only without it); R3's ships table from counter 48 400 | AI-MISC-1.3 the build test (T9) |

## Open questions

- None open from this file (Q-080 answered in E-ai-005).
