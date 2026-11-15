# Changelog

What changed in each release, newest first. Versions follow semver
([`docs/releasing.md`](docs/releasing.md)): before 1.0 any release may
break a project built on these crates, and says how under "For projects
built on OpenS2X: breaking changes". Releases before 0.9.3 were made under
the working name s2-reborn. The public repository starts with its first
public release; the sections before it describe releases made in the
maintainer's working repository, which have no tag or download there.

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
