# Architecture

## Goals

1. A clean, modifiable codebase that plays like The Settlers II: 10th Anniversary.
2. Game rules isolated in one deterministic crate, so they can be tested, tuned and
   replayed without graphics.
3. Original assets loaded at runtime from the player's own install. Nothing
   copyrighted ships in this repo.

## Two-workspace workflow

| Side | Location | Has access to | Produces |
|---|---|---|---|
| Investigation | a private workspace of its own | the original game: its program and its data | `specs/*.md` in this repo: the rules in plain words, each with the evidence id it rests on |
| Implementation | this repository | specs only | code and tests |

The investigation's raw evidence stays in its own workspace, filed under the
evidence ids the specs cite (`E-mil-002`), and is never committed here.

## Crates and their dependencies (audit of 2026-10-18)

```
   s2-app (bin)            s2-viewer (bin + lib, Bevy 0.20)
      |  \                     /   |
      |   +------> s2-ai <----+    |
      |             |  \           |
      +-----> s2-rules  \          |
      |          |   \   \         |
      +-----> s2-assets \ \        |
      |          |       v v       v
      +--------> s2-sim <----------+
```

Every arrow is a path dependency in `Cargo.toml`; `s2-ai` depends on `s2-sim`,
`s2-rules` and `s2-assets`, the two binaries on all four. `s2-sim` has no
dependencies at all. The only third-party dependencies are `mlua` (vendored Lua
5.1) in `s2-rules`, and Bevy, `zune-jpeg` and on Windows `rfd` and
`windows-sys` in `s2-viewer`. Nothing points outward: each crate depends
only on those below it.

| Crate | Job | Lines (src + tests) | Largest files |
|---|---|---|---|
| `s2-sim` | all game state and rules, fixed tick, commands | 27 200 | `production.rs` 2 570, `transport.rs` 2 260, `requests.rs` 1 610, `lib.rs` 1 500, `construction.rs` 1 440, `navy.rs` 1 350 |
| `s2-assets` | the original file formats: rc00, s2m maps, saves, Granny models, DDS/TGA/BMP, WAV, XML, cfg | 11 480 | `s2m.rs` 1 820, `sav.rs` 1 120, `gr2/mod.rs` 800, `rc00.rs` 780, `gr2/model.rs` 650, `gr2/oodle1.rs` 620 |
| `s2-rules` | runs the property scripts in Lua 5.1 into a `RuleSet`; maps it to `s2_sim::Rules`; mission scripts; the scripted human of `scenario` | 6 900 | `script.rs` 1 200, `lib.rs` 810, `model.rs` 600, `scenario.rs` 500, `mission.rs` 500 |
| `s2-ai` | the computer players: read `&World`, return `Command`s | 10 000 | `military.rs` 1 080, `roads.rs` 810, `needs.rs` 690, `player.rs` 530 |
| `s2-app` | the headless executable (`decode`, `map-info`, `dds`, `voice-track`, `rules`, `game`, `ai`) | 830 | `main.rs` |
| `s2-viewer` | 2D and 3D view, menus, in-game interface, sound, films, play harness | 37 400 | (before the structural run) `view3d.rs` 3 750, `lib.rs` 3 660, `ui/game.rs` 3 610, `scene3d.rs` 3 130, `menu.rs` 2 420, `models3d.rs` 1 810; since then no file over 1 000 |

### Public surface

- **sim**: `World` (about 230 `pub fn`s: queries, `submit`, `step`,
  `state_hash`, `restore`), `Command` and `CommandResult`, `Rules` with its
  parts (`BuildingType`, `GoodRules`, `DistributionRules`, `MilitaryRules`, ...),
  `GameSetup`, and every state type with **all fields public**: `Settler`,
  `SettlerJob`, `Work`, `Carrier`, `CarrierTask`, `Ware`, `WareLocation`,
  `ConstructionSite`, `Builder`, `Leveller`, `Warehouse`, `Vessel`,
  `ShipState`, `MilitaryBuilding`, `Duel`, `AttackStep`, ... Public modules
  `grid`, `map`, `mission`, `movement`, `navy`, `statistics`, `transport`.
  70 public structs, 27 public enums.
- **assets**: `GameDir` and one module per format (20 modules, 126 `pub fn`s),
  each with a `load`/`parse` entry and plain-data results.
- **rules**: `load`, `load_with_overlays`, `RuleSet` (and `sim_rules()`),
  `new_game`, `game_setup`, `new_world`, `restore_game`, `mission`,
  `scenario::Human`, the name tables (`START_STOCK`, `TOOL_SLOTS`, ...).
- **ai**: `AiSystem` (`new`, `activate_all`, `tick(&World) -> Vec<Command>`),
  `AiConfig`, `Profile`, and 13 public modules, among them `run::Runner` (the
  headless game loop with statistics).
- **viewer**: a binary with a library half (545 `pub fn`s), all of it internal
  to the program; nothing else depends on it.

## Determinism audit

The rule is: same map, players, seed and command stream give the same state on
every run and every machine.

### Graphics in the simulation

None. `s2-sim` has no dependencies and no rendering types. The positions the
view needs (`FinePosition`, the walker's progress in millionths of a step) are
integer game state.

### Wall-clock time

- `s2-sim`, `s2-rules`: none.
- `s2-ai`: `run::Runner::step` measures `ai_time` and `sim_time` with
  `Instant` for its report. The measurement never feeds back into a decision.
- `s2-viewer::advance_world` decides **how many ticks run in a frame** from
  `time.delta_secs()`. The simulation itself is unaffected, but the tick a
  click's command lands in depends on frame timing, and no command log is kept,
  so a played game cannot be replayed (structural problem 2 below).

### HashMap / HashSet iteration order

- `s2-sim`: no `HashMap` or `HashSet` anywhere; all keyed state is `BTreeMap`
  or `Vec`. `state_hash` relies on this.
- `s2-ai`: `roads.rs` (`flag_search`, `road_stop_search`) and `sites.rs`
  (`route_lengths`, `Scan::routes`) use hash maps and sets **for lookups
  only**; the searches iterate their `VecDeque`s and the direction order.
  `run.rs` keys its conquest statistics by field, also lookup only. No order
  dependence found.
- `s2-rules`: `model::Table::index` (lookup only). `script::record_names`
  iterates the Lua globals table, whose order is Lua's hash order; the result
  goes into a `BTreeMap` by name, so only a name given to two ids would depend
  on it (none in the shipped scripts).
- `s2-viewer`: 189 uses, none of which reach the simulation.

### Floating point in game logic

- `s2-sim` keeps `f32` in four places, because the specs give these as the
  original's single-precision values: the distribution weights and
  accumulators (ECO-DIST, "float32"), the tool priorities (ECO-PROD-7), the
  military settings and their accumulator (MIL-SOL-7), and the attack and
  defender fractions (MIL-ATK). They are stored as `u32` bits, so `state_hash`
  hashes bits, and only `+ - * /`, comparisons, `trunc` and `as` casts touch
  them. Rust never contracts these into fused multiply-adds and uses SSE on
  x86-64, so they give the same bits on every IEEE-754 target. No
  transcendental function (`sin`, `powf`, `exp`, ...) occurs in `s2-sim`.
- `s2-sim::restore`: `Restore` carries the save file's `f32` progress, turned into
  integer work once at load.
- `s2-ai` scores sites and attack targets in `f32` (83 uses), with the same
  basic operations, `round` and `total_cmp` with a tie-breaker on the field.
  Deterministic for the same reason.
- `s2-rules` turns the scripts' Lua numbers (`f64`) into integer rules with
  `round`, `ceil` and `floor` once at load. The numbers come from Lua's
  `lua_str2number`, the C library's `strtod`: glibc on Linux and the MinGW
  runtime on Windows. A one-ulp difference between them would change a rule
  only at an exact .5 boundary. Only a Linux-versus-Windows hash comparison
  on the real maps can check it.

### Thread scheduling

- `s2-sim`, `s2-ai`, `s2-rules`: single-threaded, no `thread::spawn`, no
  atomics, no `rayon`.
- `s2-viewer` runs on Bevy's multi-threaded executor, but the world lives in
  the `Viewer` resource and every system that writes it (`advance_world`, the
  interface systems that `submit`) is in one `.chain()`ed set for the map view,
  so their order within a frame is fixed.

### Order that depends on the compiler

- `sort_unstable` is used on whole elements or on unique keys (`serial`) only,
  so ties cannot reorder distinguishable items.
- `state_hash` hashes the `Debug` text of the world. That text is stable for
  integers and our own derives but is not a guaranteed format, and the
  toolchain was not pinned: golden hashes could change with a Rust upgrade
  (fixed: `rust-toolchain.toml`).

### What the audit found to fix

1. No pinned toolchain (golden hashes need one).
2. The headless game loop (AI commands, submit, step, mission script) exists in
   four copies: `viewer::advance_world`, the viewer's `--run`, `ai::run::Runner`
   and `app ai`. They agree today, but only by care.
3. No command log and no save of our own: neither a played game nor an AI game
   can be reproduced from a file.

None of these made a run differ from another run in the existing determinism
test (`crates/rules/tests/determinism.rs`, two runs and a replay of
`tutorial01`, 20 000 ticks).

### Determinism checks (since the safety-net run)

- `s2-app hash MAP TICKS [--seed S] [--every N]` prints, every N ticks, the
  world's full hash, the AI's (`AiSystem::state_hash`) and the world's hash
  per part (`World::state_hashes`: rng, settlers, wares, buildings, flags,
  roads, players, land), so a divergence shows its tick and its part.
- `tests/golden/hash/*.txt`: three install maps (`MP_2P_Storm_Coast`,
  `MP_4P_crossroads`, `MP_6P_Paradise_Island`) and two synthetic games
  (`fixture:flat2`, `fixture:flat4`, from `s2_ai::fixture`, no install needed)
  at 5 000 ticks. `crates/app/tests/golden.rs` makes them again; CI runs the
  fixtures, `scripts/golden.sh` all of them on the PC, with the Windows build
  next to the Linux one. `--bless` accepts a change; CI's
  `golden-changes-named` job refuses a commit that changes a golden file
  without naming a spec rule.
- CI hashes the fixtures on Linux (debug build) and Windows (MSVC, release)
  and diffs the two (`same-hashes`), weekly and by hand; before every push
  `scripts/golden.sh` compares the Windows build with the Linux one
  (`docs/releasing.md`, "CI").
- `crates/sim/clippy.toml` and `crates/ai/clippy.toml` forbid `HashMap`,
  `HashSet`, `Instant`, `SystemTime` and `thread::spawn`; the AI's lookup-only
  hash maps became `BTreeMap`s so its state can be hashed too. `ai::run`
  (the report's names and timing) is the one allowed exception.
- The real maps were also compared by hand at 60 000 ticks
  (`MP_4P_crossroads`): Linux and Windows builds, and debug and release,
  give the same lines.

## The viewer

`crates/viewer/src`, one directory per job (since the structural run):

| Directory | Job | Main parts |
|---|---|---|
| `app/` | the program: command-line options and usage, the Bevy app with its plugins and system order, the game folder | `mod.rs` (`run`), `options.rs` |
| `game/` | the running game: the viewer's state, the game clock and the step of the world with its computer players, loading a map or save | `mod.rs`, `load.rs`, `names.rs` |
| `input/` | the 2D view's hover and camera keys, the 2D/3D switch, the debug mode's keyboard editing | `mod.rs`, `edit.rs` |
| `render/` | drawing: the 3D view (`view3d/`: materials, scene build, camera and picking, overlays, objects, movers, building animations), terrain, models (`models3d/`), textures, colours, world geometry (`world3d/`), the 2D map view (`scene.rs`, `view2d.rs`, `terrain.rs`), the ship window's view, shaders | |
| `world/` | scene building: what the 3D view draws from the simulation (`scene3d/`: objects, movers, streets), decorations, roads, ware stacks, carried items, ships | |
| `animation/` | clips, poses and skinning, which clip a settler plays, object animations | |
| `ui/` | the in-game interface (`game/`: context dialogs, building windows, global dialogs, HUD), widgets, drawing of the UI, the debug panels (`economy.rs`, `military.rs`, `info.rs`) | |
| `menus/` | main menu and setup pages (`menu/`), campaign and tutorial, the settings file | |
| `audio/` | sound playback and sound definitions | |
| `films/` | the films and subtitles | |
| `tools/` | the play harness (`play/`), colour probe, model view, screenshots and `--compare`, the `--scene` set-ups | |

No source file is over 1 000 lines; test modules live in `tests.rs` files
beside their module (the largest about 1 050 lines). A file that was split
keeps its parts together through `use super::*` in each part and `pub(crate)
use part::*` in its `mod.rs`, so the split changed no path outside it; the
modules made from the old `lib.rs` import what they use by name.

The viewer is still one crate. Splitting it into crates would need the
shared state (`Viewer`, the model library, the UI state) as a public API
between them; the directories already separate the jobs without that cost.

## Where the viewer reaches into the simulation

*Closed in the structural run (2026-10-19): the viewer reads settlers
through the view model (`s2_sim::view`) and the specs' records only;
`scripts/api-boundary.py` checks it on CI; `docs/api.md` lists the API.
What follows is the audit as it was.*

The viewer never changes the world except through `submit` and `step`, with
three exceptions, all debug or scenario set-up:

- `scenes.rs` (`--scene ship`) calls `World::debug_add_ship` and
  `debug_launch_ship`, and the scene builders submit `Command::DebugSupplySite`.
- `lib.rs` calls `World::set_mission` when it loads a mission map.
- `campaign::after_step` runs the mission script after every step and submits
  its commands; this belongs to the game loop, not to the view.

It *reads* far more than a view needs, because the simulation has no view model
and every state type is public:

| What the viewer matches on | Files | Places |
|---|---|---|
| `SettlerJob::*` | 7 | 34 |
| `Work::*` (a worker's step) | 3 | 25 |
| `ShipState::*`, `ExpeditionState::*` | 2 | 18 |
| `Builder`, `BuilderPose`, `Leveller` fields | 4 | 25 |
| `CarrierTask::*`, `transport::idle_field` | 1 | 4 |
| `movement::Walker` internals, `movement::ONE` | 3 | 15 |
| `AttackStep`, `DefendStep`, `SpecialistStep`, `CatapultPhase` | 2 | 4 |

The animation choice (which clip, which pose, how far through it) is derived in
the viewer from these internal state machines (`settler_anim.rs`, `carry.rs`,
`animate3d.rs`, `world3d.rs`). The scene set-ups (`scene.rs`, `scene3d.rs`,
`economy.rs`) also step the world themselves to stage a picture.

## Structural problems, ranked

1. **The view depends on the simulation's internal state machines.** About 130
   places in the viewer match on `SettlerJob`, `Work`, `CarrierTask`,
   `ShipState` and builder internals to choose an animation. Any change to how
   a job is modelled breaks the picture, and the rules for "what is this
   settler doing" are split between two crates. A small, stable view model
   (per settler: kind, field, fine position, animation id, clip time, carried
   ware) produced by `s2-sim` would remove most of it.
   First step (safety-net run):
   `s2_sim::trace` gives each settler's GFX-ANIM-3.1 action and fine
   position for the trace, and the view's worker requests now come from it
   (`trace::worker_request`).
   *Done in the structural run: the view model `s2_sim::view`
   (`docs/api.md`); the viewer no longer matches on the state machines.*
2. **No single game loop and no command log.** The step "AI decides, commands
   are submitted, world steps, mission script runs" is written four times.
   The viewer keeps no record of the commands it submitted, so a played game
   cannot be replayed, a bug report cannot carry its game, and saving is not
   possible. A `Game` type (world, AI, mission script, command log) in a crate
   both binaries use would hold it once.
3. **No save of our own.** `World` cannot be written out; only the original's
   saves can be read, and only partly (`Restore`: buildings, flags, roads,
   territory, the workers the buildings name and the streets' carriers and
   donkeys, Q-257).
   *Done (2026-10-19): the snapshot (`docs/snapshot.md`) writes the whole
   game state at any tick, and loading it continues the game hash for hash.*
4. **Every state type has public fields.** `s2-sim`'s real API is its whole
   data model (70 structs). Invariants the rules keep (a ware claimed by one
   carrier, a site's delivered count) can be broken by any caller in principle,
   and refactors inside `s2-sim` ripple out to the AI and the viewer.
   *Structural run: the state machines are marked internal in their docs
   and the viewer may not name them (`scripts/api-boundary.py`); the
   fields stay public for `s2-ai`.*
5. **Debug hooks in the game API.** `Command::DebugSupplySite`,
   `World::debug_add_ship` and `debug_launch_ship` live next to the real
   commands. They are clearly named, but a network or replay layer would have
   to filter them.
6. **The viewer crate is a monolith.** 37 000 lines in one crate: game loop,
   2D and 3D rendering, interface, menus, campaign, audio, films and scene
   staging. `lib.rs` alone mixes the app set-up, the game loop, the 2D drawing
   and command-line handling. Build times and test isolation suffer.
   *Structural run (2026-10-19): split into one directory per job, see "The
   viewer" below; still one crate (see there why).*
7. **The state hash is built on `Debug` text.** Simple and complete, but it
   also hashes the static rules on every call (slow for per-tick use), depends
   on the `Debug` format and field names, and a renamed field changes every
   golden hash. Acceptable with the pinned toolchain; a hand-written hash over
   the dynamic state would be faster.
8. **Most integration tests cannot run on CI.** Everything that needs the
   original rules or maps skips without `S2_GAME_DIR`, so CI checks the unit
   tests and the formats' vectors only. The play harness runs on the PC
   (`scripts/play-test.sh`).
