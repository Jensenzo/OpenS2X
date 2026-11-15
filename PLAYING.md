# Playing OpenS2X

OpenS2X plays *The Settlers II: 10th Anniversary* with your own copy of the game. It contains no files from the game; it reads them from your install.

## What you need

- *The Settlers II: 10th Anniversary*, installed (the GOG edition, build 11757, is what the engine is checked against). The folder you need is the one that contains `data`.
- Windows 10 or 11 with a graphics card that runs Vulkan or DirectX 12.
- `OpenS2X.exe`, from the download (`OpenS2X-<version>-windows-x64.zip` on the project's [Releases](https://github.com/Jensenzo/OpenS2X/releases) page). Nothing else is installed.

## Starting

Run `OpenS2X.exe`. The first time, it asks where the game is installed: choose the folder that contains `data` (for GOG, usually `C:\GOG Games\Settlers 2 10th Anniversary`). The choice is kept in `%APPDATA%\s2-reborn\settings.txt` (the folder keeps the project's earlier name); delete its `game_dir` line, or the file, to choose again. The `S2_GAME_DIR` environment variable, or `--game-dir <folder>` on the command line, take precedence.

The game opens with the intro film (a key or a click skips it), then the start screen: **Start** leads to the main menu, where the town on the left sheet moves.

The films are optional. They are the original's, converted once from your own game files with [FFmpeg](https://ffmpeg.org) into a `video` folder beside `OpenS2X.exe`: the commands are in [docs/video.md](https://github.com/Jensenzo/OpenS2X/blob/main/docs/video.md#converting-the-films-on-windows). Without them the game starts at the main menu and the left sheet stays plain parchment.

Your settings (autosave, info window, volumes) and your campaign progress are kept where the original game keeps them, in its profile file `Documents\S2\users\<profile>.cfg`, so both games share them. The window opens at that profile's screen resolution. The first time OpenS2X changes the file it keeps the original as `<profile>.cfg.bak`.

**Exit** on the main menu asks first: **Yes** ends the program, **No** keeps the main menu. **Single player** opens the single-player page: on the scroll at the right **Continue** (loads your profile's autosave, `Documents\S2\saves\Autosave` (see "Saving and loading"); greyed while there is none), **Load Game**, **Campaign**, **Tutorial**, **Freeplay Game** and **Back**; it opens on **Campaign** (the island map), and the left sheet shows the page you pick. **Multiplayer** on the main menu does nothing (no network game).

- **Campaign**: the ten missions on the campaign map. Mission 1 is open; each mission you win opens the next one (the progress is kept with your settings). The briefing comes after 25 seconds of play; a mission is won by reaching its portal; the victory book then tells the end of the story, and **Exit map** leads back to the campaign map.
- **Tutorial**: four lessons (basic production, mining, military, seafaring), each a button beside its medallion. The tutorial window says what to do: tall at the top with **Continue** while it explains, wide at the top right while it waits for your click; **Cancel** ends the lesson. An orange arrow points at the button or the build menu's cell to press, a pointer marks the field to click, and a wrong click shows what was expected in red. **Play** at the end keeps the game running, **Exit map** leaves it.
- **Freeplay game**:

1. Click the map's bar to pick a map; its preview, size and players show below it.
2. Start resources, condition for victory and fog of war in the drop-downs at the right.
3. One row per player: you first. For the others the middle drop-down sets computer or closed, the portrait the tribe (Romans, Nubians, Chinese), the emblem the strength; the top and bottom drop-downs the headquarters and the team.
4. **Start**.

## In the game

The bar at the bottom centre is the original one:

| Button | What it does |
|---|---|
| Mission | the mission panel: the condition for victory |
| Speed | game speed x1, x2, x3 (a running settler with a clock shows at the top right above x1) |
| Construction view | the build-help icons on every field you can build on, and the names of your buildings with their productivity |
| Statistics | the statistics pages (also C): every player's land, buildings, inhabitants, goods, military strength, coins, productivity and soldiers killed as curves in the players' colours, one sample a minute (click a portrait to hide or show that player's curve); what your buildings made per minute; your buildings by type. What you choose stays until you quit |
| Minimap | the map; click it to move the camera |
| Economy | the tools, military, distribution and transport dialogs |
| Messages | your reports |
| Chat | not available (no multiplayer) |
| Game menu | save, load, settings, exit; close it with its cross or Escape |

Click on the map:

- a **free field** of your land: the build menu (tabs by size; the flag button at the top right, greyed where no flag fits); a field that takes only a flag opens the road menu with "Erect flag" alone;
- one of your **flags**: the flag menu, the mouse pointer on its first button:
  - **Build road**: click field after field; each click lays the road along the best way to that field. A click on a flag finishes it, a click on the road's end offers "Erect flag" there, a click on a part already laid cuts it back to there, a click on the starting flag cancels.
  - **Build waterway** (only at a flag on the coast): click a coast field across the water.
  - Destroy the flag, call a **geologist** (he digs at ten spots around the flag and puts up a sign for what he finds) or call an **explorer** (he uncovers the land around the flag);
- one of your **buildings**: its window (production on / off, destroy; what it needs and has, such as a mine's `2/2` of each food; headquarters and storehouses: stock and stock settings; the headquarters' military page: a row per rank, Private to General, with a slider for the reserve it keeps and how many are there; military buildings: coins, soldiers out, upgrade; a construction site: what it still needs);
- a **road**: put a flag on it, or remove it;
- an **enemy military building**: the attack dialog.

The right mouse button cancels a road being built; it and Escape close the open dialog. When the game refuses an order (a flag next to a flag, a road with no way) the reason shows in red above the bar for five seconds. The pointer's field shows an icon for what fits there (flag, hut, house, castle, mine, harbour; a crossed house where nothing does); while a road is built it shows whether the road can reach the field and whether a flag fits.

Buildings that idle show the original's floating icons: a "Z" over a workshop whose productivity has stayed below 25 %, a gear with a stop mark over one switched off, coins with a stop mark over a military building whose coins are stopped (four icons rising one after another), a soldier's head when soldiers are sent out.

Point at a building, flag or settler for its name (a building site says "(Under construction)", a finished workshop its productivity). The messages window (N) lists what happened: the four round buttons filter it, a left click on a line jumps there, a right click deletes it.

Keys, as in the original: SPACE construction view, T mission panel, V game speed, C statistics, M minimap, L economy dialogs, N messages, P pause, H or HOME back to the headquarters, Escape the game menu, F12 a screenshot (into a `screens` folder beside the program). The camera: arrow keys scroll, the middle mouse button drags, the wheel or PAGE DOWN / PAGE UP zoom, INSERT / DELETE turn.

## Settings

**Settings** on the main menu, or in the game menu, opens six pages, one per tab: **Graphics** (resolution and display mode: shown, but OpenS2X keeps its window as it is), **Engine** (the graphic quality presets and the detail sliders), **Sound** (four volume sliders), **Input** (scroll speed, mouse scrolling, border scrolling, the smart cursor and the free camera), **Game** (autosave, the information window, day and night) and **Network** (call sign and internet speed: there is no network game, so they change nothing). **Reset** sets a page back to the original's defaults (the Graphics page has none). What you choose is kept in your profile, shared with the original. The close button or Escape closes the dialog.

## Saving and loading

**Save**: Escape (or the bar's game menu button), **Save Game**. The name proposed is the date, the time and the map, as the original's, ready to be typed over: letters, digits, spaces and `!&()+-_` and the accented letters of the original's list go in, nothing else. **Save** or RETURN writes the game and goes back to it. A save of the same name is replaced without a question. The list is in the order Windows gives, by name, not by date. **Delete** works on the row you clicked, while the name still is that row's, after "Remove this file?".

**Load**: **Load Game** in the game menu, or on the single-player page; pick a save, **Load**. The game goes on where it was saved: every settler, carrier, ware, road, building and computer player as they were, the same game to the tick. The camera starts at your headquarters, as in the original.

**Autosave**: Settings, Game, **Autosave** (Off, 1, 5, 10, 20, 45 minutes, 1 hour of game time; the original's setting, shared with it). The game is saved as `Autosave` every interval. **Exit to Title** and **Exit to Desktop** ask first; the **Autosave...** box in that question saves the game as you leave, whatever the interval. It is off until you click it, and stays as you left it. **Continue** on the single-player page loads it.

Where they are: OpenS2X's saves are in the original's folder, `Documents\S2\saves`, as `<name>.s2save` (the original's own `.sav` files stay as they are: the original never sees ours, and ours never overwrite its files, its `Autosave.sav` included). The load list shows both kinds; when an original save has the same name as one of ours (usually the autosave), the original's is marked "(S2)". Continue takes the newer of the two autosaves.

Saves survive updates: from version 0.6.0 on, a save loads in every later version of OpenS2X. It plays on under the newer version's rules, and is written in the newer format when you save it again. A save from before that build, or from a version newer than the one you are running, does not load: a notice says "SAVE NOT LOADED" with the formats (or why else it did not load: a damaged file, its map gone). The game you had stays as it was, behind its game menu (the main menu when no game was running). A save of a program built on OpenS2X with rules of its own (a mod) does not load in OpenS2X, nor OpenS2X's in it: the notice names the program that saved it (OpenS2X by its earlier name, s2-reborn).

**The original's saves** load too (`.sav`): the villages come back with their buildings, construction sites, warehouse stocks, flags, roads, land and explored area, the workers in their buildings, the soldiers in their military buildings with their ranks and health, the carriers and donkeys on their roads, the wares at flags and in carriers' hands with where they are going and the orders they serve, a ship being built, the statistics and the random generator, and the computer players play on with their strength and their own plans (what they still need, their cells, where they want to expand). They build little at first in a game short of planks and stone, as they would anyway. Not taken over from them yet: the settlers on their way, builders and specialists (new ones come out of the warehouses), where idle carriers stood (they stand on their roads' idle fields), finished ships and a mission script's variables. A save with a part the program cannot read loads the rest and says so in a notice; the game waits until you close it.

## Playing the campaign

**Single player**, **Campaign**: the island map. An eagle stands on each island you may play: gold for a mission you have won, silver for the next one. A click on it starts the mission. Tutorial lessons start the same way from the **Tutorial** page. Nothing carries over from an earlier game.

After 25 seconds of play the mission's briefing opens over the game (**Next Page**, then **Continue**). Mission 1 has a second one when your land reaches the middle of the island. The game runs on behind them. **T**, or the bar's mission button, shows the condition for victory.

**Winning.** Every mission is won by reaching its portal: your land has to cover the portal's field. Mission 1 has no enemy and no computer player, and of the military buildings only barracks may be built, so lay a chain of barracks along the land with roads back to your headquarters. A scripted player does it with 8 barracks in about 45 minutes of game time, without any production (the headquarters starts with 44 planks; the 8 barracks use 16 of them). In most other missions the portal stands in a small pocket behind steep ground that no road reaches: a military building close enough claims it across the slope. The check comes every 5 seconds of game time. The victory book opens, **VICTORY** over the mission's last pages: **Continue** plays on, **Exit map** leads to the island map, with the next mission's eagle there.

**Which missions are known to be winnable.** The computer players are now as each map sets them: their tribes, teams and strengths, so from mission 4 on the enemies never fight each other, and in mission 5 the African player is your ally. A scripted player, with a player's commands only, has won missions 1, 2, 3 and 9, missions 3 and 9 only with a computer player's help on your own slot. It has not won missions 4, 5, 6, 7, 8 and 10; it has no food and mine economy of its own, so that says more about it than about the missions.

**Your progress** is kept in your profile (`Documents\S2\users\<profile>.cfg`), the list of maps the original keeps too, so a mission won in one game is open in the other.

**Losing.** A player who loses the headquarters has lost. The defeat book opens: **Exit map** leads to the main menu and nothing is recorded. A computer player that loses its headquarters only brings a message: the mission goes on until the portal is reached.

**Saving** works as in any game. A mission saved and loaded keeps its script's variables, its goal, the triggers still to come and the locked buildings, and plays on to its victory as if it had not been saved. A book that is open when you save does not come back.

**Good to know** (the campaign, and the game in general):

- **Expeditions:** a harbour's window has the expedition page: the builder, stones, planks and ship set aside so far, **Exploration** to start a preparation (greyed while one runs: as in the original nothing stops it) and **Jump to next expedition**, which goes to your expedition ship nearest that harbour and opens its window.
- Every mission's goal is "Reach the portal", missions 4 to 10 too (their maps say none, but the campaign menu sets it, as in the original). A free game plays with the victory condition chosen on its setup page.
- A free game's end screen and its button column are our own layout until the original's is known; its **Statistics** button is off.
- The tutorial moves the camera to the next marked field at 16 steps, as the original does: a glide at zoom 3 during which the arrows, wheel and right button do nothing. In lesson 4 the ship window's right arrow jumps to the second ship.
- All ten missions and four lessons start, show their briefing or window and run 3,000 game ticks (five minutes of game time) without a fault. All four lessons have been followed click by click to their ends. At 1024 x 768 the lesson's window can lie over a marked field: scroll the map to find it (the window's place is our guess).
- Sounds pan between the speakers as the original's sound library pans them (an equal-power law), and a settler's sounds move with him. The original's listener leans a little to the side (a slip in its code, reproduced), so a sound in the middle of the screen is heard slightly to the right.
- **RETURN** opens the chat line (also when you play alone): type `INeedSomeHelp`, `ShowMeTheWorld` or `GiveMeSomeMore` and RETURN for the original's cheats. `TooFast` and `FreedomOfMovement` are known words whose effect is not, so they do nothing. Where the line sits is ours (over the bar).
- A ship's name: what you type in its window stays only until the window shows another ship or is closed, as in the original; the ship keeps its default name.
- The statistics pages' series and span are drop-down lists. The buildings page lists the 31 types that have a building cost, by size; each type shows its finished buildings and those under construction, updated every 22 game ticks.
- On level ground no footstep sounds at any zoom, as in the original by its code; they belong to following a settler with **O** and to steep ground at zoom 0. **O** puts the camera just behind the settler under the pointer; **PAGE DOWN** or the wheel up turns it to face him from ahead, **PAGE UP** or the wheel down goes back behind him and, from behind, ends the follow. The zoom plays no part while following, and the arrows do nothing.

## Not there yet

- Saving in the original's format (OpenS2X writes its own, see above).
- Multiplayer.
- The tutorial's "Info" pages and its spoken end.
- The ending film.
- Sound plays on Windows only.

The open points against the original game are listed in [specs/OPEN_QUESTIONS.md](https://github.com/Jensenzo/OpenS2X/blob/main/specs/OPEN_QUESTIONS.md), and how close OpenS2X is in the [README](https://github.com/Jensenzo/OpenS2X#how-close-it-is-to-the-original).

Something wrong, or different from the original? [Open an issue](https://github.com/Jensenzo/OpenS2X/issues/new/choose); a save, or a screenshot of each program, helps most.

## Building it yourself

The source, and how to build and test it: the [README's "Build from source"](https://github.com/Jensenzo/OpenS2X#build-from-source). `s2-viewer.exe`, built from the same crate, is the developer's map viewer: the same game with keyboard panels (`--debug`) and many options (`--help`).
