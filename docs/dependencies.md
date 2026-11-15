# Dependencies

Few, pinned, permissive. `deny.toml` is checked by `cargo deny check` on
every CI run (job `deny`): licences, RustSec advisories, sources.

## Rules

- **No GPL, LGPL or AGPL code linked into a binary.** `deny.toml` allows
  only permissive licences and MPL-2.0 (file-level copyleft, linking
  allowed). A crate that offers a GPL-family licence only as an
  alternative (`r-efi`: MIT OR Apache-2.0 OR LGPL) is taken under the
  permissive one.
- **FFmpeg stays an external tool.** `scripts/install-local.sh --video`
  runs the player's or the system's `ffmpeg` to convert the films
  (`docs/video.md`); nothing links it. The game reads the converted
  Motion-JPEG with `zune-jpeg`.
- **Nothing from Return to the Roots or OpenSettlers** (GPL), as code or as
  a dependency.
- **Exact versions** (`=x.y.z`) for every direct dependency outside the
  workspace, and the toolchain pinned in `rust-toolchain.toml`: the golden
  hashes and images depend on them.
- **`s2-sim` depends on nothing.** The dependency graph points inward:
  `app`/`viewer -> ai -> rules -> assets -> sim` (`docs/architecture.md`).
  For that reason its snapshot format (`docs/snapshot.md`) is written by
  hand, not with a serialisation crate.
- Every advisory fails CI unless `deny.toml` ignores it with a reason that
  says why it does not reach us and what ends the ignore.

## What we depend on

| Crate | In | For | Licence |
|---|---|---|---|
| `mlua` 0.12.2 (lua51, vendored) | rules | the property and mission scripts (LUA-PROP) | MIT (Lua: MIT) |
| `bevy` 0.20.0 | viewer | windows, input, drawing, sound | MIT OR Apache-2.0 |
| `zune-jpeg` 0.4.21 | viewer | the converted films' frames | MIT OR Apache-2.0 OR Zlib |
| `rfd` 0.17.2 | viewer (Windows) | the game-folder picker of the first run | MIT |
| `windows-sys` 0.61.2 | viewer (Windows) | the documents folder (SET-1.1) | MIT OR Apache-2.0 |
| `png` 0.18.1 | app | `image-diff` for the golden images | MIT OR Apache-2.0 |
| `flate2` 1.1.10 (Rust back end, `miniz_oxide`) | app | reading and writing gzip traces (`specs/traces/`, ours in `target/traces/`) | MIT OR Apache-2.0 |

Bevy's features are chosen by hand (`crates/viewer/Cargo.toml`): the 3D
profile without glTF, glTF animation and morph targets (our models are
`.gr2`, read by `s2-assets`); on Windows audio with WAV and MP3 (the music;
MP3 through `symphonia`, MPL-2.0).

### Known advisory

- RUSTSEC-2026-0344, `ruzstd` 0.8 (decompression bomb). Bevy 0.20 requires
  0.8 and uses it only for the tonemapping tables compiled into Bevy itself.
  We load no KTX2 or zstd files; our textures are DDS, decoded by
  `s2-assets`. Ignored in `deny.toml` until the next Bevy upgrade.

## Build time

Clean build of the workspace, all targets (`cargo build --workspace
--all-targets`, debug, no debug info as on CI), 24 threads, WSL2:

| | Wall | CPU | Target dir | Packages |
|---|---|---|---|---|
| before (0.2.0 dependencies) | 60.7 s | 381 s | 4.7 GB | 539 |
| after (Bevy without glTF; `s2-assets` a test-only dependency of `s2-ai`) | 60.2 s | 364 s | 4.6 GB | 529 |

The wall time is set by the longest chain (Bevy's render crates, then the
viewer), so dropping side crates saves CPU, not much wall time. Removed:
`bevy_gltf`, `gltf`, `gltf-json`, `gltf-derive`, `bevy_world_serialization`,
`serde_json`, `base64`, `inflections`, `itoa`, `zmij`.

To measure again:

```bash
rm -rf target/bt && /usr/bin/time -f "%e s wall, %U s user" \
  env CARGO_TARGET_DIR=target/bt CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo build -q --workspace --all-targets
```
