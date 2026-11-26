# GFX-ANIM: Animations of settlers, animals, buildings and ships

Status: draft
Owner question(s): play-test findings F7, F20 to F22 (Run C); plan step 6.4; Q-048 (bone limit), Q-083 (building rotation attribute, part); raises Q-087 to Q-090
Game build: 11757

## Summary

Every animated object plays Granny animation files chosen by name from its kind's
animation set. Settlers pick a base animation from their logic state (standing,
walking, swimming, working, picking up, putting down, hitting), refined by the
profession and the carried good; idle settlers play random idle clips. Animals know
Stand, Walk and Dying; buildings open and close their door, play a Working clip and
show production animals; ships bob on the water. Animation time is game time (the
frame's game time step: it stops while paused and runs faster at speeds 2 and 3) times
a global speed, not game ticks; ticks only decide what plays. Settlers with the
same model and animation share one skeleton pose.

## Rules

### GFX-ANIM-1 Animation sets and names

- **GFX-ANIM-1.1 Base names.** Settlers (and the donkey) have ten base animations,
  indexed 0..9: Stand, Walk, SwimIdle, SwimFwd, WorkLoop0, WorkLoop1, WorkLoop2,
  Pickup, PutDown, Hit. Animals have three: Stand, Walk, Dying. Each `Animation`
  entry of the kind's XML has a name, a file, an optional `tribes` bitmask (bit 0
  Roman, bit 1 African, bit 2 Chinese; 0 or absent = all tribes) and optional `Sound`
  and `Particle` children with a `start` time in animation seconds.
- **GFX-ANIM-1.2 Suffixed variants.** For every entry of the game's object list
  (goods and tools as named in `items.xml`, and the professions: Fishing Rod,
  Scythe, Axe, Hatchet, Smelter, Bulldozer, Forester, Stonecutter, DonkeyBreeder,
  Pig Farmer, Baker, Helper, Miller, Smith, Locksmith, Slaughter, Scholar, Hunter,
  Constructor, ShipBuilder, Sword, Boat) the set looks up `<Base><Name>` for all ten
  bases (for example WalkWood, WorkLoop0Smith, HitSword). A variant list exists as
  soon as any one of the ten `<Base><Name>` clips exists (the loader stops at the
  first base found and then fills the list for all bases); a base the list has no
  clip for stays empty in the list (it does not copy the base clip), so
  `WorkLoop0Smith` alone gives the Smith list a WorkLoop0 entry and nothing else, and
  the lookup of 1.3 goes on to the next list for the other bases (E-gfx-010). (Corrected from "only if Stand<Name> exists"; E-gfx-008.) A profession's name is the job's display name (`Woodcutter`, `Smith`,
  `Baker`, ... as set in the job properties); a good's or tool's name is its item
  name.
  Profession names are tribe specific: their lists are kept per tribe (0..2) and a
  clip whose `tribes` mask excludes the tribe counts as missing. Goods and tools are
  tribe independent.
- **GFX-ANIM-1.3 Resolution order.** The clip for base b of a settler with lookup
  good g, profession j and tribe t is: (1) the tribe-t list of j if that list has a
  clip for b; else (2) the list of g if it has a clip for b; else (3) the base clip b.
  The lookup good g is (Q-192 b, E-gfx-010):
  - the good of the ware the settler carries, while it carries one;
  - else the good of its tool (GFX-OBJ-7.4), whether or not the tool is drawn at the
    moment (fog, GFX-OBJ-7.5), except after a ware was used up in its hand (the
    shipwright's planks): then none until it next takes or puts down a ware
    (GFX-OBJ-7.4a, Q-355);
  - a carrier rowing a boat: Boat (3.4);
  - none for a carrier or donkey without a ware (base clips).
  The profession j is the job's display name; a carrier's is `Carrier` (`Donkey` for
  the donkey). Examples from the shipped tables:
  | settler | doing | clip |
  |---|---|---|
  | woodcutter | walking out, standing | WalkAxe / StandAxe (settler_walk_pickaxe / settler_stand_pickaxe) |
  | woodcutter | chopping (request 4) | WorkLoop0Axe (woodcutter_chop) |
  | woodcutter | carrying the log | WalkWood / StandWood (settler_walk_log / settler_stand_log) |
  | stonemason | walking out, standing | WalkPickaxe / StandPickaxe (the Stonecutter list has only WorkLoop0) |
  | stonemason | quarrying | WorkLoop0Stonecutter (stonecutter_crush, profession list) |
  | stonemason | carrying the stone | WalkStone / StandStone |
  | carrier | with a plank | WalkPlank / StandPlank; without a ware Walk / Stand |
  | soldier | walking | WalkSword (soldier_walk); fighting by request codes (3.8) |
  Confidence: high. Measured (2026-10-25, TRC-29, E-heap-002): the clip the original
  resolved for every settler row of four recordings (5.36 million rows, 81 different
  clips) is the one this lookup gives, with the profession's tribe mask, the
  profession list before the good list before the base clip; the only exception is
  the shipwright's lookup good (`GFX-objects.md` GFX-OBJ-7.4a). The donkey's own set is
  used for the donkey. (The recorder does not hold the good in a settler's hand: a ware
  clip was checked against the wares linked to the settler and its workplace's
  products, not tick for tick.)
  Class rule (2026-10-24): every `Animation` entry of the settler and donkey tables is a
  clip of this lookup. Covers (inventory): *settler clip* from
  `data/character/animations/*`, `data/character/donkey/*`.
  Covers (inventory): *other file* from `data/character/animations/*`.
- **GFX-ANIM-1.4 Random idle sets.** `RandomAnimations/Random` has `minDelay`,
  `maxDelay` and `Animation` children with `src` and `probability` (default 1.0). A
  `Random` with an `item` attribute applies to settlers carrying that good; the one
  without applies to all others. Weights are integers: weight_i = trunc(p_i x q + 0.5)
  with q = 1 / p_min stored as a 32-bit float and p_min the smallest probability of the
  set (truncation toward zero; answered 2026-11-08 b, Q-088, E-gfx-041:
  the compiler's float-to-integer routine). With the shipped set (`settler_animations.xml`:
  0.01, 0.2, 0.1; q = 100.0 in 32 bits) the weights are **1, 20 and 10** (0.2 x 100 + 0.5 is
  20.5000003 and truncates to 20), in 32-bit and in extended precision alike, so the bag of
  the eleven clips holds 3 x 1 + 4 x 20 + 4 x 10 = 123 slots. A bag of sum(weights) slots
  holds clip i weight_i times and is shuffled by sum(weights) random swaps.

### GFX-ANIM-2 Playing and time

- **GFX-ANIM-2.1 Speed.** A clip plays at `globalSpeed` (1.8, `graphics.xml`
  Settings/Animations) times game time (the scene's game time step per frame, not
  real time: corrected 2026-10-18, Q-226 a, measured in live trace 1, E-trace-001),
  weight 1, starting at the render time of the frame that started it. Ticks do not drive animation time; a walk cycle is not
  synchronised with field steps.
- **GFX-ANIM-2.2 Looping.** Stand, Walk, SwimIdle, SwimFwd loop forever; WorkLoop0..2,
  Pickup, PutDown, Hit, random idle clips, Dying, DoorOpen, DoorClose and Working
  play once.
- **GFX-ANIM-2.3 Events.** A clip's Sound or Particle event fires once when
  (now - clip start) x globalSpeed reaches its `start`; only events with `start` at or
  after the clip's start offset are scheduled.
  Covers (inventory): *particle effect* from `settings/buildings.xml`. Covers
  (inventory): *sound* from `buildings.xml`.
  **When the events of building and settler clips start (2026-11-16 day, Q-186 f; E-gfx-072, code
  reading).** Events are scheduled by the clip's start on a model, together with the model's event
  target (where a sound plays, where an effect stands):
  - a **building's Working clip** is started (play once, from time 0) at each work start of the
    building when it is not already working, so its events fire once per work cycle, each at its
    `start` (high);
  - a **settler's play-once clip** (the work clips, Pickup, PutDown, Hit, the idle clips) gets a pose
    of its own restarted at time 0 (2.4 b), so all its events fire once per play, at that settler
    (high);
  - a **looping clip** shared by several settlers (2.4 a) schedules its events only when a pose is
    started or restarted for it, with the event target of the settler that started it; a settler
    that **joins** a running pose schedules nothing, so the shared clip's sounds play at the first
    settler's place, not at each joiner's. The events are not scheduled again when the loop wraps
    (medium: the wrap was not read). Code-driven sounds such as the footsteps (SND-2.2) are not clip
    events and are not affected.
- **GFX-ANIM-2.4 Shared poses (corrected 2026-10-18, Q-220; E-gfx-023).**
  Each settler (and animal) model file has one pool of skeleton poses, used
  by every settler of that model file whatever its player, site or work
  place. A pose is a running clip with its own clock. Which pose a settler
  gets when its clip changes (3.2) depends on whether the clip loops:
  (a) **Looping clips** (Stand, Walk, SwimIdle, SwimFwd): the settler takes
  the first pose in the pool, in pool order, that either has no user or
  plays the same clip. A pose with users that plays the same clip is joined
  at its current time (the settlers are in phase); a pose with no user is
  restarted at time 0 with the new clip.
  (b) **Play-once clips** (every request clip: WorkLoop0..2, Pickup,
  PutDown, Hit; and a random idle clip, 3.3): the settler always gets a pose
  of its own: the first pose in the pool with no user, restarted at time 0
  with the clip, or a new pose when every pose has a user. No other settler
  ever joins it, so a play-once clip always starts at time 0 for the settler
  that asked for it and never shows another settler's frozen last frame.
  (c) **Leaving.** A settler leaves its pose when its own clip changes (and
  when its visual is deleted). When the last user leaves, the pose stops
  and has no clip; it stays in the pool for reuse by any clip.
  (d) **Sweep.** Every 10 s, while the pool holds more than 5 poses, poses
  without a user are freed.
  The pool order is the order poses were made; freeing one moves the last
  pose into its place. The soldier's fighting clips (the kind's own clip
  choice, Q-087) are attached as looping and so are shared like (a).
  Confidence: high (E-gfx-023).
- **GFX-ANIM-2.5 Clocks.** A pose's clock advances by the frame's game time step
  each frame (frame time x game speed; 0 while paused); the whole skeleton is sampled
  and the world pose rebuilt every frame. Measured (E-trace-001): pose clocks advanced
  0.025 per frame under the fixed step at speed 1, not at all during a 20 s pause while
  frames ran, and 2x and 3x as fast at speeds 2 and 3.

### GFX-ANIM-3 Settlers: which clip plays

- **GFX-ANIM-3.1 Per tick choice.** Each logic tick (0.1 s) a settler's visual
  derives a base index: if the logic requests an action, request 2 -> PutDown, 3 ->
  Pickup, 4 -> WorkLoop0, 5 -> WorkLoop1, 6 -> WorkLoop2, 7 -> Hit. A work loop
  request is ignored (treated as none) while the settler's "inside / hidden" flag is
  set. Without a request: moving (step progress >= 0) -> Walk, else Stand; when the
  model's base is below the terrain height the swim variants SwimFwd / SwimIdle are
  used instead. **The swim test is a water-level test (2026-11-16 day, Q-180 g; E-gfx-075, code
  reading):** not the field's Water state and not the terrain: the settler swims when
  `foot + sink + height / 2 <= water level`, where **foot** is the height of the settler's drawn
  base point as last drawn (the blend of the two tick positions of GFX-ANIM-3.1a's walk, after
  the optional snap to the terrain, with the sink already added), **sink** is the settler's
  current sink offset (0 for a living settler; only a dying settler's sink lowers it, by 0.35
  per second), and **height** is the vertical extent of his model's mesh (the highest minus the
  lowest vertex height of the model's vertices, measured once when the settler's view is made).
  The **water level** is the map's global water surface height. In words: a settler swims when
  the middle of his body is at or below the water surface. So a settler on dry ground above the
  water level never swims; one walking a field whose surface lies under the water level (a boat
  carrier's waterway, a shore below the water plane) does. Confidence: high (all three terms
  read from code, 2026-11-16 day b, E-gfx-075). The settler's dying state (6) keeps the current clip.
  **Kinds and the flag (answered 2026-11-08 b, Q-089; E-gfx-041).** A
  settler's visual kind is its settler class: 0 worker, 1 builder, 2 carrier (donkeys and boat
  carriers included), 3 soldier, 4 leveller, 5 specialist (scholar, explorer); each class has
  its own kind object (worker: tool by job and the woodcutter's tree fall; carrier: the boat
  override of 3.4; soldier: no override). The "inside / hidden" flag is cleared in every tick
  the settler walks (step progress >= 0); while it stands it is set when its field is the
  field of the building its view is bound to (a worker's workplace, GFX-ANIM-6), and once set
  it stays until the next walk. So a worker's work loop inside his building is never drawn,
  and the same worker working on the land (another field) shows it.
- **GFX-ANIM-3.1a A walk counts as moving from the tick it is ordered (Q-216
  d, E-gfx-030).** "Moving" in 3.1 is the step progress of the settler's
  movement being >= 0, and ordering a walk (free walk, road walk or fine walk)
  that finds a path sets that progress to 0 in the ordering tick itself, before
  the visual's tick. So in a tick where a walk is ordered but the settler does
  not move yet (TRN-FREE-5.6, ECO-HQ-4.6b), and no request is active, the
  visual already plays **Walk** (or its carried-good variant, e.g. WalkWood),
  drawn in place: the position is from + (to - from) x 0 = from. There is no
  Stand tick between two walks. A walk whose order finds no path leaves the
  progress at -1 (Stand). An outdoor worker's walk to his target ordered on his
  flag when the target is next to it (no field to walk) counts as moving too: the
  walk clip runs on from the walk out into the fine approach without a Stand tick
  (measured 2026-10-25 evening, E-trace-010: woodcutter 90392 on his flag (142, 55)
  in trace tick 2717, Walk with its clip going on, the approach moving from 2719). Confidence: high (the movement's progress is the
  value the view tests, reset to 0 by the path start and
  the fine path start; the visuals are ticked after each logic
  tick by the world view's logic-tick hook, E-gfx-014).
- **GFX-ANIM-3.2 Change rule.** The clip is swapped only when the base index, the loop
  flag, or good / profession / tribe changed. The new clip resolves by GFX-ANIM-1.3.
  Swapping detaches the shared pose and attaches to (or creates) the pose for the
  new clip (GFX-ANIM-2.4).
- **GFX-ANIM-3.3 Random idle.** A visible standing settler with no running random clip
  gets a random set (its good's set if one exists, else the default). The set waits
  `minDelay + r x (maxDelay - minDelay)` seconds (r uniform 0..1) plus the duration of
  the clip chosen last, then plays the next clip of the shuffled bag once; at the end
  of the bag it is reshuffled. When the random clip ends (duration left < 0) the
  settler returns to its resolved Stand clip. Leaving Stand, becoming invisible, or any
  clip change deletes the random set. The wait is counted down once per tick by the tick
  length (0.1 s; E-gfx-041; Q-231 d), so it stops while paused.
- **GFX-ANIM-3.4 Kind override (corrected).** Only the carrier kind overrides the
  base index: while it rows a boat on a waterway it plays base 4 (WorkLoop0) standing
  and base 5 (WorkLoop1) moving, with the lookup good Boat, i.e. WorkLoop0Boat
  (boat_stand) and WorkLoop1Boat (boat_walk), looping (an override plays with loop
  count 0). All other kinds, the soldiers included, never override (E-gfx-010); the soldiers' fight clips come from the request codes of
  3.8 with the lookup good Sword (the round of MIL-ATK-6.3 sets them; Q-087 answered
  2026-11-08 b). So the struck soldier shows HitSword in the round that scores, while
  its health falls only at the next round's start (MIL-ATK-6.3 step 1).
- **GFX-ANIM-3.5 Carried item and tool.** See `GFX-objects.md` GFX-OBJ-7: the
  ware and the tool are drawn at the attach bone `SLOT_HandR` with the bone's full
  model-space matrix times the settler's world matrix, every frame, no offset; the
  tool shows whenever the settler is drawn, carries no ware and stands outside the
  fog (any clip, walking out and back included). The earlier "only while the clip
  marks the item as shown" was a misreading (E-gfx-010).
- **GFX-ANIM-3.6 Footsteps.** A visibility counter rises 5 per visible frame within
  squared camera distance 50, i.e. within 7.07 world units of the camera (max 100) and falls 3 otherwise; while it is positive and
  the settler is visible a step event fires every 0.5 / globalSpeed s (0.278 s). The
  step accumulator is held at 1.0 while the settler stands, so the first step sounds on
  the first moving tick; it accumulates logic-tick time, not frame time (high,
  E-gfx-008; this is the value SND-2.2 means by "0.5 s divided by the speed
  scale"). **The distance is measured from the camera's eye** and this test decides against
  SND-8.5 (Q-1006, answered 2026-11-04, E-snd-002): on level ground the
  eye is always too far, so steps belong to the follow camera and to steep slopes at zoom 0;
  see `SND-sound-music.md` SND-8.6. The bound itself passes (squared distance exactly 50
  rises).
- **GFX-ANIM-3.7 Dying.** A settler entering its dying state requests code 6
  (WorkLoop2, the fall clip) for the configured death time; when that time has run
  out it is removed. While dying, once its clip has ended, it sinks 0.35 world units
  per second until removed, applied once per tick as 0.35 x the tick length (0.035 a tick;
  E-gfx-041, the tick block's third value, the configured tick length 0.1 s; Q-231 d).
- **GFX-ANIM-3.8 Requests per step.** A request is a code with a duration: the logic
  sets both, the duration is a number of logic ticks (0.1 s each) and counts down one
  per tick, and when it reaches 0 the request clears (so a request outlives the step
  that set it only for its duration). Which step sets which code (E-gfx-008):
  | step | code | duration |
  |---|---|---|
  | outdoor work action at the spot (woodcutter, stonecutter, forester, hunter, fisher, farmer harvesting, ...) | 4 WorkLoop0 | the building type's work time (ECO-WORK) |
  | farmer sowing (the reserved spot's deposit has size 0) | 5 WorkLoop1 | work time |
  | production inside a building (each work step) | 4 WorkLoop0 | the step's duration |
  | putting a ware down at a flag or into a building; taking a ware (both halves of the exchange) | 2 PutDown / 3 Pickup | the logic's put / take time |
  | geologist at a spot: no resource entry on the field and not buildable ground | 5 WorkLoop1, else 4 WorkLoop0 | 80 ticks (8 s) |
  | builder hammering (each work **step**, four per unit; corrected 2026-10-17, E-pt5-001) | 4 WorkLoop0 | the construct time, 55 ticks, set when the step's timed action starts at its construction position (GFX-OBJ-2.6d) |
  | leveller shovelling | 4 WorkLoop0 | the action's duration |
  | catapult aiming / shooting | 4 WorkLoop0 / 5 WorkLoop1 | |
  | soldier fight round (`MIL-attack-combat.md` MIL-ATK-6.3; answered 2026-11-08 b, Q-087, E-gfx-041) | the soldier who strikes this round: 4 (WorkLoop0Sword, the attack), set before the hit draws; the other: 7 (HitSword) when the round's hit test scores, else 5 (WorkLoop1Sword) | both the fighting time (16.7 ticks, `mi_setFightingTime`) |
  | dying | 6 WorkLoop2 | configured death time |
  The woodcutter's step also arms a removal timer on the tree of round(work time) + 5
  ticks (60): if the tree has not been harvested by then (the woodcutter was
  interrupted), its removal (ECO-OUT-6.4, and the sink of 3.8c) starts anyway, so a
  tree that fell on screen always goes away (**corrected Run D, E-gfx-014: earlier read as "schedules the tree fall"**).
  Confidence: high for the codes, medium for the durations' names.
- **GFX-ANIM-3.8a Tree fall (play-test 2, chunk 4; corrected Run D, E-gfx-014).**
  The graphics side starts the fall in the first tick in which **3.0 s of game
  time** have passed since the woodcutter's work action started: elapsed = (the
  action's total ticks - its remaining ticks) x 0.1 s, i.e. tick W + 30 with W the
  tick the 55-tick action starts. This is after all three axe chops (clip 1.24,
  2.88, 4.56 s at globalSpeed 1.8 = game 0.69, 1.60, 2.53 s: ticks W + 7, W + 16,
  W + 25), as observed (OBS-8: three hits, then the tree falls). (**The earlier
  reading "work clip time 3.0 s" was wrong: the factor is the tick length, not
  globalSpeed.**) The tree is the one his job targets, found while it is not
  already falling. The tree model is turned (no clip) about a horizontal axis
  through its base that points from the tree toward the woodcutter, so it falls
  sideways, perpendicular to the line to him. Each logic tick the angle grows by
  **rate / 10** (10 = ticks per second), and the rate, which starts at (pi / 2) /
  (20 x globalSpeed) = (pi / 2) / 36 = 0.043633, is then multiplied by 1.2; the
  frame interpolates between ticks. The first step happens in the start tick
  itself. After j fall ticks the angle is 0.0043633 (1.2^j - 1) / 0.2:
  | j | 1 | 2 | 4 | 8 | 12 | 16 | 18 | 20 | 21 | 22 | 23 | 24 |
  |---|---|---|---|---|---|---|---|---|---|---|---|---|
  | angle (rad) | 0.0044 | 0.0096 | 0.0234 | 0.0720 | 0.1727 | 0.3815 | 0.5590 | 0.8146 | 0.9819 | 1.1826 | 1.4235 | 1.7125 |
  In the 24th tick (W + 53) it passes pi / 2 and the tree stops updating: it keeps
  the matrix of the last frame drawn while falling, which at any normal frame rate
  is just under 1.4235 rad (81.6 degrees; no clamp to 90). So the fall lasts 2.4 s
  and **ends two ticks before the harvest** (3.8e). (**Corrected: the earlier "12
  ticks, 1.40 to 1.73 rad" ignored the division by 10.**) A fall sound plays at the
  tree when the fall starts. Confidence: high (E-gfx-014); the side it falls to (left or right
  of the woodcutter) medium; W + 30 exact to +-1 tick (whether the action's
  countdown already ran in its start tick), medium.
  **The side (2026-11-17 night, Q-202 c; E-gfx-077, code reading, high for the formula):** the
  axis a = (x, 0, z) is the unit vector from the tree to the woodcutter, turned into the tree's own
  frame (its yaw and scale undone), and the turn is applied in that frame before the tree's own
  matrix. Its rotation matrix, in the row-vector convention the world matrices use (the translation
  is the fourth row, GFX-VIEW T16), has c + t x^2, c + t y^2, c + t z^2 on the diagonal (t = 1 - cos, s = sin,
  c = cos) and off the diagonal: row 1 (t x y - s z, t x z + s y), row 2 (t x y + s z, t y z - s x),
  row 3 (t x z - s y, t y z + s x). So the tree's up vector goes to (s z, c, -s x): the top moves
  toward **(z, 0, -x)**, which is the woodcutter's **left** as he faces the tree (in the game's
  left-handed frame, x right, y up, z away from the default camera). Test: tree at world (0, 0, 0),
  woodcutter at (0, 0, -3): the top moves toward -x; woodcutter at (3, 0, 0): toward -z.
- **GFX-ANIM-3.8b Tree growth (Run C, F20, E-gfx-013).** A tree (or any
  growing deposit) is one model throughout; there are no separate sapling
  models and no stages. While the deposit's size s is below 1.0 the model is
  scaled, on top of its element rotation and size (GFX-OBJ-1.4), by
  **height s and width min(3 s, 1)** (x and z by min(3 s, 1), y by s), with s
  interpolated between the previous and the current tick for the frame; size 0
  (a sapling whose planting has not ended, `ECO-outdoor-workers.md` ECO-OUT-6.1)
  draws it at scale 0, i.e. invisible. So a freshly planted sapling (s = 0.1) is
  0.3 wide and 0.1 high, reaches full width at s = 1/3 (tick 260 after planting:
  (1/3 - 0.1) / 0.0009) and full height at s = 1.0 (tick 1001); from then on it
  is drawn unscaled. This replaces GFX-VIEW-4.5's "scale from the start size to
  1". Confidence: high.
- **GFX-ANIM-3.8c Removed deposits sink (F21, OBS-8; corrected Run D, E-gfx-014).**
  When a deposit's removal starts (ECO-OUT-6.4: a felled tree at its harvest, the
  last stone of a pile, a deposit whose time ran out), its model sinks straight
  down into the ground until the deposit is deleted. The sink is not in the tree's
  own visual but in the deposit renderer, so it applies to every deposit model:
  - At the removal start the renderer records the model's current height
    position y0 (the y of its matrix, i.e. the ground at its base) and **h = the
    height of its world-space bounding box under its current matrix** (the model's
    local box with its eight corners transformed, then the axis-aligned box taken
    again). For a felled tree that is the lying tree's box, so h is about the
    crown's width, not the tree's height.
  - Each logic tick it sets the target s = 0.05 x the deposit's removal counter
    (the counter is 2 at the end of the harvest tick and 20 when the deposit is
    deleted, ECO-OUT-6.4), so s = 0.10, 0.15, ... 0.95: **linear, 0.05 h per tick**.
  - Each frame it draws the model with its own matrix except the height: y = y0 -
    s x h, s interpolated between the previous and the current tick's target
    (4.1's blend). Before the first tick after the removal start s is 0.
  - When the deposit is deleted (18 ticks after the harvest tick) the model goes
    at once; it has sunk 0.95 h by then (the last tick's target is drawn fully).
  - Rotation and x / z are untouched; nothing fades or scales.
  - **(Play test 10, Q-216 b, Q-300; E-gfx-022)** The box really is re-taken
    axis-aligned after the turn, so a tree that fell diagonally to its
    model axes has a larger h and sinks faster per tick (birch up to about
    10.2 at 45 degrees). The sink is written after every element has run
    its own frame update, so it also wins over a standing tree's own matrix:
    a tree whose removal starts while it still stands (its lifetime ended,
    or the 60-tick fallback of ECO-OUT-3.5 fired without a fall) **sinks
    upright**, with h = its standing box height, and never falls. The fall
    itself (3.8a) runs from the woodcutter's view object, which is ticked
    for every settler whether on screen or not, so a tree felled off screen
    still falls toward its woodcutter as usual.
  The lying tree's box reaches well below the ground (it is the rotated model box,
  not the mesh), so most of h is already underground: with the shipped models h
  is 7.4 to 7.8 (RS_TreeBirch01), 9.2 to 9.5 (RS_TreeBroadLeaf01), 6.6 to 7.0
  (RS_TreeFir01) at element size 1 (the range is the two extreme fall directions
  against the model's own axes; scaled with the element size of GFX-OBJ-1.4), the
  box's top 4.5 to 5.4 above y0. So the trunk (lowest) disappears first and only
  the crown sticks out sideways (photo `tree-sinking.jpg`), the crown is under
  the ground after about 0.6 to 0.65 of the way (tick H + 10 to H + 11), and the
  rest of the sink is invisible. This replaces the earlier "no sinking, lies still
  until deleted". (The earlier reading looked only at the tree visual, whose lying
  state indeed never changes its matrix; the deposit renderer's removal list,
  filled from the simulation's "removal started" event, moves it.) A grown tree
  also has a short shake state (a wobble about its base axis with amplitude a (1
  - t / d)^2 and phase 11 pi t, t seconds since it started) together with an
  attached effect; **nothing in this build sets the shake flag or attaches the
  effect (both are only cleared), so a tree never shakes** (Run D, E-gfx-016;
  an implementation leaves it out). Confidence: high for the rule and the numbers (E-gfx-014); the
  box from the mesh vertices medium; a standing tree (state 2, not shaking) whose
  removal starts redraws its own matrix every frame, and which of the two writes
  wins there was not traced (Q-300).
- **GFX-ANIM-3.8c1 A felled tree after a load (2026-11-24 c, Q-1087, stand-ins row 14; E-gfx-078,
  code reading, high unless marked).** A load makes every view object anew: each tree is drawn
  **standing** (no fall, no sink) until something starts one, and only two things do, both in play
  ticks after the load, never the load itself:
  - **The fall** is started by the woodcutter's view (3.8a) in any tick in which his running action
    is the work action, his job is the woodcutter's, at least 3.0 s of it have passed (elapsed =
    (total - remaining ticks) x 0.1 s, read from the action the save restored) and his target
    tree is not already falling. So a woodcutter loaded **before W + 30** fells the tree at W + 30
    as usual (counted from the saved action, not from the load); one loaded **at W + 30 .. W + 54**
    starts the fall again from upright in the first tick after the load (the fall that was under
    way or finished is not restored), about the same axis as in play (the line from the tree to
    where he stands), with the full acceleration of 3.8a; tick exact to +- 1 (medium, as 3.8a).
  - **The sink** (3.8c) is started only by the removal-start event of the simulation (the harvest,
    or ECO-OUT-3.5's fallback). A deposit whose removal was already under way when the game was
    saved (the removal counter, which the save keeps, between 2 and 19) gets no sink: it is drawn
    **standing upright and unsunk** until its counter reaches 20, then it goes at once. No fall
    starts for it either: its woodcutter has left the work action (he is taking the log or walking
    home).
  So a tree saved lying (fallen, harvest not yet reached, W + 53 .. W + 54) falls again from
  upright after the load and starts sinking at the harvest with the box of its pose then (nearly
  upright: h close to the standing height); a tree saved lying after its harvest stands upright
  after the load and vanishes when its 18 ticks run out.
- **GFX-ANIM-3.8d Axe and shovel (F22).** The woodcutter's work clip is
  `woodcutter_chop.gr2` (WorkLoop0Axe, 10.0 s, chops at 1.24 / 2.88 / 4.56 s);
  the forester (WorkLoop0Forester) and the leveller (WorkLoop0Bulldozer) play
  `work_shovel.GR2` (10.0 s, digs with the shovel sound at 0.30 / 2.35 / 5.28 /
  8.73 s, the forester's "plant grows" sound at 7.30 s). Both play once (2.2) at
  globalSpeed 1.8, so the 55-tick (5.5 s) work action reaches clip time 9.9 s:
  three axe chops, all four digs. The tool is `GO_Axe01` / `GO_Shovel01` on the
  attach bone with the bone's full matrix (`GFX-objects.md` GFX-OBJ-7; axe values
  C7-C9 of GFX-OBJ-7.8).
  **A planting as seen and heard (play test 11; `settler_animations.xml`,
  `sounds.lua`, 2.3, ECO-OUT-4.7, 3.8b).** The forester's work action starts
  when he has reached the spot (after the fine approach, 3.8e) and lasts 55
  ticks (5.5 s at speed 1); the clip runs once at globalSpeed 1.8, so an event
  at clip time c fires c / 1.8 s after the action's start:
  | event | clip time | after the action's start | sound definition |
  |---|---|---|---|
  | `StoneDust` particle | 0.0 s | 0 s | (particle) |
  | shovel dig 1 | 0.30 s | 0.17 s (tick 1 or 2) | `sound_shovel` "shovel", one of `Human/Forrester/digging_01..03.wav`, priority 9, 3D minimum distance 16 |
  | shovel dig 2 | 2.35 s | 1.31 s (tick 13) | same |
  | shovel dig 3 | 5.28 s | 2.93 s (tick 29) | same |
  | "plant grows" | 7.30 s | 4.06 s (tick 40) | `sound_plantgrows` "plant grows", `Human/Forrester/plant_growing.wav`, priority 11, minimum distance 15 |
  | shovel dig 4 | 8.73 s | 4.85 s (tick 48) | `sound_shovel` |
  All five fire (the action ends at clip time 9.9 s); each sound is a 3D
  sound at the forester (SND-2.1, 2.4, 8.3). The tree does not exist visibly
  until the action's **end** (tick 55): the deposit created at the spot choice
  has size 0 and is drawn at scale 0 (3.8b), so no sapling is seen while he
  digs; in the tick the action ends its size becomes 0.1 and it appears at
  once as the full tree model 0.3 wide and 0.1 high (0.7 s after the "plant
  grows" sound), then grows by 3.8b: full width at 260 ticks, full height
  (ripe) at 1001 ticks after the planting; no stage models, no sound or
  particle at the appearance or while it grows. Confidence: high (data and
  2.3; the fire tick to +-1 as in 3.8a).
  Shovel test values (model frame of the settler, scale
  1.2; origin of SLOT_HandR, the shovel's two ends along its long axis):
  | clip t (s) | SLOT_HandR origin | quaternion (x, y, z, w) | shovel end - | shovel end + |
  |---|---|---|---|---|
  | 0.00 | (0.697, 0.941, -0.025) | (-0.5868, 0.6173, -0.0573, 0.5209) | (0.028, 0.447, 0.348) | (1.265, 1.389, -0.438) |
  | 0.30 | (0.088, 0.673, 0.432) | (0.1178, 0.7926, 0.5334, 0.2706) | (-0.437, -0.014, 0.721) | (0.530, 1.348, 0.226) |
  | 2.35 | (0.328, 0.914, 0.358) | (0.2559, 0.9232, 0.1541, 0.2419) | (-0.148, 0.810, 1.128) | (0.767, 1.090, -0.328) |
  | 5.28 | (0.079, 0.643, 0.434) | (0.1138, 0.7990, 0.5412, 0.2362) | (-0.396, -0.071, 0.741) | (0.477, 1.342, 0.214) |
  | 8.73 | (0.002, 0.330, 0.920) | (0.2440, 0.9077, 0.1932, 0.2814) | (-0.549, 0.179, 1.630) | (0.506, 0.550, 0.294) |
  At the digs the blade end (end -) is low and forward (y about 0 at 0.30 and
  5.28 s, in the ground in front of the feet); at 8.73 s the settler bends
  forward (origin y 0.33, end - 1.63 forward). Confidence: high (computed with
  the GFX-OBJ-7 chain from the shipped files).
- **GFX-ANIM-3.8e Woodcutter from the chop to the walk home (F7).** With W the
  tick his work action starts (he stands at the fine approach point, half a
  field short of the tree, facing it, playing WorkLoop0Axe from clip time 0):
  | tick | logic | drawn |
  |---|---|---|
  | W | work starts (55 ticks) | chop clip from 0 |
  | W + 7, W + 16, W + 25 | | the three axe chops (clip 1.24, 2.88, 4.56 s) |
  | W + 30 (3.0 s of game time) | | tree starts to fall (3.8a), fall sound |
  | W + 53 | | tree lies (24th fall tick; drawn at about 81.6 degrees) |
  | W + 55 | harvest: tree marked for removal (counter 2) | clip at 9.9 s, still standing at the fine point; **the lying tree starts to sink** (3.8c), target 0.10 h |
  | W + 55 .. W + 59 | drop action (4 ticks) | PutDown clip (request code 2) |
  | W + 59 .. W + 63 | pickup action (4 ticks), the log is now carried | Pickup clip (code 3); the log appears at SLOT_HandR (GFX-OBJ-7) |
  | W + 56 .. W + 72 | | the tree sinks 0.05 h per tick (targets 0.15 .. 0.95 h); the trunk is gone within a few ticks, the crown about W + 65 / W + 66 |
  | W + 63 | walk home starts: **a fine walk back** from his stand sample to the lattice point of the field F where his walk to the tree stopped (`TRN-free-walk.md` TRN-FREE-5.6); no position change in this tick; **when F is the hut's flag** (the tree next to it, no field walked to it) he stands in W + 63 and the fine walk back is ordered in W + 64 (`ECO-outdoor-workers.md` ECO-OUT-3.8) | the walk clip from 0 (the view is already walking; corrected 2026-10-25 evening, measured E-trace-010: woodcutter 90392 at counter 2318 and hunter 90379 at 2206, progress 0 and view_base 1 (walk), earlier read as "still standing") |
  | W + 64 .. W + 63 + n | fine walk, 0.09 sample steps per tick on level ground; n = 12, 23, 34, 45 ticks for 1 to 4 sample steps | walks (WalkWood, with the log) along the lattice line, 0.135 world units per tick, turning toward F at 25.2 degrees per tick (4.2) |
  | W + 73 (harvest + 18) | deposit deleted | the rest of the tree (0.95 h down, already invisible) is removed |
  | W + 63 + n | arrives on F's lattice point (excess progress dropped); the free walk to the hut's flag is ordered from F | WalkWood in place (3.1a): the free walk counts as moving from its order, so no stand between the two walks |
  | W + 64 + n on | free walk, 0.045 fields per tick on level ground (TRN-MOVE-2) | walks on at twice the speed, from the point where the fine walk ended |
  **There is no jump and no slide**: every target the motion of 4.1 draws lies on
  his path (T16, T17). (**Corrected Run D, E-gfx-015: Run C and play-test 5 had the
  walk home start directly from the field the stand sample truncates to, drawn as
  a 1.5 to 4.5 unit one-tick slide. That reading took the walk-to-field call
  (which does reset a settler in fine mode to his current field) for the first
  step; the walk-home state first runs the fine walk back, which ends exactly on
  a field's point, so the reset there moves nothing.**) Confidence: high for the
  sequence (E-gfx-015), medium for the exact W offsets of the drop /
  pickup (ECO-OUT-3.7).

- **GFX-ANIM-3.8f Builder hammering (play test 10, F8; E-gfx-018).** The
  clip is `WorkLoop0Constructor` = `constructor_hammer.GR2` (the Roman,
  African and Asian builders use their own model files with this clip), 9.600 s
  of clip time with ten hammer sounds at clip times 0.48, 1.28, 1.97, 2.71,
  3.68, 4.68, 5.47, 6.54, 7.88 and 9.00. At `globalSpeed` 1.8 it lasts
  **5.333 s of real time (53.3 ticks at game speed 1)** and plays **once**
  (request clips have loop count 1, 2.2): it does **not** loop for the 55
  ticks; when it has run out the pose holds its last frame until the request
  ends.
  (a) **One work step, tick by tick** (ECO-HQ-4.6a, 4.6b; A = the tick the
  walk to the position ends, or T + 1 when he already stands on it): A: the
  work request (code 4, 55 ticks) is set and the first decrement made; the view
  switches to the hammer clip at time 0 in that tick, first hammer sound
  0.267 s later. A .. A + 53: the clip runs (its last hammer sound at 5.0 s,
  between A + 50 and A + 51). A + 53.3: the clip ends; the builder holds its
  last frame through A + 54 (about 0.17 s). A + 54: the 55th decrement.
  A + 55: the request has run out and the walk to the next position is
  ordered but not moved (4.6b): this tick shows Walk in place (3.1a; the
  request has run out and the order counts as moving).
  A + 56: he walks. After position 3 the unit ends in A + 55 instead (Stand),
  the next unit's material is taken in A + 56 (Stand), the walk to position 0
  is ordered in A + 57 (Walk in place, as above) and moves from
  A + 58.
  (b) **When the original's builder stands still.** Only (1) the at most
  one-tick pause of (a) between steps and the two or three between units, (2) the
  last-frame hold of about 0.17 s at the end of each step, and (3) his arrival
  tick on the flag (ECO-HQ-4.6b step 1). **Builders never share a hammer
  pose** (corrected 2026-10-18, Q-220): the hammer clip plays once, so by 2.4
  (b) each builder gets a pose of his own at the start of each step, at time
  0, whatever other builders (of his model file, at his site or another, of
  any player) are doing. Any number of builders working at once hammer
  exactly as one builder alone (T19a), each in his own phase. (The earlier
  reading, that a builder joins another's running or run-out hammer pose, is
  withdrawn.) With no material he does not stand: he repeats the approach
  walk out and back to the flag (ECO-HQ-4.6b step 3), in the Walk clip. A
  builder that stays in one pose for many seconds is not the original's
  behaviour.
  Confidence: high (E-gfx-018, E-gfx-023, the step's duration = the construct time;
  `settler_animations.xml`; clip length read from the GR2 file).

### GFX-ANIM-4 Settlers: moving between fields

- **GFX-ANIM-4.1 Tick position.** Each tick the target position = from-field position
  + (to-field position - from) x step progress (x, y and height); the heading target
  = the heading of the step direction (GFX-VIEW-4.2 table). Per frame the drawn
  position interpolates linearly from the previous tick's target to the new one over
  the tick length: t = 1 - (T_tick - now) x 10 per second.
- **GFX-ANIM-4.1a Logic jumps (2026-10-17).** The motion keeps two targets:
  the previous tick's and the current one. In every tick of a settler that
  is moving (its step progress >= 0) the current target becomes the previous
  one and a new target is computed from the logic position: from the field
  coordinates of the step, or from the fine samples while the settler is on
  a fine walk (TRN-FREE-5). In a tick in which the settler stands, the new
  target is the last target of its movement (including the evading offset of
  4.3), so a logic position change while standing is not drawn until he
  moves again. Each frame draws the linear blend of 4.1 between the two
  targets. There is no other smoothing and no distance limit: **any jump of
  the logic position, however large, is drawn as one straight slide lasting
  exactly one tick (0.1 s at game speed 1)**, after which the settler
  follows the logic again. The heading follows 4.2 (at most 0.44 rad per
  tick) across such a jump. In the normal work cycles no such jump occurs:
  a field walk ordered while a settler stands off a field's point would reset
  him to the field his sample truncates to (MAP-GRID-4.3), but the woodcutter,
  stonemason and forester first walk back on the lattice to a field's point
  (TRN-FREE-5.6, 3.8e) and the builder ends his site walks on the flag
  field's sample (ECO-HQ-4.6b), so the reset never moves them. (**Corrected
  Run D, E-gfx-015: the earlier "case" of a 1.5 to 4.5 unit slide at the start
  of the woodcutter's walk home does not exist.**) Confidence: high
  (E-pt5-001; E-pt2-001;
  E-gfx-015).
- **GFX-ANIM-4.2 Heading.** The drawn heading eases toward the target by
  0.43982297 rad per tick (25.2 deg), the shorter way round, and is interpolated per
  frame like the position.
- **GFX-ANIM-4.3 Evading.** Two settlers heading toward each other (direction dot
  product <= 0) within distance evadeStart x scale (2.0 x 1.2) get a sideways offset
  of (1 - d^2 / R^2) x scale x evadeRadiusScale (split half each when both move,
  sign by the side they already lean to). The offset decays 0.75 units per second (once per tick by 0.75 x the tick length, 0.075 a tick; E-gfx-041, Q-231 d)
  toward 0 and is applied along the settler's side axis.
  **Who takes the offset (2026-11-16 day, Q-207 g; E-gfx-073, code reading).** For a pair of settlers
  closer than R = evadeStart x scale, with f = (1 - d^2 / R^2) x scale x evadeRadiusScale and "lean"
  each settler's current offset (positive or negative along its side axis):
  - **one walks, the other stands:** only the walker gets an offset, the full f, positive when his
    own lean is 0 or positive, negative when it is negative; the standing one gets none, whatever
    the directions (high);
  - **both walk toward each other** (their headings' dot product 0 or below): each gets f / 2, both
    with the same sign: negative when the sum of their two leans is negative, else positive. As they
    face opposite ways, the same sign moves them to opposite sides, so they pass (high);
  - **both walk the same way** (dot product above 0): no offset (high);
  - neither walks: no offset.
  The offset is added to the one the settler has (several pairs add up) and decays as above. Only
  the settler renderer runs this, so settlers (carriers, workers, soldiers, donkeys drawn as settlers)
  evade each other and animals take no part (medium for the donkey). Which way "positive" points on
  the side axis (the settler's right or left) was not read (medium: right). The renderer has a
  second, forced call of the same test (both settlers treated as walking, plus a rule that flips the
  smaller lean's sign when both offsets would go the same way); its caller was not read.
- **GFX-ANIM-4.4 LOD.** The mesh LOD is the first i with squared camera distance <
  lod_i Range^2 (40, 50, 60 from settler_config, else LOD 3), clamped to the model's
  mesh count - 1 (mesh, mesh1, mesh2, mesh3).

### GFX-ANIM-5 Animals

- **GFX-ANIM-5.1 Walk and stand (play-test 2, chunk 3).** An animal has two
  looping clips, `Stand` and `Walk` (animals.xml; the idle files hold the
  grazing: deer 9.33 s with head-down phases, rabbit 3.0 s, elk 2.0 s; the
  run files 0.9 s for deer and elk, the rabbit's 1.03 s jump), scale 1.5
  (deer, elk) and 2.0 (rabbit). In the tick in which the animal's step
  progress becomes >= 0 (its walk starts, ECO-FOOD-5.11 / 5.12) the visual
  switches to `Walk`; in the tick the progress is < 0 again (the walk ended)
  it switches to `Stand`; nothing else changes the clip. The clips are
  pooled poses (2.4): all animals of one kind playing the same clip are in
  phase, an animal that stops joins the common `Stand` pose at its current
  time, and the clip runs at the frame clock (2.5) regardless of the step
  speed, so feet are not synchronised with the ground. Position: at the
  tick, the current field's world position while standing, else from-field
  + (to-field - from) x progress (4.1), interpolated per frame; heading:
  while walking the target is the step direction's heading (4.2, eased 25.2
  deg per tick, shorter way); while standing the heading keeps its last
  value. Confidence: high (E-eco-016; durations from the GR2 files).
  **Clip rate against the walk (2026-11-08 b, Q-200):** nothing adapts the run clip to the
  walk: `animals.xml` has no rate or stride, and the clip plays at `globalSpeed` x the game step
  like every clip (2.1, the animals' pose pools take the same step). The step itself moves
  `an_setMovementSpeed` of a field per tick, measured in E-trace-017 (every animal step, 34,857
  pairs of rows 10 ticks apart): deer and elk 0.045, rabbit 0.07 of a field (6 units) per tick,
  i.e. 2.7 and 4.2 units a second. So the deer's 0.5 s gallop covers 1.35 units: the original's
  deer gallops quickly over a short distance, as ours without a stand-in would. Confidence: high
  for the speeds (measured), medium that no other code scales the animal clip.
- **GFX-ANIM-5.2** When the animal enters its dying state it plays Dying once on an
  instance of its own and keeps the last frame until the logic removes it. The dying
  state begins when the hunter arrives at the animal, i.e. at the start of the hunting
  action (ECO-FOOD-5.3); the animal is removed when that action ends (high, E-gfx-008).
  Re-read 2026-11-24 b (Q-181 b, stand-ins row 10; E-eco-057, code reading, high): the hunt
  step stops the animal, sets its dying state and starts the 55-tick action **in one update**,
  the one in which the hunter's approach ends with the animal still there; nothing marks the
  animal earlier (not when it is reserved, not while he walks) and nothing later (the spear's
  moment in his clip is a view event only). So Dying starts in the action's first tick, about
  5.5 s before the animal is removed, and an animal gone before the hunter arrives never plays
  Dying (ECO-FOOD-5.4). T10a.

### GFX-ANIM-6 Buildings

- **GFX-ANIM-6.1 Door.** When a settler steps onto the building's door field from
  outside (entering) and its step progress exceeds the type's `enterDistance`, or
  leaves the door field and the progress exceeds `leaveDistance`, DoorOpen plays once
  (door sound) and DoorClose plays once (close sound) 0.6 s later. No building type in
  the data sets these attributes, so every building uses the loader defaults
  `enterDistance` = 0.3 and `leaveDistance` = 0.0 (the door opens at once when a
  settler starts leaving) (high, E-gfx-008).
- **GFX-ANIM-6.1a Door in detail (play test 10, F32, OBS-34; E-gfx-020).**
  (a) **What moves.** The door is part of the building model, moved by
  bones: `DoorOpen` and `DoorClose` (`buildings.xml` children, files
  `<model dir>\door_open.GR2` / `door_close.GR2`) are skeletal clips of the
  building's own skeleton; in the woodcutter's hut they animate the bone
  `door` (with its end bone `ee_door`); the Roman headquarters' clips move
  two doors (`Door1`, `Door2`). 87 of the 99 building types have the two
  clips; the 12 mines (iron, gold, coal, stone for each tribe) have none and
  never animate a door. Clip lengths (clip time): 0.400 s for 149 of the 168
  files, 0.333 s for the fisher's hut (African), the catapults, the Asian
  stonemason, hunter's hut, well and slaughterhouse and the Roman bakery,
  1.000 s for the Roman fortress's door_open; at `globalSpeed` 1.8 they last
  0.222 s, 0.185 s and 0.556 s of real time. Each plays once and holds its
  last frame (open, resp. closed).
  **Track groups (play test 11; read from the clip files).** A door
  clip can hold more than one track group, and the group to play is the one
  **whose name equals the building model's name** (`root` for every building
  model), not the first group in the file. The Roman woodcutter's hut,
  forester's hut and stone pit clips (`PB_R_WoodcutterHut01`,
  `PB_R_ForesterHut01`, `PB_R_Stonemason01`, `door_open.GR2` and
  `door_close.GR2`) carry **two** groups: first a group `GROUND` with one
  track `GROUND` (the ground decal object of the source scene, no bone of the
  model), then the group `root` with the tracks `door`, `ee_door`, `root` (the
  stone pit also `smoke00`). The Roman sawmill's clips carry one group `root`
  with the tracks `door`, `door_ee`. The model skeletons are `root` with the
  bones `root`, `door`, `ee_door` (hut; the stone pit also `smoke00`) and the
  sawmill's 25 bones with `door`, `door_ee`. So a player that takes the first
  group animates nothing in the three huts and works for the sawmill; a
  player that binds tracks to bones by name through the model's group works
  for all. The end bone name differs between models (`ee_door` / `door_ee`)
  and is not needed: only `door` moves. Nothing in the building type data
  gates the door: all four types list both clips in `buildings.xml`, the
  trigger (b) is the same for every finished building, and the three huts'
  workers (who work outside) open the door on every walk out and in.
  Confidence: high.
  (b) **Trigger.** In each logic tick a walking settler (not on a fine
  walk) whose current step goes **from the building's door flag field to
  the building** (entering) with step progress above `enterDistance` (0.3),
  or **from the building to the flag** (leaving) with progress above
  `leaveDistance` (0.0), calls "open" on the building's view. "Open" sets the
  close time to now + 0.6 s (real time) every time it is called; only if the
  door is not open yet does it stop the building's running clip, start
  `DoorOpen` at time 0 and play the `dooropen` sound (and only for a
  finished building: a site has no door clip; the open flag is still set).
  (c) **Closing.** Each frame, once the close time has passed: for a
  finished building the running clip is stopped, `DoorClose` starts and the
  `doorclose` sound plays; the door counts as closed. Because every tick of
  the step pushes the close time on, **the door stays open for the whole
  flag-to-building (or building-to-flag) step and closes 0.6 s after the
  last tick of that step**, so a worker is seen in the open doorway while he
  walks in or out (`woodcutter-door-open.jpg`).
  (d) **Timeline, a woodcutter leaving at game speed 1** (step building ->
  flag of n ticks starting in tick t): door starts opening in tick t (first
  tick with progress > 0), fully open 0.222 s later, stays open; the last
  tick of the step is t + n - 1; DoorClose starts 0.6 s after that tick's
  time, closed 0.222 s later. Entering: the door starts opening in the first
  tick whose progress exceeds 0.3, i.e. about a third of the way from the
  flag, and closes 0.6 s after the step's last tick (when he has reached the
  building and is hidden inside).
  Confidence: high (E-gfx-020;
  clip lengths and bone names read from the GR2 files).
- **GFX-ANIM-6.2 Working.** Whenever a worker inside the building (any profession
  except forester, fisher, hunter and farmer, whose work is outside) issues a work
  request (GFX-ANIM-3.8, normally the production step's WorkLoop0) and the building
  is not already playing Working, the building plays Working once for the request's
  duration; the clip is stopped when that duration expires. The building's logic
  state number is not consulted (high, E-gfx-008).
- **GFX-ANIM-6.3 Production animals (corrected 2026-10-24, Q-268).** The rule is
  `GFX-objects.md` GFX-OBJ-1.6a: the `Production` block's animals (donkey, pig) are
  made with the building's view, each starts its one clip once and the clip loops
  forever (loop count 0, the convention of the constants table), and how many of them
  show follows the building's productivity. They are not removed when production
  stops or the worker leaves: the shown ones stay as they were. The earlier text
  ("shown while the logic state is 4 or 5, clips playing once, otherwise removed") read
  a different object: the play-once instance made and dropped with a settler's work
  state 4 / 5 belongs to the settler's view, not to the building. Confidence: high
  (E-gfx-031, loop count argument 0; E-001 for the settler's
  instance, loop count 1).
- **GFX-ANIM-6.4 Rotation attribute.** (Q-083 answered: `GFX-world-view.md` GFX-VIEW-4.3.) The type loader reads an integer `heading`
  attribute for the building's rotation (GFX-VIEW-4.3); no building in
  `buildings.xml` sets it, so all buildings use the loader default, which is -30
  (degrees; E-gfx-008). Every building is drawn turned by it
  (GFX-VIEW-4.3).

### GFX-ANIM-7 Ships

- **GFX-ANIM-7.1** A ship plays Walk while moving and Stand while resting (both loop).
- **GFX-ANIM-7.2 Bobbing.** With phase p (advancing by 2 pi x game time x 0.25 (TRACE-5.3, aligned 2026-11-08 b, Q-231 c), i.e.
  one full turn every 4 s; each ship offset by 0.89759797 x its creation index): pitch = (sin(f1 p +
  0.2) + 1) x a1, roll = (sin(f2 p + 0.4) + 1) x a2 with (f1, a1, f2, a2) = (0.66,
  0.01, 0.4, 0.02) resting and (2.5, 0.04, 1.7, 0.08) moving; height = water height
  + (sin p + 1) x 0.06. A moving ship shows the ShipFX wake effect.
- **GFX-ANIM-7.2a Bobbing axes and order (answered 2026-11-13, Q-181 e; E-gfx-066, code
  reading and the model file).** The ship's turn is a yaw-pitch-roll in the game's own
  axes (`GFX-granny-format.md` GFX-GR2-7.2: model right +z, up +y, back +x). The game
  uses the same yaw-pitch-roll for the sun's direction.
  - **Axes.** The ship model's length lies on its x axis, with the bow at -x: the file's
    +y (art axes right +x, up +z, back -y; the hull spans file y -4.23 .. 5.5 and
    x -4.31 .. 4.31). Its beam lies on z.
  - **The two values.** 7.2's first value ("pitch": f1, a1) turns the ship about its
    **length** (model x). A positive value moves the model's +z side (the file's +x,
    starboard) down. The second value ("roll": f2, a2) turns it about its **beam**
    (model z). A positive value moves +x (the stern) up and the bow down. Both values
    lie in 0 .. 2a, so the ship heels to starboard by up to 2 a1 and dips its bow by up
    to 2 a2: resting 0.02 and 0.04 rad, moving 0.08 and 0.16 rad.
  - **Order.** First the second value's turn about z, then the first value's turn about
    x, then the yaw about the vertical: R = Ry(yaw) Rx(first) Rz(second) for column
    vectors. A turn about x by a takes +z towards -y. A turn about z takes +x towards
    +y. The yaw takes +z towards +x (GFX-VIEW-4.1).
  - **Yaw.** Heading + 90 degrees. This turns the bow (-x) to where a settler with the
    same heading faces (GFX-OBJ-1.8).
  - **Moving.** A ship counts as moving while a movement step is in progress (as a
    walking settler); otherwise it is resting.
  - **Phase.** The frame's turn and height use the phase before the frame's advance.

  Confidence: high for the order, the axes and the values (code and model file);
  medium for the names starboard and bow (taken from the art axes; not seen running).

### GFX-ANIM-8 Skinning (Q-048)

- **GFX-ANIM-8.1 Vertices.** Each vertex keeps 4 bone indices and 4 weights; the
  fourth weight is 1 - (w0 + w1 + w2) from the three stored bytes / 255. Indices
  whose weight is <= 0 are set to 0.
- **GFX-ANIM-8.2 Palette.** Per mesh the palette is the list of skeleton bones
  referenced by any vertex, in skeleton order, renumbered 0..n-1; vertex indices are
  remapped to it. At draw time the palette's composite matrices (one per palette
  bone, from the shared pose) are uploaded as the shader's instance-matrix array.
- **GFX-ANIM-8.3 Limit.** The shader array holds 50 matrices (MAX_BONES). No code
  splits a mesh; a mesh referencing more than 50 bones (PB_R_Smelter01, 51 in the
  GR2 survey) exceeds the array on the hardware path and the upload fails with a
  logged error (medium). **What is drawn (2026-11-08 b, Q-090; E-gfx-041):** the
  upload is the effect's matrix-array setter; on failure the call only logs (no status goes back)
  and the mesh is drawn anyway, so its vertices take the matrices last set successfully on that
  effect, i.e. those of the house drawn before it in the same pass (stale matrices: parts placed
  as in that other house; no fallback, no skip). Medium: the stale values are the D3DX library's
  behaviour for a too-long array, not in the game's code. An implementation that has no 50-matrix
  limit draws the smelter correctly; that is a deliberate deviation. The software path (no shader
  support) skins on the CPU with up to the mesh's weight count per vertex and has no
  palette limit.
- **GFX-ANIM-8.4 Techniques.** The character shaders use 4 weights in the Default /
  NoShadow techniques and 2 weights (w1 = 1 - w0) in the Low techniques.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| globalSpeed | 1.8 | animation seconds per real second | high | graphics.xml; E-001 |
| base animation names | 10 settler / 3 animal | | high | E-001 |
| loop count | 0 forever (base 0..3), 1 once (others) | | high | E-001 |
| random idle delay | minDelay 5 + r x (8 - 5) + clip duration | s | high (formula), data from XML | E-001 |
| random weights | round(p / p_min + 0.5) | | medium (ties open) | E-001 |
| pose cleanup | every 10 s when > 5 poses | | high | E-001 |
| tick interpolation rate | 10 | per s | high | E-001 |
| heading ease | 0.43982297 | rad per tick | high | E-001 |
| evade | evadeStart 2.0, evadeRadiusScale 1.0, scale 1.2; decay 0.75/s | | high (reads), medium (formula) | E-001, settler_config.xml |
| LOD ranges | 40, 50, 60 | world units | high | settler_config.xml; E-001 |
| footstep timer | 0.5 / globalSpeed; counter +5 / -3, cap 100, range d^2 <= 50 | s | high | E-001 |
| door close delay | 0.6 | s | high | E-001 |
| dying sink | 0.35 | units per s | high | E-001 |
| tree fall trigger | 3.0 | s of game time since the work action started | high | E-gfx-014 |
| tree fall step | rate / 10 per tick; rate (pi/2) / 36, x 1.2 per tick | rad | high | E-gfx-014 |
| deposit sink | 0.05 x removal counter x box height | world units | high | E-gfx-014 |
| ship bobbing | see 7.2 | | high | E-001 |
| ship phase offset | 0.89759797 | rad | high | E-001 |
| MAX_BONES | 50 | matrices | high | shared.inc; E-001 |
| weights per vertex | 4 (3 stored + derived) | | high | E-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | Roman smith (profession Smith, tribe 0) at work, request 4 | resolve clip | WorkLoop0Smith (tribes mask 5 includes bit 0); for an African smith (tribe 1) the mask excludes it -> falls to WorkLoop0 base clip |
| T2 | carrier with Wood walking | resolve | WalkWood; standing -> StandWood; Hit requested -> HitWood if defined else Hit |
| T3 | settler walks a step of 0.1 s with progress 0.3 at the tick | frame half way to the next tick | drawn position = from + (to - from) x 0.3 blended 50 % from the previous tick's point |
| T4 | heading target 90 deg from 0 | 4 ticks | drawn heading 25.2, 50.4, 75.6, 90 deg |
| T5 | standing settler, the shipped random set (probabilities 0.01, 0.2, 0.1; GFX-ANIM-1.4: weights 1, 20, 10) | bag build | 123 slots (3 x 1 + 4 x 20 + 4 x 10), each clip of probability 0.2 appears 20 times; after 123 picks a reshuffle (corrected 2026-11-09: the row said weights 2 / 21 / 11, sum 134) |
| T6 | two carriers with the same model both walking | poses | one shared pose, both in phase; one picks up Wood -> moves to the WalkWood pose, the other keeps the Walk pose |
| T7 | settler steps onto a door field, enterDistance 0.5 | progress 0.6 | DoorOpen plays once, DoorClose 0.6 s later |
| T7b | woodcutter leaves his finished hut, building -> flag step of 17 ticks starting in tick 100 | door | DoorOpen starts in tick 100 (0.222 s long), open through tick 116, DoorClose starts 0.6 s after tick 116, closed 0.222 s later; one dooropen and one doorclose sound (6.1a) |
| T7c | a carrier walks into a woodcutter site with a plank | door | no door clip and no sound (a site has no door animation), 6.1a b |
| T7d | a worker leaves an iron mine | door | nothing: mines have no door clips |
| T7a | two deer standing, one starts a 2-field walk | ticks 1..45 | it plays Walk from tick 1, heading turns toward the step direction by 25.2 deg per tick; the other deer's Stand pose is unchanged; at tick 46 the walker plays Stand again, joining the other deer's pose in phase |
| T8 | ship moving | frame with p = 0 | pitch (sin 0.2 + 1) x 0.04, roll (sin 0.4 + 1) x 0.08, height = water + 0.06 |
| T9 | mesh with bones {0, 3, 7} referenced | palette | 3 matrices uploaded, vertex indices 0, 1, 2 |
| T10 | animal dies | tick | Dying plays once on an unshared instance; Stand/Walk poses untouched |
| T10a | 5.2: a hunter's approach to a reserved deer ends in tick a (the animal still on the map) | the deer's view from a - 1 to a + 56 | a - 1: not Dying (Stand or Walk as before); from a: Dying, once, on its own instance, its last frame held; the deer removed at the end of the 55-tick action (ECO-FOOD-5.3), Dying never restarted. A deer that is gone before a: no Dying at all |
| T11a | 3.8c1 (Q-1087): saves of one woodcutter's work at a tree, his action begun in tick W, saved at W + 20, W + 40, and 5 ticks after the harvest (W + 60); each loaded and run on | the tree | saved at W + 20: falls from W + 30 as in T11; saved at W + 40: upright at the load, falls from the first tick after it (angle 0.0044 then, the 3.8a sequence) toward the woodcutter's left, starts sinking at the harvest W + 55 with the box of the pose then; saved at W + 60: upright, never falls, never sinks, gone at once 13 ticks after the load (counter 20) |
| T11 | woodcutter's work action (55 ticks) started in tick W | tree | no fall up to W + 29; in W + 30 (3.0 s elapsed) it starts turning about the horizontal axis tree -> woodcutter; angle after the j-th fall tick = 0.0043633 (1.2^j - 1) / 0.2 rad: 0.0044 (W + 30), 0.1727 (W + 41), 1.1826 (W + 51), 1.4235 (W + 52); in W + 53 (1.7125) it stops and stays at the last drawn angle (just under 1.4235) (3.8a, corrected Run D) |
| T11a | same; lying birch (RS_TreeBirch01, size 1) with h = 7.413, ground y0 = 10.0 | removal ticks | at W + 55 (harvest) drawn y goes 10.0 -> 9.259 over the tick (target 0.10 h); then targets 10 - 0.05 (k + 2) h: W + 56 8.888, W + 60 7.405, W + 65 5.552, W + 72 2.958; at W + 73 the model is gone; x, z and the rotation never change (3.8c) |
| T11b | a stone pile's last stone is taken | model | sinks by 0.05 h per tick the same way (h = its box height) and is removed 18 ticks after the take (3.8c) |
| T13 | woodcutter walks out (no ware) | clip and tool | WalkAxe (settler_walk_pickaxe) with the axe drawn at SLOT_HandR (GFX-OBJ-7.8 C4-C6) |
| T14 | stonemason walks out, then quarries | clip | WalkPickaxe, then WorkLoop0Stonecutter; the pickaxe stays drawn in both |
| T15 | carrier on a waterway, moving / still | clip | WorkLoop1Boat (boat_walk) / WorkLoop0Boat (boat_stand), looping |
| T12 | woodcutter A ends his chop clip, B starts chopping 2 ticks later with the same model | pose | B's chop clip starts at time 0 in a pose of his own (play-once clips are never shared, 2.4 b); it may be A's old pose, restarted |
| T16 | level ground, even row 10: tree at sample (40, 40) (field (10, 10)'s point), the woodcutter came from field (9, 10) (sample (36, 40)) and stands at sample (39, 40); his work started in tick W; the hut's flag lies west along the row | drawn x relative to field (9, 10)'s point (world units, 1.5 per sample) | W + 25 .. W + 63: 4.5 (standing; chops, drop, pickup); W + 64: 4.365, W + 74: 3.015 (first sample step done in W + 75: 2.88), W + 85: 1.53, W + 96: 0.045; W + 97: 0 (on F = (9, 10), 34 ticks for 3 steps), the free walk to the flag is ordered; W + 98: -0.27, W + 119: -5.94, W + 120: -6.21 (on (8, 10) at -6, carry 0.035), W + 141: -11.88, W + 142: -12.15 (on (7, 10)); no frame ever shows a jump; his heading turns from +x to -x over 8 ticks from W + 64 (3.8e, 4.1, 4.2; TRN-FREE-5.6) |
| T17 | same, but the tree has its random offset one sample east (tree at (41, 40)), so he stands at (40, 40) after a 4-step approach from (36, 40) | walk home | fine walk back 4 sample steps (45 ticks): from W + 64 to W + 108, arriving on (9, 10)'s point in W + 108; free walk from W + 109. His stand sample is (10, 10)'s own point (the field it truncates to), yet the walk home does not start from (10, 10): it goes back to the field where the walk to the tree stopped |
| T18 | settler standing; its logic field is changed (e.g. set into a building) | draw | no movement until his next moving tick; then a one-tick slide from the old drawn spot (4.1a) |
| T19 | builder, one work unit | requests | four WorkLoop0 requests of 55 ticks, one per step, each starting when he reaches the step's position (3.8) |
| T19a | builder alone at a site, walk to position 1 ends in tick 500 | clip per tick | 500 to 553: WorkLoop0Constructor from time 0 (hammer sounds at 500 + 2.67, 7.11, 10.94, 15.06, 20.44, 26.0, 30.39, 36.33, 43.78, 50.0 ticks); 553.3 to 554 last frame held; 555 Stand (or Walk in place, Q-216 d); 556 walking (3.8f) |
| T19b | two Roman builders at two sites (or at one site, or of two players); B's step starts 20 ticks after A's; a third builder C starts at A + 54 | clips of A, B, C | each as T19a from his own start tick: A hammers A .. A + 53.3, holds to A + 54; B hammers from B = A + 20 at clip time 0 (first hammer sound at A + 22.67) to A + 73.3, holds to A + 74; C hammers from A + 54 at clip time 0 to A + 107.3. No builder ever shows another's pose or a frozen frame for more than 0.17 s of a step (2.4 b, 3.8f b) |
| T19c | Roman builder pool in order P1 (Stand, used by a standing builder D), P2 (hammer, A), P3 (hammer, B); A's step ends, then in the same tick, after A, builder C starts a step | poses | A leaves P2 (now without user) and takes P1 (first pose that is free or plays Stand), joining D's Stand at its current time; C takes P2, restarted at clip time 0 with the hammer clip; the pool stays at 3 poses. Had C been handled before A, C would get a new pose P4 (2.4 a to c) |
| T19d | two idle carriers of one model file stand, X since 3 s, Y starts standing now | Y's Stand pose | Y joins X's Stand pose at its current time (in phase with X), unless a pose without a user comes earlier in the pool order, in which case Y restarts that one at time 0 (2.4 a) |
| T19e | woodcutter of 3.8e with n = 12: the fine walk back ends on F's point in tick W + 75, where the free walk to the flag is ordered | clip in W + 75 | WalkWood, drawn at F's point (no Stand tick); W + 76 walks on. Same for a builder in A + 55: Walk in place, then walks from A + 56 |
| T20 | forester reaches his spot in tick P, speed 1 | sounds and sapling | digging sounds at P + 0.17 s, P + 1.31 s, P + 2.93 s, P + 4.85 s; "plant grows" at P + 4.06 s; StoneDust at P; the sapling is invisible until tick P + 55 and is drawn from then at scale (0.3, 0.1, 0.3), full width at P + 55 + 260, unscaled from P + 55 + 1001 (3.8d) |
| T21 | Roman woodcutter's hut, worker walking out | door | the `door` bone turns by the `root` group of `door_open.GR2` (second group of the file); the `GROUND` group is ignored; the sawmill's single-group clip plays the same way (6.1a a) |
| T22 | 7.2a: a resting ship, phase p = 0 this frame | its turn | the bow dips by (sin 0.4 + 1) x 0.02 = 0.027788 rad (a turn about the model's z axis, taking +x towards +y), then the ship heels to starboard by (sin 0.2 + 1) x 0.01 = 0.011987 rad (about x, taking +z towards -y), then yaw = heading + 90 degrees; height = water + 0.06 |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | pose clocks in the follow tables and the pause and speed log | GFX-ANIM-2.1, 2.5 clock |
| E-001 | code reading | S2DNG.exe; also filed as E-gfx-006 | animation set, pool, settler / animal / building / ship visuals, skinning |
| E-002 | data | `data/character/settler_animations.xml`, `settler_config.xml`, `settings/animals.xml`, `buildings.xml`, `items.xml`, `graphics.xml`, `gfx/fx_20/shared.inc` | names, tribes masks, random set, constants |
| E-gfx-020 | code reading | `buildings.xml` DoorOpen / DoorClose, door GR2 durations and bones | door trigger, hold and close (6.1a) |
| E-gfx-023 | code reading | | pose pool: play-once clips private, looping clips shared (2.4, 3.8f; Q-220) |
| E-gfx-018 | code reading | `constructor_hammer.GR2` duration | builder hammer clip and step (3.8f) |
| E-pt5-001 | code reading | | logic jumps drawn over one tick; builder request per step |
| E-gfx-008 | code reading | S2DNG.exe | suffix lists, request codes per step, footsteps, door defaults, animal dying, building Working, ship phase rate |
| E-eco-057 | code reading | | 5.2: the hunt step sets the dying state in the update that starts the 55-tick action (Q-181 b) |
| E-gfx-013 | code reading | `work_shovel.GR2`, `woodcutter_chop.GR2`, `GO_Shovel01.GR2` | tree fall rate, growth scale, no sink, shake; shovel values |
| E-pt2-001 | code reading | | pose restart, tree fall (its tool-drawing reading is replaced by E-gfx-010) |
| E-gfx-014 | code reading | | fall trigger in game seconds, fall step / 10, deposit sink |
| E-gfx-015 | code reading | | walk home after a fine approach |
| E-gfx-066 | code reading and model file | `data/ships/PV_R_Ship01.GR2` | 7.2a: the ship's per-frame turn (yaw-pitch-roll, heading + 90 degrees, the phase), the hull's extents and art axes |
| E-gfx-010 | code reading | | lookup good (ware, else tool, boat), lists without base fill, carrier boat override, soldier kind without override |
| E-gfx-072 | code reading | (session 2026-11-16 day) | 2.3: when clip events are scheduled: the building's Working clip at each work start, the settlers' pose pool (a restarted pose schedules with its starter's target, a joiner none) (Q-186 f) |
| E-gfx-073 | code reading | (session 2026-11-16 day) | 4.3: the evade pair test: walker and stander, two walkers by heading, the shared sign (Q-207 g) |
| E-gfx-075 | code reading | (session 2026-11-16 day) | 3.1: the in-water test of the settler motion against the global water level (Q-180 g) |
| E-gfx-078 | code reading | (session 2026-11-24 c) | 3.8c1: the deposit's load and post-load (counter kept, only the shown event), the removal-start event's only callers (harvest, fallback), the woodcutter view's fall trigger (work action codes, job, elapsed from the restored action) (Q-1087) |
| data | clips | `data/buildings/PB_R_WoodcutterHut01/door_open.GR2`, `door_close.GR2` (and ForesterHut01, Stonemason01, Sawmill01): track groups and track names read by the investigation's GR2 reader; `settings/buildings.xml`; `character/settler_animations.xml` WorkLoop0Forester events; `scripts/properties/sounds.lua` | 6.1a track groups, 3.8d planting (play test 11) |

## Open questions

- Q-087 soldier fight clip selection (the override function it named belongs to
  the carrier's boat, 3.4); Q-088 weight rounding ties; Q-089 which
  settler kinds (0..5) correspond to which professions, and the hidden-flag semantics
  of 3.1; Q-090 visible result of a 51-bone mesh on the hardware path.
- Q-180 (d) to (i) and Q-181 (d): palette rows, swim test, builder / leveller
  drawing, production animal scale remain as the viewer's stand-ins; the carried-ware
  matrix (i) and the profession / good name (a) are GFX-OBJ-7 and 1.3 (E-gfx-010);
  (a), (b), (c) and Q-181 (a), (b), (c), (e) are answered above.
- Curve sampling is specified in `GFX-granny-format.md` GFX-GR2-7.5 / 7.6 (Q-179).
