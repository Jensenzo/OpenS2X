# TOOL-capture: Fixed-step frame time, screenshots and the hidden camera path tool

Status: draft (from code reading; nothing here has been run yet)
Owner question(s): change of method 2026-10-18, part 2 (capture mode found in the frame time's code); raises Q-304, Q-305, Q-306
Game build: 11757

## Summary

Every frame the original takes one "frame time" in seconds. Normally this is
the real time since the previous frame. A built-in debug flag replaces it with
exactly 0.025 s per frame. Then the simulation, the animations and the camera
advance by the same amount every frame, however long the frame took to draw.
This makes the original deterministic per frame, which matters for measuring it.
The flag belongs to a hidden camera path tool that flies the camera along
recorded points and saves a screenshot every frame. In this build no key, switch
or setting can reach that tool, so a measuring run has to set the flag in memory
(TOOL-4). The ordinary screenshot key (Print Screen) and its resolution factor
are part of the shipped game and are specified here too.

## Rules

### TOOL-1 Frame time

- **TOOL-1.1** Each frame has one frame time `f` in seconds. Normally `f` is the
  real time between this frame and the previous one, in whole milliseconds
  times 0.001. While the fixed-step flag is on, `f` is the constant 0.025
  (as a 32-bit float, 0.025000000372529), whatever the real time was.
  Confidence: high (measured in live trace 1, E-trace-001: in 2236 frames at real
  time each frame's step was its whole-millisecond frame time x 0.001; with the fixed
  step on, all 1620 frames had 0.025 while the real frame time stayed 16 to 17 ms).
- **TOOL-1.2** Everything that advances with `f` therefore advances in equal
  steps under the fixed step:
  - the game time step `f x time_scale x speed_factor` (ECO-TIME-1.2), so the
    logic ticks: at speed factor 1, exactly 4 frames per 0.1 s tick, 40 frames
    per second of game time;
  - the scene's animation and effect clock, which is that same game time step
    (pooled settler poses, building and ship clips, tree fall and sink,
    GFX-ANIM-2, 3.8);
  - camera movement, zoom and following (GFX-VIEW-3);
  - message timers and their refresh window (UI-GAME-11.1), the messages
    window's 1 s housekeeping and scroll repeat;
  - tooltip delay, slider repeat, tutorial text timing, mission triggers and
    the end-of-game check, the defeat screen, the minimap dialog, the ship
    dialog, the film widget, the lobby.
  Confidence: high (all 32 readers of the frame time are listed in E-tool-001;
  the game time step is the one ECO-TIME-1.2 describes).
- **TOOL-1.3** Not under the fixed step: the second clock (milliseconds since
  start), whose only visual reader is the animated speed icon (UI-GAME-2.7); its
  other readers are network session and lockstep timers. On low shader quality
  (the fixed-function water) the water's texture drift and its 64-frame texture
  animation read the system millisecond counter directly. Sound and music streams
  and films run on their own. Everything else drawn follows either the game step or
  the frame time and so the fixed step. Confidence: medium (all readers listed in
  E-tool-005, Q-305; TOOL-trace TRACE-5).
- **TOOL-1.4** The flag is off at start. Only the camera path tool (TOOL-3)
  turns it on or off. No command-line switch, registry value, configuration
  entry or other key changes it. The program recognises 17 command-line switches
  (`-low`, `-med`, `-fullscreen`, `-postFX`, `-start:`, `-log_disabled`,
  `-log_error`, `-log_warn`, `-log_info`, `-hwlab_verylow` ..
  `-hwlab_veryhigh`, `-3_0`, `-2_0`, `-ffp`). The only registry key it reads
  is `HKLM\SOFTWARE\Ubisoft\Funatics\S2TNG`, used for the online login.
  Confidence: high.

### TOOL-2 Screenshots (shipped feature)

- **TOOL-2.1** In a game, pressing Print Screen (virtual key 0x2C) requests a
  screenshot. It is taken at the start of the next scene draw: the scene is
  drawn again into an off-screen target, copied, and saved. Confidence: high.
- **TOOL-2.2** Numpad + and numpad − raise or lower a screenshot factor `k`
  by one. `k` is clamped to 1..8 and starts at 1. Each press shows the text
  `Screenshot: W x H` for 3 s of game time (it counts down by the scene's
  step, so it does not fade while the game is paused), where W and H are the
  backbuffer's width and height times `k`. The text is centred horizontally
  (x = half the backbuffer width), about 100 px from the top. Confidence: high
  for the keys, the clamp, the text and the 3 s; medium for the text position
  and the pause behaviour.
- **TOOL-2.3** With `k` = 1 the shot is one draw at screen size. With `k` > 1
  the scene is drawn in several tiles and assembled into a W x H image. After
  every screenshot `k` returns to 1. Confidence: high for the reset; medium for
  the tiling (tile count and projection were not read).
- **TOOL-2.4** The file is `<Documents>\S2\screenshots\Pic_YYYYMMDD_HHMMSS.bmp`.
  The name uses the local time with a 4-digit year and 2-digit month, day,
  hour, minute and second, and the image is a BMP. Two shots in the same
  second get the same name, and the second one replaces the first.
  Confidence: high for the folder, the name and the format; medium for the
  replace (the save routine writes without checking for an existing file).
- **TOOL-2.5 What the shot contains (answered 2026-11-09 c, Q-304; E-sav-012).** **The 3D world only.**
  When the request is seen (at the start of the scene's update, before the frame's own
  drawing), the scene draws itself once more into the off-screen target with its own world
  draw: the scene passes of `GFX-world-view.md` (sky, terrain, roads, objects, settlers,
  water; the scene's current draw mask) and the map's world-space overlays (the per-cell
  overlay layer and the pulsing marker on a field), then copies and saves it. The interface is
  drawn by the user-interface system on the frame's own pass, not by the scene, so the shot
  has **no HUD, no open windows, no screen frame, no mouse pointer, no message or "Paused"
  text, and none of the pause layer's darkening** (UI-GAME-1.11): a shot taken while paused
  shows the world at full brightness. The mirror image and shadow map are those of the last
  frame. A capture-playback frame (TOOL-3.6) uses the same request and has the same content. The
  in-game Y key that hides the interface (UI-GAME-12.1) is therefore for screen captures made
  outside the game. With `k` > 1 every tile is the same world draw (2.3). Confidence: high (the
  shot's draw is the scene's own world draw; the interface has no call in it). **Seen live (2026-11-15 night,
  E-trace-027):** PRINT SCREEN sent to the game while it stood paused (P) wrote
  `Pic_YYYYMMDD_HHMMSS.bmp`, 2560 x 1440 x 32 bit (14,745,656 bytes) at screenshot factor 1, within
  a second, with the paused counter unchanged; the picture is the world at full brightness with no
  interface and no "Paused" text, and at full resolution it equals the screen's paused frame
  multiplied by 255 / 90 (UI-GAME-1.11). So pictures of the original for comparisons can be taken
  paused at an exact tick without the pause layer.
- **TOOL-2.6** The save routine also has a numbered naming mode
  (`<prefix>_NNNNN.bmp`, 5 digits, one global counter). Nothing in this build
  sets the prefix, so that mode is never used and need not be built.
  Confidence: high.

### TOOL-3 Camera path tool (present but unreachable)

- **TOOL-3.1** The in-game key handler would enter the camera path mode on
  CTRL+SHIFT+S. The plain S binding (open the settings dialog, UI-GAME-12.1)
  is tested first on the same key event and always handles it. So
  CTRL+SHIFT+S opens the settings dialog like S, and the path mode cannot be
  reached in build 11757. An implementation must do the same (S with any
  modifiers opens the settings) and must not offer the tool to players.
  Confidence: high.
- **TOOL-3.2** Entering the mode (TOOL-4.3 is the only way in) clears the
  point list, turns playback off, and clears the fixed look-at point and the
  capture flag. Confidence: high.
- **TOOL-3.3** Keys in the mode, read on the same key event as the other
  in-game keys:

  | key | action |
  |---|---|
  | RETURN | append a point: the camera's current eye position (its view direction is stored too but never used); beep 100 Hz for 100 ms |
  | SHIFT | set the look-at point to the camera's current eye position; beep 400 Hz for 100 ms |
  | CTRL | preview: with 3 or more points start playback at real frame time; with fewer, beep 1000 Hz for 1000 ms |
  | DELETE | capture: hide the screen frame (as Y does), turn the fixed step on, then with 3 or more points start playback with a screenshot every frame; with fewer, beep 1000 Hz for 1000 ms and leave the fixed step on |
  | PAGE DOWN / PAGE UP (first press) or the mouse wheel | leave the mode: back to the normal camera |

  Confidence: high for the bindings and beeps.
- **TOOL-3.4** Playback: a segment parameter `u` starts at 0 and grows by
  `f x 0.125` per frame, so each segment between two successive points takes
  8 s of frame time (320 frames under the fixed step). When `u` reaches 1, it
  drops by 1 and the next segment starts. Playback stops when the segment
  index reaches (number of points − 1). With n points it therefore plays
  n − 1 segments. When it stops, the fixed step is turned off. The screen frame
  stays hidden until Y is pressed. Confidence: high for the timing and the
  stop; medium for the frame staying hidden.
- **TOOL-3.5** The eye follows a smooth Hermite curve through the points.
  Without a look-at point the camera looks along its direction of travel
  (this frame's position minus the previous frame's). With a look-at point
  it looks at that point. Confidence: medium (the curve's tangent rule was
  not read in detail).
- **TOOL-3.6** During a capture playback every frame requests a screenshot
  (TOOL-2). Because the file name changes only once per second (TOOL-2.4),
  only the last frame drawn in each wall-clock second survives on disk. The
  tool therefore does not give a complete frame sequence. Confidence: medium
  (follows from TOOL-2.4).

### TOOL-4 How to switch it on (for a measuring run; TOOL-4.2's byte used in live trace 1)

- **TOOL-4.1** Where the running game's code and data lie and how to attach to it:
  kept in the investigation's private notes (method of the investigation, no rule of
  the game). Never modify the installed `bin/S2DNG.exe` itself. Confidence: high that
  a data write is tolerated (measured in live trace 1); medium that the protector
  tolerates a debugger (Q-306, Q-308).
- **TOOL-4.2 Fixed step only (recommended for traces).** Write byte 1 to the fixed-step flag. From the next frame on, every frame is 0.025 s. Write 0 to go back
  to real time. To take a screenshot of a chosen frame, press Print Screen.
  For a screenshot of every frame, also set the scene's screenshot request byte to 1
  each frame, for example with a breakpoint command where the
  scene draw reads that byte. Since names repeat within a second
  (TOOL-2.4), move or rename each file after its save, for example with a
  breakpoint just after the save whose script renames the newest
  `Pic_*.bmp` to a frame number. Confidence: high for the fixed-step byte
  (measured in live trace 1 with a write script: from the next
  frame on every step was 0.025, the game showed no other effect); medium for the
  per-frame screenshot and the renaming (not executed).
- **TOOL-4.3 The camera path tool.** Reachable only through a change of the running
  game's code in memory, at the in-game key handler's plain S test (TOOL-3.1); the change
  is kept in the investigation's private notes (E-tool-001). With it, plain S no longer
  opens the settings dialog, and CTRL+SHIFT+S enters the path mode (TOOL-3). Confidence: medium
  (not executed; Q-306).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| fixed frame time | 0.025 | s per frame | high | E-tool-001 |
| frames per tick at speed 1 under the fixed step | 4 (5, rarely 3, in about one tick of 300: float32 drift, TOOL-trace TRACE-2.6) | frames | high (measured) | E-trace-001; ECO-TIME-1.2 |
| screenshot factor range | 1 .. 8, start 1, reset to 1 after each shot | | high | E-tool-001 |
| factor text duration | 3.0 | s of game time | high | E-tool-001 |
| camera path segment rate | 0.125 | segments per s of frame time | high | E-tool-001 |
| camera path minimum points | 3 | points | high | E-tool-001 |
| beeps | 100 Hz/100 ms (point), 400 Hz/100 ms (look-at), 1000 Hz/1000 ms (too few points) | | high | E-tool-001 |
| fixed-step flag | one byte: 0 off, 1 on | flag | high | E-tool-001 |
| screenshot request | one byte of the scene: 1 = take a shot at the next scene draw | flag | high | E-tool-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | game at speed factor 1, fixed step on | draw 40 frames, each taking any real time | exactly 10 logic ticks, 1.0 s of game time; every pooled animation clock advanced by 40 x 0.025 x time scale |
| T2 | fixed step off | a frame takes 16 ms | frame time 0.016 s |
| T3 | fixed step on, speed factor 2 | 20 frames | 1.0 s of game time, 10 ticks |
| T4 | local time 2026-10-18 15:04:05 | Print Screen | `<Documents>\S2\screenshots\Pic_20261018_150405.bmp` |
| T5 | factor 1, backbuffer 1024 x 768 | numpad + three times | factor 4, text `Screenshot: 4096 x 3072` for 3 s; after the next shot the factor is 1 |
| T6 | factor 1 | numpad − | factor stays 1; text `Screenshot: 1024 x 768` |
| T7 | factor 7 | numpad + twice | factor 8 |
| T8 | in game | CTRL+SHIFT+S | the settings dialog opens, as for S; no camera mode change |
| T9 | path mode (TOOL-4.3), 4 points, DELETE | play | 3 segments x 320 frames = 960 frames; afterwards the fixed step is off |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | clock watches at real time and with the fixed step | TOOL-1.1, TOOL-4.1, 4.2 |
| E-tool-001 | code reading | S2DNG.exe: frame time and the flag's setter, the game and scene time steps, the camera path mode, the in-game and scene key handlers, the screenshot and its save, the command-line switches, the registry read | flag, its two setters, consumers, keys |
| E-tool-005 | code reading | S2DNG.exe: second clock and its 18 readers, GetTickCount and QPC timer users | TOOL-1.3 |
| E-sav-002 | data | `Documents\S2` folder (FMT-savegame) | `<Documents>\S2\screenshots\` folder name |

## Open questions

- Q-304: does a screenshot include the HUD, windows and screen frame?
- Q-305: answered (medium, TOOL-1.3).
- Q-306: does the protector tolerate an attached debugger and memory writes
  to its code (TOOL-4.3) and its data (TOOL-4.2)?
