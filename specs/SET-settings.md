# SET: Settings stored between runs

Status: draft (file, format, dialog bindings and defaults verified against the binary and one
original settings file; the meaning of a few fields is open)
Owner question(s): plan 7.3; Q-098 (where the volumes are saved)
Game build: 11757

## Summary

The game keeps one settings file per profile in the user's documents folder. It holds the profile
name, the autosave settings, the input options, the per-map campaign state, the five volumes, the
player name and the graphics (scene) settings. It is written whenever a settings page is left and
read at start-up; a missing file means defaults.

## Rules

### SET-1 File

- **SET-1.1** Path `<Documents>\S2\users\<profile>.cfg`. The profile name defaults to `default`
  (observed file `default01.cfg`, profile `default01`). The file is plain, not rc00-wrapped.
  **Which profile at start (2026-11-16 day, Q-188 a; E-set-003, code reading, high):** the
  game has no profile choice. At start-up, once, it makes its settings object with the fixed
  name `default01` and reads `<Documents>\S2\users\default01.cfg`; nothing renames it later, so
  it always reads and writes that one file, whatever other `.cfg` files the folder holds. The
  player name inside the file (the profile name shown in the game, `default` when the file has
  none) is a separate field.
- **SET-1.2** Written when a settings page is closed; read when the settings object is created
  (start-up). If the file is missing or unreadable the defaults of SET-3 apply.
- **SET-1.3** The file is two objects in the marker format of FMT-S2M-1.2: `UserObject` (version 5)
  followed by `SceneSettings` (version 1). No padding.

### SET-2 `UserObject` (version 5), in order

| # | Type | Meaning | Observed |
|---|---|---|---|
| 1 | string | profile name | `default01` |
| 2 | string | autosave file name | `Autosave` |
| 3 | f32 | unused (forced to 1.0 on load) | 1.0 |
| 4 | u32 | autosave interval, seconds of game time, 0 = off (FMT-SAV-5.2) | 600 |
| 5 | bool (v >= 4) | input option A: on no settings page (the Input page does not show it, SET-5.5; 2026-11-15, E-ui-035) | true |
| 6 | bool | mouse scrolling inverted: the Input page's `!MOUSE SCROLLING` shows Inverted while true (SET-5.5) | false |
| 7 | f32 | scroll speed (medium) | 0.25 |
| 8 | bool | border scrolling on: the Input page's `!BORDER SCROLLING` shows Enabled while 1 (SET-5.5; was read as free camera) | true |
| 9 | bool | smart cursor on: the Input page's `!SMART CURSOR` shows Enabled while 1 (SET-5.5) | true |
| 10 | list | map states: `u32 n`, then n x `UserMapState` (v0): string map name, bool completed, u32 acknowledged (CAM-3.5) | `MP_2P_Storm_Coast` false 2; an empty name, false 2 |
| 11 | f32 | master volume (SND-5.1) | 1.0 |
| 12 | f32 | music volume | 0.3 |
| 13 | f32 | UI volume | 1.0 |
| 14 | f32 | sound volume | 1.0 |
| 15 | f32 | ambient (fifth, not exposed) volume | 1.0 |
| 16 | string (v >= 1) | player name shown in game and in the save header (FMT-SAV-2.1) | `default` |
| 17 | u32 | `!Infowindow` (shown as INFORMATION WINDOW, SET-5.6): **0 = On, 1 = Off** (checked 2026-11-14 b in a picture of the original, E-trace-026: a profile with 0 shows On) | 0 |
| 18 | string (v >= 3) | unknown (empty) | empty string |
| 19 | string | unknown (empty) | empty string |
| 20 | bool (v >= 2) | unknown | true |
| 21 | bool | free camera inverted: the Input page's `!FREE CAMERA` shows Inverted while true (SET-5.5) | false |
| 22 | u32 (v >= 5) | autosave on leaving: the box of `UI-menus.md` UI-MENU-3.3, 0 off (Q-298, E-ui-016) | 0 |

- **SET-2.1** The autosave interval is chosen in the game settings page from Off, 1 minute, 5, 10,
  20, 45 minutes, 1 hour (0, 60, 300, 600, 1200, 2700, 3600 s).
- **SET-2.2** The map states are keyed by map name and created on demand (CAM-3.5); an entry with
  an empty name may exist (observed) and is harmless.
- **SET-2.3** The day / night cycle switch of the game settings page is not in this block; it is a
  scene setting (SET-4).

### SET-3 Defaults (no file)

Profile `default`, autosave name empty, interval 0 (off), map list empty, player name `default`,
input option A true, the other unknown bools false, info window on (field 17 = 0). Volumes: master 1.0, music 0.3,
UI 1.0, sound 1.0, ambient 1.0 (SND-5.1). Confidence: high for the strings and volumes, medium for
the interval (the constructor zeroes the field; the observed file holds 600).

### SET-4 `SceneSettings` (version 1), in order

u32 (graphic quality preset 0..5: VeryLow, Low, Medium, High, VeryHigh, Custom; observed 4), bool,
f32, bool, u32 width, u32 height (observed 1024 x 768), f32, bool, bool, bool, f32, f32, bool, f32,
f32, u32 (shader detail: 1 off, 2 low, 3 high; observed 3), f32, f32, bool (observed values in the
investigation note). The 11th field (the f32 after the three bools) is `SHADOW QUALITY` 0..1 (observed
1.0; GFX-VIEW-2.5 d); without shader level 2 it, the next f32 and the third bool are forced to 0
(Run D, E-gfx-016). The engine page's sliders (terrain, shadow quality 5 steps, shadow detail 7,
mirror quality 5, mirror details 5, object details 4, draw distance, texture quality 4) write these
fields through the scene object. Windowed / full screen is a device setting read from the display
device, not this block.

**SET-4a The fields by name (2026-11-12, E-gfx-056, code reading; Q-100 c).** In file order:
1 preset; 2 bool (not read here); 3 `TEXTURE QUALITY`; 4 a display-mode bool (handed to the display
mode with fields 5 and 6); 5, 6 width and height; 7 `TERRAIN QUALITY`; 8, 9 bools (not read here);
10 **the shadows switch** (GFX-VIEW-2.5 e); 11 `SHADOW QUALITY` (the map's size, GFX-VIEW-2.5 d);
12 `SHADOW DETAIL`; 13 **the mirror switch** (GFX-WATER-2.6); 14 `MIRROR QUALITY`; 15
`MIRROR DETAILS`; 16 `SHADERDETAIL`; 17 `OBJECT DETAILS`; 18 `OBJ DRAW DISTANCE`; 19 `DayNightCycle`.
Every slider value is kept clamped to 0..1.
- **Presets** (applied when the preset changes, never on load): VeryLow: fields 3, 7, 11, 12, 14, 15,
  17, 18 all 0, shader detail 1. Low: 11 = 0, 12 = 1/6, 17 = 1/3, 3 = 1/3, 7 = 14 = 15 = 18 = 0.25,
  shader 2. Medium: 11 = 0.5, 12 = 1/3, 17 = 2/3, 3 = 2/3, 7 = 14 = 15 = 18 = 0.5, shader 3. High:
  11 = 0.75, 12 = 2/3, 17 = 1, 3 = 1, 7 = 14 = 15 = 18 = 0.75, shader 3. VeryHigh: all 1, shader 3.
  Custom sets no value. After any preset (Custom included): **field 10 = (field 12 above 1/6)**,
  **field 13 = (field 14 above 0)**.
- **Without a profile file (2026-11-17 night, Q-188 b; E-gfx-056 addendum, code reading, high):** the
  scene settings start as their constructor makes them: field 1 = 4 (VeryHigh) with that preset
  applied, so fields 3, 7, 11, 12, 14, 15, 17, 18 are 1, field 16 (shader detail) is 3, fields 10 and
  13 are on; fields 2, 4, 8 and 9 are off; fields 5 and 6 (width, height) are -1, unset; field 19
  (`DayNightCycle`) is on. What the window then takes for an unset size was not read (Q-188 e).
- **Slider moves:** the first move of any slider switches the preset to Custom (which applies the two
  recomputations above once). `SHADOW QUALITY` moved: field 10 = (new value above 0), then field 11 =
  the value. `SHADOW DETAIL` moved: field 12 only. `MIRROR QUALITY` or `MIRROR DETAILS` moved: that
  field, then field 13 = (`MIRROR QUALITY` above 0).
Confidence: high (the page's widgets paired with their labels, getters and setters).

**SET-4b When an Engine page change takes effect (2026-11-13, E-gfx-063, code reading).** A move on
the Engine page writes the settings at once (SET-4a). The view reads some of them every frame and
copies the others once, when a game's view is set up (a new game or a loaded one); a copied value
changes nothing until the next game. Nothing waits for a restart of the program.
- **At once (the next frame):** field 10, the shadows switch: every frame decides from it whether
  the shadow map is drawn and whether objects and ground use it (so moving `SHADOW QUALITY` to 0, or
  up from 0, turns shadows off or on at once). Field 13, the mirror switch: every frame decides from
  it whether the mirror image is drawn. Field 14, `MIRROR QUALITY`: the mirror image's size is
  recomputed from it whenever the image is drawn (GFX-WATER-2.6: 128 << trunc(4 q), none at level 0),
  so a move changes its sharpness at once. Field 16, `SHADERDETAIL`: every frame picks the plain or
  the low technique variants from it (low when it is not High). Field 18, `OBJ DRAW DISTANCE`, is
  also read every frame (medium).
- **At the next game:** field 11, `SHADOW QUALITY` as the **map size** (the view keeps the value of
  the game's start; the switch above still acts at once), field 12 `SHADOW DETAIL`, field 15
  `MIRROR DETAILS`, field 16's second use (the device's shader level, set from it at the game's
  start: Off (1) gives level 1, or 0 on a device below shader model 2; Low (2) and High (3) give 2), field
  7 `TERRAIN QUALITY` and field 17 `OBJECT DETAILS`.
- **The water's use of the mirror** is also fixed at the game's start (the water samples the mirror
  image only if field 13 was on then). So a mirror turned on during a game is drawn but not shown on
  the water until the next game, and a mirror turned off during a game stops being drawn while the
  water keeps showing its last image until the next game. Likewise a game started with
  `SHADOW QUALITY` 0 and raised later turns the switch on with a map of level 0 (no map; what is then
  drawn was not read).
Confidence: high for which values are read every frame and which once (each reader found; the
copying step has one caller, the view's set-up), medium that the set-up runs exactly once per game
(it is the view's first virtual method) and for field 18. Test T6.

### SET-5 Settings dialog (answers UI-MENU-4.3)

- **SET-5.1** Six tabs at (15, 25), 40 x 50 px steps, in the order Graphics, Engine, Sound, Input,
  Game, Network; a `Reset` button at (168, 408). Page content starts at (168, 72), rows 32 px apart,
  labels 144 px left of the control.
- **SET-5.1a Window titles (2026-10-24).** Each page sets the title of the settings window when it is
  built: Graphics `Gfx Settings`, Engine `Engine Settings`, Sound `Sfx Settings`, Input
  `Input Settings`, Network `Network Settings` (titles set in the four page builders; the Network title, its
  builder not identified). Confidence: medium (read from the title calls; the title bar of the settings
  window not photo-checked).
- **SET-5.2** Graphics: `RESOLUTION` dropdown listing the device's modes as `<w> x <h>` (at most
  24, 25 with the current one added), `MODE` Fullscreen / Windowed.
  **In detail (2026-11-15, E-ui-035: code reading and the picture of E-trace-026):** title `!Gfx
  Settings`, shown GRAPHICS SETTINGS (the title font's capitals, `UI-ingame.md` UI-GAME-1.8). Two
  drop-downs (152 x 20, the choice list of UI-GAME-8.5g): `!RESOLUTION` at (168, 72) showing the
  current mode (e.g. `2560 x 1440`), `!MODE` (shown DISPLAY MODE) at (168, 104) with `!Fullscreen`
  (shown Fullscreen Mode, value 1) and `!Windowed` (shown Windowed, 0). No `!Reset` (UI-MENU-4.0).
  Captions as on every page: left edge x 24 and **centred vertically on the control's middle**
  (y + 10: ink of RESOLUTION 78 .. 86, DISPLAY MODE 110 .. 118). Confidence: high.
- **SET-5.3** Engine: `GRAPHIC QUALITY` dropdown (SET-4 presets), sliders `TERRAIN QUALITY`,
  `SHADOW QUALITY`, `SHADOW DETAIL`, `MIRROR QUALITY`, `MIRROR DETAILS`, `OBJECT DETAILS`,
  `OBJ DRAW DISTANCE`, `SHADERDETAIL` dropdown (Off only if the device lacks shader model 3, Low,
  High), `TEXTURE QUALITY` slider when supported.
- **SET-5.4** Sound: as UI-MENU-4.2 (four sliders), greyed without a sound device.
- **SET-5.5** Input (**corrected 2026-11-15, E-ui-035**: code reading of the page's builder and the
  picture of E-trace-026; the earlier list had the order, the captions and two bindings wrong).
  Title `!Input Settings`, shown INPUT SETTINGS. Five controls from (168, 72), 32 px apart:
  1. `!SCROLL SPEED`: a slider (the sound page's slider style, UI-MENU-4.2) at (168, 72), value field
     7 (0 .. 1; 0.25 by default and after `!Reset`); its caption is the slider's own, **22 px below
     the slider's top** (ink 90 .. 98), not on the middle like the drop-downs' captions;
  2. `!MOUSE SCROLLING` at (168, 104): `!Inverted` (shown Inverted, value 0) / `!Not Inverted` (shown
     Not inverted, 1); shows Inverted while field 6 is true;
  3. `!BORDER SCROLLING` at (168, 136): `!Activated` (shown Enabled, 0) / `!Not Activated` (shown
     Disabled, 1); shows Enabled while field 8 is 1;
  4. `!SMART CURSOR` at (168, 168): Enabled / Disabled as 3; shows Enabled while field 9 is 1;
  5. `!FREE CAMERA` at (168, 200): Inverted / Not inverted as 2; shows Inverted while field 21 is
     true.
  Drop-downs 152 x 20, captions at x 24 centred on the control's middle (ink 110 .. 118, 142 .. 150,
  174 .. 182, 206 .. 214). `!Reset` shown at (168, 408) (UI-MENU-4.0: fields 6 off, 7 = 0.25, 8 on,
  9 on, 21 off). Field 5 is on no page. Jens's profile shows Not inverted, Enabled, Enabled, Not
  inverted. Confidence: high (builder, labels, choices and the fields read; the picture agrees).
- **SET-5.6** Game: `AUTOSAVE` dropdown (SET-2.1), `Infowindow` On / Off, `DayNightCycle` On / Off.
  **`!Infowindow` (2026-11-14 b, F62; E-ui-032, code reading; E-trace-026, a picture of the original's
  game page):** shown as INFORMATION WINDOW, a drop-down with the choices On and Off; it shows On while
  field 17 is 0 and Off while it is 1 (the picture: Jens's profile, field 17 = 0, reads On); the file
  is written when the page is left (SET-1.2). A new profile has it On (SET-3). What it switches is the
  help panel that windows with a help page can open (medium; the polarity of that use was not
  settled by code reading). **It is not the messages window:** that window has no setting and is
  shown at every game start (`UI-ingame.md` UI-GAME-11.3a). Confidence: high for the values (picture),
  medium for what it does.
- **SET-5.7 Network page (2026-11-15, E-ui-035: code reading and the picture of E-trace-026).** Title
  `!Network Settings`, shown NETWORK SETTINGS. `!CALLSIGN` at (168, 72): a text input 152 px wide
  (font `light_system`) showing the player's name (field 16, `default` in Jens's profile; medium for
  the binding); `!INTERNET SPEED` at (168, 104): a drop-down with `!NONE` (0), `!MODEM` (1), `!ISDN`
  (2), `!DSL/CABLE` (3), `!LAN` (4), showing the network system's current value (NONE in the picture;
  not a profile field). Captions at x 24 centred on the control's middle (ink 79 .. 87, 109 .. 118).
  `!Reset` is shown at (168, 408) and does nothing here (UI-MENU-4.0). Nothing of this page changes a
  single-player game. Confidence: high for the controls, their places and choices; medium for what
  the callsign is bound to.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| settings_path | `<Documents>\S2\users\<profile>.cfg` | | high | E-set-001; file observed |
| user_object_version | 5 | | high | E-set-001, file |
| scene_settings_version | 1 | | high | E-set-001, file |
| autosave_choices | 0, 60, 300, 600, 1200, 2700, 3600 | s | high | E-set-001 |
| default_player_name | `default` | | high | E-set-001 |
| quality_presets | 6 | | high | E-set-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | `default01.cfg` (277 bytes, 2026-10-13) | parse | `UserObject` v5: name `default01`, autosave `Autosave` / 600 s, map state `MP_2P_Storm_Coast` (not completed, acknowledged 2), volumes 1.0 / 0.3 / 1.0 / 1.0 / 1.0, player `default`; `SceneSettings` v1 with 1024 x 768; ends exactly at 0x115 |
| T2 | no file | start | defaults of SET-3; the game runs without creating the file until a settings page is closed |
| T3 | game settings page, choose "10 Minutes", close | | file rewritten with interval 600; periodic autosave every 600 s of game time (FMT-SAV-5.2) |
| T4 | campaign01 won | | the file's map list gains `campaign01` completed = true (CAM-4.2) |
| T5 | SET-4a: preset Low applied, then `SHADOW QUALITY` moved to 0.25; then to 0 | fields 10, 11 | after Low: 10 off (detail 1/6 is not above 1/6), 11 = 0; after 0.25: preset Custom, 10 on, 11 = 0.25; after 0: 10 off, 11 = 0. Preset Medium: 10 on, 11 = 0.5, 13 on |
| T6 | SET-4b: a game started with preset Medium (fields 10 on, 11 = 0.5, 13 on, 14 = 0.5); during the game `SHADOW QUALITY` moved to 0, then to 1; `MIRROR QUALITY` moved to 1, then to 0; `SHADERDETAIL` set to Low | what the view shows, frame by frame | shadows vanish in the next frame after the move to 0 and come back in the next frame after the move to 1, with the map size of 0.5 (level 2, 512) until the next game; the mirror image is redrawn at level 4 (2048) in the next frame, then at 0 it stops being drawn while the water keeps its last image; the low technique variants from the next frame; after leaving and starting a new game: shadow map of level 4, no mirror on the water |
| T7 | SET-5.5, 5.7 (E-ui-035): a profile with fields 6 false, 7 0.25, 8 1, 9 1, 21 false | open the settings, Input tab, then Network tab | Input: slider at (168, 72) with SCROLL SPEED under it (ink 90 .. 98), then MOUSE SCROLLING Not inverted, BORDER SCROLLING Enabled, SMART CURSOR Enabled, FREE CAMERA Not inverted at y 104 / 136 / 168 / 200, Reset shown; Network: CALLSIGN with the player's name, INTERNET SPEED NONE; Graphics: RESOLUTION and DISPLAY MODE, no Reset |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-set-001 | code reading | S2DNG.exe | path, format, defaults, dialog bindings |
| E-ui-032 | code reading | (session 2026-11-14 b) | SET-5.6 the `!Infowindow` drop-down and the help panel it gates; not the messages window |
| E-trace-026 | a picture of the running original | the game settings page at 2560 x 1440 (session 2026-11-14 b) | SET-2 field 17: a profile with 0 shows On |
| E-set-002 | data | the profile file `default01.cfg`, parsed | observed values, exact length |
| E-set-003 | code reading | S2DNG.exe (session 2026-11-16 day): the start-up's one settings object, created with the fixed name default01 and loaded | SET-1.1 (Q-188 a) |
| E-gfx-056 | code reading | the scene settings' setters, getters and presets; the engine page's widgets, labels and slider handlers | SET-4a, T5 |
| E-gfx-063 | code reading | (session 2026-11-13) the Engine page's change handler; every reader of the scene settings (the per-frame pass set-up, frame, mirror draw and update; the view's settings copy and its one caller, the view's set-up; the mirror image's size function; the device's shader-level setter) | SET-4b, T6 |
| E-ui-035 | code reading, a picture | (session 2026-11-15 night; the pictures of E-trace-026) | SET-5.2, 5.5, 5.7: the Graphics, Input and Network pages' controls, choices, fields and places; SET-2 fields 5, 6, 8, 9, 21 |

## Open questions

- Q-100 (c): the meaning of fields 18 to 22 of SET-2, the exact pairing of the five input fields with
  the input page controls, and the per-field meaning of the scene settings block (which slider writes
  which field).
