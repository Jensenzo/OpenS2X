# UI-SETUP: Players of a free game (tribe, team, colour, start)

Status: draft (read from the binary; not checked in the running game)
Owner question(s): Q-051 in `OPEN_QUESTIONS.md` (also Q-053, Q-211, Q-1045); supports TRN-FREE-5.5b
Game build: 11757

## Summary

A free game (single player, LAN or internet) is set up in the game-setup
screen, which has up to six player slots. Each slot holds a kind (open,
closed, human, computer), a computer strength, a **tribe**, a team, a colour
and a start position. The tribe is chosen per slot in that screen with an
icon picker; it is not taken from the map. When the game starts, every
occupied slot becomes a player, and the player's tribe is the slot's tribe.
Every building a player owns uses that tribe (construction positions, door
offsets, graphics).

## Rules

### UI-SETUP-1 Slots

- **UI-SETUP-1.1** The setup screen has 6 slots. Each holds: kind (0 open,
  1 human, 2 computer, 3 closed), computer strength, tribe (0 Romans,
  1 Africans, 2 Chinese, numbering of `buildings.lua`), team, colour and
  start position.
- **UI-SETUP-1.2** A slot that is reset (emptied, or its kind changed) has
  kind closed and tribe, team, colour and start position "unset" (-1).
- **UI-SETUP-1.3** When the host opens the LAN or internet setup screen, slot
  0 becomes the host: human, tribe 0, team 0, colour 0, start position 0.
- **UI-SETUP-1.4** The tribe of a slot is shown and chosen as one of three
  tribe icons. A joined player's change of tribe, team, colour or start
  position is sent to the host, which applies each value that differs.
- **UI-SETUP-1.5 (corrected 2026-11-04, Q-1007)** The map's per-player record (`FMT-s2m-map.md`
  FMT-S2M-2.5, 2.6) sets a campaign game's players (`CAM-campaign-missions.md` CAM-4.3a) and fills
  the setup rows of a map with preset players, which show its team, colour, strength and tribe
  (E-cam-003). It is not used by the plain free-game setup path.

- **UI-SETUP-1.6 New computer slots** (Q-053; corrected 2026-11-14, Q-1045, E-ui-028). The
  game makes a slot a computer slot in two ways, with different start values:
  - (a) **A new single-player free game:** slots 1 to 5 become computers with strength 0,
    **tribe 0**, and start position, colour and team **all equal to the slot's number**: slot i
    has start position i, colour i and team i. (In LAN and internet games these slots are made
    open instead: kind open, strength 0, tribe 0, start position unset (-1), colour i and team
    i.) The mode test reads the session type, single player taken as the type that is neither
    LAN nor internet host (medium). (The earlier reading "start position, colour and team
    unset" for (a) was a misread of the call's arguments and is withdrawn; T12's preview
    markers, slot 1 on start position 1 in red = colour 1, agree with the corrected reading.)
  - (b) **The host changes a slot's kind to computer in the slot's list:** the slot first
    becomes computer, strength 0, tribe 0 with start position, colour and team unset (-1),
    and then the values of the slot's row (tribe icon, team, colour, start position) are
    written over it, so it keeps the tribe, team, colour and start position its row showed. A
    slot changed to closed gets the same two steps (closed with -1, then the row's values).
    A slot that was a human and is changed to anything else is first reset (UI-SETUP-1.2).

  So a free play slot never shows or plays an unset colour: what a computer slot shows before
  any click is its number's cell (UI-SETUP-1.8: (520 + 24 (i mod 2), 216 + 24 floor(i / 2))),
  the first click on its colour square gives (i + 1) mod 8, and unless it is clicked the
  computer plays colour i (UI-SETUP-2.1 copies the colour unchanged). Nothing in the page
  guards against a colour of -1 (the square's cell would be read from outside its table), and
  no free play path hands it one. Confidence: high (code reading) for (a), (b) and the
  colour's way to the player; medium for the row's own values after a reset (the row widget's
  value is not traced). Test T19.
- **UI-SETUP-1.7** A player who joins a LAN or internet game gets the first
  open slot, as a human with tribe 0 and strength 0; start position, colour
  and team are set to one value the host picks (its rule is not traced).
  Confidence: high for tribe 0.

- **UI-SETUP-1.8 Free play page layout (Run D, OBS-20, Q-211).** Drawn on
  the left sheet of the single player page (UI-MENU-1.5), panel pixels:
  - Map: the map button (152 x 22 bar of UI-MENU-1.4, caption key `!Map:`,
    shown as "Select map:") at (119, 124); the preview at (122, 159) inside
    the 104 x 104 frame `uiElements0` (520, 104) at (119, 156); `!Map Size`
    at (232, 172) with its value ("w x h") at (232, 190); `!Players` at
    (232, 212) with its value at (232, 230). Captions in the `dark_system`
    font, values in `light_system_outlined` (no extra colour).
  - Options: drop-downs at (433, 124 + 45 n), each with its caption above
    it, captions `dark_system`: n 0 `!Start resources:` (`!Low`, `!Medium`,
    `!High` = 0, 1, 2; Medium preselected), n 1 `!Victory condition:`
    (`!Endless` 0, `!Destroy all enemies` 1, `!Conquer 3/4 of the Map` 2,
    `!produce 100 coins` 3; Endless preselected; shown in English as "None (Endless)", "Destroy all
    enemies", "Conquer 3/4 of the map", "Collect 100 gold coins", E-trace-031), n 2 `!FOW` (shown as "Fog
    of war:"; `!On` 0, `!Off` 1; On preselected), n 3 `!Allow Downloads:`
    (`!ALLOW_DOWNLOAD` 1, `!DENY_DOWNLOAD` 0; the network page's row).
  - Slots: six, slot i at P = (180 + 275 (i mod 2), 405 + 80 floor(i / 2)),
    so rows at y 405, 485, 565. Per slot: the emblem button 48 x 48 at P
    (`uiElements0` (520, 8) normal, (520, 56) hovered, (520, 356) the third
    state; tooltip `!Select severity`); the colour square 24 x 24
    (`uiElements0` (520, 216)) at (P.x - 65, P.y + 20), tooltip `!Select
    colour`; the portrait 40 x 80 (`uiElements0` (16, 512)) at (P.x - 47,
    P.y - 25), tooltip `!Select people`; three drop-downs at x P.x + 45 and y
    P.y - 22, P.y, P.y + 22 (in the photo: HQ, player / computer, team from
    top to bottom). In a network game the slots start 47 px lower and each
    gets a 26 x 26 box at (P.x - 69, P.y - 8).
  - **Which picture, emblem and colour cell a slot shows (2026-11-12 night, Q-211 b; E-ui-022,
    code reading).** The rectangles above are only the ones the slot is made with. Whenever the
    page shows a slot it sets them from the slot's own values. With the slot's **level** L = 2 for
    a human slot, the computer strength (0 .. 2) for a computer slot, 0 for an open or closed
    slot, and its tribe T (0 Romans, 1 Nubians, 2 Chinese):
    - portrait: `uiElements0` (16 + 168 T + 56 L, 512), **46 x 80**; hovered and pressed
      (16 + 168 T + 56 L, 600), 46 x 80. So each tribe has three pictures, one per level, and a
      human player always shows his tribe's third (level 2), as in the photo.
    - emblem (the severity button): (520 + 48 L, 8), 48 x 48; hovered and pressed
      (520 + 48 L, 56). Its picture is the same level: the human's is the third (the crossed
      swords of the photo), a computer's its strength.
    - colour square: the slot's colour c at (520 + 24 (c mod 2), 216 + 24 floor(c / 2)),
      24 x 24, the same for every state (colours 0 .. 7 in a two-wide grid of four rows).
    Test T16.
  - **What a click on them does (2026-11-13 night, Q-211; E-ui-027, code reading).** Each click
    steps one of the slot's own values and wraps:
    - the emblem steps the strength: s -> (s + 1) mod 3, so Weak -> Normal -> Strong -> Weak.
    - the portrait steps the tribe: T -> (T + 1) mod 3, so Romans -> Nubians -> Chinese ->
      Romans.
    - the colour square steps the colour: c -> (c + 1) mod 8.

    The three drop-downs open their lists instead (UI-GAME-8.5g). The page then hands the
    slot's values to the set-up. The strength is handed on only for a computer slot, so a
    click on a human's or an open slot's emblem changes nothing that is shown or played.
    Confidence: high for the three steps and the computer-only strength; medium for which
    control each step belongs to (read from the value each one changes). Test T18.
  The photo-read positions of Q-211 (a) are within a few pixels of these;
  where they differ, these hold (option rows 45 px apart, not 48; slot rows
  80 px apart; the slot column 275 px). Confidence: high for the
  positions (E-ui-007), medium for
  which drop-down holds which list.
  - **Caption of an option drop-down (Q-301 b, E-gfx-030).** The caption
    is the drop-down's own text line (not a separate label). Its font is
    `dark_system` size 16, white tint. It is anchored at x = the drop-down's
    left edge and y = the anchor of the bar's own value text: half the
    224 x 19 bar's height (9.5 px) minus 0.001 of the screen height. Read
    again for Q-307 (E-sim-002): that offset (plus 1 px
    for the 224 x 19 style) is the text offset the **bar's own value text**
    gets in the text button constructor, not the caption's; the option row is
    a caption control that owns the bar as a child at its own origin, and a
    text line's top sits at its anchor (UI-MENU-1.8), so where the row draws
    its caption relative to the bar is still untraced (Q-331). Confidence:
    low for the caption's y.
    **Where the caption sits (answered 2026-11-09 c, Q-331; E-sav-012).** The page sets each option row's caption on
    the drop-down's own text line (font `dark_system`, colour white (255, 255, 255)) and then
    calls the drop-down's "caption above" set-up, which gives that text line **alignment 4
    (left)** and the **offset (0, -10 px)** from the drop-down's origin (the bar's top-left
    corner; the constructor's own values, alignment 16 = right and the bar's text offset, are
    replaced). The text line's draw always adds alignment 1 (the y origin moved up by half the
    line height) to the line's own alignment. With `dark_system`'s line height 16 (base 13) the
    caption's line box runs from **18 px to 2 px above the bar's top**, its left edge on the
    bar's left edge (baseline 5 px above the bar). So on the free play page (bars at (433, 124
    + 45 n)) caption n's line top is at (433, 106 + 45 n). (UI-MENU-1.8 (b)'s "never moves the
    text vertically" holds for the label and text box draw, not for this text line, whose draw
    adds the 1.) Confidence: high (code).
  - **START button (Q-301 b, E-gfx-030).** In a local (not network) game,
    the first slot's block creates the START button: a menu text button
    (`!Start`, 208 x 32, UI-MENU text button style 0, tooltip
    `!START_BUTTON_TOOLTIP`). Its top-left corner is at panel pixel (42, 0),
    the origin of the layout group opened for it, with no sprite offset.
    A network game has no such button in the slot block. Confidence: medium
    (code reading; not compared with a photo).
  - **What START checks (2026-11-14, Q-211 f; E-ui-030, code reading).** START is the human
    player's **ready** switch: a click asks for the player's slot to be set ready (or not ready
    again). The request is refused (the slot stays not ready) while **two slots have the same start
    position** (unset positions do not count). The game starts as soon as every slot up to the
    map's player count is ready: a computer or a closed slot always is, an **open slot never is**
    (so an open slot within the map's players keeps the game from starting), a human slot once its
    player has clicked START. While two slots share a start position the START switch is shown off
    and its tooltip is `!READY_CHECK_UNUSABLE_TOOLTIP` (otherwise `!READY_CHECK_TOOLTIP`).
    **The tooltips in detail (2026-11-15, E-ui-035, code reading):** the START button is made with
    the tooltip `!START_BUTTON_TOOLTIP` ("Starts the game"); every refresh of the human slot's row
    then replaces that tooltip: `!READY_CHECK_TOOLTIP` ("Ready to start") while the start positions
    are distinct, `!READY_CHECK_UNUSABLE_TOOLTIP` ("Double allocation of HQ") while two slots share
    one (and the switch is set off). The row is refreshed when the page is filled, so in practice
    the button shows "Ready to start" (or "Double allocation of HQ"), and "Starts the game" never
    reaches the screen. **Seen (E-trace-028, 2026-11-15 night):** hovering START on the free play
    page of the default two-player set-up shows the tooltip `Ready to start` (a grey translucent box
    with a light border, white text, its top-left about 25 px right of and 45 px above the pointer).
    Confidence: high.
    Confidence: high for the ready rule, the shared-position refusal and the open slot; medium for
    the switch's look. Test T20.
  - **The team list** shows `!Team 1` .. `!Team n` for teams 0 .. n - 1, n the map's player
    count (at most 6), and selects the slot's team: a new computer slot i shows `!Team i+1`
    (UI-SETUP-1.6 (a), so slot 1 "Team 2" as in the photo). Confidence: high.

- **UI-SETUP-1.8a Start markers on the map preview (Q-224; E-gfx-027).**
  The map preview image (the map's `.bmp`, drawn 98 x 98 px with its
  top-left at (122, 159), inside the frame of 1.8) carries one number per
  **start position of the map** (every one the map has, used or not), drawn
  as text labels when the page is filled for the selected map:
  (a) **Text.** Start position k (0-based, in the map file's order) shows
  the number k + 1 in the `small_no_alias_white` font, not translated.
  (b) **Place.** With the map W x H fields and the start position at field
  (x, y): the point p = (122 + x x 98 / W, 159 + (H - y) x 98 / H) in panel
  pixels (the field row counted up from the bottom of the preview). The
  label is placed at p like every label: the top-left corner of its line
  box is at p (UI-MENU-1.8; Q-225). In `small_no_alias_white` (line height
  12, base 10, cells padded 3 px on every side) a digit's cell is 12 x 13
  (the "1": 8 x 13) at xoffset -3, yoffset 0, so its ink lies at
  p + (0 .. 5, 3 .. 9) ("1": p + (0 .. 1, 3 .. 9)) and its centre about
  (2.5, 6) right of and below p; the photo's reading (1 px right, 2 px
  below) is within its perspective error (Q-211 a). The markers are label
  controls made by the page's refresh method (E-gfx-027, a method
  of the page that first clears the old ones), not computed every frame; it
  runs when the page is filled for a map choice; whether a slot's start
  drop-down change runs it too is medium (T13 keeps its expected result
  either way once the markers are rebuilt).
  (c) **Outline.** Four black (0, 0, 0) copies at p + (-2, 0), p + (2, 0),
  p + (0, 2), p + (0, -2) (whole pixels at the 1024 x 768 layout; the code
  adds 2 / screen width and 2 / screen height in its normalised units),
  then the coloured copy at p on top.
  (d) **Colour.** The first of the six slots (0..5) whose start position is
  k gives its colour index c (the colour square of 1.8); the digit colour
  (r, g, b) is: c 0 (90, 90, 255) blue; 1 (255, 90, 90) red; 2
  (90, 255, 90) green; 3 (255, 255, 90) yellow; 4 (255, 255, 255) white;
  5 (90, 90, 90) dark grey; 6 (255, 90, 255) magenta; 7 (192, 192, 192)
  light grey. A start position no slot has chosen is drawn brown
  (128, 64, 0). When the list entry is a saved game (the list's save
  folder, 20), the colour is player k's colour index from the save, with
  the same table.
  Confidence: high for text, place formula, outline and colours
  (E-gfx-027: the marker labels, the slot found by start position, the 98.0 px preview size
  and the refresh that places them);
  medium for the label's anchor (top-left, as the other labels; from the
  photo) and for when the page refills the labels (the refresh is a page
  method that is called indirectly, so its callers were not all traced).

### UI-SETUP-2 From slots to players

- **UI-SETUP-2.1** When the free game starts, each of the 6 slots is copied
  into the game setup record (8 entries; entries 6 and 7 stay empty):
  - kind: open or closed -> no player; human -> human player (local or
    remote); computer -> computer player;
  - tribe, team, colour and computer strength: copied unchanged;
  - start position: the slot's position; an unset position (-1) of a slot
    whose number is not below the number of available slots (presumably
    the map's player count) becomes the slot's own number.
- **UI-SETUP-2.2** Every setup entry with a player creates a player whose
  tribe is the entry's tribe. A building takes its owner's tribe when it is
  created (`TRN-free-walk.md` TRN-FREE-5.5b).
- **UI-SETUP-2.3** The default of a setup entry (before any slot is copied)
  is tribe 0, team -1, colour = entry number, no player, and strength 0
  (strength added 2026-11-13, Q-187 i; E-ui-026: the reset writes all five).
- **UI-SETUP-2.4** A start that does not go through the setup screen (the
  path used outside the lobby) makes entry 0 the human player and entries
  1..7 computer players, all with **tribe 0**. Which start uses which path
  (Q-054):
  - **Campaign** (map `campaignNN`, and `campaignAddOnNN` for the add-on
    campaign; mission number from the campaign menu): this non-lobby path, but
    **for map kind 1 (`campaignNN`) these entries are not what the game uses**: its players
    come from the map's own records (CAM-4.3a, corrected 2026-11-04, Q-1007). Only the
    add-on's kind (101), which ships no maps here, would use the entries. Start resources
    setting 1, start positions not shuffled. Confidence: high.
  - **Tutorial** (map `Tutorial0N`): start resources setting 2; sets only the kinds (entry 0
    human, the others computer); tribes are not written, so they keep the start
    settings' values, which are tribe 0 for every entry after the settings are
    reset (UI-SETUP-2.3; the settings are reset when created and when a game
    is left). In effect tribe 0. Neither are teams nor strengths: in effect no
    team and strength 0, so a tutorial's computer players start with the Weak
    profile (`CAM-campaign-missions.md` CAM-4.3a: strength 0, 1, 2 = Weak,
    Normal, Strong; 2026-11-13, Q-187 i, E-ui-026). The map's own player records
    are read only for map kind 1 (the campaign bullet above), not for a tutorial
    (map kind 2). Confidence: high for what the start
    writes (the kinds only) and for the reset values; medium (reset moment not
    traced).
  - **Free game** (single player, LAN, internet): the setup screen
    (UI-SETUP-1, 2.1). Confidence: high.
  - **Loaded save**: the setup stored in the save is read back and the slots
    are filled from it (kind, tribe, colour, team per entry); if no entry is
    human, entry 0 becomes human. The players' tribes are therefore those of
    the saved game. Confidence: high for the slot refill, medium for the
    players themselves coming from the save's player data.
  - The map's own player record (FMT-S2M-2.6 field 4) is not used by any of
    these paths, and map scripts cannot change a tribe: the script API has
    only `pl_getTribe`, no setter, and the 12 shipped map scripts call
    neither. Confidence: high.
- **UI-SETUP-2.6 Unset tribe at start** (Q-053). The start does not check
  the tribe: the checks before starting are only that every slot is ready
  and that no start position is taken twice. A slot's tribe is copied
  unchanged, so a player can in principle start with tribe -1. In the paths
  traced only reset slots (kind closed, UI-SETUP-1.2) carry -1, and closed
  slots create no player, so the case should not arise. An implementation
  must not use -1 as an index (the game would read the construction
  position table out of range): treat an unset tribe as 0. Confidence:
  high for "copied unchanged" and the checks; the game's behaviour with -1
  is not specified.
- **UI-SETUP-2.5** A computer player's strength in the slot (0, 1, 2) selects
  AI profile 1, 2 or 3 (`ai.lua`).

### UI-SETUP-3 What an implementation needs

- **UI-SETUP-3.1** Treat the tribe as a per-player input of the game setup,
  0..2, chosen by the user per slot. Default **0 (Romans)** for every player
  (UI-SETUP-1.3, 2.3, 2.4).
- **UI-SETUP-3.2** The tribe has no effect on any economic value in the
  scripts except the per-tribe construction positions
  (`bp_setConstructionPosition`) and door offsets
  (`bp_setDoorOffsetDirection`, `ECO-buildings-catalog.md` ECO-CAT-2.4);
  the rest is graphics and music.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| slots in the setup screen | 6 | | high | E-ui-001 |
| setup entries | 8 | | high | E-ui-001 |
| reset slot tribe | -1 (unset) | | high | E-ui-001 |
| host slot 0 on LAN / internet screen | human, tribe 0, team 0, colour 0, position 0 | | high | E-ui-001 |
| non-lobby start tribes | 0 for all 8 | | high (values), medium (which modes use it) | E-ui-001 |
| setup entry default | tribe 0, team -1, colour = index | | high | E-ui-001 |
| player tribe = setup tribe | | | high | E-ui-001 |
| tribe, strength, colour by a click | (T + 1) mod 3, (s + 1) mod 3, (c + 1) mod 8 | | high (UI-SETUP-1.8) | E-ui-027 |
| new computer slot i of a single-player free game | computer, strength 0, tribe 0, position i, colour i, team i | | high | E-trn-005, E-ui-028 |
| slot made a computer in its list | computer, strength 0, tribe 0, position / colour / team -1, then the row's values | | high | E-trn-005, E-ui-028 |
| start checks | all slots ready, start positions distinct; tribe not checked | | high | E-trn-005 |
| map player record field 4 unused by setup | | | medium (absence in the traced paths) | E-ui-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | free game, slot 0 human with tribe 2 (Chinese), slot 1 computer with tribe 1 | start | player 0 tribe 2, player 1 tribe 1; a sawmill site of player 0 uses the Chinese construction positions |
| T2 | LAN setup screen opened by the host | inspect slot 0 | human, tribe 0, team 0, colour 0, start 0 |
| T3 | any free-game map | start a free game with all slots on tribe 0 | all players tribe 0, whatever the map's player record field 4 holds |
| T4 | game started outside the lobby (campaign map) | inspect players | all tribe 0 |
| T5 | computer slot strength 1 | start | the player uses AI profile 2 |
| T6 | single-player free game created | inspect slots 1..5 | computer, strength 0, tribe 0 |
| T7 | host changes an open slot to computer while the row shows tribe 2 | inspect slot | computer, tribe 2 |
| T8 | setup record entry with a player and tribe -1 (forced) | start in the implementation | player tribe treated as 0 (UI-SETUP-2.6) |
| T9 | campaign mission 3 started from the campaign menu | inspect players | map `campaign03`, player 0 human, all tribe 0 |
| T10 | free game saved with player 0 tribe 2, then loaded | inspect players | player 0 tribe 2 |
| T11 | tutorial 1 started | inspect players | map `Tutorial01`, player 0 human, tribe 0 |
| T12 | free play page, map `MP_2P_Storm_Coast` (170 x 170, start positions (29, 51) and (139, 51)); slot 0 colour 0 on start position 0, slot 1 colour 1 on start position 1 | preview markers | "1" at p = (122 + 16.72, 159 + 68.60) = (138.72, 227.60), blue (90, 90, 255); "2" at (202.13, 227.60), red (255, 90, 90); each with four black copies 2 px left, right, below and above (1.8a; `freeplay-setup-2.jpg`) |
| T13 | same map, slot 1's start drop-down (the "HQ n" list of 1.8) set to HQ 1 as well, so that no slot has start position 1 | marker "2" | still drawn, brown (128, 64, 0); marker "1" takes slot 0's colour (the first slot with position 0) (1.8a d) |
| T14 | small_no_alias_white marker "1" at p = (122 + 50, 159 + 30) = (172, 189) | glyph pixels | the quad's top-left at (168.5, 188.5) (xoffset -3, half-pixel shift), ink columns 172 .. 173, rows 192 .. 198 (UI-MENU-1.8, Q-225) |
| T15 | 1.8 (Q-331): the free play page, option row n = 1 (`!Victory condition:`) | the caption's line box | left edge x 433, top y 151 (124 + 45 - 18), bottom 167 (2 px above the bar's top at 169), baseline 164; font dark_system, white, left-aligned |
| T16 | 1.8 slot pictures: slot A human, Nubians, colour 3; slot B computer, Chinese, strength 1, colour 6; slot C open, Romans, colour 0 | the portrait, emblem and colour cells drawn | A: portrait (16 + 168 + 112, 512) = (296, 512) 46 x 80 (hovered (296, 600)), emblem (616, 8), colour (544, 240); B: portrait (16 + 336 + 56, 512) = (408, 512), emblem (568, 8), colour (520, 288); C: portrait (16, 512), emblem (520, 8), colour (520, 216) |
| T17 | 2.3, 2.4 (Q-187 i): tutorial 3 started from the main menu after a game was left | inspect players 0 and 1 | player 0 human; player 1 computer, tribe 0, no team, strength 0 (the Weak profile) |
| T18 | 1.8 (Q-211): a computer slot at strength 2 (Strong), Chinese, colour 7 | click its emblem, then its portrait, then its colour square | strength 0, emblem (520, 8); then tribe 0 (Romans), portrait (16 + 0 + 0, 512); then colour 0, square (520, 216) |
| T19 | 1.6 (a) (Q-1045): a new single-player free game on a 4-player map, no clicks | slots 1 .. 3 | computer, strength 0 (Weak), tribe 0, slot i: start position i, colour i, team i; slot 1's colour square (520 + 24, 216) = (544, 216), slot 2's (520, 240), slot 3's (544, 240); one click on slot 1's square: colour 2, square (520, 240); START without clicks: computer i plays colour i |
| T20 | 1.8 START (Q-211 f): single-player game on a 4-player map; slot 2 set to start position 1 like slot 1 | click START | nothing starts, the switch stays off, tooltip `!READY_CHECK_UNUSABLE_TOOLTIP`; after slot 2 is set back to position 2 a click starts the game; with slot 3 made open, a click does not start it |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ui-022 | code reading | (session 2026-11-12 night) the free play slot's setters for tribe and level (portrait, emblem) and for the colour (the two-wide grid), their one call when a slot is shown, and the slot record's level getter (human 2, computer its strength, open or closed 0) | 1.8, T16 |
| E-gfx-027 | code reading, data file, picture | `MP_2P_Storm_Coast.s2m` start positions; `freeplay-setup-2.jpg` | preview start markers (1.8a, Q-224) |
| E-ui-001 | code reading | `S2DNG.exe` | 6 lobby slots copied into the setup record: kind, tribe, team, colour, strength, start position |
| E-ui-001 | code reading | | lobby slot fields and setters (position, kind, strength, tribe, colour, team) |
| E-ui-001 | code reading | | slot reset (all unset); host sets a slot |
| E-ui-001 | code reading | | lobby message handler: slot change message applies tribe, team, colour, position |
| E-ui-001 | code reading | | LAN / internet screen init (host slot 0), apply slot widgets and start |
| E-ui-001 | code reading | | slot view: tribe icon picker and strength |
| E-ui-001 | code reading | | setup record defaults and accessors |
| E-ui-001 | code reading | | players created from the setup record; tribe into the player |
| E-ui-001 | code reading | | non-lobby start: all tribe 0 |
| E-ui-026 | code reading | = E-cam-007 (session 2026-11-13 night) | 2.3, 2.4: the setup reset (kind, tribe, colour, team, strength per entry) and the tutorial start (kinds only) (T17) |
| E-ui-027 | code reading | (session 2026-11-13 night) | 1.8: the free play slot's click handler (tribe, colour, strength steps) and the page's hand-over to the set-up (strength for a computer slot only) (T18) |
| E-ui-028 | code reading | (session 2026-11-14) | 1.6 (Q-1045): the single-player host start makes slots 1 .. 5 computers with start position, colour and team = the slot's number (LAN / internet: open, colour and team = the number, position unset); a kind change in the list makes the slot computer or closed with -1 and then writes the row's values; the free play slot shows the slot's own colour, with no guard against -1 (T19) |
| E-ui-030 | code reading | (session 2026-11-14) | 1.8 (Q-211 f): the slot's ready test, the host's handling of a ready request (refused while start positions repeat), the all-ready test over the map's players, the START switch and its two tooltips; UI-MENU-2.1: the map list's sort |
| E-ui-002 | code reading | | campaign menu -> non-lobby start with map `campaign` / `campaignAddOn` + number; map type 1 / 0x65 |
| E-ui-002 | code reading | | tutorial start `Tutorial0N`, map type 2, kinds only |
| E-ui-002 | code reading | | load game: slots refilled from the saved setup, entry 0 human if none |
| E-ui-002 | code reading | | start settings creation / clear -> setup reset |
| E-trn-004 | code reading | | building tribe from its owner; setup slot printout |
| E-trn-005 | code reading | | make a slot computer / open / joined human; join takes the first open slot |
| E-trn-005 | code reading | | host creates the session: slots 1..5 computer (single player) or open |
| E-trn-005 | code reading | | kind change: reset if it was human, computer defaults, then the row's values |
| E-trn-005 | code reading | | start checks: ready, distinct start positions |
| E-sim-002 | code reading | | marker anchor and refresh (1.8a), option bar text offset (Q-307) |
| E-sav-012 | code reading, data file | session 2026-11-09 c; `dark_system.fnt` (lineHeight 16, base 13) | 1.8 caption, T15 (Q-331) |

## Open questions

- When exactly the start settings are reset between games (UI-SETUP-2.4,
  tutorial case).
