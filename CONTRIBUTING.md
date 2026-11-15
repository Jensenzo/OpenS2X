# Contributing to OpenS2X

Thank you for wanting to help. This page says what helps most, the one rule
everyone has to keep, how patches reach the program, and how to build and
test it.

## What helps most

1. **A bug report with steps and a save.** What you did, what happened, what
   you expected, and the version (the download's name, or the commit). A
   save from just before the problem helps most: OpenS2X's saves are in
   `Documents\S2\saves` (`.s2save`); zip it and drag it into the issue.
   [Report a bug](https://github.com/Jensenzo/OpenS2X/issues/new?template=bug.yml).
2. **A difference from the original**, with a screenshot of each program at
   the same place: OpenS2X's (F12 saves one, into a `screens` folder) and
   the original's. Say what differs and where (the map, or a save of either
   program). The aim is the
   original exactly, so even small differences in timing, colour or a
   window's layout count.
   [Report a difference](https://github.com/Jensenzo/OpenS2X/issues/new?template=differs.yml).
3. **A question** about playing, building or the code:
   [ask it](https://github.com/Jensenzo/OpenS2X/issues/new?template=question.yml).

Please search the open issues first, and keep one problem per issue.

## The clean-room rule

OpenS2X is written from specifications of the original's behaviour, in our
own words ([`specs/`](specs/)), never from its code. The same holds for
everything you send, in issues as in patches:

- **No decompiled or disassembled code** of the original, no pseudocode made
  from it, no addresses or names from inside its program.
- **No game files**: no maps, models, textures, sounds, texts or scripts from
  the game, and no files made from them. Screenshots of the original to show
  a difference are fine.
- **No code from GPL-licensed reimplementations** (Return to the Roots,
  OpenSettlers and others): OpenS2X is Apache-2.0 and may not take their
  code. Ideas are fine; say where they came from, and they are checked
  against the original before anything is built on them.

A rule that is missing or unclear in the specs becomes a question in
[`specs/OPEN_QUESTIONS.md`](specs/OPEN_QUESTIONS.md), not a guess in the code.

## Patches and pull requests

This repository is a **release mirror**: development happens in the
maintainer's working repository, and each release lands here as one commit.
So pull requests cannot be merged here, but they are welcome as a way to
send a patch. An accepted patch is applied in the working repository with
your name in the commit (as its author, or with a `Co-authored-by` line),
and it appears here with the next release. The pull request is then closed
with a note saying which release has it.

Before you send one:

- Keep it small and about one thing; say which spec rule it implements or
  which issue it fixes.
- Every rule the code implements names its spec id in a comment (for
  example `// TRN-ROAD-6.6`), and every test names the rules it checks
  (`// spec: TRN-ROAD-6.6`).
- The simulation (`crates/sim`) stays deterministic: no floating point in the
  game state, no wall-clock time, no unseeded random numbers, no I/O.
- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass.
- A change that should not alter the game leaves the golden state hashes and
  pictures as they are. One that does names the spec rule that makes them
  change ([`docs/test-coverage.md`](docs/test-coverage.md)).

## Building and testing

The [README's "Build from source"](README.md#build-from-source) has the
prerequisites and commands. In short:

```bash
cargo test --workspace                 # tests that need the game skip without S2_GAME_DIR
export S2_GAME_DIR=/path/to/your/game  # the folder that contains data/
cargo test --workspace                 # now with the game's files
python3 scripts/spec-coverage.py --check
python3 scripts/public-check.py .
```

The tests that compare with recordings of the original need those recordings,
which are not published; they skip with a line saying so. Where to find each
kind of document: [`docs/index.md`](docs/index.md). The architecture and its
rules: [`docs/architecture.md`](docs/architecture.md).

## Conduct

Be kind and stay on the subject: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
A security problem goes to a private report, not an issue:
[`SECURITY.md`](SECURITY.md).
