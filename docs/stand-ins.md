# Stand-ins

Where a spec leaves a detail open, the program does something that works
and says so. Such a stand-in is not the original's behaviour. Each one is
marked in the code with the word "stand-in", and with the question that asks
for the answer where there is one, so a search finds them all (it also finds
a few test helpers and placeholders that use the word):

```bash
grep -rni "stand-in" crates
```

Once the investigation answers, the answer replaces the stand-in and its
mark goes. This page describes the larger groups, by question; the search
above lists every one.

## Q-256

| Part | What is open | What the program does | Where |
|---|---|---|---|
| a | Which control opens Select Map (UI-MENU-2.1) | the free play page's map button of UI-SETUP-1.8 | `menus/menu/pages.rs` (`free_play`) |
| a | Where the title bar's close button sits | UI-GAME-1.8's (w - 40, 5) on the 506 x 540 XXL panel | `menus/menu/pages.rs` (`select_map`) |
| a | The layout of the `!INVALID_MAP` message box (dialog 38) | the small panel (288 x 88) centred on the dialog, the text at (15, 40), a close button | `menus/menu/pages.rs` (`invalid_map_box`) |
| d | Which field is a good's slot number on the depot and inventory pages (UI-GAME-8.1, 8.5c) | its `gp_setSort` value ("position in sorted UI lists"; wares 0 to 30, settlers 0 to 29) | `ui/game/mod.rs` (`Catalog.good_slots`) |
| e | The style of the save dialog's name input (UI-MENU-5.1) | style 0, 256 x 20 | `menus/menu/dialogs.rs` (`save_load`) |
| e | What ESC's "clears the result" does (UI-MENU-5.3) | the input ends and the old text stays (a ship keeps its name) | `ui/widgets.rs` (`edit`), `ui/mod.rs` (`type_text`) |
| e | Whether a ship or save name has a length limit | none | `ui/mod.rs` (`type_text`) |
| f | Whether `graphics.xml`'s sun size 30 and moon size 9 are the quad's side or half of it (GFX-VIEW-6.3) | the side (the sun lies below the horizon either way) | `render/world3d/mod.rs` (`sky_sprite`) |
| g | The pose of the ship in its portrait (UI-GAME-4.5 a) | the rest pose, heading 0, at the ship's place | `render/ship_view.rs` (`aim`) |
| g | Whether "the field-of-view angle as the camera stores it" is the full vertical angle or half | the full vertical angle | `render/ship_view.rs` (`aim`) |

## Q-345: object numbers

SIM-START-4.2 to 4.5 (Q-340, Q-261 answered 2026-10-23; Q-263 answered
2026-10-24: production records, the split, boat carriers, produced wares)
say which creations take an object number, when, and where the counter
starts. What they leave open is Q-345. Some hidden numbers also take their place in a tick differently from
the original's; no settler's or building's number differs by it
(`crates/ai/tests/object_numbers.rs`, T6).

| What is open | What the program does | Where |
|---|---|---|
| When a builder leaving a ship at a landing is numbered (Q-345) | at the landing | `navy.rs` |

Hidden numbers in another place within the tick (ours, not open):

| The original | The program |
|---|---|
| a building's records (cell, production, need) between the building and its flag (4.4 b) | the computer player's records at the end of its part, before its commands take effect; those of a site it placed in its next part (it announces its buildings in its update, AI-MISC-2.2) |
| a computer player's road stub just before its road (4.4 c) | at the end of its part, with its records |
| the castle cell 90 181 before the castle's flag | after the flag, at the activation |

## Missions (Q-187)

| Q | What is open | What the program does | Where |
|---|---|---|---|
| Q-187 i | The strength of a lesson's computer players (UI-SETUP-2.4 sets only the kinds) | profile 1, every start position a player | `s2_rules::mission::players` |

## The chat line (Q-258 e)

Q-258 is closed with the empty line's look (UI-GAME-12.1); a typed line and
other screen sizes were not seen.

| Q | What is open | What the program does | Where |
|---|---|---|---|
| Q-258 e | UI-GAME-12.1: how a typed line looks; whether the box moves with other screen sizes | the text starts at the empty line's cursor place with the caret in it; the box stays at (15, 275) on every screen; `TooFast` and `FreedomOfMovement` (CAM-3.7 gives no effect) do nothing | `ui/game/mod.rs` (`chat_line`, `cheat_of`) |

## Models and poses (GFX-OBJ-1.4)

| Q | What is open | What the program does | Where |
|---|---|---|---|
| GFX-OBJ-1.4 (T32) | The making order of what the game made before the view first drew it (a game played ahead by `--run`, as the look comparison and the golden pictures do) and the list order of a save of ours | the flags in the order placed; the deposits not in the file field by field; then the signs. A save of ours lists its deposits field by field | `world/scene3d/objects.rs` (`load_poses`) |

## Saves (Q-257 b to e, Q-1033)

| Q | What is open | What the program does | Where |
|---|---|---|---|
| Q-257 b, e | FMT-SAV-3.5a: a leveller at work in an original's save (the spec leaves it open; no sample has one). A hunter at work is specified, but no sample save holds one, so that part is untried | Every worker state the spec gives goes on where it was: outdoor workers at their spot, on their walk out or back, waiting inside or producing inside, builders in their unit or on their approach (`sim/restore.rs`). A hunter goes on with his wait, his walk to his animal or his work at it, the animal held for his hut; one walking to his flag is seated inside as one who has arrived, as is any worker whose saved walk does not hold together. Levellers at work are not restored: the economy sends new ones. Carriers stand on their idle fields, except one holding a ware, which goes on from its saved field (Q-257 a) | `sim/restore.rs`, `sim/production` (`resume_outdoor`), `sim/construction.rs` (`resume_builder_walk`), `rules` (`restore_game`) |
| FMT-SAV-2.2 | The saved numbers of roads, flags' network entries and networks | the rebuild draws new ones from the saved counter on; buildings, workers, soldiers and carriers keep theirs | `sim/restore.rs` (`numbered`) |

## Ours, where the specs say nothing

- **Our save files** (`game/save.rs`, `PLAYING.md`): the program saves
  its own game as our snapshot (`docs/snapshot.md`) with a header (the
  map file, the tick, the world's and computer players' state hashes,
  the computer slots, the mission's interface state), as
  `<Documents>\S2\saves\<name>.s2save`, beside the original's `.sav`
  files, which it never writes (the original lists only `*.sav`, so it
  never sees ours). The load list shows both kinds; an original's save
  whose name one of ours has too (the autosave) is marked "(S2)", and
  CONTINUE (UI-MENU-1.5) takes the newer of the two autosaves. A save of
  another save or world format is refused with a notice naming the
  formats ("SAVE NOT LOADED"), as is a damaged one or one whose map is
  gone; the game on screen is left as it was. A save is written to a
  `.part` file first and then renamed. The camera is not saved: as
  GFX-VIEW-3.6, a continued game starts at the headquarters. A mission's
  script is loaded again without its start callbacks, then its saved
  variables are set (CAM-1.1, FMT-SAV-3.12): which variables the original
  keeps is Q-100 b; ours keeps every global the script set to a whole
  number from 0 to 2^32 - 1 (the spec's `u32` values), not booleans,
  strings or tables.
- A save that does not read whole (`s2_assets::sav::SaveGame::problems`):
  the reader keeps what it read, notes the record and offset that failed,
  and goes on at the next section it finds; the game continues from what
  was read, paused behind a notice that names the first part left out
  (`menus::menu::Notice`). A save without a readable map part does not
  load; the notice says so over the main menu. What the original does
  with such a save is not specified.
- **The Graphics settings page** (SET-5.2, `menus/menu/dialogs.rs`
  `graphics_page`): the original lists the display device's modes and
  switches between full screen and a window. Our game is a window whose
  size comes from the command line or the desktop, not a device mode: the
  page shows its size (`<w> x <h>`) as the only RESOLUTION entry and MODE
  Windowed, and a choice changes nothing. The Network page (SET-5.7) shows
  the player's name as CALLSIGN (not an input) and INTERNET SPEED NONE;
  nothing on it changes a single-player game, as in the original.
- **A project's pages** (`menus/menu/dialogs.rs` `extension_page`,
  `docs/extending.md`): the original has no such page. They sit on the
  XXL panel (506 x 540) with BACK at (50, 480); the game's own settings
  pages follow UI-MENU-4.0 (`uiDepot`, the tabs, no BACK).
