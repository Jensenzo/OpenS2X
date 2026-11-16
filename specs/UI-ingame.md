# UI-GAME: In-game interface (HUD, context menus, building and global dialogs)

Status: draft (read from the binary and the data files; not checked in the running game)
Owner question(s): play-test chunk 2 (action windows, cursor, labels, speed, tutorial marks, scale); play-test 2 chunk 1 (frame, keys, cursor icon, Q-194 b); plan 6.5 (investigation chunk 2); Q-095 to Q-097 raised
Game build: 11757

## Summary

The in-game interface is built in code from sprites cut out of a few texture
atlases; there are no layout files. A main bar at the bottom centre of the
screen opens the global dialogs, the minimap, the messages and the game menu.
Clicking a flag, a free field or a building opens a context dialog next to the
bar. Every control is a sprite button with up to three state sprites and a
tooltip whose text is a key of the language file. A button's action is one of
the game commands (each named in kebab case, such as `place-flag`, by the
rule that executes it), a UI mode
change or another dialog. This spec gives the layouts (pixel positions in a
nominal screen whose sizes are divided by the real width and height), the
atlas rectangles, the tooltip keys and the command each control sends.

## Rules

### UI-GAME-1 Common rules

- **UI-GAME-1.1 Units.** Positions and sizes are given in pixels of the
  atlas art; the game divides x values by the backbuffer width and y values
  by the backbuffer height (the resolution chosen in the settings, SET-5.2),
  so dialogs keep their art pixel size at any resolution and nothing in the
  interface is scaled: on a higher resolution the bar and the windows cover
  a smaller part of the screen. (The original played at a low resolution
  stretched to a monitor looks larger than our 1:1 window.) Confidence: high
  (E-ui-004).
- **UI-GAME-1.2 Sprites.** A sprite is a rectangle (x, y, w, h) of a named
  atlas. A button has a normal, a hover and a pressed sprite; most buttons
  use the normal rect for the pressed state and a second rect for hover.
  Confidence: high.
- **UI-GAME-1.2a Atlas files (2026-10-24).** An atlas is named in the code by its bare
  name and read from `data/gfx/<name>.dds` (the folder prefix `data\gfx\` is in the
  executable; names compared without case: the code's `uiBriefing` is `uibriefing.dds`).
  Atlases named this way and not named by another rule: `uiBuildings1`, `uiBuildings2`,
  `uiChars`, `uiCredits`, `uiDepot`, `uiDepot_long`, `uiElements2`, `uiIcons`,
  `uiMediumDialog`, `uiMedium2Dialog`, `uiTip`, `uiXXLDialog_tabbed`, `ui_lobby`,
  `uiBriefing`; the rectangles taken from them belong to the windows that draw them.
  Not covered: the two loading pictures and the desktop picture (the code names them
  without the number their files carry; how it is chosen is not read). Named by nothing
  (no table, script or string of the executable) and taken as unused: `uiXXLDialog`,
  `ui_detail_building`, `ui_detail_depot`, `ui_texture`, `S2C_Logo`, the four `help_*`
  pictures (`help_bergwerke`, `help_gebäude`, `help_militär`, `help_nahrung`),
  `sphere_normal`. Confidence: medium (names
  found among the executable's strings; each load not traced). Covers (inventory): *UI
  texture* from `data/gfx/ui*` except `data/gfx/uiloading?.dds`, `data/gfx/uidesktop0.dds`,
  `data/gfx/uixxldialog.dds`, `data/gfx/ui_detail_*`, `data/gfx/ui_texture.dds`.
- **UI-GAME-1.3 Text keys.** A label or tooltip that starts with `!` is a key
  of `data/language/<lang>/controls.txt` (tab separated, `!key<TAB>text`).
  A key without an entry is shown unchanged. Help texts of a dialog are the
  `dialog<id>_<page>` entries of `dialogs.txt` (ids below), shown in a
  285 x 360 text box at (55, 41) of the help pane. Confidence: high.
  Class rule (2026-10-24): every entry of these two tables is shown through this lookup,
  so the rule stands for each of them; where and when an entry is shown is in the rule
  of its window. Covers (inventory): *UI text* from `language/gb/controls.txt`. Covers
  (inventory): *UI help text* from `language/gb/dialogs.txt`.
- **UI-GAME-1.3a The text table as loaded (measured 2026-10-24 night, E-heap-001: the heap
  dumps of E-trace-007 at trace ticks 1 and 6000, a memory read of the running game).** The game holds **one**
  table from key to text for all eight files of `data/language/gb/` (`controls`, `dialogs`,
  `briefings`, `tutorial`, `info`, `intro`, `extro`, `credits`); no key repeats across them.
  A file's first line is an entry like the others (`Controls Keys` with a version date as
  its text, `Dialog keys`, ...; in `credits.txt` the first line is the entry `cr000`).
  Reading a file:
  - the key runs to the first TAB, the text from there to the next TAB or CR (`!AI<TAB>AI`
    gives `!AI` the text "AI"); the
    character after that end is skipped (the LF of CR LF) and the next key starts after it.
    So a line that ends in a TAB gives its text without it and leaves the next line's key
    starting with the LF: in `controls.txt` `!TFTP_ERROR_READERROR` ends in a TAB, so
    `!TFTP_ERROR_WRITEERROR` is held as LF + `!TFTP_ERROR_WRITEERROR`, never found, and
    shown unchanged (1.3);
  - in a text a backslash and the character after it are replaced: `\n` by a line break;
    the only other pair in the shipped files, the `\T` of `!Time expired`, is dropped with
    its letter (so the end-of-demo text loses the T that starts a sentence there; other pairs not seen, medium).
  A lookup of a key the table does not hold adds that key with itself as its text, which is
  how a missing key comes to be shown unchanged; by trace tick 1 a free game has added
  `!Briefing`, `!Toggle Military Debug Info.` and `<empty map>`. The table does not change in
  play (equal at trace ticks 1 and 6000). Confidence: high (all 1539 entries of the eight
  files held exactly as these rules predict, T41).
- **UI-GAME-1.4 Clicks.** A button fires when the mouse button is released
  over it while it is enabled and visible; the click plays the UI sound
  `uiClick1`; opening any dialog plays `uiDialog0`; a refused action (error
  box) plays `uiCancel` and shows the message in red for 5 seconds. See
  `SND-sound-music.md` SND-4. Confidence: high. **Only the tutorial shows the red text
  (2026-11-08 b, Q-317):** every one of the error box's 67 callers passes a tutorial text
  constant (`T01_ERROR_...` and the H-key hint), and nothing else (no script function) reaches
  it, so free games and missions never show a red message; their refusals only play `uiCancel`
  (SND-4). Confidence: high (code, E-tool-005); not checked in play.
- **UI-GAME-1.5 Disabled controls** are drawn greyed and do not fire.
  Confidence: high.
- **UI-GAME-1.6 Icons of goods, tools and settlers** are 48 x 48 cells of
  `uiElements1.dds`: a cell (48 u, 48 v) with u, v the item's two icon
  indices from the item table (`items.xml`, the pair held for each item
  type of GFX-OBJ-1.6). Building icons are 80 x 80 cells of
  `uiBuildings<tribe>.dds` (0 Roman, 1 African, 2 Chinese) addressed the
  same way from the building type's two icon indices. Confidence: high.
- **UI-GAME-1.7 Fonts.** Headings in dialogs: `small_no_alias_white`, colour (126, 183, 174),
  **centred on their x** (alignment flag 8 of `UI-menus.md` UI-MENU-1.8 b). Body text:
  `light_system`, `light_system_outlined`, `dark_system` (briefing: colour (255, 220, 0)). Minimap
  title: `font_big_white`, centred. **Corrected 2026-11-14 b (F59; E-ui-033, code reading):** the
  "size 8" (and "size 4", "size 16") of the earlier readings is the text line's **alignment
  flag**, not a font size: 8 = centred on x, 4 = plain (left edge at x), 16 = right-aligned. Every
  font is drawn at the size of its own glyph sheet (`.fnt`); nothing scales it. Confidence: high.
- **UI-GAME-1.8 Dialog frame and title.** Every dialog draws its frame, then
  its title (the `!` key given by the dialog) centred at (w / 2, 14) in
  `font_big_white` size 8, colour (0.996, 0.937, 0.486). That font's glyph
  sheet holds only capitals and digits, so titles appear in capitals:
  `!Flag Menu` reads FLAG MENU, `!Street Menu` reads ROAD MENU. (The "size 8" is the centring flag of
  1.7; like every label the title is also centred vertically on its y, `UI-menus.md` UI-MENU-1.8 b, so
  its capitals' ink runs from about y 8 to y 19, inside the frame's header bar; seen in the pictures
  of E-trace-026.) A close
  button `uiElements3` (415, 32, 24 x 24) sits at (w - 40, 5) with tooltip
  `!Close this dialog.`; opening plays `uiDialog0`. The frame art comes from
  two sheets, drawn 1:1 from the top-left of a 512 x 512 texture:
  `uiBigDialog.dds` holds a 368 x 458 px panel (a 30 px dark marbled header
  bar over a teal body with a thin gold border) and `uiSmallDialog.dds` a
  288 x 88 px panel (22 px header, 60 px body); the header bar object is the
  frame's width and 22 px high. The flag, road and deploy-flag menus use the
  small panel (288 x 88 = 15 + 4 x 52 + 48 + 17 by 29 + 48 + 11; the photo
  flag-menu.jpg matches it exactly), the building windows the big one.
  Confidence: high (E-ui-004, `font_big_white_00.tga`; E-pt2-001, the two sheets); medium for which code path selects
  the small sheet. **Corrected 2026-11-14 b (F59; E-ui-033, code reading; the menu dialogs in
  `UI-menus.md` UI-MENU-1.9):** each window passes a frame picture of its own and takes its size:
  the production building window (4.1) and the construction site window (4.2) `uiMediumDialog`
  (8, 8, 480 x 224), the military building window (4.3) `uiMedium2Dialog` (8, 8, 480 x 272), the
  flag, road and deploy-flag menus `uiSmallDialog` (8, 8, 304 x 96). The picture is drawn 1:1 with
  its top-left at the window's origin. `uiMediumDialog`'s dark header bar covers about the top 24 px
  of the picture; its body is teal with a thin gold border. Not the big sheet.
- **UI-GAME-1.9 Placement.** Context dialogs (flag, road, deploy-flag, build
  menu, building, military, attack, ship) open **at the click**: the
  dialog's bottom-right corner is put on the clicked screen point, so the
  window extends up and to the left of the click; its x is limited to at
  most screenWidth - 600 px, then x is clamped to [0, screenWidth - w] and y
  to [0, 0.85 screenHeight]. During the tutorial they open at (20, 20)
  instead. The global dialogs (tools, military, distribution, transport,
  options, chat, statistics) open centred: (screenWidth - w) / 2,
  (screenHeight - h) / 2. Confidence: high (E-ui-004). **Seen (2026-11-15 night, part 2, E-trace-029, 2560
  x 1440):** build menus clicked at (1480, 250) and (2450, 236) opened at y 0 and at x 1960 (= W - 600);
  a flag menu clicked at (1640, 1318) opened with its top at about 1224 (= 0.85 H), lifted above the
  main bar; a producer window clicked at (1440, 200) opened at (960, 0). **Seen again (2026-11-15 day,
  E-trace-030, 2560 x 1440):** build menus clicked at (575, 960) and (1655, 1075) show their frame's
  outer edge ending 18 px left of and 20 px above the click both times (the dialog art's transparent
  margin; the dialog's own corner is on the click); a headquarters window clicked at (1250, 450) was
  pushed to y 0. In the Seafaring tutorial the harbour and ship windows opened at the top left
  (frame edge at about (20, 20)); after the last lesson's end box was closed with Play, the windows
  opened at the click again. Confidence: high.
- **UI-GAME-1.10 Smart cursor.** While the `SMART CURSOR` option (SET-2
  field 9, default on) is on, opening a context dialog moves the mouse
  pointer onto the centre of the dialog's first button (its default action:
  flag menu "create a street", road menu "deploy flag" or "destroy the
  street", deploy-flag menu "create flag", build menu the flag button, building
  and military windows "destroy the building", site window "destroy the
  construction", attack dialog "attack"; the ship window has no target, so the
  pointer stays); the pointer is clamped to 1 % .. 99 % of the window. Starting road building (3.4) moves the pointer onto
  the flag's projected screen position. The jump stores the pointer's old position in the
  dialog, but nothing reads it back (answered 2026-11-08 b, Q-336; E-gfx-041: the two stored
  values are written only by the jump, and no interface code reads them): the pointer
  never returns to its old place when a dialog closes (map menus put it on their field,
  1.10a). Confidence: high (E-ui-004); medium for "never read"
  (an indirect read through a copy of the dialog cannot be excluded by the search).
- **UI-GAME-1.10a Pointer when a map menu closes (play test 11; E-ui-011).** The game screen keeps the
  **map field under the pointer**, refreshed every frame while the pointer is
  over the map with no dialog under it; a map click opens its menu for that
  field and the flag and building commands of the menus use it. When the
  **deploy-flag menu** (3.6) closes by any of its ways, `!Create Flag.`,
  `!Cancel street construction.` or a click outside / the right button, and
  when the **build menu** (3.2) closes after a type or its flag was clicked, or by
  a close request (1.10b), the pointer
  is **warped onto the projected screen position of that field** (its centre
  point), if the field projects on screen; otherwise the pointer stays where it
  is. So after `!Create Flag.` during road building the pointer sits on the
  new flag at the route's end, the field the player had clicked, and the road
  is then finished by the logic (3.4 step 4). The pointer position that the
  smart cursor saves when it jumps (1.10) is never read back by these menus:
  no menu restores the pointer to where it was before the jump (Q-336).
  Confidence: high.
- **UI-GAME-1.10b Every way of placing a flag, and the pointer (F41, Q-262,
  2026-10-23; E-sim-005: the callers of the flag command and of the warp).** A flag
  is placed by exactly three menus and by no key: (1) the deploy-flag menu's
  `!Create Flag.` (3.6): the command is sent, the pointer is warped onto the field
  (1.10a) and the menu closes, all in the click's frame; (2) the road menu's
  `!Deploy flag.` (3.3, a flag on a road): command, close and warp in the click's
  frame; the road menu warps the same way when it closes by `!Destroy the street.`
  or by a click outside / the right button, so **every exit of the road menu warps
  the pointer** (Q-262 a); (3) the build menu's flag button (3.2): the flag command
  is sent for the menu's field, the menu closes and the pointer is warped onto that
  field (1.10a), all in the click's frame, exactly as after a building type: **the
  flag button counts as one of the build menu's types for the pointer** (Q-262 b).
  The button only reacts while the menu offers a flag at its field. The build menu
  warps on every exit: flag, a type, or a close request (a click outside / the right
  button); only its tab buttons (pages) leave the pointer alone. In the tutorial its
  click handler sends the same flag command, then closes, moves the camera by script
  and warps. The building windows (construction site, military, warehouse) warp too
  when they close (their destroy button, which then opens the confirmation dialog,
  or a close request). Confidence: high (E-sim-006: the build menu's close request, flag button and type buttons).
- **UI-GAME-1.10c No double click (F50, Q-288, 2026-10-29; E-ui-012).** The
  original has **no double-click rule** of any kind: the game's window class
  does not ask for double-click messages, the game never reads the system's
  double-click time or rectangle, and its input layer keeps, per mouse button
  and key, only "pressed since the last frame", "released since the last frame"
  and "held for n frames", with no clock and no position. What players do as a
  "double click" to place a flag is two ordinary clicks:
  1. the first left press on the map is read in the next frame update of the
     game screen (1.10a): the menu for the hovered field opens in that update
     and, with the smart cursor on, the real pointer is moved onto the menu's
     first button (1.10): the build menu's flag button, the road menu's
     `!Deploy flag.`, the flag menu's "create a street", the deploy-flag menu's
     `!Create Flag.`;
  2. the next left click is an ordinary click on that button. It counts in any
     later frame update while the real pointer is inside the button (the hit
     test is the button's rectangle at the pointer's position, so there is no
     distance tolerance beyond the button itself, and moving the mouse off the
     button first makes it a click elsewhere), however long after the first
     click (no time limit), and never in the frame update that opened the menu.
  Clicks are seen once per frame update, not per message: a press whose release
  also arrives before the next frame update clears its own "pressed" mark, so
  the map (which reads "pressed") does not see a click held for less than the
  rest of a frame; such a click only matters at low frame rates (at 60 frames a
  second it must be shorter than about 16 ms). The four menus behave alike;
  without the smart cursor the second click only places the flag if the player
  puts the pointer on the button themselves. Confidence: high for the absence of a
  double-click rule and the two-click mechanism (code); medium for the frame of
  the menu's first possible click (the first frame update after the one that
  opened it, read, not measured) and for whether the menus' buttons react to the
  press or to the release (both are frame-based).

- **UI-GAME-1.11 The paused view (Q-292, 2026-10-30; E-ui-013).** While
  the game is paused (P, UI-GAME-12) the screen gets a layer of its own over
  everything else, the main bar included: the image `uiTip` (`gfx/uiTip.dds`, 16 x
  16, every texel opaque black) stretched over the whole screen, rectangle (0, 0)
  to (1, 1) in screen fractions, with the colour (1, 1, 1, 0.65): black at alpha
  0.65 blended over the frame, so every pixel keeps 0.35 of its colour; then the
  text `!Pause` ("Paused") in the font `light_system`, centred at (0.5, 0.5).
  Nothing else changes: the world's light, the fog and the frame itself are as
  unpaused. Confidence: high for the image, colour, font and position (code, and
  the measured darkening: the world of look-compare 2026-10-30 `weak-001500` is
  0.36 of ours on average, the main bar about 0.2); medium for the text size (the
  font's default) and whether the bar is under the layer or only darker art
  (the measure says under).
  **Exact blend and draw order (F53, 2026-10-31, E-ui-014).** The layer's colour is packed per vertex as clamp(c, 0, 1) x 255
  **truncated**, so 0.65 becomes the alpha byte 165. The fixed-function blend
  (texture x vertex colour, source alpha / one minus source alpha) on the stored 8-bit
  frame then gives **v -> v x 90 / 255** (0.3529) per channel, with the GPU's rounding.
  No sRGB state is set for any draw (GFX-VIEW-2.6 c), so the interface and this layer
  blend on stored values. Order of the paused frame:
  1. the whole 3D scene;
  2. the interface pass: the interface's windows (dialogs, the main bar, the minimap),
     each blended with its own alpha;
  3. the tooltip box;
  4. the tutorial pointer and the global text boxes;
  5. **the layer, last**, drawn only while the game screen is shown and the game is
     paused;
  6. "Paused" on top.
  So everything under the layer, the bar included, keeps 90/255. The bar answers the
  open point above: it is under the layer. Confidence: high (code).

### UI-GAME-2 Main bar (HUD)

- **UI-GAME-2.1 Bar.** Background `uiGame.dds` (0, 184, 504 x 96) at
  x = screenWidth / 2 - 252, y = screenHeight - 99 (bottom centre).
  Confidence: high.
- **UI-GAME-2.2 Buttons.** Nine 48 x 48 buttons, normal sprite from
  `uiGame` row y = 296, hover row y = 352, at bar-relative positions:

  | # | x | y | sprite x | tooltip key | action |
  |---|---|---|---|---|---|
  | 1 | 35 | 40 | 392 | `!Open Mission Target.` | opens the briefing panel (2.4) |
  | 2 | 83 | 45 | 344 | `!Toggle Game Speed.` | sends `set-game-speed` with the next speed: (floor(speed) mod 3) + 1, i.e. 1 -> 2 -> 3 -> 1 (2.7) |
  | 3 | 133 | 45 | 8 | `!Toggle construction view on/off.` | toggles the build-help overlay (GFX-VIEW-6) |
  | 4 | 183 | 45 | 56 | `!Open planning management sub system dialog (PMSSD).` | toggles window 25, the player statistics page (8.5a), not the Planning menu (8.5) |
  | 5 | 232 | 45 | 104 | `!Toggle minimap on/off.` | toggles the minimap dialog (2.5) |
  | 6 | 282 | 45 | 152 | `!Open master configuration dialog.` | toggles window 18, the tools page (UI-GAME-6.1), not the Logic menu (8.5) |
  | 7 | 332 | 45 | 296 | `!Toggle Messages On/Off.` | toggles the messages window (2.3) |
  | 8 | 382 | 45 | 248 | `!Start Chat.` | chat line (multiplayer) |
  | 9 | 432 | 40 | 200 | `!Open options menu.` | opens the game menu (`UI-menus.md` UI-MENU-3) |

  Confidence: high for sprites, positions and tooltips, and for the actions of 4 and 6 (E-ui-015,
  2026-11-04); medium for the actions of 3 and 8 (named from the tooltip and the dialogs that exist).
  **Seen (2026-11-15 day, E-trace-030, 2560 x 1440, pointer on each button):** the English texts
  shown are, for 1 to 9: "View Mission Objective and Status", "Toggle Game Speed", "Toggle
  Construction View on/off", "Open Statistics", "Toggle Minimap on/off", "Open Global Economy
  Settings", "Toggle Reports on/off", "Start Chat", "Open Game Menu"; the hovered button turns
  red-orange. The tooltip of a control (these buttons, the build menu's cells and tabs, a window's
  buttons) is a dark box with a thin light frame, one line of white text, 32 px high, its top left
  corner at the pointer + (25, -45) (so its bottom 13 px above the pointer), its width fitted to the
  text; near the right edge it is moved left so that it ends at the screen's edge (the minimap's
  close button). Measured on 13 controls, +- 1 px. The map's hover tooltips are placed by 9.3, not by
  this. Confidence: high for the texts and the box at this screen size; medium for how the offset
  scales with other screen sizes (one size seen).
- **UI-GAME-2.3 Messages window.** (Superseded in detail by UI-GAME-11:
  seven visible rows.) Eight message row slots (a round slot with the
  message icon, a tooltip and a label) with three buttons: scroll up, scroll
  down (`!Scroll down.`) and `!Jump to last one.`. A new message plays
  `uiMessage0` or `uiMessage1`; removing a message plays `uiRemove`.
  Confidence: medium (row content not traced).
- **UI-GAME-2.4 Briefing panel.** `uiGame` (472, 8, 216 x 80) at bar.x - 182,
  y = screenHeight - 75, with two text lines at (184, 31) and (184, 50)
  (mission text). Confidence: medium. **Seen (2026-11-15 night, E-trace-028, campaign01 at 2560 x
  1440, opened with T):** a small scroll left of the main bar; the two lines are **right-aligned** on
  x 184 (ending at screen x 1030) and centred on y 31 and 50 (screen 1396 and 1415), dark text:
  `Reach the portal.` and, once the target is met, `(VICTORY)` on the second line. T also reopens
  the mission's book (CAM-1.3). Confidence: high for the places (picture). **Target 3, produce
  coins (2026-11-16 day, Q-1063; E-cam-017, code reading):** the first line is
  `!MissionTarget: Produce coins` (English "Collect gold coins.") and the second `(<n>/100)`, where
  n is the **local player's own** coins-produced figure (`CAM-campaign-missions.md` CAM-3.3), not
  his team's sum, so in a team the line can read below 100 when the team has won; the lines are
  refreshed every half second. Confidence: high for the figure and the format.
- **UI-GAME-2.5 Minimap dialog.** Frame `uiGame` (688, 320, 168 x 320) at
  x = screenWidth - 326, y = 10; title `!Map` at (128, 21) of the frame; the
  map image 265 x 265 at (-30, +36) of the frame: the texture `regions` (the
  minimap, side 256 or the map size when larger), `cities` (the players'
  areas) and the fog-of-war texture, with the 32 x 32 `uiMinimapCursor`
  sprite at the camera's position; a toggle button `uiGame` (440, 296,
  48 x 48) `!Show/Hide Borders.` at (screenWidth - 50, frame.y + 225); a
  close button `uiElements3` (415, 32, 24 x 24) `!Close this dialog.` at
  (screenWidth - 27, frame.y - 23). Confidence: high for layout; the
  content of the two map textures is `GFX-world-view.md` (fog) and open. **Seen (2026-11-15 night,
  E-trace-028, M in Jens's 2023 save at 2560 x 1440):** what shows is a dialog in the top-right corner
  about 325 x 302 px (from x about 2235 to the screen's right edge, y about 18 to 320) with the standard
  header bar and the title MAP centred in it, the close button at its top right, a black square
  holding the map (about 265 px, from about x 2265, y 48) and the round borders button at its bottom
  right (about (2530, 290)); so the map image lies about 30 px to the right of the frame's left edge
  (the "-30" above reads as +30), and the frame is wider than the 168 px of the `uiGame` piece named
  above. Confidence: medium (one picture, edges read by eye +- 5 px). **Measured (2026-11-15 day,
  E-trace-030, M after the Seafaring lesson's end, 2560 x 1440, pixel rows read):** the frame's light
  outer line runs at x 2240 .. 2242 on the left (w - 320) and x 2556 .. 2558 on the right (w - 4), y 18
  .. 21 at the top and y 319 .. 322 at the bottom; the header bar fills y 22 .. 42 with MAP centred; the
  black map area inside spans x 2243 .. 2555, y 43 .. 318; a laurel ornament sits on the bottom left
  corner. The darker pixels from x 2234 and y 10 outside that line fit the frame placed at (w - 326, 10)
  with a 6 px transparent margin in its art. The close cross sits at the header's right end, its centre
  at about (w - 17, 31), tooltip "Close this window"; the round button with a house is centred at
  about (w - 28, 292), tooltip "Show/hide buildings and borders". The map area has no tooltip. A left
  click on the map moves the camera there (seen). During a tutorial lesson M and the bar's minimap
  button did nothing; after the lesson's end (Play) M opened it. Confidence: high for these places at
  this screen size (pixels); the two button places above (2.5's (w - 50, frame.y + 225) and (w - 27,
  frame.y - 23)) do not match what is seen and are superseded by these.
- **UI-GAME-2.6 Debug controls.** A column of 16 x 16 buttons under the bar
  (debug session, hide UI, camera mode, game speed, grabbing mode) and four
  debug panels (AI, harbour, flag, military) exist but are disabled in the
  shipped build. An implementation may omit them. Confidence: high.

- **UI-GAME-2.7 Game speed indicator.** The game speed is 1, 2 or 3 (times
  normal); the speed button (2.2) and the V key (UI-GAME-12.1) cycle it. While the speed is
  above 1 a 50 x 50 animated picture of a settler running with a clock is
  drawn at (screenWidth - 76, 20), or at (screenWidth - 382, 20) while the
  minimap dialog is open: `uiGame` row y = 640 (clock face "2") at speed 2,
  row y = 704 (clock face "3") at speed 3, six 64 x 64 frames at x = 64 x
  (t mod 6) with t the second clock (real time, runs on while paused; `TOOL-trace.md` TRACE-5.3, aligned 2026-11-08 b, Q-231 a). Nothing is shown at speed 1.
  Confidence: high for the sprites and positions, medium for the frame
  clock and the "speed 1 hides it" condition (E-ui-004).

### UI-GAME-3 Context menus on the map

- **UI-GAME-3.1 Flag menu** (`!Flag Menu`, shown as FLAG MENU, help
  `dialog5_0`), opened at the click (1.9) by a click on an own flag. A
  ROW at (15, 29) with 52 px steps in x of round 48 x 48 portrait
  buttons from `uiElements3` (frame: the small panel of 1.8, 288 x 88): the normal picture at (x, 144), the highlighted
  (hovered) picture at (x, 192) (a brighter copy); the pressed look equals
  the normal one:

  | x | tooltip (English text) | picture | action |
  |---|---|---|---|
  | 0 | `!Create a street.` (Build road) | settler with a shovel | enters road-building mode from this flag (3.4); the smart cursor lands here |
  | 48 | `!Create a Waterway.` (Build waterway) | night harbour with lighthouse | enters waterway-building mode; enabled only when the flag's field is marked water-adjacent in the map's field flags |
  | 96 | `!Destroy the flag.` (Destroy flag) | white flag with a red arrow | sends `remove-flag` for the flag and its roads; disabled for the headquarters' flag |
  | 144 | `!Call a scholar!` (Send out geologist) | old man with a book | sends `send-geologist` |
  | 192 | `!Call the explorer!` (Send out scout) | dark-haired man | sends `send-scout` |

  **Every exit of the flag menu puts the pointer back on the flag (2026-11-16 day, Q-207 h; E-ui-041,
  code reading, high).** Whatever closes it, the menu closes and the pointer is warped onto the
  flag's field (1.10a): each of the five buttons, a close request (a click outside, the right button),
  and the flag gone while it is open. With the destroy button the warp comes first and then the
  confirmation dialog opens (when the game asks for one). The geologist and scout buttons first ask
  the game whether the order can be given; when it cannot, `uiCancel` plays, no order is sent, and
  the menu still closes with the warp (what that check tests was not read).

  Confidence: high (E-ui-004, atlas `uiElements3.dds`).
- **UI-GAME-3.2 Build menu** (`!Build Building or Flag`, help `dialog0_0`),
  opened at the click (1.9) by a click on a free own field whose site kind
  is small, medium, large, mine or harbour (a flag-only field opens the road
  menu 3.3). A tab row at (15, 25) stepping 40 px with the icon tabs (3.5):
  small (id 5, `!Show small buildings`) for the kinds small, medium, large
  and harbour; medium (6) for medium, large and harbour; large (7) for
  large and harbour; a mine site shows no tabs. The menu **opens on the
  small page** whatever the site kind (on the mine page for a mine); the
  tab of the shown page is drawn pressed and disabled, a click on another
  tab rebuilds the grid for that page. The grid starts at (23, 60) with
  80 x 80 cells, four per row, left to right then down. **The first cell
  is the flag button**: slot `uiElements3` (0, 0, 80 x 80) with the flag
  icon `uiBuildings0` (0, 0, 80 x 80) on it (**for every tribe**: the menu names that sheet
  itself, it does not take `uiBuildings<tribe>` as the building cells do; Q-209 a, E-ui-021), tooltip `!Create flag at
  position.` (Erect flag), sends `place-flag`(field); the smart cursor
  (1.10) lands on it; when no flag fits it is disabled with `!Flag too close
  to another flag.`. The following cells list, in the order of the types'
  menu index (at most 40), the building types of the page's size only:
  small page size 0, medium page size 1, large page size 2 (on a harbour
  site also the harbour, size 4), mine page size 3; the headquarters is
  never listed. **(Play test 11.)** There is **no military page**: a military
  type is listed on the page of its size, among the other types, in the
  order of the types' sort index (`buildings.lua`, `bp_setSort`). The
  **headquarters (castle) can never be built by the player**: the menu skips
  it, its construction time is -1, and it exists only from the map's start
  positions. Its icon cell is (0, 0), the same 80 x 80 cell of
  `uiBuildings<tribe>` that the flag button shows, so a menu that listed it
  would show the flag icon twice (`playtest11-jens/military-tab.png`). The
  pages with their cells (icon = `uiBuildings<tribe>` rectangle (80 col,
  80 row, 80 x 80); the flag cell comes first on every page):

  | page | cells after the flag, in order (type: icon col, row) |
  |---|---|
  | small | woodcutter 1, 1; forester 2, 0; stone pit 3, 0; fisher 1, 0; hunter 0, 1; lookout tower (`WatchTower`, the explorer's tower) 5, 0; well (`Spring`) 2, 1; barracks 4, 0; guard house 3, 1 |
  | medium | sawmill 1, 2; slaughterhouse 4, 2; mill 5, 1; bakery 2, 2; smelter 2, 3; locksmith 4, 1; smithy 1, 3; mint 3, 2; shipyard 4, 3; brewery 3, 3; storehouse (`Depot`) 0, 2; catapult 0, 3; watchtower (`Tower`, 6 soldiers) 5, 2 |
  | large | farm 1, 4; pig farm 5, 4; donkey breeding 2, 4; (harbour 3, 4 here, harbour sites only); fortress 4, 4 |
  | mine | iron 1, 5; gold 3, 5; coal 2, 5; stone 0, 5 |

  Confidence: high for the skip, the sizes and the icon cells (data); medium
  that the sort index of the data is the menu order read by the code (the
  only order field the data sets). Each cell is the same slot with the type's icon (1.6) and
  the type's name as tooltip (the name and the costs while hovered). A type
  the map forbids is disabled with `!Building not allowed`; a military type
  too close to another military building is disabled with `!Too close to
  other military buildings.`. **Which cells and why (2026-11-16 day, E-eco-047, code reading,
  high):** the military cells are the types with a territory range (barracks, guard house,
  watchtower, stronghold; the lookout tower is not one). Each is tested on the menu's field by
  `ECO-building-placement.md` ECO-PLACE-6.2: it is greyed when a building or construction site
  of **any player** whose type has a territory range (the headquarters included) stands on a
  field within distance 4 (rings 0 to 4). The test runs when the page is built (opening the
  menu, a tab click) and comes before the `!Building not allowed` test. The non-military cells
  of the same page are not affected, and the tabs are not either (ECO-PLACE-4). The
  `place-building` command refuses the same types by the same test (ECO-PLACE-6.3), so a
  greyed cell is never placed by another way. Test: ECO-PLACE T18, T19. **Seen (2026-11-16 night, E-trace-031):** the disabled cell's tooltip
  box carries that text, "Too close to other military buildings", in place of the name (the cells near
  the headquarters); near the border the same cells were enabled with "Barracks (0)". Clicking a type sends
  `place-building`(field, type) and closes the menu (the pointer returns
  to the field). Confidence: high (E-ui-006). **Seen (2026-11-15 day, E-trace-030, Seafaring map after
  the lesson's end and a free game on Coastal Storm as Africans and as Chinese, 2560 x 1440, a large
  site):** title "BUILDING SELECTION MENU"; the three tabs' tooltips are "Small buildings", "Medium
  buildings", "Large buildings"; the flag cell's tooltip is "Erect flag"; a building cell's tooltip is
  the type's name followed by a number in brackets, e.g. "Woodcutter's Hut (0)", "Forester's Hut
  (0)", "Quarry (0)", "Fisherman's Hut (0)", "Hunter's Hut (0)", "Lookout Tower (0)", "Well (0)",
  "Barracks (0)", "Guard House (0)" (a player owning none of these; that the number counts the
  player's buildings of the type is taken, not seen with a non-zero count: medium); no costs show
  anywhere in the menu while a cell is hovered. The hovered cell gets a bright teal frame. The small
  page lists the flag and the nine types of the table above in that order (4 + 4 + 2 cells); the
  African and Chinese menus have the same tabs, cells, order and counts per page (small 9, medium
  14, large 4 after the flag), each cell with the tribe's own building picture, and **the flag cell
  is the same eagle with SPQR for all three tribes** (Q-209 a seen). Confidence: high (pictures).
- **UI-GAME-3.3 Road menu** (`!Street Menu`, English text "Road Menu", help
  `dialog11_0`), opened at the click (1.9) by a click on a road **or on a
  free own field whose site kind is "flag only"** (ECO building placement
  class 1; a free field of a higher class opens the build menu 3.2): the
  same row layout as 3.1 (buttons from (15, 29) stepping 52 px in x) with
  up to two buttons: `!Deploy flag.` (Erect flag),
  picture `uiElements3` (368, 160) normal / (416, 160) highlighted (a flag
  with a hammer), present only when a flag fits on the clicked field, sends
  `place-flag` there; `!Destroy the street.` (Destroy road), picture
  (240, 144) / (240, 192) (settler with a pickaxe), sends `remove-road`.
  Confidence: high for sprites and actions, medium for the condition that
  hides the destroy button (E-ui-004). **Seen (2026-11-15 night, part 2, E-trace-029):** a click on a
  dirt road where no flag fits opened ROAD MENU with the destroy button alone (tooltip "Destroy
  road"); a click on it removed the road at once.
- **UI-GAME-3.6 Deploy-flag menu** (`!Deploy Flag Menu`, English text "Flag
  Menu", help `dialog6_0`), opened at the click (1.9) while a road is being
  built by a click on the route's current end point (3.4), with the row
  layout of 3.1: `!Create Flag.` (English "Erect Flag"), picture
  `uiElements3` (368, 160) normal / (416, 160) highlighted (the flag with a
  hammer of 3.3), present only when a flag fits on that field, sends
  `place-flag` there; then `!Cancel street construction.` ("Cancel road
  construction"), picture (240, 144) / (240, 192) (settler with a pickaxe),
  sends `cancel-road`. The smart cursor lands on the first
  button present. Outside road building the same dialog shows only
  `!Create Flag.` (not opened by a map click then). Jens's photo of
  the flag spot's menu (FLAG MENU, "Erect Flag" plus
  a second button) is this dialog. Confidence: high (E-ui-005). **Seen (2026-11-15 night, part 2,
  E-trace-029):** titled FLAG MENU; at a route end where a flag fits, two buttons (the flag with a
  hammer, then the settler); where none fits, the settler alone (cancel); the flag button placed the
  flag and finished the road.
- **UI-GAME-3.4 Road building** (help `dialog6_0`), step by step:
  1. `!Create a street.` (or `!Create a Waterway.`) in the flag menu sends
     `start-road`(flag field, waterway). The logic creates
     the player's route (TRN-ROAD-8.1) and, for the local player, the game
     enters road-building mode: the pointer is warped onto the flag (1.10),
     the hovered field shows the road cursor icon (UI-GAME-13.2) and the ring
     with the neighbour markers (GFX-VIEW-6.2).
  2. A left click on any field F other than the route's own points sends
     `extend-road`(F) and plays `uiClick0`. The logic extends
     the route from its current end to F along the cheapest route
     (TRN-ROAD-8.2), possibly by many fields at once; the route is drawn as
     a normal road while it grows (TRN-ROAD-8.4). A click with no route to
     F changes nothing. A click on a field that is already part of the route
     cuts the route back to that field (TRN-ROAD-8.2). **The clicked field is tested first
     (2026-11-16 day, Q-203 b; E-ui-040, code reading, high):** the interface sends
     `extend-road` for every such click with no test of its own, and the logic runs TRN-ROAD-7.4's
     target test on F before any search; a field that fails it (a land field next to a flag,
     Border, a doodad field, too far for a waterway) extends nothing, also when it lies between
     the route's end and a field that would pass. The click sound plays all the same, and road
     building goes on. The neighbour shortcut of TRN-ROAD-2.5 comes after this test.
  3. When the route's end is a flag, or a field of another road where a
     flag fits, the road is built at once (TRN-ROAD-8.3) and the mode ends
     (the pointer is not moved). A click on an existing flag therefore ends
     the road there in one step.
  4. A click on the route's current end (its last field) opens the
     deploy-flag menu (3.6): `!Create Flag.` sends `place-flag` there and
     the road is finished by the logic on the next tick (TRN-ROAD-8.3);
     `!Cancel street construction.` sends `cancel-road`.
     Either way the pointer is warped onto that end field when the menu
     closes (1.10a).
  5. A click on the route's first field (the start flag), or the right
     mouse button while the route cannot be finished, cancels: `uiCancel`
     plays, `cancel-road` is sent and the mode ends. **"Cannot be finished" (Q-203 c; E-ui-040,
     code reading, high)** means the route is not complete in TRN-ROAD-8.3's sense. A complete
     route is built at once after the extension that completes it, so it stays complete only
     from a flag placed on its end (step 4's `!Create Flag.`) until the next tick builds it. In
     practice every right click in the mode cancels; in that short window the right click does
     nothing (no sound, the mode stays until the road is built). Escape
     does not cancel (it opens the game menu, 12.1); while the mode is on,
     a route that the logic has dropped (its start flag destroyed, a field
     blocked) ends the mode with `cancel-road`.
  Confidence: high (E-ui-006).
- **UI-GAME-3.5 Icon tabs** are 40 x 27 buttons from `uiElements0`: normal
  (8 + 40 id, 416), hover (8 + 40 id, 448), selected overlay (8 + 40 id,
  480). Ids: 5 small, 6 medium, 7 large buildings, 9 depot goods, 10 depot
  settlers, 11 military management, 12 expedition, 13 tools, 14
  distribution, 15 transport, 16 military. Confidence: high.

### UI-GAME-4 Building windows

Opened by a click on an own building; the title is the building's name.
Headings use the heading font (1.7). The window is placed next to the bar.

- **UI-GAME-4.1 Production building.** Labels `!BUILDING` at (64, 37),
  `!NEEDED GOODS ` at (364, 37) when the type has inputs, `!PRODUCTIVITY` at
  (136, 106) above a productivity gauge at (136, 112), `!TOOL` at (232, 37)
  with the worker's tool icon (1.6) in a slot at (208, 43), `!PRODUCT` at
  (160, 37) with the product icon in a slot at (136, 43); two text lines
  at (40, 160) and (40, 182) (4.1a); a portrait frame
  `uiElements3` (344, 216, 88 x 88) at (22, 43) showing the building model
  (80 x 80 at (26, 47)); separators `uiElements3` (0, 432, 456 x 8) at
  (2, 143) and (160, 0, 8 x 56) at (192, 32) and (264, 32). Buttons in a row
  from (407, 157) leftward in 50 px steps (4.1a d), from `uiElements5`:
  - destroy (448, 88) / hover (448, 184), `!Destroy the building.`: sends
    `remove-building` only: the building goes with remains (`MIL-conquest.md`
    MIL-CON-4.1b), and its flag and the flag's roads stay (corrected
    2026-11-15 day, E-mil-014: the same destroy action sends remove-flag or
    remove-road only when a flag or a road is what is selected);
  - production switch (208, 88), `!Toggle production ON/OFF.` (catapult:
    `!Toggle bombardment ON/OFF.`): sends `switch-production`; disabled for
    types without a worker; the caption `!Production stopped.` /
    `!Bombardment stopped.` is shown while off;
  - shipyard only: (256, 88) `!Toggle between boat and ship.`: sends
    `toggle-production` (the shipyard's mode, ECO-FOOD-7).
  Input slots: up to three (six for the mint) 48 x 48 slot frames
  `uiElements3` (440, 216) from (395, 43) stepping 60 px to the left, each
  with the input good's icon and its stock count. Confidence: high.
- **UI-GAME-4.1a Captions and status lines (2026-11-14 b, F59; E-ui-033, code reading).** (a) The
  frame is `uiMediumDialog`, 480 x 224 (1.8). (b) **Captions:** the headings are centred vertically on y 37
  (`UI-menus.md` UI-MENU-1.8 b), `small_no_alias_white`, colour (126, 183, 174),
  **centred** on their x: `!BUILDING` on 64 (the portrait frame 22 .. 110), `!PRODUCT` on 160 (its
  slot 136 .. 184), `!TOOL` on 232 (its slot 208 .. 256), `!NEEDED GOODS ` on 364. `!PRODUCTIVITY` is
  the one **plain** (left-aligned) caption: its left edge at x 136, the gauge's left edge, centred
  on y 106, the gauge at 112. (c) **The two status lines** are `light_system` texts with their left
  edge at x 40 (plain), centred on y 160 and y 182; they are not a worker activity:
  - line 1 (y 160): `!Production stopped.` ("Production stopped") while the building's production
    switch is off (the catapult: `!Bombardment stopped.`); empty while it is on;
  - line 2 (y 182): `!No worker at home.` ("No worker in building") while no worker is inside the
    building (the count of workers at home is 0), for a type that has a product, the forester and
    the lookout tower; empty otherwise.
  Both are set again on every refresh of the window. So a working woodcutter's hut with its worker
  out felling shows line 2 and an empty line 1; with the worker inside, both lines are empty. The
  earlier reading "`!No worker at home.` at (28, 190)" was a position object the code makes and drops;
  the text goes into line 2. (d) **Buttons:** a row from x 407 leftward in 50 px steps at y 157:
  destroy (407), production switch (357), and for the shipyard the boat / ship switch (307).
  (e) **As the original shows it (E-trace-026, a woodcutter's hut with its worker at home, 2560 x
  1440):** the window's frame top-left is its origin; the title WOODCUTTER'S HUT centred inside the
  header bar; BUILDING, PRODUCT, TOOL with their ink at y 34 .. 40, centred on x 64, 160, 232, just
  above the portrait (22, 43) and the slots (136, 43), (208, 43) (every label is centred vertically on
  its y, `UI-menus.md` UI-MENU-1.8 b); PRODUCTIVITY with its left edge at x 136 and its ink at y 103 ..
  109, above the gauge at 112, which shows the value as `100%` centred in the bar; three empty slot
  frames at x 275, 335, 395 (y 43) although the type has no input (only the caption `!NEEDED GOODS `
  is left out); the separator at y 143; both status lines empty (production on, worker inside); the
  production switch at (357, 157) and destroy at (407, 157). The status lines' y 160 and 182 are
  their lines' middles. Test T57. Confidence: high (the window's build and refresh read; the picture).
  (f) **Seen 2026-11-15 night (E-trace-028: sawmill, slaughterhouse, well and farm windows at 2560 x
  1440):** `!NEEDED GOODS ` reads REQUIRED GOODS; an input's slot shows the good's icon and under it its
  count as **`<stock>/<capacity>`** (`0/6` for the sawmill's wood, `2/6` for the slaughterhouse's pigs)
  centred under the slot on about y 100 (the coin count's style of 4.3); a single input sits in the
  rightmost slot (395, 43). **A type whose worker needs no tool shows no `!TOOL` caption, no tool slot
  and no separator after it** (the well: BUILDING, PRODUCT, then the three empty slots). The window's
  body is translucent: the map shows faintly through the teal. Confidence: high (pictures).
  (g) **Seen 2026-11-15 night, part 2 (E-trace-029: coal mine, bakery, shipyard and catapult windows at
  2560 x 1440).** **Which slot an input takes:** the inputs are the product's costs in the data's order
  (`gp_addCost` in `Goods.lua`); the **first input takes the rightmost slot (395)**, the next ones go
  leftwards (335, then 275); a cost that is a goods group (`food`) stands for its member goods in their
  sort order (`gp_setSort`: fish 6, flesh 7, bread 8). So the bakery (bread costs water, then flour)
  shows water `2/6` at 395 and flour `4/6` at 335 with 275 empty, and the coal mine (coal costs `food`)
  shows fish `1/2` at 395, meat `2/2` at 335 and bread `2/2` at 275; a mine's food counts read against
  2, the amount a mine orders of each food (ECO-PROD-6.1). **The productivity gauge** is the site
  window's gauge (4.2): x 137 .. 442, y 113 .. 129 in the window (about 306 x 16, +- 2), filled cyan
  from the left by the value with the percentage centred on it (`0%` for the coal mine, `100%` for the
  bakery and the shipyard). **Shipyard:** PRODUCT shows the boat while the yard builds boats and the
  ship while it builds ships; TOOL the hammer; planks `5/6` at 395; three buttons at 307 (the boat /
  ship switch, a sailing ship picture, tooltip `!Toggle between boat and ship.`, English text "Toggle between raft and ship production"), 357 (production) and 407 (destroy); a click on the switch changes the product's icon at once.
  **Catapult:** title CATAPULT; only `!BUILDING` and the portrait at the left; no PRODUCT, TOOL or
  REQUIRED GOODS caption, no product or tool slot, no `!PRODUCTIVITY` and no gauge, but both vertical
  separators of those columns are drawn; stones with `4/4` under them in the rightmost slot (395); the
  separator at y 143; status lines empty; two buttons, production (357) and destroy (407). **Coal mine:**
  title COAL MINE; PRODUCT coal, TOOL the pick-axe; both status lines empty although the mine had found
  nothing (the "no resources" news is a message, 11.1). Test T60. Confidence: high (pictures; the order
  rule from two types and the data).
- **UI-GAME-4.2 Construction site** (help `dialog27_0`): `!NEEDED GOODS ` at
  (364, 37), `!BUILDING` (64, 37), the gauge (completion) at (136, 112), the
  portrait, and one button (448, 88) `!Destroy the Construction.`
  (`remove-building`). Confidence: high. **Seen 2026-11-15 night (E-trace-028, a woodcutter's hut
  site just placed, 2560 x 1440):** frame `uiMediumDialog` 480 x 224 like 4.1, title the type's name
  (WOODCUTTER'S HUT); the portrait shows the site (levelled ground); REQUIRED GOODS centred on (364, 37)
  with, under it, one slot per good still to come: the planks' icon in a slot at about (380, 42) and the
  number still needed as **`x<n>`** (`x2`) to the right of the slot, centred on about (447, 73); the
  completion gauge from (137, 112) to (443, 128) with `0%` centred in it; status line 1 (4.1a: left edge
  x 40, centred on y 160) reads `!Construction in progress...`; the destroy button at (407, 157) (the
  button row of 4.1a d), no production switch. Confidence: high for what is shown, medium for the
  place of further goods' slots (only one good seen).
- **UI-GAME-4.3 Military building.** Headings `!COINS` (392, 37),
  `!SOLDIERS INSIDE` (210, 37), `!BUILDING` (64, 37). Button column at
  (407, 212): destroy (448, 88); coins toggle (304, 88) `!Toggle coin supply
  ON/OFF.` (`toggle-coins`); move-out toggle (208, 232) `!Toggle
  Moving soldiers out.` (`toggle-soldiers-out`); upgrade
  (256, 232) `!Upgrade Building` (`upgrade-building`, MIL-UPG), whose
  tooltip becomes `!Upgrade Running` while an upgrade runs, `!Building not
  allowed` when the next size is forbidden on the map, `!Upgrade not
  possible` when there is no larger type, `!Upgrade no space` when the
  larger site does not fit; in those cases the button is disabled. A coin
  slot at (369, 45) shows the coin icon and the stock. Soldier slots: one
  48 x 48 slot per soldier the type holds at (137, 45) stepping 56 px,
  three per row, 56 px between rows, each showing the soldier's rank icon
  (1.6). Confidence: high. **Frame and captions (2026-11-14 b, F59; E-ui-033, code reading):** the
  frame picture is `uiMedium2Dialog` (8, 8, 480 x 272) (1.8); the three headings are centred on their
  x (392, 210, 64) and on y 37 (ink 34 .. 40); the coin stock is a `light_system` text such as `0/1`
  centred on (392, 98). The picture of a barracks (E-trace-026) agrees with every position here and
  in Run B below: two soldier slots at (137, 45) and (193, 45), the coin slot at (369, 45), the
  separators at x 123 and 300 from y 32 to 208, the status lines `Coin supply stopped` and `Building
  evacuated` centred on y 220 and 236 with their left edge at x 20, the four buttons at y 212.
  **Run B (2026-10-17, E-gfx-012, Q-203 a, Q-208 a, c):** the four buttons
  form a row, not a column: 50 x 50 each at y 212, placed right to left
  from x 407 (destroy 407, coins 357, move out 307, upgrade 257). The
  window's content therefore reaches x 457: it is the wide (466 px) window
  of the production and site windows, not the 368 px big panel; the art is
  not stretched. Further content: the building's picture `uiElements3`
  (344, 216, 88 x 88) at (22, 43) with the 3D portrait (80 x 80) at
  (26, 47); two vertical separators `uiElements3` (160, 0, 8 x 56) drawn
  8 x 176 at (123, 32) and at (300, 32); status lines in
  `light_system_outlined` at (20, 220), (20, 236) and (20, 252) (coins,
  move out, upgrade); headings in `small_no_alias_white`, colour
  (126, 183, 174). The game has no separate "evacuate" command: the
  per-building military commands are coins, move soldiers out (the
  toggle, which is the evacuation: wanted soldiers drop to 0 once the
  building has claimed land, MIL-SOL-3.2), upgrade, destroy and attack.
  While move-out is on, the building shows the `Sleep_NoSoldier` icon
  (GFX-OBJ-8.3).
- **UI-GAME-4.4 Headquarters and storehouses** (help `dialog3_0` ..
  `dialog3_3`). Tabs at (15, 25) stepping 40 px: 9 `!List of all goods in
  depot.`, 10 `!List of all settlers in depot.`, 11 `!Military management.`
  (headquarters only), 12 `!Expedition management.` (harbour only). The goods
  and settlers pages are grids of 48 x 48 icon buttons with the count under
  each; clicking an icon toggles its "no more incoming" lock
  (`toggle-stop-flag` / `set-all-ware-flags`), a second control toggles
  "send out" (`toggle-out-flag`). The military page shows five
  rows, one per rank 0..4, each a value box 0..15; a change sends
  `set-soldier-reserve`(rank, value). The expedition page has
  a start button (`prepare-expedition`) and no stop: its layout and rules
  are UI-GAME-8.1a (corrected 2026-11-05, Q-1011). Confidence: high for
  tabs and commands, medium for the grid layout. **The headquarters as the original shows it
  (2026-11-14 b, F59; E-trace-026, a picture at 2560 x 1440):** the window's frame is the tabbed
  `uiDepot` picture, 376 x 464, put with its bottom-right corner on the click like every context
  window (1.9); the title HEADQUARTERS centred in the header bar; the three tabs (goods, settlers,
  military) side by side at (15, 25), 40 px apart, the current one drawn pressed; the goods page a
  grid of six per row, cells 62 px apart in x and y from about (23, 59), each a 48 x 48 framed icon
  with its count right-aligned under its right edge, 31 goods in six rows; a locked good shows a red
  "no entry" mark at the icon's top-left; a round button at about (300, 390) at the bottom right.
  Confidence: medium for the grid numbers (measured on one picture, +- 2 px). **The military page
  as the original shows it (2026-11-15 night, E-trace-028, the headquarters' third tab):** five rows
  65 px apart, Private, Corporal, Sergeant, Officer, General: the rank's name left-aligned at (50,
  90 + 65 k), a slider (the style of the economy dialogs, its bar from x 50 to 250) under it at y
  about 108 + 65 k, a count `n/m` right-aligned on the bar's right end on the name's line (`1/1` for
  Private, `0/1` for the others in Jens's save), and the rank's icon in a framed 48 x 48 box at about
  (260, 82 + 65 k). So the "value box" of the rule above is a slider with a count. Confidence: high
  for the parts and places (+- 3 px). **What the page does (2026-11-15 night, part 2, E-trace-029:
  the headquarters' third tab with the sliders moved by clicks, 2560 x 1440; answers Q-1053):** the
  rows' names are the five soldier settlers' names (`dlg_PrivateSettler`, `dlg_PrivateFirstClassSettler`
  "Corporal", `dlg_SergeantSettler`, `dlg_OfficerSettler`, `dlg_GeneralSettler`); each slider is the
  rank's reserve, **0 .. 15**; a click on the bar puts the knob at the clicked point at once (no
  stepping), the value following the position along the bar (bar about x 52 .. 249: a click at 96 %
  gave 14, at the middle 8, at the right end 15, at the left end 0; rounding of the position:
  medium); the cyan fill runs from the bar's left end to the knob (no fill at 0); the count `n/m`
  is m = the slider's value and n = the soldiers of that rank held in the reserve now, never more than
  m (Private 1/1 -> 1/14 -> 1/8 -> 1/15 -> 0/0 at 0, the headquarters holding one private; Corporal
  0/1 -> 0/9 with none). The reserve is 1 for every rank in Jens's save. The rank's box shows its
  insignia (chevrons, stars, crossed swords for the General), the same pictures as the five rank
  cells of the settlers page (8.1, `uiElements4` at the rank settler's icon cell; medium: a picture
  match). Test T59. Confidence: high for the range, the jump and n/m (pictures).
  **The `,` key's window (E-trace-029):** the next headquarters / storehouse / harbour window (12.1)
  opened with its frame's top-left at (1960, 836) for the harbour and two storehouses alike. **Why
  (2026-11-15 day; E-ui-038, code reading and the pictures' pointer):** the window is placed as every
  context window (1.9), at the **mouse pointer's current position**, not at the building. Its
  bottom-right corner goes on the pointer, x is at most W - 600, then it is clamped to the screen.
  In those pictures the pointer stood at (2400, 1300) when `,` was pressed, so the window went to
  x = min(2400 - 376, 1960) = 1960 and y = 1300 - 464 = 836. In the third picture the depot window
  was already open and kept its place while the key switched it to the next depot (medium: one
  case). So with the pointer at (p, q) the window opens at (min(p - 376, W - 600), q - 464), then
  clamped. Confidence: high for the placement (code), medium for the reuse of an open window.
  **Other tribes (2026-11-15 day, E-trace-030, free games on Coastal Storm as Africans and as
  Chinese):** the headquarters window is the same for every tribe: title "HEADQUARTERS", the same three
  tabs, the same 31 goods icons in the same order on the goods page, the same job icons (the soldier
  ranks included) on the settlers page and the same five rank rows (Private .. General) on the
  military page, compared with a Roman headquarters (E-trace-028). Only the 3D buildings differ. So
  no window of this group takes anything from the owner's tribe. Confidence: high (pictures).
- **UI-GAME-4.5 Ship** (`!Ship`, help `dialog32_0`; E-sim-002: the window's
  build, update, cargo, expedition, port button,
  fleet arrows and view). The
  content, in creation order (positions in the window's content frame):
  (a) **View.** If the ship's visual has a model: a model view at (113, 61),
  150 x 150, showing the ship's model alone (no water, terrain or other
  objects) rendered once, when the window is built, into a 128 x 128 texture
  with a depth buffer, stretched to the view and cut by the `uiShipMask`
  alpha; over it the porthole `uiElements5` (0, 40, 200 x 192) at (88, 40).
  The portrait camera is the one of the building windows' 3D portrait (4.3):
  the main camera's field of view and viewing direction (so the picture turns
  with the main camera's heading and pitch), aimed at the centre of the
  model's bounding box from the distance 1.25 x s / tan(f), s the largest
  side of the bounding box and f the field-of-view angle as the camera stores
  it; near plane 0.1, far plane 200.
  (b) **Buttons.** `!Jump to port.` `uiElements5` (400, 40, 48 x 48),
  highlighted (400, 136), at (8, 392): enabled while the ship has a port (its
  target or home harbour is set), otherwise disabled with the tooltip `!Ship
  has no target port.`; a click moves the camera to the port's field.
  `!Jump to ship.` `uiElements5` (352, 40, 48 x 48), highlighted (352, 136),
  at (304, 392): the camera moves to the ship. Two separators `uiElements3`
  (0, 432, 350 x 8) at (2, 232) and (2, 376): the same separator line as the
  other windows', cut 350 wide (its rows 3 and 4 are the two visible lines,
  colours (1, 45, 45) and (50, 96, 91); the rest is transparent). Corrected
  2026-11-16 night (Q-1059, E-ui-039, code reading and a picture): the code
  names `uiElements3`, not `uiElements4`; in E-trace-030's ship window the
  two lines lie at content y 235 / 236 and 379 / 380 from x 3 to 351, pixel
  equal to that art.
  (c) **Name.** Label `!Name of the ship` at (104, 392) in
  `small_no_alias_white`, colour (126, 183, 174); a text input (UI-MENU-5.3,
  style 2) at (104, 400) showing the ship's name, refilled whenever the
  window refreshes (after an arrow click). The name is a string kept per
  ship (saved with the ship as its `ShipName` property, FMT-SAV-3.x). **The
  default name (Q-228 b; E-sim-002):** a
  ship without a typed name shows the language text of the key
  `<Tribe>ShipName<k>` with `<Tribe>` the owner's tribe name `Roman`,
  `African` or `Chinese` and `k` the owner's ship-name counter at the ship's
  creation: the counter starts at 0, rises by 1 per ship created and goes
  back to 0 after 20 (so 21 keys per tribe), and is saved in the `Player
  ShipNames` chunk (FMT-SAV-3.3). **Corrected 2026-11-15 day (E-trace-030, picture and data):** the
  English language file `controls.txt` has a text for every one of the 63 keys (`RomanShipName0` ..
  `20`, `AfricanShipName0` .. `20`, `ChineseShipName0` .. `20`), e.g. `RomanShipName0` "Aurora" and
  `RomanShipName1` "Bellona"; the Seafaring tutorial's first two ships showed exactly these two names
  in their windows and in their messages ("Ship is awaiting orders (Aurora)"). (The earlier note
  that the data holds no text for these keys was wrong.) **A typed name is
  never kept (2026-11-05, Q-258 c; E-ui-018):** the
  name shown is always made at display time from the owner's tribe and the
  ship's creation number, nothing stores a ship name, and the window never
  reads its input back; what the player types stays in the input only until
  the window refreshes (an arrow click, 4.5 d, or reopening), which puts the
  default name back.
  (d) **Fleet arrows.** `uiElements0` (40, 152, 24 x 24) at (84, 400) and
  (136, 152, 24 x 24) at (253, 400) (further state sprites (8, 152) /
  (72, 152) and (104, 152) / (168, 152)). The left arrow selects the
  player's previous ship in the game's ship list, wrapping from the first to
  the last; the right arrow the next, wrapping from the last to the first;
  with one ship either reselects it. Selecting opens that ship's window
  (selection kind 5) and moves the camera to the ship.
  (e) **Cargo.** Label `!Goods on board.` at (16, 248); twelve slots from
  (16, 255), six per row 56 px apart, the second row 63 px lower; each slot
  is a button with `uiElements3` (440, 216, 48 x 48) as its normal and
  highlighted picture; a used slot (the ship's cargo list in order) adds the
  good's icon from `uiElements1` or the settler's from `uiElements4` at
  (column x 48, row x 48, 48 x 48), the entry's icon cell, and the amount at
  (+24, +53), centred on that x (UI-MENU-1.8 flag 8), in `light_system`. The slot buttons have no click action.
  (f) **Expedition.** In expedition mode: `!Cancel expedition mode.`
  `uiElements5` (448, 40, 48 x 48), highlighted (448, 136), at (8, 176)
  (`cancel-expedition`), and, exactly while a harbour can be founded where
  the ship waits (`SHIP-ships-expeditions.md` SHIP-7.1, checked at every
  refresh; a ship a mission's script made is no exception, it has a last
  port position from its creation, SHIP-2.7; 2026-11-16 night, Q-1060),
  `!Construct harbor.` `uiElements5` (352, 88, 48 x 48), highlighted
  (352, 184), at (305, 176) (`found-harbour`). While the ship
  waits for orders (its expedition course value is negative) six compass
  buttons of 40 x 40 from `uiElements5`, each with its tooltip and enabled
  only when a landing place lies in its direction (the ship's target for
  that direction is set): `!East` (120, 0) at (278, 118), highlighted
  (360, 0); `!SouthEast` (200, 0) at (240, 192), (440, 0); `!SouthWest`
  (160, 0) at (104, 192), (400, 0); `!West` (80, 0) at (67, 118), (320, 0);
  `!NorthWest` (0, 0) at (104, 46), (240, 0); `!NorthEast` (40, 0) at
  (240, 46), (280, 0). A click sends the ship toward that direction
  (`sail-expedition` with the direction index 0 .. 5
  in this order). Confidence: high for the layout, the arrows, the port and
  ship buttons and the view's content; medium for the camera distance (the
  form of the field-of-view value), the cargo label's position and the
  direction indexes. **Seen (2026-11-15 day, E-trace-030, the Seafaring tutorial, a ship in
  expedition mode carrying 4 planks, 1 builder, 6 stones, 2560 x 1440):** title "SHIP"; the porthole
  with the ship's model sits top centre with six orange compass spikes around it (two above, one at
  each side, two below), each with its English direction as tooltip ("East", "Southwest", ...); left
  of the porthole the round "Cancel expedition" button (a red no-entry sign over a ship); "Goods on
  board" over two rows of six square slots, the used ones with the good's or settler's icon and the
  count under it (planks 4, builder 1, stones 6, in that order); at the bottom the "Ship has no
  destination" button on the left (an anchor with an arrow, disabled), the label "Name of ship" over
  the name between the two arrows, and "Jump to ship" on the right. A click on the Southwest spike
  sent the ship off at once. At a landing place the window, opened with the right arrow from the
  first ship, showed the second ship with a "Build harbour" button right of the porthole (tooltip
  "Build harbour"); its click turned the site into a levelled, paved harbour building site with the
  builder standing at its flag, and new border buoys on the water around it. Confidence: high
  (pictures).

### UI-GAME-5 Attack dialog

- **UI-GAME-5.1** (`!Attack Menu`, help `dialog21_0`), opened by a click on
  an enemy military building. A wide slider at (164, 132) sets the share of
  soldiers to send (label at (245, 160) shows the number); `!Use weak units
  first.` `uiElements5` (8, 456) at (178, 49) and `!Use strong units first.`
  (64, 456) at (254, 49) (one pressed); `!Soldiers` at (243, 124); four
  preset buttons from `uiElements1`: (8, 288) `!Low number of soldiers.` at
  (36, 117), (64, 288) `!Medium number of soldiers.` at (92, 117), (8, 344)
  `!High number of soldiers.` at (344, 117), (64, 344) `!Highest number of
  soldiers.` at (400, 117); `!ATTACK` text button at (140, 180) sends
  `attack-building`(building, count, strong-first). Separators at (37, 103)
  and (37, 170). Confidence: high. **Seen with no attackers (2026-11-16 night, E-trace-031, a
  computer player's guard tower clicked by a player with no soldiers in reach, 2560 x 1440):** title
  "ATTACK MENU"; the weak / strong pair on top (a shield-and-sword icon, crossed swords), "Soldiers"
  under them, the four count buttons in two pairs at the sides (one, two, three and four shields), and
  in the middle the line "Cannot attack this building!" (MIL-ATK-1.3); **no slider, no number and no
  ATTACK button** are shown while A = 0. Confidence: high (picture).
  **Measured (Q-1064):** the line is centred on x 243 of the dialog, its glyphs from y 154 to 166 (the
  number label's row, (245, 160)), white, about 13 px high.

### UI-GAME-6 Global dialogs

All four share the tab strip (icons 13 tools, 16 military, 14 distribution,
15 transport, at (15, 25) stepping 40 px), a reset button `uiElements3`
(440, 264) / hover (440, 312) `!Reset dialog settings` at (295, 433), and
open next to the bar. **Corrected 2026-11-15 night (E-trace-028, pictures at 2560 x 1440 of all four
pages, L key and tab clicks):** they open **centred on the screen**, not next to the bar: one dialog
(its frame's visible gold border runs from x 1101 to 1455 and y 471 to 963, +- 2 px), the same place for
every page (a tab
click swaps the page in place). **The frame (E-ui-036, code reading):** the frame picture is
`uiDepot_long` (8, 8, 368 x 503) (`gfx/uiDepot_long.dds`), so the dialog is 368 x 503, centred at
((W - 368) / 2, (H - 503) / 2) = (1096, 468) at 2560 x 1440; the picture's visible gold border is the
measured 1101 .. 1455 x 471 .. 963. The transport page draws its background picture
`ui_detail_transport` (8, 8, 360 x 496) at (0, 0) inside it (6.4). Titles as shown: TOOLS MENU,
MILITARY MENU, GOODS DISTRIBUTION MENU, TRANSPORT MENU; the four icon tabs at the top left, the round
reset button at the bottom right. Confidence: high for centred and the same place (pictures) and for the
frame picture and its size (code: five dialogs are made with `uiDepot_long`; medium for which of them
are these four pages).

- **UI-GAME-6.1 Tools** (`!Tools Menu`, help `dialog18_0`). Twelve rows, one
  per tool of the player's tool list, at (87, 70 + 30 i): the tool icon
  30 x 30 at (-38, -6), a small slider (value = the tool's priority, ECO-PROD
  tool rule), a 24 x 24 check button `uiElements0` (104, 104) / (168, 104)
  `!Produce only this tool.` at (+210, -1). Slider -> `set-tool-weight`
  (tool, value); check -> `set-only-tool`(tool) when
  set, `clear-only-tool` when cleared; reset ->
  `reset-tools`. Confidence: high.
- **UI-GAME-6.2 Military** (`!Military Menu`, help `dialog22_0`). Headings
  `!Arrangement` (32, 64), `!Defense` (32, 237), `!Attack` (32, 362);
  separators (0, 432, 336 x 8) at (15, 224) and (15, 349). Rows (icon,
  small slider at +60, +20, label at +60, +12): `!Far` (176, 400), `!Medium`
  (232, 400), `!Close` (288, 400) at (32, 79) stepping 48 px -> `set-far-occupation` /
  `set-medium-occupation` / `set-close-occupation`; `!Intercepting Defenders`
  (344, 400) at (32, 245 + 52) -> `set-interceptors`;
  `!Attack Strength` (344, 456) at (32, 378) -> `set-attack-strength`; a pair at (32, 245): `!Prefer weak soldiers`
  `uiElements5` (8, 344) and `!Prefer strong soldiers` (64, 344), the current
  one pressed and disabled -> `set-soldier-preference`(0 weak / 1 strong);
  reset -> `reset-military-settings`. Confidence: high.
- **UI-GAME-6.3 Distribution** (`!Arrangement Menu`, help `dialog20_0`).
  Six good buttons at (32, 60) stepping 50 px from `uiElements4`: normal
  (64, 144 + 56 k), hover (8, ..), pressed (120, ..) for the six
  distributed goods (k = 0..5, the ECO-DIST goods), tooltip `!Arrangements
  for:` + the good's name; below, at (110, 162) stepping 75 px, one small
  slider per consumer building of the chosen good with the building's icon
  60 px to the left; separator at (15, 115). Slider -> `set-distribution-weight`
  (good, consumer, value); reset -> `reset-distribution`. Confidence: high.
- **UI-GAME-6.4 Transport** (`!Transport Menu`, help `dialog19_0`).
  Background `ui_detail_transport` (8, 8, 360 x 496) at (0, 0); a list of
  14 transport classes in priority order (TRN-CARRY priorities), each with
  its icon and up / down arrows; an arrow sends `move-priority`
  (class, direction); reset -> `reset-priorities`. Confidence:
  high for the commands, medium for the row layout.
- **UI-GAME-6.5 The sliders' art (2026-11-15 day, Q-1054; E-ui-037, code reading and the
  shipped pictures).** The game has two horizontal slider kinds.
  (a) **The economy slider, 200 x 20 (with a fill).** It is used by the tools (6.1), military
  (6.2) and distribution (6.3) dialogs and the headquarters' military page (4.4). It is drawn
  in this order:
  - **bar:** `uiElements2` (0, 0, 200 x 20), a dark brown bar with a thin gold edge;
  - **fill:** `uiElements2` (0, y, 200 v x 20) drawn at the slider's left edge, 200 v pixels
    wide, so the picture is cut, not stretched (v is the value, 0 .. 1). The row y is 56 for
    v >= 0.65, 104 for 0.3 <= v < 0.65 and 80 for v < 0.3, but the three shipped rows are
    pixel for pixel the same cyan bar, so the fill looks the same at every value. While the
    slider is disabled the fill is drawn with the colour (170, 170, 255) at alpha 170/255 (a
    paler, bluish fill); enabled, it is drawn plain;
  - **knob:** a 20 x 20 square from `uiElements0`: (8, 104) normal, (40, 104) highlighted,
    (72, 104) pressed and beyond. The art at 40 and 72 is the same brighter orange-gold square.
    The knob's left edge is at 2 + v x (200 - 23) = 2 + 177 v pixels from the slider's left,
    its top at the slider's top. So the fill always ends under the knob (at v = 0.5 the fill
    ends at 100 and the knob's centre is at 100.5).
  (b) **The settings slider, 152 x 24 (no fill).** It is used by the settings pages (`UI-menus.md`
  UI-MENU-4) and the attack dialog (5). Bar: `uiElements0` (264, 344, 152 x 24). It has no fill:
  the slider kind makes no fill picture. The same 20 x 20 knob, at 2 + v x (152 - 23) =
  2 + 129 v pixels, top 0.
  (c) The list boxes' vertical scroll bar is a third kind (`UI-menus.md`, E-ui-024): bar
  `uiElements0` (456, 197, 24 x 194), knob 24 x 24 from the same three places.
  Confidence: high (code: the slider's kinds, pictures, rectangles, the fill's width and row,
  the knob's place and pictures; data: the rows are identical). This agrees with the pictures
  of the original (E-trace-028 tools dialog: cyan fill to the knob; E-trace-026 Sound page: no
  fill). Test T61.

### UI-GAME-7 Overview dialogs

- **UI-GAME-7.1** Statistics (`dialog25_0`), inventory (`dialog26_0`,
  512 x 536, tabs goods / settlers `dialog35_0`), buildings overview
  (`dialog29_0`), goods production (`dialog28_0`) and fleet (`dialog39_0`)
  exist with the help texts given; their layouts: UI-GAME-8.5a to 8.5d and
  the fleet paragraph of 8.5 (the graph drawing itself is still open,
  Q-096).
- **UI-GAME-7.2 End of game.** The statistics dialog becomes the end screen:
  `!VICTORY` or `!DEFEAT` with the reason (`!Castle destroyed!`, `!Mission
  not completed`, `!You have destroyed all enemies!`, `!You have conquered
  the map!`, `!You have enough Coins in your depots!`, `!You have reached
  the magic gate!`, by the mission's victory condition 0..4), a row per
  player (`Player #n`, team, `!VICTORY` / `!DEFEAT` in (255, 255, 155) or
  (190, 190, 90)) at (140, 190) stepping 25 px, and the buttons
  `!Statistics` (300, 445), `!EXIT MAP` (300, 467), `!CONTINUE` (300, 489).
  Confidence: high.

### UI-GAME-8 Answers to Q-095 and Q-097 (2026-10-17)

- **UI-GAME-8.1 Depot goods and settlers pages (UI-GAME-4.4).** Every good of the
  goods table whose kind matches the page (goods or settlers) gets a slot by its
  dialog slot number: x = 23 + (slot mod 6) x 54, y = 60 + (slot div 6) x 62 in the
  1024 x 768 space, background `uiElements3` (0, 80, 48 x 56), icon from
  `uiElements1` (goods) or `uiElements4` (settlers) at the property's icon
  column / row (48 x 48), count label at (46, 49), and two 16 x 16 flag markers
  (`uiElements3` (312, 72) at (0, 0) and (336, 72) at (0, 33)). Clicking a slot
  toggles that good's flag through `cycle-ware-flags` with the good id. The page
  button "For all wares." (`uiElements5` (304, 40, 48 x 48)) cycles a mode
  0 -> 1 -> 2 -> 0 and sends `set-all-ware-flags` with (page is settlers,
  mode). `toggle-stop-flag` is not used by this dialog. Page 3 (soldier
  reserve) has five rows with sliders (4.4; corrected 2026-11-15 night, no +/-) sending `set-soldier-reserve` (row, value);
  page 4 (harbour) is UI-GAME-8.1a; "Destroy the building." (`uiElements5`
  (448, 88, 48 x 48)) is absent for the headquarters. Confidence: high.
- **UI-GAME-8.1a Harbour expedition page (tab 12; corrected 2026-11-05, Q-1011;
  E-ship-004: the page's build, slots, counts,
  start button and update).** Positions in the window's content frame
  of the 1024 x 768 space.
  (a) **Four slots, no +/-** (the earlier "three goods with +/-" was wrong):
  at (23 + 54 i, 60), i = 0 .. 3, for the builder (`good_constructor`, icon
  from `uiElements4`), stones, planks (icons from `uiElements1`) and the
  ship (`ship`, `uiElements1`), each built like a goods slot of 8.1:
  background `uiElements3` (0, 80, 48 x 56), the good's icon 48 x 48, the
  good's name as tooltip and a count label at (46, 49) in
  `small_no_alias_white`, `-/-` until the first refresh. Each page update
  the labels read "n/1" (builders set aside), "n/6" (stones), "n/4"
  (planks) with n the harbour's expedition store (SHIP-5.2), and the ship
  slot "1/1" while a ship is called for the expedition (SHIP-5.3, 5.4),
  else "0/1". The slots have no click action. Nothing on the page changes
  the amounts: they are the harbour's fixed cost (SHIP-5.2).
  (b) **`!Exploration.`** (prepare expedition): a button `uiElements5`
  (256, 88, 48 x 48), pressed / highlighted (256, 184), at **(298, 386)**.
  It is enabled only while the harbour prepares no expedition (its
  expedition state is 0, SHIP-5.1); a click sends `prepare-expedition`. While
  a preparation runs (gathering, getting ship, waiting for ship) it is
  disabled, and its disabled tooltip is `!Exploration is running!`. The
  page has **no stop**: once started, a preparation ends only by the
  transfer to a ship (SHIP-5.5); the goods set aside stay set aside until
  then (SHIP-5.1a).
  (c) **`!Jump to next expedition`**: a button `uiElements5` (352, 40,
  48 x 48), highlighted (352, 136), at **(248, 386)** (50 px left of the
  start button). Enabled while the player has at least one ship in
  expedition mode, with that tooltip; otherwise disabled with the tooltip
  `!No expedition started`. A click picks the player's ship in expedition
  mode **nearest to this harbour's field** (MAP-GRID-2.1 distance; the
  earlier ship in the ship list on a tie), moves the camera to it, selects
  it and opens its ship window (4.5). It does not cycle: clicking again
  picks the same ship. A harbour that is only preparing is not a target.
  (d) An empty label at (50, 400) is made and never filled (medium). **Corrected 2026-11-15 night
  (E-trace-028, a picture of the harbour in Jens's 2023 save, preparing an expedition):** the label is
  filled: while the harbour prepares an expedition it shows `!Exploration is running!` (the English text
  is "Preparing expedition!"; white, left edge x 45, on y 400); the four slots read `1/1`, `6/6`, `4/4`,
  `0/1` there (builder, stones and planks set aside, no ship yet). Confidence: high for the text while
  preparing (picture); the text in the other states not read. **Seen before and after the start
  (2026-11-15 day, E-trace-030, Seafaring tutorial):** before a preparation the label is empty and
  all four slots read `-/-`; the left button's tooltip is "Jump to next expedition ship", the right
  one's "Prepare expedition"; right after the click the right button is greyed out, "Preparing
  expedition!" shows at the lower left and the slots still read `-/-` (nothing set aside yet in that
  frame). Confidence: high (pictures).
  Confidence: high for the slots, the texts, the enabling and the actions;
  medium for "disabled tooltip" (the call that sets it differs from the
  normal tooltip) and for whether the harbour window closes when the ship
  window opens.
- **UI-GAME-8.2 Transport menu (UI-GAME-6.4).** Background `ui_detail_transport`
  (8, 8, 360 x 496) at (0, 0), close button `uiElements3` (440, 264, 48 x 48),
  two `uiElements3` (440, 312) buttons and "Reset dialog settings". Which arrow
  moves a row up or down was not extracted (Q-100 d). Confidence: medium.
- **UI-GAME-8.3 Messages window (UI-GAME-2.3).** Eight rows, scroll up / down /
  "Jump to last one." buttons, four filter toggles (misc, economic, scholar,
  military), tooltip "Left Click for go to, right click for delete". The icon per
  message kind was not extracted (Q-100 d).
- **UI-GAME-8.4 Minimap textures (UI-GAME-2.5).** `regions` is map-sized (at
  least 256): every field owned by a player is that player's colour with alpha
  0x99 on Border fields and 0x55 inside, 0 where unowned. `cities` (256 x 256)
  marks buildings as 2 x 2 blocks: 0xAAAAAA, military buildings 0xFF8888, and
  white for a building with its occupied flag set. Both textures are refreshed
  one row per update, cycling through the rows. Confidence: high.
- **UI-GAME-8.5 Statistics family (UI-GAME-7.1).** One tab bar (Q-227 c;
  E-sim-002): a one-row grid group at (15, 25) with 40 x 50 px
  cells, holding six icon tab buttons (the `uiElements0` tab sprites of ids
  17, 18, 19, 22, 21, 12 in that order, 40 px apart; confirmed, E-sim-002, with the tabs' targets: windows 25, 28, 26, 35, 29, 39) with the tooltips
  `!Player statistics`, `!Production statistices` (sic), `!All goods in all
  depots and the castle` (inventory), `!All settlers in all depots and the
  castle`, `!Size of village and productivity`, `!Fleet`; the current page's
  tab is drawn pressed. Every page of the family is a dialog of **size
  512 x 536** (Q-227 d): the first time it is placed by the base placement
  for that size, afterwards at the position the family remembers (one shared
  "last position", medium whether every page shares it).
  **As the original shows it (2026-11-14 b, F61; E-trace-026, pictures of all six pages at 2560 x
  1440, opened with C and switched by the tabs):** (a) **Place:** every page puts its frame's
  top-left at the same point, the family's position; the first time that is where a centred 512 x
  536 dialog would start, ((W - 512) / 2, (H - 536) / 2), (1024, 452) on 2560 x 1440. So the window
  **does not move when the tab changes**, and a narrow page is not centred on the screen (it sits
  68 px left of the centre line). The 7 px (8.5b) and 10 px (8.5d) shifts move a page's **content**
  inside the frame, not the frame. (b) **Size:** each page draws its own frame picture from that
  corner, so the size changes with the tab: the player statistics, production, both inventory and
  the fleet pages use the tabbed `uiDepot` frame, 376 x 464; the buildings page a tabbed frame of
  512 x 536 (its six columns need the width). (c) **Title:** centred on the frame's width and
  inside its header bar (vertically centred, `UI-menus.md` UI-MENU-1.8 b): STATISTICS, GOODS
  PRODUCTION, INVENTORY (both pages), BUILDINGS, FLEET LIST. (d) **Tabs:** the six at (15, 25), 40 px
  apart, on the frame's grey tab band, the current one pressed. (e) **Player statistics page**
  (8.5a): one portrait per player 50 px apart from about (40, 60), its colour square at its lower
  left; the graph's parchment from (18, 129) to about (346, 392); the axis labels `-15 -13 -10 -8 -5
  -3 -0` under the plot (centred on about y 359) and the value of the top line (e.g. `94%`) at the
  plot's top right; the two drop-downs at (18, 412) (`15 minutes`) and (192, 412) (`Size of
  territory`), 152 x 20, inside the frame's bottom. (f) **Production page** (8.5b): the fourteen good
  buttons in two rows of seven, 48 px apart, from about (20, 60); the graph's parchment below from
  about y 175; the span drop-down and `Clear selection` at the bottom. Test T58. Confidence: high for
  the place, the sizes and the titles (pictures, frame edges equal within 1 px on all six); medium
  for the content positions (+- 3 px). The fleet dialog
  (`!Fleet`, help `dialog39_0`; Q-227 e, f; E-sim-002: the dialog's build,
  list and update): size 512 x 536; the frame picture
  `uiGame` (136, 440, 344 x 184) at (6, 255); the family tabs; a list box
  (the one of UI-MENU-2.1) at (50, 85) with 11 visible rows (the "(50, 49)"
  of the first reading is the layout cell the list is created in, not a row
  step). The list holds one row per **finished** ship of the player (frame
  and main construction done), in the game's ship list order, each row the
  ship's name (the name string of UI-GAME-4.5 c, nothing else); it is
  rebuilt whenever the number of such ships changes. A click on a row
  selects that ship (its ship window opens, UI-GAME-4.5), moves the camera
  to the ship and closes the fleet dialog. Every update writes the dialog's
  position to the family's shared last position. The **Planning menu** (Q-227 a; E-sim-002) is a
  dialog titled `!Planning` with five menu text buttons (UI-MENU-1.4 style)
  in a vertical list at (70, 60) stepping 40 px: `!Statistics`,
  `!Production`, `!Inventory`, `!Buildings`, `!Productivity` (a key without a
  language entry is shown as the key); its frame is the one its opener passes
  to the dialog base (medium: the big panel). **The Logic menu** (`!Logic`, window 1; Q-258 d,
  answered 2026-11-04, E-ui-015): a dialog built like the Planning menu, its
  list at (70, 60) stepping 40 px, four menu text buttons: `!Tool Production` opens window 18 (the
  tools page, UI-GAME-6.1), `!Goods Arrangement` window 20 (6.3), `!Transport Priorities` window
  19 (6.4), `!Military Settings` window 22 (6.2). A click opens that page unless it is already
  open, first closing the windows of this family (and the game menu), and closes the menu.
  **Neither the Logic menu nor the Planning menu (window 2) is ever opened:** every call that opens
  a window by number was listed, and none names 1 or 2; no script does either. The HUD goes straight
  to the pages: button 6 and L toggle window 18, button 4 and C toggle window 25 (2.2), and the pages
  have their own tabs. So both menus are unreachable leftovers: **do not build them.** The menu
  buttons' targets above and the Planning menu's (25, 28, 26, 29, 30) stand only as a record.
  Confidence: high for the layout and the targets; medium-high that nothing opens them (a table
  held in data could not be ruled out by this search).
- **UI-GAME-8.5a Player statistics page (Q-227 b; E-sim-002).**
  Title `!Statistics`. A static image `uiElements1` (176, 232, 328 x 271) in a
  grid group at (18, 129); the players: a grid at (30, 45) with 50 x 49 cells,
  one cell per present player p (0 .. 7, left to right): a push button with
  the tribe's first portrait `uiElements0` (16 + 168 t, 512, 56 x 80) (t the
  player's tribe; hover and pressed states 88 rows lower) and the player's
  name as tooltip, enabled while the player is in the game; under it, 45 px
  lower, the player's colour square `uiElements0` (520 + 24 (c mod 2), 216 +
  24 (c div 2), 24 x 24) as a button (c the colour index). A 3D view in a grid
  at (41, 153) (50 x 50) and two choice lists (drop-downs, 8.5g): at (192, 412) the series
  `!Size of region` 0, `!Number of buildings` 1, `!Number of settlers` 2,
  `!Economic state` 3, `!Military power` 4, `!produced Coins` 5,
  `!Productivity` 6, `!Frags` 7 (the series of the data paragraph below), at
  (18, 412) the spans `!15 Minutes` 15, `!1 Hour` 60, `!4 Hours` 240,
  `!16 Hours` 960 (samples shown); the chosen series and span are kept by the
  interface; a separator `uiElements3` (0, 432, 336 x 8) at (15, 115), the
  family tab bar, and the opening sends the event `STATS_MENUOPENED`. The
  "3D view" is the graph canvas: a texture view of the plotted series (its
  drawing is open, Q-096). Confidence: high for the controls and their keys,
  medium for the grid cell positions (read from the grid groups, not
  photo-checked). The other pages: 8.5b to 8.5d.
  **Player statistics data.** The game keeps per player eight time series sampled once per
  600 ticks (one sample per minute of game time, at most 960 samples each, oldest dropped):
  0 land (owned fields as a percentage of all fields), 1 buildings (count), 2 inhabitants
  (settlers outside depots except carriers, plus stocked settlers of every kind except
  helpers and donkeys), 3 goods (sum of all building stocks), 4 military strength (sum over
  soldiers of rank index + 1, stocked soldiers included), 5 **coins produced so far** (a running
  total: 8.5e), 6 productivity (mean of the buildings' productivity in percent), 7 **frags so
  far** (a running total of the player's kills: a soldier's blow that brings an enemy's health to
  0 or below, or the player's catapult shot killing a soldier; answered 2026-11-04, Q-258 f, E-ui-015); plus fourteen per-cycle counters that start a new sample every 600 ticks
  (the production statistics page, 8.5e). The samples are taken at ticks 0 to 4 of the
  cycle in the order land, goods and counter 5, buildings, inhabitants and military,
  productivity; the per-cycle series and counter 7 roll over at tick 600. A building's
  productivity entry loses 0.05 (to a minimum of 0) every 600 ticks in which it did no work.
  **The two counters (corrected 2026-11-07, E-ui-020).** The
  sampling cycle counts 0 .. 600 and starts again: **601 ticks** per cycle (as AI-CON-1.8), the
  samples at its counts 0 to 4, the roll-over at 600. The productivity decay has a counter of its
  own, 600 ticks per pass. Both start at 0 in a new game. Only the decay counter is saved
  (`FMT-savegame.md` FMT-SAV-3.11, restored as saved); after a load the sampling counter starts
  at 0 again, so the first five ticks after a load take the five samples and the 601st ends the
  cycle, whatever the game's tick.
  **How four of the series are counted (2026-11-16 day, Q-198 d; E-ui-042, code reading, high
  unless noted):**
  - **0 land:** the player's owned fields that lie on a land continent (fields of a water body do
    not count), times 100, divided by the map's count of land fields, in **whole percent**
    (integer division, truncated); not hundredths. **Which fields are land (2026-11-16 day b,
    Q-1066 b):** a field counts in the numerator only when its continent index (`FMT-s2m-map.md`
    FMT-S2M-4.8) is not -1 and that continent is not water; a field of index -1 (mountain or
    other ground no continent holds) never counts. The denominator is the same set over the whole
    map: the number of fields on land continents, which the map file stores as the u32 after the
    continent objects (FMT-S2M-4.8's "meaning unknown" value: on all 34 shipped maps it equals the
    summed field counts of the land continents, e.g. MP_4P_Fight 2548 of 6400 fields, 206 of index
    -1). Confidence: high for the numerator (code); medium-high for the denominator being that
    stored count (the code divides by the loaded continent layer's count; the equality is data,
    34 of 34 maps);
  - **1 buildings:** every building object of the player, **construction sites included**;
  - **3 goods:** the sum of the amounts in the stores of the player's **warehouses** (headquarters,
    storehouses, harbours) only; the inputs held by workshops and the material on sites do not
    count;
  - **6 productivity:** each productivity entry whose building still exists gives its value times
    100, rounded to a whole number, to its owner; the series is the sum divided by the number of
    such entries (integer division). **Which buildings have an entry (corrected 2026-11-16 day b,
    Q-1066 a; it said "made at its first finished work cycle"):** the entries are those of
    `ECO-outdoor-workers.md` ECO-OUT-3.11: made at 0.5 **when the site is placed** for every type
    with a product, so a **construction site counts** (at 50, the fall skips sites) and a finished
    building that never produced counts at its current entry (50 less 5 per fall); the forester's
    hut counts only from its first planting; a building without an entry (no product type) is
    not counted at all (not as 0). The loop makes no finished test and no worker test: only the
    building's existence (an entry whose building is gone is skipped until the next fall drops it).
  Series 2 (inhabitants) and 4 (military) were not re-read.
  The "Player statistics" page plots these series per player; the exact page layout is open
  (Q-096). Confidence: high for the series' meanings 0 to 4 and 6, medium for the sampling
  order (E-cmd-001).
- **UI-GAME-8.5b Production statistics page (Q-227 b; E-sim-002).** Title `!Good Production`, help `dialog28_0`, 512 x 536 (or
  the family's remembered position), its origin moved 7 px to the right of
  the base placement. Content: the graph image `uiElements1` (176, 232,
  328 x 271) in a grid group at (16, 175) and the graph canvas (as 8.5a) in
  a grid at (49, 195); fourteen good toggle buttons in a grid at (13, 60),
  7 per row, 48 px apart in x and y (button i at (13 + 48 (i mod 7), 60 +
  48 (i div 7))): base `uiElements3` (0, 80, 48 x 56) with the good's icon
  (`uiElements1`, the good's icon cell x 48, 48 x 48), the pressed and
  hover pictures `uiElements3` (384, 336, 48 x 48) and (336, 336, 48 x 48),
  the good's name as tooltip; the goods in order: wood, plank, stone, food,
  water, beer, coal, iron ore, gold, iron, coin, tools, weapons, boat (the
  fourteen per-cycle counters of 8.5a's data paragraph, in this order); a
  button is enabled while the interface's flag for that good is set (which
  goods are shown as curves). Below: the span choice list (a drop-down, 8.5g) `!15 Minutes` 15,
  `!1 Hour` 60, `!4 Hours` 240, `!16 Hours` 960 in a grid at (18, 455) (the
  chosen span kept by the interface) and the text button `!Unmark all`
  (UI-MENU-1.4 style 1) in a grid at (192, 455); a separator `uiElements3`
  (0, 432, 336 x 8) at (15, 155); the family tabs; the opening sends
  `STATS_MENUOPENED` with the page name `Good Production`. Confidence: high
  for the controls and the goods' order, medium for the positions (grid
  groups, not photo-checked). The page shows the **local player** only. At first nothing is marked
  and the span is `!15 Minutes`; marks and span stay for the rest of the session. It re-reads the
  counters when opened, on a toggle, on `!Unmark all`, on a change of span, and every 601 page
  updates otherwise; it draws every frame (8.5f).
- **UI-GAME-8.5e What the fourteen counters count (answered 2026-11-04, Q-258 f; E-ui-015).** When a worker finishes a work cycle, the building's productivity is raised
  (a new entry starts at 0.5, then +0.25, at most 1) and the counter of its type's **first good** gets +1 in the player's current
  sample. A good with a group counts as its group (`gp_setGroupGoodId`), so:

  | Counter | Buildings |
  |---|---|
  | 0 wood | woodcutter |
  | 1 plank | sawmill |
  | 2 stone | stone pit, stone mine |
  | 3 food | fisher (fish), hunter and slaughterhouse (flesh), bakery (bread) |
  | 4 water | spring |
  | 5 beer | brewery |
  | 6 coal, 7 iron ore, 8 gold | their mines |
  | 9 iron | smeltery |
  | 10 coin | mint; also +1 to the coins total of series 5 |
  | 11 tools | locksmith, whichever tool it made |
  | 12 weapons | smithy, one count per cycle, sword or shield |
  | 13 boat | shipyard: a finished boat, and a ship-building step that ends away from the building (medium) |

  Corn, flour, pigs and donkeys belong to no counter, so farm, mill, piggery and donkey breeder
  raise productivity only; the forester likewise. It is one count per finished cycle, not per ware
  stocked or delivered, and it counts at the end of the production, before the worker carries the
  ware out. The series are empty until the first new sample at tick 600 and a count on an empty series
  is dropped, so **nothing made in the first 600 ticks is recorded** (series 5's total does count it).
  Confidence: high (code; two saves agree: each player's coin samples sum to the running total, 3, 1,
  943, and the categories are filled as the table says); medium for the first-minute rule and the
  ship step.
- **UI-GAME-8.5f The graph (answered 2026-11-04, Q-258 f; E-ui-015).**
  With (x0, y0) the canvas's top left (its grid position), n the span in samples and M the largest
  sample of the **whole history** of all marked series (0 if none is marked; not only the part in
  the span):
  - **Axes**, every frame, colour (0.1, 0.1, 0.1): from (x0, y0 + 198) to (x0 + 250, y0 + 198), and
    from there up to (x0 + 250, y0 + 2).
  - **Time labels**, `dark_system`, seven of them, i = 0, 5, .. 30: at x0 + trunc(245 i / 30), y0 +
    205, centred, the text "-" and n - trunc(n i / 30), minutes ago: for 15 minutes -15, -13, -10, -8,
    -5, -3, -0.
  - **Top label**: M at (x0 + 260, y0), the other alignment of the font call (not read further).
  - **One curve per marked series p (0 .. 13)**, in this order, as line segments from the newest
    sample at x0 + 245 leftwards, 245 / n per sample, at most n + 1 samples; y = y0 + 185 - trunc(v x
    185 / M) + p (each series one pixel lower than the one before, so equal curves stay apart). A
    series with fewer samples just ends earlier. With M = 0 nothing visible is drawn.
  - **Curve colours** (r, g, b): 0 wood (1, 0, 0), 1 plank (0.5, 0, 0), 2 stone (0.6, 0.6, 0.6), 3 food
    (0, 1, 0), 4 water (0.5, 0.5, 1), 5 beer (0.5, 0.5, 0.5), 6 coal (0, 0, 0), 7 iron ore (1, 0.6, 0.6),
    8 gold (0.9, 0.8, 0), 9 iron (1, 1, 0.7), 10 coin (0.9, 0.8, 0.3), 11 tools (0.5, 0, 0), 12 weapons
    (0.5, 0.5, 0.5), 13 boat (0.6, 0.2, 0).
  The player statistics page (8.5a) uses the same two drawing routines. Confidence: high for the
  arithmetic and colours (code), not compared with a picture.
  **The player page's curves (answered 2026-11-06, Q-1016; E-ui-019).** On 8.5a the curve routine gets one curve per player p (0 .. 7, in that order) whose
  portrait button is enabled (the player is in the game) **and pressed**: the chosen series (0 .. 7)
  of that player, drawn in **the player's own colour** (the colour of his record, GFX-OBJ-2.11, not
  the counters' table above) and with **p** as the curve's index (so p pixels lower). M differs from
  the production page: it is the largest sample **within the span** (the last n samples only) over
  the drawn players, rounded **up** to a whole number. A click on an enabled portrait toggles it
  (pressed: its curve shows; released: hidden); the page keeps the eight pressed states while the
  game runs and restores them when it opens again. The colour squares have no action.
  **Start state (answered 2026-11-08 c, look E-trace-020):** the first time the page is opened in a
  game, every present player's portrait is pressed, so every player's curve is drawn (Storm Coast,
  two players: both portraits in the pressed sprite, player 1's line one pixel under player 0's;
  the series list shows `!Size of region` and the span list `!15 Minutes`). Confidence: high (code)
  for the toggling; high (look) for the start state.
- **UI-GAME-8.5g The choice lists and the players' row of 8.5a and 8.5b (answered 2026-11-05,
  Q-258 f; E-ui-018).** (a) What
  8.5a and 8.5b call "option groups" (8.5a: the series at (192, 412) and the span at (18, 412);
  8.5b: the span at (18, 455)) are **drop-down lists, not rows or columns of options**: each is
  one text button of 152 x 20 (`uiElements0` (264, 176), highlighted (264, 200), UI-MENU-1.4)
  that shows the chosen choice's text in `light_system`, colour (227, 189, 168). A click on it
  opens the list of all its choices in their order, one text button per choice, 19 px apart,
  starting one step below the button, or upward from it when the list would run past the bottom
  of the screen. A click on a choice selects it (the button shows its text, the page takes its
  value) and closes the list; a click on the button again or anywhere else closes it unchanged.
  Only one list is open at a time. The choices and their values are 8.5a's and 8.5b's (series 0
  .. 7; spans 15, 60, 240, 960); the shown choice at opening is the remembered one (15 minutes at
  first). (b) **Players' row of 8.5a:** a portrait's tooltip is the player record's own name
  string (the `PlayerObject` name of `FMT-savegame.md`; the observed autosave has `default` for
  the human and `AI` for the computer players), not a made-up "Player #n"; the colour square's
  index c is the player's colour index (the record's colour, 0 .. 7), its cell `uiElements0`
  (520 + 24 (c mod 2), 216 + 24 (c div 2), 24 x 24). Confidence: high for the control and its
  behaviour; medium for the list buttons' exact look (made by a helper that was not read) and for
  where a new game's player names come from.
- **UI-GAME-8.5c Inventory pages (Q-227 d; E-sim-002, both pages).** Two
  dialogs titled `!Inventory`, 512 x 536 (or the remembered position): the
  goods page (help `dialog26_0`, tab 19) and the settlers page (help
  `dialog35_0`, tab 22). Each shows one slot per property of its kind (goods
  on the first, settlers on the second), placed by the property's slot
  number n (LUA-PROP) in a grid from (23, 60) with 54 x 62 cells, six per
  row: x = 23 + 54 (n mod 6), y = 60 + 62 (n div 6) (40 slot controls, so at
  most 40 slots). A slot is a button `uiElements3` (0, 80, 48 x 56) holding
  the icon (`uiElements1` for goods, `uiElements4` for settlers, the
  property's icon cell x 48, 48 x 48) with the property's name as tooltip
  and no click action, and a count label at (+46, +49), right-aligned
  (UI-MENU-1.8 flag 0x10), `small_no_alias_white`, refreshed every frame
  with the total of that property over all the player's warehouses (the
  castle included). The family tabs; the goods page sends `STATS_MENUOPENED`
  with `Inventory`. Confidence: high.
- **UI-GAME-8.5d Buildings page (Q-227 b; E-sim-002).** Title `!Buildings`, help `dialog29_0` (tab 21, tooltip
  `!Size of village and productivity`), placed centred by the base dialog
  placement with the base size, or at the remembered position, its origin
  moved 10 px to the right. Content: one cell per building type **that has
  a building cost** (`bp_addCost`; 31 types in the shipped data: every type
  but the headquarters and the internal `construction` type), ordered by
  the type's **size class** (`bp_setSize`: small 0, medium 1, large 2, mine
  3, harbour 4) and within a class by its **`bp_setSort`** number (two types
  with the same pair: the later in id order wins the cell), in a grid from (23, 62) with 79 x 77 cells, six per row (35
  cell controls): a button `uiElements3` (0, 0, 80 x 80) with the type's
  picture from the `uiBuildings` atlas (the type's icon cell x 80, 80 x 80)
  and the type's name as tooltip; a label at (+6, +10),
  `small_no_alias_white`, refreshed every frame with two numbers separated
  by a **hyphen**: the player's count of **finished** buildings of that type,
  then its count **under construction** (e.g. `3-1`; a building counts as
  finished when both its remaining main and frame work are 0). The two
  counts are the player's per-type tallies, recomputed for all types once
  every 22 logic ticks, so a new site shows up within 22 ticks. For a
  **production type** (one with a first product, `bp_setGoods`: 22 types in
  the shipped data; not the forester, watchtower, catapult, military
  types, storehouse or harbour) a second
  label at (+70, +64), right-aligned, `small_no_alias_white`, showing the
  type's mean productivity as `N%` (`-%` before the first refresh), grey
  (150, 150, 150) when it is 0 and white otherwise. The family tabs; the
  opening sends `STATS_MENUOPENED` with `Buildings`. Confidence: high for
  the layout and labels, the filter, the order and the counts (2026-11-05,
  Q-258 a; E-ui-018); medium for
  the dialog's size. **The frame (2026-11-15, E-ui-035, code reading):** the page's frame picture is
  `uiXXLDialog_tabbed` (4, 8, 512 x 536) (`gfx/uiXXLDialog_tabbed.dds`), drawn 1:1 with its top-left at
  the dialog's origin, the dialog taking its size, 512 x 536 (UI-GAME-8.5); the other statistics pages
  use `uiDepot` (8, 8, 376 x 464). So `uiXXLDialog_tabbed` is drawn (by this page only); `uiXXLDialog`
  itself is still drawn by none (UI-GAME-1.2a). Confidence: high.
- **UI-GAME-8.6 Road building end (UI-GAME-3.4, Q-097).** The world click while
  building sends `extend-road` with the clicked field; the logic
  decides whether the road ends there: the clicked field passes the end test of
  TRN-ROAD-7.4 (a flag, a building door, or a free field where a flag fits) and the
  route search finds a way, else the click is ignored. The waterway mode starts with
  `start-road` (waterway) and uses TRN-ROAD-7. Confidence: high (E-cmd-001).

### UI-GAME-9 Construction view labels

- **UI-GAME-9.1** The HUD button `!Toggle construction view on/off.` (2.2,
  button 3) and its key toggle the construction view: the build-help icons
  over the fields (GFX-VIEW-7) and a floating label over **every own
  building** whose field projects onto the screen. Font
  `light_system_outlined` size 9. Line 1, the building's name (the language
  text of the type's name key), colour (1.0, 0.9, 0.78), at the projected
  point minus d px in y with d = 60 for size classes 0 and 3, 80 for class
  1, 120 for classes 2 and 4 (tall buildings). For a finished building of a
  type with a worker: line 2 at d - 20: the productivity as "<n>%" in grey
  (0.9, 0.9, 0.9); `!Production is locked!` at d - 40 while production is
  stopped; `!No worker at home.` at d - 56 while no worker is inside. The
  labels are not shown outside the construction view. Confidence: high
  (E-ui-004).
  **English texts and conditions (play test 11, `language/gb/controls.txt`):**
  `!No worker at home.` reads **"No worker in building"** and `!Production is
  locked!` reads **"Production is stopped"** (the building window's caption
  `!Production stopped.` is "Production stopped"). The lines are drawn only
  for a **finished** building (no construction work left) of a type **with a
  worker**; "No worker in building" whenever the building's count of workers
  inside is 0, so while the worker is out at work or on his way in or out
  (`woodcutter-label.png`: the woodcutter in his doorway, "Woodcutter's Hut",
  "95%", "No worker in building"), and also before a worker has arrived at
  all; "Production is stopped" while the production switch is off. Both can
  show at once (their own lines). A site shows its name only (9.2).
  Confidence: high.

- **UI-GAME-9.2 What the labels show (play test 10, F31, OBS-33, OBS-34;
  E-ui-008).** The labels of 9.1 are the only text drawn over buildings and
  they are tied to the construction view alone: not to hover, selection or a
  setting. A **site** shows only its name (`site-hover.jpg`); a finished
  building of a type with a worker shows its name and below it the
  **productivity** as an integer percentage (the statistics value of
  `ECO-specialists.md` / the building productivity, clamped at 0), never a
  construction percentage: the "65%" of `woodcutter-door-open.jpg` is the
  finished hut's productivity. A finished building without a worker type
  (storehouse, military buildings, HQ) shows the name only. Text colours as
  9.1: name (1.0, 0.9, 0.784), productivity line (0.9, 0.9, 0.9), drawn
  with an outlined font (white glyphs with a black outline,
  `light_system_outlined`, size 9), each line centred on the projected field
  point. The photos show the text pale blue rather than cream: Q-302.
  Confidence: high (code), the colour is open.
- **UI-GAME-9.3 Hover tooltips over the map (E-ui-008).** Each
  frame, while the pointer is over the map with no dialog or HUD panel under
  it, one tooltip is chosen, first match wins:
  (1) **an own settler** under the pointer (picked by its model): a carrier ->
  `!Carrier`; a soldier -> his rank name followed by " (" hit points "/"
  maximum hit points ")" (medium: two more short parts are appended that were
  not resolved); a specialist (geologist, scout) -> `!Specialist` or the
  specialist's profession name; any other settler (workers, builders,
  donkeys and the like) -> his profession's name. (2) Else, by the field under the pointer: an
  **own flag** -> `!Flag`; else **a building** under the pointer (picked by
  its model; any owner): under construction -> "<name> (Under construction)"
  (`!(Under construction)`, joined with one space); finished and own with a
  productivity >= 0 -> "<name> <p>%" (p rounded down to an integer, at least
  0; medium for the exact joining); otherwise (a foreign building, or no
  productivity) -> "<name>". Else another picked object of the player (a
  ship) -> its own tooltip text (medium); else a map object on the field
  (tree, stone, ...) with a name -> that name; else no tooltip.
  **Placement:** at the pointer plus (0.02, -0.02) of the screen size (2 %
  to the right and 2 % up), moved left so that it does not leave the right
  edge, and kept at least 20 px from the top; a single-line box, 32 px high
  in screen terms of a 1024 x 768 layout (scaled with the screen height),
  drawn with the HUD tooltip frame. It appears in the first frame the pointer
  is over the object and goes when it leaves; there is no delay.
  The O key with an own settler under the pointer makes the camera follow
  him (camera mode 3; the key table of this file), from the same handler.
  Confidence: high for the order, the texts and the click; medium where
  marked.
- **UI-GAME-9.4 What a left click on the map selects (added 2026-11-05,
  Q-1014 b; E-ship-004; the
  dispatch E-ui-006).** With no road being built, a left click
  first clears the selection and then selects the first of: (1) an own flag
  on the field under the pointer; (2) a building under the pointer (picked
  by its model, as in 9.3); (3) a road on the field under the pointer;
  (4) a **ship under the pointer, picked by its model**: the ray through the
  pointer is tested against every pickable object of the scene (a bounding
  sphere around the object, then a finer test), the hit nearest along the
  ray wins, and it counts when it is a ship. A ship is never found from its
  field, so a click selects a ship only when its model is under the pointer,
  whichever field that is. Then the window opens by the **build class of
  the field under the pointer** (`ECO-building-placement.md`): class 0
  (nothing can be placed, e.g. Water or an occupied field) opens the window
  of what was selected (an own ship: the ship window, 4.5); class 1 opens
  the road menu (3.3) and class 2 or more the build menu (3.2), whatever was
  selected. So a click on a ship's model over a coast field where a flag
  fits opens the road or build menu there, not the ship window. Confidence:
  high for the order and the ray pick; medium for "a bounding sphere, then
  a finer test" (the finer test was not read).

### UI-GAME-11 Messages (play test 10, F33, OBS-35; E-ui-009)

- **UI-GAME-11.1 Messages and kinds.** Each player keeps a list of
  messages in arrival order. A message has a kind, a text key, an optional
  name key, an optional field and a timer. Kinds, row icon and filter:
  | kind | filter (tooltip) | icon | texts (key -> English) |
  |---|---|---|---|
  | 1 military | `!Show military messages` | crossed swords | `!Military building occupied!` "Military building manned!" (a military building gets its first soldier), `!Watchtower occupied!` "Lookout tower manned!" (the same for a watchtower), `!Military Building conquered.`, `!Lost Military Building to enemy.`, `!Attacked Castle destroyed.`, `!You are under attack!`, `!Lost Territory.`, `!Soldier killed by Catapult` |
  | 2 geologist | `!Show scholar messages` | magnifier | `!Found Stone!`, `!Found Coal!`, `!Found Gold!`, `!Found Water!`, `!Found Iron Ore!` |
  | 3 economic | `!Show economic messages` | red house | `!No resources found.`, `!Building without worker.` |
  | 4 misc | `!Show misc messages` | exclamation mark | `!Building constructed` "Building completed", `!Ship constructed`, `!Ship waiting for Orders` |
  | 5 to 7 | always shown | speech bubble | e.g. 7 `!Autosave...` |
  The row text is the translated key, followed by " (" + translated name +
  ")" when the message carries a name (the building's type name for the
  building messages), e.g. "Building completed (Woodcutter's Hut)".
  Duplicates: "found", "no resources", "building without worker" and "lost
  territory" are not added again while a message of the same kind and text
  exists at (about) the same field (for "found", "no resources" and
  "building without worker" that message's timer is reset to 600 instead);
  "under attack" is not added again while the newest one with that text
  within one field of the place still has a timer of at least 540, i.e. is
  at most 60 **timer units** old (11.2; with the fixed 1/60 s per tick that
  is 3600 ticks): its timer is reset to 600 instead; an older one stays as it
  is and a new message is added (corrected 2026-10-18, Q-222; E-gfx-025). Confidence: high (E-ui-009).
- **UI-GAME-11.2 Lifetime.** The timer starts at 600; in every logic tick
  it loses the duration of the last drawn frame in seconds, and once a
  second the window removes messages whose timer is <= 0. The lifetime in
  real time therefore depends on the frame rate: 600 / (ticks per second x
  frame time), one hour at 60 frames per second and game speed 1, half
  that at speed 2. An implementation may use a fixed 1/60 s per tick (36 000
  ticks). Removing a message by hand: right click (11.4). Confidence: high
  for the rule; the equivalence is derived.
- **UI-GAME-11.3 Window.** Shown at the top left while the messages
  toggle (2.2 button 7, key N) is on. Four round filter buttons in a row at
  (36 + 35 k, 10) px, k = 0 .. 3, 32 x 32, from left to right military,
  geologist, economic, misc (`uiElements3` icons 24 x 24: dim at y 248, lit
  at y 280, x = 72 swords, 104 magnifier, 40 house, 8 exclamation mark; lit =
  the kind is shown); clicking one toggles its kind (`uiClick0`). Below, **7
  message rows**, newest at the bottom, 29 px apart: the kind's square icon
  (`uiElements3` y 344, 28 x 28, x as above; 136 the speech bubble for kinds
  5 to 7) and the text 32 px right of it and 12 px lower, font
  `light_system_outlined`; row tooltip `!Left Click for go to, right click
  for delete.` To the left a column of three buttons: scroll up (`!Scroll
  up.`), scroll down (`!Scroll down.`) and jump to the newest (`!Jump to last
  one.`); held, up and down repeat every 0.05 s; the view follows the newest
  message until scrolled up, and again after "jump to last". Up is enabled
  while older messages are hidden, down and jump while newer ones are.
  Confidence: medium for the pixel positions (read from the layout code and
  the photo), high for counts, icons and behaviour.
- **UI-GAME-11.3a Shown at every game start (2026-11-14 b, F62; E-ui-032, code reading).** The
  messages window is **on** when a game starts: a new free game, a loaded game, a campaign mission or a
  tutorial lesson all build the game screen with the window shown (its "shown" switch is set on there,
  with nothing read before it). N and main-bar button 7 (2.2) toggle it for the rest of that game
  only. **No setting and no profile field holds it:** the switch is not in the settings file
  (`SET-settings.md` SET-2) and not in a save, so every new start shows it again, whatever the player
  did in the last game. The game settings page's `!Infowindow` choice is a different thing (the help
  panel, SET-5.6) and does not touch the messages. Since a new message sounds only while the window is
  shown (11.4), a game plays the message sounds from its start. Test T56. Confidence: high (the
  window's constructor and the game start both set it on; the toggle is the only other writer).
- **UI-GAME-11.4 Clicks and sounds.** Left click on a row: the message is
  marked as read; if it has a field the camera **jumps to that field**
  (GFX-VIEW-3.3 jump: target at the field, rotation reset, zoom kept); a
  kind-5 message opens the ship window for its ship instead of (before) the
  jump. Right click on a row deletes the message and plays `uiRemove`. A
  new message plays `uiMessage1` when it is military, otherwise
  `uiMessage0`, only while the window is shown. Confidence: high
  (E-ui-009). **Seen (2026-11-15 night, E-trace-028, Jens's 2023 save at 2560 x 1440):** seven rows
  29 px apart, the four filter buttons in a row above them, the three scroll buttons in a column at the
  left; some rows' icons carry a **green tick** over their lower right part (taken to be the read
  mark of the click above; medium: which messages were clicked in that save is not known), the others
  none. Confidence: high for the look, medium for the tick's meaning. **Confirmed (2026-11-15 day,
  E-trace-030, Seafaring tutorial):** a left click on a row's text did nothing; a left click on the
  row's **icon** marked it: the icon got a green (teal) frame with a green tick over its lower right
  corner, the row's text turned grey, and the camera jumped to the message's ship. The other row
  kept its plain icon and white text. Confidence: high for the tick's meaning (seen before and
  after one click); the click target being the icon, not the whole row, is seen once (medium).
  **Seen, the kinds in a free game (2026-11-16 night, E-trace-031, 2560 x 1440):**
  - misc: "Building completed (Forester's Hut)", "Building completed (Barracks)", with the red square
    "!" icon;
  - military: "Military building manned! (Barracks)", with the crossed swords icon;
  - a chat line: "Look (All): GiveMeSomeMore" (the sender's name, "(All)", then the text), with an
    orange speech-bubble icon, one row per line sent (the chat line opens with RETURN at the top left,
    under the message rows);
  - a click on the military row's icon moved the camera to the barracks and marked the icon, as above.
  The sounds are not in pictures (11.4's `uiMessage1` / `uiMessage0` from the code stand).
  **Seen, three more kinds (2026-11-16 day, E-trace-032, free game on Keyhole, 2560 x 1440):**
  - geologist: "Iron ore found! (Geologist)" (`!Found Iron Ore!` with the name "Geologist"), with
    the magnifier icon, a short while after a geologist was sent to a mountain flag;
  - economic: "No resources found (Fisherman's Hut)" (`!No resources found.`, English without the
    full stop), with the orange-red house icon, for a fisherman's hut with no water within its
    radius, some game minutes after the hut was completed (not at its first search; consistent with
    `ECO-outdoor-workers.md` ECO-OUT-3.11's entry starting at 0.5);
  - military: "Territory lost to enemy!" (`!Lost Territory.`) with the crossed swords icon, **one
    row per lost place**: six rows within about a minute while a stronger neighbour's new military
    buildings took our border fields (the duplicate rule of 11.1 keeps one per place, not one per
    event).
  Still not seen: "You are under attack!" (no computer player attacked our buildings in about 30
  real minutes at speed 20; the Hard players' attack test (`AI-military.md` AI-MIL-5.4) never
  passed against our cheat-raised soldier strength, and the Medium neighbour never met the
  ring condition of `MIL-attack-combat.md` MIL-ATK-1.2) (Q-095).

### UI-GAME-10 Tutorial marks

- **UI-GAME-10.1** For a step that expects a map click the scene draws the
  pointer sprite `uiTutorialPointer` (64 x 64) at the step's field
  (`CAM-campaign-missions.md`); the pointer is removed as soon as the dialog
  holding the expected button opens. For a step that expects a button no
  marker is drawn: the text window names the button and a click elsewhere
  shows `T01_ERROR_SelectMarkedButton`. `uiTutorialCircle.dds` is shipped
  but never referenced. Confidence: medium (E-ui-004).

### UI-GAME-11 Screen frame

- **UI-GAME-11.1 Pieces.** The game view carries an ornamental frame read
  from `data/settings/graphics.xml` (`GUI` / `Border`): texture
  `data/gfx/uiframe.dds` (256 x 256), colour (1, 1, 1, 1), and eight pieces
  given as atlas rectangles (x, y, w, h): `Top`, `Bottom`, `Left`, `Right`
  = (0, 83, 256, 12); `TopLeft` = (0, 0, 80, 80); `TopRight` =
  (176, 0, 80, 80); `BottomLeft` = (0, 112, 120, 144); `BottomRight` =
  (136, 112, 120, 144). Confidence: high (E-ui-005, graphics.xml).
- **UI-GAME-11.2 Layout per resolution.** With W x H the backbuffer size in
  pixels (nothing is scaled, 1.1): the corners are drawn at their rectangle
  size in the screen corners, TopLeft at (0, 0), TopRight at (W - 80, 0),
  BottomLeft at (0, H - 144), BottomRight at (W - 120, H - 144). The four
  edges are drawn as bars of the strip height (12 px) along the whole
  edge: Top at (0, 0) W x 12, Bottom at (0, H - 12) W x 12, Left at (0, 0)
  12 x H, Right at (W - 12, 0) 12 x H. Draw order: Top, Bottom, Left, Right,
  then the four corners (the corners cover the bars). The frame is part of
  the game scene (drawn after the 3D view, under the interface); menus draw
  no scene and therefore no frame. Confidence: high (E-ui-005).
- **UI-GAME-11.3 Edge texturing.** The edge bars do not tile the 256 px
  strip: a bar's texture coordinates run over the strip's 12 rows (v) but
  only over a sliver of its width (u from the strip's x to x + L / 256 / w
  texels, L = the bar's length, w = 256), so the first few texels of the
  strip are stretched along the edge and the vertical bars are not
  rotated. Visually each edge is a thin bar in the strip's colours; an
  implementation may draw the bar from the strip's first 4 texels.
  Confidence: medium (the quad construction is traced; its look is only
  checked against the photo `speed-and-frame.jpg`).

### UI-GAME-12 Keyboard

- **UI-GAME-12.1 In-game keys.** Keys are read as Windows virtual key
  codes on press (not repeated while held) unless noted; SHIFT and CTRL are
  modifiers only where listed. Keys that duplicate a main-bar button
  (2.2) do exactly what the button does:

  | key | action |
  |---|---|
  | ESC | toggle the game menu (`UI-menus.md` UI-MENU-3); same as button 9 |
  | RETURN | open the chat line (button 8); SHIFT+RETURN the team chat line; with the chat line open RETURN sends the line. **Also in a single-player game** (2026-11-05, Q-258 e; E-ui-018): RETURN opens the line whenever the save dialog and the game menu are closed; on RETURN with the line open, a game that is not a network client first compares the line with the cheat words of `CAM-campaign-missions.md` CAM-3.7 (`TooFast`, `ShowMeTheWorld`, `INeedSomeHelp`, `FreedomOfMovement`, `GiveMeSomeMore`) and applies a match, so the chat line is how cheats are typed. **The line's look (answered 2026-11-08 c, Q-258 e; looks E-trace-020, E-trace-021, 2560 x 1440):** a box of 200 x 32 px with its top-left at (15, 275), filled black at about half opacity (the picture under it keeps 0.49 of its brightness), no frame, no title, no prompt text; an empty line shows only a white text cursor 1 px wide and 10 px tall at x = 47, y = 284 .. 293 (32 px right of the box's left edge). RETURN and main-bar button 8 open the same line; ESC closes it. RETURN while the statistics dialog was open did not open it (seen once). Confidence: high for the 2560 x 1440 position and size; whether the position scales with other screen sizes was not seen |
  | SPACE | toggle the construction view (button 3, UI-GAME-9) |
  | L | toggle the global dialog, tools page (button 6, UI-GAME-6.1); other dialogs close |
  | S | open the settings dialog at its last page (SET-5) |
  | C | toggle the statistics / planning dialog (button 4, UI-GAME-7) |
  | I | toggle the inventory dialog (UI-GAME-7.1), closing other dialogs when it opens |
  | B | toggle the buildings overview dialog (UI-GAME-7.1) |
  | M | toggle the minimap (button 5, 2.5) |
  | N | toggle the messages window (button 7, 2.3) |
  | T | toggle the briefing panel (button 1, 2.4) |
  | V | cycle the game speed 1 -> 2 -> 3 -> 1 (button 2, 2.7); only without SHIFT or CTRL |
  | P | pause / resume: while paused the simulation speed is 0 and the speed is restored afterwards (a network client that is not the host cannot pause); measured in live trace 1 (E-trace-001): the speed value is 0 while paused and nothing in the world moves (OBS-31) |
  | H or HOME | zoom back to the start zoom level and jump the camera to the player's headquarters (if it still exists) |
  | . (period) | jump to the next finished military building of the player and open its window (4.3); SHIFT+. the previous one |
  | , (comma) | jump to the next headquarters / storehouse / harbour of the player and open its window (4.4); SHIFT+, the previous one |
  | O | with a settler under the mouse pointer: the camera follows that settler (free camera mode 3) until the next camera input |
  | Y | hide / show the interface (HUD and frame) for screenshots |
  | arrows | scroll the camera (held) |
  | INSERT / DELETE | turn the camera by one of its three yaw positions (GFX-VIEW-3) |
  | PAGE DOWN / wheel up, PAGE UP / wheel down | zoom in / out one level |
  | middle button drag | pan |
  | right button held | free look: the mouse turns the view, the arrows fly; dropped on release (GFX-VIEW-3.2a) |
  | X | multiplayer only: toggle the player list dialog |
  | PRINT SCREEN | save a screenshot (`TOOL-capture.md` TOOL-2) |
  | numpad + / numpad − | screenshot size factor 1..8 (TOOL-2.2) |
  | F9 | debug camera mode (free fly); an implementation may omit it |
  | CTRL+SHIFT+S | same as S (the S binding is tested first and takes the key event, so the camera path tool behind it is unreachable, TOOL-3.1) |

  "Next" for . and , walks the player's building list from the last building
  the key jumped to, wrapping to the first; only finished buildings count.
  **The list and the cursor (2026-11-05, Q-258 b; E-ui-018):** the
  list is the game's one list of all buildings of all players in **creation
  order** (a new building is appended, a removed one taken out), filtered
  to the player's; each key ('.' military, ',' depots) keeps its own cursor,
  the last building it jumped to. A press finds the cursor's building in
  the list (with no cursor: the player's first building in the list) and
  takes the first later building of the player that is finished and of the
  key's types, wrapping to the list's start (the cursor's building itself
  can be taken again when it is the only one). If the cursor's building no
  longer exists, or nothing qualifies, the press only clears the cursor
  (no jump); the next press starts afresh. Military types: barracks, guard
  house, tower and fortress (not the watchtower or the catapult); depot
  types: harbour, storehouse, headquarters.
  There is no key for the build menu, the flag menu or road building; the
  key table of 41 letters the input system fills at start is never read.
  The chat line swallows all keys while open. Confidence: high for the
  bindings (E-ui-005), medium for Y hiding the frame (the scene call it makes is read as the
  frame's visibility setter, reached indirectly).
- **UI-GAME-12.2 Dialog keys.** Context dialogs have no keyboard shortcuts;
  a click outside them or ESC (game menu) is the only way out besides their
  close button. Text inputs (chat, save name) take RETURN. Confidence: high.

### UI-GAME-13 Cursor icon on the map

- **UI-GAME-13.1 Field icon.** Whenever the pointer is over the map, no
  context dialog is open and the pointer is not over a dialog or the
  minimap, the hovered field shows a screen-space icon 46 x 32 px from
  `uiElements3` whose top-left corner is 22 px left of and above the
  field's projected point (the anchor of GFX-VIEW-6.2): by the field's
  build class for the player (ECO building placement): 1 flag (448, 0)
  yellow flag; 2 small (256, 0) hut; 3 medium (304, 0) house; 4 large
  (352, 0) castle; 5 mine (400, 0) hammer and pick; 6 harbour (312, 32)
  anchor; class 0: the flag icon (448, 0) when a flag stands on the field,
  else (448, 66) a building with a red cross. The icon is shown in every
  mode, not only in the construction view; the construction view's
  build-help billboard (GFX-VIEW-6.1) is omitted on the hovered field, so
  the icon takes its place there (the icon does not vanish, it changes
  from a ground billboard to the flat cursor icon). Confidence: high
  (E-ui-005).
- **UI-GAME-13.2 Road-building icons.** While a road or waterway is being
  built the hovered field shows instead a 40 x 32 icon: (8, 456) a flag on
  a road when the road may continue to the field and a flag fits there;
  (88, 456) a road piece when it may continue but no flag fits; (48, 456)
  a flag with a no-entry sign when the road cannot use the field. In
  addition the ring and neighbour markers of GFX-VIEW-6.2 are drawn (road
  building only). Confidence: high (E-ui-005).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| hud_bar_rect | uiGame (0, 184, 504, 96) | px | high | E-ui-003 |
| hud_bar_position | (W/2 - 252, H - 99) | px | high | E-ui-003 |
| hud_button_size | 48 | px | high | E-ui-003 |
| hud_button_rows | normal y 296, hover y 352 | px | high | E-ui-003 |
| flag_menu_origin / step | (15, 29) / 52 | px | high | E-ui-003 |
| flag_menu_portraits | uiElements3 x = 0, 48, 96, 144, 192; y 144 normal / 192 highlighted | px | high | E-ui-004 |
| road_menu_sprites | deploy flag (368, 160) / (416, 160); destroy road (240, 144) / (240, 192) | px | high | E-ui-004 |
| context_dialog_placement | bottom-right corner at the click; x <= W - 600; x in [0, W - w]; y in [0, 0.85 H]; tutorial (20, 20) | px | high | E-ui-004 |
| dialog_title | font_big_white size 8, colour (0.996, 0.937, 0.486), at (w/2, 14), capitals only | | high | E-ui-004 |
| smart_cursor | pointer to the first button's centre, clamped 1 % .. 99 % | | high | E-ui-004 |
| game_speed_levels | 1, 2, 3 (cycle) | x normal | high | E-ui-004 |
| speed_indicator | uiGame rows 640 (speed 2) / 704 (speed 3), 6 frames of 64 px, 50 x 50 at (W - 76, 20) or (W - 382, 20) with the minimap | px | high | E-ui-004 |
| building_label_offsets | 60 / 80 / 120 px by size class; lines at -20, -40, -56 | px | high | E-ui-004 |
| building_grid | origin (23, 60), cell 80, 4 per row | px | high | E-ui-003 |
| icon_tab_size | 40 x 27, x = 8 + 40 id | px | high | E-ui-003 |
| good_icon_cell | 48 | px | high | E-ui-003 |
| building_icon_cell | 80 | px | high | E-ui-003 |
| building_window_buttons | column (407, 157) step 50; military (407, 212) | px | high | E-ui-003 |
| soldier_slot_step | 56, 3 per row | px | high | E-ui-003 |
| input_slot_step | 60, from x 395 leftwards, 3 (mint 6) | px | high | E-ui-003 |
| tools_rows | 12 at (87, 70 + 30 i) | px | high | E-ui-003 |
| reset_button | uiElements3 (440, 264) at (295, 433) | px | high | E-ui-003 |
| error_message_time | 5 | s | medium | E-ui-003 |
| hq_reserve_max | 15 per rank | soldiers | high | dialogs.txt `dialog3_2` |
| frame_pieces | uiframe.dds: edges (0, 83, 256, 12); TL (0, 0, 80, 80); TR (176, 0, 80, 80); BL (0, 112, 120, 144); BR (136, 112, 120, 144) | px | high | E-ui-005, graphics.xml |
| frame_layout | corners at the screen corners, edge bars 12 px along the whole edge | px | high | E-ui-005 |
| cursor_field_icon | uiElements3 46 x 32: flag (448, 0), small (256, 0), medium (304, 0), large (352, 0), mine (400, 0), harbour (312, 32), nothing (448, 66); anchor (-22, -22) | px | high | E-ui-005 |
| cursor_road_icon | uiElements3 40 x 32 at y 456: x 8 flag fits, 88 passable, 48 blocked | px | high | E-ui-005 |
| key_bindings | see UI-GAME-12.1 | | high | E-ui-005 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | 1024 x 768 screen | draw the HUD | bar at (260, 669); button 1 at (295, 709), button 9 at (692, 709) |
| T2 | own flag with water on the next field, not the HQ flag | open the flag menu | five enabled buttons; waterway enabled |
| T3 | HQ flag | open the flag menu | destroy button disabled |
| T4 | free field, site kind medium | open the build menu | tabs small and medium; grid lists medium types with large ones absent |
| T5 | free field where a flag does not fit | open the build menu | flag button disabled, tooltip `!Flag too close to another flag.` |
| T6 | woodcutter window, worker out | open | `!No worker at home.` shown, production switch enabled, no shipyard button |
| T7 | mint window | open | six input slots at x 395, 335, 275, 215, 155, 95 |
| T8 | guard house (2 soldiers) with one soldier | open | two slots at (137, 45) and (193, 45), one filled |
| T9 | tools dialog, slider of tool 3 moved to 0.75 | release | `set-tool-weight`(tool 3, 0.75) is sent |
| T10 | military dialog | click `!Prefer strong soldiers` | `set-soldier-preference`(1); the button becomes pressed and disabled, the other enabled |
| T11 | any dialog button | release the mouse over it | `uiClick1` plays once; the action runs once |
| T12 | attack dialog, 7 soldiers available, slider 0.5 | `!ATTACK` | `attack-building` with the count shown by the label |
| T13 | 1024 x 768, click on an own flag at (300, 400), flag menu 200 x 300 | open | window at (100, 100) (bottom-right corner on the click); the pointer moves to the centre of the "create a street" button; title FLAG MENU |
| T14 | 1024 x 768, click at (900, 700), dialog 200 x 300 | open | x = min(700, 424) = 424, y = min(400, 653) = 400 |
| T15 | tutorial running, click on a flag | open | flag menu at (20, 20) |
| T16 | speed 1 | speed button | speed 2; the running-settler clock "2" appears at (948, 20) on 1024 x 768; next presses: 3 ("3"), then 1 (hidden) |
| T17 | construction view on, woodcutter (size class 0) at screen (500, 400), productivity 73, worker out | draw | "Woodcutter" at (500, 340); "73%" at (500, 360); "No worker at home" at (500, 404) |
| T18 | 1024 x 768 | draw the frame | TopRight corner at (944, 0) 80 x 80; BottomLeft at (0, 624) 120 x 144; Top bar (0, 0) 1024 x 12; Right bar (1012, 0) 12 x 768 |
| T19 | pointer over a free large site at screen (400, 300), no dialog open, construction view off | draw | castle icon (352, 0) at (378, 278); with the construction view on the same icon and no billboard on that field |
| T20 | building a road, pointer over a field where the road may continue and a flag fits | draw | icon (8, 456) at the field; the ring and the neighbour markers are drawn |
| T21 | game running, no dialog | press SPACE, then N, then V | construction view on; messages window shown; speed 2 |
| T22 | two finished guard houses | press . twice, then SHIFT+. | camera jumps to guard house A (window opens), then B, then back to A |
| T23 | flag-only spot off any road | left click | the road menu with one button "Erect flag" |
| T24 | road being built, click on its current end where a flag fits | click | Flag Menu with "Erect Flag" and "Cancel road construction" |
| T25 | large site | open the build menu | tabs small, medium, large; small page shown; cell 1 flag, cells 2.. the small types |
| T26 | mine site | open the build menu | no tabs; cell 1 flag, then the mine types |
| T27 | road being built from flag A, click on flag B three fields away | click | `extend-road`(B); the logic builds the road A-B; the mode ends |
| T28 | road being built, route A-x-y, click on x | click | the route is cut back to A-x |
| T29 | click on an own flag | flag menu | 288 x 88 px small panel, 22 px header with FLAG MENU and the close button at (248, 5), five portraits in a row from (15, 29) every 52 px (1.8, 3.1) |
| T30 | construction view on; own woodcutter site; own finished woodcutter's hut with productivity 0.65 | labels | site: "Woodcutter's Hut" only; hut: "Woodcutter's Hut" and 20 px lower "65%" (9.2) |
| T31 | construction view off, pointer on the same site | tooltip | "Woodcutter's Hut (Under construction)" at pointer + (2 %, -2 %) of the screen; no label over the site (9.3) |
| T32 | pointer on the finished hut, then on its flag, then on a carrier on the road | tooltip | "Woodcutter's Hut 65%", then "Flag", then "Carrier" |
| T33 | own woodcutter's hut finished, then a barracks finished, then its first soldier arrives | messages | rows "Building completed (Woodcutter's Hut)" (!), "Building completed (Barracks)" (!), "Military building manned! (Barracks)" (swords), newest last; sounds uiMessage0, uiMessage0, uiMessage1 (11.1, 11.4) |
| T34 | click the military filter off | rows | only the two misc rows remain; on again: all three |
| T35 | left click on "Building completed (Barracks)" | camera | jumps to the barracks' field (target z - 18, yaw 0, zoom unchanged); right click instead: the row disappears with uiRemove |
| T36 | 60 frames per second, speed 1, no clicks | lifetime | the message disappears after 3600 s (timer 600 - 36 000 x 1/60), removed in the next 1-second sweep |
| T36a | fixed 1/60 s per tick; "You are under attack!" for field F at tick 0, then attack events at F at ticks 3000, 6500 and 6700 | messages | tick 0: new message M1 (timer 600); tick 3000: M1's timer 550 >= 540 -> reset to 600, no new row; tick 6500: M1's timer 600 - 3500/60 = 541.7 -> reset to 600; tick 6700: 596.7 -> reset. Second case: message at tick 0, next event at tick 3601: timer 539.98 < 540 -> a second row M2 (timer 600), M1 left as it is (11.1) |
| T36b | "Building without worker" for building B at tick 0, the request still unserved at tick 20 000 | messages | one row; its timer is reset to 600 at tick 20 000 (and on each later failed try), so it never runs out while the request stays unserved (11.1, ECO-REQ-3.8) |
| T37 | construction view on; finished woodcutter's hut, productivity 0.95, its woodcutter walking out through the door; English texts | labels | "Woodcutter's Hut", 20 px lower "95%", 56 px below the name "No worker in building"; the third line goes in the frame after the worker's walk-in step ends (9.1) |
| T38 | road from flag F being built, route end E on screen at (600, 420); the player clicks E and then `!Create Flag.` | pointer | on the click the deploy-flag menu opens and the smart cursor jumps to `!Create Flag.`; after the click on it the pointer is at (600, 420), E's projected point, and the menu is closed; the same after `!Cancel street construction.` or a right click (1.10a) |
| T39 | Roman large site, build menu | large page | flag cell, then farm (uiBuildings0 80, 320), pig farm (400, 320), donkey breeding (160, 320), fortress (320, 320); no castle, no second flag icon (3.2) |
| T40 | Roman small site | small page | flag, woodcutter (80, 80), forester (160, 0), stone pit (240, 0), fisher (80, 0), hunter (0, 80), lookout tower (400, 0), well (160, 80), barracks (320, 0), guard house (240, 80) (3.2) |
| T41 | `controls.txt` lines `!AI<TAB>AI`, `!TFTP_ERROR_READERROR<TAB>Read error<TAB>`, `!TFTP_ERROR_WRITEERROR<TAB>Write error`, `!Time expired<TAB>` with the end-of-demo text, which holds one `\T` pair; `briefings.txt` `b01_A_00<TAB>` with mission 1's title, `\n\n` and its first paragraph (UI-GAME-1.3a) | the loaded table | `!AI` -> "AI"; `!TFTP_ERROR_READERROR` -> "Read error"; no key `!TFTP_ERROR_WRITEERROR` (the held key is LF + it), so `!TFTP_ERROR_WRITEERROR` shows as itself; `!Time expired` -> the file's text with each `\n` a line break and the `\T` pair dropped (the backslash and the T both); `b01_A_00` -> the title, two line breaks, the paragraph; `Controls Keys` -> the text of `controls.txt`'s first line (a version date) unchanged; 1542 entries in all after the start of a free game (1539 from the files, 3 added by lookups) |
| T42 | 1.10c: smart cursor on, 60 frames a second, a free field the player may place a flag on; left press at 0 ms, release at 80 ms on the field, then a second press at 200 ms and release at 280 ms without moving the mouse | the two clicks | the first opens the build menu (in the first frame update after 0 ms) and moves the pointer onto its flag button; the second is a click on the flag button: the flag command for the field is sent, the menu closes and the pointer is warped onto the field (1.10a, 1.10b) |
| T43 | 1.10c: as T42, but the second press comes 3 s after the first, the mouse not moved | the second click | the flag is placed as in T42: there is no time limit |
| T44 | 1.10c: as T42, but between the clicks the mouse is moved so the pointer leaves the flag button by a few pixels | the second click | a click where the pointer now is, not on the flag button: no flag from it (no distance tolerance beyond the button) |
| T45 | 1.10c: the game at 10 frames a second (frame updates 100 ms apart, at 0, 100, 200 ms); a left press at 110 ms and its release at 150 ms on a free field | the frame update at 200 ms | the click is not seen: no menu opens (its release cleared the press mark before the update read it); a press at 110 ms released at 230 ms opens the menu in the update at 200 ms |
| T47 | 8.5e: a sawmill finishing 3 cycles in one 600-tick cycle after tick 600, a mint 1 | production counters | plank +3, coin +1 in the current sample, series 5's total +1 |
| T48 | 8.5e: a sawmill finishing a cycle at tick 300 | counters at tick 601 | plank has one sample, 0: the count was dropped |
| T49 | 8.5e: farm, mill, piggery, donkey breeder each finish a cycle | counters | all fourteen unchanged; each building's productivity rises |
| T50 | 8.5f: canvas at (x0, y0), wood marked, samples ... 0, 2, 4 (newest 4), history's largest 8, span 15 | curve | newest point (x0 + 245, y0 + 185 - 92 + 0 = y0 + 93), next (x0 + 228.67, y0 + 139), next (x0 + 212.33, y0 + 185); top label 8; time labels -15 .. -0 |
| T51 | 8.5: any game | HUD button 4 / button 6 | window 25 / window 18 opens; the Planning and Logic menus never appear |
| T46 | 1.11: any game, paused with P | the picture | every stored pixel v becomes round(v x 90 / 255) (255 -> 90, 200 -> 71, 100 -> 35, 10 -> 4): 0.3529 of the unpaused frame's colour (black at alpha 0.65 over it, the main bar included), "Paused" in `light_system` at the screen centre; unpaused the same frame without the layer |
| T52 | 9.4: a game, an own ship on a Water field W next to a coast field C where a flag fits; the pointer on C with the ship's model not under it, then on W with its model under it | left click each time | first: the road menu (class 1), no ship selected; second: the ship window of that ship |
| T53 | 8.1a: a finished harbour with 2 planks, 6 stones and no builder in stock, no expedition ship of the player | open tab 12; press `!Exploration.`; wait one pass (SHIP-5.2) | before: labels `0/1`, `0/6`, `0/4`, `0/1`, start enabled, jump disabled with `!No expedition started`; after the press: start disabled with `!Exploration is running!`; after the pass: `0/1`, `6/6`, `2/4`, `0/1`; no control on the page changes them or stops the preparation |
| T54 | 8.5d: a player with 2 finished woodcutters, 1 woodcutter site and a forester | open the buildings page; wait 22 logic ticks after placing a second forester site | 31 cells, the first row woodcutter, forester, stone pit, fisher, hunter, watchtower; woodcutter label `2-1` with a productivity label, forester `1-0` and no productivity label; within 22 ticks the forester shows `1-1` |
| T55 | 8.5g: production page, span `!15 Minutes` | click the span button; click `!4 Hours` | a list of four buttons 19 px apart below the button opens; after the second click the button reads `!4 Hours`, the list is closed and the graph uses 240 samples |
| T56 | 11.3a: a profile with `!Infowindow` Off (field 17 = 1); a free game; N pressed during it; quit; a second free game | start the first game; press N; start the second game | first game: the messages window is shown from the first frame, and a new message plays its sound; after N it is hidden and new messages are silent; the second game shows the window again; the profile file is unchanged by N |
| T57 | 4.1a: a finished woodcutter's hut, production on, its worker out felling; then the worker home; then production switched off | open its window at each step | frame 480 x 224 (`uiMediumDialog`); `BUILDING`, `PRODUCT`, `TOOL` centred on x 64, 160, 232 and on y 37 (ink y 34 .. 40, above the slots at 43); three empty slot frames at x 275, 335, 395; `PRODUCTIVITY` with its left edge at x 136; worker out: line 1 empty, line 2 "No worker in building" at (40, 182); worker home: both lines empty; production off (worker home): line 1 "Production stopped" at (40, 160), line 2 empty; never a worker activity text |
| T58 | 8.5 (F61): 2560 x 1440, the statistics opened with C for the first time; then each tab clicked in turn | look at the frame | the frame's top-left at (1024, 452) for every page; statistics, production, inventory, settlers and fleet 376 x 464, buildings 512 x 536; the title centred in the header bar; nothing moves when a tab is clicked except the right and bottom edges for the buildings page |
| T59 | 4.4: the headquarters' military page, Private reserve 1 with one private in the headquarters | click the Private bar at its right end, at its middle, at its left end | `1/15` with the bar filled to the knob; `1/8`; `0/0` with no fill; each click moves the knob at once; `set-soldier-reserve`(0, value) sent each time |
| T60 | 4.1a (g): a bakery with flour 4 and water 2; a coal mine with fish 1, meat 2, bread 2 | open both windows | bakery: water `2/6` at slot 395, flour `4/6` at 335, slot 275 empty; coal mine: fish `1/2` at 395, meat `2/2` at 335, bread `2/2` at 275; both gauges at x 137 .. 442, y 113 .. 129 |
| T61 | 6.5: an economy slider (200 x 20) at value 0.5, enabled; then at 0.2 disabled; a settings slider (152 x 24) at 1.0 | pixels drawn | 0.5: bar (0, 0, 200 x 20) of `uiElements2`; fill `uiElements2` (0, 104, 100 x 20) over the slider's x 0 .. 99, plain; knob `uiElements0` (8, 104, 20 x 20) at x 90.5 .. 110.5 (left 2 + 88.5); 0.2: fill row 80, 40 px wide, tinted (170, 170, 255) at alpha 170/255; knob left at 37.4; settings slider at 1.0: bar `uiElements0` (264, 344, 152 x 24), no fill, knob left at 131 (right edge 151) |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | pause and speed log: the speed value 0 while paused, 1 / 2 / 3 from V | P and V keys |
| E-heap-001 | memory read of the running game | the heap dumps of E-trace-007 at trace ticks 1 and 6000: the text table's tree (its nodes: key and text strings) | UI-GAME-1.3a, T41 |
| E-ui-003 | code reading | | event id table, dispatcher, senders, click, flag menu, icon tab, error box |
| E-ui-003a | code reading | | HUD constructor (bar, buttons, tooltips) |
| E-ui-004 | code reading | | placement, smart cursor, button sprites, labels, speed, tutorial marks, scale |
| E-ui-003b | code reading | | build menu tabs and grid |
| E-ui-003c | code reading | | building windows |
| E-ui-003d | code reading | | global dialogs |
| E-ui-003e | code reading | | minimap, messages, end screen, help pages |
| E-ui-003f | data files | `data/language/gb/controls.txt`, `dialogs.txt`; `data/gfx/ui*.dds` | keys and atlases (their sizes) |
| E-ui-005 | code reading | | screen frame, keys, cursor icons, flag-spot and deploy-flag menus, text buttons |
| E-gfx-025 | code reading | | message timer units (Q-222) |
| E-ui-009 | code reading | | messages (11) |
| E-ui-008 | code reading | | hover tooltips, label colour packing (9.2, 9.3) |
| E-ui-006 | code reading | | build menu, road building, deploy-flag menu |
| E-ship-004 | code reading | | UI-GAME-8.1a, 9.4 |
| E-ui-018 | code reading | buildings page (filter, order, counts), the . and , keys, ship name, RETURN (chat line, cheats), production page option groups | UI-GAME-8.5b, 8.5d, 12.1, 4.5 c |
| E-cmd-001 | code reading | S2DNG.exe | statistics series, road end |
| E-pt2-001 | code reading | | dialog frame sheets, flag menu size, ship arrows, smart cursor targets |
| E-sim-006 | code reading | | UI-GAME-1.10a, 1.10b (Q-262 b) |
| E-ui-011 | code reading, data files | `scripts/properties/buildings.lua` (sort, size, icon cells, castle construction time -1), `language/gb/controls.txt` | UI-GAME-1.10a, 3.2 pages, 9.1 texts (play test 11) |
| E-ui-012 | code reading | imports: no GetDoubleClickTime, GetSystemMetrics or GetMessageTime; GetTickCount only for a random number, the low-quality water and a wait loop | UI-GAME-1.10c |
| E-ui-013 | code reading and data | `gfx/uiTip.dds` (16 x 16 DXT5, all texels (0, 0, 0, 255)); the look comparison's pictures of 2026-10-30 (original paused, ours unpaused) | UI-GAME-1.11, T46 |
| E-ui-014 | code reading, data file | `ui_shader.fx` (fixed-function passes) | UI-GAME-1.11, T46 |
| E-ui-015 | code reading | counters (for each worker kind), frags, page data and refresh, graph, menus, window slots | Q-258 d, f |
| E-sim-002 | code reading | | UI-GAME-8.5, 8.5a |
| E-ui-019 | code reading | (= E-gfx-039, session 2026-11-06) | UI-GAME-8.5f |
| E-ui-021 | code reading | (= E-gfx-055, session 2026-11-11 eve): the build menu's flag button names its icon sheet directly, the building cells by the tribe's sheet name | UI-GAME-3.2 (Q-209 a) |
| E-eco-047 | code reading | the build menu's military cells use the placement's military clearance (`ECO-building-placement.md` ECO-PLACE-6) | UI-GAME-3.2 |
| E-cam-017 | code reading | the briefing panel's coin line reads the result record of the local player | UI-GAME-2.4 |
| E-ui-040 | code reading | the road-building click and right click; the route search's target test before the search | UI-GAME-3.4 steps 2 and 5 (Q-203 b, c) |
| E-ui-041 | code reading | the flag menu's update: every exit closes and warps the pointer onto the flag | UI-GAME-3.1 (Q-207 h) |
| E-ui-042 | code reading | the statistics samples: land percent, buildings, goods in warehouses, productivity mean | UI-GAME-8.5a (Q-198 d) |
| E-ui-032 | code reading | (session 2026-11-14 b) | UI-GAME-11.3a (F62): the messages window shown by its constructor and at every game start, the N / button 7 toggle; `SET-settings.md` SET-2 field 17 and SET-5.6: the `!Infowindow` drop-down's values and what reads them |
| E-ui-033 | code reading | (session 2026-11-14 b) | UI-GAME-1.7, 1.8 (F59): the text line's alignment flag (8 centred, 4 plain) read as a size before; the title's line top y 14; the frame pictures of the production, site and military windows; 4.1a the captions' alignment and the two status lines' texts and conditions; 4.3 the headings' alignment and the coin number |
| E-ui-036 | code reading | (session 2026-11-15 night) | UI-GAME-6: the economy dialogs' frame picture `uiDepot_long` (8, 8, 368 x 503); the transport page's background |
| E-ui-037 | code reading and data | (session 2026-11-15 day) | 6.5 (Q-1054): the slider's kinds and their bar, fill and knob pictures, the fill's width and row by value, the disabled tint, the knob's place; the three fill rows of `uiElements2` compared pixel for pixel |
| E-ui-038 | code reading, pictures | (session 2026-11-15 day) | 4.4: the `,` key's window placed at the pointer (the context windows' placement: the pointer minus the window's size, x at most W - 600, then clamped); E-trace-029's pointer positions |
| E-ui-039 | code reading, a picture | (session 2026-11-16 night) | 4.5 (b): the ship window's two separators are `uiElements3` (0, 432, 350 x 8); E-trace-030's ship window pixel equal |
| E-trace-031 | pictures of the original | (session 2026-11-16 night) | the interface catalogue part 4: message kinds (misc, military, chat), the build menu's disabled tooltip, the attack menu with no attackers, the campaign map with a mission won, the forester's dust |
| E-trace-032 | pictures of the original | (session 2026-11-16 day) | 11.4: the geologist, economic and lost-territory message rows; the headquarters window's three pages; the barracks window; the road-building cursor marks |
| E-trace-028 | pictures of the original | (session 2026-11-15 night) | the interface catalogue part 1: the windows and dialogs seen in UI-GAME-2.4, 2.5, 4.1a (f), 4.2, 4.4, 6, 8.1a |
| E-trace-029 | pictures of the original | (session 2026-11-15 night, part 2) | the interface catalogue part 2: the military page's sliders moved, coal mine, bakery, shipyard, catapult and storehouse windows, build / flag / road / deploy-flag menus and their placement: UI-GAME-1.9, 3.3, 3.6, 4.1a (g), 4.4, 8.1 |
| E-trace-030 | pictures of the original, data | (session 2026-11-15 day; pictures kept outside the repository) the interface catalogue part 3: the Seafaring tutorial (harbour expedition page, ship window, expedition landing, messages, main bar tooltips, minimap, build menu tooltips) and free games as Africans and Chinese (headquarters window, build menu); the ship-name texts of the language file | UI-GAME-1.9, 2.2, 2.5, 3.2, 4.4, 4.5 (c), 4.5, 8.1, 11.4 |
| E-trace-026 | pictures of the running original | Jens's 2023 save loaded, 2560 x 1440 (session 2026-11-14 b; pictures kept outside the repository) | UI-GAME-1.8 title, 4.1a (e) woodcutter's hut, 4.3 barracks, 4.4 headquarters, 8.5 the six statistics pages, 11.3a the messages window shown after a load |
| E-trace-020, E-trace-021 | looks | runs of the original 2026-11-08 c (E-trace-020, E-trace-021: the runs' logs and pictures): the statistics page at its first opening, the chat line by button 8 and by RETURN | UI-GAME-8.5f, 12.1 |

## Open questions

- Q-095: the messages window rows and the depot goods grid (positions, the
  second "send out" control) and the transport list rows are only partly
  traced.
- Q-096: layouts of the statistics, inventory, buildings overview, goods
  production and fleet dialogs.
- Q-097: which click ends road building (flag vs. free field) and the
  waterway mode's rules; the minimap's `regions` / `cities` textures.
