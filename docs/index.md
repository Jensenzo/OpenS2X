# Documentation

Every document of the repository, by what you want to do.

## Start here

| Document | What |
|---|---|
| [`README.md`](../README.md) | what OpenS2X is; from clone to running game and tests |
| [`PLAYING.md`](../PLAYING.md) | playing the game: setup, menus, the in-game bar, keys, saving and loading |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | reporting a bug or a difference from the original, the clean-room rule, patches, building and testing |
| [`architecture.md`](architecture.md) | crates, dependency graph, the viewer's modules, determinism, the audit |

## Building on OpenS2X

| Document | What |
|---|---|
| [`api.md`](api.md) | the simulation's public API: commands in, state, views and events out; what is not API |
| [`extending.md`](extending.md) | the extension points: rules overlays, asset folders, render options, plugins, menu entries and pages, captions, start-up films, music, camera, window title, settings files, the player's folders; what must never be patched |
| [`releasing.md`](releasing.md) | semver tags, the changelog, what a release checks, the public copy, how a project built on it pins a version |
| [`CHANGELOG.md`](../CHANGELOG.md) | what each release brought, for players and for developers |
| [`dependencies.md`](dependencies.md) | dependencies, licences (`deny.toml`), the known advisory, build times |

## Checking the program

| Document | What |
|---|---|
| [`test-coverage.md`](test-coverage.md) | the safety net: hashes, golden images, soak, fuzzing, the play harness; how to bless an intended change |
| [`trace-format.md`](trace-format.md) | the per-tick trace of settlers, buildings and the random generator, for comparison with the original |
| [`fidelity.md`](fidelity.md) | how close our game is to the original's recording: the first differing tick and the matching generator ticks, the record the maintainer's CI checks; what runs where |
| [`snapshot.md`](snapshot.md) | our own snapshot of the whole game state: save at any tick, load, continue; in soak and trace |
| [`rng-diff.md`](rng-diff.md) | our random draws per tick against the original's (SIM-RNG-2.5), and which of our systems draw |
| [`compare.md`](compare.md) | the look comparison: the same shot from both programs |
| [`unbuilt.md`](unbuilt.md) | what the specs describe that the program does not build yet |
| [`stand-ins.md`](stand-ins.md) | where a spec leaves a detail open: what the program does meanwhile, marked in the code by its question (Q-256) |

## How the picture and sound are made

| Document | What |
|---|---|
| [`clocks.md`](clocks.md) | which clock drives what in the viewer (game time, real time) |
| [`colour.md`](colour.md) | from texel to pixel: lighting, colour space, where we differ from Bevy's defaults |
| [`shadows.md`](shadows.md) | Bevy's shadow map against GFX-VIEW-2.5 |
| [`video.md`](video.md) | the films: converted once with FFmpeg, played from Motion-JPEG |

## The specifications (the only source of game behaviour)

| Document | What |
|---|---|
| [`specs/README.md`](../specs/README.md) | how the specs are written and named; the area codes |
| [`specs/OPEN_QUESTIONS.md`](../specs/OPEN_QUESTIONS.md) | the questions the program has asked the investigation, each with its status and answer |
| [`specs/INVENTORY.md`](../specs/INVENTORY.md) | everything the original contains, and how much of it is specified |
| [`specs/TRACED.md`](../specs/TRACED.md) | behaviour measured in the running original |
| [`specs/OBSERVED.md`](../specs/OBSERVED.md) | behaviour seen while playing the original |
| [`specs/_TEMPLATE.md`](../specs/_TEMPLATE.md) | the template of a spec |

The spec files themselves are `specs/<AREA>-<topic>.md`, such as
`ECO-worker-cycle.md` or `GFX-world-view.md`; `specs/README.md` names the
area codes.

## Release records

- [`releases.txt`](releases.txt): every release's version, one line each
  (written by `scripts/release.sh`; the number after it counts the
  maintainer's work log lines the release covers).
- [`breaking.txt`](breaking.txt): what each release breaks and what to do
  (listed first in its `CHANGELOG.md` section).
