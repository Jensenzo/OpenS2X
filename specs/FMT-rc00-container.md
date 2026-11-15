# FMT-RC: The `rc00` file container

Status: verified
Owner question(s): Q-006 (a.k.a. "Q-container") in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Most data files of the game (scripts, XML, text, maps, models, textures, shaders)
are not stored plainly. Each is wrapped individually: the original contents are
compressed with a small LZSS scheme, the compressed bytes are scrambled with a
keyed XOR, and a 20-byte header with two checksums and the plain size is put in
front. There is no archive: one wrapped file holds exactly one plain file, at the
same path and with the same name. The key is not secret per install; it is
derived from a constant and from the file's own name.

Verification status: a decoder written from these rules decodes all 1700 wrapped
files of the install, and for every one of them the three header checks
(FMT-RC-1.4, FMT-RC-1.5, FMT-RC-1.6) pass.

## Rules

All integers are little-endian. "CRC-32" always means the standard
CRC-32 used by zlib/PNG/zip (reflected polynomial 0xEDB88320, initial value and
final XOR 0xFFFFFFFF).

### FMT-RC-1 File layout

- **FMT-RC-1.1** A wrapped file is a 20-byte header followed by the body. The body
  runs to the end of the file; its length is `file size - 20` and is not stored.

  | Offset | Size | Field | Meaning |
  |---|---|---|---|
  | 0 | 4 | magic | bytes `12 18 09 06` (u32 `0x06091812`) |
  | 4 | 4 | tag | ASCII `rc00` |
  | 8 | 4 | data_crc | CRC-32 of the plain contents |
  | 12 | 4 | key_crc | CRC-32 of the 16-byte file key (FMT-RC-2) |
  | 16 | 4 | plain_size | length of the plain contents in bytes |
  | 20 | ... | body | LZSS stream (FMT-RC-4), XOR-scrambled (FMT-RC-3) |

- **FMT-RC-1.2** A file whose first four bytes are not the magic is a plain file
  and is used as is. (Observed from the install: `.ini`, `.fnt`, `.pfx`, `.bin`,
  `.lep`, `.bmp`, `.png`, `.wav`, `.mp3`, `.bik`, `brushes.dat` are plain. Where
  exactly the game makes this decision was not traced; confidence medium.)
- **FMT-RC-1.3** Decoding order: check tag, derive key, check key_crc, descramble
  body, decompress, check plain_size, check data_crc.
- **FMT-RC-1.4** The tag must equal `rc00`.
- **FMT-RC-1.5** CRC-32 of the derived key must equal key_crc. A mismatch means
  the file was renamed (FMT-RC-2.2) or is not from this game.
- **FMT-RC-1.6** The decompressed length must equal plain_size and the CRC-32 of
  the decompressed bytes must equal data_crc.
- **FMT-RC-1.7** In the original game each failed check is logged
  ("FCC mismatch", "pw crc mismatch", "datasize mismatch", "data crc mismatch")
  and then the game deliberately crashes. A reimplementation should treat each as
  a fatal load error for that file.

### FMT-RC-2 File key

- **FMT-RC-2.1** The key is 16 bytes. It starts as the base key
  `C9 59 46 CA D9 F0 4F 0A A1 00 AA B8 CB E8 DB 6B`.
- **FMT-RC-2.2** Take the file path the game was asked to open, convert it to
  lower case, and keep only the part after the last `\` or `/` (the base name,
  including extension). No directory component takes part.
- **FMT-RC-2.3** If the base name ends in `.s2m` or `.sav`, the key is the base
  key unchanged. This is why maps can be renamed and still load. (`.s2m` is
  verified on all 36 maps, which all carry the same key_crc `0xD7E4A7F3`. `.sav`
  verified 2026-11-12 on four saves, two of them renamed by hand (`Autosave-2026-10-27-0905.sav`,
  E-sav-005; the 2023 save under its dated name): all decode with the base key unchanged.)
- **FMT-RC-2.4** Otherwise compute CRC-32 over the bytes of the lower-case base
  name, seed a generator (FMT-RC-5) with it, draw 16 bytes, and XOR byte *i* of
  the base key with the *i*-th drawn byte.
- **FMT-RC-2.5** Consequence: two wrapped files with the same base name share a
  key regardless of directory, and a non-map file stops decoding if it is renamed.

### FMT-RC-3 Body descrambling

One generator (FMT-RC-5) is seeded with the CRC-32 of the 16-byte key and then
used for everything below, in this order. `L` is the body length.

- **FMT-RC-3.1** Pass 1, whole body. Draw one value `v`; pad length
  `n = (v AND 0x7F) + 0x80` (128..255). Draw `n` bytes into a pad. XOR body byte
  `i` with pad byte `i mod n`, for every `i` in `0..L-1`.
- **FMT-RC-3.2** Pass 2, sparse. Draw one value `v`; second pad length
  `m = (v AND 0x0F) + 0x11` (17..32). Draw `m` bytes into a second pad. Draw one
  value and reduce it modulo `L`: this is the first position `p`. Draw one value
  `v`; `step = (v AND 0x1FFF) + 0x2000` (8192..16383).
- **FMT-RC-3.3** For `p`, `p + step`, `p + 2*step`, ... while the position is
  below `L`: XOR the body byte at that position with second-pad byte number
  `(key[position mod 16] XOR position) mod m`. Here the key byte is widened to a
  32-bit unsigned value, XORed with the full position, and the remainder is
  unsigned.
- **FMT-RC-3.4** Pass 2 always touches at least one byte (since `p < L`). For a
  body shorter than 8192 bytes it touches exactly one.
- **FMT-RC-3.5** Both passes are XOR, so scrambling a body for writing uses the
  identical procedure.
- **FMT-RC-3.6** A zero-length body is not expected (pass 2 would divide by
  zero). Reject it.

### FMT-RC-4 Decompression (LZSS)

- **FMT-RC-4.1** State: a 1024-byte ring buffer, every byte initialised to
  `0x20` (space), and a write cursor starting at ring position `0x3F0` (1008).
  (The original only clears positions 0..1007 and reuses a static buffer, so
  positions 1008..1023 hold leftovers from the previous file until written. No
  shipped file depends on them; initialise the whole ring to spaces.)
- **FMT-RC-4.2** The stream is a sequence of groups. A group is one flag byte
  followed by up to eight items. Flag bits are consumed from the least significant
  bit upward; one bit per item.
- **FMT-RC-4.3** Flag bit 1: literal. Read one byte, emit it.
- **FMT-RC-4.4** Flag bit 0: copy. Read two bytes `a` then `b`.
  Ring position `pos = a + ((b AND 0xF0) * 16)`, length `len = (b AND 0x0F) + 3`
  (3..18). Emit `len` bytes read one at a time from ring positions
  `pos, pos+1, ...`, each taken modulo 1024. `pos` is an absolute ring position,
  not a distance back from the cursor. Since positions wrap at 1024, the top two
  bits of `b` have no effect.
- **FMT-RC-4.5** Every emitted byte (literal or copied) is appended to the output
  and also stored at the write cursor, which then advances by one modulo 1024.
  Copies are byte-by-byte, so a copy may read bytes it has just written.
- **FMT-RC-4.6** Decoding ends when the input runs out, including in the middle
  of a group or when only one of the two bytes of a copy item remains (that last
  byte is dropped). There is no end marker and the output length is not known to
  the decompressor; it is checked afterwards (FMT-RC-1.6).

### FMT-RC-5 Generator

- **FMT-RC-5.1** State is one 31-bit value. Each draw replaces the state with
  `state * 16807 mod 2147483647` and yields the new state. "Draw a byte" means the
  low 8 bits of the yielded value; "draw a value" means the whole value.
- **FMT-RC-5.2** Seeding from a 32-bit number `s`: clear the top bit
  (`s AND 0x7FFFFFFF`). Count the set bits `c` among bits 0..30.
  - If `c < 8`: set bits at the first `8 - c` positions of the list
    `12, 23, 10, 25, 8, 27, 6, 29`.
  - If `c > 24`: clear bits at the first `32 - c` positions of the same list.
  - If the result is 0 the state becomes 1; otherwise the state is the result.
- **FMT-RC-5.3** The original computes the product with a 16-bit split and
  end-around carry rather than a 64-bit multiply. The two agree for every state
  in `1..2147483646`. (Note for exactness: the split method can never yield 0 and
  could in principle yield 2147483647 where a true modulo yields 0; no state
  reached from a valid seed produces this.)

### FMT-RC-6 What comes out

- **FMT-RC-6.1** The plain contents are ordinary files of the type the extension
  says. Decoding the whole install gives: 884 Granny `.gr2`, 503 DDS textures
  (+1 file named `.dds` that is really a 24-bit BMP, `data/animals/pig.dds`),
  156 Lua scripts, 63 TGA, 36 `.s2m` maps (own binary format, see Q-002),
  33 HLSL text files (`.fx`, `.inc`), 15 XML, 9 text (`.txt`, `menu.cfg`).
- **FMT-RC-6.2** All 156 Lua files are plain Lua source text with CRLF line ends.
  None is precompiled Lua 5.0 bytecode.
- **FMT-RC-6.3** Total: 171,270,252 bytes wrapped become 299,775,293 bytes plain.
  Small or incompressible files can be larger wrapped than plain.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| magic | `0x06091812` (bytes `12 18 09 06`) | u32 | high | E-fmt-001 |
| tag | `rc00` | 4 bytes | high | E-fmt-001 |
| header_size | 20 | bytes | high | E-fmt-001 |
| base_key | `C95946CAD9F04F0AA100AAB8CBE8DB6B` | 16 bytes | high | E-fmt-001 |
| path_separators | `\` and `/` | chars | high | E-fmt-001 |
| fixed_key_extensions | `.s2m` (high), `.sav` (high, 2026-11-12) | | see FMT-RC-2.3 | E-fmt-001 |
| pad1_length | `(v & 0x7F) + 0x80` | bytes | high | E-fmt-001 |
| pad2_length | `(v & 0x0F) + 0x11` | bytes | high | E-fmt-001 |
| pass2_step | `(v & 0x1FFF) + 0x2000` | bytes | high | E-fmt-001 |
| rng_multiplier | 16807 | | high | E-fmt-001 |
| rng_modulus | 2147483647 | | high | E-fmt-001 |
| seed_min_bits / seed_max_bits | 8 / 24 | set bits | medium | E-fmt-001 |
| seed_fix_positions | 12, 23, 10, 25, 8, 27, 6, 29 | bit index | medium (path not exercised by a test file) | E-fmt-001 |
| lzss_ring_size | 1024 | bytes | high | E-fmt-001 |
| lzss_fill | `0x20` | byte | high | E-fmt-001 |
| lzss_start_cursor | `0x3F0` | ring position | high | E-fmt-001 |
| lzss_min_copy / max_copy | 3 / 18 | bytes | high | E-fmt-001 |
| crc | CRC-32 (zlib) | | high | E-fmt-001 |

"high" here means: read from the binary and confirmed by all 1700 files passing
their checksums.

## Test cases

### Whole-file vectors

Input paths are relative to the game install. Expected is the SHA-256 of the
plain contents. T1..T10 must also pass FMT-RC-1.4 to 1.6.

| ID | Input | Wrapped size | Plain size | Expected SHA-256 |
|---|---|---|---|---|
| T1 | `data/scripts/properties/streets.lua` | 136 | 229 | `a55af21cb9a9e7c0808c1bd7af0dd2a81060dcb5424f725f4a8c4b14a0a12e24` |
| T2 | `data/scripts/properties/config.lua` | 466 | 898 | `58a03b99e76a88d79f5fad1e0d37f4f211b6b43400a2469b5a48c92f860ad8f5` |
| T3 | `data/scripts/properties/buildings.lua` | 16007 | 54737 | `56148a7d6edf478709deb2a3b9a5973b66526a7403fd9f7f5a64077c6f68f25b` |
| T4 | `data/settings/decals.xml` | 124 | 133 | `25d28a0bfd052158171d0a0cb0cdea7722ef98c2ddcf07ca5ab4bdf97e4e2466` |
| T5 | `data/scripts/menu.cfg` | 231 | 458 | `6dab3c747896c6d28ad1d16ffc9648589a4d03f45209192d72de7e5f9d6c370e` |
| T6 | `data/language/gb/info.txt` | 11031 | 24404 | `4a6acb05d2fc2fe1f0f038744d18fcd3906843025c6044a577d2f135e6016909` |
| T7 | `data/gfx/fx_20/shared.inc` | 7847 | 23192 | `48bce141f85037671509b6ff60d9bd85359e81133c58e4ca7e2afcd93a929314` |
| T8 | `data/map/textures/z_snow_00.dds` | 100 | 184 | `be89d064a18e4c3b8c3da29d4c853f2052152f1ee13938daba146d0dee73bf13` |
| T9 | `data/character/colormaps/eye_table.tga` (wrapped is larger than plain) | 104 | 92 | `898ac4591172ddbb1615819c561e8479a5ce4a25661c08c60f4dee67e08e0801` |
| T10 | `data/maps/campaignMaps/campaign01.s2m` (fixed key, multi-step pass 2) | 610653 | 1488768 | `a5403060eba0b55483d0f38661c68c46534e021684c70055d67ea7ace0349b6b` |

T3, T6, T7, T10 have bodies longer than 8192 bytes, so pass 2 touches several
positions. T10 exercises FMT-RC-2.3.

### Step vectors

| ID | Setup | Action | Expected |
|---|---|---|---|
| S1 | generator state `0x12345678` (already valid, 13 set bits) | draw three values | `0x2A18E99E`, `0x499199A9`, `0x75FF4BFA` |
| S2 | seed with `3` (2 set bits) | read state after seeding | `0x0A801503` |
| S3 | seed with `0xFFFFFFFE` (30 set bits after masking) | read state after seeding | `0x7F7FEFFE` |
| S4 | name `Data\Scripts\Properties\STREETS.LUA` | base name CRC-32 | `0x2B2965CE` (same as for `streets.lua`) |
| S5 | name `streets.lua` | derive key | `4CE7375BB21B1133761970EF80F5CA37`, CRC-32 `0xF6F4A1B8` |
| S6 | key from S5 | pass 1 pad length | 139 |
| S7 | name `campaign01.s2m` | derive key | base key, CRC-32 `0xD7E4A7F3`; pass 1 pad length 153 |
| S8 | T1 input | read header | data_crc `0x2F0FDB76`, key_crc `0xF6F4A1B8`, plain_size 229 |
| S9 | T1 input copied to `roads.lua` | decode | fails FMT-RC-1.5 |
| S10 | T10 input copied to `anything.s2m` | decode | succeeds, same SHA-256 as T10 |
| S11 | T1 output | first bytes | CR LF, then `-------------- Streets` |

S2 and S3 are produced by our decoder following FMT-RC-5.2; no shipped file
exercises that branch, so they pin our reading of the rule rather than observed
game behaviour.

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-fmt-001 | code reading | `S2DNG.exe` | container check + decode driver: header fields, order of checks, log messages |
| E-fmt-001 | code reading | | raw file read (whole file into memory) |
| E-fmt-001 | code reading | | base key, base-name extraction, extension special case, name mixing |
| E-fmt-001 | code reading | | key checksum |
| E-fmt-001 | code reading | | XOR pass 1 and pass 2 |
| E-fmt-001 | code reading | | generator seeding and step |
| E-fmt-001 | code reading | | decompression |
| E-fmt-001 | code reading | | checksum routine (table-driven CRC-32) |
| E-fmt-002 | data survey | | header fields of all 1700 wrapped files |
| E-fmt-003 | full decode | | 1700/1700 files pass all checks; type and SHA-256 per file |

## Open questions

- `.sav` uses the fixed key by inference only; confirm when a save file exists
  (part of Q-009).
- Where the game decides wrapped vs. plain (FMT-RC-1.2) was not traced.
- The seed repair branches (FMT-RC-5.2) are not exercised by any shipped file.
- The writer side (how the original tool chose LZSS matches) is unknown and not
  needed: any valid stream decodes. Byte-identical re-wrapping is not a goal.
- The editor (`S2DNGEditor.exe`) presumably contains the writer; not examined.
