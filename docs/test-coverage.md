# Test coverage and the safety net

What checks the program, where it runs, and how to accept an intended change.

## Determinism (state hashes)

`s2-app hash MAP TICKS [--seed S] [--every N]` prints, every N ticks, the
world's full hash, the AI's hash and the world's hash per part (rng, settlers,
wares, buildings, flags, roads, players, land). See `docs/architecture.md`,
"Determinism checks".

| Golden file (`tests/golden/hash/`) | Game | Runs on |
|---|---|---|
| `fixture-flat2`, `fixture-flat4` | synthetic, rules only (no people) | CI (Linux and Windows), the maintainer's PC |
| `fixture-town2`, `fixture-town4` | synthetic small economy (`s2_ai::fixture::town_rules`) | CI (Linux and Windows), the maintainer's PC |
| `MP_2P_Storm_Coast`, `MP_4P_crossroads`, `MP_6P_Paradise_Island` | the install's maps and rules | the maintainer's PC (`scripts/golden.sh`, `cargo test` with `S2_GAME_DIR`) |

All at 5 000 ticks, a line every 500. `scripts/golden.sh` also runs the
Windows build from WSL and requires the same lines; `--bless` writes the files
again, and the commit must name the spec rule that made them change (the
maintainer's CI checks it).

## Soak and invariants

`s2-app soak MAP TICKS [--seed S]` plays computer players against each other
and runs `s2_sim::invariants::Checker` after every tick:

| Invariant | What it checks |
|---|---|
| `wares` | Wares are conserved: what the world holds (warehouse stocks, input stocks, construction sites, flags, carriers, settlers' hands, harbours, ships) equals the start plus the ledger's made minus used. A difference is reported in the tick it appears, with the places that changed. |
| `settler-place` | Every settler and carrier is on the map; a worker's building stands; a soldier's building stands and lists him; a carrier holds the place of its road. |
| `one-job` | One worker per building (with the one on his way), one builder and one leveller per site, one carrier per road place, one carrier per ware; a ware at a flag is claimed only by a carrier whose task is about it (TRN-CARRY-2.6). |
| `roads` | Both ends of a road carry the owner's flag, its fields are neighbours, inner fields carry the Street bit and the street index; every ware at a flag is in that flag's list and the other way round. |
| `idle` | No builder stands at a levelled site with a material it can build in, and no worker waits for inputs it has, for more than 600 ticks. |
| panic | A panic in a step is caught and reported like a broken invariant. |

The ledger (`s2_sim::ledger`) counts what is made (products, a mission's or a
debug command's goods) and used up (inputs, building materials, tools made into
professions, weapons made into soldiers, coins and catapult stones, destroyed
wares, the contents of removed buildings). It measures the game and is not
part of the state hash.

Every failure prints its tick; the run writes a replay file to `target/soak`
(or `--out DIR`) that plays the game to that tick: `s2-app soak --replay FILE`
checks it again, `s2-app replay FILE` gives its hash line. It also writes a
game snapshot of the tick before the first failure (`docs/snapshot.md`),
made from a checkpoint kept every 2 000 ticks: `s2-app soak --snapshot FILE 1`
runs the failing tick at once.

| Run | Where |
|---|---|
| `fixture:town2`, `fixture:town4`, 20 000 ticks each | CI, every code push |
| every free play map of the install, 200 000 ticks | `scripts/soak-nightly.sh` on the maintainer's PC |

The checker's own tests (`crates/sim/src/invariants.rs`) break a world on
purpose for each invariant but `idle`.

## Golden images

`scripts/golden-images.sh [--bless] [NAME]` draws the scenes of
`tests/golden/images/scenes.txt` with the Windows build (from WSL) at
1024 x 768 and compares each with its stored picture: a pixel differs when a
colour channel is more than 8 apart (`TOLERANCE`), and a scene fails when more
than 0.2 % of its pixels differ (`MAX_SHARE`). A failure writes
`target/golden-images/diff/<scene>.png`: the stored picture darkened, the
differing pixels red. `s2-app image-diff` does the comparison.

The scenes: the start camera on three maps at the five zoom steps, a
woodcutter's hut site at 25, 50, 75 and 100 % of its work, a project's own
building model (the repository's `tests/models/facets.glb`, hook W30) as the
well beside a woodcutter's hut, a project's own trees (`tests/models/tree_*.glb`
for the birches and firs at the default zoom, near and far versions, hook
W30b), a project's own map objects (`tests/models/stones_*.glb` for the stone
piles by their states, the bushes and small stones as other test models,
hook W30c), the title, the
main menu, the free play page and the other menu pages (the six settings
pages among them), every in-game dialog that can be opened from the command
line (tools, military, distribution, transport, the six statistics pages,
minimap, messages, game menu, headquarters with its military page, flag,
barracks, ship, a sawmill's, a woodcutter's hut's and a farm's window), and
the mission interface (mission 1's briefing book, the tutorial window's two
forms).

Every scene is drawn with `--compare` or `--still`: the game paused, the
real-time clock that drives tree sway, flag cloth, terrain wobble and water
drift standing at 0, and the window ignoring the desktop's mouse pointer, so
that two runs give the same pixels.

**Not on CI.** Every scene needs the original's art from the install, which
CI does not have; a software renderer (Mesa lavapipe or llvmpipe through
wgpu) would not change that. The pictures show the original's art, so they
stay out of git (`.gitignore`); `tests/golden/images/MANIFEST` (their
SHA-256) is committed, and `--bless` rewrites it, so a picture change shows in
a commit that must name its spec rule like a golden hash change.

## Frame time

`scripts/frame-bench.sh [--bless]` runs the Windows viewer on a late game
(MP_2P_Storm_Coast, both players computer players, played headless to tick
100 000, then the 3D view over player 0's headquarters at game speed 1) and
times 600 frames after 120 to settle (`s2-viewer --bench N --bench-report
FILE`): the mean, median and 95th percentile frame, and the time of each
part of the map's update between marks in the schedule (input and camera,
the interface, the game and its computer players, the scene objects, the
settlers, buildings, wares and their animation, the drawing lists), the
render world's CPU time between Bevy's render system sets (`render-world-*`:
assets, views and specialize, queue, phase sort, prepare, render graph; the
render world runs beside the next update, so a frame takes the longer of
the two), and each render pass's CPU and GPU time from Bevy's render
diagnostics (`render-*-elapsed_cpu` / `_gpu`, the mean of the last frames). It
fails when the median frame is more than 25 % above the maintainer's PC's
baseline (the mean moves by up to a quarter between two runs of one build),
`tests/bench/frame-bench.txt`; `--bless` writes the baseline. It is that
PC's timing, not game behaviour, so it lives outside `tests/golden` and a
re-blessing needs no spec rule in its commit (that check covers
`tests/golden` only).
It also fails when the game falls behind real time: fewer than 9.5 logic
ticks per second of the timed frames at speed 1 (`ticks-per-second` in the
report; ECO-TIME-1.2 to 1.4 run 10). The frames go as fast as they come;
`--bench-vsync` holds them to the screen's refresh, as a player sees the game.
Like the golden images it needs the install and a desktop, so it runs on the
maintainer's PC only, left alone while it runs.

`--bench-pan` holds the arrow keys while the frames are timed, through the
program's own input, with the pointer over the map (what lies under it changes
with the camera), and `--bench-drag <pixels>` drags with the middle button
instead, far faster; the report then has the worst frame, the 99th percentile,
and the parts of the eight slowest frames. Run these with the exe and
the game data on a Windows drive: read from WSL's file system over its file
server (`\\wsl.localhost`), a first read of a sound or a page of the program
takes 30 ms and shows up as a slow frame anywhere in it.

The harness's `panning_stays_smooth` (`crates/viewer/tests/play.rs`, held to
the refresh, limits set for the maintainer's PC in `tools/play/scripts.rs`;
another machine may need its own)
judges the whole pan so far: a median over 20 ms, a p99 over 50 ms, a worst over 150 ms or more than 1 % of
the frames over 50 ms fail it.

`scripts/play-test.sh` hands the program the Windows side's own copy of the
install (`S2_PLAY_GAME_DIR`) when its files and sizes are those of
`S2_GAME_DIR`: a first read of a sound or model over WSL's file server takes
20 to 100 ms and would show as a slow frame. The harness keeps the display on
while it runs (`tools/play/display.rs`), and before a timed step waits for 60
frames in a row within 50 ms (`tools/play/settle.rs`).

`--bench-seconds`, `--bench-log`, `--bench-invariants` and `--mute` make a long
sitting of it: frame times with a line a second and a line per frame over 33 ms
with its parts and what it did, ticks per second, the program's memory and every
soak invariant after every tick (their time left out of the frame times).

**The long play** (`scripts/long-play.sh`, a step of the release net since
0.11.4): 30 minutes of wall time at speed 3, 2560 x 1440, held to the
refresh, `--free-game MP_4P_Close_Frontier --ai 0=3`. It starts from tick
0 with the view on, so it sees what grows with a game played in view, which a
benchmark that plays ahead headless (`--run`) or loads a save does not. It
fails on a broken invariant, a minute below 29.5 ticks per second, a frame
p99 over 25 ms, any frame over 100 ms that is not the PC's licence check
(below), a bone-palette buffer over 12.5 MB at the end, a palette slot no
owner holds, or Windows' licence events not read; `--check DIR` judges a
run's report and log, `--self-test` the check itself on made-up runs (CI). It holds the display on for the run and wakes it first by a
pointer move of 0 (`scripts/display-hold.ps1`, as the play harness's keeper):
with the display off after the power plan's idle timeout every present waits
about 264 ms (2026-11-20, a run after a morning nobody used the PC: 6,807
frames of 263 ms, the GPU's work 17 ms each). Every model or clip the view reads after the game's start
is a first use during play, a `U` line in the log; a slow frame's line
names those of its last eight frames, and `--check` prints them beside each
frame over 100 ms (0.12.2's run failed on one of 107 ms in the frames its
game showed the first well's site; since then a game's start reads every
model its tribes can show, `models3d::preload`). The log has a line a minute (`P`) with the palette slots by
owner: the skinned parts' rest slots, the poses' parts (and the most one
pose holds), the buildings' own parts. The long-game slowdown came twice
from slots taken and never given back: until 2026-11-04 (poses' and sites'
slots), and from 0.9.3 to 0.11.3 a shared pose's slot per figure look (each settler figure's
look has its own atlas block since GFX-OBJ-3.2 (3), and the walk of a busy
map is one pose that never empties); the buffer doubled to 100 MB in 25
minutes and the frames went from 16.6 to 45..59 ms. The unit test
`a_pose_that_never_empties_gives_a_leaving_looks_slot_back`
(`crates/viewer/src/animation/animate3d.rs`) holds the mechanism without a
desktop.

The run records the PC's load beside the frames (`scripts/pc-load.ps1`, a
line a second in `pc-load.txt`: the whole machine's CPU, the disks, the
graphics card's 3D engine, the run's own CPU and page faults, and the five
busiest processes that are not the run's), and the viewer's log says the wall
clock of its 0 s (`T`) and each growth of the palette buffer (`B`; a slow
frame's line names a growth in it or the eight frames before). `--check`
prints the load over the run and the samples either side of every frame over
100 ms. That is information beside the limits, which stay as they are: a
slow frame still fails the run, and the report says what else the PC did in
its second. 0.14.0's run failed on a cluster of render-prepare frames
(157 ms the worst) 0.27 s after Windows' licence service (ClipSVC) was
trigger-started by the Store's 15-minute licence poll; a quiet run on
2026-11-23 failed the same way (283 ms) 0.26 s after the same poll, with the
machine's CPU at 58 % for that second and no process not ours at a full
core. Not every poll stalls the game (two of the eight in four runs did,
both late in a run, 1380 and 1634 s in), and a run with a foreign load of
CPU bursts, process starts and disk writes beside it (WSL at a core or more
for 421 s) had no frame over 100 ms, only scattered frames of 35..50 ms. The
run therefore also lists the licence service's starts in its time
(`windows-events.txt`), and `--check` names one that came in the 3 s before
a frame over 100 ms.

Since 2026-11-24 the log also cuts the render world's frame into its steps
(`crates/viewer/src/tools/frame_steps.rs`): for a frame over 50 ms, the
render frames kept from before it and those of the three frames after it get
an `R` line each, with the wall clock (unix ms) and the working set at the
frame's start and end, and for each of 15 steps (the wait for the main world
and the extract; assets; views, specialize and the swapchain's acquire;
queue and sort; prepare's six sets: resources, batch phases, write phase
buffers, collect phase buffers, flush, bind groups; the render system's
start, its graph's encode, its submit, its finish and present; cleanup) the
wall time, the CPU time of the thread that ran the step's end mark when its
start mark ran on the same thread (`thread -` otherwise), its kernel time
(at the clock tick's resolution, about 15.6 ms), the whole process's CPU and
kernel time, and the process's page faults. A smooth run writes none. Every
slow frame's `F` line names its page faults and working set, every `S` line
the second's page faults (about 27,000 a second in a normal run).
`PROBE_WINDOW=1` opens a second, tiny window of ours beside the game
(`--probe-window <log>`, `crates/viewer/src/tools/probe_window.rs`: the same
exe, Bevy and wgpu, one triangle, held to the refresh, on the left display)
whose own log `probe.txt` has every frame over 25 ms with the wall clock: a
stall of the graphics stack for everyone shows in both windows at once.
`LONG_PLAY_EXE` runs another build, such as the investigation build
`--features frame-trace` (`crates/viewer/src/tools/span_log.rs`, never the
game's): every system run (Bevy's trace spans) and every wgpu call wgpu
times itself, from enter to exit, and one of 25 ms or more is a line in
`log.txt.spans` (`Y <unix ms> <ms> <thread> <name>`), and a system whose
wgpu calls add up to 5 ms or more a `W` line with each call's count, total
and longest. It costs the frames more as the game grows: from about 17
minutes of a long play its frames ran at 35..48 a second, so its p99 is not
a player's number, only its stall frames are read.

What the instruments showed on 2026-11-24 (three runs, seven licence polls,
two stalls): at the poll's stall the time is in prepare's bind-groups set
(109..130 ms in a trial; 116.5 and 184.1 ms in the long run), in one Bevy
system, `prepare_preprocess_bind_groups` (GPU preprocessing: it builds a
few dozen bind groups fresh every frame, per view and phase), on two
threads in two frames, while the whole process used 2..12 ms of CPU and
took 0..64 page faults in that step, the working set flat; no single wgpu
call took 25 ms or more; submit and present were normal; the probe window's
frames were at most 36 ms. So our render thread waits inside many short
graphics calls, it does not fault, spin or lose its working set. GPU
preprocessing also makes and drops graphics objects every frame in ordinary
frames: the shadow views' instance writer alone 1,100..1,900 buffer writes
(each a staging buffer) and 20..430 new buffers by 25 minutes; later W lines
counted about 1,750 bind groups created and dropped a frame in
`prepare_preprocess_bind_groups` and 2,300 buffer writes in the shadow
writer. Bevy keeps four buffers and two bind groups per batch set (pipeline,
material bind group, mesh slab) per view and phase and renews them every
frame, and with a material of its own for nearly every object nearly every
object is a batch set of its own.

Since 2026-11-26 the viewer builds the instance data on the CPU
(`crates/viewer/src/render/instances.rs`): one buffer written once a frame,
the material bind groups kept, no preprocessing bind groups. Bevy's switch
for it (`PbrPlugin::use_gpu_instance_buffer_builder: false`) alone drew
nearly nothing (2026-11-24, 15 golden scenes, no error in the log): the
render phases take their mode from the device's GPU-preprocessing support,
not from the switch, so every mesh went into multi-draw bins the CPU batcher
never reads. `CpuInstancesPlugin` therefore also sets that support to none
in the render world at its start. The pictures are the same, and the render
world's prepare fell from a median of 5.2 ms to 1.5 in a long play. The
licence poll's stall stayed: in 2.5 hours on the CPU path (ten polls) it
came at three, 55, 125 and 156 ms, in the bind groups (45 ms), in one game
tick of the main world (101 ms, CPU work, no graphics call) and in the
frame's submit (130 ms). The many graphics calls only decided where the wait
showed; whatever runs at that moment is held. `pc-load.ps1` also records our working
set and the machine's page reads (hard faults) and transition faults; no
counter gives a process's own hard faults without elevation. `--check`
prints, beside each frame over 100 ms, the slowest step of each render frame
near it and the probe window's frames in the 3 s around it.

**The PC's licence check** (since 2026-11-26). A frame over 100 ms that
begins 0.2 to 1.0 s after a start of ClipSVC in the run's Windows events is
the PC's licence check: `--check` lists it with its slowest part (and the
render frame's slowest step) and its time after the service's start, and it
does not fail the run. A frame over 100 ms that begins at most 1.0 s after
the end of such a frame belongs to it. A frame's beginning is the run's 0 s
(`T`) plus its logged time (taken at its end) minus its length. Every other
frame over 100 ms fails the run as before (one per change of the display's
power state, at most 500 ms, aside), and so does a run whose events could not
be read: `scripts/licence-events.ps1` writes `read N` first, and without that
line (or, in a run's files from before, a start listed) no frame can be told.
Why: every 30-minute run from 23 to 26 November that failed on a slow frame
failed on these only. The release run of 26 November had one frame of 129.5 ms and two of 152.9 and 392.8 ms,
beginning 0.58, 0.23 and 0.39 s after the service's start, while invariants,
pace, p99 and palette were in their limits; and with our graphics calls cut
by the CPU instance path (the measurements of 24 to 26 November) the wait moved
into a game tick, into the frame's submit, and once into a separate process,
so it is Windows' own and no change of ours takes it away. The window is the
measured one: the frames of those runs began 0.23 to 0.69 s after a start. A
150 s trial of 24 November had two such frames 1.08 and 1.21 s after one, at
its 22nd second; the rule leaves those failing (not a release's run; the
window stays as measured on the 30-minute runs). The limits for p99, the
lowest minute, the invariants and the palette stay as they are.

## Lessons 2 to 4

`tutorial_two`, `tutorial_three` and `tutorial_four` (`crates/viewer/tests/play.rs`)
follow each instruction with its clicks, to the lesson's end, as lesson 1's does. A
marked field that is off screen or under a window is scrolled to with the arrow keys,
as a person does (`Act::Reveal`): lessons 2 and 3 stopped at their first marked field
for that, at (720, 299) under our tutorial window at 1024 x 768 and at (388, 842) below
the screen; since CAM-5.3b (2026-11-06) the tutorial's own camera moves bring 16 marked fields into view. Lesson 3 waits until a building
has a soldier to spare. Writing them showed lesson 2's step table out of the order of
CAM-5.2 (fixed with a test), a harbour without its expedition page (UI-GAME-4.4, built)
and lesson 4's ship one field from its marked field (Q-1014).

## Sounds

`--bench` also reports the sounds started per second, the most one-shot sounds
playing in a frame, the sounds refused and cut per second and the eight most
started. The limit is SND-2.6: 32 channels, so at most 32 less the music and
the ambient loops playing; `audio/sound.rs` tests the choice (2.T1 to 2.T5).


## Saving and loading

- `crates/ai/src/save.rs` tests (the file format), `crates/viewer/src/game/save.rs`
  (the autosave clock) and **fixture saves of every supported format**
  (`tests/saves`, `docs/snapshot.md` "Changing the format"): each loads, plays
  1000 ticks with every invariant and saves again; a format change without a new
  number, a fixture or a migration fails.
- `crates/ai/src/save.rs` tests: a game of computer players saved
  and read back has the same tick, state hashes and bytes, and plays on
  the same as never saved; another save or world format, a cut or longer
  file and computer players without rules are refused with a reason.
- The play harness's `save_quit_load` (`scripts/play-test.sh`, three
  starts of the program): a free game saved from the game menu under a
  typed name, the autosave after its interval (`--autosave 60`), the
  program ended; LOAD GAME and then CONTINUE each give the game with the
  state hashes (world and computer players) it was saved with, and it
  plays on. Every harness run saves to `target/tmp/play/saves`
  (`--save-dir`), never the player's folder.
- The original's saves (`S2_SAVES_DIR`): `crates/rules/tests/savegame.rs`
  reads them whole and continues them; `crates/ai/tests/original_save.rs`
  continues the autosave with its computer players, their records taken over
  (`AiSystem::take_over`), and prints what it holds against what is taken over
  and the buildings planned in 3000 ticks.

## Missions

`s2_ai::mission` plays a campaign mission or tutorial headless, started as the
menu starts it (CAM-4.3, 4.3a, UI-SETUP-2.4: `s2_rules::mission::players`, which the
viewer's `start_mission` calls too), with its script running. Nothing is
written: saves stay in the test's memory, and the profile goes to a folder of
the test's own. All of it needs `S2_GAME_DIR`.

- `crates/ai/tests/campaign.rs`: mission 1 from its start to the portal with a
  player's commands only (`TerritoryPlayer`), every soak invariant after every
  tick; lost with the headquarters on a prepared world; saved at three places
  and loaded, on to the same victory in the same tick with the same state hash;
  every campaign mission and lesson for 3,000 ticks with every invariant, no
  script error and no refused script command. `mission_rosters` and
  `what_lies_between_start_and_gate` (`--ignored`) print each mission's players,
  soldiers and stock, and what stands between its start and its portal.
- Missions played to their portal, every invariant after every tick, in
  `crates/ai/tests/campaign.rs`, each longer than a minute, so not in the default
  run: `cargo test --release -p s2-ai --test campaign is_won -- --ignored
  --nocapture` (about 3 minutes on 24 cores). `s2_ai::campaign_player` plays them
  with a player's commands only. Alone: a wood and stone economy, military
  buildings towards the portal with their soldiers sent home where no enemy is near
  (MIL-SOL-4.6), attacks; it has only the start stock's soldiers. Helped: the same
  player beside a Hard computer player on the human's own slot
  (`MissionGame::start_with_computer_help`), which is what brings food, mines and
  new soldiers; no menu starts a game like that.

  With each mission's players as its map sets them (CAM-4.3a: tribes, teams,
  strengths) and setting 1's stock (ECO-HQ-2.2), up to 300,000 ticks, four ways of
  playing each (alone, attacking always or when outnumbering; helped, attacking the
  weakest, or never and without flags on roads):

  | Mission | Enemies | Won in tick | How | What stopped the others |
  |---|---|---|---|---|
  | 1 | none | 26846 | `TerritoryPlayer`; the campaign player in 29396 to 33596 | |
  | 2 | one Weak | 58363 | alone, outnumbering; helped 77663, 113363 | alone, attacking always: lost at 146881 |
  | 3 | one Weak | 97699 | helped; also 141599 | alone: the portal 48 to 55 fields off at 300,000 |
  | 4 | two Normal | not won | | lost at 95022 to 127938; helped without attacks: alive at 300,000, 54 fields off |
  | 5 | Strong and Weak, an ally | not won | | lost at 51125 to 153535; helped without attacks: alive with 44 fields |
  | 6 | two Normal | not won | | lost at 102497 to 106650; helped: one enemy beaten, alive, 52 fields off |
  | 7 | Strong and Normal | not won | | lost at 179931 to 227635; helped without attacks: alive, 17 fields off |
  | 8 | two Strong | not won | | lost at 43838 to 110693; helped without attacks: alive, 10 fields off |
  | 9 | two Strong, one Normal | 170609 | helped, attacking the weakest | alone: lost at 195352 and 231454 |
  | 10 | three Strong | not won | | lost at 76322 to 117155 |

  The computer players are as the maps set them now, so 4 and 7, won against the
  weak stand-ins before, are not won by this player. It is the player's limit as far
  as is known: it has no food and mine economy of its own, and helped it shares one
  Hard computer player's. No invariant broke in any run. `one_run`
  (`S2_RUN=5,3,w,300000`) tries another way of playing one.
- `crates/viewer/src/menus/campaign_tests.rs`: what the interface shows of those
  games: the end screens, the next mission unlocked, the map state in the profile
  file, read back by a new `Settings`; the briefing and goal panel of every
  mission, the window of every lesson, in the words of the language files.
- The play harness, real program and real clicks (`crates/viewer/tests/play.rs`):
  `tutorial_one` follows lesson 1's every instruction from the Tutorial page to
  its end; `mission_end` goes from the island map through the briefing to the end
  screen, debriefing and mission 2; `mission_defeat` the defeat screen;
  `mission_save_quit_load` a mission saved and loaded by the program. The two end
  screens are reached on a prepared world (`Act::Prepare`): playing to the portal is
  the headless test's. `--play` never writes the player's settings or profile
  (`settings::hold_files`). `campaign_state_is_written_and_read_back` gives the
  program a profile folder of its own (`--profile-dir`, below `target/tmp/play`):
  it completes mission 1 by CAM-3.7's command, the test finds the map state in the
  file, and a new start of the program has mission 2 open.
  `mission_after_a_free_game_on_high` leaves a free game on High (88 planks) and
  starts mission 1 from the menu: 44 planks.

## While the original is recorded

Two guards for a machine that also records the original, or shares its
mouse with another computer; on any other machine they never stop a run.
The scripts that run a Windows build or install the game on the PC
(`play-test.sh`, `golden-images.sh`, `view-map.sh`, `golden.sh`,
`install-local.sh`, `frame-bench.sh`, `look-compare.sh`, `long-play.sh`)
refuse to start while the original is being recorded on the same machine, which its recorder says
with a flag file (TOOL-trace TRACE-7.3; `S2_RECORDING_FLAG` names the file).
`crates/app/tests/recording_guard.rs` runs each of them with and without the
flag.

The three that drive the pointer (`play-test.sh`, `golden-images.sh`,
`look-compare.sh`) also refuse while a shared-mouse tool (Synergy) has given
the pointer to another computer (`scripts/input-guard.sh`);
`crates/app/tests/input_guard.rs` runs them with a log of the test's
(`S2_SYNERGY_LOG`) and a stand-in pointer probe (`S2_POINTER_PROBE`).

## The player's own files

Nothing the safety net starts writes the player's settings
(`%APPDATA%\s2-reborn\settings.txt`), profiles (`Documents\S2\users`) or saves
(`Documents\S2\saves`), not even the same bytes:

- `golden-images.sh` gives each scene a profile folder of its own
  (`--profile-dir target/golden-images/profile/<scene>`), made fresh with
  only the seed profile `s2-app profile-seed` writes as its
  `users\default01.cfg` (the first start's values, SET-3, with the Very high
  graphic preset applied, SET-4a: every engine slider at its maximum; no
  settings file), and a save folder in it unless the scene names one (the menu
  scenes use `tests/golden/images/saves`). The end screens record their
  completed map (CAM-3.5) there. The pictures so no longer depend on the
  profile of the PC they are drawn on (an empty folder, the first start's
  sliders at 0, drew the far zooms, the water's mirror and the engine page
  differently from a profile at Very high).
- `frame-bench.sh` the same in `target/frame-bench/profile`, `long-play.sh` in
  `target/long-play/profile`; `look-compare.sh`
  through `s2-app shot --profile-dir` in `target/look-compare/profile/<picture>`.
- `play-test.sh`: a play script without `--profile-dir` holds both files
  unwritten (`settings::hold_files`), its saves go to `target/tmp/play/saves`.
- `install-local.sh` leaves the settings file as it is when it names the game
  folder already.
- `view-map.sh` opens the viewer for a person: what that person changes in its
  menus is written to their files, as in the game.

`scripts/player-files-guard.sh` checks it: these four scripts read the modified
times of those files when they start (names and times only, never the
contents) and fail when one changed, or a file came or went, by the end
(`golden-images.sh` and `look-compare.sh` after each scene, naming it).
`crates/app/tests/player_files_guard.rs` checks the guard on folders of the
test's (`S2_PLAYER_SETTINGS`, `S2_PLAYER_USERS`, `S2_PLAYER_SAVES`) and runs
`golden-images.sh` with a stand-in viewer that rewrites the profile.


## File formats

`crates/assets/tests/fuzz.rs` damages input on purpose and feeds it to every
decoder of the original's files: rc00 (header, decompression, decoding), s2m
maps, saves, Granny models (header, raw sections, model, Oodle1), DDS, TGA,
BMP, WAV, particle effects, the profile file, the XML tables (animations,
colour slots, landscape, models) and the text files (languages, fonts,
subtitles, terrain patterns, `menu.cfg`). None may panic. Synthetic seeds run
everywhere (CI); `--ignored` takes four real files per format from the install
(and the player's saves with `S2_SAVES_DIR`) as seeds.
`crates/rules/tests/fuzz_scripts.rs` does the same for the property scripts
(the Lua rule tables) through `script::run` and `RuleSet::sim_rules`.

What the fuzzers found (fixed; a valid file decodes as before): a Granny
file's damaged section count reserved 58 GB and aborted the process; the
Oodle1 decompressor checked its part boundaries and model invariants with
`debug_assert` (a panic in debug builds, garbage in release) and underflowed
on a parameter block without literal values: these are errors now; a BMP's
damaged size overflowed `width * height * 4`. A run of the long fuzzers
(eight install files per format, 1 500 damaged copies each, debug build;
600 and 6 000 damaged property script runs) then found nothing.

Round trips where we also write: the profile file (`cfg::write` then
`cfg::parse` gives the same settings, and a parsed file written back is the
same bytes) and the save of a headless game, the replay file:
`crates/app/tests/save_continue.rs` saves a game halfway, loads it, plays on
and requires the same hash lines as the game played straight through. The
game's own saves are checked under "Saving and loading" above.

## Spec rule coverage

Every test and every play-harness step names the spec rules it checks, in
a comment tag directly above `#[test]` (or in the step's description):

```rust
// spec: ECO-DIST-2.1, ECO-DIST-2.3 (T1)
#[test]
fn t1_grain_picks() {
```

Lists written the short way count in full (`TRN-ROAD-6.1, 6.2`, or a spec
test case and a rule: `GFX-ANIM T18, 4.1a`). A test of our own tooling that
checks no game rule says so: `// spec: none (what it checks)`.

`scripts/spec-coverage.py` reads the rules each spec defines (`- **ID**` and
heading-only sections such as `### LUA-PROP-7`) and the rules the tags name,
and prints the coverage per spec file; `--uncovered` lists the rules no test
names, `--untagged` the tests that name nothing, and `--check` (run on CI)
fails when a test names nothing. It also lists rule ids a spec defines twice.

`python3 scripts/spec-coverage.py` prints the numbers as they are now, per
spec file and in all; `scripts/metrics.sh` writes the totals with the other
progress numbers (`docs/releasing.md`, "Progress numbers").

A rule counts as covered when some test names it; how well that test checks
the rule is for review.
