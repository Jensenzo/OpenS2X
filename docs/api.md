# The simulation's public API

The viewer, the tools and projects built on these crates use the simulation
(`s2-sim`) only through what this page lists: **commands in, state and events out**, and the **view
model** for what settlers are doing. Everything else in `s2-sim` is its own
business and may change in any release (`docs/releasing.md`).
`scripts/api-boundary.py` checks the viewer against this on every CI run.

The examples below are compiled and run as
`crates/app/tests/api_example.rs`.

## Setting up a game

```rust
// From the player's install: the rules from the property scripts, a map.
let dir = s2_assets::GameDir::from_env()?;                 // S2_GAME_DIR
let (rules, _report) = s2_rules::load(&dir)?;              // or load_with_overlays
let map = s2_assets::s2m::load(name, &bytes)?;
let mut world = s2_rules::new_game(seed, &map, &rules, start_resources)?;

// Or with an explicit setup (players, tribes, teams, resources):
let mut setup = s2_rules::game_setup(&map, &rules, start_resources);
setup.tribes = vec![(PlayerId(1), 2)];
let mut world = s2_sim::World::new_game(seed, map.terrain(), rules.sim_rules()?, &setup);

// Computer players: they read the world and give commands like a person.
let mut ai = s2_ai::AiSystem::new(s2_ai::AiConfig::new(&rules, &rules.sim_rules()?), &world);
ai.activate_all(&mut world, &[(PlayerId(1), 2)]);
```

### A headless game

The game `s2-app hash` (and `soak`, `trace`) plays: a map by its name,
the original's seed, start resources 1 (medium), every player a computer
player of profile 2. Built from these calls it is the same game: the same
state hashes tick for tick (`crates/app/tests/api_example.rs` checks it
against `s2-app hash`).

```rust
let dir = s2_assets::GameDir::from_env()?;
let (rules, _report) = s2_rules::load(&dir)?;
let file = s2_assets::find_map(&dir, "MP_2P_Storm_Coast").ok_or("no such map")?;
let map = s2_assets::s2m::load(&file.to_string_lossy(), &std::fs::read(&file)?)?;
let setup = s2_rules::game_setup(&map, &rules, 1);
let sim = rules.sim_rules()?;
let seed = u64::from(s2_sim::rng::SIM_SEED);
let mut world = World::new_game(seed, map.terrain(), sim.clone(), &setup);
let mut ai = s2_ai::AiSystem::new(s2_ai::AiConfig::new(&rules, &sim), &world);
let players = s2_ai::every_player(&world); // [(PlayerId, 2)] for every player
ai.activate_all(&mut world, &players);
for _ in 0..ticks {
    ai.step(&mut world);
}
let (world_hash, ai_hash) = (world.state_hash(), ai.state_hash()); // s2-app hash's columns
```

A campaign mission or lesson, as the menu starts it, with its script:
`s2_ai::mission::MissionGame::start(&dir, &rules, "campaign01")` and its `step`
(the world, the computer players, the script's calls), `save` and `load`.
`s2_rules::mission::players` says who plays one, and
`s2_rules::setup_of_players` makes the setup of any list of players.
`s2_ai::campaign_player::CampaignPlayer` is a test's player for them, with a
player's commands only (`docs/test-coverage.md`, "Missions"); nothing in the game
runs it.

`s2_assets::find_map(&dir, name)` looks for `<name>.s2m` in the install's
`data/maps/freeGameMaps`, `campaignMaps` and `tutorialMaps`, in that order
(FMT-3.3), or takes `name` as the path of an `.s2m` file; maps are not
looked up in the asset folders. `s2-app`'s `--resources`, `--ai` and
`--closed` are `game_setup`'s start resources, the list given to
`activate_all`, and players removed from `setup.players` before
`World::new_game`.

Also part of setting up: `World::set_mission` (a campaign map's mission,
`s2_rules::mission::mission_of`), `World::restore` (an original save,
through `s2_rules::restore_game`).

## Commands in

Every change to the world is a `s2_sim::Command`, queued with
`World::submit` and applied at the start of the next `World::step`, in the
order submitted. A step is one logic tick (ECO-TIME).

```rust
world.submit(Command::PlaceBuilding { player: me, building: woodcutter, at: hut });
world.submit(Command::BuildRoad { player: me, from: hq_flag, to: site_flag });
ai.step(&mut world); // or world.step() without computer players
for result in world.last_results() {
    // result.command, and result.outcome: Ok(()) or Err(Rejection)
}
```

The computer players are the last part of a tick (AI-2.1a):
`World::step_then(|world, rng, numbers, routes| ...)` runs the update, then
the closure, which reads the world as the update left it and may draw from
the simulation's generator it is lent (AI-2.7), take object numbers from its
counter (`Numbers`: the computer player's own records are objects of the
game, SIM-START-4.2) and ask the players' stored route costs (`RouteStore`,
TRN-CARRY-5.3a: a cost it asks is stored, as in the game;
`RouteStore::stored` reads one without asking it); the commands it
returns take effect inside the same tick. `AiSystem::step(&mut world)` is
that with the computer players. Each command is in effect for the parts of
the think that come after the one that issued it (AI-2.1a): the computer
players read the world through `s2_ai::view::ThinkView`, a private copy on
which `World::apply_at_once` applies the tick's commands as they are issued;
the world itself takes them after the think.
`World::lend(|world, rng, numbers, routes| ...)` lends them outside a tick:
`AiSystem::activate(&mut world, ...)` uses it, as the activation numbers the
castle's cell.

The commands are those of the player (placing and removing buildings,
flags and roads, production and distribution settings, the military
settings and attacks, ships and expeditions, specialists), of the map scripts
(`AddGoods`, `LockBuilding`, `CreateBuilding`, `Cheat`, ...) and a debug
command (`DebugSupplySite`, below). `Rejection` says why a command was
refused; it changes nothing then.

## State out

`World`'s queries, read between steps. The main ones:

| Query | What |
|---|---|
| `tick()` | the tick of the last step |
| `terrain()` | heights, terrain types, field state (FMT-S2M-4, MAP-GRID) |
| `field_owner(f)`, `is_explored(p, f)`, `explorations()`, `owned_field_count(p)` | territory (TER), exploration; `explorations()` counts every exploration by any player (GFX-VIEW-5.4 d) |
| `placement_at(f)`, `compute_placement(p, f)` | what a field takes (ECO-PLACE-4.1) |
| `building_at(f)`, `buildings()`, `building_type(id)` | buildings and construction sites (`Building`) |
| `site(f)`, `sites()` | a site's progress and materials (`ConstructionSite`, ECO-HQ-4) |
| `decorations_cleared()` | the fields whose map decorations a site, or a start castle in the first update, has removed from the world, for good (ECO-PLACE-5.3) |
| `field_mark(f)` | the number of the settler or carrier holding the field's settler mark (it came to stand there; any settler leaving the field clears it), once per tick (AI-CON-8.1e) |
| `decoration_cleared_fields(f)` | of the footprint and flag fields of the site or building on `f`, those whose decorations are gone (ECO-PLACE-5.3) |
| `stock(f)`, `warehouses()` | warehouse stocks (ECO-HQ-2) |
| `flag_at(f)`, `flags()`, `flag_order()`, `roads()`, `road(id)` | flags and roads (TRN-ROAD); `flag_order()` every flag in the order placed (TER-6.3) |
| `wares()`, `ware(id)`, `wares_at(flag)` | wares lying or carried (TRN-CARRY) |
| `requests(p)`, `request_queue()`, `orders()` | a player's open settler requests; the one queue of all players and its cursor (ECO-REQ-2.1); open ware orders (ECO-REQ) |
| `deposits()`, `deposit(f)`, `resource_at(f)` | trees, stones, grain, ores and fish (ECO-OUT) |
| `animals()` with `Animal::step()` | game animals (ECO-FOOD-5) |
| `ships()` with `Vessel::step()`, `harbours()`, `expedition_goods(at)` | ships and harbours (SHIP) |
| `garrison(f)`, `military_at(f)`, `duels()`, `catapult_at(f)` | the military (MIL) |
| `statistics(p)`, `tribe(p)`, `soldier_rank(good)` | per player |

The records these return (`Building`, `ConstructionSite`, `Road`, `Ware`,
`Deposit`, `Animal`, `Ship`, `Vessel`, `Harbour`, ...) are spec objects; their
states (`DepositState`, `ShipState`, `ExpeditionState`, `CatapultPhase`) are
the specs' states and part of the API.

## Views out: what settlers are doing

A settler's job, a worker's step, a carrier's task and a builder's approach
are the simulation's state machines; they are **not** API. A view reads
`World::settler_views()` (or `settler_view(id)`): one `SettlerView` per
settler, road carrier, builder and leveller, in that order.

```rust
use s2_sim::view::{Activity, SettlerRef};
for v in world.settler_views() {
    let (from, to, f) = (v.walk.at, v.walk.next, v.walk.fraction());  // on the fields
    if let Some(fine) = v.fine { /* on the height samples, TRN-FREE-5 */ }
    match v.activity {
        Activity::Worker { building, phase, .. } => { /* ECO-WORK, ECO-OUT */ }
        Activity::Carrier { road, retired } => {}
        Activity::Attack { waiting } => {}
        _ => {}
    }
    // v.request (GFX-ANIM-3.1), v.inside, v.hidden, v.carrying, v.health,
    // v.fighting, v.work_spot, v.stand, v.on_water, v.donkey
}
```

| Field | Meaning |
|---|---|
| `id` | `Settler(n)`, `Carrier(n)`, `Builder(site)`, `Leveller(site)` |
| `player`, `profession`, `number` | owner, profession (a good), own number |
| `walk`, `has_walk`, `walking` | the step on the fields (`Step`: `at`, `next`, `done` of `total`, `speed`), GFX-ANIM-3.1a's walk flag, a walk under way |
| `fine` | the step on the height samples with the facing (fine walks, builders) |
| `request`, `request_age` | the animation the logic asks for (GFX-ANIM-3.1) and, for builders and levellers, how long it runs |
| `inside`, `hidden` | inside its building (a work loop not shown); standing on its building's field (not drawn) |
| `activity` | `Activity`: to a site or building, worker (with `WorkPhase`), home, soldier (to a building, garrison, attack, defend), specialist, dying, carrier, builder, leveller |
| `carrying`, `health`, `fighting` | carried good, soldier health, an attacker's fight action |
| `work_spot`, `stand` | the field a worker works at; where he stands taking the ware |
| `on_water`, `donkey` | a boat carrier; a donkey |
| `detail` | the current step in the simulation's words, for debug text only |

`s2_sim::trace` turns the views into the trace rows of `docs/trace-format.md`.

## Events out

| Query | What |
|---|---|
| `last_results()` | every command of the last step and its outcome |
| `messages()` | messages to the players (UI-GAME-11), oldest first |
| `mission_events()`, `outcome(p)` | what happened to the mission in the last step; a player's victory or defeat (CAM-3) |

## Determinism and checks

`state_hash()` and `state_hashes()` (by part), `ledger()` and
`ware_holdings()` (the soak's ware conservation), `s2_sim::invariants`
(the soak's checks): `docs/test-coverage.md`.

The simulation's random number generator (`s2_sim::rng`, SIM-RNG): the
`seed` of `World::new`, `World::new_game` and `s2_rules::new_game` is its
seed (the low 32 bits, repaired as SIM-RNG-1.2); the original game always
uses `s2_sim::rng::SIM_SEED` (0x5432ACE0), and the generator is reset at the
end of set-up, so every new game draws the same numbers. `World::rng()`
reads its state, seed and draw count (`docs/trace-format.md`, generator
rows); `World::record_draws()` and `take_draw_record()` name the places that
drew (`docs/rng-diff.md`), outside the game state.

## Not API

- The state machines and their parts: `SettlerJob`, `Work`, `CarrierTask`,
  `BuilderPose`, `Builder`, `Leveller`, `AttackStep`, `DefendStep`,
  `SpecialistStep`, `FinePosition`, `movement::Walker` (and the fields of
  `Settler` and `Carrier` that hold them), `World::fine_position`,
  `World::builder_pose`. They stay public for `s2-ai` and the tests; each says
  "internal" in its documentation.
- The debug hooks: `Command::DebugSupplySite`, `World::debug_add_ship`,
  `World::debug_launch_ship`. For tests, the viewer's debug mode and its
  screenshot scenes only; a game or a project does not use them.

## Change notes

From 0.9.0 on, every release's changes to the public API are in its
[`CHANGELOG.md`](../CHANGELOG.md) section ("For projects built on OpenS2X:
breaking changes") and in [`breaking.txt`](breaking.txt). The notes below
cover the releases before.

- 0.8.0: a save's mark (`docs/extending.md` "Saves"): `s2_ai::save::SaveMark`,
  `Header::mark`, `text_hash`, `read_as`, `LoadError::OtherMark`; save format 2.
  `read` reads as OpenS2X and refuses a project's save. New building types
  (`docs/extending.md`): `Rules::recipes`, `World::recipe`,
  `AiConfig::type_inputs`; world format 67.
- 0.6.0: a save of ours loads in every later version: `s2_ai::save` (the file
  format, moved from the viewer), `s2_sim::snap::{OLDEST, Shape}`,
  `Reader::format`, `Snap::shape`, a state field's `name @ N = default`
  (`docs/snapshot.md`, "Changing the format"). `AiSystem::take_over(world,
  save)` takes over an original save's computer players (FMT-SAV-3.10).
- 0.5.0: `Requester::Unlinked` (ECO-REQ-2.3a): a cancelled request
  stays in the queue, unlinked, until its turn deletes it, so
  `World::requests` can list it; it belongs to no building or road.
  `s2_ai::expansion::Expansion::placed` is now `remembered`.
  `World::apply_at_once` and `s2_ai::view::ThinkView` (AI-2.1a): the
  computer players read the commands issued earlier in the tick.
- 0.4.0: the headless start (above): `s2_assets::find_map` (moved from
  `s2-app`) and `s2_ai::every_player`. `AiSystem::new(config, &world)`
  takes no seed (the computer players draw from the simulation's generator,
  AI-2.7); `AiSystem::activate` and `activate_all` take `&mut World` (the
  activation takes object numbers, SIM-START-4.2); `AiSystem::tick(world,
  rng, numbers)` is the closure of `World::step_then`.
- 0.4.0: `s2_sim::Rng` is the Park-Miller generator of SIM-RNG
  (`draw`, `range`, `fraction`, `reset`, `state`, `seed`, `draws`; it was a
  xorshift with `next_u64`), `s2_sim::rng::SIM_SEED`, `World::rng`,
  `World::record_draws`, `World::take_draw_record`. Every game draws
  different numbers than with 0.3.0, so its states differ.
- 0.3.0: `s2_sim::view` added; the viewer moved to it. `Rules` and its
  parts now live in `s2_sim::rules`, `Command`, `Rejection` and
  `CommandResult` in `s2_sim::command`; both are re-exported at the crate
  root, so their paths did not change.
- 0.3.0: the extension points (`docs/extending.md`): `GameDir::with_overlay`,
  `GameDir::find` and `s2_assets::find_in`; `resolve` moved to
  `s2_assets::resolve` (`s2_rules::resolve` re-exports it);
  `InstallSource` has an `overlays` field (`InstallSource::of(&dir)`
  builds one); `s2_viewer::run_with` with `Extensions`.
