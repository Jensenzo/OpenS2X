# FMT: Install layout

Status: draft
Owner question(s): Q-001 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

The game ships as loose files: a `bin/` folder with the executables and middleware
libraries, and a `data/` folder with every asset, map, text and rule table. There
are no archive files, but most data files are individually wrapped in one common
container that must be decoded before the inner format (Lua, XML, DDS, ...) can be
read. All simulation code is in the main executable; the numbers it runs on (costs,
times, limits) are supplied by Lua tables under `data/scripts/properties`.

This spec is a survey. It says what exists and what needs a loader. Inner formats
get their own `FMT-*` specs.

## Rules

### FMT-1 Top-level layout

- **FMT-1.1** The install root contains `bin/` (programs), `data/` (all game
  content), documentation, and one loose editor file `brushes.dat`. The game is
  started from `bin/` as working directory, so data paths resolve as `..\data\...`
  relative to the executable folder.
- **FMT-1.2** There are no pack or archive files. Every asset is addressed by its
  relative path under `data/`. Path lookups are case-insensitive (the original
  mixes `Data\` and `data\`, and `.GR2`/`.gr2`).
- **FMT-1.3** User-writable state (saves, logs) is not under the install root. It
  lives in a folder named `S2` under the user's documents folder and under the
  local application-data folder.
- **FMT-1.4** Only one language folder ships in this build: `data/language/gb`.

### FMT-2 The common file container

- **FMT-2.1** A wrapped file starts with the 4 bytes `12 18 09 06` followed by the
  ASCII tag `rc00`. A file without this 8-byte prefix is stored plain.
- **FMT-2.2** Bytes 8..15 are an 8-byte per-file value of unknown meaning. Bytes
  16..19 are a little-endian unsigned 32-bit integer equal to the size of the
  decoded content. The payload starts at offset 20.
- **FMT-2.3** The payload is compressed and not directly readable. The decoding
  scheme is not yet specified (Q-006). A loader must check for the prefix on every
  file it opens and route wrapped files through the decoder.
- **FMT-2.4** Wrapping is decided per file type. In this build every file of these
  types is wrapped: `lua`, `cfg`, `xml`, `txt`, `s2m`, `gr2`, `dds`, `tga`, `fx`,
  `inc`. Every file of these types is plain: `ini`, `fnt`, `pfx`, `bin`, `lep`,
  `dat`, `bmp`, `png`, `wav`, `mp3`, `bik`.

### FMT-3 File categories under `data/`

- **FMT-3.1 Rule tables.** `scripts/properties/` holds 16 Lua scripts that define
  the game's content and tuning: `buildings`, `Goods`, `Jobs`, `config`,
  `military`, `deposits`, `doodads`, `patterns` (terrain types), `animals`, `ship`,
  `remains`, `streets`, `signs`, `sounds`, `ai`, `data`. Each script calls
  host-provided functions to create an entry and set its attributes. Lua version
  is 5.0.
- **FMT-3.2 Attributes known to be data-driven** (the host exposes a setter for
  each; values are not yet read):
  - Buildings: construction cost, construction time, production time, input
    goods, second and alternative inputs, output, worker type and worker limit,
    working radius, stock limit, soldier limit, territory range, explore radius,
    size class, health, depot / harbor / catapult flags, switchable production,
    remains after destruction, AI score and threat.
  - Goods: cost, type, group, value, transport priority, rest time, whether it can
    be planted, whether it occurs as deposit or field, AI priority.
  - Global config: game speed, settler speed, pickup time, drop time, construct
    time, bulldoze time, per-flag package limit, fishing distance, settler dying
    time, catapult flight and rotation time factors.
  - Military: fighting time, soldier max health per rank, building upgrades.
  - Terrain types: blocked, buildable, harbor-capable, minable, plantable, tree
    type, minimap colour.
  - Deposits and doodads: yielded good, growing speed, start size, lifetime,
    blocking.
  - Animals: movement speed, move distance, range, idle factor.
  - Ships: cost, construct time, size, speed, stock limit.
  - AI profiles: military mode, attack strength, allowed attack count, limits on
    certain building counts, toggles for catapults, harbors and route optimizing.
- **FMT-3.3 Maps.** A map is a `.s2m` file. Campaign maps are
  `maps/campaignMaps/campaign01..10`, each with a `.s2m`, a mission script `.lua`
  and a preview `.bmp`. Free-game and multiplayer maps are in `maps/freeGameMaps/`
  (20 maps, `.s2m` + `.bmp`, no script; names start `sp` or `MP_<players>P_`).
  Tutorials are `maps/tutorialMaps/tutorial01..04` (scripts for 03 and 04 only).
- **FMT-3.4 Mission scripts** are small Lua files next to the map. The host gives
  them queries about players (good amounts, has castle, reached or spotted another
  player, building productivity), map actions (explore an area, read territory
  owner, swap terrain type) and AI setup.
- **FMT-3.5 Settings.** `settings/` holds three plain INI files (`engine`,
  `network`, `properties_filterlist`), ten wrapped XML files that bind named
  entities to presentation data (`buildings`, `items`, `animals`, `ships`,
  `flocks`, `decals`, `daytimes`, `engine`, `graphics`, `terrain_static_data`) and
  twelve 688-byte binary lighting presets (`daytimes.bin`, `noDayCycle.bin`, and
  optional per-map overrides named after the map).
- **FMT-3.6 Tilesets.** `settings/properties_filterlist.ini` assigns each terrain
  type, deposit, tree and doodad name to one or more of the tilesets `Standard`,
  `Lava`, `Mediterran`, `Desert`, or to `DoNotUse`. It is an editor filter, not a
  runtime rule.
- **FMT-3.7 Per-entity info scripts.** `scripts/info/` holds 124 tiny Lua files
  named `b_<hex id>.lua` (41) and `i_<hex id>.lua` (83), loaded by id.
- **FMT-3.8 Text.** `language/gb/*.txt` (wrapped) holds UI and story strings;
  `language/gb/fonts/` holds bitmap fonts as a plain-text glyph table (`.fnt`,
  AngelCode BMFont text layout) plus wrapped TGA pages named `<font>_00.tga`.
- **FMT-3.8a String tables and font pages (2026-10-24).** Each `language/<lang>/*.txt`
  table (after the container, FMT-2) is text lines: the first line is the table's title
  and date (for example `Controls Keys<TAB>Stand 03.10.2006`) and holds no entry; every
  other line is `key<TAB>text`. In the text, `\n` is a line break; a leading `<hcenter>`
  centres the text (intro, extro); in `credits.txt` a leading `#` or `%` marks a heading
  line (its drawing is not extracted, UI-MENU-6.3). A key without an entry is shown as
  the key itself (UI-GAME-1.3). A font `F` is `fonts/F.fnt` (BMFont text: `info`,
  `common` with `pages=1`, one `char` line per glyph, no `page` line) and its only page
  `fonts/F_00.tga`: the game appends `_00.tga` to the font name. Confidence: high
  (shipped files; `_00.tga` in the executable). Covers (inventory): *UI texture* from
  `data/language/gb/fonts/*_00.tga`.
- **FMT-3.8b Glyph lookup (2026-11-24 b, Q-1088 a; E-ui-044, code reading, high).** A font holds
  256 glyph slots, one per byte value. Each `char` line of the `.fnt` file fills the slot of its
  `id` (its `x`, `y`, `width`, `height`, `xoffset`, `yoffset`, `xadvance`; `page` is read and
  ignored) and marks it present; ids are not checked against 255. To measure or draw a text, each
  byte of the text, taken as an unsigned number 0 to 255, selects the slot of that number: there
  is no other mapping (no code page conversion, no fallback glyph). A byte whose slot is not
  present is skipped: nothing is drawn and it adds no width. A present glyph adds its `xadvance`
  plus one font-wide spacing value to the width. So a Windows-1252 text byte 0x96 draws glyph 150,
  and a byte with no glyph in that font simply vanishes from the line. (A second path draws with a
  system font instead of the page when a global switch is on and the font's size reaches a limit,
  not followed here.) Not read: the
  byte stored for a typed key (Q-1088 b).
- **FMT-3.9 Graphics.** Models and animations are Granny `.gr2`; textures are
  `.dds` and `.tga`; shaders are Direct3D 9 effect files in `gfx/fx_20/`. Building
  art is per tribe and type in `buildings/PB_<R|N|C>_<Name>01/` (Roman, Nubian,
  Chinese; 31 types each). Particle effects are plain binary `.pfx` in
  `scripts/effects/`.
- **FMT-3.10 Audio and video.** Sound effects are `.wav`, music and voice are
  `.mp3`, videos are Bink `.bik`.

### FMT-4 Loaders the reimplementation needs

- **FMT-4.1** Required for the simulation: container decoder (FMT-2), Lua 5.0
  compatible host for `scripts/properties` and mission scripts, `.s2m` map loader.
- **FMT-4.2** Required for presentation: XML settings, `.gr2`, `.dds`, `.tga`,
  `.fnt`, `.txt` string tables, `.pfx`, lighting `.bin`, `.wav`/`.mp3`, `.bik`.
- **FMT-4.3** Not required: `.fx` shaders (renderer is ours), editor files
  (`.lep`, `brushes.dat`, `scripts/editor`), GOG metadata.
  These files are therefore specified as not reproduced. Confidence: high (a decision of
  this project). Covers (inventory): *shader effect* from `data/gfx/fx_20/*`. Covers
  (inventory): *other file* from `data/gfx/fx_20/*`, `data/editor/*`. Covers
  (inventory): *script* from `data/scripts/editor/*`.

### FMT-5 Module ownership

- **FMT-5.1** The main executable `bin/S2DNG.exe` contains all game logic,
  rendering, UI, the multiplayer protocol, the Lua interpreter and the container
  decoder. No game rule lives in a DLL.
- **FMT-5.2** The DLLs are middleware only: model/animation runtime, audio, video,
  XML parsing, network transport, lobby matchmaking, C runtime, crash helper.
- **FMT-5.3** `bin/S2DNGEditor.exe` is the map editor. It embeds the same engine,
  Lua host and property bindings.
- **FMT-5.4** Multiplayer is lockstep: the executable distinguishes a logic tick
  from a network tick and exchanges checksums to detect desync. (Tick lengths are
  Q-003.)

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| container_magic | `12 18 09 06 72 63 30 30` | bytes | high | E-install-002 |
| container_header_size | 20 | bytes | medium | E-install-002 |
| container_decoded_size_offset | 16 | byte offset, u32 LE | medium | E-install-002 |
| lua_version | 5.0.2 | | high | E-install-003 |
| tribe_count | 3 | | high | E-install-004 |
| building_types_per_tribe | 31 | | medium (art folders, not rule table) | E-install-004 |
| campaign_map_count | 10 | | high | E-install-004 |
| free_game_map_count | 20 | | high | E-install-004 |
| net_game_port | 5479 | UDP/TCP port | high | E-install-005 |
| net_broadcast_port | 6582 | port | high | E-install-005 |
| net_ping_interval_ingame | 2.5 | unit not confirmed (seconds likely) | low | E-install-005 |
| lighting_preset_size | 688 | bytes | high | E-install-004 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | Original install | Classify every file under the root by the 8-byte prefix | 1700 wrapped, 500 plain; split by extension exactly as FMT-2.4 |
| T2 | `data/scripts/properties/streets.lua` (136 bytes) | Read decoded-size field | 229 |
| T3 | `data/language/gb/fonts/mini_00.tga` | Read decoded-size field | 262188 (18-byte TGA header + 512x512x8-bit + 26-byte footer) |
| T4 | `data/settings/network.ini` | Parse as INI without decoding | `Basics.gamePort` = 5479 |
| T5 | `data/maps/campaignMaps` | Enumerate maps | 10 maps, each with `.s2m`, `.lua`, `.bmp` |
| T6 | Open `data\settings\ENGINE.INI` | Case-insensitive lookup | Resolves to `data/settings/engine.ini` |
| T7 | 3.8b: the two `credits.txt` (gb) lines holding the byte 0x96 (`cr170` "Hey – once again…" and `cr501` "Mathieu Mariolle – Bug Tracker", once each; corrected 2026-11-24 c), drawn in a font whose `.fnt` has `char id=150`; and the same lines in `mini` (glyphs stop at 126) | glyphs drawn and the lines' widths | first font: glyph 150 in each line, each adding its `xadvance` plus the font's spacing; `mini`: the byte draws nothing and add no width |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-install-001 | code reading | `S2DNG.exe` overview | the shipped executable is packed |
| E-install-002 | data file observation | headers of all 2200 files | Common prefix, size field checked against TGA font pages and three other files |
| E-install-003 | the program's strings | `S2DNG.exe` | Embedded Lua and zlib version strings; names of script host functions; data paths |
| E-install-004 | data file observation | directory listing of `data/` | Counts and naming patterns |
| E-install-005 | data file observation | `data/settings/network.ini` | Plain text values |
| E-install-006 | the program's metadata | import tables and version resources in `bin/` | Module roles in FMT-5 |
| E-ui-044 | code reading | | 3.8b: 256 glyph slots filled by `char id`, lookup by the unsigned byte, missing glyph skipped with no width (Q-1088 a) |

## Open questions

- Q-006 How is the common container payload decoded (compression, cipher, key,
  meaning of the 8-byte field)? Blocks reading every rule table and map.
- Q-007 Which rules are hard-coded rather than set from `scripts/properties`?
  FMT-3.2 lists what is data-driven; anything not listed (carrier choice, path
  finding, combat resolution, territory calculation) is presumed code.
- Q-008 What are the actual values in `scripts/properties/*.lua` (once Q-006 is
  solved), and what do `Jobs`, `streets`, `signs` and `data` define?
- Q-009 Save game format, file extension and exact location.
- Q-010 Layout of the 688-byte lighting presets and the `.pfx` particle format.
- Q-011 What do the hex ids in `scripts/info/b_*` and `i_*` identify, and what do
  those scripts set?
- Q-012 What do the XML settings files contain: presentation only, or any
  gameplay values (for example building footprints or door positions)?
