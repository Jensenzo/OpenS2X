# Extending the game

OpenS2X is the faithful game. A project built on it (in the examples
below called `my-mod`) changes or adds rules, assets and drawing **without forking it**, through the
extension points below. Without extensions every one of them is exactly
the original's: the golden hashes and golden images run with the defaults
and prove it (`docs/test-coverage.md`).

Such a project pins an OpenS2X version by tag (`docs/releasing.md`) and uses the
simulation only through its public API (`docs/api.md`).

## What a project can change

| What | How | Where |
|---|---|---|
| Rules: costs, production, jobs, goods, military values, anything the property scripts set | Lua run after the original property scripts, in their environment: it sees their globals (`woodCutter`, `stone`) and calls the same `properties` functions (LUA-PROP) | `s2_rules::Overlay`, `s2_rules::load_with_overlays`, `Extensions::rules_overlays`, `--rules-overlay <file>` |
| New building types | an overlay creates them with the original's `properties.bp_create` and sets them up as the scripts do theirs; what one product consumes is the type's own recipe | `reborn.bp_addInput` (in an overlay), `s2_sim::Rules::recipes`, `World::recipe` |
| The computer players and a new type | they leave a project's types alone unless the overlay says when one is worth building in place of a mine | `reborn.bp_setAiAlternative` (in an overlay), `s2_ai::AiConfig::alternatives` |
| A whole property script | a file of the same path in an asset folder (below) | `GameDir::with_overlay` |
| Models, textures, sounds, interface pictures, fonts, settings files | asset folders laid out like the install (`data/...`, any case); a file there takes the place of the install's file of the same path; the last folder wins | `GameDir::with_overlay`, `Extensions::asset_folders`, `--assets <dir>` |
| Textures by name | a `.dds` anywhere in an asset folder replaces the install's texture of that file name (models name their textures without a path) | as above |
| Drawing quality | shadow map size, shadow cascades, anisotropic filtering, background colour | `Extensions::render` (`RenderOptions`) |
| More on screen, more input | Bevy systems, resources and plugins added to the app before it runs | `Extensions::plugins`, `s2_viewer::bevy` (the exact Bevy the viewer uses) |
| Main menu entries and their pages | up to two buttons in the main menu's empty rows, each opening a page of captions, toggles, choices and buttons drawn like the settings pages | `Extensions::menu` (`MenuEntry`, `MenuPage`, `PageWidget`) |
| A line on the start screen and the main menu | a title over the game's title line; a footer under the menu's buttons | `Extensions::captions` (`Captions`) |
| The films at start-up | start at the start screen, without the intro | `Extensions::skip_launch_films` |
| Music | what the viewer's player switches to, as Bevy messages; its own player silent while a project plays | `MusicSlot`, `Extensions::music_external`, `CurrentSettings` (`Settings::music_volume`) |
| The camera | a point the 3D camera follows until the player moves it | `CameraFollow` (a resource) |
| What is on the map | the settlers, wares and animals under the pointer, nearest first, and where each is drawn, read only | `Extensions::map_things`, `MapThings` (a resource) |
| The window's title | the game window, its message boxes and the folder picker | `Extensions::window_title` |
| Screenshots | the folder the viewer's F12 screenshots go to | `Extensions::screenshot_dir` |
| Saves | a folder of the project's for its saves and autosave (the original's `.sav` still listed); a mark in each save of the project and its rules overlays, so no other program plays it with other rules | `Extensions::save_dir`, `Extensions::project_name`, `s2_ai::save::SaveMark` |
| Where settings are written | the original's profile file left unwritten, or a copy of it kept in a folder of the project's; the program's own file elsewhere; every change reported | `Extensions::settings` (`SettingsOptions`), `Extensions::on_settings_change` |
| The player's folders, the game folder | the documents folder, the profiles' folder and the install the game runs with, as the game finds them | `s2_viewer::paths::{documents, users_folder, game_dir}` |
| A different front end, a server, tools, an AI | the crates as libraries: `s2-sim` (commands in, state and events out), `s2-rules`, `s2-assets`, `s2-ai` | `docs/api.md` |
| A headless game, the one `s2-app hash` plays | a map by its name, every player a computer player; the same state hashes as `s2-app hash` (a drift alarm for a project) | `s2_assets::find_map`, `s2_ai::every_player`; `docs/api.md`, "A headless game" |

### Rules overlays

```rust
let dir = s2_assets::GameDir::from_env()?;
let overlay = s2_rules::Overlay {
    name: "woodcutter-costs-one-more-stone".into(),
    code: b"properties.bp_addCost(woodCutter, stone, 1)".to_vec(),
};
let (rules, report) = s2_rules::load_with_overlays(&dir, &[overlay])?;
// report.scripts ends with "overlay:woodcutter-costs-one-more-stone"
```

An overlay runs with the same sandbox as the originals (LUA-PROP-1.1: no
standard library, no files). Its errors name it (`@overlay:<name>`).

### New building types

An overlay adds a building type with the original's own functions:
`properties.bp_create("<8 hex digits>")` with an id no type or good has,
then its size, costs, construction time, worker, product, production time,
stock limit, sort, look and icon as the property scripts set theirs
(LUA-PROP-6). The type is in the game's catalogue as the original's are:
in the build menu, placed, built, staffed, working by the cycle its data
give it (ECO-CAT-3: a worker and a product that does not grow on the map
make a workshop), saved and loaded with the game. Nothing in OpenS2X's
tables names it, and none needs to.

What the original's functions cannot say is what the new type consumes:
a recipe belongs to the product (`gp_addCost(coal, wood, 2)`), so a second
maker of coal would change coal's recipe for the coal mine too, in the
computer players' and the production window's eyes. Hence the hook:

```lua
charcoalBurner = properties.bp_create("C4A4C0A1")
properties.bp_setName(charcoalBurner, "Charcoal Burner")
properties.bp_setGfxName(charcoalBurner, "woodcutter")      -- a look of the install's
properties.bp_setIconXY(charcoalBurner, 1, 1)
properties.bp_setSort(charcoalBurner, 9)                    -- small: 0 to 8 are taken
properties.bp_setSize(charcoalBurner, 0)
properties.bp_addFrameCost(charcoalBurner, plank, 1)
properties.bp_addCost(charcoalBurner, plank, 1)
properties.bp_addCost(charcoalBurner, stone, 1)
properties.bp_setConstructionTime(charcoalBurner, 442)
properties.bp_setWorker(charcoalBurner, good_helper)
properties.bp_setGoods(charcoalBurner, coal, 1)
properties.bp_setProductionTime(charcoalBurner, 666)
properties.bp_setStockLimit(charcoalBurner, 6)
reborn.bp_addInput(charcoalBurner, wood, 2)                 -- its own recipe
```

| Item | What |
|---|---|
| `reborn.bp_addInput(building, good, count)` | appends (good, count) to the type's own recipe: what one product consumes, in place of its product's cost list (ECO-PROD-2.1). The type orders each good of it up to its stock limit (ECO-PROD-6.1) and takes it per product. Only in an overlay: the table `reborn` is made after the original scripts ran, which never see it, and `properties` keeps the original's 148 functions |
| `s2_rules::model::Building::inputs` | the recipe as the scripts left it; empty for every original type |
| `s2_sim::Rules::recipes`, `World::recipe(type)` | the recipes by type id, kept in the game state and its saves (world format 67). Empty in the original's game, which then has the hashes it had without the hook |
| `s2_ai::AiConfig::type_inputs(type, product)` | what the computer players read for a type's inputs: its needs when built (AI-CON-2.5), its producer needs (AI-MIL-6.2), a site's value by its inputs' suppliers rather than by the product's deposits or fields (AI-CON-6) |

The production window's needed goods are the recipe's. `crates/rules/tests/new_building_type.rs`
is the charcoal burner on Storm Coast with the install's rules: built,
staffed by tick 1 202, its first coal at 2 222, in the headquarters at 2 402,
and its game snapshotted and loaded as the same game.

What it does not cover:

- **A model of its own.** The look is a `bp_setGfxName` entry of the
  install's `buildings.xml` (an original type's look); a new model needs an
  entry there, and an asset folder can only replace that whole file, which
  is the original's. A building whose entry is missing is not drawn.
- **An icon of its own.** `bp_setIconXY` picks a cell of the tribe's
  `uiBuildings` picture; a new one means replacing the whole picture.
  Without an icon the build menu shows a placeholder picture in its cell.
- **Its name in the game's languages.** `bp_setName`'s text is shown as it
  is when no language file has it.
- **A new consumer of a good with a distribution table** (coal, iron,
  planks, water, grain, the foods: ECO-DIST-2.1, `s2_rules::DISTRIBUTION`
  by script name): such a type is never served those goods. Wood has no
  table.
- **A product that grows on the map** (wood, stone, grain) makes the type
  an outdoor gatherer (ECO-CAT-3), which consumes nothing, recipe or not.
- **The computer players' use of it** without the next hook: since 0.10.0
  they leave every type an overlay made out of their producer tables
  (AI-CON-1.4) and never build it; below is how an overlay says when they
  should.
- **New goods**, the missions' locks of types (CAM) and the original's
  saves, which name the original's types only.
- **A shared sort.** The buildings statistics page (UI-GAME-8.5d) has one
  cell per (size class, sort), the later id taking a shared one; the build
  menu's pages hold 20 cells.

### The computer players and a project's type

A type an overlay created (an id the original's scripts do not have) is in
neither of the computer players' producer tables (AI-CON-1.4): with no more
than that, they play exactly the game they play without the overlay. An
overlay says when a type of its is worth building for a square good (coal,
iron ore, gold, stone, wood, fish: the goods whose producer sits on a
resource square) in place of that good's mine or hut:

```lua
-- type, the good it stands in for, the player's stock of that good below
-- which it is wanted, the input good and the stock of it that must be
-- there, the most of the type one player has placed or planned
reborn.bp_setAiAlternative(charcoalBurner, coal, 4, wood, 8, 2)
```

| Item | What |
|---|---|
| `reborn.bp_setAiAlternative(building, good, stock_below, input, input_at_least, most)` | the alternative, read into `s2_rules::model::Building::ai_alternative`. Part of the rules: its line is in the overlay's text, so in the saves' mark (`SaveMark`) |
| `s2_rules::RuleSet::project_buildings` | the building types the overlays created; empty without overlays |
| `s2_ai::AiConfig::project_types`, `alternatives`, `set_project(types, alternatives)` | the same in the computer players' rules (`AiConfig::new` reads them; a test that assembles its own rules calls `set_project`); `alternatives` is by good, an `s2_ai::AiAlternative { building, stock_below, input, input_at_least, most }`. Two types for one good: the lower type id's |

Where it acts: a need of the good that no cell offers a square for
(AI-CON-3.4) goes on the waiting list and drives expansion towards the
good, as always, unless, for that player, the stock of the good (on the
roads too) is below `stock_below`, the warehouses hold at least
`input_at_least` of `input`, and fewer than `most` of the type are placed
(sites too) or planned. Then the need is arranged as a plain good's
(AI-CON-3.3's cell choice, with the type's recipe as the inputs; with no
cell that takes it, it waits as before), its missing inputs are the
recipe's (AI-CON-3.2), and the needs construction orders the type,
checking the type's worker (AI-CON-4.1, 4.3). A need that went on the
waiting list is looked at again when the cell system takes it off
(AI-CON-3.5). `crates/ai/src/tests_construction.rs` (`w12_*`) has a
coal-from-wood type on the fixture: left alone without the alternative
(the computer players' state as without the type), built, staffed and
making coal with it, and the limits.

What it does not cover:

- **A good that is not a square good.** A plank or a sword has its producer
  from the tables and no waiting list; an alternative for it does nothing.
- **Any other choice of the computer players.** A project's type is never
  their military building, depot, harbour or upgrade, and their needs
  from a built consumer, their sites' values and their production control
  (AI-CON-2.5, 6, 10) take it only where they already read its recipe.
- **Tearing it down** when the good's mine comes: they keep what they
  built (AI-CON-9 as for any producer).

### Asset folders

```rust
let dir = s2_assets::GameDir::from_env()?
    .with_overlay("my-mod/assets")      // data/buildings/..., data/sound/...
    .with_overlay("my-mod/hd-textures"); // wins over the first
let file = dir.find("data/sound/menu.wav")?;  // the folder's, else the install's
let (rules, _) = s2_rules::load(&dir)?;       // property scripts come through the folders too
```

A file in an asset folder must have the original's format (`FMT-*` specs,
read by `s2-assets`): a `.dds` texture, a `.gr2` model, a `.wav` sound.
Scripts, texts and fonts wrapped in rc00 (FMT-RC) are unwrapped with the
key of their own name, so a replacement of those may be plain or wrapped. Maps and saves are found
by their own paths (`--map`, `--save`), not through the folders.

### The game with extensions

```rust
use s2_viewer::bevy::prelude::*;
use s2_viewer::{Extensions, RenderOptions};

fn main() -> std::process::ExitCode {
    s2_viewer::run_with(true, Extensions {
        rules_overlays: vec![overlay],
        asset_folders: vec!["my-mod/assets".into()],
        render: RenderOptions { shadow_map_size: Some(4096), ..default() },
        plugins: vec![Box::new(|app: &mut App| {
            app.add_systems(Update, my_hud);
        })],
        ..default()
    })
}
```

`crates/viewer/examples/extended.rs` is this program (built by `cargo test`
on every CI run). `s2_viewer::run(true)` is `run_with(true,
Extensions::default())`. From the command line, `--assets <dir>` and
`--rules-overlay <file>` (both repeatable) add to the extensions of either.

Every field of `Extensions` has a default that adds nothing; set the ones
you use and end the literal with `..Default::default()`, so a release that
adds a hook does not break the build.

`RenderOptions` and their defaults (the game's):

| Field | Default | Meaning |
|---|---|---|
| `shadow_map_size` | `None` (from SHADOW QUALITY, GFX-VIEW-2.5 d) | texels per side; `Some(0)` turns shadows off |
| `shadow_cascades` | 2 | Bevy shadow cascades (docs/shadows.md) |
| `anisotropy` | 8 | anisotropic filtering of the repeating textures |
| `clear_color` | `[0.02, 0.03, 0.06]` | the colour behind everything |

### Bevy's derives in a project

`s2_viewer::bevy` is the viewer's Bevy, enough for
`use s2_viewer::bevy::prelude::*` and every type and function. The derive
macros (`Resource`, `Component`, `Message`, `SystemParam`, `Event`) do not
see the re-export: they look in the project's own `Cargo.toml` for a
dependency named `bevy` (or `bevy_ecs`) and write their paths through it.
So a project that derives them names Bevy itself, at exactly the viewer's
version and without default features (the viewer turns on the features it
needs; Cargo merges them into the one build):

```toml
[dependencies]
# vX.Y.Z: the latest release tag (docs/releasing.md, "Pinning a release").
s2-viewer = { git = "https://github.com/Jensenzo/OpenS2X", tag = "vX.Y.Z" }
# The viewer's exact Bevy (crates/viewer/Cargo.toml); moves with the tag.
bevy = { version = "=0.20.0", default-features = false }
```

```rust
use bevy::prelude::*; // the same crate as s2_viewer::bevy

#[derive(Resource, Default)]
struct Switches {
    follow_camera: bool,
    music: usize,
}
```

The example program derives `Resource` this way; inside the viewer's
package the name `bevy` is the viewer's own dependency. A different
version of Bevy is a second, separate crate: its resources and plugins do
not meet the viewer's app, and the build fails or the plugin never runs.
Check the version when moving to a new tag.

**A project's own Bevy UI** is drawn by the viewer's interface shader, which
blends on stored 8-bit values as the original does (GFX-VIEW-2.6 c,
`docs/colour.md`): colours and pictures mean what they mean in Bevy, an opaque
pixel comes out as in plain Bevy, and a translucent one mixes with the stored
value under it, as the game's own windows and the paused view's layer do. A
`UiMaterial` of a project writes linear light like any Bevy material; to blend
like the rest it encodes its colour itself (`linear_to_srgb` of
`bevy_render::color_operations`, as the viewer's ship window does).

### Menu entries and pages

```rust
use s2_viewer::extend::{MenuEntry, PageWidget};

fn page(world: &World) -> Vec<PageWidget> {
    let switches = world.resource::<Switches>();
    vec![
        PageWidget::Caption("Extended".into()),
        PageWidget::Toggle {
            label: "Follow camera".into(),
            value: switches.follow_camera,
            on_change: Box::new(|world, on| world.resource_mut::<Switches>().follow_camera = on),
        },
        PageWidget::Choice {
            label: "Music".into(),
            options: vec!["Original".into(), "Own".into(), "Shuffle".into()],
            index: switches.music,
            on_change: Box::new(|world, n| world.resource_mut::<Switches>().music = n),
        },
        PageWidget::Button {
            label: "Defaults".into(),
            on_click: Box::new(|world| *world.resource_mut::<Switches>() = Switches::default()),
        },
    ]
}

Extensions {
    menu: vec![MenuEntry { label: "EXTENDED".into(), page: Box::new(page) }],
    ..Default::default()
}
```

| Item | What |
|---|---|
| `Extensions::menu: Vec<MenuEntry>` | the entries; default none (the main menu is the original's) |
| `MenuEntry { label: String, page: Box<dyn MenuPage> }` | the button's label, also the page's title |
| `trait MenuPage: Send + Sync { fn widgets(&self, world: &World) -> Vec<PageWidget> }` | the page's rows from the top, asked for every frame the page is on screen; any `Fn(&World) -> Vec<PageWidget>` is a page |
| `PageWidget::Caption(String)` | a row with its text alone |
| `PageWidget::Toggle { label, value: bool, on_change: OnChange<bool> }` | Off and On, as INFORMATION WINDOW |
| `PageWidget::Choice { label, options: Vec<String>, index: usize, on_change: OnChange<usize> }` | the options three to a line, as AUTOSAVE |
| `PageWidget::Button { label, on_click: OnClick }` | the dialogs' 152 x 22 button |
| `OnChange<T> = Box<dyn FnOnce(&mut World, T) + Send + Sync>`, `OnClick = Box<dyn FnOnce(&mut World) + Send + Sync>` | run with the app's world after the frame's menu actions, before the page is asked for its rows again |

**The main menu.** The entries take the two empty rows of UI-MENU-1.2
between CREDITS and EXIT, at (700, 450) and (700, 500) of the parchment,
on the same 208 x 32 bars in `font_big_white` (capitals only); the original
buttons keep their places. Two entries fit: the column has no other free
row above EXIT, so further entries are left out with a warning in the log.
Below EXIT the scroll has room for one more row, (700, 600), above the
footer line of the captions hook; a later release can open it (and a
page of further entries behind it) if a project needs more than two.

**The page** is the menu dialog of the settings pages (the XXL panel
centred, the label as its title) with the rows of UI-MENU-4.1: from y 72
stepping 32 px, the label at x 24 in the headings' font and colour, the
values from x 168 (Off and On at 168 and 268; a choice's options 100 px
apart, three to a line, lines 24 px apart, 32 px of the page per line), the
chosen value yellow, a button's bar at x 168. Eleven rows fit (down to y
424, above where the settings pages have their page switch); rows past
them are not drawn. BACK at (50, 480) returns to the main menu; ESC, the
mouse and the click sound behave as on the settings pages.
`--menu-screen extension` opens the first page (for screenshots).

### Captions

```rust
Extensions {
    captions: Captions {
        start_screen_title: Some("EXTENDED - BUILT ON OPENS2X".into()),
        main_menu_footer: Some(format!("extended example, OpenS2X {}", env!("CARGO_PKG_VERSION"))),
    },
    ..Default::default()
}
```

| Field | Default | Where |
|---|---|---|
| `start_screen_title: Option<String>` | `None` | the start screen (UI-MENU-0.2), in `font_big_white` and the colour of the game's title line (229, 209, 196), its top-left at (210, 496) of the parchment: left-aligned, one line above `!SplashText0` (210, 520), below the picture's logo |
| `main_menu_footer: Option<String>` | `None` | the main menu, in `dark_system` (the parchment's small dark text), its right end at x 908 (the right edge of the button column) and its top at y 672: on the scroll between EXIT and the scroll's roll |

Positions are of the 1024 x 768 parchment, which is centred on the screen
(UI-MENU-1.1). Each is one line; nothing wraps it.

### Films at start-up

```rust
Extensions { skip_launch_films: true, ..Default::default() }
```

`skip_launch_films: bool`, default `false`. The game started plainly (no
`--menu`, `--menu-screen`, map or save) shows the intro (UI-MENU-0.1; the
three publisher films are never shown, a project decision of OpenS2X),
then the start screen. With `true` it opens at the start screen; START
leads to the main menu as always. The music of the start screen (the main
theme, SND-3.1) starts at once instead of after the intro.

### Music

```rust
use s2_viewer::extend::{CurrentSettings, MusicSlot};

fn music(mut slots: MessageReader<MusicSlot>, settings: Res<CurrentSettings>) {
    for slot in slots.read() {
        match slot {
            // Menu, Credits, Tribe(t), Battle(on), Briefing, Jingle(won):
            // play the project's track for it at
            // settings.get().music_volume().
            MusicSlot::Jingle(won) => { /* once, then silence */ }
            MusicSlot::Briefing => { /* looping */ }
            _ => { /* the others; a later slot lands here too */ }
        }
    }
}

Extensions {
    music_external: true, // the viewer's player silent
    plugins: vec![Box::new(|app: &mut App| { app.add_systems(Update, music); })],
    ..Default::default()
}
```

| Item | What |
|---|---|
| `#[non_exhaustive] enum MusicSlot { Menu, Credits, Tribe(u8), Battle(bool), Briefing, Jingle(bool) }` | a Bevy message (Bevy 0.17 and later call buffered events messages: `MessageReader<MusicSlot>`), sent in the frame the viewer's player starts or stops a track (SND-3): `Menu` the main theme (start screen and menus), `Credits`, `Tribe(t)` each new track of the in-game playlist with the tribe whose set it is from (0 Roman, 1 African, 2 Chinese), `Battle(true)` when the battle loop starts while the player fights, `Battle(false)` when it stops 20 s after the last fight (SND-3.4; a `Tribe` follows as the playlist goes on). Since 0.10.0 the mission interface's music (SND-2.2, CAM-3.6): `Briefing` when the book dialog (a briefing or debriefing) opens over the game, its music looping until the next slot (a `Tribe` when the book closes), and `Jingle(won)` when the end dialog (or the victory or defeat screen) appears, `true` won, `false` lost, played once with nothing after it until the next slot; the end dialog wins over a book. Nothing is sent while the music goes on unchanged (from the main menu to the settings, say), nor during the intro film (its own sound). Non-exhaustive: a `match` needs a `_` arm, so a later slot is no breaking change |
| `Extensions::music_external: bool` | default `false`: the viewer plays the original's music. `true`: its player keeps every track and its timing (so the messages come exactly as without it) at volume 0 |
| `CurrentSettings` | a resource the viewer keeps (`Res<CurrentSettings>`): `get()` is the player's `Settings` as they are now, read only; it changes (Bevy's change detection) only when a setting does |
| `Settings::music_volume() -> f32` | the music volume of the sound settings (SND-5.1, 0 to 1), the volume the original's music plays at (SND-7.1) |

The playlist's order, the gap between tracks and the battle loop's hold
are the viewer's (SND-3.3, 7.2); a project that wants its own track
lengths can keep playing its track over a `Tribe` and change only on
`Battle` or `Menu`.

### Camera

```rust
use s2_viewer::extend::CameraFollow;

fn drift(time: Res<Time>, mut switches: ResMut<Switches>, mut follow: ResMut<CameraFollow>) {
    if follow.cleared {
        switches.follow_camera = false; // the player took the camera back
    }
    if !switches.follow_camera {
        if follow.target.is_some() {
            follow.target = None;
        }
        return;
    }
    let at = follow.target.unwrap_or(Vec3::new(64.0, 0.0, -64.0));
    follow.target = Some(at + Vec3::X * time.delta_secs());
}
```

`CameraFollow { target: Option<Vec3>, cleared: bool }` is a resource the
viewer keeps (default: no target). While `target` is `Some`, the 3D camera
keeps the point at the screen's centre, every frame, at the zoom and turn
it has; the point is in Bevy's world coordinates (those of the scene's
`Transform`s, so a mover's `GlobalTransform` can be followed), and its
height is not used: the camera keeps to the ground as always
(GFX-VIEW-3.3). The player's own camera input takes the camera back: the
arrows, the middle button's drag, the wheel, the zoom and turn keys, the
minimap, a jump to a message or home (H), the look mode (the right button)
and following a settler with O. Then `target` becomes `None` and `cleared`
is `true` for one frame (every system sees it once). A project that sets a
new point after `cleared` takes the camera again. The 2D view (debug) is
not moved.

**O and a project's follow.** O pressed while the project's point is
followed ends the project's follow (`cleared`), as above. The other way
round, a point the project sets while O already follows a settler (the
project followed no point in the frame before: a Ctrl-click on a ware, say)
ends O's follow (GFX-VIEW-3.7, as PAGE UP from behind would): the normal
camera goes on from O's eye and follows the point, without `cleared`. So a
project that wants O to keep the camera sets no point while O follows; one
that sets a point every frame after `cleared` takes it back from O at once.
Without a project's point, O is as without the hook.

### Things on the map

```rust
use s2_viewer::extend::{MapThing, MapThings};

fn pointed(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    things: Res<MapThings>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !(ctrl && buttons.just_pressed(MouseButton::Left) && things.over_map) {
        return;
    }
    if let Some(hit) = things.under_pointer.first() {
        let at = things.position(hit.thing); // where it is drawn
        info!("{:?} (carrying {:?}) {:.1} away, at {at:?}", hit.thing, hit.carrying, hit.distance);
    }
}

Extensions { map_things: true, ..Default::default() }
```

| Item | What |
|---|---|
| `Extensions::map_things: bool` | default `false`: there is no `MapThings` resource and nothing of it is computed. `true`: the viewer keeps it every frame |
| `enum MapThing { Settler(SettlerRef), Ware(usize), Animal(usize) }` | by the simulation's numbers (`docs/api.md`): the `SettlerRef` of `World::settler_views` (settlers, road carriers with their donkeys and boat carriers, builders, levellers), the number of `World::wares` and of `World::animals`. Wares and animals are only ever added, so a number stays with its thing all game |
| `MapHit { thing: MapThing, distance: f32, carrying: Option<usize> }` | a thing under the pointer, how far along the pointer's ray from the camera it enters the thing's box, and the ware a road carrier carries (a worker's good on his way home is no ware of `World::wares` until he puts it down at the flag) |
| `MapThings::over_map: bool` | the pointer is over the map: in the window, over no part of the interface (a window, the minimap, the bar), no menu, book or end screen open, the 3D view on |
| `MapThings::under_pointer: Vec<MapHit>` | the things whose box the pointer's ray crosses, nearest first, of any owner, on fields the player has explored (GFX-VIEW-5.1); empty while `over_map` is false |
| `MapThings::positions: HashMap<MapThing, Vec3>`, `MapThings::position(thing) -> Option<Vec3>` | where each thing is drawn in the frame, in the coordinates of `CameraFollow`: a settler's or an animal's feet, a lying ware at its place in the flag's stacks (GFX-OBJ-5), a carried ware where it is held (GFX-OBJ-7.2; at the carrier when the ware itself is not drawn). A thing inside a building, used up, dead or not drawn is not there; in the 2D view nothing is |

The boxes: a settler's is the game's own (UI-GAME-9.3's pick: 0.8 wide and
2 high where he is drawn), a ware's 0.7 on each side standing on its point,
an animal's by its model's size (0.5 to 6 in each direction). Ships are not
among the things.

**When it is filled.** In `Update`, after the frame's movers and wares are
placed (`sync_movers`, `sync_wares`), from the transforms the frame draws
and the frame's camera. A plugin's system in `PreUpdate` reads what the
last frame drew and may set `CameraFollow::target` to a thing's position;
the camera is placed early in `Update`, before the movers are moved again,
so a followed thing is one frame's step off the screen's centre. **The
choice** between this and a `CameraFollow` that takes a `MapThing` and
resolves it itself (as the O key follows a settler, UI-GAME-12.1): this
one, because one read-only resource lets a project follow, mark or count
anything on the map, and the other would save only that one frame's step;
`MapThings` would still be needed for what is under the pointer.

### Window title

```rust
Extensions { window_title: Some("Frontier".into()), ..Default::default() }
```

`window_title: Option<String>`, default `None`, which is `OpenS2X`. The
game's window has it from its first frame and again after every map load
(which sets the title), and the message boxes (a game folder that does not
hold the game) have it as theirs. The first run's game folder picker asks
"`<title>`: Where is The Settlers II: 10th Anniversary installed?" (without
a title, the question alone, as before). The map viewer (`s2-viewer`,
`run_with(false, ..)`) keeps its own titles with the map's name.

### Screenshots

```rust
Extensions { screenshot_dir: Some(documents.join("S2").join("frontier").join("photos")), ..Default::default() }
```

`screenshot_dir: Option<PathBuf>`, default `None`: the `--screenshot-dir`
option, else `screens` below the working folder (for the installed game,
the folder its shortcut starts in), as without the hook. With a folder,
the viewer's F12 (a developer's tool, in every mode of the game and the
map viewer: `crates/viewer/src/tools/shots.rs`) writes what is on screen
there as `<map>-<unix seconds>.png`, making the folder at the first shot.
Nothing else moves: `--shot`, `--compare` and the play harness's pictures
keep their paths (the harness's under `--screenshot-dir`). The original's
own screenshot key, PRINT SCREEN (TOOL-2: a BMP in
`<Documents>\S2\screenshots`), is not built yet.

### Saves

```rust
Extensions {
    save_dir: Some(documents.join("S2").join("frontier").join("saves")),
    project_name: Some("my-mod".into()),
    rules_overlays: vec![charcoal],
    ..Default::default()
}
```

| Field | Default | With it |
|---|---|---|
| `save_dir: Option<PathBuf>` | `None`: `<Documents>\S2\saves`, the original's save folder (FMT-SAV-1.1), as the game | the folder the project's games are saved in (Save Game, the periodic autosave and the one on leaving, FMT-SAV-5) and listed from (Load Game, Save Game's list, CONTINUE), made at the first save. The original's `.sav` files are still listed from `<Documents>\S2\saves`, after the project's own; OpenS2X's `.s2save` files there are not. CONTINUE takes the newer of the project's autosave and the original's `Autosave.sav`. `--save-dir` (the play harness) wins over both folders |
| `project_name: Option<String>` | `None`: `s2-reborn`, OpenS2X's own (its saves keep the project's earlier name) | the project's name in the mark of its saves |

**The mark.** Every save of ours carries in its header (`s2_ai::save::Header::mark`, save
format 2) a `SaveMark { project, overlays }`: the project's name and the
rules overlays the game runs with (`Extensions::rules_overlays`, then each
`--rules-overlay`), each by its name and `s2_ai::save::text_hash` of its
code (64-bit FNV-1a, the same on every platform and version), in the order
they ran. OpenS2X's own mark has the name `s2-reborn` (the project's
earlier name, kept so its saves stay its own) and no overlays. A save loads
only in a program of the same mark: another one gives the "SAVE NOT LOADED"
notice of another version (`LoadError::OtherMark`), naming the program that
wrote it ("saved by my-mod, another version: my-mod with the rules overlays
charcoal (...); this is s2-reborn", where `s2-reborn` is OpenS2X's mark), never a crash, and the game on screen
stays. So a project's save does not play on with OpenS2X's rules, nor
OpenS2X's with the project's, not even as a file copied from one folder to
the other; a project that changes an overlay's text cannot load its games
from before the change. A save of format 1 (0.7.0 and older) has no mark:
it is OpenS2X's. `s2-app` reads saves as OpenS2X does (its fixtures with
their own mark). A project reading saves itself calls `s2_ai::save::read_as(bytes,
&SaveMark::of(Some(name), &overlays), ..)`; `read` is `read_as` with the
empty mark.

### Settings files

```rust
Extensions {
    settings: SettingsOptions {
        profile_write: false,
        app_settings_dir: Some(documents.join("S2").join("frontier")),
        profile_dir: Some(documents.join("S2").join("frontier").join("profile")),
    },
    on_settings_change: Some(Box::new(|settings| save_my_copy(settings))),
    ..Default::default()
}
```

The game keeps the player's settings in two files (SET-1.2): the
original's profile `<Documents>\S2\users\<profile>.cfg` (volumes,
autosave, the information window, campaign progress; the first write keeps
the old file as `<profile>.cfg.bak`) and the program's own
`%APPDATA%\s2-reborn\settings.txt` (the game folder, the profile chosen,
day and night). Both are written whenever a setting changes: a settings
page, the first run's game folder, a mission won.

| Field | Default | With it |
|---|---|---|
| `settings.profile_write: bool` | `true` | `false`: the profile file and its backup are never written; the values are read from it at start as always, and changes live in memory for the session |
| `settings.app_settings_dir: Option<PathBuf>` | `None` (`%APPDATA%\s2-reborn`, `~/.config/s2-reborn` elsewhere) | the folder of the program's own `settings.txt`, read and written there (it is still written when `profile_write` is false) |
| `settings.profile_dir: Option<PathBuf>` | `None` | a folder of the project's holding a copy of the profile file, `<profile>.cfg` in the original's format: read at start in place of the original's file when it is there (so the first start reads the original's), written at every change whatever `profile_write` says (no backup is kept of the copy). With `profile_write: false` the settings pages' changes (and campaign progress) then last from start to start while the original's file stays as it was |
| `on_settings_change: Option<SettingsHook>` | `None` | `SettingsHook = Box<dyn Fn(&Settings) + Send + Sync>`, called with the settings after every change, written or not |

`Settings` is read only for a project: `music_volume()` (the music hook)
is its public reading; `CurrentSettings` gives the settings in a system.

### The player's folders and the game folder

```rust
let documents = s2_viewer::paths::documents().expect("a documents folder");
let frontier = documents.join("S2").join("frontier"); // beside the profiles
let profiles = s2_viewer::paths::users_folder();     // Some(<documents>\S2\users)
let game = s2_viewer::paths::game_dir();             // in a plugin: Some(<the install>)
```

`documents() -> Option<PathBuf>` is the user's documents folder as the
game finds it (SET-1.1): Windows' known folder Documents, wherever it is
redirected (OneDrive, say); `$HOME/Documents` elsewhere.
`users_folder() -> Option<PathBuf>` is `<documents>\S2\users`, the
profiles' folder. The saves are in `<documents>\S2\saves` (a project's in
its `save_dir`, "Saves" above).
`game_dir() -> Option<PathBuf>` is the game folder the program runs with,
read only, as `run_with` found it: `--game-dir`, else `S2_GAME_DIR`, else
(the game) `game_dir=` in the program's `settings.txt`, where the first
run's folder picker puts it; only a folder that is there. It is `None`
until `run_with` has looked, which it has before it adds a project's
`plugins` (so ask in a plugin's hook or a system, not before calling
`run_with`), and when there is no game folder. It is the install itself:
the asset folders do not change it. The example program logs the
profiles' folder and the game folder at start.

## What a project must never patch

- **The simulation.** No change to `crates/sim`, and no reach into its
  internals (`docs/api.md`, "Not API"). Game state changes only through
  `Command`s at tick boundaries; everything else is read. A plugin cannot
  reach the game's `World` (the viewer keeps it private): it draws and
  takes input. A project that drives the game itself (its own AI, a
  server) runs `s2-sim` as a library.
- **Determinism.** Same seed and command stream, same state, bit for bit
  (`state_hash`). Rules overlays change the rules, so a game with overlays
  has its own hashes; it must still be deterministic. Never seed anything
  from the clock, never put floating point in game state.
- **The defaults.** An extension adds or replaces; it never changes what
  the game does without it. If a change needs OpenS2X's code to behave
  differently by default, it is a change of OpenS2X: through a spec in
  `specs/` and the investigation, not a patch from a project.
- **Original files.** Asset folders hold the project's own files. Never
  ship or commit the original game's files (`S2_GAME_DIR` stays the
  player's install).
- **GPL code.** Nothing from Return to the Roots or OpenSettlers, in OpenS2X
  or linked into it (`deny.toml`).

## Later extension points (not yet)

- Commands from a plugin into the running game (a project's AI inside the
  viewer): needs a public resource for the world's command queue.
- Replacing the computer player (`s2-ai`) in the viewer.
- Shaders: the viewer's WGSL is built in (`crates/viewer/src/render/shaders`).
