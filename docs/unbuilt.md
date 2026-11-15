# Specified but not built

The input for the next runs: items of the original that `specs/INVENTORY.md`
links to a spec section of **high** confidence, and that our program does not
use yet. Made on 2026-10-18 from the investigation's inventory of the install
(written into a scratch folder, so `specs/INVENTORY.md` was not touched): 4205
items, 1437 with a spec section, **1033 high**.

## How each item was checked

1. Every high item's names (its own name, the file name and stem of its source)
   were searched in our code (`crates/**/*.rs`, the shaders, `scripts/`), not
   counting test modules, `tests/` folders and comments. A quoted literal
   counts (`"!Game Menu"`, `"door_open"`, `"buildings.xml"`).
2. Where the only hit is the file that defines a whole class, the item counts as
   used when our loader takes every entry of that file and the program draws or
   runs it from the loaded data, not by name. These loaders:
   - the property scripts, run whole by `s2-rules` from `data.lua` (all 33
     building types, 67 goods (wares and the 30 professions, `Jobs.lua`), 38
     deposits, 112 doodads, 3 animals, the street, 104 sounds, the AI profiles
     of `ai.lua`): `cargo run -p s2-app -- rules`;
   - `buildings.xml` (every building model, its `DoorOpen` / `DoorClose` clip,
     174 clips, played by `view3d::animate_buildings`), `settler_config.xml`,
     `items.xml`, `animals.xml`, `ships.xml`, `terrain_static_data.xml`
     (`s2-assets::model_tables`, `terrain_patterns`);
   - `settler_animations.xml` with the name rule of GFX-ANIM-1.2
     (`settler_anim::resolve`: `Stand` + `Wood` = `StandWood`, `Hit` + `Sword`,
     `WorkLoop0` + a profession), so every clip variant there is reachable;
   - textures named by the models (`.GR2` materials, the `_L` light maps of
     GFX-OBJ-2.1) and by the terrain patterns a map uses; the music lists by
     tribe (`sound::playlist`); the `.fnt` font pages.
3. The rest (about 120 items) was looked at one by one.

So "used" means the item reaches the program. It does not mean every rule of its
spec section is built; that is what the sections' own tests check.

## Not built

Updated 2026-10-18, when the list was built, and 2026-10-20, after the
answers to Q-225, Q-227, Q-228, Q-229 and Q-307 were built. Struck items are built;
what the build left as a stand-in is in the question named.

### Interface

| Item | Spec | What is missing |
|---|---|---|
| ~~`Inventory` dialog~~ | UI-GAME-8.5, 8.5c, 7.1 | built: goods and settlers pages of all warehouses, slots by slot number (read as `gp_setSort`), counts right-aligned; the 512 x 536 family frame art and the slot number's field are Q-256 c, d |
| ~~`Fleet` dialog~~ | UI-GAME-8.5 | built: the finished ships by name in an 11-row list at (50, 85); a row selects the ship (its window, the camera) and closes the fleet; the list box's row height is Q-185 c; names wait for ship names in the game state (below) |
| ~~`Select Map` dialog~~ | UI-MENU-2.1 | built: no OK / Back, the map under the pointer previewed, a click takes it and closes, an unreadable map shows `!INVALID_MAP`; the opener, the list's scrolling, the close button and the message box layout are Q-256 a |
| ~~`uiShipMask.dds`~~ | UI-GAME-4.5 | built: the ship model alone, rendered once into 128 x 128 by the portrait camera, through the mask; the pose and the angle are Q-256 g |
| ~~`uiKey0.wav`~~ | SND-4.1, UI-MENU-5.3 | built: text inputs with their bar art, caret placed by the click, `|` / `.` caret, LEFT / RIGHT / HOME / END / DELETE / BACKSPACE; the save input's style is Q-256 e |
| ~~Production statistics page~~ | UI-GAME-8.5b, 8.5e, 8.5f, `dialog28_0` | built (2026-11-05): the fourteen counters, the coin total and the frags in the game (snapshot 66), toggles, spans, `!Unmark all`, the graph by 8.5f; (2026-11-05 night) the counters read when opened, on a toggle, `!Unmark all`, a change of span and every 601 page updates, `STATS_MENUOPENED`, the toggles' hover picture, marks and span kept for the session. Still a stand-in: the option groups' arrangement (Q-258 f; a column here) and the top label's alignment |
| ~~Player statistics page's graph~~ | UI-GAME-8.5a, 8.5f | built (2026-11-05 night): the production page's routines at the canvas, one curve per player in the game (his button enabled), M over their whole history; the series and span kept for the session, `STATS_MENUOPENED`. The curves' colours by player number are Q-1016; the players' names and colour index are Q-258 f |
| ~~Player statistics page layout~~ | UI-GAME-8.5a | built (2026-10-20): image, players' row, series and span options; the graph's drawing (Q-096), the options' arrangement and the players' names are stand-ins (Q-258 f) |
| ~~Buildings overview / "Size of village and productivity" page~~ | UI-GAME-8.5d, 7.1, 12.1 (B key) | built (2026-10-20): a cell per type with its counts and mean productivity, its tab, Planning row and B; which types, their order and the counts' order are Q-258 a |
| ~~Ship names in the game state~~ | UI-GAME-4.5 c | built (2026-10-20): `Command::NameShip`, the ship's name in the game (snapshot version 16), the default `<Tribe>ShipName<k>`; the commit path is Q-258 c |
| ~~Save dialog: `!Save`, `!Delete`, `!Remove this file?`~~ | UI-MENU-5.1, 6.1 | built (2026-11-02): our own save files (`game/save.rs`), the delete question with Yes / No; the overwrite, the list order and the name's characters are Q-298 |
| ~~`!Saving...`, `!Disk Full!`~~ | UI-MENU-5.1 | built (2026-11-06 night): `!Save` or RETURN hides the controls and shows `!Saving...` at (50, 50) for the update before the game is written; under 5 MB free on the save drive (read when the dialog opens, Windows' free bytes for this user) `!Disk Full!` shows and `!Save` and `!Delete` are disabled. Where `!Disk Full!` sits, MB as 2^20 and when the space is read are Q-1019 |
| ~~`!Autosave...` message (kind 7)~~ | FMT-SAV-5.2, UI-GAME-11.1 | built (2026-11-06 night): the periodic autosave posts it to the human player before it saves; the window lists it with the simulation's messages by arrival, always shown, for 36 000 ticks; it has no field, so a click on it moves no camera. It is the interface's own (not in the save) |

### Graphics

| Item | Spec | What is missing |
|---|---|---|
| ~~`map/textures/caustics.tga`~~ | GFX-WATER-2.5 | built in the terrain shader: V twice, drawn at shader detail 2 or more (SET-4) |
| ~~`map/objects/street_buoy.GR2`, `street_buoy.dds`~~ | GFX-OBJ-6.2b | built: a pair per step at the curve's midpoint, 1.7 to each side, not turned, no strip |
| ~~`enviroment/sun.tga`, `enviroment/moon.dds`~~ | GFX-VIEW-6.3 | built: the quads along the light's travel direction, so the sun lies below the horizon and is never seen (confirmed, Q-229 d); size as side or half is Q-256 f |
| ~~A settler's "seen" mark~~ | GFX-VIEW-5.4 (T36, T36a) | built (2026-11-08): every settler's mark (one per settler, by its object number, kept while it changes jobs and while it is inside a building) starts clear and is refreshed from the local player's exploration layer (all explored with fog of war off) in a frame that draws its field, or the renderer's first, while it walks (a leveller too, though drawn standing) or the frame is forced; any player's exploration (`World::explorations`) forces the next frame; a settler is drawn only while marked. A game a tool plays ahead (`--run`, `--until-built`, scenes, the look comparison) keeps the marks with the camera on the start view for every tick played, then the view's first frame is the jump (5.4 g, 2026-11-08; the earlier stand-in that refreshed every settler is gone). Not built: (f), the pass only while the drawn field rectangle lies inside the map (Q-1031) |
| ~~A finished building's collapse (`DemolitionDust`, the house sinking by its box height x clock squared)~~ | GFX-OBJ-2.2a (3 b), 8.7 | built (2026-11-12): the house stays without its decal, icon and flags and sinks by H x c^2 (c 0.35 a second of frame time) until its slot is freed; `DemolitionDust` starts once at its position; the collapse sound for a finished building's removal only (not a capture); the ruin `<tribe>_<remains>` on a copy of the building's decal in the site frame's pass, while the remains burn. Since 2026-11-12 night (8.7 (2), T39) the clips playing at the removal go on while it sinks (an opening door runs to its end, the Working clip to its end), nothing starts and it is not reset to its rest pose; since 2026-11-13 night (T43) its production animals, as many as showed, loop on its bones and sink with it, and the Working clip's sounds still to come fire at their times (no door sound) |

### Found on the way

- Fixed: N and P (UI-GAME-12.1 messages, pause) also reloaded the next or
  previous map in a running game; map cycling is now the map viewer's only.
- The play harness could not drive the menus; it now has menu steps (a
  step marked `menu` runs before the game, its deadline in frames).
- UI-GAME-12.1 keys: S (settings at their last page), B (buildings
  overview), O (follow a settler) and the `.` and `,` jumps are read since
  2026-10-20; RETURN opens the chat line, where the cheats are typed,
  since 2026-11-06 (its look is a stand-in, Q-258 e).

## Not counted as unbuilt

### Matched by a common word, no rule for the item

The inventory found these through a word of their name in an unrelated rule; no
spec section describes them, so there is nothing to build from yet (the
investigation side may want to look):

| Item | Linked section | Word |
|---|---|---|
| `Buildings/smeltery/loop.wav`, `Buildings/windmill/loop.wav` | ECO-OUT-3 | loop |
| `Buildings/windmill/end.wav`, `Buildings/shipyard/end.wav` | AI-CON-8 | end |
| `Buildings/Catapult/Fire.wav`, `scripts/effects/Fire.pfx` | ECO-FOOD-2.1 | fire |
| `Buildings/Catapult/Impact.wav` | MIL-CON-5 | impact |
| `groundcover.fx` | GFX-PAT-1.1 (only the pattern's `groundCover` attribute; no drawing rule) | |
| `items/dummy_item.tga` (`DonkeyItems`, Wood) | GFX-OBJ-1.6 (named, no rule for when it is drawn) | |
| `map/textures/grass.dds` | GFX-OBJ-1 | grass |
| `gfx/cursor.tga` | AI-CON-8 | cursor |
| `system.fnt` | AI-CON-1 | system |

The building sound files are reached when a clip event names their sound
(SND-2.2, `audio`), so some may already play.

### Not used by the original, or left out on purpose

- `uiButtonDefault.dds`, `uiPushButton.dds`, `uiTitleButtons.dds`: UI-MENU-1.3
  says the shipped build does not use them.
- `Ubisoft.bik`, `BB_Logo.bik`, `funaticsAlpha.bik`: UI-MENU-0.1, project
  decision: no company logos.
- `extro.bik`: its subtitles and tracks are specified (UI-MENU-0.3, 0.3a), but
  when it plays is medium (UI-MENU-0.3a, "medium that the second caller is the
  extro"), so it waits.
- The game menu's title and close button at (188, 14) and (336, 5) of the frame
  (UI-MENU-3.1b, medium) and the START button's place (UI-SETUP-1.8, medium).
  (The option drop-down's caption anchor, Q-331, was answered 2026-11-09 and
  is built: UI-SETUP-1.8, T15.)

### Text lines (UI-MENU-1.8, 2026-10-20)

Every text is now placed with its line box's top-left on its anchor; centring
and right alignment move it only in x, by half or all of its run (no
kerning). Labels that were centred vertically before (the building windows'
headings, the construction view's labels) now hang from their anchor.
