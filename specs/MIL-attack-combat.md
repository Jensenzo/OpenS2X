# MIL-ATK: Attacks and combat

Status: draft (read from the binary; no in-game measurement)
Owner question(s): plan step 4.5 (chunk 1); Q-060; Q-061; Q-141; Q-1044; follows `MIL-soldiers.md` (MIL-SOL) and `MIL-coins-promotion.md` (MIL-PROM); conquest is `MIL-conquest.md` (MIL-CON)
Game build: 11757

## Summary

A player attacks an enemy military building by sending soldiers from his own
military buildings near it. Every sending building keeps at least one soldier
at home; the attack strength setting scales how many it may send. When the
order is given, enemy buildings near the target send interceptors that walk out
to meet attackers, scaled by the defender's "intercepting defenders" setting.
Attackers walk to the target and wait on a free field next to it; the target
sends its own soldiers out one at a time to fight them. Two soldiers fight a
duel in rounds of 17 ticks; each round one of them strikes and may take one
health point off the other.

## Rules

### MIL-ATK-1 Who can attack what

- **MIL-ATK-1.1 Sending buildings.** A building B of the attacking player can
  send attackers when all hold:
  1. B is a military building (territory range > 0) and **not** a warehouse-type
     building (so never the headquarters, MIL-SOL-6.4);
  2. B is finished;
  3. no interceptors are currently out on B's behalf (MIL-ATK-3);
  4. B and the target stand on the same land region (the region number used by
     path finding, so not across water);
  5. `d = dist(B, target) < 24` (MAP-GRID-2.1);
  6. a free walking path from the target to B exists and is **shorter than
     2 x d** steps (the free path search of `TRN-free-walk.md`);
  7. `k = trunc((n - 1) x attackStrength x f(d)) > 0`, with `n` the number of B's
     soldiers that are present (MIL-SOL-2.4) and idle (not on an errand),
     `attackStrength` the player's "Attack Strength" setting (MIL-SOL-7.1), and the
     **distance factor** `f(d) = min(36 - d, 24) / 24` (corrected 2026-11-17 day, E-mil-020:
     the factor was missing). So a building up to 12 fields from the target counts fully,
     and from there each field further takes 1/24 off, down to 13/24 at d = 23.
     Number order: `f` is worked out first and kept as a float32: `min(36 - d, 24)` times the
     float32 constant 1/24 (0.0416666679), rounded to float32; `(n - 1)` times
     `attackStrength` (float32) times that factor is then truncated. Every `f` so made is a
     hair above the exact fraction, so truncating the exact rational product gives the same `k`
     at strength 1.0.
  B can then send `k` soldiers. So one soldier always stays home (at strength
  1.0 and d <= 12, `n - 1` go). Confidence: high for the conditions; medium for "region"
  being the water-separated land area; high for the factor (read from the machine code, its
  three constants 24, 12 and 1/24, and measured: T23).
- **MIL-ATK-1.2 Available attackers.** `A(target, P)` = 0 unless at least one
  field of the ring at distance exactly `range(target) + 1` around the target
  (the six-sided ring, MAP-GRID-2.1; fields off the map are skipped) is owned
  by P. Otherwise `A` is the sum of `k` over all of P's buildings (MIL-ATK-1.1),
  in any order. Confidence: high.
- **MIL-ATK-1.3 Targets.** The attack dialog is opened on an enemy building;
  when `A = 0` it shows "Can not attack this building!" and sends no order.
  The order itself checks only that the target building exists. Confidence:
  high for the `A = 0` message. **Which buildings open it (2026-11-16 night, Q-144; E-mil-015, code
  reading and data, high):** a click on a building of a hostile player opens the attack dialog only
  when that building is the **headquarters**, or its type has a **soldier limit above 0** (barracks
  2, guard house 3, watchtower 6, fortress 9) **and it is occupied** (it has had its first soldier,
  TER "Military building occupied!"). Any other enemy building (the lookout tower, which has no
  soldiers, a harbour, a storehouse, a workshop, a military building not yet occupied, any
  construction site) opens nothing: the click clears the selection. Seen in E-trace-031: an occupied
  enemy guard tower opened "ATTACK MENU" with "Cannot attack this building!". Test T1b.
- **MIL-ATK-1.4 Dialog count.** The dialog has four count buttons ("Low",
  "Medium", "High", "Highest number of soldiers") setting a fraction `s` of
  0.25, 0.5, 0.75, 1.0, and a choice "Use weak units first" / "Use strong units
  first". The number put into the order is
  `count = trunc(s x (A - 1) + 1.5)`, i.e. `1 + round(s x (A - 1))` with halves
  rounded up. Button to fraction (Q-061): "Low number of soldiers" 0.25,
  "Medium" 0.5, "High" 0.75, "Highest number of soldiers" 1.0 (each button's
  tooltip is set right after the button in the dialog's setup). Confidence:
  high.
- **MIL-ATK-1.5 The order** "attack" (`attack-building`) carries the target building, `count` and
  the strong-first flag. When executed (in the command phase of the tick,
  ECO-REQ-0.2) `A` is computed again; if `A = 0` nothing happens, otherwise
  `count' = min(count, A)` attackers are launched (MIL-ATK-2) and then the
  interceptors (MIL-ATK-3). Confidence: high.

### MIL-ATK-2 Choosing the attackers

- **MIL-ATK-2.1 Buildings by distance.** Only P's finished non-warehouse
  military buildings with `d < 24` take part. Distance levels are visited in
  increasing order of `d`; at each level the buildings with that `d` are
  visited in the world's military building list order. Each sends
  `min(k, count' - sent so far)` soldiers (`k` of MIL-ATK-1.1; a building that
  fails a condition of 1.1 sends 0). This stops when `count'` are sent.
  Confidence: high.
- **MIL-ATK-2.2 Soldiers of one building.** With "weak first" the ranks are
  taken 0, 1, 2, 3, 4; with "strong first" 4, 3, 2, 1, 0. Within a rank, the
  idle present soldier with the **highest health** goes first (first in the
  building's soldier list on equal health), then the next, until the building's
  share is sent or the rank is exhausted. Confidence: high.
- **MIL-ATK-2.3 What sending does.** The soldier is linked to the target as
  attacker and added to the target's attacker list. One attacker record is made per soldier, in sending order; making it draws
  **one random number** from the shared generator and sets the record's round
  counter to `draw mod 2` (0 or 1, MIL-ATK-6.2). Confidence: high for the draw
  and the link.
  **He stays on his home building's soldier list (corrected 2026-11-17 night, Q-1070; E-mil-018,
  code reading, high).** Sending does not touch the home building's list: the sending loop goes
  through that list from its start again after each soldier sent and passes over the ones already
  sent by its test "in the building, idle" (MIL-SOL-4.4's present test). So the home building still
  counts him (MIL-SOL-4.2's S; it requests no replacement for him) and treats him as **not present**:
  a building that wants fewer soldiers releases him while he is out (MIL-SOL-4.4 step 2). Was: "unlinked
  from its home building's soldier list at once" (medium), which the code does not do.

### MIL-ATK-3 Interceptors ("Intercepting Defenders")

- **MIL-ATK-3.1 Who intercepts.** Right after the attackers are chosen, every
  military building D of the **target's owner** (world list order) that is
  finished, not warehouse-type, on the same land region as the target, with
  `e = dist(D, target) < 24`, and (unless D is the target itself, `e = 0`) a
  free path from the target to D shorter than `2 x e`, provides
  `j = trunc((m - 1) x defendOutside x f(e))` interceptors, `m` = D's present soldiers,
  `defendOutside` = the owner's "Intercepting Defenders" setting, `f(e) = min(36 - e, 24) / 24`
  the same distance factor as the attackers' count (MIL-ATK-1.1 item 7). D sends
  `j - (its interceptors already out)` new ones. The target itself takes part.
  Confidence: high.
  **The distance factor (corrected 2026-11-21 eve, E-mil-032: code reading and an experiment
  build; high):** the count was given without it. D sends nothing unless
  `(m - 1) x defendOutside > 0`; then `min(36 - e, 24)` times the float32 constant 1/24, times
  that product, truncated. As in 1.1, every factor is a hair above the exact fraction, so
  truncating the exact rational `min(36 - e, 24) x (m - 1) x defendOutside / 24` gives the same
  `j`. So a building up to 12 fields from the target sends as before, a farther one fewer: with 6
  soldiers present and the setting 0.5, `j` is 2 up to `e = 16` and 1 from `e = 17` (T39).
- **MIL-ATK-3.2 Pairing.** Each new interceptor is paired with the first
  attacker record (record order) for this target whose soldier still stands on
  its home building's door field and has no meeting field yet. If there is
  none, all interception for this order stops. The interceptor is the soldier
  of D chosen as in MIL-ATK-5.2 among soldiers that are idle (in the building
  or waiting outside as defender) and not already intercepting. Confidence:
  high.
- **MIL-ATK-3.3 Meeting field.** From D's field, a breadth-first search over
  walkable fields, limited to fields closer than 20 to D, finds the field
  with the smallest nonzero distance to the target (first found on ties).
  From that field a second breadth-first search finds the first **free**
  field (MIL-ATK-4.3). Both the attacker and the interceptor get this field as
  their meeting field. Confidence: medium (search order details in the notes).
  **The order of both searches (2026-11-17 day, E-mil-022: code reading, the R6 recording and an
  experiment build; high):** each search tests its start field first and then works first in, first
  out; a field's six neighbours are taken in the ring order of `MAP-grid-geometry.md` MAP-GRID-3.2,
  **directions 5, 0, 1, 2, 3, 4** (one step in direction 4, then around), not 0 to 5. "First found on
  ties" is in that order. So when D is the target itself, the meeting field is the target's
  direction-5 neighbour if that is free (T25). The free-field search of 4.3 visits in the same order.
- **MIL-ATK-3.4** An interceptor that reaches its meeting field, or whose
  duel has ended, waits there ("defense wait"). In each of its updates: if it
  is paired with an attacker (again), it walks to that attacker's waiting
  field and fights (MIL-ATK-5.3); otherwise, while the building it defends
  still has attackers (any soldier with that building as attack target), it
  keeps standing; when it has none, the interceptor drops its errand and
  walks home to D. Confidence: medium-high (Q-061).
  **The pairing only links him (2026-11-17 day, E-mil-025: the war recording and an experiment
  build; high):** an attacker who pairs with a waiting interceptor (5.1, at the door or in his
  waiting check) only links the two; the interceptor's walk starts in the interceptor's own update,
  so in the next tick when his update comes before the attacker's in the tick (the settlers' update
  order), never inside the attacker's update. Test T30.

### MIL-ATK-4 Walking out and lining up

- **MIL-ATK-4.1 Leaving.** A soldier with an attack or interception errand
  leaves its building only when the building's release cooldown is 0 and then
  sets it to 40 (MIL-SOL-8.1). So one building lets out one soldier per 40
  ticks: if the order is executed in tick c and the cooldown is 0, the
  soldiers leave in ticks c + 1, c + 41, c + 81, ... Confidence: medium for
  the exact ticks (a soldier switches to waiting in tick c and walks out on its
  next update).
- **MIL-ATK-4.1a Out through his own flag (2026-11-17 day, E-mil-021: the four-player recording
  E-trace-019 and an experiment build; high).** The walk out is the walk of any settler leaving a
  building: one step from the building's field to **its own flag**, started at progress 0 in the
  tick he leaves. He ends it on the flag's point exactly (the step's excess dropped) and, in that
  arrival tick, starts his next walk at progress 0 from the flag: an attacker to his meeting field
  if his record holds one, else towards the target (4.2); an interceptor to his meeting field
  (MIL-ATK-3.3). The free walk is planned from the flag, not from the building's field, so the
  first field after the flag can lie on the other side of the building. Test T24.
  **The target's own defender too (2026-11-17 day, E-mil-024: code reading and the four-player
  recording; high):** a soldier of the target paired with an attacker (MIL-ATK-5.3) leaves by the
  same step to his building's flag and starts his walk to the attacker's waiting field from the flag,
  in the tick he reaches it. Every soldier leaving his building for an errand (attack, interception,
  defence) does so; the step is the leave itself, before the errand's own walk. Test T27.
- **MIL-ATK-4.2 Walk.** An attacker with a meeting field walks there (free
  walk, `TRN-free-walk.md`); otherwise it walks to the target building's door.
  On arriving at the door, if the target can be taken (MIL-CON-1.1) it goes
  in (MIL-CON); otherwise it picks a waiting field (MIL-ATK-4.3), and if an
  opponent is available (MIL-ATK-5.1) it is paired at once; it walks to the
  waiting field and waits. Confidence: medium. **The door (2026-11-15, Q-1051; E-mil-012, code
  reading):** the walk "to the door" ends on the field of the **target's flag** (the flag the
  building is linked to), not on the building's field; the arrival test and the waiting-field
  search of 4.3 run with the attacker standing there. An attacker whose record already holds a
  field when he has walked out of his own building (a meeting field set by an interception,
  MIL-ATK-3.3) walks straight to that field instead. Confidence: high.
- **MIL-ATK-4.3 Free field.** A waiting field is found by breadth-first search
  from the attacker's own field over walkable fields (the start field is
  tested first; neighbours are taken in the ring order 5, 0, 1, 2, 3, 4, as 3.3). A field is free
  when no other attacker's waiting field and no interceptor's meeting field is
  that field, it is not a road field (field state bit Street, FMT-S2M-4.4) and
  no flag stands on it. The first free field wins. Confidence: medium.
  **In detail (2026-11-15, Q-1051; E-mil-012, code reading):** the search starts on the
  attacker's own field, at the door that is the target's flag field (4.2); the start field is
  tested as it is, a neighbour is **entered only when walkable**: none of the field state bits
  Blocked, Water, ElevationBlocked, MovementBlocked, DoodadBlocked (`FMT-s2m-map.md` FMT-S2M-4.4).
  Every field of a building's footprint carries MovementBlocked (`ECO-building-placement.md`
  ECO-PLACE-1.5), so **no building's field, the target's or any other, is ever reached or chosen**;
  road fields are walked through (they are walkable) but are not free. The free test itself looks
  at: the other records' waiting and meeting fields (its own record left out), the Street bit, and
  the flag layer only (not the buildings layer). With no free field found the attacker gets none
  (the field stays unset). Confidence: high for the walk test, the start and what the free test
  reads; medium for the ring order of the neighbours (the lattice walk of the map's search).
- **MIL-ATK-4.4 Waiting.** A waiting attacker checks each tick, in this order:
  its target is gone or no longer an attack target: it walks home; the target
  can be taken (MIL-CON-1.1): it walks to the door; an opponent is available
  (MIL-ATK-5.1): it is paired with it and walks to its waiting field (it may
  already stand there). Confidence: medium.
  **Walking home, and a home that is gone (2026-11-16 night and day, Q-142; E-mil-016, E-mil-017,
  code reading, high).** "Walks home" is a free walk to the flag of **his own military building**
  (the one that sent him), after which he goes in as every settler does. When that building no
  longer exists, the walk does not start. Every tick, before his state is handled, a soldier
  outside a building checks his four links: his home building, his attack target, the opponent of
  the fight he may be in, and (an interceptor) the building he defends (corrected 2026-11-17 day,
  E-mil-019: not a second link of the fight). When none of them exists any more, he is **sent home as a
  settler without a building** (`TRN-free-walk.md` TRN-FREE-6.4: the nearest accepting warehouse
  over the roads, and so on), in that tick. So an attacker whose target and home are both gone
  ends in a warehouse. (How a home building that now wants fewer soldiers takes a returning one
  was not read.)
  **When he is home (2026-11-17 day, E-mil-025: code reading, the war recording and an experiment
  build; high):** the walk home (any soldier's: an attacker's, a target's defender's after his duel)
  ends with the step into the building in one update; his return ends in his **next** update, and
  only from then is he idle in the building (a candidate of 5.1, present). A reinforcement walking in
  from a warehouse (5.1) is not this walk. Test T33.
  **Not at a headquarters (2026-11-19 night, E-mil-026: code reading and the whole war recording;
  high):** the return's last part is the walk in, started in the update the walk home ends (its
  result in that update is not counted). At a building that is no warehouse it has nothing more to
  do and reports done in the next update: the one further update above. At a castle (a warehouse;
  its soldiers stand on its door field, `MIL-soldiers.md` MIL-SOL-6.5) it orders, in that same
  update, the walk on to the door field at progress 0 (no standing tick), and it reports done in the
  update in which that walk ends: he is idle then, with **no** further update. R6: all 14 such returns to player 2's castle
  (e.g. soldier 14311: step in from counter 30769, on to the door field from 30797, idle at 30820);
  all 61 returns to other military buildings have the one further update.

### MIL-ATK-5 Defenders coming out of the target

- **MIL-ATK-5.1 Opponent.** For a waiting attacker the opponent is: first an
  unpaired interceptor that belongs to the target itself; otherwise, if the
  target's owner is still an enemy, a soldier of the target chosen by
  MIL-ATK-5.2 among those that are idle and not on an errand. None: no
  opponent. Confidence: high. **Idle means home with the walk there ended** (2026-11-14, Q-1044 c;
  E-mil-010): a soldier still walking to the target from a warehouse (a reinforcement) is not idle
  and is never chosen; he becomes a candidate in the update in which his walk ends at the building.
  Nothing else about being inside is tested (the choice has no presence test of its own), and
  nothing lets a soldier be sent out before his walk in has ended. Confidence: high (code).
  **Corrected 2026-11-19 night (E-mil-029: the whole war recording; high for the ticks):** "his
  walk ends" is the end of his walk job, which comes one update after the movement's: the step onto
  the building's field ends in one update (he stands there, still walking in), and he is idle, a
  candidate, from his **next** update, as after a walk home (4.4, T33). R6: all 181 reinforcements
  stand one update on their building's field first. So an attacker updated before him in each tick
  pairs with him two ticks after his step ended. Test T35.
- **MIL-ATK-5.2 Which defender.** With defense type 1 ("prefer strong") the
  candidate with the **highest rank**, on equal rank the higher health, on
  equal both the first in the soldier list; with the other setting the
  **lowest rank**, then the higher health. Confidence: high.
- **MIL-ATK-5.3** The paired defender leaves the building under the cooldown
  rule (MIL-ATK-4.1), through his own flag (4.1a), and walks (free walk) to the **waiting field of the
  attacker it is paired with** (the field stored in that attacker's record,
  MIL-ATK-4.3), then stands until its movement has stopped there. Its duel
  starts when both stand (MIL-ATK-6.1). Confidence: high for the target
  field (Q-061).
  **When his arrival counts (2026-11-17 day, E-mil-025: code reading, the war recording and an
  experiment build; high):** the defender's walk ends on the waiting field's point in one update;
  his arrival (6.1: the message, and the pair counted as fighting) comes in his **next** update,
  when his job sees the movement stopped. So when he is the second of the pair to arrive, the duel's
  first round (6.3, its two draws) runs in the tick after his walk ended. The attacker's own arrival,
  at the end of his offset step (6.1a), counts in the update in which the step ends. Test T28.
  **When he leaves: held until his attacker is there (2026-11-19 night, E-mil-026: code reading and
  the war recording; high).** A soldier of the target paired with an
  attacker by 5.1 (the target's own soldier, not an interceptor) has his opponent from the pairing
  on, but stays idle in the building. In each of his updates he turns to **wait-for-leave only
  when** his attacker
  1. stands on the field of the target's flag (the attacker paired on reaching the door, 4.2), or
  2. **has arrived**: the attacker's arrival of 6.1a, at the end of his offset step (it stays set
     while the pair holds).

  Otherwise he stays idle inside the building (no candidate of 5.1 any more, as he has an opponent)
  and asks again in his next update. In the update
  after his wait-for-leave he leaves under the release cooldown (4.1; through his own flag, 4.1a).
  So a defender paired with an attacker already **waiting** (4.4: the pairing, the draw and the
  offset step in the attacker's next update) leaves only after that step: wait-for-leave in his
  first update after the step ended, the leave in the next. An attacker paired at the door is still
  on the flag field in the defender's next update, so that defender turns at once (in the same tick
  when he is updated after the attacker). If the pair breaks while he is held, he simply stays idle.
  A soldier whose home link is gone, an interceptor (MIL-ATK-3) and a soldier sent to attack turn to
  wait-for-leave at once, as before. Test T34.
- **MIL-ATK-5.4 Headquarters.** The headquarters' defenders are its soldiers
  outside the stock (MIL-SOL-6.1); when it has attackers and no soldier is
  present it brings out one per rank from the stock (MIL-SOL-6.3).
  Confidence: high (rule from MIL-SOL).

### MIL-ATK-6 The duel

- **MIL-ATK-6.1 Start.** A duel starts when both soldiers of a pair have
  arrived at the shared field. When either arrives, the target's owner gets
  "You are under attack!" at the target's field, unless the newest message
  with this text within one field still has at least 540 of its 600 timer
  units (it is then refreshed to 600 instead). The timer is a message timer
  (`UI-ingame.md` UI-GAME-11.1, 11.2), not a tick count: it loses the last
  frame's duration in seconds each tick, so with the fixed 1/60 s per tick of
  11.2 the refresh window is 3600 ticks and the message lasts 36 000 ticks
  like every other message (corrected 2026-10-18, Q-222: the earlier "60
  ticks" and "600 ticks" readings are withdrawn). Confidence: high (E-gfx-025, the constant 540.0 read in the code).
- **MIL-ATK-6.1a Where the two stand (2026-11-14, Q-1044; E-mil-010, code reading and two saves
  of the original).** The "shared field" of 6.1 is the attacker's **waiting field** (the field of
  his attack record, MIL-ATK-4.3), and the two do not stand on the same point of it:
  - the **defender** (the target's own soldier, MIL-ATK-5.3, or an interceptor, MIL-ATK-3.4) walks
    to the waiting field and stands on its lattice point;
  - the **attacker**, when he begins the step in which he waits for his opponent at his waiting
    field (after reaching it, also when he is paired again after a duel), and only he, takes **one
    draw** from the game's random number generator, `d = draw mod 6`, and walks **one lattice step
    from his field's lattice point in lattice direction `d`** (a short walk on the lattice, as a
    deposit's offset in `TRN-free-walk.md`; 12 ticks on level ground, TRN-FREE timing). He counts as
    arrived (6.1) only when that step has ended, so his arrival comes after the step's duration.
    When a duel ends without him being hit dead and he is paired again, he first goes back to the
    field's lattice point and then makes a new step with a new draw.
  - **Unpaired, and leaving the offset point (2026-11-15, Q-1050; E-mil-011, code reading).** (a)
    The step and its draw belong to the attacker's **wait job, which starts only with an opponent**.
    An attacker for whom no opponent can be paired when he arrives at the attack walks to his
    waiting field by a plain walk, stands on its lattice point, takes no draw and makes no step,
    and waits there, looking for an opponent in each of his updates. When he is paired there, his
    wait job starts: a walk to his waiting field (he already stands on its point, so it is over at
    once), and in the job's next update (the tick after the pairing; measured, T26) the draw and the step.
    An attacker paired before he reaches the field makes the step on arrival, as above.
    **Which attacker is "paired before he reaches the field" (2026-11-17 day, E-mil-024: code
    reading, the war recording E-trace-019 and an experiment build; high):** only one who walks to
    his waiting field **inside his wait job**, that is, one paired at the target's door (4.2), whose
    wait job starts there and walks him to the field. An attacker who walks to the **meeting field an
    interception gave him** (3.3, 4.2) is not in his wait job, even though an interceptor is already
    linked to him: on arriving he becomes a waiting attacker and does nothing more in that update;
    in his next update he makes the checks of 4.4 (target gone: home; target can be taken: the door;
    an opponent: paired, and his wait job starts, its walk over at once); in the job's next update
    the draw and the step. So his draw comes two updates after his arrival, and the tick of the draw
    after a pairing while standing is measured. Test T26. (b) He
    stays on the offset point only while he has an opponent **and** is marked arrived. In his first
    update without an opponent (the duel ended with the defender dead, or the pair was undone) both
    arrived marks of his record are cleared and he **walks the one lattice step back to the field's
    point** (12 ticks on level ground); only when that step has ended does he choose again: a new
    opponent (a new wait job: walk, draw, step), the door of a target that can be taken, or home
    when the target is gone. He never starts for the door or for home from the offset point.
    **The step back's end (2026-11-21 day, E-mil-030: the war recording and an experiment build;
    high).** In the update in which the step back ends he only becomes a waiting attacker again,
    standing on the field's point, and does nothing more in that update; the choice of (b)
    (an opponent, the door, home) is made in his next update. So a door walk after a step back
    starts at progress 0 in the update after the step's end and moves from the one after. Test T36.
    Confidence: high (the wait job's start and update and the soldier's waiting state, code),
    and for the tick of the draw after a pairing while standing (measured T26, 2026-11-17 day).

  The defender makes no such step and takes no draw. A soldier's field as the save stores it
  (`FMT-savegame.md` FMT-SAV-3.5h, the truncating field of his sample, MAP-GRID-4.2) can therefore
  be the neighbour of the waiting field for the attacker (**when** (2026-11-17 day, E-mil-025, the war
  recording; high): during the step his field stays the waiting field; in the update the step ends
  his field becomes the truncating field of the sample he stands on, and stays so while he stands
  there; T29; **the step back (2026-11-21 day, E-mil-030; high):** during the step back to the
  field's point his field is likewise the field of the sample he steps from, the truncating field of
  the offset sample, and it becomes the waiting field in the update that step ends; in short, during
  either step his field is the truncating field of the sample the step started from, after it the
  truncating field of the sample he stands on; T37): in the original's two fight saves the
  three attackers stood one lattice step off their waiting fields' points (offsets (x, y) of
  (-1, +1), (-1, +1) and (0, +1); the third attacker, re-paired, (+1, 0) in the later save), their
  stored fields the waiting field or its west neighbour, and every defender on the point. **For an interceptor** the
  duel too is fought on the attacker's waiting field: the interceptor's meeting field (MIL-ATK-3.3)
  is only where he walks first and waits; it becomes his first attacker's waiting field when they
  are paired, and once paired with any attacker he walks to that attacker's waiting field. When his
  duel ends he waits where he stands (defense wait, 3.4), not back at the meeting field; the
  meeting field stays in his record unchanged. Confidence: high for the step, the draw, who makes
  it and the field (code; the three save pairs agree); medium for the moment of the draw within
  the tick (the step's start was read, its caller's order among the tick's other draws was not).
- **MIL-ATK-6.2 Who strikes.** Each pair has a round counter, set when the
  attacker was sent (MIL-ATK-2.3) to 0 or 1 at random. In a round with an even
  counter the **attacker** strikes, with an odd counter the **defender**; the
  counter rises by 1 every round. So strikes alternate and the first striker
  is a fair coin flip made at the attack order. Confidence: high.
- **MIL-ATK-6.3 A round.** In the military update of every tick (after the
  settlers' updates, ECO-REQ-0.2), for each fighting pair in list order: if
  both soldiers have health > 0 and neither is still in a fight animation:
  1. if the previous round scored a hit, the soldier who strikes now (the one
     who was hit) loses 1 health; if that kills it (health <= 0), the round
     ends here;
  2. both soldiers start a fight animation of 16.7 ticks (`mi_setFightingTime`);
  3. two numbers are drawn, `r1 = draw mod 7`, then `r2 = draw mod 7`;
  4. hit test of MIL-SOL-1.3: `a = max(2, rankS + 1)`, `h = a div 2`,
     `b = max(2, rankO + 1 - h)` (S striker, O other); hit when `r1 <= a` and
     `r2 > b`. A hit is remembered and takes effect at the next round's start
     (step 1);
  5. the counter rises by 1.
  Confidence: high.
- **MIL-ATK-6.3b The list order (added 2026-11-21 pm, E-mil-031; recording and an experiment
  build).** The "list order" of 6.3 is the order in which the pairs were **made** (an attacker given
  his defender or interceptor), the oldest pair first; not the order the attackers were sent in. It
  decides which pair gets which draws when several pairs fight a round in one tick. Test T38.
  Confidence: medium-high (one measured tick with two rounds whose hit results the order swaps; the
  original's own list is not read in the code).
- **MIL-ATK-6.4 Round length.** A fight animation counts down by 1 per tick in
  the soldier's own update and ends when it reaches 0 or below; 16.7 takes 17
  updates. Since the settlers' updates run before the military update, a round
  started in tick t is followed by the next in tick t + 17. Confidence: high.
- **MIL-ATK-6.5 Hit chance.** `P(hit) = (a + 1) / 7 x (6 - b) / 7`:

  | striker \ other | 0 | 1 | 2 | 3 | 4 |
  |---|---|---|---|---|---|
  | 0 (a 2, h 1) | 12/49 | 12/49 | 12/49 | 9/49 | 6/49 |
  | 1 (a 2, h 1) | 12/49 | 12/49 | 12/49 | 9/49 | 6/49 |
  | 2 (a 3, h 1) | 16/49 | 16/49 | 16/49 | 12/49 | 8/49 |
  | 3 (a 4, h 2) | 20/49 | 20/49 | 20/49 | 20/49 | 15/49 |
  | 4 (a 5, h 2) | 24/49 | 24/49 | 24/49 | 24/49 | 18/49 |

  Confidence: high (follows from 6.3).
- **MIL-ATK-6.6 Damage and death.** Every hit takes exactly 1 health
  (MIL-SOL-1.1 maxima 3..7). A soldier at 0 dies in its next own update
  (tick t + 18 for a hit applied in tick t + 17): its links are released, the
  pair is dissolved. **He dies as every settler does** (2026-11-17 day, E-mil-025, the war recording; high):
  standing where he is, he makes the settler dying of `TRN-carriers-transport.md` TRN-CARRY-7.1
  (28 ticks), one more update, and is then removed; meanwhile he is on the map, in no pair. Test T31.
  14 ticks into his dying the skeleton doodad is put where he lies and takes an object number
  (TRN-CARRY-7.1, corrected 2026-11-19 night, E-mil-027). Test T31a. Survivors (Q-061):
  - an **attacker** is back in its waiting state at its waiting field and
    follows MIL-ATK-4.4 in its next update (home if the target is gone,
    door if takeable, next opponent if one is available, else it waits);
  - an **interceptor** goes to "defense wait" at its spot (MIL-ATK-3.4);
  - a **defender of the target** (not an interceptor) walks back into its
    building; when it arrives, if it has meanwhile been paired with another
    attacker it walks straight out again to that attacker's waiting field
    (MIL-ATK-5.3), else it is idle inside (present, heals).
  **When and how the defending side moves on (2026-11-17 day, E-mil-025: code reading, the war
  recording and an experiment build; high):** a defender or interceptor whose opponent dies (in the
  dead one's update, tick t) notices it in his **first update after the dead one's** (his fight job
  sees the pair gone and ends) and acts in his update after that: the interceptor turns to defense
  wait, the defender of the target starts home. So he acts in t + 2 when he comes **before** the dead
  one in the settlers' order (a lower number in the soldiers' list, ECO-REQ-0.2), in t + 1 when he
  comes **after** him (corrected 2026-11-19 night, E-mil-026: the whole war recording, 43 survivors
  before the dead one all at t + 2, 26 after him all at t + 1). The defender's way home is the walk home of 4.4: a free walk to **his
  building's flag**, then the step in, as every settler (not a walk straight onto the building's
  field). Test T32.
  Health is not restored after a duel; only present soldiers heal
  (MIL-SOL-1.2). Confidence: high for the damage; medium-high for the
  survivors' next steps.
- **MIL-ATK-6.7** A duel never ends otherwise (no retreat, no time limit).
  Confidence: medium.

### MIL-ATK-7 Soldier leftovers (Q-060, Q-141)

- **MIL-ATK-7.1 Health after a promotion.** Promotion only changes the rank;
  health is kept. The higher maximum then applies to healing. Confidence: high.
- **MIL-ATK-7.2 Health in the stock.** A warehouse keeps soldiers only as
  counts per rank; a soldier that leaves the stock is made new with its rank's
  maximum health, and a soldier that enters a warehouse loses its health
  value. Confidence: medium-high (the health is set from the rank when the
  soldier object is made; the stock holds no per-soldier data).
- **MIL-ATK-7.3 Slider range.** Every settings slider value is held in 0..1
  (lower values are set to 0, higher to 1) and, when the slider has a step
  count N > 1, snapped to the nearest of N evenly spaced values. The sliders
  of the military settings dialog (attack strength, intercepting defenders
  and the others built by the same helper) have **no step count: they are
  continuous** in 0..1 (Q-061). The graphics options use stepped sliders.
  Confidence: high.
- **MIL-ATK-7.4 Computer players** set attack strength and intercepting
  defenders from their AI profile (`ai.lua`: weak 0.51 / 0.5, normal 0.75 /
  1.0, strong 1.0 / 0.5), adjust near/medium allocation and the defense type
  by military mode, and never change the recruit count (stays 10).
  Confidence: high for the recruit count and the profile values; the AI's
  allocation logic is out of scope.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| attack / interception reach | d < 24 | fields | high | E-mil-003 |
| distance factor of k and j | min(36 - d, 24) / 24 (float32, 1/24 = 0.0416666679) | | high | E-mil-020, E-mil-032 |
| path limit | < 2 x d | steps | high | E-mil-003 |
| target ring | range + 1 | fields | high | E-mil-003 |
| count fractions | 0.25, 0.5, 0.75, 1.0 | | high | E-mil-003 |
| fighting time | 16.7 (17 ticks per round) | ticks | high | E-mil-003 |
| hit draws | 2 x (draw mod 7) | | high | E-mil-003 |
| first striker | draw mod 2 at order | | high | E-mil-003 |
| meeting search limit | distance < 20 from the interceptor's building | fields | medium | E-mil-003 |
| leave cooldown | 40 | ticks | high | E-mil-002 |
| message refresh | 540 of 600 (60 ticks) | ticks | medium | E-mil-003 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | P's guard house G with 3 present idle soldiers at d 10 from enemy barracks T; attack strength 1.0; P owns a field on T's ring | available | `A = 2` |
| T1b | 1.3 (Q-144): enemy buildings: its headquarters; an occupied barracks; a finished guard house no soldier has entered yet; a lookout tower; a harbour; a fortress site | a click on each | the attack dialog opens for the headquarters and the occupied barracks only; the other four open nothing and the selection is cleared |
| T2 | as T1, strength 0.3 | available | `trunc(2 x 0.3) = 0`: "Can not attack this building!" |
| T3 | `A = 7` | dialog counts | Low `trunc(0.25 x 6 + 1.5) = 3`, Medium 4, High 6, Highest 7 |
| T4 | buildings X (d 8, k 2), Y (d 5, k 1), Z (d 8, k 3), list order X, Y, Z; count 4 | launch | Y sends 1, X 2, Z 1 |
| T5 | fortress with present idle soldiers ranks 0 (health 3), 2 (5), 2 (2), 4 (7), 0 (1); send 3, weak first | choice | rank 0 health 3, rank 0 health 1, rank 2 health 5 |
| T6 | as T5, strong first | choice | rank 4, rank 2 health 5, rank 2 health 2 |
| T7 | target T with 4 present, defend outside 1.0; P sends 2 | interceptors | T itself sends 3 but only 2 attackers can be paired: 2 interceptors |
| T8 | duel private (attacker) vs private, record counter 0, rounds start t | rounds | t: attacker strikes; t + 17: defender; t + 34: attacker; each with P(hit) 12/49 |
| T9 | general strikes private | hit chance | `a 5, b 2`: 6/7 x 4/7 = 24/49 |
| T10 | private strikes general | hit chance | `a 2, h 1, b 4`: 3/7 x 2/7 = 6/49 |
| T11 | round in tick t is a hit on a soldier with health 1 | next ticks | t + 17: health 0, no new round; t + 18: soldier dies |
| T12 | soldier health 2 of max 3 promoted to rank 1 | promotion | health 2, max 4; heals to 4 in 2 x 602 ticks while present |
| T13 | 3 soldiers of one building sent in tick c, cooldown 0 | leaving | out in ticks c + 1, c + 41, c + 81 |
| T14 | dialog, A = 5 | press "Medium number of soldiers" | fraction 0.5, count `trunc(0.5 x 4 + 1.5) = 3` |
| T15 | military settings, drag attack strength to 0.437 | value | 0.437 kept (no snapping); -0.2 -> 0, 1.3 -> 1 |
| T16 | target's defender paired with attacker X whose waiting field is W | defender comes out | walks to W, not to X's current field |
| T17 | interceptor wins its duel; its building still has 2 attackers walking towards it, none free to pair | after the duel | stands at its spot; paired again -> fights; once the building has no attackers left (all dead or gone home), walks home |
| T18 | target's defender wins; another attacker gets paired with it while it walks home | arrival at home | walks out again to that attacker's waiting field |
| T19 | 6.1a: attacker A reaches his waiting field W on level ground and is paired with a defender; the next draw is r | A's position, A's arrival, the defender's position | A walks one lattice step from W's lattice point in direction r mod 6 (12 ticks) and counts as arrived when it ends; the defender stands on W's lattice point and makes no draw; the duel's first round comes when both have arrived |
| T20 | 6.1a, interceptor I with meeting field M, paired with a waiting attacker whose waiting field is W != M | where the duel is fought | on W (I walks to W's lattice point); after the duel I stands where he is; M unchanged in I's record |
| T21 | 6.1a (Q-1050): attacker A arrives at his waiting field W when no defender can be paired; later defender D is paired with A at tick t; D dies in the duel and the target can be taken | A's position and draws | A walks to W's lattice point and draws nothing while unpaired; at t + 1 one draw and the one-step offset walk (12 ticks); after D's death A walks the step back to W's point (12 ticks) and only then starts for the door |
| T22 | 4.2, 4.3 (Q-1051): attacker A sent against guard house G (field g, flag field f), no interception, no opponent when he arrives | where A walks and the waiting field | A's walk ends on f; the search starts on f (a flag: not free), never enters g or any other building's footprint (MovementBlocked) and passes over road fields without taking them; the waiting field is the first walkable field without a flag, road or other waiting / meeting field in the search's order |
| T23 | 1.1 (the distance factor), 1.2, 2.1, 2.3: E-trace-019 (R6, Keyhole `MP_4P_Fight`, players 1 to 3 Hard, attack strength 1.0), player 2's first attack, against player 3's guard house on (31, 21), in counter 12168 (trace tick 12169); its buildings in range: the guard house on (42, 23) (d 12, 2 soldiers present and idle), the tower on (47, 19) (d 17, 6), the guard house on (51, 17) (d 22, 3, being upgraded) | k of each; the attackers sent; the draws | k = trunc(1 x 1.0 x 24/24) = 1, trunc(5 x 1.0 x 19/24) = trunc(3.96) = 3, trunc(2 x 1.0 x 14/24) = trunc(1.17) = 1, so 5 available and 5 sent: 1 from (42, 23), then the first 3 of (47, 19)'s list, then 1 of (51, 17)'s (by distance, MIL-ATK-2.1), five attack records and so five draws in that tick (recorded: 8 draws and 8 attackers, 1 + 5 + 2, without the factor). The upgrading building sends: it counts as finished (its own construction is done; the upgrade is separate). Experiment build (the factor; AI-MIL-4.4a on): the war's generator first differs at 12171 instead of 12169 |
| T24 | 4.1a: E-trace-019 (R6), player 2's first attack on player 3's guard house (31, 21), counter 12168 (trace tick 12169); the interceptor 16370 of (31, 21) with meeting field (32, 21); the attackers 16579 of (47, 19), 17868 of (51, 17), 18180 of (42, 23) | the walks out | each leaves with a walk at progress 0 in trace tick 12171 to his own building's flag: 16370 to (32, 20), on its point at 12194, then from that tick to (32, 21) (one step, first moving in 12195); 16579 to (48, 18) (12194), then west over (47, 18) (12217); 17868 to (52, 16) (12194), then over (51, 16) (12217); 18180 to (43, 22) (a climb, 12199), then over (42, 22). The second and third soldiers of (47, 19) leave 40 and 80 ticks later the same way (16592: (48, 18) at 12234). Walking straight from the building's field (16370 east to (32, 21) from 12172) differs from 12172 on, and its free walk from the building's field draws 13 numbers in 12171 that the original does not draw. Experiment build (with T23, T47b and AI-MIL-4.4a): the war's settlers (animation aside) first differ at 12188 instead of 12172, its generator at 12199 instead of 12171 |
| T25 | 3.3, 4.3 (the order): E-trace-019 (R6), player 2's first attack on player 3's guard house (31, 21) (an odd row), counter 12168; (31, 21) defends itself (D = the target) with 16370 | the meeting field | the search from (31, 21): the start (distance 0), then its neighbours in the order 5, 0, 1, 2, 3, 4: (32, 22) first at distance 1, kept; it is free, so the meeting field is (32, 22) for 16370 and the attacker paired with him (18180, waiting there from 12484); recorded: 16370 walks out to his flag (32, 20), on over (32, 21) without stopping, and waits on (32, 22) from 12239 (state 39). In the order 0 .. 5 the field would be (32, 21) and he would stop there in 12217. Experiment build (the tick-exact branch's head with 4.1a and this order): the war's settlers (animation aside) first differ at 12227 instead of 12217, its generator at 12476 instead of 12199 (the attacker's free walk to (32, 22) draws as the original's) |
| T26 | 6.1a (the interception's attacker on his meeting field): E-trace-019 (R6, the war), player 2's attacker 18180 against player 3's (31, 21), whose interceptor 16370 waits on the meeting field (32, 22) from counter 12238; 18180 walks there and his walk ends on (32, 22) at counter 12484 | his states, position and the generator from 12484 | 12484: arrived, waiting (standing, no draw); 12485: the waiting check pairs him with 16370 and his wait job starts (standing); 12486: one draw and his step off the point begins (recorded: the walk at progress 0 in trace tick 12487, the generator's one draw of that tick; 16370 turns to defending in counter 12486, after his own next look at his link). Making the draw and step on arrival (12484) puts the war's settlers apart from trace tick 12486 and its generator from 12485; with the rule (experiment build, the tick-exact branch's head, the attacker paired at the door still stepping on arrival) settlers 12486 -> 12655, buildings 12705 -> 12943, generator 12485 -> 12668, every Storm Coast pair unchanged. (Holding the door-paired attacker 16579 as well puts the generator apart at 12643: he reaches his waiting field (32, 21) at trace tick 12643 and walks on in that tick.) |
| T27 | 4.1a, 5.3 (the target's defender): E-trace-019 (R6, the war), player 3's (31, 21) (flag (32, 20)); its soldier 16378 paired with attacker 16579 (waiting field (32, 21)), waiting to leave in counter 12620 | 16378's position | leaves in counter 12621: the step from (31, 21) to the flag starts at progress 0 (recorded: trace tick 12622, the building's field point); on the flag's point exactly at trace tick 12645, his defence walk starting there in that tick; on (32, 21) at trace tick 12668, where he stands |
| T28 | 5.3, 6.1 (the defender's arrival): E-trace-019 (R6, the war), defender 16378 of (31, 21) walking to attacker 16579's waiting field (32, 21); 16579 already stands off its point (his step ended at counter 12654) | the duel's first round | 16378's walk ends on the point of (32, 21) in counter 12667 (recorded: trace tick 12668, standing); his arrival counts in counter 12668 and the first round's two draws are made then (recorded: the fight animations and the generator's two draws at trace tick 12669). Counting the arrival in the update the walk ends starts the duel a tick early and the war's generator differs from trace tick 12668; with the rule (experiment build) from 12705 |
| T29 | 6.1a (the field after the step): E-trace-019, attacker 16579 steps from the point of (32, 21) (sample (130 000, 84 000) in the recording's units of 1/4000 field) to (129 000, 85 000), counters 12643 .. 12654; attacker 18180 steps from the point of (32, 22) (128 000, 88 000) to (129 000, 88 000), counters 12486 .. 12500 | each one's field | 16579: (32, 21) during the step, (31, 21) from the update the step ends (trace tick 12655); 18180: (32, 22) throughout (the end sample truncates to his waiting field) |
| T30 | 3.4 (the interceptor's walk after a pairing): E-trace-019 (R6, the war), interceptor 16370 waiting on (32, 22) after his duel; attacker 17868 (in each tick updated after 16370, as in the experiment build's settler order) reaches the target's door (32, 20) in counter 12704 and pairs with him | 16370's walk | starts at progress 0 in counter 12705 (recorded: trace tick 12706; 17868 walks on to his waiting field from trace tick 12705). Starting it inside the pairing (counter 12704) puts the war's generator apart from trace tick 12705; with the rule (experiment build) from 12950 |
| T31 | 6.6 (the dead soldier): E-trace-019, attacker 18180 loses his last health in the duel with 16370 on (32, 22) | 18180's rows | standing on (129 000, 88 000), dying from trace tick 12689 to 12716, one more row at 12717, gone from 12718 |
| T31a | 6.6, TRN-CARRY-7.1 (the skeleton doodad): E-trace-019 (R6, the war), soldiers killed in duels: 18180 in counter 12688 at (129 000, 88 000), 16667 in 12845 at (132 000, 80 000), 16370 in 12893 at (126 000, 76 000), 16579 in 12924 at (129 000, 85 000) | the objects made | a `doodad_deadsettler` doodad on the sample of each: 18513 on (129, 88) made in counter 12702, 18546 on (132, 80) in 12859, 18559 on (126, 76) in 12907, 18564 on (129, 85) in 12938 (14 ticks after each death; the counter table's next number rises there). Over the whole recording: 170 of 208 deaths made one, each at death + 14 on the settler's own sample; the other 32 died on a sample whose doodad was still there (none made), the last 6 past the recording's object reads; dying carriers, workers and builders made theirs too. Without the doodad, request 18514 of counter 12704 (the soldier request for 18180's home) takes 18513 and every later object number is one lower (two after the second death, ...): the object numbers apart from counter 12704, and with them the carriers' work grid (`TRN-carriers-transport.md` TRN-CARRY-2.1); with the doodads: not yet measured (no experiment build on 2026-11-19 night) |
| T32 | 6.6 (the survivors' next step): E-trace-019 (R6, the war). (a) Attacker 16579 dies in counter 12924 in his duel with defender 16378 on (32, 21); (b) attacker 18180 dies in counter 12688 in his duel with interceptor 16370 on (32, 22) | the survivors' next step | (a) 16378 stands through counters 12924 and 12925 and starts home in 12926 (recorded: walk at progress 0 at trace tick 12927), towards his building's flag (32, 20) (recorded: (129 909, 83 819) at 12928, north-west), not west onto (31, 21); (b) 16370 turns to defense wait in counter 12690 (recorded: from trace tick 12691). Starting home one update after the death, straight onto the building, puts the war's generator apart from trace tick 12950; with the rule (experiment build) from 12974. (c) The survivor after the dead one in the order: attacker 16592 dies in counter 14959 in his duel with defender 18867 (a higher number): 18867 starts home in counter 14960 (state 16 from there), one update after the death |
| T33 | 4.4, 5.1 (home after the walk home): E-trace-019 (R6, the war), defender 16378 walking home to (31, 21) after his duel; attacker 16592 waits on (33, 20) for an opponent of the target (31, 21) | 16378's state and 16592's pairing | 16378's step into the building ends in counter 12972 (recorded: standing in it at trace tick 12973, still returning); he is idle from counter 12973 (recorded: from trace tick 12974); 16592 is paired with him in counter 12973 and makes his draw and step in 12974 (recorded: walk at progress 0 at trace tick 12975). Making him idle in the update his walk ends pairs him a tick early and puts the war's generator apart from trace tick 12974; with the rule (experiment build) from 13023, every Storm Coast pair unchanged |
| T33a | 4.4 (home at a castle): E-trace-019 (R6, the war), player 2's castle on (52, 24), flag (52, 23), door field (51, 25); soldier 14311 walking home after a duel | his rows (counters) | on the step (52, 23) -> (52, 24) from counter 30769 at progress 0; the walk (52, 24) -> (51, 25) at progress 0 in counter 30797 (no standing tick between); on (51, 25) and idle (state 12) in counter 30820, the update that walk ends |
| T34 | 5.3 (the held defender): E-trace-019 (R6, the war), the guard on (31, 21), flag (32, 20). (a) Defender 16378, idle again from counter 12973 (T33), is paired in counter 12973 by attacker 16592 waiting on (33, 20); 16592's offset step starts in 12974 (walk at progress 0 at trace tick 12975) and ends in counter 12986 (standing at (131 000, 79 000) from trace tick 12987); 16592 is updated after 16378 in each tick. (b) The same defender paired at counter 12619 by attacker 16579 on the flag field (32, 20). (c) Defender 16667 paired at counter 12659 by attacker 16592 on the flag field, 16592 updated first; the building's release cooldown set by 16378's leave in counter 12621 | each defender's turn to wait-for-leave and his leave | (a) idle through counter 12986 (recorded state 12 to trace tick 12987), wait-for-leave in 12987 (trace tick 12988), the step out to (32, 20) at progress 0 in 12988 (trace tick 12989). (b) wait-for-leave in 12620, out in 12621 (trace ticks 12621, 12622). (c) wait-for-leave in 12659, held by the cooldown in 12660, out in 12661 (trace ticks 12660 .. 12662). Leaving at the pairing in (a) sends him out at counter 12975: his duel's first draws come at trace tick 13023 instead of the original's later round, the war's generator apart from 13023; with the rule: not yet measured (no experiment build on 2026-11-19 night) |
| T35 | 5.1 (a reinforcement becomes a candidate): E-trace-019 (R6, the war), soldier 18545 of player 3 sent from a warehouse to the guard on (31, 21) (state 4, walking in), attacker 16600 waiting on (32, 19) (a lower number: updated before 18545 in each tick) | 18545's state and 16600's pairing (counters) | the step (32, 20) -> (31, 21) from counter 13098 ends in 13121 (standing on (31, 21), still walking in); idle from 13122; 16600 paired with him in 13123 (state 18), his offset step from 13124 to 13136; 18545 turns to wait-for-leave in 13136 (T34, updated after 16600) and steps out in 13137. Idle in the update his step ends, he would be paired in 13122; with the rule: not yet measured (no experiment build on 2026-11-19 night) |
| T36 | 6.1a (b), the step back's end: E-trace-019 (R6, the war), player 2's attacker 17868 stepping back from his offset sample (127, 76) to the point (126, 76) of his waiting field (31, 19) (counters 13420 .. 13431), the target then takeable | his state and walk | the step ends in counter 13431 (standing on (126, 76), waiting attacker); his door walk (31, 19) -> (32, 20) at progress 0 in counter 13432, +0.027 per tick from 13433 (trace tick 13440: progress 0.189). Choosing in the step's end update starts the walk a tick early: war settlers first differ at trace tick 13440; with the rule 13450 (experiment build on 0.12.3 with T37; buildings 14320 and generator 13453 unchanged; every Storm Coast pair unchanged). Whole recording: 43 of 43 step-back ends choose in a later update (11 door walks and 32 new pairings exactly one update after) |
| T37 | 6.1a, the field during the step back: E-trace-019, attacker 16592 stepping back from (131, 79) to the point (132, 80) of his waiting field (33, 20), counters 13240 .. 13252 | his field | (32, 19) (the truncating field of (131, 79)) through counter 13251, progress 0.99; (33, 20) from 13252, the update the step ends. Showing the waiting field during the step: war settlers first differ at trace tick 13250; with the rule 13440 (experiment build on 0.12.3; buildings and generator unchanged; Storm Coast pairs unchanged). Whole recording: 2309 of 2309 rows of an attacker in a lattice step show the truncating field of the step's start sample |
| T38 | 6.3b: E-trace-019 (R6, the war), the two duels at the flag of player 3's guard house (21, 20): player 2's attacker 18218 against the defender 18874, fighting from counter 14790; player 2's 16600 against 18867 (whose earlier duel began at 14771), paired later and fighting from 15096; both run a round in counter 15147 (4 draws in that tick) | the hit results of 15147 | 18218 strikes and hits 18874 (the hit animation; 18874 dying from 15165), 16600 strikes 18867 and misses (16600 hit only at 15164). Running the pairs in their sending order (16600's first) swaps the two results: 18218 dies at 15182, the war's settlers first differ at trace tick 15170, buildings 15650, generator 15181; with the rule settlers 16700, buildings 16850, generator 16716 (experiment build on 0.12.3 with the rules of 2026-11-21; every Storm Coast pair unchanged) |
| T39 | 3.1 (the distance factor): E-trace-019 (R6, the war), player 3's attack order of counter 16650 on player 2's building (31, 21) with 3 attackers (19304, 19481 of (21, 20), 18814 of (21, 30)); player 2's defend-outside setting 0.5; (47, 19) at `e = 17` with 6 soldiers present, (42, 23) at `e = 12` with 3, (51, 17) at `e = 22` with 1, the target with 1 | the interceptors sent | 2: one from (47, 19) (16608, paired with 19304) and one from (42, 23) (18285, paired with 19481; 18814 has none); without the factor (47, 19) sends 2 (16608 and 16639) and 3 go out. Without the factor the war's settlers first differ at trace tick 16700, buildings 16850, generator 16716; with the rule all three from 17030 (settlers and buildings) and 17032 (generator) (experiment build on 0.12.3 with the rules of 2026-11-21; every Storm Coast pair unchanged) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-mil-003 | code reading | `S2DNG.exe` | attack order (`attack-building`), execution |
| E-mil-003 | code reading | | dialog count, fractions, "Can not attack" |
| E-mil-003 | code reading | | availability, per-building count, launch, rank order, link, record and first-striker draw |
| E-mil-003 | code reading | | interceptors, pairing, meeting and waiting fields |
| E-mil-003 | code reading | the debug state table | soldier states during an attack |
| E-mil-003 | code reading | | opponent choice and pairing |
| E-mil-003 | code reading | | rounds, hit test, animation countdown, fighting time |
| E-mil-003 | code reading | | arrival, message, death cleanup |
| E-mil-003 | code reading | | promotion, health init, slider clamp, AI settings |
| E-mil-005 | code reading | | button-to-fraction mapping |
| E-mil-005 | code reading | | sliders continuous |
| E-mil-005 | code reading | | defender walk target, survivors' next steps |
| E-mil-010 | code reading, saves of the original | (session 2026-11-14; the fight saves of E-sav-017) | 6.1a: the attacker's one-step offset with one draw, the defender on the point, the interceptor's duel field; the defender choice needs no presence (5.1) |
| E-mil-011 | code reading | (session 2026-11-15 night) | 6.1a: the wait job starts only with an opponent; an unpaired attacker waits on the point without a draw; the step back to the point before any next choice (Q-1050) |
| E-mil-012 | code reading | (session 2026-11-15 night) | 4.2, 4.3: the door is the target's flag; the waiting-field search's start, walk test and free test (Q-1051) |
| E-mil-015 | code reading and data | (session 2026-11-16 night) | 1.3: which enemy buildings open the attack dialog (headquarters, or a soldier limit above 0 and occupied), T1b |
| E-mil-016 | code reading | (session 2026-11-16 night) | 4.4: the waiting attacker's return to his own building's flag; the return refused when that building is gone |
| E-mil-017 | code reading | (session 2026-11-16 day) | 4.4: the soldier's per-tick test of his four links; with none left he goes home as a settler without a building (Q-142) |
| E-mil-020, E-trace-019 | code reading, recording, heap dumps | (session 2026-11-17 day): the per-building share's x87 sequence and its constants; R6's attackers and garrisons at counter 12168, player 2's attack strength 1.0 and the soldiers' links in the heap dump of 12000 | 1.1 item 7 (the distance factor), T23 |
| E-mil-021, E-trace-019 | recording and experiment build | (session 2026-11-17 day): the settler rows of R6's first attack's six soldiers from trace tick 12169 | 4.1a (T24) |
| E-mil-022, E-trace-019 | code reading, recording, experiment build | (session 2026-11-17 day): the two searches' neighbour order; R6's interceptor 16370 and attacker 18180 on (32, 22) | 3.3, 4.3 (T25) |
| E-mil-024, E-trace-019 | code reading, recording, experiment build | (session 2026-11-17 day): the soldier's states on reaching a meeting field (waiting, then the waiting check, then the wait job); R6's attacker 18180 at (32, 22) | 6.1a which attacker steps on arrival (T26) |
| E-mil-025, E-trace-019 | code reading, recording, experiment build | (session 2026-11-17 day): the fight walk's arrival test (movement seen stopped in the next update); R6's duel of 16579 and 16378; the soldiers' fields after the offset steps | 5.3 the defender's arrival (T28), 6.1a the field after the step (T29) |
| E-mil-027, E-trace-019 | code reading, recording | (session 2026-11-19 night): the settlers' shared pre-update in the dying state (the doodad at 14 of 28 ticks, on the lattice point or the field's point, unless one stands there); R6's objects and counter tables (doodad0 of type dacb0a13 at each duel's death) | 6.6 / TRN-CARRY-7.1 the skeleton doodad (T31a) |
| E-mil-029, E-trace-019 | recording | (session 2026-11-19 night): R6's 181 reinforcement arrivals, each standing one update on its building's field before turning idle | 5.1 a reinforcement a candidate one update after his step (T35) |
| E-mil-026, E-trace-019 | code reading, recording | (session 2026-11-19 night): the idle soldier's tests before wait-for-leave (his opponent on his building's flag field, or the opponent's arrival mark in their pair record); R6's defenders 16378 and 16667 of the guard on (31, 21) | 5.3 the held defender (T34) |
| E-mil-030, E-trace-019 | recording, experiment build | (session 2026-11-21 day): the war's settlers table around the first capture (counters 13240 .. 13600) and over the whole recording | 6.1a the field during the step back (T37), the step back's end (T36) |
| E-mil-031, E-trace-019 | recording, experiment build | (session 2026-11-21 pm): R6's settlers table (the four soldiers' states and clips at counters 14773 .. 15182) and generator | 6.3b the order of the pairs' rounds (T38) |
| E-mil-032, E-trace-019 | code reading, recording, experiment build | (session 2026-11-21 eve): the interceptor count's machine code (its distance factor) and R6's settlers table at counter 16650 | 3.1 the distance factor of the interceptors' count (T39) |
| data | script | `military.lua`, `ai.lua` | fighting time, AI profile values |

## Open questions

