# MIL-PROM: Gold coins and soldier promotion

Status: draft (read from the binary; no in-game measurement)
Owner question(s): leftover of plan 3.2 ("soldier promotion with coins"); see `ECO-professions.md`
Game build: 11757

## Summary

Military buildings order gold coins for their soldiers. Every 301 ticks a
global promotion run visits every military building: if it holds at least one
coin and at least one soldier below the top rank, it promotes **one soldier of
each rank below general** (at most four soldiers) and uses **one** coin for the
whole step. Promotion changes the soldier's profession to the next rank; the
cost lists of `Jobs.lua` ("previous rank + coin") are not used for it.

## Rules

### MIL-PROM-1 Ranks

- **MIL-PROM-1.1** A player has five soldier ranks, index 0 to 4: private
  (`good_soldier0`, 15E86843), private first class (BA0E7F83), sergeant
  (466C6C23), officer (906014F3), general (559DDEA3) (`ECO-professions.md`
  ECO-PROF-2). A soldier's rank is the index of its profession in this list.
  Confidence: high.

### MIL-PROM-2 Which buildings take part

- **MIL-PROM-2.1** A building **can use coins** when all hold:
  - its type claims territory (territory range > 0, `bp_setTerritoryRange`);
  - its type is not a warehouse that keeps soldiers (headquarters, harbour:
    these keep soldiers by rank in stock instead);
  - at least one soldier that is present in the building (not out
    defending or attacking) has rank 0 to 3.
  Confidence: high for the checks; medium for naming the type flag
  "warehouse that keeps soldiers".

### MIL-PROM-3 Ordering coins

- **MIL-PROM-3.1** A military building checks its coins once every 33 ticks,
  on the ticks where `(t + k) mod 33 = 0` (unsigned 32-bit sum), `t` the
  logic tick and `k` the low 32 bits of the building's **object number**: a
  64-bit counter shared by all game objects that get one (buildings, ships,
  ...), handed out in creation order and saved with the game (Q-056).
  Confidence: high for the period and the field; medium for which objects
  share the counter.
- **MIL-PROM-3.2** At a check:
  - if no soldier is present, or the building's **coin supply switch** is
    off (the player command "toggle gold coin supply", `toggle-coins`, per building), or the
    building cannot use coins (MIL-PROM-2.1): its open coin orders are
    cancelled;
  - otherwise, only if it has **no** open coin order: it orders
    `stock limit - coins in store` coins (the building type's stock limit,
    `bp_setStockLimit`), one order per coin.
  Confidence: high. The coin supply switch is **on** when a building is
  created (Q-056). Confidence: high.
- **MIL-PROM-3.3** Coins are delivered by order matching
  (`ECO-distribution.md` ECO-DIST-3; coins never take a free ware, ECO-REQ-5.3).
  Which coin order is served: among the open coin orders whose building is
  reachable and can use coins, each such building's coin accumulator is
  raised by `1 / n`, and the order of the building with the **highest**
  accumulator is served (strictly higher wins, so the first in order list on
  ties); that building's accumulator is set to 0. `n` is the building's
  **enemy distance** (`MIL-soldiers.md` MIL-SOL-3.1): the distance to the
  nearest occupied military building of an enemy, 1000 if there is none. So
  buildings near the enemy collect coins faster. Before the first distance
  update `n` is unset (-1): no increment, and the accumulator starts at -1;
  the first update sets the accumulator to 0 (Q-056). **Only that first
  update** resets it; later distance updates (every 11 ticks) change `n`
  only, so an accumulator carries over between servings for as long as the
  building waits (Q-140). The raise is `acc = float32(acc + float32(1 / n))`
  (`ECO-time-units.md` ECO-TIME-5.1). Confidence: high.

### MIL-PROM-4 Promotion

- **MIL-PROM-4.1** A global promotion counter runs once per logic tick, after
  the transport and resource updates (ECO-REQ-0.1). While it is above 0 it is
  lowered by 1; when it is 0 the promotion run happens and the counter is set
  to 300. So runs are **301 ticks apart**. The counter is saved with the
  game. It is **300 at game start**, so the first run is in **tick 300**
  (Q-056), then 601, 902, ... Confidence: high.
- **MIL-PROM-4.2** A run visits all military buildings of all players, in the
  world's military building list order. For each building that can use coins
  (MIL-PROM-2.1) and holds at least one coin:
  1. Go through its soldiers in their list order. A soldier that is present
     and has rank `r` from 0 to 3 is promoted to rank `r + 1` if no soldier
     of (original) rank `r` has been promoted in this step yet.
  2. Then remove **one** coin from the building's store.
  So one coin promotes up to four soldiers, one per rank 0 to 3; a soldier
  promoted in this step is not promoted again in the same step.
  Confidence: high.
- **MIL-PROM-4.3** A promoted soldier keeps its place in the building; only
  its profession (rank) changes, and the player's counts of soldiers per
  rank are updated. Confidence: high.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| coin check period | 33 | ticks | high | E-mil-001 |
| promotion period | 301 (counter 300 .. 0) | ticks | high | E-mil-001 |
| promotions per coin | up to 4 (one per rank 0..3) | soldiers | high | E-mil-001 |
| coins used per step | 1 | | high | E-mil-001 |
| ranks | 5 | | high | E-mil-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | barracks with 2 privates present, 1 coin | promotion run | the first private in the list becomes private first class; the other stays private; coins 0 |
| T2 | building with one soldier of each rank 0..4 and 1 coin | promotion run | ranks become 1, 2, 3, 4, 4; coins 0 |
| T3 | building with 3 privates and 3 coins (no new coins arrive) | three runs (301 ticks apart) | after run 1: ranks 1,0,0; run 2: 2,1,0; run 3: 3,2,1; coins 0 |
| T4 | building with only generals present, 2 coins | run | nothing changes; coins stay 2 |
| T5 | coin supply switched off, 2 open coin orders | next coin check | both orders cancelled |
| T6 | stock limit 2, 0 coins in store, no open order, soldiers present below general | coin check | 2 coin orders filed |
| T7 | same with 1 open order | coin check | nothing filed |
| T8 | headquarters with privates and coins | promotion run | no promotion in the headquarters |
| T9 | new game | promotion runs | ticks 300, 601, 902 (none in tick 0) |
| T10 | building with object number 5 | coin checks | ticks 28, 61, 94, ... ((t + 5) mod 33 = 0) |
| T11 | two buildings with open coin orders (first one first in the list), enemy distances 10 and 30, accumulators 0 | three coin servings in a row | after the raises: 0.1 / 0.033333335 -> first served, reset 0; then 0.1 / 0.06666667 -> first; then 0.1 / 0.10000001 -> **second** served (float32 sum is larger, ECO-TIME-5.1), reset 0; the first building's accumulator stays 0.1 |
| T11a | as T11, a distance update between the servings | serving | accumulators are not reset by the update (only the first update sets 0) |
| T12 | new building | coin switch | on |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-mil-001 | code reading | `S2DNG.exe` | tick driver: order of the per-tick updates; military update last but the triggers |
| E-mil-001 | code reading | | global military update; promotion counter 300 |
| E-mil-001 | code reading | | can use coins; promotion step; rank change; rank index |
| E-mil-001 | code reading | | coin check every 33 ticks; no open order; soldiers present |
| E-mil-001 | code reading | | coin order choice: accumulator 1/n, highest wins, reset |
| E-mil-001 | code reading | | warehouse soldier ranks (castle soldier amounts), type flag shared with MIL-PROM-2.1 |
| E-mil-005 | code reading | | only the first distance set resets; raise in float32; strict-greater choice |
| E-eco-013 | code reading | | coin switch default, distance setter resets the accumulator |
| E-eco-013 | code reading | | `k` of the coin check |
| E-eco-013 | code reading | | initial promotion counter; enemy distance |

## Open questions

- None of Q-056 left. Soldier allocation, recruiting and the military
  settings are in `MIL-soldiers.md`.
