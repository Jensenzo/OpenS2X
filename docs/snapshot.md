# Snapshots: the game state saved at any tick

A snapshot holds the whole state of a game at one tick, in our own format.
Loading it and stepping on gives the same game, hash for hash, as never
having saved. Before snapshots a game could only be "saved" as a replay
file (its start and a tick, `docs/test-coverage.md`): loading meant playing
it again from tick 0.

The original's save format (`specs/FMT-savegame.md`, read by `s2-assets`
and taken over by `World::restore`) is a separate thing. It is read only
in part and never written. Snapshots are ours alone: nothing outside
OpenS2X reads them, and their format follows our state types, not the
original's.

## What is in it

| Part | Written by | What |
|---|---|---|
| world | `World::snapshot` (`crates/sim/src/snap/`) | every field of `World`: tick, generator (state, seed, draw index), pending commands and last results, terrain, territory, flags, roads, buildings, sites, warehouses, settlers, carriers, wares, requests and orders, deposits, animals, ships, harbours, military buildings, attacks and duels, catapults, ruins, timers, statistics, mission, messages, the rules the world was made with, and the ledger |
| computer players | `AiSystem::put_state` (`crates/ai/src/snap.rs`) | the AI's tick counter, its generator, every computer player (cells, records, maps, plan, construction, expansion, roads, cursors, gates) and the military maps: what `AiSystem::state_hash` covers |
| report | `Runner::put_state` | the run's counts (commands issued and rejected, attacks, conquests, duel ticks, first contact) |
| start | `game::snapshot` (`crates/app/src/game.rs`) | map, seed, start resources and computer slots: from these the rules and the AI's configuration are loaded again |

Not written: the generator's record of draws (`s2-app rng`), the run times,
the AI's configuration (it comes from the rules), the invariant checker's
history (`soak` starts a new checker on the loaded world, so an `idle`
report needs 600 ticks after the load again), the trace's clip clocks and
the route topology (each player's flags with their roads, which the route
searches keep between road changes; a loaded world makes it again from its
roads at its first search).

## Format

A game snapshot file (`.s2snap`):

```
"s2-game\0" (8 bytes), version (1)
map (string), seed (u64), start resources (u8), slots ([(player, profile)])
"s2-snap\0" (8 bytes), world version (1), the world
the computer players, the report
```

Values: unsigned integers LEB128, signed ones zigzag and then LEB128,
`u8` and `bool` one byte, `f32`/`f64` (the AI's scores only; the simulation
keeps none) their bits little-endian, strings and sequences their length
first, maps as (key, value) pairs in key order, an `Option` a byte 0 or 1
and then the value, an enum the index of its variant and then its fields.
Struct fields follow in declaration order.

Every state type implements `s2_sim::snap::Snap`, almost all through
`snap_struct!` and `snap_enum!`, which name every field and variant. A
field added to a struct and not to its line does not compile, and neither
does a new variant. `VERSION` is the format of the world and the computer
players together; `SNAPSHOT_VERSION` is only the start of an `.s2snap`, which
is a tool's file, not a player's save: it is refused unless it is this build's.

## The game's save files

The game (`OpenS2X`, the viewer) saves with the same parts: Save Game,
the autosave and Continue (`PLAYING.md`, `crates/viewer/src/game/save.rs`)
write and read `<Documents>\S2\saves\<name>.s2save`:

```
"s2-save\0" (8 bytes), save format (2), world version (66)
map file (string), tick, world hash, AI hash (Option), slots ([(player, profile)])
the mission's interface: target, tutorial step, debriefing, extro, letters,
  the script's variables ([(name, u32)], FMT-SAV-3.12)
the mark (from save format 2): the project's name (string, empty for
  s2-reborn), its rules overlays ([(name, u64 FNV-1a of the text)])
the world (as above, with its own "s2-snap\0" and version)
whether there are computer players (bool), then their state
```

The file format is `crates/ai/src/save.rs` (`s2_ai::save`), which needs no
Bevy, so its tests run without building the viewer. The two version numbers
come first. A save of a format this build does not read (before world format
65, or newer than this build) is refused with a notice naming both before
anything else is read, and never crashes. The hashes are those of the game when
it was saved: a game of this build's format must give them again (the play
harness's save-quit-load step checks it, and the program logs a warning if
not). A game of an earlier format cannot: a new field is part of the state hash.
A save whose mark (`s2_ai::save::SaveMark`) is not the running program's,
a project's game or one played with other rules overlays, is refused with
the same notice of another version, naming the program that wrote it
(`docs/extending.md`, "Saves").

## Changing the format

**A save of ours loads in every later version.** The first supported saves are
save format 1 with world format 65 (commit cd27ec4, the build installed on
2026-11-02). `OLDEST` in `s2_sim::snap` is never raised. The way it works: the
reader knows the format of the bytes it reads (`Reader::format`), and a type's
line says what came later.

What a rule that changes the snapshot must do. All of it is checked by
`cargo test -p s2-ai --lib save::` (about 10 s after the build, no Bevy):

1. **Raise `VERSION`** in `crates/sim/src/snap/mod.rs` (for a change in the
   header of a save: `VERSION` in `crates/ai/src/save.rs`). Every commit that
   changes the recorded format raises it, also when an earlier commit of the
   same branch raised it and no release came between: CI checks each commit.
2. **Say what an older game has**, on the type's macro line:
   | The change | Write |
   |---|---|
   | a new field | `hidden @ 66 = false` (66: the new format; the value any expression) |
   | a new field of a `{ .. }` variant | the same inside its braces: `Walk { to, back @ 66 = false }` |
   | a new variant | at the **end** of the macro's list, wherever the enum declares it: the place in the list is its number in the bytes |
   | a field's type changed, or a field gone | `traffic @ 66 => \|r: &mut Reader\| { let old: u32 = r.get()?; Ok::<_, SnapError>(old.into()) }`: reads what the older bytes hold at its place and makes the field; a gone field is read and dropped by the field after it |
   | a field the older game can only have from the rest of the state | a default on the line, then set it in `World::read_snapshot` (or `AiSystem::get_state`) `if r.format() < 66` |
   | a variant gone, its meaning changed, a header field | an `impl Snap` by hand, or the same `if r.format() < 66` |
3. **Run `scripts/save-format.sh`** (about 30 s): it moves the old record to
   `tests/saves/format-previous.txt`, writes `tests/saves/format.txt` and the
   new format's fixture `tests/saves/save<S>-world<W>.s2save`.
4. **Commit them** with the rule. The state hash changes with any new field,
   so the golden hashes do too, as they always did.

**A loaded older game must also keep working, not only load.** A default
that leaves a feature dead for the rest of the game is a fault: check each
new field for it. Some rule fields come only from the scripts (by their
names), which the world does not have: `s2_ai::save::read` takes this
build's rules of the game, and `World::take_later_rules` gives an older
world those fields from them.

What an older game shows, by format:

| Saved in | What it shows when loaded |
|---|---|
| 99 (before 2026-11-24) | The interceptors out at the save have no send order (a defender's `sent` is 0): among them an attacker arriving on his meeting field takes the first in settler order (MIL-ATK-3.2 T47); every one sent after the load comes after them. |
| 90 (before 2026-11-19 evening, exact f) | Nothing differs in what loads (format 91 adds the attack step `Capturing`, MIL-CON-1.2 T18): before it an attacker took the building in the update he reached the door, so none is capturing at the save. |
| 89 (before 2026-11-19 evening, exact e) | A soldier who had stepped into his building on the update of the save is idle at once (MIL-ATK-4.4 before T33, T33a); after the load each soldier's return ends one update after his step in, at a castle when his walk on to the door field ends (format 90 adds the steps `In`). |
| 88 (before 2026-11-19 evening, exact d) | A target's soldier paired with an attacker at the save, still inside waiting to leave, leaves as he was sent (MIL-ATK-5.3 before T34: without waiting for his attacker to stand on the flag field or arrive); one paired after the load waits (format 89 adds the step `Held`). |
| 87 (before 2026-11-19 evening, exact c) | Nothing differs in what loads (format 88 adds a defender's step `Ended`, MIL-ATK-6.6 T32). A defender walking home at the save walks on to his building as he was sent (straight onto its field); one sent home after the load goes through his flag. |
| 86 (before 2026-11-19 evening, exact b) | No skeleton doodad lies where a settler died before the load, and a settler dying at the save more than 14 ticks into his dying leaves none (TRN-CARRY-7.1, MIL-ATK-6.6 T31a); the skeleton doodad of the rules is taken from this build's scripts (`World::take_later_rules`), so deaths after the load leave theirs. |
| 85 (0.12.0, 0.12.1) | A settler dying at the save (TRN-CARRY-7.1; no soldier killed in a duel, who was removed at once then) lies on his field's point and is removed one update earlier than one dying after the load (MIL-ATK-6.6 T31: the dying's 28 ticks and one more update; his saved countdown has no room for it). |
| 84 (before 2026-11-17 day, exact c) | An attacker walking to the meeting field an interception gave him at the save walks on as one paired at the target's door (MIL-ATK-6.1a before T26): arriving there he makes his draw and his step off the point at once, two updates earlier than one sent after the load. |
| 83 (before 2026-11-17 day, exact b) | An attacker out at the save is off his home building's soldier list (sent before MIL-ATK-2.3's correction, Q-1070): he is put back at the end of its list, in settler order, so the home counts him again (it may cancel a replacement request or let a soldier go in its next pass, MIL-SOL-4.4) and he comes back to it when his attack ends. |
| 82 (before 2026-11-17 day, exact) | A computer player's needs that hold a cell at the save do not know their place in its need list (AI-CON-2.5b, `celled` 0): they count as given it first, among themselves by need id, and a producer announced after the load is tied to the first of them that has no production. Needs given a cell after the load join the list's end. |
| 81 (before 2026-11-17 night) | Nothing different: no game of that format has a ware order or settler request attached to a harbour (SHIP-4.1, Q-063: the need of a harbour came from no rule then); the open ones attach at their next turn when the land cannot serve them. |
| 80 (before 2026-11-16 day b) | A "Ship waiting for Orders" message posted before the load names no ship: its row shows the text without the ship's name in brackets. Those posted after the load name it. |
| 79 (before 2026-11-16 day) | Nothing different: no game of that format has a command refused for the military clearance (ECO-PLACE-6.2, the result `TooCloseToMilitary`); a military site placed within 4 fields of another military building stays as it was. |
| 78 (before 2026-11-15 day d) | Nothing different: no game of that format has a project's paving rule or paved roads (hook W19, `reborn.cfg_setPaving`). |
| 77 (before 2026-11-15 day c) | Nothing different: no game of that format has a project's recipe groups or kept stock (hook W16, `reborn.cfg_setRecipeKeep`). |
| 76 (before 2026-11-15 day b) | Nothing different: no game of that format has a project's mine deepenings (hook W15, `reborn.bp_setDeepening`), so none keeps first resources or a deepened mine. |
| 75 (before 2026-11-15 day) | Nothing different: no game of that format has a project's recipe sets (hook W20, `reborn.bp_addRecipe`), so none has a recipe chosen. |
| 74 (before 2026-11-15 night b) | An interceptor out at the save does not know the building he defends (MIL-ATK-3.4): when his attacker is gone he walks home at once, as before, instead of waiting outside while the building has attackers. Interceptors sent after the load wait. |
| 73 (before 2026-11-15 night) | An attacker standing on his waiting field at the save has made no offset step (MIL-ATK-6.1a): one paired with an opponent makes it, with its draw, in his next update and arrives 12 ticks later (level ground); an unpaired one stands on the field's point, as the rule has it. |
| 72 (before 2026-11-13 late) | A soldier, builder or leveller going home after a short walk (TRN-FREE-6.3) at the save is no longer linked to his military building or site, as the rule before TRN-FREE-6.3a had it: the building or site may ask for another while he walks. Those sent home after the load stay linked until they go. |
| 71 (before 2026-11-12) | A ruin burning at the save (MIL-CON-4.1) does not know the building it was: it marks its fields and burns out as ever, but the view draws no ruin model or decal for it (GFX-OBJ-8.7 (5)); it is gone at most 333 ticks after the load. Ruins made after the load are drawn. |
| 70 (before 2026-11-11 night) | The doodads removed for good (GFX-OBJ-1.4a) are taken from the fields as they stand: those under its roads, flags and buildings stay away; one under a road, flag or building that went before the save shows again. |
| 69 (before 2026-11-09) | A soldier going home kept no health (`health` of the leaving steps is none): if he is homeless, his first lost point (TRN-FREE-6.4 step 4, Q-1034) counts from his rank's maximum. Every other settler is the same. |
| 68 (before 2026-11-08) | Nothing different: a settler there with no warehouse in reach had died at its first failed home search; the wander between tries (TRN-FREE-6.4 step 4, the leaving steps `Wander` and `Retry`) starts with 69. |
| 67 (before 2026-11-08) | The statistics' sampling count (UI-GAME-8.5a, now 601 ticks a cycle) and the productivity decay count (ECO-OUT-3.11) run from the tick, as in a new game (phases 0); the series go on from where they were. The exploration count (GFX-VIEW-5.4 d) starts at 0, which the view does not mind: it looks for changes. |
| 66 (before 2026-11-07) | No recipes of a project's building types (`World::recipe`): none existed. Every type works by its product's cost list, as before. |
| save format 1 (before 2026-11-07) | No mark: it is s2-reborn's game. s2-reborn loads it as before; a project's program (with a project name or rules overlays) refuses it with the notice of another version, "saved by s2-reborn, another version". |
| 65 (before 2026-11-05) | The production statistics (UI-GAME-8.5e): its goods get their counters from this build's rules; each counter shows zero for the minutes before the load and counts from the load on, as does the coins' total (series 5). The frags (series 7) count from the load: the kills before it are not in the old game. Its series 7 held zeros sampled at tick 4 of each minute; it gets the new series' zero samples. |

The cost: one annotation per change, 30 s for the script, one fixture per format
(145 KB on disk, 12 KB compressed in git; 7 format changes would be under 100
KB of history), and old types' shape kept forever on the macro lines. A value
that means something new while its bytes stay the same is a format change too:
the checks cannot see it, so raise `VERSION` and convert it (`if r.format() <
N`). Moving or renaming a state type changes the recorded text only, not the
bytes; it still takes a new number (the CI rule below) and costs a fixture.

What fails when that is not done:

| Done wrong | What fails |
|---|---|
| a field or variant added, nothing else | `the_format_is_the_recorded_one` |
| `VERSION` raised, no fixture | `every_supported_format_has_its_fixture` |
| recorded again, but a new field without `@ N`, a variant in the middle, a field moved or gone | `this_format_keeps_the_previous_formats_bytes_readable` (holds even for a type no fixture contains) |
| an older game that does not load or play on | `every_fixture_save_loads_plays_on_and_saves_again`: every fixture loads, plays 1000 ticks with every soak invariant holding, saves again, and what it saved loads as the same game |
| the bytes changed under the same number | `every_supported_format_has_its_fixture` (this format's fixture read and written gives the same bytes) |
| a fixture changed or removed; `format.txt` changed under the same number | CI, job `golden-changes-named`, on every commit of a pull request or push |

The fixtures are computer-player games on our own synthetic map and rules
(`s2_ai::fixture`, tick 9000 of `town4`: sites, wares on the roads, soldiers on
their way), so they hold nothing from the original's files. They do not hold
every kind of state (no battle, no ship), which is why the format text is
checked as well. Real games can be checked by hand, also from a player's folder
(`S2_GAME_DIR` for the rules):

```bash
s2-app soak --save "<...>/S2/saves/my game.s2save" 1000
```

## Using it

```bash
s2-app hash MP_2P_Storm_Coast 3000 --snapshot game.s2snap   # save at tick 3000
s2-app replay game.s2snap --to 4000 --every 500              # load, continue, hash lines
s2-app trace --snapshot game.s2snap 3200 --out t.csv         # a trace from tick 3001 to 3200
s2-app soak --snapshot game.s2snap 1000                      # check 1000 more ticks
```

`replay` and `trace --snapshot` take a replay file too.

`soak` keeps a snapshot in memory every 2 000 ticks. When a tick fails (a
broken invariant or a panic), soak writes the replay file of that tick and
a snapshot of the tick before it: the last checkpoint loaded and played on.
`s2-app soak --snapshot FILE 1` then runs just the failing tick.

## The checks

- `crates/sim/src/snap/tests.rs`: a battle and a new game's economy, saved
  at every tick: the loaded world equals the saved one, writes the same
  bytes, and 30 to 40 ticks later has the hash of the world never saved.
  Broken, longer and other-version snapshots are refused, and a type with
  later fields or variants reads older bytes.
- `crates/ai/src/save.rs`: the fixtures and the format ("Changing the format").
- `crates/app/src/snapshot_tests.rs`, the property: a game of computer
  players saved at 25 ticks spread by a fixed pseudo-random sequence. Each
  save is loaded into one other runner (all of whose state it must replace)
  and continued 40 ticks. World and AI hashes are compared at the load, one
  tick later and at the end. Fixtures `flat2`, `town2` and `town4` (3 000
  ticks, on CI), the maps `MP_2P_Storm_Coast`, `MP_4P_crossroads` and
  `MP_6P_Paradise_Island` (6 000 ticks, with `S2_GAME_DIR`), and a long run
  of the same maps (60 000 ticks, 100 saves, 200 ticks on;
  `cargo test --release -p s2-app long_maps_continue -- --ignored`).
