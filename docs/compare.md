# The look comparison: the same shot from both programs

To compare our picture with the original's pixel by pixel, both programs show the
same save, from the same camera, at the same window size, with no interface. Our
side is one command; the original's side is a few keys. Both rest on rules in the
specs, so where the two shots differ it is the drawing, not the setup.

## Our shot

```
s2-viewer.exe --compare "<save>.sav" --zoom 3 --size 1024x768 --shot out.png
```

From WSL, with the paths in Windows form:

```bash
target/x86_64-pc-windows-gnu/release/s2-viewer.exe \
  --game-dir "$(wslpath -w "$S2_GAME_DIR")" \
  --compare "$(wslpath -w "$S2_SAVES_DIR/<a save of the original>.sav")" \
  --zoom 3 --size 1024x768 --shot "$(wslpath -w "$PWD")\\out.png"
```

What `--compare` does:

- **The save** is read as the original writes it (`FMT-savegame.md`; the same reader
  as `--save` and the game's own load dialog) and continued.
- **The game stands still**: it starts paused, so the shot shows the state stored in
  the save.
- **The camera** is made afresh, as every game start does (GFX-VIEW-3.6 a): yaw 0,
  then a jump to the local player's headquarters, the target 18 units in front of its
  field point, so the HQ sits just above the screen centre (27 px on a 768-pixel
  screen at zoom 3, 3.6 b). `--zoom N` (0 nearest to 4 farthest, default 3) sets the
  zoom step the camera starts at. Our camera has no easing to wait for at a start: its
  first frame is the settled position.
- **No interface**: no HUD, dialogs, tooltips or screen frame, as after the Y key
  (UI-GAME-12.1). In a normal game Y now hides and shows the interface too.
- **The shot** is the first frame drawn once the 3D scene of the save is built. The
  renderer compiles every pipeline in the frame that first needs it (only in this
  mode), so that frame is complete. The window presents nothing in its first few
  frames; a capture that comes back one flat colour is taken again in the next frame.
  The program then quits.
- `--size WxH` is the window's size in pixels (default: the profile's resolution).

## The original's shot (by hand)

1. **Resolution.** In the original's settings (graphics page) choose the same size as
   `--size`, e.g. 1024 x 768, and restart it if it asks to.
2. **Load the save.** Main menu, LOAD GAME, the same save. When the game screen is
   built the camera starts at zoom 3, yaw 0, on your headquarters (GFX-VIEW-3.6). Do
   not move the mouse over the map or touch the keys yet.
3. **Pause:** press **P** at once. The game stands still and so do its animations
   (their clock is the game's time step, TOOL-1.2).
4. **Zoom**, only if `--zoom` was not 3: **PAGE DOWN** (or wheel up) zooms in one step,
   **PAGE UP** (or wheel down) out. From 3, zoom 2 is one PAGE DOWN, zoom 1 two, zoom 4
   one PAGE UP. If the camera moved, **H** (or HOME) puts it back on the headquarters at
   zoom 3 (UI-GAME-12.1), then zoom again.
5. **Hide the interface:** press **Y** (HUD and frame go, UI-GAME-12.1).
6. **Move the mouse pointer off the map** (onto the window's title bar or another
   screen), so no hover marker or tooltip is drawn.
7. **Wait a second** for the camera to settle (its height eases in over about ten
   frames, 3.6 c), then press **PRINT SCREEN**. The original saves
   `Documents\S2\screenshots\Pic_YYYYMMDD_HHMMSS.bmp` (TOOL-2.4); with OneDrive that is
   `%USERPROFILE%\OneDrive\Documents\S2\screenshots\`. Keep the screenshot factor at 1
   (numpad + and - change it, TOOL-2.2; it shows "Screenshot: W x H" when changed).
8. Copy the `.bmp` next to our `out.png` in the look-compare folder with the same name
   stem, e.g. `cmp-storm-zoom3-original.bmp` and `cmp-storm-zoom3-ours.png`.

Whether Print Screen captures the interface at all is open (Q-304); with Y pressed it
does not matter. The fixed frame step of TOOL-4 is not needed for a paused still.

## Comparing

A difference image (black where equal) and a side-by-side, with FFmpeg:

```bash
ffmpeg -i ours.png -i original.bmp -filter_complex "[0][1]blend=all_mode=difference" diff.png
ffmpeg -i ours.png -i original.bmp -filter_complex hstack pair.png
```

What will differ even when both draw alike:

- **Settlers' animation phases**: our clips start when our scene is built, the
  original's when its scene was built; the poses differ although the settlers stand
  on the same fields. Carriers and settlers out on the roads come back from the save
  only as far as our save reader restores them.
- **Clouds and water** move with time; the two shots are not taken at the same time.
- **Shadows**: our shadow map is Bevy's, not the original's (`docs/shadows.md`).
- **Edges**: the two programs' anti-aliasing differs (ours is 4 x MSAA).

## The automatic comparison (TOOL-trace TRACE-8)

The investigation's recorder pauses the original at chosen trace ticks, reads its
camera from memory and takes the window's picture: one run's shots land in
`$S2_SCREENS_DIR/look-compare-auto/<date>/original/` (the maintainer's
screens folder, outside the repository) as `<stem>.png` with
`<stem>.json`, the camera file (`tick`, `map`, `game`, `window`, `eye`, `dir`, and
for reference `target`, `zoom`, `yaw`; agreed 2026-10-29). Then

```bash
scripts/look-compare.sh "$S2_SCREENS_DIR/look-compare-auto/<date>"
```

draws ours for each with `s2-app shot --map M --ai 1=1 --tick T --camera FILE
--size WxH --out PNG`: the headless game of the trace played to tick T, drawn by
the viewer (`--compare`, `--run T`, `--camera FILE`, `--shot`) with the camera at
the original's position and view direction. The viewer's state hash at T must be
the headless game's, else no picture is made. The script writes `ours/<stem>.png`
and the page `index.html` with the pairs side by side; nothing goes into the
repository. The same differences as above remain (animation phases, clouds and
water, shadows, edges). Our day clock is set to the tick's hour (`--hour`: 12 plus
0.033 hours per game second, GFX-VIEW-2.3c); a `light` field in the camera file,
when the investigation reads one, is not used yet.

To measure a pair rather than look at it, `s2-app colour-mean ORIGINAL.png
OURS.png NAME=X,Y,W,H...` gives each rectangle's mean red, green and blue in both
pictures and their ratio per channel; `--mark OUT.png` outlines the rectangles on
the first picture, to check that they sit on the same ground in both (by
light level: lit ground, shadowed ground, a site, a roof).

A whole run of `scripts/look-compare.sh` is measured with `s2-app look-measure
DIR [--masks MASKS]`: the fixed regions of each scene (cell63, default, farm),
none touching the original's interface (the 12-pixel frame, the corner
ornaments, the button bar: `GFX-water.md` GFX-WATER-2.6c), and the water by the
depth of the ground under it through the investigation's per-sample bin masks
(`MASKS/mask-<stem>.png`, T20). It prints a Markdown table with the ratio
original / ours and two pass columns: within 3 %, and within 3 % or 1 level
where the original's mean is below 30.
