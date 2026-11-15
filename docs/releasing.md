# Versions and releases

OpenS2X is released as git tags `vX.Y.Z`. The public repository
`Jensenzo/OpenS2X` is a release mirror: development happens in the
maintainer's working repository, and each release lands here as one commit
with its tag ("The public copy"). A project that builds on these crates pins
one tag ("Pinning a release"); it never follows `main`.

## Version numbers (semver)

The workspace has one version (`[workspace.package] version` in `Cargo.toml`)
for all crates. Before 1.0, any release may break a project built on these
crates; the version says how large a step it is, not whether it breaks:

| Bump | What it means | Examples |
|---|---|---|
| minor (0.8 -> 0.9) | a larger step of the game or of the crates | 0.9.0: the first release with a Windows download |
| patch (0.9.2 -> 0.9.3) | the releases within it: fixes, closer to the original, new hooks, docs | 0.9.3 renamed the game's binary and raised the snapshot version; 0.9.6 changed only documents |

A breaking change is one a project notices: a public item of `s2-sim`,
`s2-assets`, `s2-rules`, `s2-ai` or the viewer's hooks (`docs/api.md`,
`docs/extending.md`) removed, renamed or changing meaning, a new field in a
struct a project may build, a new snapshot version, a changed golden hash
or golden picture (a game rule changed). A minor or a patch release may
carry one (0.9.2 and 0.9.3 did); every one is listed under "For projects
built on OpenS2X: breaking changes" in its `CHANGELOG.md` section, so a
project reads that section before it moves its pin.

After 1.0 the usual rules apply (major for breaking changes).

## The changelog

[`CHANGELOG.md`](../CHANGELOG.md) has one short section per release, newest
first, written by hand for players and for developers: "What is new",
"Fixed", and "For projects built on OpenS2X: breaking changes". The first
public release (0.10.1) also says what OpenS2X holds as a whole ("What this
release holds"), as its notes are the first a reader of the public
repository sees. The release notes on GitHub are that section with a
two-line preface (`scripts/changelog.py --notes X.Y.Z`; `--check X.Y.Z`
fails when the section is missing).

The full wording of every breaking change, one line per change as
`version | what changed and what to do`, is kept in
[`docs/breaking.txt`](breaking.txt); the changelog's breaking section says
the same in fewer words.

## What a release checks

Every release commit has passed, in this order:

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings` and `cargo test --workspace`, with the game installed (the tests
  that need its files run);
- `scripts/spec-coverage.py --check` (every test names the spec rules it
  checks) and `scripts/public-check.py .` (nothing that must not be public);
- the golden state hashes of whole games, on Linux and on the Windows build,
  which must be the same (`scripts/golden.sh`);
- the golden pictures of 60 scenes, the same to the pixel within a small
  tolerance (`scripts/golden-images.sh`);
- the play harness, which drives the real Windows program with the mouse and
  the keyboard through the menus, the tutorial and the campaign
  (`scripts/play-test.sh`);
- the fidelity measure against the recordings of the original
  ([`docs/fidelity.md`](fidelity.md)): no first difference may move earlier;
- CI green on the release commit.

[`docs/test-coverage.md`](test-coverage.md) describes each of these and how
an intended change is accepted.

## Progress numbers

`scripts/metrics.sh [FILE]` writes the numbers the README's "How close it is"
quotes into a JSON file, with the date and commit (`dirty` when `crates/` or
`tests/` hold uncommitted changes):

| Key | What | From |
|---|---|---|
| `fidelity` | per pair: the first difference, the settler column it leaves out (`without`: `animation` for E-trace-002's view lag, else none) and the measures (first differing tick per kind, matching and compared ticks) | `s2-app fidelity` (`docs/fidelity.md`) |
| `inventory` | items in all and per category: specified (a spec section), open (none), verified (traced, or observed in the original), traced, observed | `specs/INVENTORY.md` |
| `questions` | open questions by status: open (narrowed counts as open), partly, answered, closed, other | `specs/OPEN_QUESTIONS.md` |
| `stand_ins` | the lines of code marking a stand-in, their files and the questions they name | `crates/` |
| `unbuilt` | the items of "Not built" that are not struck | `docs/unbuilt.md` |
| `tests` | tests and harness steps | `scripts/spec-coverage.py` |

`python3 scripts/metrics.py --test` checks the parsers (CI).

## The Windows download

`scripts/dist.sh X.Y.Z` builds `OpenS2X.exe` with the dist profile and packs
`target/dist/OpenS2X-X.Y.Z-windows-x64.zip` with `README.md` (without its
picture lines, which only GitHub shows), `PLAYING.md`, `CHANGELOG.md`,
`LICENSE` and `THIRD-PARTY-LICENSES.txt`, the notices of every crate the exe
links (`scripts/third-party-licenses.py`: their licence and notice files from
the cargo registry, Lua's notice, where symphonia's source is). MIT, BSD,
Zlib, ISC and Apache require those notices to go with a binary. The archive
is made from that fixed list and its listing checked against it, so no game
file can slip in. The game's films are not in it: the player converts them
from their own copy ([`docs/video.md`](video.md)).

## The public copy

The working repository's history holds what is not published (every version
of the specs while they still carried the original program's addresses, the
recordings of the original in `specs/traces`, the maintainer's notes). The
public repository has a fresh history: one commit per release, made from
the release's tree. Its history starts with its first public release.

```bash
python3 scripts/public-check.py --report .        # what would fail, per kind and file
python3 scripts/public-check.py --report . specs  # the specs alone
scripts/public-export.sh vX.Y.Z ../OpenS2X-public  # a folder of your choice
```

`scripts/public-export.sh COMMIT [TARGET]` writes `git archive COMMIT` into
TARGET (default: a folder `OpenS2X` under `~/s2/public`, the maintainer's
layout; name any other) without what
`scripts/public/exclude.txt` leaves out (each with its reason) and with the
public CI (`scripts/public/ci.yml`, GitHub's hosted runners only, no secrets,
no variables, a read-only token) as `.github/workflows/ci.yml`. TARGET is a
git repository of its own (made on the first export); each export replaces
its tree and adds one commit "OpenS2X VERSION" under the GitHub no-reply
identity, and tags it at a release tag. It never adds a remote or pushes.
`scripts/public-check.py` must find nothing in the result, else nothing is
committed (`--draft` commits anyway, marked as a draft, for a dry run's
folder only).

`scripts/public-check.py FOLDER [PATH...]` fails with file and line on what
must not be public: game file types, pictures (but the README's, which
`scripts/public/allow.txt` names one by one), files over 1 MB, the original
program's addresses, struct offsets, its game event names and numbers (the
commands have names of our own), virtual tables and decompiler names, the
investigation's tools and private paths anywhere (and the words for its
methods, decompiled or disassembly, in the specs), local users' paths and
e-mail addresses, the maintainer's private folders and repositories, the
words of the maintainer's working process (run names, finding ids, planning
steps), a workflow that can reach a self-hosted runner, a variable or a
secret, and a Markdown link to a file the export leaves out or that is not in
the tree (`crates/app/tests/public_check.rs`). Its exceptions are
`scripts/public/allow.txt`, each with its reason. On this tree it checks what
the export would publish; the public CI runs it on every push and pull
request that changes more than Markdown files.

The public copy's tests pass without the game, its saves and the recordings:
each test that needs one skips with a line saying so (with the game but
without the recordings, the tests of `crates/ai/tests/real_game.rs` that
compare with a table of `specs/traces` print "skipped: specs/traces/... is
missing").

## CI

The public repository's CI (`.github/workflows/ci.yml`) runs on GitHub's
hosted runners, on every push and pull request that changes code:
formatting, Clippy, `cargo test --workspace` (the tests that need the game
skip), [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) (licences,
advisories, sources), the spec-coverage check, the API boundary check, the
public check, the fixture hashes and short soaks; then the Windows build, its
tests and its fixture hashes, which must be the same as Linux's. It also runs
weekly and by hand.

Left out of the public CI, because they need the game data, the recordings
or a desktop: the fidelity measure, the play harness and the golden images.
Each release has passed them on the maintainer's PC ("What a release
checks").

## Pinning a release

A project built on these crates (`docs/extending.md`) takes them from the
public repository by tag. Its tags start with its first public release;
in the example, `vX.Y.Z` stands for the latest release tag (the
[Releases](https://github.com/Jensenzo/OpenS2X/releases/latest) page):

```toml
[workspace.dependencies]
s2-sim = { git = "https://github.com/Jensenzo/OpenS2X", tag = "vX.Y.Z" }
s2-assets = { git = "https://github.com/Jensenzo/OpenS2X", tag = "vX.Y.Z" }
s2-rules = { git = "https://github.com/Jensenzo/OpenS2X", tag = "vX.Y.Z" }
s2-ai = { git = "https://github.com/Jensenzo/OpenS2X", tag = "vX.Y.Z" }
s2-viewer = { git = "https://github.com/Jensenzo/OpenS2X", tag = "vX.Y.Z" }
```

All crates come from the same tag. To move to a new release, change the tag
in one place, read the new `CHANGELOG.md` section (its breaking changes) and
`docs/api.md`'s change notes, and run the project's tests. A need such a
project has from the engine becomes a hook here first
(`docs/extending.md`), released in a patch version.
