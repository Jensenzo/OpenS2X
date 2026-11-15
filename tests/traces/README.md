# Our traces

Our own traces (`docs/trace-format.md`), for comparing with the
original's recordings (TOOL-trace TRACE-4.6). They are **not committed**
(since 2026-10-27): the program is deterministic, so the code makes them
again, the same bytes every time. `s2-app fidelity` makes all of them from
the game data (`S2_GAME_DIR`) before it compares (about 30 s in a release
build) and leaves them in `target/traces/`; the pairs are in
`docs/fidelity.md`. Only the original's recordings are kept, in
`specs/traces/` (in the working repository only: the public copy leaves
them out, `docs/releasing.md` "The public copy").

| File | Game | Made with |
|---|---|---|
| `MP_2P_Storm_Coast-default-40000.csv.gz` | MP_2P_Storm_Coast, the default set-up of OBS-30: player 0 a human who gives no command, player 1 a computer player of the default slot strength 0, which plays profile 1 (Weak, UI-SETUP-2.5; Q-275), start resources Medium; the game seed 0x5432ACE0 (SIM-RNG-2.1); trace ticks 0 (the world as set up, the original's counter -1) to 40 000 (TRACE-4.5, the range of the original's dense recordings, E-trace-005 to 6000, E-trace-011 to 20 000 and R4, E-trace-014, to 40 000); settler, building and generator rows, gzip without a time stamp (31 MB) | `cargo run --release -p s2-app -- trace MP_2P_Storm_Coast 40000 --ai 1=1 --from 0 --out target/traces/MP_2P_Storm_Coast-default-40000.csv.gz` |
| `MP_2P_Storm_Coast-1p-30000.csv.gz` | the one-player game of E-trace-004: MP_2P_Storm_Coast, player 0 a human who gives no command, player slot 1 closed (no player is created on it), start resources Medium, the game's seed; trace ticks 0 to 30 000, settler, building and generator rows, gzip without a time stamp | `cargo run --release -p s2-app -- trace MP_2P_Storm_Coast 30000 --ai none --closed 1 --from 0 --out target/traces/MP_2P_Storm_Coast-1p-30000.csv.gz` |
| `MP_2P_Storm_Coast-hard-20000.csv.gz` | the default game with player 1 a computer player of profile 3 (Strong) instead, the game of E-trace-012 (the original's slot strength 2); trace ticks 0 to 20 000, settler, building and generator rows, gzip without a time stamp | `cargo run --release -p s2-app -- trace MP_2P_Storm_Coast 20000 --ai 1=3 --from 0 --out target/traces/MP_2P_Storm_Coast-hard-20000.csv.gz` |
| `MP_2P_Storm_Coast-default-generator-150000.csv.gz` | the default game, generator rows only, trace ticks 0 to 150 000, gzip without a time stamp | `cargo run --release -p s2-app -- trace MP_2P_Storm_Coast 150000 --ai 1=1 --from 0 --kinds G --out target/traces/MP_2P_Storm_Coast-default-generator-150000.csv.gz` |
| `MP_2P_Storm_Coast-hard-150000.csv.gz` | the Hard game (player 1 of profile 3), trace ticks 0 to 150 000: generator rows every tick, settler and building rows every 10th trace tick (the original's long recording R3 holds them every 10th to 40 000 and every 100th after; `clip_time` counts from the rows written), gzip without a time stamp (70 MB) | `cargo run --release -p s2-app -- trace MP_2P_Storm_Coast 150000 --ai 1=3 --from 0 --rows-every 10 --out target/traces/MP_2P_Storm_Coast-hard-150000.csv.gz` |

The file names and arguments are `fidelity::ALL_OURS` in
`crates/app/src/fidelity.rs`. Comparing with the original's recording
(`specs/traces/`, converted to the same format by the investigation,
TRACE-4.4) is `s2-app fidelity` (`docs/fidelity.md`); one pair by hand, after
a fidelity run (or the trace command above):

```bash
cargo run --release -p s2-app -- trace-diff target/traces/MP_2P_Storm_Coast-default-40000.csv.gz specs/traces/original-dense-20000.s2trace.gz --by-place --common-ticks --from 1 --ignore clip_time
```

`--by-place` matches settlers by place since the two programs number them
differently; `--common-ticks` leaves out ticks the recording missed. The
generator rows alone (the first tick whose draws differ, `docs/rng-diff.md`):
`--kinds G`. Our computer player draws from the simulation's generator
(AI-2.7) and parts from the original's where a rule is not built yet
(`docs/fidelity.md`); the one-player game has no computer player, so its
generator rows are the animals' draws alone.

Before 2026-10-27 these traces were committed here and
`crates/app/tests/reference_trace.rs` checked that they were current; every
rule that changed behaviour rewrote the 9 MB trace, which made the repository
grow by megabytes a commit. They are in the history up to that date.
