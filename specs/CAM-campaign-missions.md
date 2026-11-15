# CAM: Campaign, mission scripts and tutorial

Status: draft (verified in the binary and the shipped scripts; the tutorial's per-step widget
checks are only partly extracted)
Owner question(s): plan 7.2; leftovers in Q-100 (b)
Game build: 11757

## Summary

A campaign mission is a map with a Lua script and region triggers. The script shows briefings,
locks buildings and grants start resources; the engine evaluates victory and defeat itself. The
ten missions unlock one after the other through a "completed" flag per map stored in the player's
settings. The tutorial's four lessons are maps with (optional) scripts; their step-by-step guidance
is engine driven, keyed by one global step index.

## Rules

### CAM-1 Mission scripts

- **CAM-1.1** When a map is started from the campaign folder (kind 1) or the tutorial folder (kind
  2), the engine loads `<folder>\<map name>.lua` into the same Lua 5.0 state that holds the
  properties scripts (LUA-PROP). Free-game maps have no script. A save of such a game loads the
  script again before the saved script variables (FMT-SAV-3.12) are restored.
  Covers (inventory): *campaign mission* from `maps/campaignMaps/*.lua`. Covers (inventory): *tutorial mission* from `maps/tutorialMaps/*.lua`.
- **CAM-1.2** Callbacks, called by name if defined: `onLoaded()` once after the map part is loaded
  (before the game runs), `onStarted()` when the session starts running, `onLogicTick(seconds)`
  every logic tick with the elapsed game time in whole seconds (so `seconds == 25` is true for the
  ten ticks of the 26th second; the scripts open the briefing on it and it opens once because the
  dialog is modal), `onTriggerIsActive(name)` for an active trigger (CAM-2.3).
- **CAM-1.3** The `menu` table (all return nothing):
  - `openBookDialog(key)`: opens the book dialog with page 1 = text `key` (keys from
    `briefings.txt`).
  - `addPage(key)`: appends a page to the open book dialog.
  - `addPicture(name)`: sets the picture of the current page to `data\gfx\<name>.dds`, drawn at
    (201, 30) with size 512 x 512 in the 1024 x 768 reference space.
  - `addDebriefing(key)`: appends a page to the debriefing shown after the mission.
  - `addExtro(key)`: stores a text key for the outro (unused by the shipped scripts).
  - `createLetter(key)`: puts a message titled "You have Mail!" with that text into the local
    player's message list.
  - `addResource(player, good, amount)`: adds `amount` of the good to the player's headquarters
    stock. `clearResources(player)`: empties it.
  - `lockBuilding(player, building)`: the building type cannot be chosen in that player's build
    menu for this mission.
  - `setPlayerColor(player, r, g, b)`: sets the player colour (floats 0..1, alpha 1).
  - `createBuilding(player, building, x, y)`: places the building, finished, at the field.
  - `createBuildingWithSettler(player, building, x, y, settler)`: the same, then creates the
    settler kind in it (a soldier kind occupies a military building).
  - `createShip(player, x, y)`: creates a ship at the field, loaded with 1 + 4 + 6 units of three
    goods (ids 0xdd2daebe, 0xcd6ec133, 0x55e952d3), switches its expedition mode on and explores
    around it (2026-11-05, E-ship-004: the same expedition switch as SHIP-5.5).
  - `createStreet(player, x1, y1, x2, y2)`: builds a road along the pathfinder route between the
    two flags.
  - `setLanguage`, `setVideoVoiceStream`: not used by the shipped scripts.
- **CAM-1.3b `createStreet` in detail (answered 2026-11-13, Q-187 c; E-cam-007, code
  reading).** `createStreet(player, x1, y1, x2, y2)` searches a land route for that player
  from (x1, y1) to (x2, y2), the same search as the road-building mode
  (`TRN-roads-flags.md` TRN-ROAD-2). It then builds in this order:
  1. If (x1, y1) holds no flag of the player, nothing is built.
  2. The end (x2, y2) gets a flag only when it lies on **another, existing road** and a
     flag may be placed there (the flag test of ECO-PLACE, as for a player's flag). That
     flag splits the road like any flag placed on a road (TRN-ROAD-4.2).
  3. If the end now holds a flag other than the start's, the road is built along the
     route: every field from the second on is added with the ground evened
     (TRN-ROAD-3.3a), and the road asks for a carrier like a road the player built.
     Otherwise nothing is built. In particular an end on a free field without a flag
     gets no flag and no road.

  The shipped calls: tutorial03 builds (51, 37) - (45, 37), then (51, 35) - (48, 37).
  (48, 37) is a field of the first road without a flag, so step 2 places a flag there,
  splitting the first road into (51, 37) - (48, 37) and (48, 37) - (45, 37), and the
  second road ends on it. tutorial03's (56, 28) - (60, 21) for player 1 and tutorial04's
  (103, 59) - (107, 54) join two flags. Confidence: high for the order and the tests
  (code); medium for what happens when no route is found (nothing is built; the empty
  route was not followed through).
- **CAM-1.3c The book dialog's layout (2026-11-14, Q-187 e; E-cam-008, code reading).** The book
  dialog is the game's standard dialog (centred; its title in `font_big_white`, colour (254, 239,
  124)), with the standard dialog's close button hidden and its title moved by (-100, +60) from the
  standard place (half the dialog's width, 14), so to (width / 2 - 100, 74). Inside it, in the
  dialog's own pixels:
  - the page text: a text box at (150, 120), 230 x 360, font `dark_system`, black;
  - the page picture (`addPicture`, 1.3) at (201, 30), 512 x 512;
  - **next page**: a 24 x 24 button `uiElements0` (40, 376) at (300, 430), hovered and pressed
    (104, 376), tooltip `!Next Page.`; **previous page**: (8, 376) at (210, 430), hovered and
    pressed (72, 376), tooltip `!Previous Page.`; both disabled when the dialog is made;
  - a page label at (267, 442), `dark_system`, centred, reading `-/-` until a page is shown;
  - `!CONTINUE`: a text button of the style of the Save Game dialog's `!Save` (UI-MENU-5.1) at
    (190, 465);
  - two strips `uiBriefing` (8, 568), 248 x 8, at (142, 100) and (142, 405).
  The dialog starts its music when made. The defeat screen (CAM-3.6) is built on this dialog: it
  hides CONTINUE, the two page buttons and the page label, adds `!EXIT MAP` (same style), made at
  (190, 487) and then moved to (190, 465), CONTINUE's place, and the `!DEFEAT` text at (260, 90) in
  `light_system_outlined` (3.6c). Confidence: high for the
  places, sizes, art and texts (code); still open: where the page label's text comes from. Test T18.
  **The frame (2026-11-15 night, Q-187 e; E-cam-012, code reading):** the book dialog's maker gives
  it the frame picture `uiBriefing` (0, 0, 728 x 568) (`gfx/uibriefing.dds`), so the dialog is 728 x
  568, drawn 1:1 with its top-left at the dialog's origin and centred on the screen (UI-MENU-1.9's
  rule). The victory book (the debriefing, 3.6) and the defeat screen are made on the same book
  dialog and have the same frame. Confidence: high. **Seen (2026-11-15 night, E-trace-028, campaign01's
  briefing at 2560 x 1440):** the scroll art fills the frame exactly at (916, 436) = ((W - 728) / 2,
  (H - 568) / 2); every place above holds in the picture: title MISSION 1 centred on (264, 74),
  strips at y 100 and 405 from x 142, the page text from (150, 120), the page picture on the right
  half, the arrows centred on (222, 442) and (312, 442), CONTINUE (190, 465) 152 x 22. **The page
  label reads `<page>/<pages>`** (`1/2` on the first of two pages); the previous arrow is shown
  greyed on the first page. **The title's text (2026-11-15 night, Q-1055; data, E-trace-028):** the
  map's name used as a key of the interface texts (`controls.txt`): `campaign01` gives `Mission 1`, shown
  in capitals as MISSION 1 like every dialog title; every campaign map has its key (`campaign01` ..
  `campaign10`: `Mission 1` .. `Mission 10`) and so do the free-play maps (`MP_2P_Storm_Coast`: `Coastal
  Storm`); of the four tutorial lessons (`tutorial01` .. `tutorial04`) only `tutorial01` has one
  (`Tutorial 1`). **A missing key (2026-11-15 day; E-cam-013, code reading):** the interface texts'
  lookup gives back the key itself when the key is not in the texts (and keeps it as its own text from
  then on). So the lessons without a key show their bare map name as the title: `tutorial02` ..
  `tutorial04` are shown as TUTORIAL02 .. TUTORIAL04 (in capitals like every dialog title). An empty
  key gives an empty text. Confidence: high (the lookup and the title's set-up read in the code; the
  text matches the picture of campaign01). Test T19a.
- **CAM-1.3a Book pages, their texts and voices (2026-10-24, E-sim-007).** A page of the
  book dialog (1.3: briefing pages, debriefing pages) shows the `briefings.txt` entry
  of its key and, each time the page is shown, plays
  `data/language/<lang>/voices/<key>.mp3` when speech is enabled (the dialog's page
  display builds the path `voices\` + key + `.mp3` in the language folder through the
  voice player, which the tutorial's step text also uses, CAM-5.2). The shipped campaign scripts name 46 of the 51
  entries (all `b01` .. `b10` keys); `d00_A_00` .. `02` and `default0`, `default1` are
  named by no script (`default0` appears in the executable, its use is not read): the
  texts `d00_A_00`, `d00_A_01`, `d00_A_02` and their voice files `d00_A_00.mp3`,
  `d00_A_01.mp3`, `d00_A_02.mp3` are named by nothing and are taken as unused (medium). Each
  named key has its voice file. The tutorial's voice files `T01_*` .. `T04_*` (one per
  step name of 5.2) are played by the tutorial's step text (CAM-5.2). Confidence: high for the page voice path;
  medium for "when speech is enabled" (a settings byte is tested, which option it is
  was not read). Covers (inventory): *mission text* from `language/gb/briefings.txt`
  except `d00_A_00`, `d00_A_01`, `d00_A_02`, `default0`, `default1`. Covers
  (inventory): *voice file* from `data/language/gb/voices/b*` . Covers (inventory):
  *UI texture* from `data/gfx/ui_briefing_screen_*`.
- **CAM-1.4** The `triggers` table has one function, `removeTrigger(name)`: deactivates and deletes
  the trigger with that name.
- **CAM-1.5** `player0 .. player4`, the building names and good names used by the scripts are plain
  Lua globals: `player0 = 0`, others `1`; buildings and goods are the property ids of LUA-PROP.

### CAM-2 Triggers

- **CAM-2.1** A trigger (FMT-S2M-3.3) has a kind, a field, a player index, a name, an active flag
  and a 5-second timer (which of the record's fields is which: FMT-S2M-3.3; the player is the
  `u32` after the name, the timer starts from the stored `f32`; Q-187 a, E-cam-009). Kinds: 1 = the player has explored the field (editor name `FogOfWar_<A..>`),
  2 = the player's territory covers the field (`Region_<A..>`), 3 = like 2 but reaching it wins the
  mission (`RegionVictory_<n>`).
- **CAM-2.2** Every trigger is evaluated once per 5 s of game time (timer accumulates frame time,
  resets at 5): the condition sets the active flag (kind 1 and 2) or, for kind 3 when true, marks
  the map completed with value 4 (CAM-3.5) and sets the flag.
- **CAM-2.3** Each logic tick, after the evaluations, the first trigger (list order) whose flag is
  set is reported to the script: `onTriggerIsActive(name)`. The flag is not cleared: a script that
  does not remove the trigger is called again every tick. All shipped scripts call
  `triggers.removeTrigger(name)` in the callback.
- **CAM-2.4** Shipped triggers: `Region_K` (kind 2) in campaigns 1 to 3 (second briefing);
  `RegionVictory__0` (kind 3) in every campaign map, campaign 5 has `RegionVictory__0..3`. The
  tutorial maps have none.

### CAM-3 Mission target, victory and defeat

- **CAM-3.1** The map header's first closing value (FMT-S2M-2.7) is the **mission target**: 0 none
  (endless), 1 destroy all enemies, 2 conquer the map, 3 produce coins, 4 reach the portal. The
  free-game setup dialog offers 0 to 3 ("Endless", "Destroy all enemies", "Conquer 3/4 of the Map",
  "produce 100 coins"). Campaigns 1 to 3 carry 4, campaigns 4 to 10 carry 0 (they are won by their
  victory region only).
- **CAM-3.1a The target a game plays with (added 2026-11-05, Q-1003; E-cam-005).** The header value is not always the one
  used: at the end of a new game's setup (after the players, castles and start stock), when the map
  kind is 0, 1 (campaign), 2 (tutorial), 100 or 102, the game's mission target is **replaced by
  the start settings' target**, and the map kind is recorded with it. The campaign menu sets that
  target to **4** for every mission, the tutorial menu to 0, and the free-game setup to the chosen
  entry (0 .. 3). So **all ten campaign missions play with target 4, "reach the portal"**,
  missions 4 to 10 included, although their headers say 0; the mission panel (UI-GAME-2.4) shows
  `!MissionTarget: Reach the portal` (English "Reach the portal."), the HUD shows `!VICTORY` once
  the map is acknowledged (CAM-3.5), the end screen says "You have reached the magic gate!" and
  closing it returns to the campaign menu (CAM-3.6). The victory itself still comes only from the
  victory region (CAM-3.3: target 4 is never won by the per-tick test). Other map kinds keep the
  header's value. A loaded game keeps the target it was saved with. Confidence: high for the
  replacement and the values.
- **CAM-3.1b The gate on screen (2026-11-09, Q-1003; look E-trace-025 at mission 4, code E-cam-006).** Neither the HUD nor the minimap draws a symbol for the gate. What the player gets:
  - **The gate's surroundings start explored.** In the same set-up step as 3.1a, when the map kind
    is 1 (campaign), the game goes through the trigger list and, for every trigger of kind 3
    (`RegionVictory_*`, CAM-2.1), explores the fields within **radius 2** of the trigger's field
    (rings 0 .. 2, 19 fields, MAP-GRID-2.1) for the trigger's player (0, the human, in every shipped
    map; the explore routine applies it to each player that passes a relation test with that
    player, which in the shipped campaigns, where the human is alone on team 0, is the human only).
    Kind 1 and 2 triggers are not explored. So the minimap (UI-GAME-2.5) shows, besides the
    player's own area, one small explored patch at the gate in its land colours, and the 3D view
    shows the gate doodad (`doodad_gate_0`, "Gate01", a plain blocking doodad) there once the
    camera goes to it. Mission 4 (130 x 130, the human's castle at (68, 87), `RegionVictory__0` at
    (53, 26)): the minimap at counter 617 shows the castle's explored area and one patch of about
    5 x 5 fields centred at about (54, 24), nothing else. Campaign 5's four victory triggers (19, 54
    .. 56; 20, 55) give one patch.
  - **The mission panel (UI-GAME-2.4)**, opened with T, reads "Reach the portal." (3.1a); in the
    look it opened together with the briefing book again. The book blocks the HUD keys while open
    (M, T do nothing); **ESC closes the book** (E-trace-025: book at 545, ESC at 550, closed at
    581), the panel stays open.
  Confidence: high for the explore step (code) and the absence of a symbol (one look, mission 4,
  HUD and minimap); medium for which trigger field names the player (0 in every shipped map).
- **CAM-3.2** Defeat: once per logic tick, every player who is not yet lost and owns no
  headquarters building is marked lost (reason "Castle destroyed"). In single player, when a
  computer player is lost, the human gets the message "DEFEAT <name> (#n)".
- **CAM-3.3** Victory test, once per logic tick for every active player neither lost nor won, in
  player order; the first success ends the test for this tick:
  - target 1: the player's "enemies destroyed" count equals the enemy count;
  - target 2: the territory percentages of the player's team (the player and allies) sum to more
    than 74 (i.e. at least 75 % of the map);
  - target 3: the coins in the team's depots sum to more than 99 (at least 100);
  - targets 0 and 4: never by this test.
- **CAM-3.4** When a player wins: all team members are marked won, all other living players are
  marked lost with reason "Mission not completed", the tick is recorded for each. If the human is
  among the winners the map is marked completed with the mission target as value (CAM-3.5).
- **CAM-3.5** Map state (settings file, SET spec): per map name a record with `completed` (bool),
  `acknowledged` (u32) and a transient value. Marking completed sets completed = true, value = the
  reason (1 to 4), acknowledged = 0 unless it is already 1. Next game-screen update: if the record
  exists and is not acknowledged, it is acknowledged, the game is left and the event
  `LEVEL_COMPLETED <map name>` is raised (sound / end screen). The HUD shows "VICTORY" for target 4
  once the record is acknowledged.
- **CAM-3.6** End screen: "VICTORY" or "DEFEAT" with the reason, the target's text (1 "You have
  destroyed all enemies!", 2 "You have conquered the map!", 3 "You have enough Coins in your
  depots!", 4 "You have reached the magic gate!"), per player / team VICTORY or DEFEAT, buttons
  "EXIT MAP", "Statistics", "CONTINUE". The defeat screen says "You have lost the Map!" with the
  picture `ui_briefing_screen_lost`. The game clock is stopped (time scale 0) when the end dialog opens, and
  the music changes: the playlist stops and `Winning Jingle` (VICTORY) or `Loosing Jingle` (DEFEAT)
  plays (`SND-sound-music.md` SND-2.2; corrected 2026-11-08 b, E-gfx-041). Closing it: target 4 goes to the campaign menu, any other
  target to the main menu.
- **CAM-3.6b The victory book in a campaign mission (2026-11-15 night, Q-187 f; E-trace-028, pictures
  of campaign01 won with the cheat `INeedSomeHelp`; E-cam-012).** When a campaign mission's target is
  met, the game shows the **victory book**: the book dialog of 1.3c (frame `uiBriefing`, centred)
  with the title `!VICTORY` in the title's place (centred on (264, 74)), the target's text of 3.6
  (`You have reached the portal!` for campaign01) centred under it on about (264, 91) in a small white
  font, the mission's **debriefing pages** (`b<nn>_C_00`, `_01`, CAM-1.3a, registered by
  `addDebriefing`) as the book's pages with the page arrows and the `<page>/<pages>` label, the
  picture `ui_briefing_screen_won`, `!CONTINUE` at (190, 465) and **`!EXIT MAP` (shown `Exit map`)
  directly under it at (190, 487)**, both 152 x 22. CONTINUE closes the book and the mission goes on
  (the mission panel's second line then reads `(VICTORY)`, UI-GAME-2.4); EXIT MAP leaves the
  mission. So the debriefing shows in this book, not in the end screen of 3.6a. Confidence: high
  (pictures; the book's builder read, E-cam-012).
- **CAM-3.6a The end screen's layout (2026-11-14, Q-187 f; E-cam-010, code reading).** The end
  screen is the standard dialog (CAM-1.3c's base) with its close button hidden; its title is the
  same text the book dialog's title shows (the map's name looked up as an interface text key, 1.3c; medium), moved by (+10, +60)
  from the standard place. In the dialog's pixels:
  - the three buttons, text buttons of the Save Game dialog's style, **one under the other at
    x 300**: `!Statistics` at (300, 445), `!EXIT MAP` at (300, 467), `!CONTINUE` at (300, 489);
  - the result at (374, 110) (`!VICTORY` or `!DEFEAT`, `light_system_outlined`, size 8) and,
    25 px lower, its reason (`dark_system`, size 8): for a defeat `!Castle destroyed!` or
    `!Mission not completed`, for a victory the target's text of 3.6;
  - the players' rows from (140, 190), one row per player that exists (up to 8), 25 px apart:
    `Player #n`, the player's name, `!Team n`, and the player's own result (`!VICTORY`, `!DEFEAT` or
    nothing), the result in (255, 255, 155) for a player still in the game and (190, 190, 90) for
    a defeated one. **The frame (2026-11-15 night; E-cam-012, code reading):** the end
  screen's maker gives it the same frame picture as the book, `uiBriefing` (0, 0, 728 x 568), so it
  is 728 x 568, centred. Confidence: high.
  **The rows' columns and the defeat's button (2026-11-15 day; E-cam-015, code reading).** Each
  row's four texts stand at x 0, 90, 260 and 360 from the rows' origin (140, 190), so at x 140
  (`Player #n`), 230 (the name), 400 (`!Team n`) and 500 (the result) in the dialog, and row k
  (k = 0, 1, ...) at y 190 + 25 k. The labels use `light_system_outlined` for `Player #n` and
  `!Team n`, `dark_system` for the name. **On a defeat the CONTINUE button is disabled** (greyed,
  not clickable), so a defeated player can only look at the statistics or leave the map. The
  defeat's own code makes no other test (no network check was found; the earlier "not a network
  game" reading is withdrawn). Confidence: high for the buttons' places and texts, the two result
  lines, the rows' start, spacing and columns, and the disabled CONTINUE; the frame high (E-cam-012).
- **CAM-3.6c The defeat screen's layout (2026-11-15 day; E-cam-015, code reading).** The defeat
  screen ("You have lost the Map!", 3.6) is made on the book dialog (1.3c: the same `uiBriefing`
  frame, 728 x 568, centred, the same title from the map's name). The defeat screen changes the
  book as follows:
  - **hides** the book's page arrows (next and previous), its page label and its CONTINUE button;
  - **adds** a label at (260, 90) reading `!DEFEAT` (`light_system_outlined`, size 8);
  - puts `!You have lost the Map!` in the book's text box at (150, 120);
  - shows the picture `ui_briefing_screen_lost` in the book's picture place;
  - has **one button, `!EXIT MAP`**, a text button made at (190, 487) and moved to (190, 465),
    the place of the book's CONTINUE;
  - plays the losing jingle (music slot 3, as the end screen's defeat, SND).

  There is no CONTINUE and no Statistics button on this screen. Confidence: high (code); not seen
  in a picture (no defeat was reached in the original). Test T20a.
- **CAM-3.7** Cheats typed in the game (exact strings): `INeedSomeHelp` marks the map completed
  with the current target; `TooFast` changes the speed; `ShowMeTheWorld` reveals the map;
  `FreedomOfMovement`; `GiveMeSomeMore` adds 10 of every good to the headquarters.

### CAM-4 Campaign order and progression

- **CAM-4.1** Missions are `campaign01 .. campaign10` (map name = file name = settings key,
  format `campaign%02d`). An add-on campaign (`campaignAddOn`, 12 missions, kind 101) is coded but
  not shipped.
- **CAM-4.2** Campaign menu: mission 1 is always selectable; mission n + 1 becomes selectable when
  the map state of `campaign<nn>` is completed (CAM-3.5), for n = 1 .. 9 in order (the first
  incomplete mission stops the chain). Buttons sit on the campaign picture at (300,187), (465,187),
  (594,309), (551,490), (455,589), (200,510), (150,333), (322,460), (345,302), (457,379) in the
  1024 x 768 space, drawn offset by (-22, -75); hovering shows `campaign<nn>_dsc`.
- **CAM-4.3** Starting mission n: map kind 1, name `campaign<nn>`; setup: slot 0 human, slots 1 to
  7 computer players (the map's own player types decide which exist), the human's name from the
  profile (default "default"). Nothing carries over from the previous mission: resources, locks and
  start soldiers come from the script (`onLoaded` / `onStarted`). The menu writes start resources
  setting **1** (Medium, x 2, ECO-HQ-2.2) at every start, so an earlier free game's setting does
  not matter (Q-1002, answered 2026-11-04, E-cam-003).
- **CAM-4.3a Who the players are (answered 2026-11-04, Q-1007, E-cam-003).** For a campaign map the game creates each player from the **map's own record**
  (FMT-S2M-2.5, 2.6), not from the menu's entries: kind, tribe, colour, team and, for a computer
  player, strength. Strength 0, 1, 2 starts AI profile Weak, Normal, Strong. **A team is an
  alliance:** two players of one team are not enemies (MIL-SOL-3.1): they do not attack each
  other, their AI does not pick each other as targets and counts each other as friendly in its
  threat balance, soldiers do not fight, and they win together (CAM-3.4). This includes a computer
  player on the human's team. Every player present gets a headquarters with the start stock.
  Confidence: high, read in the code; not yet seen running (R13 would show it).

  | Mission | Human: team | Computer players: tribe, colour, team, strength |
  |---|---|---|
  | 1 | 1 | none |
  | 2, 3 | 1 | P1 Africans, 3, team 0, Weak |
  | 4 | 0 | P1 Africans, 1, team 1, Normal; P2 Africans, 2, team 1, Normal |
  | 5 | 1 | P1 Chinese, 1, team 0, Strong; **P2 Africans, 5, team 1, Normal: the human's ally**; P3 Chinese, 1, team 0, Weak |
  | 6 | 1 | P1 Chinese, 6, team 0, Normal; P2 Chinese, 3, team 0, Normal |
  | 7 | 1 | P1 Chinese, 1, team 0, Strong; P2 Chinese, 1, team 0, Normal |
  | 8 | 0 | P1 Chinese, 1, team 1, Strong; P2 Chinese, 2, team 1, Strong |
  | 9 | 0 | P1 Chinese, 3, team 1, Strong; P2 Chinese, 2, team 1, Normal; P3 Chinese, 6, team 1, Strong |
  | 10 | 0 | P1, P2, P3 Chinese, colours 1, 2, 3, team 1, Strong |

  The human is Romans, colour 0, in all ten. So from mission 4 on the enemies never fight each
  other, and from mission 7 most are Strong.
- **CAM-4.4** Shipped scripts: all missions register the debriefing pages `b<nn>_C_00` and
  `b<nn>_C_01` in `onLoaded` and open the briefing `b<nn>_A_00` + `b<nn>_A_01` with picture
  `ui_briefing_screen_<n>` at 25 s. Missions 1 to 3 open `b<nn>_B_00` + `b<nn>_B_01` with
  `ui_briefing_screen_<n>b` when `Region_K` is reached and remove the trigger. Mission 1 locks
  guardHouse, tower, catapult, fortress, donkeyBreeding, brewery, piggery, slaughterHouse, harbor
  and shipYard for the human and grants 40 `good_soldier0` in `onStarted`; mission 2 locks piggery,
  slaughterHouse, harbor and shipYard; missions 3 to 10 lock nothing and grant nothing.

### CAM-5 Tutorial

- **CAM-5.1** Four lessons (menu entries "Lesson n" / "Info n"), maps `tutorial01 .. 04` (kind 2).
  Lesson 3's script creates a guard house with a `good_soldier3`, a watchtower with a helper, roads,
  and an enemy barracks with a `good_soldier0`; lesson 4's script clears the resources and creates
  a harbour, a road, a shipyard with a ship builder and two ships. Lessons 1 and 2 have no script.
- **CAM-5.2** One global step index drives the guidance. Text key by step: steps 0x00..0x0d = T04
  (14 steps), 0x0e..0x1c = T03 (15), 0x1d..0x34 = T01 (24), 0x35..0x50 = T02 (28). Lesson 1 starts
  at 0x1d, lesson 2 at 0x35, lesson 3 at 0x0e, lesson 4 at 0x00. Advancing increments the index
  and resets the step timer; the window shows the step's text. The keys come from the game's own
  step table (E-sim-007, corrected 2026-10-24): for lessons 1, 3 and 4 it is the
  lesson's keys in `tutorial.txt` order; **lesson 2 differs**: 0x35 `T02_Introduction`, 0x36 ..
  0x3a the five `T02_Flag_*` steps, then 0x3b `T02_Scholar_OpenFlagDialog` and 0x3c
  `T02_Scholar_CallScholar`, then 0x3d .. 0x41 `T02_Coal_*`, 0x42 .. 0x46 `T02_Iron_*`, 0x47 ..
  0x49 `T02_Smeltery_*`, 0x4a .. 0x4f `T02_Smith_*` and 0x50 `T02_End` (in `tutorial.txt` the two
  Scholar steps stand just before `T02_End`). With this order the lesson-2 steps that expect a
  map click (CAM-5.3, 0x41, 0x46, 0x4f) are the three `EndStreet` steps (Q-187 j). Each time a
  step's text is shown, its voice `voices/<key>.mp3` is played with it (the step text and the
  voice come from the same call, which every step handler makes when the step changes; the
  same voice player as CAM-1.3a). Confidence: high for the table and the voice call; medium for
  "each time" (the callers were listed, not each read). Covers (inventory): *tutorial step* from
  `language/gb/tutorial.txt`. Covers (inventory): *voice file* from
  `data/language/gb/voices/T0*`.
- **CAM-5.2a The tutorial window as the original shows it (2026-11-15 night, E-trace-028: pictures
  of tutorial01 at 2560 x 1440; positions measured, +- 2 px).** Two forms, both titled TUTORIAL
  (the title font's capitals, centred in the header bar), the step's text in a white text box below
  the header from the frame's (20, 66), words wrapped:
  - **explanation steps** (the introduction: the step is done with the window's own button): the
    tall frame `uiBigDialog` (8, 8, 360 x 448) (E-cam-012: the window sets this picture and size for
    these steps) with its top-left at about (1100, 22) = ((W - 360) / 2, 22), i.e. centred at the top
    of the screen; two text buttons of 152 px one under the other at the bottom right, `Cancel` (frame
    (190, 376)) and `Continue` (frame (190, 401));
  - **action steps** (a map click or a widget is expected): the wide frame `uiMediumDialog` (8, 8,
    480 x 224, the production window's frame; the window switches its picture and size between the two
    forms) with its top-left at about (2075, 22), i.e. its right edge 5
    px from the screen's right edge, at the top right; one text button `Cancel` at the bottom right
    (frame (289, 167), 152 px); a wrong click adds the step's error key in red under the text
    (`T01_ERROR_SelectMarkedButton` and the others of 5.3).
  `Cancel` ends the tutorial and returns to the single player page with its TUTORIAL page open.
  **The expected widget is marked** with an orange arrow pointing down and to the right, its tip on
  the widget's top-left corner (seen on main-bar button 3 at step 0x1e). Escape does not open the
  game menu during a tutorial. Confidence: high for the two forms, their frames, buttons and the
  arrow (pictures); medium for the exact pixel places (one picture each, read from the frames'
  panel edges) and for the Escape observation (one try).
- **CAM-5.3** Steps that expect a map click show the pointer at a field (CAM constants table); all
  other steps expect a UI widget. A click elsewhere shows `T01_ERROR_SelectMarkedPosition` (field
  steps) or `T01_ERROR_SelectMarkedButton` (widget steps) and does not advance. At a lesson's last
  step (0x0d, 0x1c, 0x34, 0x50) the window offers "Play" and "EXIT MAP". Dialogs check the step
  before advancing: the ship dialog at step 9, the attack dialog at 0x18, the military / tools dialog
  at 0x1a, the flag menu at 0x32, the building dialog (production change) at step 2, among others;
  the complete widget-per-step table is open (Q-100 b) but follows the step names.
- **CAM-5.3c The widget a step expects, per dialog (2026-11-15, Q-100 b in part; E-cam-011, code
  reading).** Each dialog checks the step itself; a step is done only by a click on **one** control,
  and the dialog shows the step's error otherwise:
  - **Build menu (UI-GAME-3.2), SelectBuilding steps:** the expected control is one cell of the grid
    of the page shown (cell 0 is the flag button, cells 1, 2, ... the buildings in the page's order):
    0x10 cell 8, 0x16 cell 12, 0x20 cell 1, 0x26 cell 1, 0x28 cell 2, 0x2d cell 3, 0x37 cell 0 (the
    flag button), 0x3e cell 3, 0x43 cell 1, 0x49 cell 5, 0x4c cell 7. A click elsewhere in the menu
    shows `T01_ERROR_SelectMarkedButton` ("Click the highlighted button to continue!").
  - **Build menu, SelectPage steps (0x15, 0x25, 0x48, 0x4b):** only the **medium buildings tab**
    counts (the page switches to medium, the page is rebuilt with the step's building cell
    highlighted, the step advances). Any other click shows `T01_ERROR_SelectStreetIcon` ("Select the
    highlighted flag.", the original's own choice of key for these four steps).
  - **The expected control is highlighted** when the dialog is built at that step: the build menu
    highlights the medium tab at the SelectPage steps and the cell at 0x10 (cell 8), 0x20 / 0x43
    (cell 1), 0x28 (cell 2), 0x2d / 0x3e (cell 3), 0x37 (the flag button).
  - **Flag menu (UI-GAME-3.1):** the SelectCreateStreet steps (0x12, 0x22, 0x2a, 0x2f, 0x39, 0x40,
    0x45, 0x4e) need its build-road button (road building starts, the step advances); 0x3c
    (`T02_Scholar_CallScholar`) its call-geologist button (the call is sent, the step advances).
  - **Main bar:** 0x1e and 0x33 (show / hide the construction view) advance on a click of the
    construction-view button, 0x19 (open the military dialog) on the bar's own button; these checks
    look at the button's click, not at the key (SPACE).
  - **The other dialogs (completed 2026-11-15 day, Q-100 b; E-cam-014, code reading).** Each
    highlights the expected control when the step begins (the dialog built at that step, or the
    previous step of the same dialog moving the highlight on), advances on a click of that control
    only, and shows an error for any other click. `T01_ERROR_SelectStreetIcon` is shown for the
    page-choice steps (5, 0x1a) and `T01_ERROR_SelectMarkedButton` for the others:
    - **Building window (UI-GAME-4.1), lesson 4:** step 2 (`T04_BuildShip_ChangeProduction`)
      expects the shipyard's boat / ship switch. Its click sends `switch-production` (the switch to
      ships), refreshes the window, advances, and moves the highlight to the close button. Step 3
      (`T04_BuildShip_CloseDialog`) expects the window's close button (UI-GAME-1.8).
    - **Harbour window (UI-GAME-4.4, 8.1a):** step 5 (`T04_StartExpedition_SelectPage`) expects the
      **expedition tab** (`!Expedition management.`). Its click switches to the expedition page,
      advances, and highlights the page's start button. Step 6 (`T04_StartExpedition_SelectIcon`)
      expects the **expedition start button**. Its click sends `prepare-expedition`, refreshes,
      advances, and highlights the close button. Step 7 expects the close button.
    - **Attack dialog (UI-GAME-5), step 0x18 (`T03_Attack_StartAttack`):** expects the attack button.
      Its click sends `attack-building` with the dialog's count and strength choice, then advances.
    - **Economy dialogs (UI-GAME-6), steps 0x1a and 0x1b:** the step 0x1a check sits in the tools
      page, so the dialog the bar button opens at 0x19 is expected to show the tools page first
      (medium: the bar button's own target was not read). At **0x1a** (`T03_Misc_SelectPage`) the tools page highlights the **military tab** (its
      second icon tab, `!Global military management module`) and points the tutorial's arrow at the
      dialog. A click on that tab advances and opens the **military page**. At **0x1b**
      (`T03_Misc_CloseDialog`) the military page highlights its **close button**, and a click on it
      advances and closes the dialog.
    - The ship window's steps 9 .. 0xc are CAM-5.3a.
  Confidence: high for the build menu cells, the tab, the error keys, the flag menu buttons and the
  other dialogs' controls (each dialog's step cases read, each control named by its tooltip or the
  command its click sends); medium for the main-bar buttons' identity (read from the control's place
  in the bar object, not from its tooltip). Test T19, T19b.
- **CAM-5.3a Lesson 4's voyage (steps 0x08 to 0x0c; added 2026-11-05, Q-1014, E-ship-004).** Both ships of lesson 4's script are made by `createShip` (CAM-1.2), so
  both start in expedition mode with the load aboard; the shipyard builds none in the lesson (its
  resources are cleared). The voyage is staged, not sailed:
  1. **0x08** `T04_StartSailing_SelectShip`, a map click on the marked field (101, 63): the ship
     standing on that Water field (found by its field, not by its model) is selected and its ship
     window opens (`UI-ingame.md` UI-GAME-4.5). That window stays open up to step 0x0c; opening it
     at step 0x09 removes the map pointer, and no step from 0x09 to 0x0d places one again.
  2. **0x09** `T04_StartSailing_SelectArrow`: the expected widget is the compass button of
     direction 2 (`!SouthWest`, UI-GAME-4.5 f). Its click sends "go" in direction 2 for this ship
     (`SHIP-ships-expeditions.md` SHIP-6.2; with no option there the ship stays), refreshes the
     compass and advances.
  3. **0x0a** `T04_BuildPort_SelectShip`: **not a map click**. The expected widget is the window's
     **right fleet arrow** (UI-GAME-4.5 d, at (253, 400)). Its click selects the player's ship
     before the shown one in the ship list, wrapping to the last (with lesson 4's two ships: the
     second ship, made at (127, 36) next to the island's harbour spot (129, 34)), refreshes the
     window for it, moves the camera to that ship's field (`GFX-world-view.md` GFX-VIEW-3.6 d jump)
     and advances. Nothing waits for the first ship to arrive. A map click during this step does
     nothing (no message). The pointer table's field for 0x0a, (127, 35), is the land field next to
     the second ship but is never shown (step 1 above).
  4. **0x0b** `T04_BuildPort_SelectIcon`: the expected widget is `!Construct harbor.`; its click
     sends `found-harbour` for the shown ship (the second one) and advances; the step
     advances whether or not SHIP-7.1 accepts the command.
  5. **0x0c** `T04_BuildPort_CloseDialog`: the window's close button closes it and advances to
     0x0d.
  At each of 0x09 to 0x0c any other button of the window shows `T01_ERROR_SelectMarkedButton`
  and does not advance. Steps 10 to 13 are the steps CAM-5.4 exempts, since the camera is then at
  the second ship, far from the castle. Confidence: high for the steps' widgets and effects;
  medium for "never shown" (the placing calls were listed, none in the ship window) and for
  whether the second ship's harbour order is accepted (it depends on its last port position,
  SHIP-7.1, which `createShip` was not read for; one play of step 0x0b in the original settles it).
- **CAM-5.3b Which steps move the camera (added 2026-11-05, Q-1004; E-cam-005).** When the steps below begin (each right
  after the previous step was done), the game places the map pointer on the new step's field and
  starts a **scripted camera move** to that field (`GFX-world-view.md` GFX-VIEW-3.6 e). New step
  (field), with the step that leads to it:
  - lesson 1: 0x1f (42, 35) after the button step 0x1e; 0x24 (43, 37) after 0x23; 0x2c (40, 42)
    after 0x2b; 0x2e (40, 41) after the build-menu step 0x2d; 0x31 (42, 38) after 0x30;
  - lesson 2: 0x36 (48, 41) after the introduction 0x35; 0x38 (48, 41) after the build-menu step
    0x37; 0x3b (48, 41) after 0x3a; 0x3d (46, 43) after the flag-menu step 0x3c; 0x42 (46, 45)
    after 0x41; 0x47 (47, 40) after 0x46;
  - lesson 3: 0x0f (43, 31) after the introduction 0x0e; 0x13 (45, 37) after the flag-menu step
    0x12; 0x14 (44, 33) after 0x13;
  - lesson 4: 0x04 (103, 60) after the shipyard window's close (0x03); 0x08 (101, 63) after the
    harbour window's close (0x07); and the right fleet arrow of step 0x0a jumps the camera to the
    second ship at once (CAM-5.3a; an immediate jump, not this move).
  Every other step leaves the camera where it is, lesson 1's and lesson 4's first steps included
  (the start camera, GFX-VIEW-3.6, shows their fields). Confidence: high (each call site read
  through its step dispatch).
- **CAM-5.4** In lesson 4, when the camera is more than 150 units from the castle and the step is
  outside 10..13, the camera jumps back to the castle (`GFX-world-view.md` GFX-VIEW-3.6 d) and the
  hint with the key `!You can return to your castle, by pressing the 'H'-key.` (`tutorial.txt`) is shown (play test 10, E-gfx-019).
  `T04_EndTutorial` also plays `voices/T04_EndTutorial.mp3`. Leaving asks "Continue Tutorial" /
  "Cancel Tutorial".

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| trigger_period | 5 | s game time | high | E-cam-001 |
| briefing_delay | 25 | s (script) | high | campaign scripts |
| conquer_threshold | > 74 | percent of map (team sum) | high | E-cam-001 |
| coins_threshold | > 99 | coins (team sum) | high | E-cam-001 |
| campaign_missions | 10 | | high | data |
| mission_key_format | `campaign%02d` | | high | E-cam-001 |
| tutorial_steps | 81 (0x00..0x50) | | high | E-cam-001 |
| lesson_start_steps | 0x1d, 0x35, 0x0e, 0x00 | | high | E-cam-001 (step table) |
| pointer_fields | step: field | | high | E-cam-001 |

Pointer fields: 0x01 (104,56), 0x04 (103,60), 0x08 (101,63), 0x0a (127,35), 0x0f (43,31), 0x11
(44,30), 0x13 / 0x23 / 0x2b / 0x3a / 0x4f (45,37), 0x14 (44,33), 0x17 (55,29), 0x1f (42,35), 0x21
(43,34), 0x24 (43,37), 0x27 (46,35), 0x29 (47,34), 0x2c (40,42), 0x2e (40,41), 0x30 / 0x4a (44,36),
0x31 (42,38), 0x36 / 0x38 / 0x3b / 0x41 (48,41), 0x3d (46,43), 0x3f / 0x46 (47,42), 0x42 (46,45),
0x44 (47,44), 0x47 (47,40), 0x4d (44,35).

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | campaign01 started | 25 s of game time pass | book dialog with `b01_A_00`, second page `b01_A_01`, picture `ui_briefing_screen_1`; 40 soldiers of rank 0 were added at start; the ten locked buildings are absent from the build menu |
| T2 | campaign01, human territory grows over field (55,51) | next 5-second evaluation | `onTriggerIsActive("Region_K")`: dialog `b01_B_00/01`, trigger removed; no second call |
| T3 | campaign01, human territory covers (64,70) | next evaluation | map state `campaign01` completed (value 4); next game-screen update raises `LEVEL_COMPLETED campaign01`; end screen VICTORY "You have reached the magic gate!"; afterwards the campaign menu offers mission 2 |
| T4 | free game, target 2, team of one, territory 75 % | tick | the player wins; every living enemy is marked lost with "Mission not completed" |
| T5 | free game, target 3, 100 coins in the team's depots | tick | victory |
| T6 | any game, human headquarters destroyed | tick | human marked lost, defeat screen "You have lost the Map!" |
| T7 | free game target 1, last enemy destroyed | tick | victory; end screen "You have destroyed all enemies!" |
| T8 | tutorial lesson 1 | start | step 0x1d `T01_Introduction`; a wrong map click at step 0x1f shows `T01_ERROR_SelectMarkedPosition` and keeps the step |
| T9 | campaign menu, `campaign01` not completed | open | only mission 1 selectable |
| T10 | a trigger kept active and not removed | ticks | `onTriggerIsActive` called every tick |
| T11 | 3.1a: campaign menu, mission 4 (`campaign04`, header target 0) | start the mission | the game's mission target is 4; the mission panel reads `!MissionTarget: Reach the portal`; the per-tick victory test never wins it; reaching `RegionVictory__0` wins with "You have reached the magic gate!" and closing the end screen opens the campaign menu |
| T12 | 3.1a: free game on any map, setup entry `!Conquer 3/4 of the Map` | start | target 2 whatever the map's header says; tutorial lesson 1: target 0 |
| T15 | 3.1b: campaign04 started from the campaign menu (human at (68, 87), `RegionVictory__0` at (53, 26)) | the human's explored fields right after the set-up | the castle's area plus exactly the 19 fields within distance 2 of (53, 26); the minimap shows that patch and no gate symbol; campaign01: the 19 fields around (64, 70), none around `Region_K` (55, 51) |
| T13 | 5.3a: lesson 4 at step 0x09, the ship window of the ship from (101, 63) open | click the `!SouthWest` compass button; then the right fleet arrow; then `!Construct harbor.`; then the close button | go 2 is sent for that ship and the step is 0x0a; the window shows the ship made at (127, 36) and the camera jumps to (127, 36), step 0x0b; the harbour command is sent for that ship, step 0x0c; the window closes, step 0x0d (`T04_EndTutorial`) |
| T14 | 5.3b: lesson 3 started, camera at the start camera | finish the introduction (step 0x0e) | the pointer appears at (43, 31) and the camera moves there (GFX-VIEW-3.6 e: zoom 3, 12.5 % of the eye's gap per frame, no scrolling until it has arrived); finishing step 0x10 moves nothing |
| T16 | 1.3b: tutorial03's `onStarted` (guard house (51, 36) and watch tower (51, 38) placed with their flags (51, 35) and (51, 37), the headquarters' flag (45, 37)) | `createStreet(player0, 51, 37, 45, 37)`, then `createStreet(player0, 51, 35, 48, 37)` | three roads: (51, 37) - (48, 37), (48, 37) - (45, 37) (the first road split by the flag placed at (48, 37)) and (51, 35) - (48, 37); each asks for a carrier |
| T17 | 1.3b: `createStreet` from a player's flag to a free field that is on no road | the call | no flag at the end, no road |
| T18 | 1.3c (Q-187 e): campaign01's briefing opens | the dialog's parts | title at (w / 2 - 100, 74), no close button; text box (150, 120) 230 x 360 black `dark_system`; previous (210, 430) and next (300, 430) disabled at first; label (267, 442); CONTINUE (190, 465); strips at (142, 100) and (142, 405); picture (201, 30) 512 x 512 |
| T19 | 5.3c (Q-100 b): lesson 1 at step 0x25 (`T01_SawMill_SelectPage`), the build menu open on the small page | a click on the large tab, then on the medium tab, then at step 0x26 a click on cell 2 | the large tab shows "Select the highlighted flag." and nothing changes; the medium tab switches the page and advances to 0x26; cell 2 shows "Click the highlighted button to continue!"; only cell 1 (the sawmill) advances |
| T19a | 1.3c (missing key): the book of tutorial lesson 2 (map `tutorial02`) opens with the English texts | the book's title | TUTORIAL02 (the map name itself, no key `tutorial02` in `controls.txt`); lesson 1 shows TUTORIAL 1 (its key gives `Tutorial 1`); campaign01 MISSION 1 |
| T19b | 5.3c (Q-100 b, the other dialogs): lesson 4 at step 5 with the harbour window open; then lesson 3 at step 0x1a with the tools page open | highlights, clicks, errors | harbour: the expedition tab highlighted; a click on the goods tab shows "Select the highlighted flag." (`T01_ERROR_SelectStreetIcon`) and nothing advances; the expedition tab: the expedition page, step 6, its start button highlighted; the start button: `prepare-expedition` sent, step 7, the close button highlighted; the close button: step 8. Tools page at 0x1a: the military tab highlighted; a click on the distribution tab shows `T01_ERROR_SelectStreetIcon`; the military tab: the military page opens at step 0x1b with its close button highlighted; its close button: the dialog closes, step 0x1c |
| T20a | 3.6a, 3.6c: a free game with three players lost by the human; then a campaign mission lost | the end screen; the defeat screen | end screen: DEFEAT at (374, 110), `Castle destroyed!` 25 px lower; rows at y 190, 215, 240 with `Player #1` at x 140, the name at 230, `Team n` at 400, the result at 500; CONTINUE greyed; Statistics and EXIT MAP work. Defeat screen: the book frame and title, DEFEAT at (260, 90), `You have lost the Map!` in the text box, the lost picture, only EXIT MAP at (190, 465), no page arrows |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-cam-001 | code reading | S2DNG.exe: menu API, triggers, script load, trigger tick and update, result system, map state, level completion and cheats, HUD target, end screens, campaign menu, tutorial | all rules |
| E-cam-003 | code reading | | who a mission's players are, teams, start resources setting |
| E-cam-007 | code reading | | 1.3b: `createStreet`: the route search, the start flag test, the end flag (only on another road where a flag may stand, splitting it), the road's building (T16, T17) |
| E-cam-008 | code reading | (session 2026-11-14) | 1.3c (Q-187 e): the book dialog's builder (title move, text box size, page buttons, label, CONTINUE, strips), the standard dialog's set-up (title font and colour, close button), the defeat screen built on it |
| E-cam-009 | code reading, data | (session 2026-11-14; the shipped campaign maps) | 2.1 / FMT-S2M-3.3 (Q-187 a): the trigger record's load order and the fields the trigger check reads (kind, field, player, timer, active flag) |
| E-cam-010 | code reading | (session 2026-11-14) | 3.6a (Q-187 f): the end screen's builder (buttons, result lines, player rows) |
| E-cam-005 | code reading | the end of a new game's setup (target from the start settings; map kinds 0, 1, 2, 100, 102), the start settings' target setter and its callers (campaign: 4, tutorial: 0, free game: the chosen entry) | CAM-3.1a, T11, T12; CAM-5.3b, T14 (with step dispatches and the scripted move) |
| E-cam-006 | code reading | = E-gfx-045 (session 2026-11-09 b): the new game's setup, for map kind 1, explores radius 2 at every kind-3 trigger's field for its player | CAM-3.1b, T15 |
| E-trace-025 | look | auto.log, drive.log, run.log: campaign mission 4 started with a profile copy (campaigns 1 .. 3 completed; the real profile restored byte for byte), pictures `campaign04-*` of the look comparison of 2026-11-09: book at 240 / 545, ESC closes it, minimap (617), mission panel (654) | CAM-3.1b |
| E-ship-004 | code reading | tutorial map clicks, the ship window's steps 9 .. 12, `createShip` (expedition on) | CAM-5.3a, CAM-1.2, T13 |
| E-cam-002 | data | `data\maps\campaignMaps\campaign01..10.lua` + `.s2m` triggers, `tutorialMaps\tutorial03/04.lua`, `language\gb\briefings.txt`, `tutorial.txt` | CAM-1.5, 2.4, 4.4, 5.1, 5.2 |
| E-cam-011 | code reading | (session 2026-11-15 night) | 5.3c: the build menu's step cases (cells, the medium tab, the error keys), the flag menu's, the main bar's (Q-100 b in part) |
| E-cam-012 | code reading | (session 2026-11-15 night) | 1.3c, 3.6a: the book's, the victory book's, the defeat screen's and the end screen's frame picture `uiBriefing` (0, 0, 728 x 568) |
| E-cam-013 | code reading | (session 2026-11-15 day) | 1.3c: the book's title is set from the map's name through the interface texts' lookup, which returns the key itself for a missing key (and stores it) and an empty text for an empty key |
| E-cam-014 | code reading | (session 2026-11-15 day) | 5.3c: the building window's, harbour window's, attack dialog's, tools page's and military page's step checks, highlights, commands and error keys |
| E-cam-015 | code reading | (session 2026-11-15 day) | 3.6a: the end screen's row columns (0, 90, 260, 360 from (140, 190)), the defeat branch disabling CONTINUE; 3.6c: the defeat screen built on the book (hidden arrows, label and CONTINUE, the DEFEAT label, the text, the picture, EXIT MAP moved) |

## Open questions

- Q-100 (b): which script globals the `GameScript` block stores; the exact widget expected per
  tutorial step (only the step names and pointer fields are extracted); what the two unexplained
  u32 setup fields set at mission start mean; whether the add-on campaign code paths matter.
