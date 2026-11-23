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
| The computer players and a project's rules | they deepen, pave and choose recipes when the overlay says when (hook W25) | `reborn.ai_setDeepen`, `reborn.ai_setPaving`, `reborn.ai_setRecipeRule` (in an overlay), `s2_rules::model::AiProjectRules`, `s2_ai::AiConfig::project_rules` |
| The computer players and a new type | they leave a project's types alone unless the overlay says when one is worth building in place of a mine | `reborn.bp_setAiAlternative` (in an overlay), `s2_ai::AiConfig::alternatives` |
| Recipes a player chooses | a type (a project's or an original one) with several recipes, each a product and its inputs; the building's window chooses which one it works | `reborn.bp_addRecipe`, `reborn.bp_addRecipeInput` (in an overlay), `Command::ChooseRecipe`, `World::recipe_set`, `World::chosen_recipe` |
| A group good in a recipe, a kept stock | a recipe takes "any 2 tools"; a warehouse hands out a member above the stock the player keeps back | a group good in `reborn.bp_addRecipeInput` or `reborn.bp_addInput`, `reborn.cfg_setRecipeKeep` (in an overlay), `s2_sim::Rules::recipe_groups`, `recipe_keep` |
| Paved roads | a road paved with a good per field, carried there over the roads; its carriers and donkeys walk it faster | `reborn.cfg_setPaving` (in an overlay), `Command::PaveRoad`, `World::can_pave`, `World::is_paved`, `World::paving_work` |
| The look of a paved road | a project's street textures for paved roads (hook W21) | `RenderOptions::paved_road` (`StreetLook`) |
| A building's own model | a building type drawn with a project's binary glTF model (`.glb`): its objects, door and work clips, owner's colour, ground patch and smoke, in the game's light (hook W30) | `Extensions::building_looks` (`BuildingLook`), `--building-model` |
| A tree's own model | a tree kind drawn with a project's `.glb` and its far version, as the game draws its trees: turn and sizes, growing, felling and sinking, the wind, shadows, the day's light (hook W30b) | `Extensions::tree_looks` (`TreeLook`, `TreeGrowth`), `--tree-model` |
| A map object's own model | the stonemason's stone piles (each state its own model), the grain fields, the map's decorations and blocking rocks and the geologists' signs drawn with a project's `.glb`, each where and as the game draws its kind: turn and sizes, a field's growing, a pile's states and sinking, shadows, the day's light (hook W30c) | `Extensions::object_looks` (`ObjectLook`), `--object-model` |
| A figure's own model | a settler kind (the carriers first) drawn as a project's `.glb` figure of rigid parts with clips of its own and a far version: the clip chosen by the motion the game asks for, the walk played by the ground covered, the ware at the hand, the owner's colour; heading, turn, stepping aside and the pick box stay the engine's (hook W30d, a trial) | `Extensions::figure_looks` (`FigureLook`), `--figure-model` |
| Deepening a mine | an exhausted mine of a type the overlay names is deepened for material a builder works; part of the ore comes back and the mine eats more food per product | `reborn.bp_setDeepening`, `reborn.bp_addDeepeningCost`, `reborn.bp_addDeepeningFood` (in an overlay), `Command::DeepenMine`, `World::can_deepen`, `World::mine_deepening` |
| The pictures of the hooks' buttons | the Deepen and Next recipe buttons of a building's window and the road menu's Pave button drawn with a 48 x 48 cell of the project's picture | `Extensions::button_pictures` (`ButtonPictures`, `ButtonPicture`) |
| A message when a project's rule finishes | the player's message when a mine's deepening is done or a road is paved, in the project's words (hook W24) | `Extensions::messages` (`ProjectEvent`) |
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
| Particle effects at settlers at work | a `.pfx` effect played at every settler of a profession while his work loop runs (dust where a builder hammers, chips where a woodcutter chops) | `Extensions::effects` (`EffectRule`) |
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

- **A model of its own** without the hook W30 (below, "A building's own
  model"): the look is a `bp_setGfxName` entry of the install's
  `buildings.xml` (an original type's look), and an asset folder can only
  replace that whole file. A building whose entry is missing is not drawn.
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

### The computer players and a project's rules

The computer players never deepen a mine, pave a road or choose a recipe of
their own accord (the original has none of these). An overlay tells them
when (hook W25); each call is optional and none is the default:

```lua
-- deepen an exhausted mine while 6 or more food is in stock (a second
-- value false: tear down a mine not deepened now, W25b)
reborn.ai_setDeepen(6)
-- pave the busiest road once its traffic average is 3 or more and more
-- than 10 stones beyond the road's cost are in stock
reborn.ai_setPaving(3, 10)
-- the smelter works its second recipe (index 1) while more than 40 tools
-- (a group good: its members above their kept stock) are in stock, else
-- its first
reborn.ai_setRecipeRule(building_smelter, 1, good_tools, 40)
```

| Call | What a computer player does |
|---|---|
| `reborn.ai_setDeepen(food_at_least[, keep_waiting])` | when its destruction run (AI-CON-9.2, every 41 ticks, one building at a time) comes to an exhausted mine it would tear down, and the mine's type has a deepening (W15) with deepenings left: it sends `Command::DeepenMine` if its stock of fish, meat and bread together is at least `food_at_least`, no other mine of its own is being deepened and the command would be accepted; otherwise it **keeps** the mine for a later run, or with `keep_waiting` `false` (W25b, since 0.11.4) tears it down as the original does (AI-CON-9.3). A mine being deepened is kept; a mine at its most deepenings is torn down as in the original |
| `reborn.ai_setPaving(traffic_at_least, reserve)` | every 97 counts (shifted per player as its parts are): while none of its roads is being paved, it paves its land road of the highest traffic average (TRN-CARRY-6.1; the lowest object number of equal ones) that can be paved, when that average is at least `traffic_at_least` and its stock of the paving good (`reborn.cfg_setPaving`'s) is more than `reserve` plus the road's cost (the good per field times the path's fields): `Command::PaveRoad` through the road's first inner field |
| `reborn.ai_setRecipeRule(building, index, good, above)` | every 97 counts: each of its finished buildings of the type works recipe `index` (0 the first) while its stock of `good` is more than `above` (a group good of W16: its members together, each counting only its stock above its kept stock, `reborn.cfg_setRecipeKeep`, which is what its warehouses would hand out; W25b, since 0.11.4: until 0.11.3 the plain sum), else the first; it sends `Command::ChooseRecipe` when the building's choice differs. The last call for a type wins; an index the type's set has not got does nothing |

The stock is the computer player's own count of its warehouses (AI-CON-1.7),
as for its other decisions. `s2_rules::RuleSet::ai_project`
(`AiProjectRules { deepen_food, deepen_tears_down, paving, recipes }`) holds the calls, and
`AiConfig::project_rules` takes it from the rules; it is not saved (it
comes from the rules, as the rest of the configuration). Without any call
nothing of it runs: the golden hashes and the fidelity pairs are the
original's game. `crates/ai/src/project.rs` has the decisions' tests and
a computer player paving its road on the AI fixture.

What it does not cover: a computer player building a project's new type
(that is W12's `bp_setAiAlternative`), paving a waterway, deepening more
than one mine at a time, choosing by anything but one good's stock, and any
rule changing what it builds or where.

### Recipes a player chooses

A type can have several recipes, each one product and what one product
consumes; every building of the type works one of them, the first until
its player chooses another in the building's window (hook W20). The type
may be a project's own (a trading post) or an original one (a smelting
works that melts tools as well as ore):

```lua
tradingPost = properties.bp_create("C4A4C0A2")
-- ... size, costs, worker, stock limit as for any new type (above);
-- bp_setGoods gives the type its product class (a workshop), the
-- recipes say what each cycle makes
properties.bp_setGoods(tradingPost, coal, 1)
reborn.bp_addRecipe(tradingPost, coal)          -- recipe 1: coal from
reborn.bp_addRecipeInput(tradingPost, fish, 2)  -- 2 fish and 1 bread
reborn.bp_addRecipeInput(tradingPost, bread, 1)
reborn.bp_addRecipe(tradingPost, ironOre)       -- recipe 2: iron ore
reborn.bp_addRecipeInput(tradingPost, wood, 4)  -- from 4 wood
```

| Item | What |
|---|---|
| `reborn.bp_addRecipe(building, product)` | starts the type's next recipe: one cycle makes one `product` |
| `reborn.bp_addRecipeInput(building, good, count)` | appends (good, count) to the type's last recipe (an error before the first `bp_addRecipe`) |
| `s2_rules::model::Building::recipes` (`RecipeChoice { product, inputs }`) | the set as the scripts left it; empty for every original type |
| `s2_sim::Rules::recipe_sets`, `s2_sim::Recipe { product, inputs }`, `World::recipe_set(type)` | the sets by type id, in the game state and its saves (world format 76). Empty in the original's game, which then has the hashes it had without the hook |
| `Command::ChooseRecipe { player, at, index }` | the building on `at` works recipe `index` (0 the first) from now on; refused (`Rejection::NotARecipe`) for an index the set does not have, and as any building command for another player's or an unfinished building |
| `World::chosen_recipe(at)` | the index the building works; `None` for a type without a set |

What it does: the building orders the chosen recipe's inputs up to its
stock limit (ECO-PROD-6.1), begins a cycle when they are in stock and
makes the recipe's product (ECO-WORK-3, ECO-PROD-5; a recipe whose product
is the tools group makes a tool by the tool priorities, ECO-PROD-7). A
choice withdraws the open orders of goods the new recipe does not take
(wares already on their way are delivered, as when production is switched
off, ECO-PROD-1.2) and orders the new inputs; what the building holds of
the old recipe's goods stays in its stock. A recipe set comes before an H6
recipe (`bp_addInput`) and before the product switch of a switchable type.
The production window shows the chosen recipe's product and its inputs as
the needed goods, and a button in the third place of its column (the
fourth on a switchable type) chooses the next recipe, after the last the
first again. A type with three recipes or more (hook W23) has a Recipes
button there instead, which turns the window to its list: the big panel
titled with the building's name, a row every 40 px from y 56, each the
tools window's check box at x 20 (marked on the recipe the building works),
the product's icon, `<` and each input's icon with its count; a click on a
box chooses that recipe, the close button goes back to the production
window; ten rows fit. The computer players choose only by a rule of the overlay
(hook W25, `reborn.ai_setRecipeRule`, below); else their buildings work the
first recipe, and they read its inputs as the type's
(`AiConfig::type_inputs`). `crates/sim/src/production/tests.rs`
(`a_type_with_a_recipe_set_works_the_chosen_recipe`) is a two-recipe type
on the test fixture.

What it does not cover:

- **More than one ware per cycle.** A recipe makes one product per cycle
  (the worker carries one ware out, ECO-PROD-5); "1 coal for 2 fish" is a
  recipe, "3 boards for 1 tree" is not.
- **A list of the recipes in the window.** The window shows the chosen one
  and steps to the next; a project that wants a list draws its own window
  (`Extensions::plugins`) and sends the command.
- **A good with a distribution table** (coal, iron, planks, water, grain,
  the foods: ECO-DIST-2.1, `s2_rules::DISTRIBUTION`) as an input of a
  project's new type needs the type in that table, or it is never served
  those goods (as for H6). An overlay adds it:
  `reborn.dist_addConsumer(good, building, weight)` puts `building` into the
  table of `good` (any of the table's goods names it: `fish` the food
  table) after the original's consumers, with its default weight in
  thousandths (`s2_rules::RuleSet::dist_consumers`); the player's weights
  work on it as on any entry (ECO-DIST-2.2). The economy dialog's
  distribution page shows a table's first four consumers only (UI-GAME-6.3),
  so a fifth is served but not shown.
- **A group good as an input and a kept stock**: the next section.
- **The computer players choosing a recipe**, and the original's saves
  (which name the original's types and no choice).

### A group good in a recipe

A recipe (a recipe set's, above, or an H6 recipe's) may take a **group
good**, a good that is the group of other goods (`gp_setGroupGoodId` in
the original's scripts: the tools group of the twelve tools). The building
then orders the group, and a warehouse serves the order with any member it
holds; the delivered ware counts as the group in the building's stock and
is taken per product as the group (hook W16). An overlay sets how many of
each member a player keeps back from such orders, so a tool melter does not
take the tools new workers are made from:

```lua
-- the smelting works: iron from ore and coal (the original's recipe,
-- first, so the computer players' buildings work it), or from 2 tools of
-- any kind and 1 coal; 2 of every tool are kept back
reborn.bp_addRecipe(building_smeltery, good_iron)
reborn.bp_addRecipeInput(building_smeltery, good_ironore, 1)
reborn.bp_addRecipeInput(building_smeltery, good_coal, 1)
reborn.bp_addRecipe(building_smeltery, good_iron)
reborn.bp_addRecipeInput(building_smeltery, good_tools, 2)
reborn.bp_addRecipeInput(building_smeltery, good_coal, 1)
reborn.cfg_setRecipeKeep(good_tools, 2)
```

| Item | What |
|---|---|
| a group good in `reborn.bp_addRecipeInput` or `reborn.bp_addInput` | makes the group a recipe group: `s2_sim::Rules::recipe_groups` (group to members, in good order), in the game state and its saves (world format 78); empty in the original's game |
| `reborn.cfg_setRecipeKeep(good, count)` | on a group: each member keeps `count`; on a good: that good (a good's own keep wins over its group's). `s2_rules::RuleSet::recipe_keep`, `s2_sim::Rules::recipe_keep` (by member) |

Where it acts (ECO-REQ-5.3's serving, unchanged for every other good): a
warehouse can serve a recipe group's order when it holds a member of which
the player has more than the kept number across all its warehouses; it
hands out the first such member in good order. A free ware lying at a flag
never serves a group's order. Wares and helpers on their way for such an
order count as the group when the building tops up its orders
(ECO-PROD-6.1). Workers' tools (ECO-REQ-3.3, the professions' cost lists)
are taken as before: the kept stock is only for the groups' orders.
`crates/sim/src/requests/tests.rs`
(`a_recipe_group_is_served_by_a_member_above_the_kept_stock`): 3 axes and 1
hammer in store, a keep of 2: one axe goes, the rest stays.

What it does not cover:

- **Free wares** of a member lying at a flag serve no group order (they go
  to a warehouse as usual, ECO-REQ-5.5).
- **The production switch** as the recipe's switch: Frontier asked for the
  smithy's `ProductionSwitch` to toggle the recipes; the recipe choice is
  `Command::ChooseRecipe` and the window's next-recipe button (above).
- **A kept stock for exact goods' orders**: the keep holds back only from
  the orders of a group.
- **The computer players** choosing a recipe on their own: only by the
  overlay's rule (hook W25, "The computer players and a project's rules").

### Paved roads

A project can let the player pave a road (hook W19): its material is taken
from the warehouses and carried over the roads to the road, and once it has
all arrived the road's own carriers and donkeys walk it faster:

```lua
-- one stone per field of the road; 130: 30 % faster on a paved road
reborn.cfg_setPaving(good_stone, 1, 130)
```

| Item | What |
|---|---|
| `reborn.cfg_setPaving(good, per_field, speed_percent)` | the rule, into `s2_rules::RuleSet::paving` and `s2_sim::Rules::paving` (`s2_sim::PavingRule { good, per_field, speed_percent }`), in the game state and its saves (world format 79); none in the original's game, which then has the hashes it had without the hook |
| `Command::PaveRoad { player, at }` | `at` an inner field of the player's road (as `RemoveRoad` takes it); refused (`Rejection::NotPavable`) without the rule, for a waterway, and for a road paved or being paved (`Rejection::NoRoad` for no road of the player's there) |
| `World::can_pave(player, at)`, `World::is_paved(number)`, `World::paving_work(number)` (`PavingWork { needed, sent, laid }`), `World::paving_rule()` | by the road's object number (`Road::number`) |

What it does: an accepted pavement needs `per_field` times the road's
fields (both flags included) of the good. Each tick, while some is still to
be sent, one ware of it leaves the warehouse nearest to the road's first
flag that holds the good (ECO-REQ-5.3's choice of the warehouse), put at
that warehouse's flag with the road's first flag as its destination, and
the carriers take it there as any ware (TRN-CARRY); where the road starts
at the warehouse's own flag it is laid at once. Each ware that reaches the
road's first flag is laid there and leaves the world; the last one makes
the road paved. A ware lost on its way (destroyed with a road it was on) is
sent again: each tick a pavement counts the wares of the good still bound
for its first flag. A carrier or donkey of a paved road walks a step whose two
fields both lie on its road at its base speed times the factor (on top of
the slope factor: TRN-MOVE-1.2's rate with base x factor); every other
step, and every other walker on it, as before. A road split by a new flag
keeps the pavement on both halves; a pavement under way is dropped by a
split, and what was laid is lost. The road menu (UI-GAME-3.3) has a pave
button (the flag-with-hammer picture, "Pave the road."), usable while the
command would be accepted. `crates/sim/src/requests/tests.rs`
(`a_paved_road_costs_its_stone_and_is_walked_faster`).

What it does not cover:

- **Laying it field by field**: the road's carrier does not stop to lay
  the stone, and no part of the road is paved before all of it has
  arrived; the stone leaves the warehouse at its flag without a helper
  carrying it out (ECO-REQ-5.3c's walk).
- **Its look** without W21 (below): a paved road is drawn as a dirt road.
- **The computer players** paving on their own: only by the overlay's
  rule (hook W25, "The computer players and a project's rules").

### The look of a paved road

A project gives paved roads (hook W19) street textures of its own (hook
W21); without them a paved road is drawn as the original's dirt road:

```rust
use s2_viewer::{Extensions, RenderOptions, StreetLook};

let extensions = Extensions {
    asset_folders: vec!["assets".into()], // assets/data/textures/paved_street.dds, ..._n.dds
    render: RenderOptions {
        paved_road: Some(StreetLook {
            texture: "paved_street.dds".into(),
            normal: "paved_street_n.dds".into(),
        }),
        ..Default::default()
    },
    ..Default::default()
};
```

| Item | What |
|---|---|
| `RenderOptions::paved_road: Option<StreetLook { texture, normal }>` | the colour texture (its alpha the blend, as `street.dds`) and the tangent-space normal map (as `street_n.dds`), by file name, found as the install's textures are (an asset folder first); default `None` |

What it does: the street pass (GFX-OBJ-6.2) gets a fifth part. Every land
road with `World::is_paved(road.number)` leaves its own part (0 with a
donkey, 1 without) and is drawn in the fifth with the project's textures,
the same strip (its width, its texture stretch, stopping short at shared
flags), lit as a road strip. The crossings at its flags stay the
original's (parts 2 and 3). The part's number includes the pavement, so the
square's mesh is made again in the frame a road becomes paved, and a split
keeps both halves in the fifth part. Without the option nothing changes:
the four parts and their numbers are the original's (`crates/viewer/src/
world/scene3d/tests.rs`, `a_paved_look_draws_paved_roads_in_a_part_of_
their_own`; the golden images unchanged).

What it does not cover: a crossing of its own at a paved road's flags; a
texture per player or per road; a road paved field by field (W19 paves a
road whole). It was not looked at on screen in OpenS2X.

### A building's own model

A project draws a building type with a model of its own, a binary glTF file
(`.glb`), in place of the type's entry of the install's `buildings.xml`
(hook W30). The type keeps its rules, its graphics name (`bp_setGfxName`)
included, so neither the rules nor a save's mark change; only its look does.
The model is found through the asset folders like any file of the install:

```rust
use s2_viewer::{BuildingLook, Extensions};

let extensions = Extensions {
    asset_folders: vec!["assets".into()], // assets/data/buildings/frontier/charcoal_burner.glb
    building_looks: vec![BuildingLook {
        building: "Charcoal burner".into(), // the type's bp_setName
        model: "data/buildings/frontier/charcoal_burner.glb".into(),
        site_of: Some("Lumberjack".into()), // the woodcutter's site model while it is built
        smoke: Some("Smoke".into()),        // data/scripts/effects/Smoke.pfx at anchor_smoke
    }],
    ..Default::default()
};
```

| Field | Meaning |
|---|---|
| `building` | the type drawn with it, by its name in the rules (`bp_setName`'s: `Spring` the well, a project type's own name), for all three tribes; the other types of its graphics name keep theirs |
| `model` | the `.glb` file's path below the install (`data/...`, any case), an asset folder's first |
| `site_of` | the graphics name of the type whose site model (the scaffolding of `buildings.xml`'s `frame`, GFX-OBJ-2.6a) stands while this one is built, of the builder's tribe; `None`: no scaffolding, only the rising house |
| `smoke` | the effect (`data/scripts/effects/<name>.pfx`, the install's or a project's) that rises from the model's point `anchor_smoke` while the building works; `None`: no smoke |

From the command line: `--building-model <type>=<file>[,site=<gfx>][,smoke=<effect>]`
(repeatable), with `--assets` for the folder. `s2-viewer --map-name
MP_2P_Storm_Coast --3d --scene pair:WoodcutterHut:Spring --assets tests/models
--building-model Spring=facets.glb,site=Lumberjack,smoke=Smoke` shows the
repository's test model as the well beside a woodcutter's hut;
`crates/viewer/examples/extended.rs` draws the well with it when
`S2_NEXT_ASSETS` holds `facets.glb`.

**What the file must hold.** Anything else in it is not read (skins, morph
targets, cameras, lights, more images); a file the reader refuses is not
drawn, with a line on the console.

| Item | Rule |
|---|---|
| Frame | glTF's own: +Y up, metres (one engine unit is a metre), **the door facing +Z**, +X to the right of someone standing in front of the door, the origin at the building's field point (the walk of a settler ends there, behind the door), the ground at Y = 0. The engine turns every building by the loader's -30 degrees about +Y (GFX-VIEW-4.3a): the model must not hold that turn. In Blender with the exporter's "+Y up": Z up, the door facing -Y |
| Objects | one node per object, its mesh in the node's own frame; nodes may have children. At most 50 nodes in all (the shader's bone palette, GFX-ANIM-8.3), points included. Triangle lists only |
| `teamcolour` | the node tinted with its owner's colour: its colour times the player colour (give it the palette's white `team` swatch) |
| `ground` | the node (or a primitive whose material is named `..._ground`) laid on the terrain as a building's ground patch is (GFX-OBJ-2.3: each triangle draped over the fields, 0.01 above them, lit as level ground, receiving shadows, casting none), blended by its vertex alpha. Every other part is opaque (alpha is not read) |
| Points | empty nodes. `anchor_smoke`: where the smoke rises (its rest place; it may not move with a clip). `anchor_origin`, `anchor_door`, `anchor_light` are read as points but not used |
| Faces | flat as authored: the file's normals are used as they are (a mesh without normals gets one normal per face) |
| Colour | `TEXCOORD_0` into the material's base colour image (an embedded PNG; its bytes are taken as the display colours, like the original's textures, no conversion), times `COLOR_0` (RGB: the face's occlusion, 1.0 in the open; A: the ground's fade). The material's metallic-roughness image (embedded PNG): green is the roughness, blue the metallic, red is not read (the occlusion is the vertex colour). Every face is lit by the game's own sun and sky (GFX-VIEW-2.3) with its shadow, both sides drawn; a smooth face (roughness below 1) gets a small highlight of the sun, on metal in its own colour. Factors, emissive, normal and occlusion images are not read |
| Clips | glTF animations of node translation, rotation and scale (whole objects; skins are not read), in clip seconds, played 1.8 times as fast as written, as the original's (`globalSpeed`, GFX-ANIM-2.1). `DoorOpen` and `DoorClose`: played once when the engine opens and closes the door for a settler (GFX-ANIM-6.1a), last pose held; 0.4 s turning the `door` node about its hinge by about 100 degrees, outwards. `Working`: **loops** while the building's worker is in its work step (the original's play once per work start, GFX-ANIM-6.2), from 0 at each start. Linear keys (a step channel is read as linear; a cubic spline's tangents are dropped). The names must match exactly; other clips (`door_open`, `work`) are not read |

**What the engine does with it**, as for an original house otherwise:

- **Placed, frame work**: the ground patch, and the `site_of` type's site
  model with its pieces going up (GFX-OBJ-2.6a).
- **Main work**: the model rises out of the ground inside the scaffolding:
  drawn below a plane at the height the original's houses use (GFX-OBJ-2.6b,
  the model's height without its ground), the cut not capped. Then the
  scaffolding goes.
- **Finished**: the model in its rest pose; door and Working clips as above;
  the smoke while it works, its particles living out their life when the
  work stops; the owner's colour on `teamcolour`; the status icons at the
  model's box (everything but the ground).
- **Burnt, torn down, land lost**: the engine's own collapse (GFX-OBJ-8.7:
  the model sinks by its height with the dust, a running door or work clip
  goes on), then the ruin of the type's remains class; the ruin lies on the
  model's ground patch. Upgrades (GFX-OBJ-8.4) cut it as a house.
- **Without any look**: nothing of it is read or computed; every original
  building is drawn as before (the golden images unchanged).

**At night** a project's building is darkened as the original's houses are
by their light map (GFX-OBJ-2.1: theirs multiplies a face by its light map,
all of it at night): by 0.533, the median over the Roman houses of their
light map where their faces sample it (0.30 to 0.70; the woodcutter's 0.41),
times how much of the night it is (none while the light and ambient
luminances add up to noon's 1.4377, all of it at the night's 0.5993, in
between linearly). By day nothing changes. Windows that light up at night
are not drawn (a light map of its own is not read).

What it does not cover: settlers (see "A figure's own model": rigid parts,
no skin), map objects (see "A tree's own
model" and "A map object's own model"), a switch between whole graphics sets; a site model of
a project's own (`site_of` borrows an original's); the original's
per-building colour variation (GFX-OBJ-2.2: a project model is never tinted
by the colour map) and its own light map (the night factor above is one
number); a project's own colour map; the original's levels of detail (one
model at every distance); the T31 quirk of the original's house light (a
project model is lit by its world normal, the original's houses by their
unturned one); clips' sound and particle events other than the smoke.

### A tree's own model

A project draws a tree kind with a model of its own, a binary glTF file
(`.glb`), and optionally a lighter far version, in place of the kind's
landscape element (hook W30b). The kind keeps its rules and its graphics name
(`dp_setGfxName`); only its look changes. The files are found through the
asset folders like any file of the install:

```rust
use s2_viewer::{Extensions, TreeGrowth, TreeLook, TREE_FAR_BELOW};

let extensions = Extensions {
    asset_folders: vec!["assets".into()], // assets/data/trees/frontier/birch_a.glb, ...
    tree_looks: vec![TreeLook {
        tree: "BirchA".into(), // the kind's dp_setName
        model: "data/trees/frontier/birch_a.glb".into(),
        far: Some("data/trees/frontier/birch_a_far.glb".into()),
        far_below: TREE_FAR_BELOW,      // 0.25 of the view's height
        growth: TreeGrowth::RootWidth,  // or TreeGrowth::Original
    }],
    ..Default::default()
};
```

| Field | Meaning |
|---|---|
| `tree` | the kind drawn with it, by its name in the rules (`dp_setName`'s: `BirchA`, `BirchB`, `BirchC`, `BroadLeafA`, `BroadLeafB`, `BroadLeafC`, `FirA`, `FirB`, `PalmA`, `PalmB`, `CypressA`, `OliveA`, `AfricanA`, `AsianA`, `LavaTreeA` .. `C`, a project kind's own); a later look of the same name wins |
| `model` | the `.glb` file's path below the install (`data/...`, any case), an asset folder's first |
| `far` | the far version's file; `None`: the model at every size |
| `far_below` | the far version is drawn while the tree's size on the screen, the longest side of its box as it stands, grows or lies, is below this share of the view's height. The size is the box's longest side over the view's height at the tree's depth, not its foreshortened picture (the camera looks down at 53 degrees). `TREE_FAR_BELOW` (0.25): in a 16:9 view a grown 10 m tree is 0.16 to 0.25 of the view's height at the farthest zoom (top to bottom of the screen) and 0.22 to 0.33 at the default zoom; so the farthest zoom draws the far versions, the default zoom the full models but along the screen's far edge, closer zooms the full models; a young tree turns far sooner. A narrower window (4:3) shows the trees larger. 0: never the far version |
| `growth` | how a young tree grows: `Original` (GFX-ANIM-3.8b: height s, width min(3 s, 1), so a third-grown tree has the full crown on a third of the height) or `RootWidth` (height s, width the square root of s: 0.58 of the width at a third, 0.71 at a half; both full at s = 1). The original's trees always grow by `Original` |

From the command line: `--tree-model <kind>=<file>[,far=<file>][,far-below=<share>][,growth=root]`
(repeatable), with `--assets` for the folder. `s2-viewer --compare
MP_2P_Storm_Coast --zoom 4 --assets tests/models --tree-model
BirchA=tree_round.glb,far=tree_round_far.glb --tree-model
FirA=tree_cone.glb,far=tree_cone_far.glb` shows the repository's test trees
(`scripts/make-test-trees.py`) for two kinds; `crates/viewer/examples/extended.rs`
draws the birches with them when `S2_NEXT_ASSETS` holds `tree_round.glb`.

**What a tree's file must hold.** As a building's (above), anything else is
not read.

| Item | Rule |
|---|---|
| Frame | glTF's own: +Y up, metres (one unit a metre), **the origin at the foot**, where the trunk meets the ground (the engine puts it on the ground at the tree's place, upright on any slope); the trunk may reach a little below (the original's trees reach 0.2 to 1.0 under). No turn of its own: the engine turns each tree by its own random yaw (0 to 360 degrees) |
| Size | as the tree stands full grown; the engine scales each tree by its kind's three random sizes (x, y, z each 0.8 to 1.0 for most kinds; GFX-OBJ-1.4) and while it grows. The original's kinds are 6.7 to 14.3 high and 2.8 to 12.2 across (the slot sheet's section 1); a crown may reach over the neighbouring fields (6 m apart) |
| Objects | one mesh (several nodes are allowed and drawn as one, at most 50); no `teamcolour` (a tree has no owner); a `ground` node is laid on the terrain as a building's (none needed). Points (`anchor_origin`) are read but not used |
| Faces | flat as authored. Both sides are drawn. In the wind every face is lit by its own normal as it moves (below), not by the file's normals; a mesh without normals is fine |
| Colour | as a building's: `TEXCOORD_0` into the base colour image (embedded PNG, its bytes the display colours) times `COLOR_0` RGB (the face's occlusion); the metallic-roughness image's green and blue as there (foliage at roughness 0.8 gets no highlight to speak of). Opaque: alpha is not read (no alpha-tested leaves) |
| Light | the game's sun and sky (GFX-VIEW-2.3) by day and night, the tree's shadow cast and received. The original's tree pass scales only the direct light by its vertex colour (GFX-OBJ-2.4); a project's tree is palette x vertex colour under the whole light, as a building |
| Far version | the same outline from the game camera, the same palette swatches, its origin at the same foot; it is drawn with the same turn and sizes. Fewer faces is the point (the brief: at most 250 triangles, the model at most 900) |

**What the engine does with it**, as with the original's trees otherwise:

- **Placed**: on its deposit's fine position on the ground, with the
  kind's own random turn and three sizes, drawn from the view's generator in
  the original's order (GFX-OBJ-1.4): a forest of project trees stands
  exactly where and how the original's would, turned and sized the same.
- **Growing**: scaled by `growth` from the sapling's size 0.1 to 1
  (100 seconds; ECO-OUT-6.2), re-placed every 20 ticks as the original's.
- **Wind**: the original's sway (GFX-OBJ-2.4): each point is pushed by
  saturate(0.003 |p|²) x the tree's height scale, along x by sin(0.35 pi x +
  t) and along z by cos(0.35 pi z + t) of the tree's place and the frame
  clock (a 6.28 s period; a point 10 m up moves 0.3 m, the foot not at all).
  The faces stay flat: each face is lit by the normal of the face as drawn,
  taken from the screen derivatives of its world position, so every face
  stays one plane with one normal while its corners move; the shared corners
  of two faces move alike (the push is a function of the point), so no gap
  opens. The shadow sways with the tree.
- **Felled**: tipped about its foot toward the woodcutter's left, about 81.6
  degrees over 2.4 s (GFX-ANIM-3.8a), then sunk straight down by its lying
  box's height over 1.8 s (3.8c). The model must stand being tipped.
- **Many of them**: each tree is one entity whose meshes and material are the
  kind's shared ones (and the far version's), so the renderer batches a wood
  as it batches the original's trees; per frame the engine compares each
  tree's size on the screen with `far_below` and switches only the trees whose
  version changes.
- **Without any look**: nothing of it is read or computed; every tree is
  drawn as before (the golden images unchanged).

What it does not cover: figures (stone piles, grain fields, decorations and
signs: "A map object's own model", below); alpha-tested leaves or a texture of a tree's own besides the palette; a
switch between whole graphics sets; a sound or effect at a project tree.

### A map object's own model

A project draws the other kinds of things lying on the map with models of its
own, binary glTF files (`.glb`), in place of their landscape elements (hook
W30c): the stonemason's stone piles, the grain fields, the map's decorations
(the small ones and the blocking rocks) and the geologists' signs. A kind
keeps its rules and its graphics name; only its look changes, so a look
changes no rule and no state: a blocking rock blocks its field as before, a
decoration under a new road goes as before, a sign stays under one as
before. The files are found through the asset folders like any file of the
install:

```rust
use s2_viewer::{Extensions, ObjectLook};

let extensions = Extensions {
    asset_folders: vec!["assets".into()], // assets/data/objects/frontier/stones_6.glb, ...
    object_looks: vec![
        ObjectLook::new("StoneRessourceA06", "data/objects/frontier/stones_6.glb"),
        ObjectLook::new("Field01", "data/objects/frontier/grain_field.glb"),
        ObjectLook::new("Coal (few)", "data/objects/frontier/sign_coal_s.glb"),
        ObjectLook::new("bush01", "data/objects/frontier/bush.glb"),
    ],
    ..Default::default()
};
```

| Field | Meaning |
|---|---|
| `object` | the kind drawn with it, by its name in the rules (below); every type of that name takes it; a later look of the same name wins. A tree kind's name does nothing here (trees: `tree_looks`) |
| `model` | the `.glb` file's path below the install (`data/...`, any case), an asset folder's first |

**The names.** The rules name is the script's own (`dp_setName` for a
deposit, `dd_setName` for a doodad), spaces and brackets included;
`s2-app rules deposits` and `s2-app rules doodads` list them with their
graphics names.

| Kind | Rules names |
|---|---|
| Stone piles (the first set), by how many stones are left | 6: `StoneRessourceA06`, 5: `StoneRessourceA05`, 4: `StoneRessourceA04`, 3: `StoneRessourceA03`, 2: `StoneResourceA02`, 1: `StoneResourceA01` (the rules spell the first two without the double s). Each stone taken replaces the pile by the kind of one stone fewer on the same place, so each state is its own look; the last one sinks into the ground over 18 ticks when its stone is taken, by its model's own height |
| Stone piles (the second set, other maps) | the same with `!!!MED ` before: `!!!MED StoneRessourceA06` .. `!!!MED StoneResourceA01` |
| The grain field | `Field01`: one model, grown as the game grows a field: from size 0.2 at sowing to 1 when ripe (106.7 s), its width min(3 s, 1) and its height s (GFX-ANIM-3.8b, the trees' rule); harvested, it sinks as a pile |
| The geologists' signs | `Empty`, `Water`, `Coal (few)`, `Coal (medium)`, `Coal (much)`, `Iron (few)` .. `Iron (much)`, `Gold (few)` .. `Gold (much)`, `Granit (few)` .. `Granit (much)` |
| Blocking rocks and the gate | `rock 1` .. `rock 4`, `!!MED rock 1` .. `!!MED rock 4`, `((LAVA rock 0` .. `((LAVA rock 2`, `Gate01` |
| The other decorations (98) | `bush01`, `stone01` .. `stone04`, `stone01 grey` .. `stone04 grey`, `dead Tree 1`, `dead Tree 2`, `bones0` .. `bones3`, `grass01` .. `grass04`, `mushroom red`, `mushroom red big`, `mushroom brown`, `mushroom brown big`, `cactus01` .. `cactus04`, `high flower white big`, `high flower white`, `high flower red big`, `high flower red`, `high flower yellow big`, `high flower yellow`, `fern big`, `fern medium`, `fern small`, `swampthing01`, `swampthing02`, `swamp calmus 01` .. `03`, `agave big`, `agave medium`, `agave small`, `nettle`, `nettle big`, `nettle high`, `flower red`, `flower red big`, `flower red high` (and the same for `white`, `yellow`, `violet`), `grass translucent`, `grass translucent big dark`, `fingerpost E`, `fingerpost SE` .. `fingerpost NE`, `wreck`, `wreck big`, `shell`, `shell small`, `((LAVA fog`, `((LAVA fog high`, `((LAVA fog highest`, `((LAVA fog vertical`, `!!MED nettle`, `!!MED nettle big`, `!!MED nettle high`, `waterplant 1` .. `3`, `waterlily 1`, `waterlily 2`, `DoNotUse-Skull01` |

From the command line: `--object-model <name>=<file>` (repeatable; quote a
name with spaces), with `--assets` for the folder. `s2-viewer --map-name
MP_2P_Storm_Coast --3d --assets tests/models --object-model
bush01=stones_1.glb --object-model "Coal (few)=sign_post.glb"` draws Storm
Coast's bushes with the repository's one-stone test pile
(`scripts/make-test-objects.py`); `crates/viewer/examples/extended.rs` draws
the first set's stone piles, the fields and the empty sign with the test
models when `S2_NEXT_ASSETS` holds `stones_3.glb`.

**A file that cannot be drawn stops the start.** Before the game starts the
engine reads every look's file; one it cannot find or read ends the program
with a line that starts with the file's name as the look gives it
(`data/objects/frontier/stones_6.glb: not found (...)`), not a hole in the map.

**What the file must hold.** As a tree's (above), anything else is not read.

| Item | Rule |
|---|---|
| Frame | glTF's own: +Y up, metres (one unit a metre), **the origin at the thing's place on the ground**: the engine puts it there, upright on any slope, and turns it about +Y by the kind's own turn. It may reach a little below the ground (the original's piles reach 0.04 to 0.15 under, the rocks several metres) |
| Size | as it stands (a field: ripe). The engine scales each instance by its kind's three random sizes and turns it by its random turn (GFX-OBJ-1.4: for most decorations a turn of 0 to 360 degrees and 0.8 to 1.2 on each axis; none for stone piles, the fingerposts and the gate). The original's sizes are in the slot sheet's section 5: a pile of six stones about 5 x 5 and 3.6 high, a field 1.1 x 1.3 and 1.8 high, a sign 2.0 high, a blocking rock up to 13 across |
| Objects | one mesh (several nodes are drawn as one, at most 50); no `teamcolour`, no `ground` patch. `anchor_origin` is read but not used |
| Faces | flat as authored; both sides drawn, each face lit by its own normal (a mesh without normals is fine) |
| Colour | as a tree's: `TEXCOORD_0` into the base colour image (embedded PNG, its bytes the display colours) times `COLOR_0` RGB (the face's occlusion); the metallic-roughness image's green and blue as a building's. The palette may be of any size: the trees' 512 x 320 and the map objects' 512 x 512 both work (a side that is not a power of two is stretched to one, so a uv finds the same swatch). Opaque: alpha is not read |
| Light | the game's sun and sky by day and night, as a project's tree: the instance's shadow cast (where its element casts one, as the original's) and received, no sway |

**What the engine does with it**, as with the original's kinds otherwise:

- **Placed**: where the original's instance stands (a deposit's fine
  position, a doodad's sample, a sign's field), with its turn and sizes drawn
  from the view's generator in the original's order (GFX-OBJ-1.4): a project's
  bushes stand exactly where and how the original's would.
- **A stone pile** shows the model of its state; when a stone is taken the
  pile of one fewer takes its place (a new instance, its own model); the last
  one sinks.
- **A grain field** grows from sowing to ripe as above, re-placed every 20
  ticks; the original's field shader also turns a young field green (GFX-OBJ-2.7):
  a project's field is not tinted, its colours are the palette's at every size.
- **Many of them**: each instance is one entity whose meshes and material are
  the kind's shared ones, so the renderer batches a thousand bushes as it
  batches the original's.
- **Without any look**: nothing of it is read or computed; every map object is
  drawn as before (the golden images unchanged).

What it does not cover: the underwater haze of the original's rocks and signs
(GFX-OBJ-2.5: a project's rock under water is drawn without it); the young
field's green; the gate's water plane (`Gate_water`) and its frames; road
flags, border markers and buoys (player-coloured, the roads' and borders'
sheet), the ruins of buildings (the buildings'), animals; a far version (one
model at every distance: these are small and simple); alpha-tested leaves or
grass blades.

### A figure's own model

A project draws a settler kind as a figure of its own, a binary glTF file
(`.glb`) of **rigid parts that turn at their joints** (as a building's door
turns), moved by **clips of its own** (hook W30d). Nothing of the original's
skeleton, skin or clips is used. It is a trial: it is whole for the road
carriers of every tribe; any other kind takes the same path (its own
motions, see the clip table below) but has not been looked at. A kind keeps
its rules; only its look changes, so a look changes no rule and no state:

```rust
use s2_viewer::{Extensions, FigureLook};

let mut carrier = FigureLook::new("Carrier", "data/settlers/frontier/carrier.glb");
carrier.far = Some("data/settlers/frontier/carrier_far.glb".into());
let extensions = Extensions {
    asset_folders: vec!["assets".into()],
    figure_looks: vec![carrier],
    ..Default::default()
};
```

| Field | Meaning |
|---|---|
| `figure` | the kind drawn with it, by its name in the rules: `Carrier` for the road carriers (of every tribe, on land; the donkeys stay the original's); for the others the job's good name the settler list uses (`Woodcutter`, `Constructor` the builder, `Bulldozer` the leveller, `Private` .. the soldiers' ranks); a later look of the same kind wins |
| `model` | the `.glb` file's path below the install (`data/...`, any case), an asset folder's first |
| `far` | a lighter file with the same nodes, points and clips, drawn while the figure is small on the screen. Default none: the model at every size |
| `far_below` | the far file is drawn while the figure's standing height on the screen is below this share of the view's height. Default `FIGURE_FAR_BELOW`, 0.04 (43 pixels of a 1080-pixel view: in the middle of the view a 2.15 figure is about 46 pixels high at the default zoom and 36 at the farthest, so the far file shows at the farthest zoom and along the far edge of the default's) |

From the command line: `--figure-model <kind>=<file>[,far=<file>][,far-below=<share>]`
(repeatable) with `--assets` for the folder. `s2-viewer --map-name
MP_2P_Storm_Coast --3d --assets tests/models --figure-model
Carrier=figure.glb,far=figure_far.glb` draws Storm Coast's carriers as the
repository's box figure (`scripts/make-test-figure.py`);
`crates/viewer/examples/extended.rs` does the same when `S2_NEXT_ASSETS` holds
`figure.glb`.

**A file that cannot be drawn stops the start**, with a line that starts with
the file's name as the look gives it: one that cannot be found or read, has
no mesh, more than 50 nodes, no clip `Stand` or `Walk`, or no point
`attach_hand` (`data/settlers/carrier.glb: no clip Walk`).

**What the file must hold.**

| Item | Rule |
|---|---|
| Frame | glTF's own: +Y up, **facing +Z**, the origin on the ground between the feet; made in game units (a carrier 2.15 high). The settlers' scale of 1.2 is **not** applied to it. The engine turns it once, half round, into the settlers' facing; the game turns it to its heading |
| Nodes | at most 50 (the shader's palette, as a building's); every mesh rigid on its own node, the nodes' tree the joints. The parts should overlap at the joints so that no gap opens in any motion. Empty nodes are points |
| Points | `attach_hand` (required): where the carried ware hangs, its pose taken from the playing clip every frame with nothing added (GFX-OBJ-7.2). The ware is the install's item model as the view holds it: its up along the point's +Y, a plank's or log's length along the point's Z, its grip at the point's origin. `anchor_origin` is read but not used |
| Clips | node animation (translation, rotation, scale; linear), **in real seconds**, named for the motions below. `Stand` and `Walk` are required; `Stand`, `Walk`, `StandCarry` and `WalkCarry` loop, the others play once. **One `Walk` (and `WalkCarry`) cycle covers 1.5 units of ground**: the feet planted on the ground travel back 1.5 units over the clip, whatever its length (the carrier's: 0.556 s, two steps) |
| Colour | as a building's: `TEXCOORD_0` into the embedded palette image times `COLOR_0` (the face's occlusion), roughness and metallic from the second image. **The owner's colour**: the faces whose corners' middle lies on the palette's `team` swatch (palette version 2: column 4, row 4, u and v 0.5 .. 0.625 in glTF's coordinates) are multiplied by the owner's player colour, as a building's `teamcolour` part (the swatch is white, so they show the player colour, shaded by their occlusion). No `teamcolour` node is needed |
| Far file | the same node names: each of its parts is moved by the model's node of its name, its vertices staying where the far file has them at rest. Its own points and clips are not read (the model's play for both) |

**What the engine does with it.**

- **The clip of the motion.** Each tick the game's clip choice
  (GFX-ANIM-3.1) gives a motion; the figure plays its clip for it:

  | The game asks | With a ware at the hand | Without |
  |---|---|---|
  | standing | `StandCarry` | `Stand` |
  | walking (also in the water) | `WalkCarry` | `Walk` |
  | taking up a ware | `Pickup` | `Pickup` |
  | putting it down | `PutDown` | `PutDown` |
  | another (a worker's `WorkLoop0` .. `2`, a soldier's `Hit`, a rowing carrier) | the base's name, once | the base's name, once |

  A motion the file has no clip for falls back to the plain one
  (`WalkCarry` to `Walk`, `StandCarry` to `Stand`), else to `Walk` while the
  settler moves and `Stand` at rest, looping; the program logs it once by the
  file's and the clip's name (`figure carrier.glb: no clip PutDown, drawn with
  Stand (hook W30d)`). Never a rest pose, never a crash. The original's random
  idle clips (GFX-ANIM-3.3) and the clips' sound events are not played for a
  figure.
- **The walk does not slide.** `Walk` and `WalkCarry` are not played by the
  clock but by the ground the figure covers between two frames, one cycle
  per 1.5 units, so a carrier slowed on a slope or on the slow free walk steps
  slower in the same measure. The cycle is drawn in 32 phases (at the full
  walk, 2.7 units a second, a phase lasts 17 ms, a frame at 60 frames a
  second); each phase is one pose shared by every figure in it.
- **What stays the engine's**: the place between ticks (GFX-ANIM-4.1), the
  heading and its turn, the stepping aside (4.3), the in-water test, the
  dying sink, the pick box (UI-GAME-9.3), the seen marks, "inside a building
  nothing is drawn", the boat of a rowing carrier.
- **Light**: the map objects' facet family (flat faces, the game's sun and
  sky by day and night, shadows cast and received).
- **Cost.** The engine merges a figure's parts into one mesh per version
  (model, far) and kind (plain, owner's colour), each vertex bound with
  weight 1 to its node's matrix: the trial's carrier is two drawn meshes,
  880 triangles (its far file 244), against the original's one skinned mesh
  of 1,700 to 2,400. Every figure in the same pose (a looping clip, a
  walking phase) and of the same owner shares its meshes' materials, and
  those poses are kept once made, so the renderer draws them in batches and a
  walker moving from phase to phase makes no new material; the original's
  settler has colours of its own (GFX-OBJ-3.2) and is drawn on its own. A
  play-once clip (`Pickup`, `PutDown`) takes a pose of its own. Measured on
  a late Storm Coast game (about 960 settlers, the frame benchmark's scene):
  10.7 ms a frame with the carrier against 10.4 ms with the original's.
- **Without any look**: nothing of it is read or computed; every settler is
  drawn as before (the golden images unchanged).

What it does not cover: a skinned figure (a skin bound to the original's
skeleton and moved by its clips; the slot sheet's other way); the
original's per-figure colour variety (every figure of a kind and owner is
the same); the donkeys, ships and animals; clips' sound and particle events;
the original's idle variants; any motion beyond the clip table above (the
work, fight and dying motions of other kinds fall back as described).

### Deepening a mine

A mine type can be deepened once its mine is exhausted (hook W15): the
player orders it in the mine's window, the material comes and a builder
works it, and then part of each field's first amount of ore is back:

```lua
-- the coal mine: a quarter of each field's first amount comes back, at
-- most twice per mine, for 4 planks and 2 stone; after the first
-- deepening the mine eats 2 food per coal, after the second 3
reborn.bp_setDeepening(building_coalmine, 250, 2)
reborn.bp_addDeepeningCost(building_coalmine, good_plank, 4)
reborn.bp_addDeepeningCost(building_coalmine, good_stone, 2)
reborn.bp_addDeepeningFood(building_coalmine, 2)
reborn.bp_addDeepeningFood(building_coalmine, 3)
```

| Item | What |
|---|---|
| `reborn.bp_setDeepening(building, share_per_mille, most)` | gives a mine type its deepening: of each field's first amount the share that comes back, per mille, and the most deepenings of one mine |
| `reborn.bp_addDeepeningCost(building, good, count)` | adds to what one deepening costs (an error before `bp_setDeepening`) |
| `reborn.bp_addDeepeningFood(building, food)` | the food one product eats after the next deepening: the first call after the first, the second after the second; the last one given for any later deepening (1 before the first, as ECO-PROD-2.2) |
| `s2_rules::model::Building::deepening` (`DeepeningRule`) | as the scripts left it; none for every original type |
| `s2_sim::Rules::deepenings`, `s2_sim::Deepening`, `World::deepening(type)` | by type id, in the game state and its saves (world format 77); empty in the original's game, which then has the hashes it had without the hook |
| `Command::DeepenMine { player, at }` | accepted for the player's finished mine of a type with a deepening that is exhausted (it has no current ore field and none within its working radius holds its product: ECO-FOOD-2's own search), has fewer deepenings than the most and none under way; else `Rejection::NotDeepenable` (`Rejection::NoBuilding` for another player's) |
| `World::can_deepen(player, at)`, `World::mine_deepening(at)` (`MineDeepening { done, under_way }`) | whether the command would be accepted now; the mine's deepenings so far |

What it does: a game whose rules have any deepening keeps each field's
resource as it was before its first unit was dug (in the state; an older
save's fields keep what is left at their first dig after the load). An
accepted deepening is a construction of the mine's own type on its field,
with the costs as its main material and no frame and no levelling, worked
as an upgrade's (MIL-UPG-2.3): its site orders the material by the
construction table (ECO-HQ-4), a builder comes and works it, and the mine
goes on meanwhile. When it is done, each field within the mine's working
radius whose first resource was the mine's product holds `share` per mille
of its first amount again, rounded down but at least 1 (at most 7, the
field's cap; a field holding another good, or more of it, is left as it
is), the count rises by one, and from then on one product eats the
deepening's food: that many times the first of fish, bread, ham in stock,
a cycle beginning only with that many in stock, each food ordered up to 2
plus the extra. The mine's next search (every 101 ticks, ECO-FOOD-2) finds
the ore again. The production window of such a type has a button in the
third place of its column, usable while the command would be accepted;
its tooltip names the cost and the deepenings so far.
`crates/sim/src/workshop.rs` (`a_deepened_mine_digs_again_at_a_higher_food_cost`)
is a coal mine on the test fixture.

What it does not cover:

- **The computer players** deepening on their own: only by the overlay's
  rule (hook W25, "The computer players and a project's rules"); without
  it their exhausted mines are torn down as in the original.
- **A look of its own**: while it is deepened the mine is drawn as an
  upgrading building (GFX-OBJ-8.4: the house above a rising plane, the
  same house growing below it, the type's site model around both), and its
  window is the production window as before.
- **A status line** in the window ("Deepened twice"): the tooltip says it.
  A message when a deepening is done is hook W24 (below).
- **The map's first amounts** of fields never dug before the first save of
  an older game: such a field keeps what it has when first dug after the
  load (only a game saved before 0.11.0, which had no deepenings).
- **A geologist's signs** for the restored ore: they show what a search
  finds as for any field (ECO-SPC), nothing more.

### The pictures of the hooks' buttons

The Deepen button (W15) and the Next recipe button (W20) of a building's
window are drawn with the boat-and-ship switch's picture (`uiElements5`
(256, 88)), and the road menu's Pave button (W19) with the flag-with-hammer
portrait (`uiElements3` (368, 160), lit (416, 160)), unless the project
names a picture of its own for each (`pave` since 0.11.4, W34):

```rust
use s2_viewer::{ButtonPicture, ButtonPictures, Extensions};

let extensions = Extensions {
    button_pictures: ButtonPictures {
        deepen: Some(ButtonPicture {
            file: "frontier_buttons.dds".into(), // in an asset folder
            at: [0, 0],
            hover: Some([0, 48]),
        }),
        recipe: Some(ButtonPicture {
            file: "frontier_buttons.dds".into(),
            at: [48, 0],
            hover: None,
        }),
        pave: Some(ButtonPicture {
            file: "frontier_buttons.dds".into(),
            at: [96, 0],
            hover: Some([96, 48]),
        }),
    },
    ..Default::default()
};
```

`file` is a `.dds` found by its file name as the models' textures are: one
of the install's interface sheets (`uiElements4.dds`) or a file anywhere
in an asset folder. The button is the 48 x 48 cell at `at` of it, and while
the pointer is over it the cell at `hover` (none: the same); greyed as any
button while it cannot be used. A picture that is not found leaves the
button with its picture as before. Each picture is loaded once with the
interface's atlases.

### Messages when a project's rule finishes

The original has no deepening and no paving, so it has no message for
them. With hook W24 the player gets one, in the project's words:

```rust
use s2_viewer::{Extensions, ProjectEvent};

let extensions = Extensions {
    messages: vec![
        (ProjectEvent::MineDeepened, "Mine deepened".into()),
        (ProjectEvent::RoadPaved, "Road paved".into()),
    ],
    ..Default::default()
};
```

| Event | When | The message |
|---|---|---|
| `ProjectEvent::MineDeepened` | one of the local player's mines has finished a deepening (`World::mine_deepening(at).done` rose) | at the mine, with its type's name in brackets as "Building completed (Woodcutter's Hut)" has |
| `ProjectEvent::RoadPaved` | one of the local player's roads is paved (its last stone laid; the halves of a later split keep the pavement and give none) | at the road's first flag |

Each is listed in the messages window as the original's misc messages
(UI-GAME-11.1 kind 4: the exclamation mark, its filter, its 36 000 ticks,
a left click jumps to the place, a right click deletes it); the text is
shown as given, not looked up in the language file. The messages are the
viewer's, as the autosave's (not game state, not in saves): after a load
the watch starts from what it finds. `ProjectEvent` is `#[non_exhaustive]`.
None given (the default): nothing is watched and no message comes, as in
the game.

What it does not cover: the computer players' events (only the local
player's); a sound with the message; a message of the project's own
choosing for any other event.

### Asset folders

```rust
let dir = s2_assets::GameDir::from_env()?
    .with_overlay("my-mod/assets")      // data/buildings/..., data/sound/...
    .with_overlay("my-mod/hd-textures"); // wins over the first
let file = dir.find("data/sound/menu.wav")?;  // the folder's, else the install's
let (rules, _) = s2_rules::load(&dir)?;       // property scripts come through the folders too
```

A file in an asset folder must have the original's format (`FMT-*` specs,
read by `s2-assets`): a `.dds` texture, a `.gr2` model, a `.wav` sound. The
one exception is a building's or tree's own model of hooks W30 and W30b, a
`.glb` (above).
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
| `shadow_filter` | `ShadowFilter::Hardware2x2` (GFX-VIEW-2.5c) | the map camera's shadow filter; `ShadowFilter::Gaussian` is Bevy's wider filter, soft edges over several texels (hook W17) |
| `shore_foam` | `None` (the original has none) | foam on the water along the shores (hook W18, S2N-005): `ShoreFoam { reach, colour, strength }`, the foam colour mixed in by `strength` at the waterline, fading out where the ground lies `reach` below the water level (raw height units of the water's depth term, GFX-WATER-4.4: about 2 900 from the waterline to the deepest; 300 a narrow band) |
| `ambient_occlusion` | `false` (the original has none) | screen-space ambient occlusion on the map's ambient light, the map camera without multisampling (hook W17b, below) |
| `shadow_map_scale` | `None` (the profile's size) | the profile's shadow map size (`SHADOW QUALITY`) times this factor, at most `MAX_SHADOW_MAP` (8192); the Shadows switch, quality 0 and shader detail 1 still turn shadows off. `shadow_map_size` wins when both are set (hook W17) |

A project that draws sharper or softer shadows than the original (hook
W17) sets them like this; the engine page keeps its say over whether
shadows are drawn at all:

```rust
render: RenderOptions {
    shadow_filter: s2_viewer::ShadowFilter::Gaussian,
    shadow_map_scale: Some(2), // 2048 at SHADOW QUALITY 0.75, 4096 at 1.0
    ..default()
},
```

The viewer's own shaders read the shadow map through Bevy's lookup
(`fetch_directional_shadow`), so the filter acts on the ground, the
objects and the water alike.

**Ambient occlusion** (hook W17b, S2N-004's third part):

```rust
render: RenderOptions {
    ambient_occlusion: true,
    ..default()
},
```

The map camera gets Bevy's screen-space ambient occlusion
(`ScreenSpaceAmbientOcclusion` at High quality, its default radius) with
the depth and normal prepasses it needs: the terrain draws them with its
own prepass shader, the objects with their shadow shader (skinned, swaying
and waving as drawn, alpha-tested at 0.5, a house being built cut at its
plane); both write the surface's own normal (from the screen derivatives
of the position, toward the camera), which agrees with the depth. Bevy's
occlusion reads 1 - 1/6 on any flat surface at that quality (a sample on
the surface marks the sector at the hemisphere's edge); the shaders divide
that out, so open ground stays as it was. The ground's and the objects'
**ambient term** (GFX-VIEW-2.3's ambient colour; a settler's ambient
luminance) is multiplied by the occlusion at the pixel: measured on Storm
Coast's headquarters at zoom 0, the open ground keeps 0.99 of its colour,
a wall 0.91, a column's foot 0.82. The sun's
light, the shadows, the water, the roads and decals (blended, not in the
prepass) and the water's mirror image are as without it. It needs the map
camera **without multisampling** (`Msaa::Off`): edges are not smoothed
while it is on. Off (the default) no camera has a prepass, no shader reads
the occlusion (the texture is read only under Bevy's
`SCREEN_SPACE_AMBIENT_OCCLUSION` definition) and the pixels are the
original's. `crates/viewer/examples/extended.rs` turns it on with the
environment variable `EXTENDED_AO`.

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
| `PageWidget::PageBreak` | hook W22: the rows after it start the page's next part |
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
424). **A longer page** (hook W22) is shown a part at a time: a row that
would reach past y 424 starts the next part, and so does a
`PageWidget::PageBreak` (a project breaks before each section; breaks at
the start, the end or twice make no empty part). A page of more than one
part has a switch at its foot, y 436: `<` at x 168, `n/m` at 218, `>` at
268, in the body font, an arrow greyed at either end; a click shows that
part's rows from y 72. The part shown is the viewer's (`MenuState::page_part`,
the first whenever the page is opened), each row keeps its index for its
callback. The original's settings pages switch by their tab row instead,
which has a picture per page; the switch is a project's look of its own.
A page that fits shows no switch, as before. BACK at (50, 480) returns to the main menu; ESC, the
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

### Effects at settlers at work

The original plays particle effects (`.pfx` scripts, GFX-OBJ-8.3a) at a
building's status, at a collapse, and at two settlers' work: the
leveller's and the forester's work loops start `StoneDust`; a builder's
hammering and a woodcutter's chopping have sounds only (GFX-OBJ-8.3b). A
project adds effects of its own at settlers at work (hook W18):

```rust
effects: vec![s2_viewer::EffectRule {
    effect: "data/scripts/effects/StoneDust.pfx".into(), // or a project's own
    profession: "good_constructor".into(),               // the builder
    work_loop: Some(0),                                  // WorkLoop0: hammering
}],
```

| Field | Meaning |
|---|---|
| `effect` | the `.pfx` script's path in the game data, of an asset folder (a project's own effect) or the install |
| `profession` | the settler's profession by its script name (`good_constructor`, `good_woodcutter`, ...) |
| `work_loop` | the work loop it plays during (`WorkLoop<n>` of GFX-ANIM-3.8: 0 is the builder's hammering and the woodcutter's chopping); `None` any |

Each rule plays at every settler of that profession, any player's, while
the logic asks for that work loop (`SettlerView::request`): the effect
starts at his feet as drawn when the loop begins, is played on the
animation clock (game time) as the original's effects are, with the
original's particle texture, and is gone when the loop ends or he is no
longer drawn. Each particle takes the colour the effect file gives it (its
start colour times its colour curve) over its texture cell, as the
collapse dust does (hook W33; until 0.11.2 a project's particle was drawn
white, so it showed its cell's own colour): a wood-coloured chip is a
`.pfx` with a brown start colour on a light cell. Render side only: nothing of it is game state, and with no
rules the system is not even in the app. `crates/viewer/examples/extended.rs`
plays `StoneDust` at the builders.

What W18 does not cover (the rest of the plan's step 8.3, not built):

- **Normal maps on objects** (S2N-005): the object shader takes no
  project textures yet. (Shore foam is built: `RenderOptions::shore_foam`,
  above.)
- **Animation crossfades** (S2N-007): clips still cut as the original's.
- **Weather** (S2N-024): no fog or rain layer; the original's day and
  night cycle stays the only change of light.
- **Effects at other moments** than a settler's work loop (a felled tree's
  fall, a finished building).

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
