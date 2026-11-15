# UI-MENU: Main menu, game menu, settings, save and load, map selection

Status: draft (read from the binary; not checked in the running game)
Owner question(s): plan 6.5 (investigation chunk 2); game setup screens are `UI-game-setup.md`
Game build: 11757

## Summary

The menus are code-built like the in-game dialogs (`UI-ingame.md` UI-GAME-1
applies: pixel layouts, `!` text keys, sprite buttons, `uiClick1` on every
click, `uiDialog0` when a dialog opens). The main menu is a full-screen
parchment with a column of text buttons; the game menu is a small centred
list; settings are rows of radio choices and sliders; save and load share a
list box of files. The player-slot screen of a free game is in
`UI-game-setup.md`.

## Rules

### UI-MENU-0 Start-up (play test 10, F23, F27, F28; OBS-26, OBS-27; E-ui-010)

- **UI-MENU-0.1 Sequence.** At launch the game shows, full screen, the
  films `Ubisoft` (720 x 576, 25 fps, 8.0 s), `BB_Logo` (640 x 480, 30 fps,
  12.7 s), `funaticsAlpha` (1024 x 768, 25 fps, 6.1 s) and `Intro` (1024 x
  576, 25 fps, 134.6 s, with subtitles 0.3) one after the other (files
  `data/video/<name>.bik`), then the **start screen** (0.2). The pointer is
  hidden while a film runs. A key press or a left click during one of the
  three logo films stops it and goes **straight to the intro**; during the
  intro it stops the intro and shows the start screen. The START button
  leads to the main menu (UI-MENU-1). **Project decision (Jens, play test
  10):** s2-reborn shows no company logos: it skips the three logo films
  and leaves the publisher logos off its main menu; the intro and the start
  screen stay. The films are converted with FFmpeg at install; their format
  is not specified here. Confidence: high (E-ui-010;
  `.bik` headers).
- **UI-MENU-0.2 Start screen.** `ui_back` stretched over the screen; a
  1024 x 768 panel centred like the main menu (UI-MENU-1.1) showing
  **`uiLoading0`** (its top-left 1024 x 768: the parchment with the
  picture of a town by a pond); on it the game logo `ui_s2Logo` (256 x 256)
  at (384, 22); the line `!SplashText0` ("THE SETTLERS II - 10th
  ANNIVERSARY") at (210, 520) in `font_big_white`, colour (229, 209, 196);
  the legal text `!SplashText1` in a text box at (210, 540), 640 x 360 px,
  font `dark_system`; the text button `!Start` (tooltip
  `!START_BUTTON_TOOLTIP`, "Starts the game") with its layout origin at
  (432, 615). While the game is still loading its data the START button is
  hidden and `!SplashText2` ("Preparing to start game...") shows at (210,
  630) in `light_system_outlined`; when loading is done the text goes and
  START appears. No company logo is on this screen (`start-screen.jpg`).
  Confidence: high for files, keys and positions (E-ui-010), medium for which of the two (START, the loading line) shows
  when.
- **UI-MENU-0.3 Film display and subtitles.** A film is scaled to fit the
  screen keeping its aspect ratio, centred (black bars where the shapes
  differ). `data/video/Intro.xml` lists subtitles as (frame, text key)
  pairs; at 25 fps frame f is shown at f / 25 s; an empty key clears the
  line. The intro has 13 lines (`Intro_01` .. `Intro_13` of
  `language/<lang>/intro.txt`, each starting with `<hcenter>`, i.e.
  centred), e.g. `Intro_01` from frame 33 (1.32 s) to 403 (16.12 s),
  `Intro_02` 415 to 616, ..., `Intro_13` from 2821 (112.84 s) to the end.
  The line is drawn in `font_big_white` (white), in a box 80 % of the film
  area wide starting at 10 % from its left: if the black bar under the
  film is taller than the text, the text is centred in that bar; if there
  is no bar (a 16:9 film on a 16:9 screen) its bottom sits 16 px above the
  screen's bottom edge; otherwise it is centred on the film's bottom edge.
  The ending film `extro` uses `extro.xml` the same way (3 lines).
  Confidence: high (E-ui-010; XML files).
  The outro film `extro.bik` has the same list in `data/video/extro.xml`: `Extro_01`
  from frame 17 to 292, `Extro_02` 308 to 487, `Extro_03` from 499 to the end (the same
  reader is assumed; the file has the same form). Covers (inventory): *UI text* from
  `language/gb/intro.txt`, `language/gb/extro.txt`.
- **UI-MENU-0.3a Film sound tracks (Q-223; E-gfx-026).** `Intro.bik` and
  `extro.bik` carry four stereo 44.1 kHz audio tracks with ids 0 to 3: track
  0 is the language-free mix (music and effects), tracks 1 to 3 are voice
  tracks, one per language. `data/scripts/menu.cfg` (run at start-up) sets
  the language and maps languages to voice tracks:
  `menu.setLanguage("gb")`, `menu.setVideoVoiceStream("de", 1)`,
  `("gb", 2)`, `("fr", 3)`. When the game plays one of these films it looks
  the current language up in that map (a language not in the map gets track
  2) and plays **track 0 and that voice track together, mixed**; the other
  two tracks are opened but held at volume 0. So the shipped (English, "gb")
  game plays tracks 0 + 2. Both played tracks get the same volume, the
  sound master volume (SND-5, 0 to 255) x 128 in Bink units (32 768 =
  unchanged), applied every frame; with the sound device muted or absent all
  tracks are silent. Films without a voice map entry (the logo films, one
  track or none) play every track they have at that volume. Confidence: high
  (E-gfx-026: the film player opens all four tracks and sets their volumes every
  frame, the voice track comes from the language lookup with default 2, filled by
  `setVideoVoiceStream`; `menu.cfg`); medium that the second of the two places that
  play such a film is the extro.

### UI-MENU-1 Main menu

- **UI-MENU-1.1** Background `ui_back.dds` stretched over the whole screen; a
  1024 x 768 panel centred on the screen (x = W / 2 - 512, y = H / 2 - 384)
  showing **`uiTitle`** (the parchment with the scroll and the publisher
  logos); the logo `ui_s2Logo` (256 x 256) at (677, 17). On a wide screen
  the stretched `ui_back` fills the sides; the panel keeps its 1024 x 768
  pixel size (Q-189 c, re-checked 2026-10-17, E-ui-004).
  Confidence: high.
- **UI-MENU-1.1a Title video (Run D, OBS-19, Q-211 h).** The picture of the
  town in the mountains on the left sheet is the video
  `data/video/title.bik`, drawn at (56, 62) of the panel at its own pixel
  size, **616 x 665**, unscaled (1:1 screen pixels, like the panel). It has
  1308 frames at 25 frames per second (52.3 s), no sound track, and
  **loops** while the main menu is shown. (The earlier "title text at (56,
  62)" was this video; it is not a text and in none of the menu atlases.)
  The page's own buttons and logo are drawn over the panel as usual; the
  video is a plain opaque picture whose painted border blends into the
  parchment. Confidence: high (E-ui-007;
  `title.bik` header).
- **UI-MENU-1.2** Text buttons at (700, 250) of the panel stepping 50 px:
  `!SINGLE PLAYER`, `!MULTIPLAYER`, `!SETTINGS`, `!CREDITS`, then two empty
  rows, `!EXIT` (shown as EXIT GAME). **All five are enabled**: none is
  disabled at any time, so MULTIPLAYER wears the normal red-brown bar
  (blue while hovered) like the others (Run D, E-ui-007). An implementation without network play still draws it as a
  normal button; the orange bar of 1.4 is the pressed look. Confidence:
  high.
- **UI-MENU-1.5 Single player page (Run D, OBS-20, Q-211 a, g).** The scroll
  keeps the logo at (677, 17) and shows, in the same button column (x 700,
  50 px rows from y 250): `!CONTINUE` (250), `!LOAD GAME` (300), an empty row,
  `!START CAMPAIGN` (400, shown as CAMPAIGN), `!TUTORIAL` (450), `!START FREE
  GAME` (500, shown as FREEPLAY GAME), an empty row, `!BACK` (600). The
  button of the page that is open is drawn with the pressed (orange) bar. **The page opens on
  CAMPAIGN** (seen 2026-11-15 night, E-trace-028: SINGLE PLAYER from the main menu shows CAMPAIGN
  pressed and the campaign map of 1.7 on the left sheet); TUTORIAL and FREEPLAY GAME swap the left
  sheet in place. Leaving a tutorial with its Cancel button comes back to this page with TUTORIAL
  open. Confidence: high (pictures).
  **CONTINUE** loads the autosave: the file named by the profile's autosave
  name (`SET-settings.md`, default "Autosave") in the save folder; it is
  enabled exactly when that file exists, and disabled (orange bar) otherwise.
  It does not pick the newest save. Confidence: high for the rows and the
  enable test (E-ui-007), medium for the load.
- **UI-MENU-1.3** The menu plays the main theme (`SND-sound-music.md`
  SND-3.1). Confidence: high.
- **UI-MENU-1.4 Text buttons.** Every menu text button (main menu, single
  player, campaign and game-setup screens, planning and logic menus, end
  screen) is a 208 x 32 px bar from `uiElements0.dds`: normal (8, 184) a
  dark red-brown bar with a thin gold rim, hovered (8, 224) a blue bar,
  pressed or disabled (8, 264) a bright orange bar; the label is drawn
  centred on the bar in `font_big_white` size 16 (capitals only, 1.8),
  raised by 0.003 of the screen height. The highlighted look is shown while
  the pointer is over the button, the pressed look while the mouse button
  is down on it. Smaller text buttons inside dialogs use the same scheme
  with other bars: 152 x 22 (264, 224) / (264, 248), 224 x 19 with a
  drop-down arrow (264, 8) / (264, 32), 152 x 20 (264, 176) / (264, 200),
  136 x 22 (8, 848) / (8, 872), all teal when highlighted, label in
  `light_system` size 16; a check box is 28 x 28 (132, 100) empty /
  (164, 100) ticked with its label 32 px to the right. The pre-rendered
  title buttons of `uiTitleButtons.dds` and the bars of `uiPushButton.dds`
  / `uiButtonDefault.dds` are not used by the shipped build. Confidence:
  high (E-ui-005, `uiElements0.dds`).

- **UI-MENU-1.6 Tutorial page (Run D).** TUTORIAL keeps the single player
  scroll (1.5, TUTORIAL pressed) and draws the left sheet on the panel
  **`uiDesktop1`** (the parchment with the pale town in fog; the free play
  page uses the plain parchment). On it: the strip of four round lesson
  medallions `uiElements0` (936, 520, 80 x 280) at (210, 215), and for
  lesson k = 1..4 a text button `!Lesson k` (the 208 x 32 bar of 1.4) at
  (290, 240 + 70 (k - 1)) with the caption `!Info k` 15 px above it in
  `dark_system`. So each button stands to the right of its medallion
  (medallion k spans y 215 + 70 (k - 1) .. + 70). A lesson button starts
  map `Tutorial0k` (UI-SETUP-2.4, CAM-5). Confidence: high (E-ui-007).

- **UI-MENU-1.7 Campaign page (play test 10, F29, OBS-29; E-ui-010).**
  CAMPAIGN (pressed bar on the scroll) shows the panel **`uicampaign`**
  (its top-left 1024 x 768: the sea map with the islands I to X on the
  left sheet and the empty scroll on the right, on which the single player
  buttons of 1.5 are drawn). Each mission that may be played (CAM-4.2:
  mission 1, and n + 1 once n is won) gets a marker button on its island,
  drawn at the island point minus (22, 75), 64 x 90 px from `uiElements0`:
  a **silver** SPQR eagle (678, 20; hovered 678, 156) on the mission still
  to be won and a **gold** one (798, 20; hovered 798, 156) on missions
  already won (medium which colour is which: read from the order the code
  builds them and from `singleplayer-map.jpg`, where only island I has a
  silver eagle). Island points: I (300, 187), II (465, 187), III (594, 309),
  IV (551, 490), V (455, 589), VI (200, 510), VII (150, 333), VIII (322,
  460), IX (345, 302), X (457, 379). Missions not yet open have no marker
  and cannot be clicked. **Hovering** a marker shows the mission's
  description `campaign<nn>_dsc` (e.g. "During a storm, the settlers are
  shipwrecked ...") in a text box on the framed area at the lower left of
  the map (the stamped box of the picture, about (110, 590) to (390, 695);
  medium); **clicking** it starts the mission (CAM-4.2).
  Confidence: high for picture, points and markers' sprites (E-ui-010).

- **UI-MENU-1.8 Text lines (Q-307, Q-225; E-sim-002).** Every font is an
  AngelCode BMFont text file `language/<lang>/fonts/<name>.fnt` with its page
  `<name>_00.tga` (FMT-install-layout); the game keeps per font the line
  height and base of its `common` line and, per character code 0..255, the
  page rectangle (x, y, width, height), xoffset, yoffset and xadvance. A text
  run is placed from an origin (x, y) in screen pixels:
  (a) **Anchor.** The origin is the **top-left corner of the first line's box**
  (a box lineHeight tall): character c is drawn as a quad at
  (x + xoffset(c) - 0.5, y + yoffset(c) - 0.5) of size width(c) x height(c)
  (the half-pixel shift is the Direct3D texel offset), and the pen advances by
  xadvance(c) (+ the font's extra spacing, 0 in all shipped fonts). Nothing is
  centred on the anchor or hung from a baseline: a label, caption, text box or
  tooltip puts its text's line top at the control's text position. Codes without
  a `char` entry draw nothing and advance nothing.
  (b) **Alignment flags.** A run can carry flags: 8 = x origin moved left by
  half the run's width (centred), 16 = by the whole width (right-aligned),
  1 = y origin moved up by half the line height, 2 = by the whole line
  height. **Corrected 2026-11-14 b (F59, F61; E-ui-033 code reading, E-trace-026 pictures of the
  original):** a label (a single text line: dialog titles, headings, captions, status lines, counts,
  the drop-downs' and buttons' texts) is drawn with **flag 1 always added** to its alignment (4, 8 or
  16): its line box is **centred vertically on y**, i.e. the line top is y minus half the font's line
  height (`small_no_alias_white` 12 -> 6 px, `light_system` and `font_big_white` 16 -> 8 px). Measured:
  the building window's captions with y 37 have their ink at y 34 .. 40, just above the slots at 43;
  the title with y 14 has its ink at y 8 .. 19 inside the 24 px header bar. (The earlier sentence
  "the label draw never moves the text vertically" was wrong.) Multi-line text boxes were not checked.
  (c) **Wrapping.** With a maximum width, a line breaks at the last space,
  '.', '!' or newline when the pen would pass the width; the next line starts
  lineHeight lower (the text's height grows by lineHeight per line).
  (d) **Mixed fonts.** A text box of several runs in different fonts aligns
  them on the baseline: a run's y origin is the line's y plus (base of the
  line's font - base of the run's font).
  Confidence: high (code); the half-pixel shift and the per-line height are
  read from the quad builder and not photo-checked.

- **UI-MENU-1.9 The menu dialogs' frames (2026-11-14, Q-185 c; E-ui-031, code reading).** Every
  menu dialog is the standard dialog (title, close button, `UI-ingame.md` UI-GAME-1.2) made with a
  frame picture of its own, drawn 1:1 and centred on the screen, the dialog taking the picture's
  size:
  - **Select Map** (2.1), **Save Game** (5.1), **Load Game** (5.2, 6.1) and the **game menu** (3.1b):
    `uiBigDialog` (8, 8), 376 x 464, the frame of 3.1b (its visible panel 364 x 448);
  - the **game settings** (4): `uiDepot` (8, 8), 376 x 464, the tabbed frame the build menu also
    uses;
  - the in-game flag and road menus, for comparison: `uiSmallDialog` (8, 8), 304 x 96.
  The 512 x 512 `uiBigDialog` drawn whole is only the fallback of a dialog made without a picture.
  `uiXXLDialog` is drawn by none of them (UI-GAME-1.2a). The buttons of each dialog are those of
  its own rule (2.1, 3.1, 4, 5.1, 5.2, 6.1). Confidence: high for Select Map, Save, Load, the game
  menu and the game settings (each dialog's own set-up read); the sound settings and the other
  menu pages were not checked.

### UI-MENU-2 Game setup

- **UI-MENU-2.1 Select map** (`!Select Map`): a list box at (50, 50) with 11
  visible rows, filled from the map folder (single player maps; folder 3 for
  campaign maps when allowed; saved games under folder 20 for "load");
  below it `!Map Size:` and `!Players:` with their values 5 px to the right
  (font size 16, colour (200, 160, 111)) at (120, 325) stepping one row; a
  map preview at (195, 315) framed by `uiElements0` (520, 104, 104 x 104)
  offset (-3, -3). **Behaviour (Q-227 g; E-sim-002):**
  the dialog has no OK or Back button and no scroll arrows of its own (the
  list box scrolls itself); it closes by its title-bar close button or by a
  choice. The list is filled from the maps folder (0), the campaign folder
  (3) and, when allowed, the saved games folder (20), the entries from the
  file list (`*.xml`); a row's text is coloured by the map's validity. While
  the pointer is over a row, that map's header is read: `!Map Size:` shows
  `W x H` and `!Players:` the player count, and the preview shows the map's
  picture (98 x 98, framed); over no row, or over an unreadable map, both
  values are blank and the preview is cleared. A click on a row takes the
  map: if its header reads, the dialog closes and the free play page gets
  the file, its size and its player count; otherwise a message box (dialog
  38) titled `!INVALID_MAP` with the text `!INVALID_FILE` is shown and the
  dialog stays. Confidence: high for layout and behaviour, medium for the
  folder ids and the list's entry filter.
  **The list box and a rule (2026-11-12 night, Q-256 a; E-ui-023, code reading).** The list box is
  the plain style (style 0): its rows are 21 px apart (row n's text at y = 21 n inside the box),
  each row on a 252 x 18 bar of that style, and it carries two controls of its own, placed at
  (0, -19), just above the first row, and at (0, 231), right under the eleventh row (11 x 21);
  these are its scroll controls (medium: their art is set by the list box class, not read). So
  the "scrolls itself" above means these two, not arrows beside the rows. Between the list and the
  `!Map Size:` / `!Players:` block the dialog draws a horizontal rule: `uiElements3` (0, 432),
  336 x 8, at (12, 300).
  **The scroll bar (2026-11-13, Q-209 e; E-ui-024, code reading; corrects the places above).**
  The two controls are the arrow buttons of the list box's **vertical scroll bar**, and their
  places are relative to that bar, not to the list. The plain-style list carries the bar at
  (252, 18) from its corner, right of the 252-wide row bars: a track `uiElements0` (456, 197),
  24 x 194, with a 24 x 24 thumb `uiElements0` (8, 104) that slides along it with the scroll
  position. The up button sits at (0, -19) from the bar, so (252, -1) in the list, the down
  button at (0, 194), the bar's foot, so (252, 212) in the list. Both buttons are 24 x 24 from
  `uiElements0`: up (8, 128), down (104, 128), each with two further looks 32 and 64 px to the
  right (highlighted and pressed: (40, 128), (72, 128); (136, 128), (168, 128)). (The same
  button class gives the messages window its third button, `!Jump to last one.`, at (104, 328).)
  Confidence: high for the art, the bar's place and the buttons' offsets from the bar; medium for
  which of the two further looks is highlighted and which pressed. Seen (E-ui-025): the Save Game
  dialog's list (the same list box, 5.1) shows the bar right of its rows, an arrow button above
  and one below, the thumb on the track.
  **How the bar works (2026-11-14, Q-1042; E-ui-029, code reading).** The bar holds one value
  v from 0 to 1 (kept in that range after every change); the list follows v, not the other way
  round. With N entries and V = 11 rows shown:
  - **The list's first row** is s = floor(v (N - V) + 0.5). When everything fits (N <= V) the
    list sets v = 0.5 and s = 0 in every update.
  - **The thumb's place:** its top is 12 + v (194 - 24) - 12 = **170 v px** below the bar's top,
    not rounded (so 0 at v = 0, 170 at v = 1, and **85, the middle of the track, when everything
    fits**). The thumb is drawn in every case. Its look follows its state: (8, 104) normal,
    (40, 104) the second state, (72, 104) the others (24 x 24 each).
  - **An arrow click** changes v by one **step = 1 / floor(N / 2)** (up: minus, down: plus), the
    same for every list length, so it is not one row: with the 20 shipped maps a step is 0.1 and
    shows s = round(0.9 k) after k clicks. **Holding** the left button on an arrow steps at once
    and then again in the first update in which a countdown of 0.05 s, lowered by each update's
    frame duration, is no longer above 0 (about 20 steps a second). The up arrow is disabled
    while v = 0, the down arrow while v = 1, and both while everything fits; a disabled arrow takes
    no clicks (its look when disabled was not traced).
  - **The track:** while the bar is pressed (anywhere on its 24 x 194, thumb included), each
    update moves v three quarters of the way to the pointer: v <- v + 0.75 (p - v), with p the
    pointer's height below the bar's top divided by 194. So a click on the track moves the
    thumb toward the pointer (no paging), and dragging follows the pointer.
  - **The Save Game dialog** (UI-MENU-5.1) makes the same list box with the same style (11
    rows at (50, 50), rows 21 px apart): its bar, arrows and thumb are at the same places as
    here, and it behaves the same; only its rows are not translated.
  Confidence: high for the formulas, the step, the disabling and the 0.05 s repeat (code);
  medium for which pointer states count as pressed (the generic button and widget states were
  not traced) and for the mouse wheel and keys, which the bar also reads with a step of its own
  that was not traced. Tests T18, T19.
  **The order of the maps (2026-11-14, Q-211 f; E-ui-030, code reading).** The dialog lists every
  map file of its folders (two folders, a third one in a network game) in one list, each by its **file name cut at the first `.`**, and keeps the list sorted after every
  entry: names compared after turning `a`..`z` into `A`..`Z`, byte by byte, shorter first when one
  is the start of the other. So `_` (after `Z`) sorts after every letter: `AB` before `A_B`. The
  shipped 20 come out as MP_2P_Storm_Coast, MP_2P_Summer_Breeze, MP_4P_black_hole,
  MP_4P_Close_Frontier, MP_4P_crossroads, MP_4P_DeathValley, MP_4P_Fight, MP_4P_Fight2,
  MP_6P_Paradise_Island, MP_6P_the_others, then sp01_big_fish .. sp11_southshore_attack in their
  number order (the order read live, E-ui-023). Confidence: high (code; agrees with the live read).
  Test T20.
  Class rule (2026-10-24): the preview of every shipped map is its `.bmp` beside the
  `.s2m` (20 free-game, 10 campaign maps). Covers (inventory): *texture* from
  `data/maps/*/*.bmp`. Covers (inventory): *free game map* from
  `maps/freeGameMaps/*.bmp`. Covers (inventory): *campaign mission* from
  `maps/campaignMaps/*.bmp`.
- **UI-MENU-2.2** The player slots, tribes, teams and colours: `UI-game-setup.md`.

### UI-MENU-3 Game menu (in game)

- **UI-MENU-3.1** (`!Game Menu`), centred, opened by the HUD options button
  or Escape. Logo `ui_s2Logo` (256 x 256) at (-10, -80) of the list origin
  (70, 60); items stepping 32 px: `!Save Game`, `!Load Game`, a separator
  (`uiElements3` (0, 432, 336 x 8) at (12, +10)), `!Settings`, a separator,
  `!Exit to Title`, `!Exit to Desktop`. Confidence: high.
  **Measured (2026-11-13, E-ui-025, a picture of the original at 2560 x 1440, the menu opened with
  Escape in a running game):** the frame (3.1b) at its centred place, top-left (1092, 488); the
  five rows' 208 x 32 bars with their left edge at frame x 88 and their tops at about frame y 198
  (`!Save Game`), 230 (`!Load Game`), 294 (`!Settings`), 358 (`!Exit to Title`, shown as MAIN MENU)
  and 390 (`!Exit to Desktop`) (+- 3 px): 32 px per row and one more row's height at each
  separator. So the list origin (70, 60) above is not measured from the frame's corner (a click at
  frame (174, 76) lands on the logo); positions here hold for drawing and clicking. Confidence: high
  for the measured places (one picture, the bars' colour edges), medium for the +- 3 px.
- **UI-MENU-3.1a (Run D)** There is no CONTINUE or RESUME row: the menu is
  closed with the dialog's close button or Escape. The seven rows of 3.1
  are all it has (E-ui-007).
- **UI-MENU-3.1b Game menu frame (Q-301 a, E-gfx-030).** The game menu does
  not use either default dialog frame (small 288 x 88 or big 368 x 458 art).
  It supplies its own frame picture: the rectangle x 8, y 8, 376 x 464 px of
  `uiBigDialog`, drawn 1:1 as a 376 x 464 px frame centred on the screen. The
  texture's opaque art covers x 8..371, y 8..455 of that rectangle, so the
  visible panel is 364 x 448 px from the frame's top-left corner, with 12 px
  of transparent texture on the right and 16 px at the bottom (the art's top
  4 rows, y 4..7 of the texture, are cut off). Title and close button sit
  as on every dialog, measured from the 376 px frame width: title centred at
  (188, 14), close button at (336, 5). Confidence: high for the rectangle
  (the menu's constructor passes it to the dialog base; the frame takes the picture's width and height);
  medium for the title and close positions (they use the frame width, not
  re-read for this dialog).
- **UI-MENU-3.2** `!Save Game` is disabled in a multiplayer game on a client
  that is not the host. Confidence: medium.
- **UI-MENU-3.3 Leaving (answered 2026-11-04, Q-298 e; E-ui-016).** `!Exit to Title` and `!Exit to Desktop` close the game menu and open a dialog titled
  `!Request!`: the text `!Exit to title?` or `!Exit to desktop?` from (80, 70); the buttons `!Yes` at
  (130, 110) and `!No` at (130, 144), both 5 px higher while a game runs; and, only while a game runs,
  a check button `!Autosave...` at (130, 175). Its state is setting 22 (`SET-settings.md` SET-2),
  off by default, and a click writes it straight back. On `!Yes`, if a game runs and the box is on,
  the autosave is written (FMT-SAV-5.3), and the game is left. In a network game, on a client that is not
  the host, the box is off and disabled. **The autosave interval plays no part.** Confidence: high (code).
- **UI-MENU-3.3a The leave dialog's look and its other ways in (added 2026-11-05, Q-1013;
  E-ui-017).**
  (a) **Frame:** the dialog is 480 x 224 in the 1024 x 768 space, drawn with `uiMediumDialog`
  (8, 8, 480 x 224) (the same picture-frame way as the game menu's `uiBigDialog`), centred on the
  screen, with the usual title bar text `!Request!` and close button of every dialog
  (`UI-ingame.md` UI-GAME-1.8). The positions of 3.3 are in this frame.
  (b) **The `!Autosave...` box** is the check box of 1.4: 28 x 28, `uiElements0` (132, 100) when
  off and (164, 100) with the tick when on, its label in `light_system` 32 px right of the box and
  4 px lower.
  (c) **Without a game:** the main menu's `!EXIT` (1.2) does not end at once: it opens this dialog
  in its desktop mode (`!Exit to desktop?`), with `!Yes` at (130, 110) and `!No` at (130, 144) and
  no box; `!Yes` ends the program, `!No` closes the dialog and the main menu stays. A click on
  `!EXIT` while the dialog is open closes it. (When opening it, any other open dialog is closed
  first.)
  (d) **`!No`, the close button:** close only this dialog. While a game runs the pointer is first
  put back on the field it was over (`UI-ingame.md` UI-GAME-1.10a) and the game goes on: the game
  menu, closed when the dialog opened, is not reopened.
  Confidence: high (code), medium for the label's 4 px.
  (e) **Seen (2026-11-15 night, E-trace-028: MAIN MENU in campaign01's game menu, 2560 x 1440):** the
  frame at (1040, 608) = centred 480 x 224; title CONFIRM; the question (`Do you want to exit the game
  and return to the Main Menu?`) is **centred on the frame's middle** (x 240), on y 70, not
  left-aligned at x 80; YES and NO bars 208 x 32 at (130, 105) and (130, 139) (5 px higher in a game,
  as said); the `Autosave...` box at about (135, 175) with its label to the right. Confidence: high.

### UI-MENU-4 Settings

- **UI-MENU-4.0 The settings window (2026-11-14 b, Q-1047; E-ui-034, code reading).** The six
  settings pages (Graphics, Engine, Sound, Input, Game, Network; `SET-settings.md` SET-5.1) are six
  dialogs built on one tabbed base, **each with the frame picture `uiDepot` (8, 8), 376 x 464**
  (UI-MENU-1.9), centred. The base gives every page: the standard title and close button
  (`UI-ingame.md` UI-GAME-1.8); the **page switch**, a one-row grid of six icon tabs at (15, 25)
  with 40 x 50 cells (tooltips `!Graphics settings`, `!Engine settings`, `!Sound settings`, `!Input
  settings`, `!Game settings`, `!Network settings`; a click opens that page in place of this one);
  and the wide text button `!Reset` at (168, 408). **There is no BACK button:** the window closes
  with its close button (or Escape). Each page's controls start at (168, 72) and step 32 px down;
  a control's caption is a label 144 px left of it (x 24). Confidence: high for the frame, the tabs,
  `!Reset` and the absence of BACK; medium for a slider's drawn width (its bar art is 152 x 24).
  **The tabs' icons:** the icon tab ids (`UI-ingame.md` UI-GAME-3.5's tab row) are Graphics 0, Engine
  1, Sound 2, Input 3, Game 4, Network 23. **`!Reset`** acts on its own page only, setting fixed
  values, and the page is rebuilt to show them (the file is written when the page is left, SET-1.2):
  Game: autosave interval 600 s (10 minutes), field 17 = 1 (`!Infowindow` Off, SET-2), mouse scrolling
  on, the autosave-on-leaving box off (`SET-settings.md` SET-2 fields 4, 17, 5, 22; also field 3 =
  1.0); `!DayNightCycle` is not touched. Input: inverted off, scroll speed 0.25, free camera on, smart cursor on, field 21
  off (fields 6 .. 9, 21); mouse scrolling not touched. Sound: the four volumes back to the sound
  device's defaults (SND-5.1). The button is **not shown** on the Graphics and Engine pages (seen in
  the pictures of E-trace-026), disabled on the Sound page together with the sliders when there is no
  sound device; on the Network page it is shown and does nothing. Confidence: high for the tab ids, the per-page action
  and the disabling; medium for which profile field each of the game page's last two writes is (read
  by offset) and for the sound defaults' values (the device's reset not read).
- **UI-MENU-4.1 Game settings** (`!Game Settings`): rows at (168, 72)
  stepping 32 px, each a caption and a **drop-down** (the choice list of `UI-ingame.md`
  UI-GAME-8.5g: one 152 x 20 text button showing the chosen value, the list of all choices opens on
  a click; corrected 2026-11-14 b, Q-1047, E-ui-034: not radio choices), the caption 144 px left
  of the row: `!AUTOSAVE` at (168, 72), `!Infowindow` at (168, 104), `!DayNightCycle` at (168, 136):
  - `!AUTOSAVE`: Off, `!1 Minute` (60 s), `!5 Minute` (300), `!10 Minutes`
    (600), `!20 Minutes` (1200), `!45 Minutes` (2700), `!1 Hour` (3600);
    the current choice is the profile's autosave interval in seconds;
  - `!Infowindow` (shown as INFORMATION WINDOW): On / Off (the help panel; `SET-settings.md` SET-5.6);
  - `!DayNightCycle`: `!On` / `!Off` (GFX-VIEW daytimes).
  Confidence: high.
- **UI-MENU-4.2 Sound settings** (frame `uiDepot` 376 x 464, 4.0): wide sliders at (168, 72) stepping 32 px
  (each slider's bar 152 x 24, medium, 4.0):
  `!MASTER VOLUME`, `!MUSIC VOLUME`, `!UI VOLUME`, `!SOUND VOLUME`, values
  0..1 from the sound manager; all four disabled when sound is unavailable
  (SND-5). Confidence: high.
- **UI-MENU-4.3** Graphics, Input and Network pages: `SET-settings.md` SET-5.2, SET-5.5 and SET-5.7
  (2026-11-15, E-ui-035): their controls, choices, profile fields and places, on the frame of 4.0.

### UI-MENU-5 Save and load

- **UI-MENU-5.1 Save** (`!Save Game`): list box at (50, 50), 11 rows, of the
  saved games (folder 10; 20 in a multiplayer game); a name input at
  (50, 316) prefilled with the selected file's name; `!Save` (148, 344) sends
  `save-game`(name) and shows `!Saving...` at (50, 50) while the game is
  written; `!Delete` (148, 409) asks `!Remove this file?` at (80, 319) with
  `Yes` (148, 339) and `No` (148, 364); separators at (12, 290) and
  (12, 390). With less than 5 MB free on the save drive, `!Disk Full!` is
  shown and `!Save` and `!Delete` are disabled. Confidence: high.
  **`!Disk Full!` in detail (answered 2026-11-06, Q-1019; E-ui-019):** (a) **where:** it is not a text of its own: it **replaces the caption of the
  `!Save` button** (148, 344), which is then disabled with `!Delete`, so the button reads
  `!Disk Full!` (translated like any caption) in its disabled look; (b) **5 MB** is 5 x 2^20 =
  5,242,880 bytes, and the test is strict (full when fewer bytes are free); (c) **when and which:**
  once, while the dialog is built (each opening), from the operating system's bytes free **to the
  calling user** on the volume of the saves path (the path with its last four characters cut off
  is what is asked; not re-read while the dialog stays open); (d) **text:** the key goes through
  the language table like every caption; no shipped language file has it, so what shows is what
  the table gives for a missing key (`UI-ingame.md` / `UI-menus.md` rule for missing keys, as for
  `!INVALID_MAP`). Confidence: high for (a) to (c) (code); (d) follows the general missing-key
  rule, not checked in a picture.
  **Behaviour (answered 2026-11-04, Q-298, E-ui-016):**
  - **Opening.** The game menu closes and this dialog opens. The input holds the default name
    (`DD-MM-YYYY HH-MM-SS <map name>`) with the keys on it. `!Delete` starts disabled.
  - **Saving.** `!Save`, or RETURN while the input has the keys, hides every control and shows
    `!Saving...`; the save happens in the next update, then **the dialog closes and the game goes on**:
    the game menu does not come back. **There is no question before replacing a save:** a name
    that is already in the list is written over at once. `!Save` is enabled only while the input
    is not empty.
  - **The name** uses the **restricted character mode** of 5.3 while this dialog is open, so the
    characters a file name cannot hold can never be typed. It sets no length limit of its own.
  - **The list** shows the folder's files without extension, **in the order the file system returns
    them, not sorted**: on NTFS that is by name, ignoring case, so the default names sort by
    day of the month first, not by date. The list box scrolls itself, as in 2.1. A click on a row
    copies its name into the input and enables `!Delete`; `!Delete` is disabled again as soon as the
    input's text differs from that row's name.
  - **Deleting.** `!Delete` hides `!Save`, `!Delete` and the input and shows the question; `Yes` deletes
    the file, rebuilds the list and puts the default name back; `No` goes back.
  Confidence: high (code), except that the NTFS order is the file system's, not read here.
- **UI-MENU-5.2 Load** (`!Load Game`): the same list; `!Load` starts the
  selected game. Confidence: medium (its code was not read).

- **UI-MENU-5.3 Text input (Q-228 a; E-sim-002).** A text input is
  a bar drawn from `uiElements0` by style and state (normal / hovered /
  other): style 0 (256 x 20): (8, 8), (8, 32), (8, 80); style 1 (224 x 19):
  (264, 128) / (264, 152) (18 px tall); style 2 and 4 (152 x 19 / 20):
  (266, 296) / (266, 320) (152 x 18), style 4 while being edited (8, 80,
  152 x 18); style 3 (490 x 20, no bar art); style 5 (704 x 20): (159, 799),
  (159, 831), (159, 863). Its text is `light_system` at (10, 1) (style 0, 4),
  (0, 1) (1, 2), (0, 0) (3), (5, 1) (5) from the bar's top-left, line top at
  the anchor (1.8). Editing: a click on the input focuses it (plays
  `uiClick0`); a click inside existing text puts the caret at the character
  whose left edge is nearest the click (the first prefix wider than the
  click's x minus 3 px). The text is kept as the part left of the caret and
  the part right of it, and is shown as left + caret + right while focused,
  the caret alternating between `|` and `.` on every other draw. Keys while
  an input is focused and no modal dialog holds the keys: a printable
  character (code 32 and above) is inserted at the caret when the whole text
  is shorter than the input's limit (a per-input count, unset = no limit; the
  network page's call sign input sets 15); in the restricted mode only
  space, digits, A .. Z, a .. z and `äÄöÖüÜß!&()+-_çâáàôéèê` pass. LEFT /
  RIGHT move the caret by one character, HOME / END to the ends, DELETE
  removes the character right of the caret, BACKSPACE the one left of it, ESC
  clears the result. `uiKey0` plays for an inserted character, DELETE and
  BACKSPACE (only when something was removed); a key refused at the limit
  plays nothing. RETURN and the closing of the dialog give the text back
  (not traced: medium). Confidence: high for the art, keys, caret and sound;
  medium for which inputs use the restricted mode and for RETURN.

### UI-MENU-6 Answers to Q-096 (2026-10-17)

- **UI-MENU-6.1 Load dialog layout (UI-MENU-5.2).** Title "Load Game"; an 11-row
  file list at (50, 50); "Select game state to load." at (40, 316); "Load" at
  (148, 344), "Delete" at (148, 409); the delete confirmation "Remove this file?"
  at (80, 319) with Yes (148, 339) / No (148, 364); separator frames `uiElements3`
  (0, 432, 336 x 8) at (12, 290) and (12, 390). The list holds every `*.sav` of
  the saves folder without extension (FMT-SAV-1.3). Confidence: high.
- **UI-MENU-6.2 Settings pages (UI-MENU-4.3).** See SET-5 for the six pages, their
  controls and the stored fields.
- **UI-MENU-6.3** The credits screen (`credits.txt`): see 6.3a (2026-10-24; before that
  the layout was not extracted, Q-096).
- **UI-MENU-6.3a The credits screen (2026-10-24, E-sim-007).** Positions are in a 1024 x 768 panel centred on the
  screen. The screen shows `ui_back` (its 512 x 512 rectangle) stretched over the whole
  screen, the credits background picture (1024 x 768) over the panel, a 256 x 256 picture
  at (677, 17), and the text button `!BACK` at (700, 550); the music is the credits
  music (SND-3.1). The text: for n = 0 to 999 the key `cr` + n in three digits
  (`cr000`, `cr001`, ...) is looked up, and every key that exists (UI-GAME-1.3; the
  shipped `credits.txt` has `cr000` .. `cr247`) becomes one line, in number order. Each
  line is a label at x = 100 of the list (the list at (270, 120)); the first line's y is
  639 + its spacing and each next line is placed its own spacing below the previous one:
  a line starting with `%` is drawn without the `%` in `dark_system`, spacing 35; a line
  starting with `#` in `font_big_white`, spacing 45 (the `#` handled by a text function
  not read; it is assumed to be dropped as well); any other line in
  `light_system_outlined`, spacing 21. Every frame each line moves up by frame time
  (seconds) x 0.05 of the screen height; its colour is white with an alpha of 1, falling
  linearly to 0 over the 40 px below the top edge y = 68 and over the 40 px above the
  bottom edge y = 700; a line that has passed above y = 68 is done; lines below the
  bottom edge are not drawn. What happens after the last line is not read. Confidence:
  high for the keys, their order, the fonts and spacings, the speed and the fades;
  medium for the `#` handling, for the scroll unit being the screen height and for the
  two pictures (the 256 x 256 one not identified). Covers (inventory): *UI text* from
  `language/gb/credits.txt`.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| main_panel | 1024 x 768 centred | px | high | E-ui-003g |
| main_buttons | (700, 250), step 50 | px | high | E-ui-003g |
| text_button_bar | uiElements0 208 x 32: normal (8, 184), hover (8, 224), pressed (8, 264); font_big_white 16 | px | high | E-ui-005 |
| game_menu_list | (70, 60), step 32 | px | high | E-ui-003g |
| settings_rows | (168, 72), step 32 | px | high | E-ui-003g |
| autosave_choices | 0, 60, 300, 600, 1200, 2700, 3600 | s | high | E-ui-003g |
| save_list_rows | 11 | rows | high | E-ui-003g |
| disk_full_limit | 5 | MB | high | E-ui-003g |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | 1280 x 1024 screen | show the main menu | panel at (128, 128); `!EXIT` at panel (700, 550) |
| T2 | in game, host | open the game menu | seven rows; `!Save Game` enabled |
| T3 | multiplayer client | open the game menu | `!Save Game` disabled |
| T4 | settings, autosave 10 minutes | open game settings | the `!10 Minutes` choice is selected |
| T5 | save dialog, 3 MB free | open | `!Disk Full!` shown; `!Save` disabled |
| T6 | save dialog, name "test" | `!Save` | `save-game`("test"); `!Saving...` shown |
| T8 | main menu shown for 60 s | left sheet | `title.bik` 616 x 665 at panel (56, 62), frame 0 again at 52.32 s (loops) |
| T9 | main menu, no network support | MULTIPLAYER | normal red-brown bar, blue on hover (enabled) |
| T10 | save folder without `Autosave.sav` / with it | single player page | CONTINUE disabled (orange) / enabled; pressing it loads `Autosave` |
| T11 | single player page, TUTORIAL | left sheet | `uiDesktop1`; medallions at (210, 215); `!Lesson 1` .. `!Lesson 4` at (290, 240 / 310 / 380 / 450), `!Info k` captions at y - 15 |
| T7 | main menu, pointer over `!SETTINGS` | hover, then press | the bar changes from red-brown (8, 184) to blue (8, 224), then orange (8, 264) while the button is held |
| T12 | launch, no input | screens | Ubisoft 8.0 s, BB_Logo 12.7 s, funaticsAlpha 6.1 s, Intro 134.6 s, then the start screen with START (0.1) |
| T13 | launch, click during BB_Logo | screens | BB_Logo stops, the intro starts at once (funaticsAlpha is skipped); a click during the intro shows the start screen |
| T14 | intro on a 1920 x 1080 screen | frame 33 | film fills the screen; "There was once a time, ..." centred, bottom 16 px above the screen bottom, white; gone at frame 403 (16.12 s) |
| T15 | intro on a 1024 x 768 screen | subtitle | film 1024 x 576 centred with 96 px bars; the line centred in the lower bar |
| T16 | campaign page, no mission won | markers | one silver eagle at (278, 112) (island I, 300 - 22, 187 - 75); hover shows campaign01_dsc in the box at the lower left |
| T17 | 3.3a: the main menu, no game | click `!EXIT`; then `!No`; then `!EXIT` and `!Yes` | the 480 x 224 `uiMediumDialog` dialog `!Request!` with `!Exit to desktop?`, `!Yes` (130, 110), `!No` (130, 144), no `!Autosave...` box; `!No` closes it and the main menu stays; `!Yes` ends the program |
| T18 | 2.1 (Q-1042): Select Map with the 20 shipped maps, at the top | the thumb, then 1, 5, 6 and 10 clicks on the down arrow | thumb top 0, up arrow disabled; after 1 click v = 0.1, first row 1, thumb top 17; after 5: v = 0.5, row 5 (floor(4.5 + 0.5)), top 85; after 6: v = 0.6, row 5 still, top 102; after 10: v = 1, row 9, top 170, down arrow disabled (v summed in float32 steps of 0.1) |
| T19 | 2.1: a list with 8 entries (everything fits) | the thumb and the arrows | first row 0, thumb top 85 (the track's middle), both arrows disabled |
| T20 | 2.1 map order (Q-211 f): map files `AB.s2m`, `A_B.s2m`, `ab2.s2m`, `Zeta.s2m` | the list | AB, ab2, A_B, Zeta (upper-cased: AB, AB2, A_B, ZETA; `_` after the letters and digits) |
| T16a | shipped `menu.cfg` (language "gb"), intro plays, master volume 255 | audio | tracks 0 and 2 at Bink volume 32 640 each, mixed; tracks 1 and 3 at 0. With `setLanguage("de")`: tracks 0 and 1. With a language "it" not in the map: tracks 0 and 2. Master volume 0: silence (0.3a) |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ui-023 | code reading | (session 2026-11-12 night) the Select Map dialog's constructor (the list box's style, rows, its two child controls' places, the rule and the preview frame) and the list box base constructor (row spacing and bar by style) | UI-MENU-2.1 |
| E-ui-024 | code reading | (session 2026-11-13) the list box's creation (its scroll bar by style and the two arrow buttons placed on the bar), the slider class by style (track, thumb and their sizes), the arrow button class (sprite by kind, two further looks) | UI-MENU-2.1 |
| E-ui-025 | a picture of the original (kept outside the repository) | (session 2026-11-13) the game menu opened with Escape in a running free game, 2560 x 1440, the button bars' colour edges measured; the Save Game click at the measured place opened the save dialog | UI-MENU-3.1 |
| E-ui-029 | code reading | (session 2026-11-14) | 2.1 (Q-1042): the list box's bar value, the first row, the thumb's place, the arrow step and repeat, the arrows' disabling, the track press; the Save Game dialog's list style |
| E-ui-030 | code reading | (session 2026-11-14) | 2.1: the map list's entries (file names cut at the first dot) and its sort (upper-cased byte order, after every entry) |
| E-ui-031 | code reading | (session 2026-11-14) | 1.9 (Q-185 c): the standard dialog's frame (own picture, or the 512 x 512 fallback), the frame pictures of Select Map, Save, Load, the game menu, the game settings' tabbed base and the flag / road menus |
| E-ui-034 | code reading | (session 2026-11-14 b) | 4.0, 4.1, 4.2 (Q-1047): the six settings pages on one tabbed base (frame, tabs, `!Reset`, no BACK), the game page's drop-downs and rows, the slider's bar art |
| E-trace-026 | pictures of the running original | the settings pages and the windows at 2560 x 1440 (session 2026-11-14 b) | 1.8 (b) vertical centring, 4.0 `!Reset` not shown on Graphics and Engine |
| E-gfx-026 | code reading, data files | `data/scripts/menu.cfg`; `.bik` track headers | film sound tracks (0.3a, Q-223) |
| E-ui-010 | code reading, data files | `data/video/*.bik` headers, `Intro.xml`, `extro.xml`, `uicampaign.dds`, `uiLoading0.dds` | start-up, start screen, subtitles, campaign page |
| E-ui-003g | code reading | | layouts and actions |
| E-ui-003 | code reading | | sound settings page |
| E-ui-005 | code reading | | text button styles and state art |
| E-ui-007 | code reading | | menus, Run D |
| E-ui-016 | code reading | | Q-298 |
| E-ui-017 | code reading | | UI-MENU-3.3a, T17 |
| E-sim-002 | code reading | | text line anchor, alignment flags, wrapping, baseline alignment (UI-MENU-1.8); text input (5.3) |
| E-ui-019 | code reading | (= E-gfx-039, session 2026-11-06) | UI-MENU-5.1 |

## Open questions

- Q-096 (graphics and control settings pages, load dialog, credits).
