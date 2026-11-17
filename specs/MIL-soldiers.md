# MIL-SOL: Soldiers and military buildings

Status: draft (read from the binary; values from the scripts; no in-game measurement)
Owner question(s): plan step 4.3; Q-056 (enemy distance); follows `MIL-coins-promotion.md` (MIL-PROM) and `TER-territory.md` (TER)
Game build: 11757

## Summary

Soldiers are settlers of five ranks. Warehouses turn helpers with a sword, a
shield and a beer into privates. Military buildings keep a number of
soldiers that depends on how close the nearest enemy military building is and
on the player's military settings; every 21 ticks each building asks for one
more soldier or sends one away. The headquarters keeps one soldier of each
rank outside its stock. Soldiers inside a building do nothing but slowly heal;
attacking and defending are plan 4.4.

## Rules

### MIL-SOL-1 Ranks and their values

- **MIL-SOL-1.1** Five ranks 0..4 (MIL-PROM-1.1): private, private first
  class, sergeant, officer, general. Values per rank:

  | Rank | Profession id | Max health (`mi_setSoldierMaxHealth`) | Attack value `a` | Defence value base |
  |---|---|---|---|---|
  | 0 private | 15E86843 | 3 | 2 | 1 |
  | 1 private first class | BA0E7F83 | 4 | 2 | 2 |
  | 2 sergeant | 466C6C23 | 5 | 3 | 3 |
  | 3 officer | 906014F3 | 6 | 4 | 4 |
  | 4 general | 559DDEA3 | 7 | 5 | 5 |

  Confidence: high for the ids and health (scripts); the attack and defence
  values are those of MIL-SOL-1.3.
- **MIL-SOL-1.2 Health.** A soldier has a health value, set to its rank's
  maximum when it is created. A hit lowers it by 1; at 0 or below the soldier
  dies. While present in a building (MIL-SOL-2.4) it heals: a counter rises
  by 1 per tick and, in the tick after it has passed 600, health rises by 1
  (not above the maximum) and the counter restarts at 0, so **1 point per 602
  ticks**; the counter is reset whenever the soldier is not present.
  Confidence: high for the rule and the start value. A promotion keeps the
  health (MIL-ATK-7.1); a soldier made from a warehouse stock starts at its
  rank's maximum (MIL-ATK-7.2). A homeless soldier (no warehouse in reach,
  `TRN-free-walk.md` TRN-FREE-6.4 step 4) loses 1 health after every 52nd
  failed search for a warehouse and dies at 0; there is no search for a
  military building (2026-11-09, Q-1034, E-mil-009).
- **MIL-SOL-1.3 Duel numbers (for plan 4.4).** A fight between two soldiers
  runs in rounds of the fighting time (`mi_setFightingTime` 16.7, so 17
  ticks per round, MIL-ATK-6.4). In each round the soldier whose turn it is (turns
  alternate) attacks the other: two numbers `r1`, `r2` are drawn in that order
  from the shared random generator, each `draw mod 7` (0..6). With
  `a = max(2, rank_attacker + 1)`, `h = a / 2` (integer division) and
  `b = rank_defender + 1 - h`, raised to 2 if it is below 3, the attack hits
  when `r1 <= a` and `r2 > b`, so with probability `(a + 1) / 7 x (6 - b) / 7`
  (private against private: 3/7 x 4/7 = 12/49). A hit takes effect at the
  start of the next round. Confidence: high. Who strikes first, the round
  order and timing: `MIL-attack-combat.md` MIL-ATK-6.
- **MIL-SOL-1.4 Exploring.** A soldier that is walking marks the fields within
  2 of its field as explored for its player (ECO-HQ-3.3), whenever its current
  field is not yet explored. Confidence: high.

### MIL-SOL-2 A military building's soldiers

- **MIL-SOL-2.1** The military buildings are the types with a territory
  range > 0 (TER-1.1). The headquarters is one, but is handled by MIL-SOL-6
  instead of 3 to 5. Confidence: high.
- **MIL-SOL-2.2** A building has a list of its soldiers (linked to it: on
  the way there or inside) and a list of its open soldier requests.
  Confidence: high.
- **MIL-SOL-2.3** `bp_setSoldierLimit` (barracks 2, guard house 3, tower 6,
  fortress 9) is the most soldiers it ever asks for (MIL-SOL-3.2).
  Confidence: high.
- **MIL-SOL-2.4 Present.** A soldier is present when it is linked to the
  building, is not moving, stands on the building's door field and is not in
  a fight or on an attack or defence errand. Present soldiers are the ones
  that count for occupation (TER-2.1), coins (MIL-PROM-2.1) and healing.
  Confidence: high.

### MIL-SOL-3 How many soldiers a building wants

- **MIL-SOL-3.1 Enemy distance.** Every 11 ticks (first in tick 10, then 21,
  32, ...; a counter 10 .. 0), for every military building other than the
  headquarters, the game sets its enemy distance: the smallest distance
  (MAP-GRID-2.1) from the building's field to an **enemy** military building
  (any type with range > 0, the headquarters included) whose territory flag is
  set (TER-2.1); 1000 if there is none. Players X and Y are enemies when
  X != Y and they are not in the same team (a team of -1 is nobody's team,
  `UI-game-setup.md`). The value is -1 until the first update after the
  building is created; the first update also sets the coin accumulator to 0
  (MIL-PROM-3.3). Confidence: high.
- **MIL-SOL-3.2 Wanted soldiers.** `wanted = max(1, trunc(f x limit + 0.5))`
  with `limit` the soldier limit and `f` the player's allocation factor for
  the building's enemy distance `n`: **far** if `n > 24`, **medium** if
  `19 <= n <= 24`, **near** (close) if `n <= 18` (so also while `n` is still
  -1). Defaults: near 1.0, medium 0.3, far 0.1 (MIL-SOL-7). If the building's
  "move soldiers out" switch is on (4.6), `wanted` is instead 1 while the
  building has not yet claimed land and 0 afterwards. Confidence: high.
- **MIL-SOL-3.3** With the defaults: far from enemies every type wants 1;
  at medium distance barracks 1, guard house 1, tower 2, fortress 3; near:
  the full limit. Confidence: high (follows from 3.2).

### MIL-SOL-4 Requesting and sending away (every 21 ticks)

- **MIL-SOL-4.1 Timing.** Every 21 ticks (first in tick 20, then 41, 62, ...;
  a counter 20 .. 0), in the military update (ECO-REQ-0.1), the military
  buildings are visited in the world's military building list order. A
  building is skipped while it is not finished (frame or main work left) and
  if it is the headquarters type. Confidence: high.
- **MIL-SOL-4.2** For a visited building let `S` be the number of its soldiers
  (MIL-SOL-2.2, present or not), `R` the number of its open requests and
  `d = wanted - S - R`. `R` counts every settler request linked to the
  building (filed for it and not yet served or cancelled), whatever its
  profession: its soldier requests, and while it is being upgraded
  (`MIL-building-upgrade.md` MIL-UPG-2) the upgrade's leveller and builder
  requests (`ECO-requests-dispatch.md` ECO-REQ-1.2, 1.3). The building keeps
  these links in filing order. Confidence: high (E-trace-011, T18; E-mil-006).
- **MIL-SOL-4.3 Too few (`d >= 1`).** It files **one** soldier request (a
  settler request of `ECO-requests-dispatch.md` ECO-REQ-1, good "any
  soldier"), whatever `d` is. So a building gains at most one request per 21
  ticks. Confidence: high.
- **MIL-SOL-4.4 Too many (`d < 0`).** In this order, each step raising `d` by
  one, until `d = 0`:
  1. open requests of the building (4.2) are cancelled, one at a time, the
     oldest first (filing order), whatever their profession. A cancelled
     request loses its link to the building at once and is taken out of the
     queue, unserved, when its turn comes (ECO-REQ-2.3). A cancelled leveller
     or builder request of an upgrade is filed again, with a new number, by
     the building's next update (the next tick: no request of that profession
     is linked and none is assigned, ECO-REQ-1.2, 1.3) and goes to the end
     of the queue (T18);
  2. soldiers that are **not present** (still on their way) are released,
     in list order;
     **"Not present" is any linked soldier not standing inside the building (2026-11-17 night, Q-142;
     E-mil-018, code reading, high):** a soldier counts as present only when he is linked to the
     building, stands in it and has no attack target and no fight partner. So an attacker sent out from
     the building, or one fighting or returning from an attack, is "not present" and is released here
     like a soldier on his way in. A released attacker has no home building any more: when his attack
     ends he is homeless and goes to a warehouse (`MIL-attack-combat.md` MIL-ATK-4.4, TRN-FREE-6.4). One
     the building still keeps becomes present again the moment he arrives back (no test of the wanted
     number then); a later pass releases a surplus by step 3.
  3. if `d` is still below 0, at least one soldier is left and the building's
     release cooldown is 0: **one** present soldier is released (MIL-SOL-4.5)
     and the cooldown is set to 40.
  The cooldown falls by 1 in every building update of that building. So
  outside step 1 and 2 a building lets at most one present soldier go per
  pass, and because 40 > 21, in practice one per 42 ticks. Confidence: high.
- **MIL-SOL-4.5 Which soldier leaves.** With the defense-type setting 1
  ("prefer strong soldiers", the default) the present soldier with the
  **lowest** rank leaves (first in list order on equal rank); with the other
  setting the one with the **highest** rank. A released soldier is no longer
  linked to the building and goes home (`TRN-free-walk.md` TRN-FREE-6.4) to a
  warehouse, whose stock it joins. Confidence: high for the choice; medium
  for the setting's label.
  **How it leaves (2026-10-27 b, measured T19):** it first takes the one step
  from the building's field to the building's flag; it is sent home only on
  arriving there: with T the tick it arrives on the flag's point, it goes
  through `TRN-free-walk.md` TRN-FREE-6.4a as a settler standing on its
  field's point (stands in T, T + 1 and T + 2, its walk home ordered at
  progress 0 in T + 3 and first advancing in T + 4). The walk home is
  searched from the flag then, not from inside the building. Confidence:
  high (one soldier measured tick by tick).
- **MIL-SOL-4.6 Move soldiers out.** The player command "toggle moving
  soldiers out" (`toggle-soldiers-out`, per building) switches the rule of 3.2. Confidence: high for
  the effect on `wanted`; its intended use (emptying a building while keeping
  its land, TER-2.1) is inferred.
- **MIL-SOL-4.7 Serving a soldier request.** The request queue
  (ECO-REQ-2, 4 requests per tick) serves it from the warehouse with the
  lowest route cost (ECO-REQ-3) among the finished warehouses connected to
  the building's flag that hold **at least one soldier of any rank**; a
  warehouse never makes a soldier for a request. That warehouse sends, with
  defense type 1, its **highest** rank in stock (general first), otherwise
  its lowest (private first). The soldier leaves by the door like any settler
  (door cooldown 30, ECO-REQ-3.3), walks along the roads to the building's
  flag and enters (as a worker does, ECO-REQ-3.6); it is linked to the
  building from the start, so it counts in `S` while walking.
  Confidence: high for the choice; medium for the walk being the worker's.
- **MIL-SOL-4.7a The soldier's first tick.** Unlike a worker, builder or
  leveller sent for a request (`ECO-time-units.md` ECO-TIME-1.5: they move in
  their creation tick), the soldier **stands in its creation tick** on the
  warehouse's door field point with the walk animation at progress 0 and takes
  its first step (door field -> building field) in the next tick; from then on
  its walk is a settler's way out of a warehouse (`ECO-worker-cycle.md`
  ECO-WORK-3.8a, 3.8c), one tick later. As for a worker, its road walk ends
  on the military building's flag point and the step into the building
  starts there at progress 0 in the next tick (ECO-WORK-3.8c). Confidence:
  high for this case (measured, E-trace-010: soldier 90744 made at counter
  5502 for the guard house 90300, on (556000, 208000) in trace tick 5503,
  (556090, 207819) in 5504, on the flag point (556000, 224000) of (139, 56)
  in 5755, standing in the building in 5783; an experiment build with both
  rules matches every row of it to 6000);
  medium for every soldier sent out of a warehouse (the only soldier sent
  in the dense recording; the code links the soldier to its building through a
  different call than a worker, E-eco-019,
  and neither starts the walk there, so the soldier's own first update starts
  it).

### MIL-SOL-5 Recruitment

- **MIL-SOL-5.1** Every 101 ticks (first in tick 100, then 201, ...; a counter
  100 .. 0), for every warehouse of every player (headquarters, warehouses,
  harbours; in the world's warehouse list order): up to `k` times, while the
  warehouse's stock holds the private's whole cost list (`Jobs.lua`: helper 1,
  sword 1, shield 1, beer 1), those goods are removed from its stock and one
  private (rank 0) is added to it. `k` is the player's recruit count, 10 by
  default. Confidence: high.
- **MIL-SOL-5.2** No military-settings command changes `k` (it is reset to 10
  by "reset"), and computer players do not change it either (MIL-ATK-7.4).
  Confidence: high.
- **MIL-SOL-5.3** Higher ranks are never recruited; they only come from
  promotion (MIL-PROM-4). Confidence: high.

### MIL-SOL-6 Soldiers of the headquarters

- **MIL-SOL-6.1** The headquarters keeps soldiers in its stock (by rank) and
  in addition a wanted number of soldiers per rank **outside the stock**,
  linked to it and standing at its door (present), default **1 of each
  rank** (for the headquarters type; 0 for any other type of this kind). The
  amounts are changed per building and rank with "set castle soldier amount".
  Confidence: high for the defaults; medium for the command's arguments
  (building, rank, amount).
- **MIL-SOL-6.2** Every tick, while no attackers are reported at the
  headquarters (its attacker list is empty): for each rank 0..4, if more
  soldiers of that rank are present than wanted, one present soldier of that
  rank is released (and goes back into the stock); if fewer, and the stock
  holds one of that rank, and **the warehouse's door is free** (its send
  cooldown of `ECO-requests-dispatch.md` ECO-REQ-3.7 is 0), one soldier of
  that rank is taken out of the stock at the door and linked to the building,
  and **that starts the 30-tick cooldown** as a settler sent out does. So
  soldiers come out at most one every 30 ticks, and a soldier coming out
  blocks the warehouse's other sending and its take-out draw
  (`ECO-distribution.md` ECO-DIST-1.7c) for those 30 ticks, as any settler
  sent out blocks the soldiers. (Corrected 2026-11-01, T20: the field tested
  was read as "the building is not being destroyed"; it is the door wait.)
  Confidence: high (E-mil-008; the take-out draw tests
  the same field; measured T20).
- **MIL-SOL-6.3** While attackers are reported and no soldier is present, one
  soldier of each rank that is in stock is brought out, general first (plan
  4.4), each only while the door is free (6.2), so in one tick only the first
  of them comes out. Confidence: high for the order; medium for the door
  test here (the same test as 6.2 in the code, not measured); the attack side
  is plan 4.4.
- **MIL-SOL-6.4** The headquarters takes no part in 3, 4 (no requests, no
  enemy distance) and in promotion (MIL-PROM-2.1); its stock soldiers are the
  source for soldier requests (4.7). Confidence: high.

- **MIL-SOL-6.5 Where the present soldiers stand.** The headquarters' door field is
  its own field stepped once in the door offset direction (ECO-CAT-2.4: north-west,
  direction 4 of MAP-GRID-1.2), so for a castle at (x, y) with y odd it is (x, y+1),
  with y even (x-1, y+1). A present soldier stands on that field, at the field's own
  height sample (fine position = 4 x the field, MAP-GRID-4.2), with the Stand
  animation, from the end of the update that took it out of the stock (the first
  update of a new game, SIM-START-1.3) until it leaves. It never stands on the
  castle's own field. Confidence: high (measured, live trace 2, 5000 ticks for player
  0's soldier).
- **MIL-SOL-6.6 Leaving.** The present soldier is released (6.2, back into the
  stock) in the tick the wanted number drops below the present number; it vanishes
  from the map in that tick (no walk). On MP_2P_Storm_Coast the computer player's
  soldier left at trace tick 609, player 0's never (no command). Confidence: high for
  the vanishing (measured); the computer player's reason is AI-MIL-4.6 (medium).
### MIL-SOL-7 Military settings (per player)

- **MIL-SOL-7.1** The military settings dialog ("Military Settings") sets
  these player values, each by its own command (`set-close-occupation`,
  `set-medium-occupation`, `set-far-occupation`, `set-attack-strength`,
  `set-interceptors`, `set-soldier-preference`, in the order of the table),
  and has a reset button ("Reset dialog settings", `reset-military-settings`)
  that restores all defaults at once:

  | Setting (dialog label) | Default | Used by |
  |---|---|---|
  | allocation factor "Close" (near) | 1.0 | MIL-SOL-3.2 |
  | allocation factor "Medium" | 0.3 | MIL-SOL-3.2 |
  | allocation factor "Far" | 0.1 | MIL-SOL-3.2 |
  | "Attack Strength" factor | 1.0 | attacks (plan 4.4) |
  | "Intercepting Defenders" (defend outside) factor | 1.0 | defence (plan 4.4) |
  | defense type: "Prefer strong soldiers" (1) / "Prefer weak soldiers" (other) | 1 | MIL-SOL-4.5, 4.7 |
  | recruit count `k` (no command) | 10 | MIL-SOL-5.1 |

  A slider's position is sent as the factor itself; slider values are held
  in 0..1 (MIL-ATK-7.3; the number of slider steps is not traced). Confidence: high for values and use; medium for
  the label-to-value mapping of the defense type.
- **MIL-SOL-7.2** Per building (building window): "Toggle coin supply ON/OFF"
  (`toggle-coins`, MIL-PROM-3.2, on at creation), "Toggle Moving soldiers out"
  (`toggle-soldiers-out`, 4.6, off at creation), upgrade (`upgrade-building`,
  `mi_addBuildingUpgrade`, TER-2.3), attack (`attack-building`, plan 4.4).
  Confidence: high.

### MIL-SOL-8 What soldiers do in a building

- **MIL-SOL-8.1** A present soldier stays at the door field doing nothing
  except healing (1.2) and being promoted (MIL-PROM-4). It leaves only when
  it is released (4.4, 6.2) or sent to attack or defend (plan 4.4). A soldier
  that leaves for any reason also has to wait for the building's release
  cooldown to be 0 and then sets it to 40 (one departure per 40 ticks).
  Confidence: high for the cooldown use; medium for the full list of reasons.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| enemy distance period / first | 11 / tick 10 | ticks | high | E-mil-002 |
| allocation period / first | 21 / tick 20 | ticks | high | E-mil-002 |
| recruiting period / first | 101 / tick 100 | ticks | high | E-mil-002 |
| promotion first run | tick 300 | | high | E-eco-013 |
| far / medium thresholds | n > 24 / n > 18 | fields | high | E-mil-002 |
| no-enemy distance | 1000 | fields | high | E-mil-002 |
| release cooldown | 40 | ticks | high | E-mil-002 |
| heal step | 1 per 602 | health / ticks | high | E-mil-002 |
| recruit count | 10 | per warehouse per run | high | E-mil-002 |
| allocation defaults | near 1.0, medium 0.3, far 0.1 | | high | E-mil-002 |
| max health per rank | 3, 4, 5, 6, 7 | | high | `military.lua` |
| fighting time | 16.7 | ticks | high (value) | `military.lua` |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | guard house (limit 3) finished, no enemy anywhere, default settings | allocation passes | one request in the first pass (wanted 1); none later once the soldier is on its way |
| T2 | tower (limit 6), enemy occupied barracks at distance 20 | wanted | medium: trunc(0.3 x 6 + 0.5) = 2 |
| T3 | fortress (limit 9), enemy at distance 18 | wanted | near: 9; one request per 21-tick pass, so 9 requests after 9 passes (if none are served earlier) |
| T4 | barracks with 2 present soldiers (ranks 0 and 3), enemy moves out of reach (distance becomes 1000) | next passes | wanted 1: the rank-0 soldier is released (defense type 1); no further release |
| T5 | as T4 with defense type "prefer weak" | next pass | the rank-3 soldier is released |
| T6 | fortress with 9 present soldiers, wanted drops to 1 | passes from tick P | releases at P, P + 42, P + 84, ... (cooldown 40, passes 21 apart) |
| T7 | building with 2 soldiers walking towards it and wanted 0 (move out) | pass | both walking soldiers released at once |
| T8 | HQ stock: 3 privates, 1 general; a barracks requests a soldier, defense type 1 | serving | the general is sent |
| T9 | warehouse with helper 12, sword 3, shield 5, beer 9 | recruiting run | 3 privates made; helper 9, sword 0, shield 2, beer 6 |
| T10 | warehouse with 30 complete sets | one run | 10 privates; the next 10 after 101 ticks |
| T11 | new game, HQ stock with soldiers of every rank, no attackers | first ticks | one soldier of each rank leaves the stock and is linked to the HQ (6.2); the stock count of each rank falls by 1 |
| T12 | soldier with health 1 of max 3, present | 1204 ticks | health 3 |
| T13 | private attacks private | one attack | hit probability 12/49 |
| T14 | general attacks private | one attack | a = 5, h = 2, b = max-rule(0 + 1 - 2) = 2: probability 6/7 x 4/7 = 24/49 |
| T15 | players 0 and 1 in team 0, player 2 in team -1 | enemy distance of a player-0 building | only player 2's (and team -1 players') buildings count |
| T16 | castle at (29, 51) (y odd), new game | the present soldier at trace tick 1 and after | field (29, 52), fine (116000, 208000), Stand; castle at (139, 51): field (139, 52), fine (556000, 208000) |
| T17 | castle at (40, 40) (y even) | door field | (39, 41) |
| T18 | E-trace-011 (`specs/traces/original-dense-20000*`): player 1's guard house 90300 at (138, 57), upgrading since counter 5582, one present soldier (90744), enemy far (wanted 1); the upgrade's builder request 90823 filed at counter 6317 | allocation pass of counter 6320 (20 + 21 x 300) | d = 1 - 1 - 1 = -1: request 90823 is cancelled (in the objects table its link is already none in the snapshot of 6320); counter 6321: the upgrade files builder request 90824 and the queue deletes 90823 unserved; 90824 is served at 6335 by builder 90825, made from a helper and a hammer of castle 90180 (helpers 83 -> 82, hammers 16 -> 15); later numbers are as recorded, e.g. road 90837's carrier is 90841 (counting soldier requests only gives 90840; settlers' first difference against E-trace-011 7420 -> 7702) |
| T19 | MIL-SOL-4.5, TRN-FREE-6.4a: E-trace-013 (Hard game, every tick recorded from counter 0; the same game as E-trace-012) and E-trace-012, player 1's soldier 90700 garrisoned in the military building on (138, 57), flag (139, 56), released | its ticks | garrison state to counter 4744; the step to (139, 56) at progress 0 in 4745, on the flag's point and standing in 4768, 4769, 4770; the walk home (towards (140, 56)) at progress 0 in 4771, 0.045 in 4772. E-trace-012 (every 10 ticks there): standing on (139, 56) in trace tick 4770, at fine (557440, 224000) in 4780. Experiment build: the Hard pair's settlers 4770 -> 4830 (a walk home searched inside the building and walked without the stand passes the flag at once) |
| T20 | 6.2 (the door wait): E-trace-019 (R6, MP_4P_Fight, players 1 .. 3 Hard), player 3's castle (26, 24), door field (25, 25); its computer player raises the castle reserve in counter 598 (`AI-military.md` AI-MIL-4.6, (598 + 15) mod 613 = 0) | the soldiers present at the door, by trace tick | 2 from 599, 3 from 629, 4 from 659, 5 from 689 (one every 30 ticks); the warehouse's take-out draw of counter 602 (ECO-DIST-1.7) is skipped for this castle (2 draws, not 3). Bringing one out every tick (5 more by 604) differs from settlers 600 and generator 603; with the door wait (experiment build): settlers 784, buildings 1156, generator 1660; every Storm Coast pair unchanged |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-mil-002 | code reading | `S2DNG.exe` | military update: recruiting, distances, allocation, fight connections, promotion |
| E-mil-002 | code reading | | recruiting |
| E-mil-002 | code reading | | enemy distance |
| E-mil-002 | code reading | | allocation, wanted count, release choice, cooldown |
| E-mil-006 | code reading | | MIL-SOL-4.2, 4.4 step 1 (T18) |
| E-trace-011 | live read of the running game | `objects.csv.gz` (requests 90823, 90824), `stocks.csv.gz` (castle 90180), `settlers.csv.gz` (90825) | T18 |
| E-mil-002 | code reading | | per-building military part: HQ soldiers or coin check, cooldown decrement |
| E-mil-002 | code reading | | headquarters soldiers |
| E-mil-002 | code reading | | serving a soldier request |
| E-mil-002 | code reading | | present test |
| E-mil-002 | code reading | | soldier update: exploring, healing, damage, death, health init |
| E-mil-002 | code reading | | duel round |
| E-mil-002 | code reading | | settings defaults and commands (`toggle-soldiers-out`, MIL-SOL-7.1's seven) |
| E-mil-002 | strings | | dialog labels |
| E-mil-008, E-trace-019 | code reading and recording | R6 settlers table (player 3's door field (25, 25)) | MIL-SOL-6.2, 6.3, T20 |
| E-trace-002 | live read of the running game | `settlers.csv.gz` (objects 90186, 90187), `specs/traces/original.s2trace.gz` ticks 1 .. 5000 | present soldiers on the door field; player 1's gone at trace tick 609 |
| E-sim-001 | code reading | | soldier death outside a fight (ECO-WORK-6) |
| E-trace-013 | live read of the running game | TRACED plan R3, recorded 2026-10-27 from 01:18, MP_2P_Storm_Coast, player 1 computer Hard, every tick from counter 0, heap dumps every 500 | the Hard game tick by tick (T19) |
| data | script | `military.lua`, `Jobs.lua`, `buildings.lua` | health, fighting time, soldier ids and cost, limits |

## Open questions

- Number of steps of the military settings sliders (MIL-SOL-7.1).
- Fights, attacks and defence: `MIL-attack-combat.md`; conquest:
  `MIL-conquest.md`.
