# Changelog

What changed in each release, newest first. Versions follow semver
([`docs/releasing.md`](docs/releasing.md)): before 1.0 any release may
break a project built on these crates, and says how under "For projects
built on OpenS2X: breaking changes". Releases before 0.9.3 were made under
the working name s2-reborn. The public repository starts with its first
public release; the sections before it describe releases made in the
maintainer's working repository, which have no tag or download there.

## OpenS2X 0.14.0 (2026-11-23)

The road carriers can be drawn with a project's own figure, the long
two-player games play on tick for tick alike with the original for 99
minutes (Weak; 94 in 0.13.0) and 90 minutes (Strong; 89), and the
four-player war for its first 29 minutes (27). The game's rules change only
where the original's own play showed a difference.

### What is new

- **A figure's own model, a trial** (for projects built on OpenS2X): the road
  carriers can be drawn as a figure of the project's own, a binary glTF model
  (`.glb`) of rigid parts that turn at their joints, with motions of its own
  and a lighter version for the far zooms. The game picks the figure's motion
  as it picks the original's (standing or walking, with a ware or without,
  taking it up, putting it down); the walk follows the ground the carrier
  covers, so his feet do not slide on a slope or on a slow walk; the ware
  hangs at the figure's hand, and the tunic takes the owner's colour. Where
  he stands, turns and steps aside stays the game's. What the file must hold:
  docs/extending.md, "A figure's own model". The viewer's `--figure-model`
  shows it without a project.
- **The four-player war** follows the original for its first 28 minutes (27
  in 0.13.0): buildings farther than twelve fields from an attacked one send
  fewer soldiers out to meet the attackers, as in the original.
- **The Strong computer player's long game** stays equal to the original's
  for 90 minutes (89): a ware lying free at a flag goes to the building that
  ordered it at once when no storehouse holds that good, without asking the
  route's cost.
- **Ships:** a ware that must go overseas picks its harbour by the same road
  costs the original remembers (the Weak game's random numbers equal for a
  further 259 ticks); a ship with nothing to do lies at its harbour until it
  is needed, and makes room when another ship comes to unload or to load
  there, as the original's ships do.
- **The Weak computer player's long game**: its random numbers stay equal to
  the original's for 99 minutes (94): an upgraded building orders its
  materials one tick sooner, and a mine whose ore field another mine emptied
  looks for a new field at its next try, as in the original.
- **The Strong game recorded densely** is now equal to the original
  throughout its 33 minutes: a worker held before his full flag shows his
  walk again in the tick his ware is let on.
- **The four-player war**: a computer player's torn-down building takes its
  own orders for goods with it; an attacker takes a waiting interceptor of
  another building that defends his target.
- **Harbours** work on the original's beat: a new harbour's first pass is a
  full pass after it is finished, a harbour asks overseas only for what it
  does not hold, and a ship it calls is loaded in a later pass with what the
  other island still needs then. Saves from 0.13.0 load as before.
- **The four-player war** follows the original for 29 minutes now: an
  attacker never waits on a field an interceptor was sent to meet him on;
  an interceptor walks first to that field and takes up his fight there a
  little later, and one whose duel has just ended can be taken by the next
  attacker at once; duels at a building that is taken over go on. A builder
  whose site is lost while he walks to a work position starts home a tick
  later.
- **Harbours:** a settler needed overseas is asked for over the roads that
  have a carrier, as wares are (the Strong game's random numbers equal for a
  further 104 ticks).
- **For developers:** the viewer's `--shot-frames N` saves a shot as N frames
  in a row, each a 60th of a second of game time (for motion strips).

### For projects built on OpenS2X: breaking changes

- `s2_viewer::Extensions` has a new field `figure_looks` (hook W30d); a
  struct literal without `..Default::default()` needs `figure_looks:
  Vec::new()`. New `s2_viewer::FigureLook` and `FIGURE_FAR_BELOW`. Without a
  look every settler is drawn as before.

## OpenS2X 0.13.0 (2026-11-22)

A map's own objects can be drawn with a project's models, the long
two-player game plays on tick for tick alike with the original for 94 minutes
(87 in 0.12.3), and the four-player war for its first 27 minutes (22). The
game's rules change only where the original's own play showed a difference.

### What is new

- **A map object's own model** (for projects built on OpenS2X): the
  stonemason's stone piles, the grain fields, the map's decorations (the small
  ones and the large rocks) and the geologists' signs can be drawn with binary
  glTF models (`.glb`) in place of the original's, each kind by its name in the
  rules. Each one stands where and as the original's would, turned and sized
  as the game does it; a stone pile shows a model of its own for each number
  of stones left and changes as the stonemason takes them, a field grows from
  sowing to harvest, and all of them cast and catch shadows and take the day's
  and the night's light. A look changes only the picture: a rock still blocks
  its field, a decoration under a new road still goes. A model file the
  game cannot read stops it at the start with the file's name. What the files
  must hold: docs/extending.md, "A map object's own model". The viewer's
  `--object-model` shows them without a project.
- **For developers:** the viewer's `--ground-study` draws the ground cut into
  flat facets in the colours of a project's palette file, for pictures that
  help decide whether a ground hook is wanted. It is not a hook and changes
  nothing when it is not given. Its second round adds `ragged` (the terrain
  kinds meet in an irregular line of whole facets), `ragged-flat` and `calm`
  (no facets on the ground, for comparison), colours of its own for rock,
  snow and meadow, and see-through water over the ground. Its third round
  adds `hybrid` (the soft colours of `calm` with each facet lit on its own)
  and `hybrid-crisp` (as `hybrid`, but meadow, sand, rock, snow, earth and
  lava meet in a line of whole facets), with deeper, greener water and a thin
  pale line where it meets the land.
- **The war after the first capture follows the original further:** the
  four-player war recorded in the original now plays tick for tick alike for
  its first 27 minutes of game time (22 in 0.12.3). Workers and soldiers of
  land that is lost go home as the original's do (a worker on his way back
  from a tree walks on to his flag first, a soldier still walking out of the
  storehouse walks on to its flag first), a carrier whose road goes stops on
  the field he has just reached, settlers on a flag with no road home walk
  over open ground to the nearest storehouse's roads instead of wandering,
  and the computer player counts a building's soldiers who are out or just
  stepping in when it decides on an attack. A ware lost on the way keeps its
  place in the line of orders instead of going to the front, a settler's first
  step is as quick as the ground was when he set off, and two duels fought at
  the same moment take their turns in the order they began.
- **Woodcutters, stonemasons, foresters and hunters** reach their tree, stone
  or animal as the ground is when each step begins: a building site levelled
  next to them while they walk changes their pace as it does in the original.
  The long two-player game's random numbers stay equal to the original's for
  94 minutes (87).
- **A founded harbour:** the ship that founded it sails home three ticks after
  the founding (not about ten seconds later), the new harbour's site needs no
  levelling, and its builder walks from the site to its flag before he
  starts. A builder sent for an expedition walks out to the storehouse's
  flag and from there to the harbour, where he is kept for the expedition at
  once. A fisherman whose fishing ground another fisherman has emptied catches
  his fish without taking one from the water next to him, and the computer
  player gives a new quarry or mine to its oldest waiting need. The Strong
  computer player's long game stays equal to the original's for 89 minutes.
- **Project models:** the faces of a project's buildings, trees and map
  objects that the sun only grazes no longer show fine shadow stripes.

### For projects built on OpenS2X: breaking changes

- `s2_viewer::Extensions` has a new field `object_looks` (hook W30c); a struct
  literal without `..Default::default()` needs it (`Vec::new()`: as before).
  Without a look every map object is drawn as before: the golden pictures and
  state hashes are unchanged.
- `s2_sim::Command` has a new variant `PlanHarbour { player, ship, records }`
  (the computer player's founding), and `s2_sim::Work::Approach` and
  `Work::Return` a new field `walk`: an exhaustive match or a struct literal
  needs them. `s2_sim::AttackRecord` has a new field `paired` and
  `s2_sim::movement::Walker` a new field `first_heights`: a struct literal needs them
  (`0`, `None`). The world snapshot is format 96 (92: a worker's fine walk,
  93: the new command, 94: a walk's first step, 95: an expedition builder's
  home, 96: the order of the duels); every older save of ours loads.

## OpenS2X 0.12.3 (2026-11-20)

0.12.2 with one fix, and trees for projects. 0.12.2 was made but not
published: its 30-minute test game had one frame of a tenth of a second, the
moment its game first showed a well's building site. A project built on
OpenS2X can now also draw trees with models of its own. The game plays
exactly as in 0.12.2; what 0.12.2 brings is listed under it below.

### Fixed

- **A short hitch the first time something appears.** The picture could stop
  for up to a tenth of a second when a game first showed a kind of building,
  site, ware or figure, while its model was read and sent to the graphics
  card. A game now reads every model its tribes can show while its loading
  picture is up. For developers: the long play's log names every model read
  after the start, and beside a slow frame the ones read just before it.

### What is new

- **A tree's own model** (for projects built on OpenS2X): a tree kind can be
  drawn with a binary glTF model (`.glb`) in place of the original's. A wood of
  them stands exactly where and how the original's would, each tree turned and
  sized as the game turns and sizes its trees; they grow, sway in the wind with
  their faces kept flat, fall when felled and sink into the ground, cast and
  catch shadows and take the day's and the night's light. A lighter version of
  each model is drawn while a tree is small on the screen, at a size the project
  chooses. Young trees can grow slimmer than the original's, which spread their
  full crown at a third of their height. What a tree's model must hold:
  docs/extending.md, "A tree's own model". The viewer's `--tree-model` shows
  them without a project.
- **A building's own model at night** is now darkened as the original's houses
  are; by day it looks as before.
- **For developers:** the viewer's `--until-falling` catches a tree while it
  falls (pictures), and `scripts/long-play.sh` can play another map with extra
  viewer arguments (`LONG_PLAY_MAP`, `LONG_PLAY_ARGS`, `LONG_PLAY_OUT`). The
  long play now holds the display on and wakes it first: with the display
  turned off by the power plan every frame waited a quarter of a second.

### For projects built on OpenS2X: breaking changes

- `s2_viewer::Extensions` has a new field `tree_looks` (hook W30b); a struct
  literal without `..Default::default()` needs it (`Vec::new()`: as before).
  Without a look every tree is drawn as before: the golden pictures and state
  hashes are unchanged.
- A building drawn with a project's own model (`building_looks`) is darker at
  night than in 0.12.2 (by 0.533 at full night); by day it is unchanged.

## OpenS2X 0.12.2 (2026-11-19)

Battles and the Strong computer player's expeditions follow the original more
closely. The four-player war recorded in the original now plays tick for tick
alike for its first 22 minutes of game time (21 in 0.12.1, with fewer
differences after it); a Strong computer player's expedition ship founds its
harbour where it has landed.

### What is new

- **A soldier killed in a fight lies where he fell** for about three seconds
  of game time before he is gone, as every settler who dies (he vanished at
  once before).
- **Defenders come out when their attacker is there:** a soldier of the
  attacked building waits inside until his attacker stands at the door or has
  taken his place, then comes out; a soldier back from a fight is ready to
  fight again a moment after he has stepped in (at the headquarters, when he
  reaches its door), and so is one who has just walked in from a storehouse.
- **Taking a building:** the attacker who reaches the door of a building with
  no defenders left takes it a moment later, then walks in.
- **The Strong computer player's expeditions** found a harbour when the ship
  stands close enough to a free harbour place, as the ship window's "Construct
  harbor" button allows; before, the ship sailed on.
- **For developers:** `s2-app fidelity` counts the original recorder's
  one-tick lag in showing a settler's new animation, so the dense and the
  one-player recordings now equal ours to their ends; `s2-app trace-diff
  --view-lag` does the same for any pair.

### For projects built on OpenS2X: breaking changes

- `s2_sim::Settings` has a new field `skeleton_doodad` (the doodad a dying
  settler leaves, `cfg_setSettlerSkeletonDoodad`); a struct literal of it needs
  the field (`None`: no skeleton). Saves of 0.12.1 load in 0.12.2, not the
  other way round.
- `s2_ai::ships::sail_decision(vessel, can_found, pick)` takes the founding
  test's result (`World::can_found_harbour`) instead of the ship's position and
  a spot test.

## OpenS2X 0.12.1 (2026-11-19)

A project built on OpenS2X can now draw a building with a model of its own,
made in Blender and saved as glTF. The game plays exactly as in 0.12.0.

### What is new

- **A building's own model** (for projects built on OpenS2X): a building type
  can be drawn with a binary glTF model (`.glb`) in place of the original's
  model. It stands where the original's houses stand, in the game's sunlight
  and shadows, its faces flat as made, coloured from one palette picture and
  the shading painted into its corners; one part takes its owner's colour and
  its ground patch lies on the terrain and fades out at the edges. Its door
  opens and closes for the settlers who walk in and out, its working
  animation runs while it works, smoke rises from its chimney point, and it is
  built, burnt and torn down as any house. What the model must hold:
  docs/extending.md, "A building's own model". The viewer's
  `--building-model` shows one without a project.

### For projects built on OpenS2X: breaking changes

- `s2_viewer::Extensions` has a new field `building_looks` (hook W30); a struct
  literal without `..Default::default()` needs it (`Vec::new()`: as before).
  Without a look every building is drawn as before: the golden pictures and
  state hashes are unchanged.

## OpenS2X 0.12.0 (2026-11-17)

The computer players and the battles follow the original more closely. A
four-player war of three Strong computer players, recorded in the original,
now plays tick for tick alike for its first 21 minutes of game time (8 in
0.11.5); the long two-player games stay equal to the original's for 87
minutes (Weak) and 84 minutes (Strong).

### What is new

- **Soldiers who go out to attack stay their building's soldiers.** The
  building counts them while they are away and asks for no replacement; if
  it wants fewer soldiers meanwhile, it lets them go first, and they go to a
  storehouse when their fight is over. When a building is lost, its soldiers
  out fighting fight on.
- **Attacks as the original makes them:** a building far from the target
  (more than 12 fields) sends fewer soldiers; soldiers leave one at a time
  through their building's flag; the meeting places of attackers and
  defenders, who arrives when, and when the first blow falls follow the
  original.
- **The computer players** choose their military buildings, upgrades,
  attack targets and new production as the original does in many more
  cases: where to upgrade when a new military building finds no place, which
  building a need is tied to, which goods are short, which roads to build
  next, when a storehouse is worth it, and how many coins to order.
- **Settlers walking home** whose road is removed under them stop at the end
  of their step and choose a new way home at once, as in the original.

### For projects built on OpenS2X: breaking changes

- The world snapshot version is 85 (0.11.5: 82). Saves of earlier versions
  load; what an older game shows is listed in docs/snapshot.md (attackers
  out at the save go back on their building's list).
- `s2_sim::AttackStep` has two new steps, `Meeting` and `Arrived`;
  `World::garrison` lists a building's soldiers out on an attack, not
  present. The golden state hashes changed for the rules named in each
  commit.

## OpenS2X 0.11.5 (2026-11-17)

0.11.4 as published: the same program, with two of its documents worded for
readers outside the project. 0.11.3 and 0.11.4 were made but not published;
what they bring is listed under them below (the long-game slowdown fixed in
0.11.4).

### Fixed

- `docs/test-coverage.md` ("The long play") and the head of
  `scripts/long-play.sh` no longer use the project's internal finding
  numbers, and `scripts/release.sh` now runs the public check
  (`scripts/public-check.py`) over the whole tree before it makes a release.

## OpenS2X 0.11.4 (2026-11-17)

Long games stay smooth again: the slowdown after 15 to 30 minutes of a big
game, which every release since 0.9.3 had (the public 0.10.1 and 0.11.2
among them), is fixed. Two small hooks for projects.

### Fixed

- **Long games slowed down.** In a big game the picture grew slower and
  slower after 15 to 30 minutes of play, with more and more single frames
  of a tenth of a second or longer; the game itself kept its pace. Releases
  0.9.3 to 0.11.3 have it, the public 0.10.1 and 0.11.2 too. Measured on
  their downloads, 30 minutes of a four-player game from the start at
  2560 x 1440: 0.11.2 ran at 60 frames per second for 15 minutes, then at 39
  after 20 minutes and at 16 after 25, with 1 200 frames of a tenth of a
  second or longer; 0.10.1 at 60, then 43 after 20 minutes and 32 after 25,
  with 776 such frames. The cause: settlers that walk (or work)
  the same way share one animation, and each figure drawn in it kept a slot
  in the buffer of bone positions for as long as anyone at all still walked
  that way, which on a busy map is for good; the whole buffer goes to the
  graphics card every frame. Since 0.9.3 every settler figure has colours of
  its own, so every settler that ever walked took one more slot. Now a
  figure's slot is given back when it stops using the animation. In a
  30-minute test game the buffer stays at 6 MB (it reached 100 MB) and the
  frames at 16.6 ms (99 % within 21 ms), as before 0.9.3.

### What is new

- **The long play** is a step of the release checks: 30 minutes of a
  four-player game from the start, held to the refresh rate, which fails on
  a slowdown, a growing palette buffer or a broken rule
  (`scripts/long-play.sh`; docs/test-coverage.md, "The long play").
- New hook **W34**, off by default (docs/extending.md, "The pictures of the
  hooks' buttons"): a project's picture for the road menu's Pave button,
  `ButtonPictures::pave`.
- Hook **W25b** (docs/extending.md, "The computer players and a project's
  rules"): `reborn.ai_setDeepen(food, false)` makes a computer player tear
  down an exhausted mine it does not deepen in that run, as the original's
  rule does, instead of keeping it idle; and `reborn.ai_setRecipeRule` with a
  group good counts each member's stock above its kept stock
  (`reborn.cfg_setRecipeKeep`), what the warehouses would hand out.

### For projects built on OpenS2X: breaking changes

- `s2_viewer::ButtonPictures` has a new field `pave` (W34); a struct literal
  of it needs the field or `..Default::default()`. The default draws as
  before.
- `s2_rules::model::AiProjectRules` has a new field `deepen_tears_down`
  (W25b; default `false`, the mine kept as since 0.11.2); a struct literal
  needs it or `..Default::default()`. `s2_ai::project::mine_fate` takes a
  seventh value, `keep_waiting`. `reborn.ai_setRecipeRule` with a group good
  now counts each member above its kept stock, no longer the plain sum.
  Without `reborn.ai_set*` calls the computer players play as before: every
  golden state hash and the fidelity measure are unchanged.

## OpenS2X 0.11.3 (2026-11-17)

Ships carry what your islands lack; the pointer picks the field the original
picks; the end screen's Statistics button works; ambient occlusion and the
effects' own colours for projects.

### What is new

- **Ships supply your islands**, as in the original: a construction site or
  building whose roads reach no storehouse with the goods it needs (or no
  storehouse that can send a settler) is put on the list of the nearest
  harbour of yours on its island that has a ship of yours on its sea. That
  harbour asks your other harbours, which gather the goods and ship them
  over; once they are unloaded, the waiting buildings are served from the
  harbour. Before, ships carried goods only for expeditions.
- **The field under the pointer** is found as the original finds it: over
  the fields' own points, the first one hit along the line of sight in the
  order the original goes through the map's pieces on screen. On slopes and
  behind ridges a click now lands on the field the original would choose.
- **The end screen's Statistics button** opens the statistics window over the
  end screen, as in the original; its close button returns to the end
  screen.
- **A geologist's find** names the geologist in its message, as in the
  original: "Iron ore found! (Geologist)".
- **Under attack**: "Military building lost to enemy!" and "Territory lost
  to enemy!" no longer name a building (the original's do not); a building's
  window closes by itself when an enemy takes the building.
- **The end screen of a free game** no longer stops the game: the world goes
  on behind it, as in the original (after a defeat you watch the winner take
  your land), and the winner's row says VICTORY. On a defeat, CONTINUE keeps
  its label's colour and only its bar is darker.

### For projects built on OpenS2X: breaking changes

- **SHIP-4.1**: a harbour's need comes from the rule: `s2_sim::Order` and
  `s2_sim::Request` have a new field `harbour` (the harbour they are attached
  to); a struct literal needs it. `World::add_harbour_need` is gone (it stood
  in for the rule), and unloaded goods no longer lower the need: the orders
  they serve do. The world snapshot version is 82; saves of 0.6.0 on load.
  The golden state hashes changed in their world, wares, settlers and
  players parts only (the new field); no game plays differently where no
  harbour and ship are involved, and the fidelity measure is unchanged.
- New hook **W17b**, off by default (docs/extending.md): screen-space ambient
  occlusion on the map, `RenderOptions::ambient_occlusion`; a struct literal
  of `RenderOptions` needs it or `..Default::default()`. The terrain and object
  materials now draw a depth and normal prepass for any camera that asks for
  one.
- **W33**: a project's particle effects (`Extensions::effects`) are drawn in
  the effect file's own colours, as the collapse dust is (they were drawn
  white, showing the texture's own colour).

## OpenS2X 0.11.2 (2026-11-16)

Military buildings keep their distance as in the original; the coins
victory counts the coins your mints make; a leveller's and a forester's work
raises dust; the hooks a project asked for to finish its mods.

### What is new

- **The build menu** greys the barracks, guard house, watchtower and
  fortress where they would stand within four fields of another military
  building or construction site (yours or an enemy's, the headquarters
  included), with the tooltip "Too close to other military buildings", as in
  the original; the game refuses them there too. The lookout tower,
  workshops and storehouses are not affected. A building a mission forbids is
  greyed with "Building not allowed".
- **Collect 100 gold coins** (a free game's victory condition) counts the
  coins your team's mints have made since the start, as in the original:
  coins in the headquarters, from the start or from the cheat do not count,
  and coins you spend still count. The panel under the main bar shows your
  own count, "(n/100)".
- **Dust** rises around a leveller's shovel and a forester planting a tree,
  as in the original: once each time he starts that work, at his feet,
  fading a few seconds after he stops.
- **A ship waiting for orders** says which ship it is in its message: "Ship is
  awaiting orders (Aurora)".
- **Carriers and settlers when a building goes**, as in the original: a carrier
  carrying a ware into a building that is torn down meanwhile brings it back
  out and puts it down at the flag, where it is sent on (it was lost); a
  worker whose building is torn down while he carries his product loses it at
  once; a builder whose site goes while he walks back to its flag starts home
  a moment later.
- **Geologists and scouts** never pick the same spot as another one at work;
  one with no spot left goes home at once.
- **Statistics**: the goods curve counts what lies in your warehouses only
  (no longer the workshops' supplies).
- **Settlers stepping aside**: one walking past a standing settler steps aside
  alone; two meeting step aside to opposite sides, as in the original.
- **Your profile**: a first start uses `default01.cfg`, the original's own
  profile file, whatever other files the profiles folder holds; a profile an
  earlier version of OpenS2X chose stays.
- Nothing of the game's own checks writes your settings, profile or saves any
  more (our picture checks used to rewrite the profile with the same
  contents).

### For projects built on OpenS2X: breaking changes

- `s2_sim::Rejection` has a new variant `TooCloseToMilitary` (ECO-PLACE-6);
  `World::counts_as_military`, `World::military_near` and
  `s2_sim::MILITARY_CLEARANCE` are new. A project's player who placed
  military sites that close is now refused.
- `s2_sim::Message` has a new field `ship` (the ship a message names); a
  struct literal needs it.
- `s2-app profile-seed FILE` (new) writes the profile the golden pictures
  start every scene with; `--still` forgets the pointer, so no hover depends
  on where the desktop's pointer lies. The world snapshot version is 81; saves of 0.6.0
  on load. The golden state hashes changed in their world and players parts
  only (every message carries the field); no game plays differently.
- The coins victory reads `World::coins_produced`, not the warehouses' stock.
- New hooks, each off by default (docs/extending.md): **W22** a project's
  menu page longer than eleven rows, or with the new `PageWidget::PageBreak`,
  shown a part at a time with a switch at its foot (a `match` on
  `PageWidget` needs an arm for it); **W23** the production window lists a
  type's recipes when it has three or more; **W24** `Extensions::messages`
  (`ProjectEvent`): a message in the project's words when a mine's deepening
  is done or a road is paved; **W25** `reborn.ai_setDeepen`,
  `reborn.ai_setPaving`, `reborn.ai_setRecipeRule`: the computer players use
  the project's deepening, paving and recipes (`RuleSet::ai_project`,
  `AiConfig::project_rules`); and `Extensions::button_pictures`: pictures of
  the project's own for the Deepen and Next recipe buttons. A struct literal
  of `Extensions`, `RuleSet` or `AiConfig` needs the new fields or
  `..Default::default()`.

## OpenS2X 0.11.1 (2026-11-16)

The program says which version it is; lesson 4's second ship founds its
harbour; the end screen names the computer players; fog of war Off works as
in the original.

### What is new

- **Which version you have**: right-click `OpenS2X.exe`, Properties,
  Details (Product version); or the first line of the game's new log file,
  `%APPDATA%\s2-reborn\log.txt`; or `OpenS2X.exe --version` in a command
  prompt. The log keeps the program's messages and, when it stops with an
  error, where it stopped: attach it to a bug report (PLAYING.md, "Which
  version you have").
- **Tutorial lesson 4**: the second ship's **Construct harbor** founds the
  harbour on the island, as in the original (before, the button showed but
  nothing was built).
- **Ships** remember their last port as the original does: a ship made by a
  mission starts with the land beside it, a docked ship its harbour, and a
  ship under way none. On an expedition, **Construct harbor** shows exactly
  where a harbour can be founded, for every ship.
- **The ship window** draws its two separator lines; a finished ship's
  message reads "Ship constructed", at the ship.
- **The attack menu** opens only on an enemy's headquarters or an enemy
  military building with soldiers in it, as in the original (a lookout
  tower, a harbour, a storehouse, a workshop or an empty military building
  opens nothing). With none of your soldiers in reach it says "Cannot attack
  this building!" where the number stands, without the slider and ATTACK.
- **The end screen** of a free game names the computer players "AI", as the
  original does.
- **Fog of war Off** on the free game setup uncovers the whole map for every
  player at the start, the computer players included, as in the original:
  they see the whole map too, which changes how they play (before, only the
  drawing left the fog out).

### For projects built on OpenS2X: breaking changes

- New hook W21, off by default (docs/extending.md, "The look of a paved
  road"): a project's street textures for paved roads,
  `RenderOptions::paved_road` (`StreetLook { texture, normal }`); a struct
  literal of `RenderOptions` needs it or `..Default::default()`.
- `s2_sim::GameSetup` has a new field `fog_of_war_off` (default `false`); a
  struct literal needs it or `..Default::default()`. Set, the new game
  explores the whole map for every player as it makes them. The viewer's
  own fog-off drawing switch is gone.
- A ship's last port position (`Vessel::port`) follows SHIP-2.7: set at its
  creation, at docking (the harbour's own field, before the docking
  place's water field) and at an expedition's arrival, cleared at every
  route start. A save of ours keeps the port it holds until the ship's next
  route.
- The world snapshot format is unchanged (79); the state hashes and the
  fidelity measure are as 0.11.0's. The pictures change only by the ship
  window's separators and the end screens' names.

## OpenS2X 0.11.0 (2026-11-15)

Six new hooks for projects built on OpenS2X (W15 to W20), the player's demolish as the
original's, the sliders and end screens as the original draws them.

### What is new

- **Demolishing a building gives material back**, as in the original: part
  of what it cost lands on its flag for the carriers, and a finished
  building leaves a burning ruin for about half a minute. The flag and its
  roads stay.
- **The sliders** of the tools, military and distribution dialogs, the
  headquarters' military page, the attack dialog and the settings pages
  are the original's pictures: a dark bar, the cyan fill up to the knob and
  the orange knob.
- **The end of a free game** shows the original's end screen: the map's
  name, victory or defeat with its reason, a row per player with their
  team and result, and Statistics, Exit map and Continue (no Continue
  after a defeat). A lost campaign mission shows the original's defeat
  page, with Exit map only.
- **The tutorial's dialog steps** take only the control they mark (lesson
  4's close buttons, the economy dialog's military tab and close).
- The minimap dialog sits at the screen's right edge as in the original,
  its buttons where the original has them; during a tutorial lesson the
  minimap stays closed until the lesson ends. The `,` key opens the next
  storehouse's window at the mouse pointer.
- **Tooltips** of buttons and controls are the original's framed box, above
  and to the right of the pointer; a build menu cell's tooltip names the
  building and how many of it the player has ("Woodcutter's Hut (0)").
- **The ship window** as the original lays it out: twelve cargo places in
  two rows, Jump to port greyed while the ship has no port to go to, and on
  an expedition Build harbour only where a harbour can be founded, with the
  six direction buttons around the picture while the ship waits. The
  harbour's expedition page reads -/- until a preparation starts.
- **Messages**: a left click on a message's icon marks it read (the tick
  over it, the text grey) and jumps to its place; a right click deletes it.

### Fixed

- A worker whose way to his building was cut no longer waits on a flag
  for good: he goes home, and his building sends for another once he is
  back (as the original).

### For projects built on OpenS2X: breaking changes

- New hooks, each off by default (docs/extending.md): recipes a player
  chooses per building (`reborn.bp_addRecipe`, `bp_addRecipeInput`,
  `Command::ChooseRecipe`) and a good's distribution table taking a
  project's type (`reborn.dist_addConsumer`, W20); a group good in a recipe
  and a kept stock (`reborn.cfg_setRecipeKeep`, W16); deepening an
  exhausted mine (`reborn.bp_setDeepening` and companions,
  `Command::DeepenMine`, W15); paved roads (`reborn.cfg_setPaving`,
  `Command::PaveRoad`, W19); a softer shadow filter, a factor on the
  profile's shadow map and shore foam (`RenderOptions::shadow_filter`,
  `shadow_map_scale`, `shore_foam`, W17, W18); particle effects at
  settlers at work (`Extensions::effects`, W18).
- `s2_sim::Command` has the variants `ChooseRecipe`, `DeepenMine` and
  `PaveRoad`, `s2_sim::Rejection` `NotARecipe`, `NotDeepenable` and
  `NotPavable`: a `match` on either needs arms for them.
- `s2_sim::Rules` has the fields `recipe_sets`, `deepenings`,
  `recipe_groups`, `recipe_keep` and `paving`, `s2_rules::model::Building`
  `recipes` and `deepening`, `s2_rules::RuleSet` `recipe_keep`, `paving`
  and `dist_consumers`, `s2_viewer::RenderOptions` `shadow_filter`,
  `shadow_map_scale` and `shore_foam`, `s2_viewer::Extensions` `effects`:
  a struct literal needs them or `..Default::default()` / `..Rules::empty()`.
- `Command::RemoveBuilding` now removes with remains (material on the flag,
  a ruin), as `TearDown` does: a project's game in which a player demolishes
  plays differently from that command on.
- The world snapshot is format 79 (76 to 79 for the four hooks with game
  state); every save of 0.6.0 and later loads. The original's rules have
  none of the hooks: the state hashes and the fidelity measure are as
  0.10.x's; the view's pictures change only by the sliders, the minimap's
  place and buttons, the tooltips' box, the ship window and the end screens.

## OpenS2X 0.10.1 (2026-11-15)

The first public release. The game plays as 0.10.0, the last release made
before the public repository; ships are now drawn as the original draws
them, and the documents and pictures show this release.

### What this release holds

- **Free games** on every map of your install, up to its player count,
  against Weak, Normal and Strong computer players: the three tribes, start
  resources, the condition for victory, fog of war.
- **The campaign and the tutorial:** the ten missions and the four lessons,
  with their briefings, scripted events and victory and defeat books.
- **The game's rules**, written from specifications of the original's
  behaviour: the economy, roads, carriers and donkeys, warehouses,
  territory, soldiers, attacks, catapults, ships, harbours and expeditions.
- **Saving and loading** in OpenS2X's own format, which every later
  version loads; the original's saves load and play on.
- **The original's look and sound**, read from your copy: its models,
  animations, shading, day and night, its menus, settings pages, in-game
  bar and windows; effects, ambience and the tribes' music (on Windows).
- **How close it is**, measured on this release: in the recorded
  two-player game against a Weak computer player, every settler's place and
  every building equal the original's at every compared tick to 40,000 (67
  minutes of game time), and its random generator for the first 51,723
  ticks; against a Strong computer player for the first 48,042 ticks. In
  the picture comparison 52 of 62 regions are within 3 % of the original.
  The README has the details.
- **Not there yet:** multiplayer, saving in the original's file format, the
  tutorial's "Info" pages and spoken end, the ending film; sound on Linux.

### Fixed

- **Ships** take the player's colour only on their sails and banner, as a
  tint, as in the original. The hull, deck and oars keep their wood
  (they were drawn in the player's colour all over).

### What is new

- `PLAYING.md` describes the six settings pages, Single player opening on
  the campaign's island map, and the tutorial's window and arrow.
- The README's numbers are measured on this release, and its coast and
  windows pictures show this release's ship and building window.

### For projects built on OpenS2X: breaking changes

- Only pictures: ships are drawn differently (GFX-OBJ-2.8a), so a
  project's own reference pictures with a ship change; look at them before
  accepting them again. No public item of the crates or the hooks, no save
  or snapshot format and no game state hash changed.

## OpenS2X 0.10.0 (2026-11-15)

Every window of the game put beside a picture of the original and redrawn
to match, the campaign's books, three more settings pages, fights as the
original fights them, and a first minute without stutter. Two hooks for
projects built on OpenS2X.

### What is new

- **Windows as the original draws them.** Each window and page was compared
  with a picture of the original. A building's window shows what it needs
  and has (`0/6`; a mine's foods, `2/2` each) and no tool slot for a worker
  who needs none, such as the well's; the construction site, the harbour
  ("Preparing expedition!"), the catapult and the shipyard have their own.
  The economy dialogs (tools, military, distribution, transport) open
  centred on their tall frame. The headquarters' military page has a row
  per rank, Private to General, with a slider for the reserve it keeps and
  how many are there. The mission panel and the buildings statistics page
  are on their own frames.
- **The campaign's books.** A mission's briefing opens as the original's
  book, centred, with its page count between the arrows and the mission's
  name as its title (MISSION 1). A won mission shows the victory book at
  once with the debriefing's pages: **Continue** plays on, **Exit map**
  goes to the island map. The defeat book likewise.
- **The tutorial's window.** Tall at the top while it explains (Cancel and
  Continue), wide at the top right while it waits for a click. An orange
  arrow points at the button or build menu cell to press, a wrong click
  shows what was expected in red, and a step that wants one tab or button
  takes only that one.
- **Settings.** The Graphics, Input and Network pages are there, so every
  tab of the settings dialog opens a page, with centred captions and the
  original's Reset button. The Input page's choices are kept in your
  profile (scroll speed, mouse and border scrolling, the smart cursor, the
  free camera).
- **Menus.** Single player opens on the campaign's island map, as in the
  original; Exit asks first. A list's scroll arrow repeats while held, and
  the scroll bar can be pressed and dragged. START says "Ready to start" or
  "Double allocation of HQ".
- **Fights as in the original.** An attacker steps aside on his waiting
  field before the duel starts, so a duel starts a moment later; he walks
  up to the target's flag, never onto a building's field; an interceptor
  who won his duel waits outside and takes on the next attacker; a soldier
  who takes a building walks in from its flag. A saved game's duels go on
  with the soldiers where they stood.
- **A smoother first minute.** Models, animations and sounds are read ahead
  in the background, so a slow disk no longer stutters when a game first
  shows them. The developer's tools play a game ahead much faster again.
- **Campaign 5's** portals test the right player. Settlers keep their
  colours in long games.

### For projects built on OpenS2X: breaking changes

- `s2_viewer::extend::MusicSlot` has `Briefing` and `Jingle(bool)` (hook W13:
  the briefing book's music and the end dialog's jingles) and is
  `#[non_exhaustive]`: a `match` needs the new arms and a `_` arm.
- Hook W12: the computer players leave every building type an overlay
  created out of their producer tables; `reborn.bp_setAiAlternative` says
  when they build one (`docs/extending.md`). `s2_rules::model::Building`
  (`ai_alternative`) and `s2_rules::RuleSet` (`project_buildings`) have new
  fields.
- `s2_assets::s2m::Trigger`'s fields are named by their meaning
  (`reserved`, `active`, `player`, `timer`).
- The world snapshot is format 75 (an attacker's step, an interceptor's
  defended building): saves of 0.6.0 and later load. `s2_sim::AttackRecord`,
  `s2_sim::RestoredFight` and `SettlerJob::Defend` have new fields. Golden
  hashes are unchanged.

## OpenS2X 0.9.7 (2026-11-15)

Documents and pictures only. The game plays exactly as in 0.9.6.

### What is new

- New pictures in the README: the two windows side by side, a coast with a
  ship, and the original beside OpenS2X in a busy town. The fidelity chart
  says exactly what a bar is.
- The documents explain what the public CI runs and what runs only on the
  maintainer's machine, how versions are numbered in practice, and where
  the public history starts. The maintainer's working notes are out of them.
- `PLAYING.md` reads more easily: shorter entries, one word for a
  mission's goal (the portal).
- The developer's viewer (`s2-viewer`) can open a building's window at a
  chosen point (`--context-at`) and frame a ship by the shore
  (`--scene ship-shore`) for screenshots.

### For projects built on OpenS2X: breaking changes

- None. No public item of the crates or the hooks, no save format, no golden
  hash and no golden picture changed.

## OpenS2X 0.9.6 (2026-11-14)

The project's public face. The game plays exactly as in 0.9.5.

### What is new

- The README has pictures: a town, a coast, the windows, the original
  beside OpenS2X at the same tick, and a chart of how many ticks match the
  recordings of the original. It also states the goals and the
  scope, and answers the common questions.
- The picture comparison was measured again on this release.
- This changelog is written by hand for players and developers. The
  maintainer's task-by-task log stays in the working repository.
- `CONTRIBUTING.md`, `SECURITY.md`, a code of conduct and issue templates
  (a bug, a difference from the original, a question).
- `PLAYING.md` is written for someone who has only the download.
- The download's README leaves out the pictures and badges, which only
  GitHub shows.

### For projects built on OpenS2X: breaking changes

- None. No public item of the crates or the hooks, no save format, no golden
  hash and no golden picture changed.

## OpenS2X 0.9.5 (2026-11-14)

### Fixed

- Dirt roads no longer leave a bare circle at a flag. The first road built
  to a flag runs to it; later ones stop a little short under the crossing.
- Roads and the stone patches at flags are lit from above, as in the
  original: no dark purple cobbles, and dirt roads a lighter brown.
- Building windows on the original's own frames: the title inside the
  header, the labels above their frames, the productivity percentage inside
  its bar, and the status lines "No worker in building" and "Production
  stopped". The headquarters, military buildings and the flag menu likewise.
- The statistics window keeps its top-left corner on every tab.
- The messages window is shown at every game start, and new messages play
  their sound, without switching anything on. N hides it for that game.
- The Select Map, Save and Load dialogs and the settings pages on the
  original's frames, with its tabs, drop-down lists and Reset button.
- The map list is in the original's order and its scroll bar moves as the
  original's. New computer players take their slot's number for position,
  colour and team.

### For projects built on OpenS2X: breaking changes

- The view and the windows draw differently, so a project's own pictures of
  the view change: look at them before accepting them again. No public item,
  save format or golden hash changed.

## OpenS2X 0.9.4 (2026-11-14)

### What is new

- The public repository is prepared,
  [Jensenzo/OpenS2X](https://github.com/Jensenzo/OpenS2X): one commit per
  release, made from the release's tree (`docs/releasing.md`, "The public
  copy").
- The public copy's check also refuses links that lead nowhere.

### For projects built on OpenS2X: breaking changes

- None in the crates. A project pins `https://github.com/Jensenzo/OpenS2X`
  by tag, from its first public release on (`docs/releasing.md`, "Pinning a
  release").

## OpenS2X 0.9.3 (2026-11-13)

### What is new

- The program's public name is OpenS2X: `OpenS2X.exe`, the window's title
  and the download `OpenS2X-<version>-windows-x64.zip`. Settings made by
  earlier versions (`%APPDATA%\s2-reborn`) are found as before.
- A save with a fight in progress goes on where the fight stood.
- The Select Map list has the original's scroll bar; a player slot's colour
  square steps through the colours.
- Settlers' colours, a ship's heading and its bobbing turn as in the original.
- The shadows switch takes effect at once; a collapsing house has its
  animals and sounds.

### For projects built on OpenS2X: breaking changes

- The game's binary is `OpenS2X` (`cargo run -p s2-viewer --bin OpenS2X`; it
  was `s2-reborn`) and the window's default title is `OpenS2X`. The crates
  keep their names.
- The world snapshot is version 73: soldiers, builders and levellers going
  home stay linked to the building that sent for them. Saves from 0.6.0 on
  load. Golden hashes changed.
- `s2_sim::Restore` has a new field `fights`, `RestoreReport` the fields
  `fights` and `fights_skipped`, and `s2_assets::sav::SavedSettler` a new
  field `soldier`.
- The view draws differently (settlers' colours, ships).

## OpenS2X 0.9.2 (2026-11-13)

### What is new

- A building that is torn down or burnt collapses onto its ruin, and the ruin
  keeps the type of the building it was.
- The small decorations (doodads) under a new road, flag or building stay
  gone, as in the original.
- In the original's saves, harvesting workers, hunters and woodcutters go on
  where they were, and a fight in progress is read.
- The engine settings page with the original's presets.
- Water: the deep sea's colour within 1 % of the original by day and at
  night, the map edge as the original draws it, shadows on the water.
- Buildings in their own colour slot; door walls in their roof's shadow; the
  original's night sky.

### For projects built on OpenS2X: breaking changes

- The world snapshot is version 72. Saves from 0.6.0 on load; an older
  game's ruin is not drawn. Golden hashes changed.
- New fields: `s2_sim::Ruin::building_type`, `RestoredOutdoor::leg`,
  `Restore::workers_waiting`; `s2_assets::sav::SavedSettler::harvest` and the
  fight fields of `SaveGame`.
- `UserObject::default()` has the scroll speed 0.25 and
  `SceneSettings::default()` the shadows switched on. Decide shadows by
  `SceneSettings::shadows()`, not by the quality.
- The view draws differently.

## OpenS2X 0.9.1 (2026-11-11)

### What is new

- The tests that compare with the recordings of the original skip with a
  line when the recordings are missing, so the tests pass on a copy without
  them.

### For projects built on OpenS2X: breaking changes

- None.

## OpenS2X 0.9.0 (2026-11-10)

### What is new

- The first release with a Windows download: the program, its documents
  and the notices of the libraries it is built with.
- In the original's saves, settlers at work go on where they were: workers
  out on the land, builders on their way, ships with their cargo.
- Open water drawn as the original draws it, with shadows; the terrain's
  normal maps at their own size.
- Homeless settlers wander between their searches for a home, as in the
  original.
- Campaign mission 4: the area around each goal starts explored.

### For projects built on OpenS2X: breaking changes

- The world snapshot is version 70. Saves from 0.6.0 on load. Golden hashes
  changed.
- `s2_sim::Restore` and `RestoreReport` have new fields for an original
  save's settlers at work; `s2_assets::sav::SavedMovement::stop` (a bool) is
  `stop_short` (a count), and `SavedMovement`, `SavedSettler`,
  `SavedBuilding` and `SavedShip` have new fields.

## Earlier development

Versions 0.1.0 to 0.8.0 (October and early November 2026) built the
simulation, the computer players, the 3D view, the interface, sound and
music, the campaign and the tutorial, and the comparison with recordings of
the original. They are summarised here rather than listed.
