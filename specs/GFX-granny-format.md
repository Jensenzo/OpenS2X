# GFX-GR2: Granny model and animation files (.gr2)

Status: draft (container, compression, type system and the content used by the game established and checked on all 884 files; GFX-GR2-2 rewritten as a complete algorithm with test vectors (Q-122); rendering semantics not specified)
Owner question(s): plan phase 6 (graphics), spike
Game build: 11757

## Verdict

**Fully decodable without granny2.dll.** All 884 `.gr2` files of the game use one
container variant (Granny file format 6, 32-bit little endian) and one
compression method (Oodle1) besides plain storage. Oodle1 was decoded with an
independent decoder, and every file decodes to its stated sizes and parses
completely: 3903 meshes, 856,538 vertices, 1207 bones, 376 animations. What is
left is not decoding but **behaviour**: evaluating the animation curves and
skinning exactly as Granny does, and the game's own transform to its world axes
(GFX-GR2-7). Loading the user's granny2.dll is not needed; it remains a
Windows-only fallback (GFX-GR2-8).

## Summary

A `.gr2` file is a small header followed by 8 sections of data. Most
sections are Oodle1-compressed. After decompression, pointers inside the data
are restored from fixup tables. The data is self-describing: the file contains
type definitions (member name, member type, sub-type) for its root object, so a
generic reader can walk it. The game's files hold Granny's standard structures:
art tool info, textures (file name references), materials, skeletons (bones
with names, parent index, rest transform), meshes (vertex arrays with named
components, triangle index lists, material groups, bone bindings), models
and animations (per-bone tracks of B-spline curves).

## Rules

All integers little endian. "Pointer" = a 32-bit slot in a section that, after
fixups, refers to a (section, offset) location; a slot without a fixup is null.

### GFX-GR2-1 Header

- **GFX-GR2-1.1** Bytes 0..15: magic `B8 67 B0 CA F8 6D B1 0F 84 72 8C 7E 5E 19 00 1E`
  (32-bit pointers, little endian; Granny also defines 64-bit and big-endian
  magics, none used). Then u32 header size (`0x1B8` in all files), u32 header
  format (0), 8 reserved bytes.
- **GFX-GR2-1.2** At offset 32 the file info: s32 format version (6 in all
  files), u32 total file size, u32 CRC, u32 offset of the section table
  (relative to offset 32), u32 section count (8 in all files), root type
  definition reference (u32 section, u32 offset), root object reference (u32
  section, u32 offset), u32 type tag (`0x80000013` in all files; Granny checks it
  against its run-time tag and converts when it differs), 4 x u32 extra tags.
- **GFX-GR2-1.3** Section table: per section 11 x u32: compression, data offset
  (in file), stored size, expanded size, alignment, first-16-bit offset,
  first-8-bit offset, fixup table offset (in file), fixup count, marshalling
  table offset, marshalling count.
- **GFX-GR2-1.4** Compression values: 0 none, 1 Oodle0, 2 Oodle1, 3 and 4
  BitKnit. **In the game: 6188 sections Oodle1, 884 none** (section 5 in every
  file; it holds data in 124 files). No Oodle0 or BitKnit.
- **GFX-GR2-1.5** Fixup table (stored uncompressed, 12 bytes per entry): u32
  source offset in this section, u32 target section, u32 target offset. Each
  entry makes the 32-bit slot at the source offset a pointer to the target.
- **GFX-GR2-1.6** Marshalling table (stored uncompressed, 16 bytes per entry):
  u32 element count, u32 offset in this section, type reference (u32 section,
  u32 offset). An entry marks an array whose elements mix member sizes, so that
  a loader converting byte order knows how to swap it. 262 files have exactly
  one entry, always in section 3 at offset 0, and it describes the skinned
  vertex array there (type with float members and the 4-byte `BoneWeights` /
  `BoneIndices` members; count = vertex count). All other tables are empty.
  A 32-bit little-endian reader does not need the table and can ignore it
  (Q-123). Example: `buildings/PB_R_Sawmill01.GR2` section 3: count 4293,
  offset 0, type at section 6 offset 12320 (the `House` mesh vertices).

### GFX-GR2-2 Oodle1 decompression

Notation: all quantities are unsigned 32-bit integers; `a div b` is integer
division rounding down. "Entry" = one symbol slot of a model. Test vectors for
everything in this section are T9 to T12. Checked against all 3134 non-empty
Oodle1 sections (E-gfx-002): the procedure below reproduces every expanded
section byte for byte.

- **GFX-GR2-2.1 Section layout.** A compressed section with stored size 0 has
  expanded size 0 (3054 sections) and needs no decoding. Otherwise the stored
  bytes are: three 12-byte parameter blocks (bytes 0..35), then the coded
  stream starting at byte 36. The output is produced in three parts that fill
  the output in order: part 0 up to offset `E0` = the section's
  first-16-bit offset, part 1 up to `E1` = the first-8-bit offset, part 2 up
  to `E2` = the expanded size (GFX-GR2-1.3). Part `i` always uses parameter
  block `i`, also when an earlier part is empty.
- **GFX-GR2-2.2 Parameter block.** u32 `a`, u32 `b`, then 4 bytes `c0 c1 c2 c3`.
  Literal limit `L` = bits 0..8 of `a`; distance limit `R` = bits 9..31 of `a`;
  literal capacity = bits 0..8 of `b`; high capacity = bits 19..31 of `b`
  (bits 9..18 of `b` are 0 in every block and unused); `c0..c3` are the size
  capacities. In every block of the game's files `L` = 256 and `R` = 262143.
- **GFX-GR2-2.3 Arithmetic decoder.** State: a value `N`, a range `D`, and a
  byte position `p` in the stored section. Reading a byte at or past the stored
  size gives 0.
  1. Start (once per section, before part 0): `p` = 36, `N` = byte[36] shifted
     right by 1, `D` = 0x80. `p` is not advanced by the start.
  2. Refill: as long as `D` is at most 0x800000: `N` becomes `N x 256` plus a
     byte made of bit 0 of byte[p] (as its bit 7) and bits 1..7 of
     byte[p + 1] (as its bits 0..6); `p` += 1; `D` = `D x 256`. (The stream is
     read in 8-bit units offset by one bit.)
  3. Peek with bound `m` (`m` at least 1): refill, then `k` = `D div m`, and the
     peeked number `x` = the smaller of `N div k` and `m - 1`.
  4. Consume an interval `[v, v + w)` of a bound `m`, using the `k` of the peek
     just made: `N` -= `k x v`; then if `v + w < m`, `D` = `k x w`, otherwise
     (the interval reaches the bound) `D` -= `k x v`.
  5. A uniform number below `m` = peek with bound `m`, then consume `[x, x + 1)`.
     With `m` = 1 this yields 0 and changes nothing but the refill.
  At the end of a section `p` is 1 to 3 below the stored size (1: 1156, 2:
  1966, 3: 12 sections).
- **GFX-GR2-2.4 Model state.** A model is created with a limit `M` and a
  capacity `C` (table in 2.8). It holds:
  - an ordered list of entries, each with a weight and a value; entry 0 is the
    escape and has no value;
  - the total weight `T` (sum of all weights, the escape included);
  - a cumulative table of `t + 1` boundaries `s0 = 0 <= s1 <= ... <= st` with
    `st` = 0x4000, covering the first `t` entries ("listed" entries). Entries
    added after the last table recompute are "unlisted" and cannot be hit
    directly;
  - an increment `I`, an increment cap `Icap`, a table threshold `Tt` and a
    halving threshold `Th`.
  Initial state: one entry (the escape, weight 4); `T` = 4; table `[0, 0x4000]`
  (`t` = 1, only the escape listed); `I` = 4; `Tt` = 8;
  `Th` = max(256, min(32 x M, 15160)); `Icap` = min(2 x M, `Th` div 2 - 32) if
  `M` > 64, else 128. Example: `M` = 64 gives `Th` 2048, `Icap` 128; `M` = 255
  gives `Th` 8160, `Icap` 510; `M` = 3 gives `Th` 256, `Icap` 128.
  The list never holds more than `C + 1` entries (escape included).
- **GFX-GR2-2.5 Decoding one symbol** from a model, with a bound `B` for new
  values supplied by the caller (2.8, 2.9). In this order:
  1. Upkeep: if `T >= Tt`: first, if `Tt >= Th` (compared before anything
     changes), halve the weights (2.7); then, in either case, recompute the
     table (2.6). Nothing else triggers either step.
  2. Peek with bound 0x4000. The hit entry is the listed entry `j` with the
     largest index such that `s_j <= x`. (An entry whose interval has width 0,
     possible for the escape, is never hit in the data.) Consume
     `[s_j, s_(j+1))` of bound 0x4000. Add 1 to the weight of entry `j` and to
     `T`. This applies to the escape as well, even when its weight was 0.
  3. If `j >= 1`: the result is the value of entry `j`. Done.
  4. Escape (`j` = 0). Let `u` = number of unlisted entries (list length minus
     `t`). If `u > 0`, decode a uniform number below 2. If it is 1, decode a
     uniform number `q` below `u`; the chosen entry is the unlisted entry `q`
     places after the last listed one (list index `t + q`). Add 2 to its weight
     and to `T`; the result is its value. Done.
  5. New entry (`u` = 0, or the number in step 4 was 0): append an entry with
     weight 2 at the end of the list; `T` += 2. If the list now holds `C + 1`
     entries, subtract the escape's weight from `T` and set it to 0. Then the
     entry's value is a uniform number below `B`; the result is that value. The
     new entry stays unlisted until the next table recompute.
  Facts over all files: step 5 is never reached with a full list; a new value
  never equals a value already in the model; when `B` = 1 the new value is 0
  and takes nothing from the stream (high model 3965 times, mid models 1059
  times).
- **GFX-GR2-2.6 Table recompute.** Let `f` = 0x20000 div `T`. Over all current
  entries in list order (the escape first): `s0` = 0 and
  `s_(i+1)` = `s_i` + (`weight_i x f`) div 8; the boundary after the last entry
  is set to 0x4000 instead of the computed sum (the last entry takes the
  rounding remainder). All current entries are listed afterwards (`t` = list
  length). Then the thresholds: if `I > Icap div 2`, `Tt` = `T + Icap`
  (`I` unchanged); otherwise `I` = 2 x `I` and `Tt` = `T + I`.
  Example (T11): weights `[6, 2, 2]`, `T` 10, `I` 4: `f` = 13107, table
  `[0, 9830, 13106, 0x4000]`, `I` becomes 8, `Tt` = 18.
- **GFX-GR2-2.7 Weight halving** (only from 2.5 step 1, always followed by
  2.6):
  1. Every weight, the escape's included, becomes `weight div 2`; `T` becomes
     the new sum.
  2. Remove entries of weight 0 except the escape: go through list positions
     from 1 upward; while the entry at the current position has weight 0 and
     the position is still inside the list, move the last entry into this
     position (shortening the list by one) and look at the same position
     again; then go to the next position.
  3. Among the entries from position 1 on, find the one with the largest
     weight, the lowest position on a tie, and swap it with the last entry.
  4. If the list holds fewer than `C + 1` entries and the escape's weight is
     0, set the escape's weight to 1 and add 1 to `T`.
  Example (T12): capacity 4, values `[esc, 0, 3, 1]` with weights
  `[10, 33, 208, 7]`, `T` 258, `Tt` 258 >= `Th` 256: after halving values
  `[esc, 0, 1, 3]`, weights `[5, 16, 3, 104]`, `T` 128; table
  `[0, 640, 2688, 3072, 0x4000]`; `I` stays 128, `Tt` = 256.
- **GFX-GR2-2.8 Models of one part.** Created fresh at the start of every
  non-empty part from that part's parameter block. With
  `lowLimit` = min(R + 1, 4), `highCount` = R div 1024 + 1 and
  `midLimit` = min(R div 4 + 1, 256) (with the game's `R`: 4, 256, 256):

  | Model | Count | Limit `M` | Capacity `C` | Bound `B` for new values |
  |---|---|---|---|---|
  | size `k`, `k` = 0..15 | 16 | 64 | `c3` | 65 |
  | size `k`, `k` = 16..31 | 16 | 64 | `c2` | 65 |
  | size `k`, `k` = 32..47 | 16 | 64 | `c1` | 65 |
  | size `k`, `k` = 48..63, and size 64 | 17 | 64 | `c0` | 65 |
  | literal `j`, `j` = 0..3 | 4 | `L - 1` | literal capacity | `L` |
  | low | 1 | `lowLimit - 1` | `lowLimit` | `lowLimit` |
  | high | 1 | `highCount - 1` | high capacity + 1 | `r div 1024 + 1` |
  | mid `h`, `h` = 0..highCount-1 | `highCount` | `midLimit - 1` | `midLimit` | min(`r div 4 + 1`, 256) |

  `r` = min(`R`, `produced`) at the time of the token (2.9): the high and mid
  bounds grow with the output of the current part, while their thresholds
  (from `M`) are fixed. The low bound does not depend on `produced`.
- **GFX-GR2-2.9 Decoding a part.** Set `prev` = 0 and `produced` = 0. While the
  output position `pos` is below the part's end:
  1. Decode a symbol `z` (0..64) from size model number `prev`; set `prev` = `z`.
  2. If `z` = 0 (literal): decode a symbol from literal model `pos mod 4`
     (`pos` counted from the start of the section); write it as the byte at
     `pos`; `pos` and `produced` += 1.
  3. Otherwise (copy): length `n` = `z + 1` for `z` up to 60; 128, 192, 256, 512
     for `z` = 61, 62, 63, 64. With `r` = min(`R`, `produced`), decode in this
     order: `l` from the low model, `h` from the high model, `md` from mid
     model number `h`. Distance `d` = `h x 1024 + md x 4 + l + 1`. Copy `n`
     bytes one at a time, each from `d` bytes before the byte being written (an
     overlap repeats recent bytes). `pos` and `produced` += `n`.
- **GFX-GR2-2.10 Part boundaries.** An empty part (end not above `pos`)
  creates nothing and reads nothing. At the start of each non-empty part,
  all models, `prev` and `produced` start afresh; the decoder state (`N`,
  `D`, `p`) and `pos` carry on. Facts over all files: no copy reaches before
  the start of its own part, no copy goes past the end of its part (so a
  part always ends exactly at its boundary), `d` never exceeds `produced`,
  and every part boundary is a multiple of 4. Part layouts: parts 0 and 2
  (884 sections: section 0 of every file, part 1 empty), part 0 only (1626),
  part 1 only (358), part 2 only (262), parts 0 and 1 (4). **No section has
  three non-empty parts.**
- **GFX-GR2-2.11 Check.** The decoder fills exactly the expanded size and ends
  as in 2.3; T1 and T9 to T12 must pass.

### GFX-GR2-3 Type system

- **GFX-GR2-3.1** A type definition is an array of 32-byte member records,
  ended by a record of type 0: u32 member type, pointer to name (C string),
  pointer to sub-type definition, s32 array count (0 = single value), 16 bytes
  unused.
- **GFX-GR2-3.2** Member types and their sizes in the object: 1 inline struct
  (size of the sub-type), 2 pointer to one object (4), 3 count + pointer to an
  array of objects (8), 4 count + pointer to an array of pointers (8), 5 variant:
  pointer to type + pointer to data (8), 7 variant array: pointer to type, count,
  pointer to data (12), 8 string pointer (4), 9 transform (68: u32 flags,
  position 3 floats, orientation quaternion 4 floats, scale/shear 3x3 floats), 10
  float32, 11 int8, 12 uint8, 13 normalised int8, 14 normalised uint8, 15 int16,
  16 uint16, 17 normalised int16, 18 normalised uint16, 19 int32, 20 uint32, 21
  float16, 22 empty reference (4). An array count repeats the member.
- **GFX-GR2-3.3** The root object is read with the root type definition at the
  root reference. A reader needs no built-in knowledge of the structures; field
  names below are those stored in the files.

### GFX-GR2-4 Content used by the game

- **GFX-GR2-4.1** Root members include `ArtToolInfo` (3D Studio MAX;
  `UnitsPerMeter` 1.0 for most files, 39.37 for some; origin and axis vectors),
  `ExporterInfo`, `FromFileName`, `Textures`, `Materials`, `Skeletons`,
  `VertexDatas`, `TriTopologies`, `Meshes`, `Models`, `TrackGroups`,
  `Animations`, `ExtendedData`.
- **GFX-GR2-4.2 Meshes**: `Name`, `PrimaryVertexData` (a variant array: the
  vertex layout is the variant's type, e.g. `Position` 3 floats, `BoneWeights` 4
  uint8, `BoneIndices` 4 uint8, `Normal`, `Tangent`, `DiffuseColor0`,
  `TextureCoordinates0..2`), `PrimaryTopology` (`Groups`: material index, first
  triangle, triangle count; `Indices16` or `Indices`), `MaterialBindings`,
  `BoneBindings` (bone name, bounding box, triangle list). Vertex layouts found
  (count of meshes):

  | Meshes | Components |
  |---|---|
  | 2785 | Position, Normal, DiffuseColor0, TextureCoordinates0 |
  | 710 | Position, Normal, TextureCoordinates0 |
  | 76 | Position, BoneWeights, BoneIndices, Normal, Tangent, TextureCoordinates0..2 |
  | 76 | Position, Normal, Tangent, TextureCoordinates0..2 |
  | 70 | Position, Normal, Tangent, TextureCoordinates0..1 |
  | 69 | Position, BoneWeights, BoneIndices, Normal, DiffuseColor0, TextureCoordinates0 |
  | 63 | Position, BoneWeights, BoneIndices, Normal, Tangent, TextureCoordinates0..1 |
  | 54 | Position, BoneWeights, BoneIndices, Normal, TextureCoordinates0 |

  Skinned vertices have 1 (584,416), 2 (35,965), 3 (5078) or 4 (213) non-zero
  weights. The most bone bindings in one mesh is 51 (the Roman smelter).
- **GFX-GR2-4.3 Skeletons**: `Name`, `Bones` (`Name`, `ParentIndex`, rest
  `Transform`, `InverseWorldTransform` 4x4). Buildings have skeletons for their
  moving parts (sawmill: `root`, `blade`, `reel1..5`, ...); settlers use a biped
  (`Bip01`, 44 bones in the forester mesh's bindings; donkey 39 bones).
- **GFX-GR2-4.4 Animations**: `Name` (the source .max path), `Duration` (s),
  `TimeStep` (s, 1/24 for settlers, 1/50 for buildings), `TrackGroups` with
  `TransformTracks` per bone: `PositionCurve`, `OrientationCurve`,
  `ScaleShearCurve`, each `{Degree, Knots[], Controls[]}` (old-style Granny
  curves, not the newer compressed curve formats). Degree 0 (constant or held
  keys), 1, or 2 (quadratic B-spline); controls are 3, 4 and 9 floats per knot
  for position, orientation and scale/shear; an empty curve means the rest pose
  value. Counts over all files: orientation degree 2: 2851, position degree 2:
  383, scale/shear degree 1: 3, the rest degree 0 or empty.
- **GFX-GR2-4.5 File roles** (by folder): `buildings/` building meshes per tribe
  (`PB_<tribe>_<name>.GR2`, `...ConstructionSite.GR2`, `...LOD0.GR2`) and per
  building animations in a subfolder (`working`, `door_open`, `door_close`);
  `character/meshes/` settler meshes with LODs; `character/animations/` settler
  animations (`settler_idle_01`, `bulldozer_shovel`, ...); `character/donkey/`;
  `map/objects/` trees, stones, doodads; `animals/`, `ships/`, `items/` (wares).
  Which animation plays when is in `character/settler_animations.xml` and
  `settler_config.xml` (not specified here).

- **GFX-GR2-4.6 Files named by nothing (2026-10-24).** These model and animation files are
  named by no table, model, script or string of the executable and are taken as unused:
  `character/donkey/donkey_walk_boat.GR2`, `character/animations/settler_fishing.GR2`,
  `settler_idle_11x.gr2`, `settler_idle_bow.gr2`, `settler_idle_sneeze.gr2`,
  `character/meshes/nubian_soldier_mesh.GR2`, `soldier_mesh.GR2`, the five ship clips
  `ships/PV_R_Ship01Blow.GR2`, `PV_R_Ship01BlowIn.GR2`, `PV_R_Ship01BlowOut.GR2`,
  `PV_R_Ship01FoldIn.GR2`, `PV_R_Ship01FoldOut.GR2`, `buildings/PB_R_Locksmith01Colortest.GR2`,
  `Ruin_Small.GR2`, `holzfaellerhuette.GR2`, `PB_R_Smithy01/door2_open.GR2`,
  `door2_close.GR2`, `map/objects/RS_TreeBroadLeaf03LOD0.GR2`, `RS_Wheatfield01.GR2`,
  `character/meshes/soldier2_mesh.GR2` (the characters name the tribe files
  `roman_soldier2_mesh` etc.), the `dummy_working.GR2` files of `PB_R_Barracks01`,
  `PB_R_Farm01`, `PB_R_ForesterHut01`, `PB_R_Spytower01`, `PB_R_Well01`, `PB_R_Fortress01`,
  `PB_R_Watchtower01`, `PB_R_Warehouse01`, `PB_R_Catapult01`, `PB_R_HunterHut01`,
  `PB_R_WoodcutterHut01` and `PB_R_Guardhouse01` (no `Working` entry names them), and the
  textures `map/transitions/hardTransitionTest.tga`, `map/objects/RS_TreeBirch02.dds`,
  `RS_TreeBirch03.dds`, `RS_TreeBroadLeaf02.dds`, `RS_TreeBroadLeaf03.dds` (their tree
  meshes name other textures) and `buildings/PB_R_ForesterHut01ColorIndex.tga` (the
  `.dds` of that name is the one used, GFX-OBJ-1.1). Confidence: medium (a search of
  the data and the executable's strings; a file-open trace would confirm it, Q-348).
  **File-open trace (2026-10-24, TOOL-trace TRACE-7.5):** none of these 38 files was read
  by the original in the sessions of 2026-10-13 to 2026-10-23 (menus, the first start
  that converted every model it showed, and free games on MP_2P_Storm_Coast in which every
  Roman settler, building and ship of the free game appeared). (Corrected the same day:
  `character/donkey/donkey_mesh_LOD1.GR2`, `character/donkey/donkey_mesh_LOD2.GR2` and
  `character/donkey/donkey_mesh_LOD3.GR2` were on this list, but
  `settler_config.xml` names them as the donkey's `mesh1` to `mesh3`, spelled `LoD`, and
  the original read them at its first start.) Confidence: high that they are not used in
  a free game with Romans; medium for the campaign and tutorial, which those sessions
  did not play.

### GFX-GR2-5 Textures

- **GFX-GR2-5.1** Textures are referenced by file name (`FromFileName`, an
  artist path such as `Z:\S2C\Development\Data\buildings\PB_R_Sawmill01.dds`).
  Taking the base name and finding it in the game data finds all 737 `.dds`
  references and 59 of 79 `.tga`. The `.bmp`, `.tif`, `.psd` and remaining
  `.tga` names (character art) do not exist; characters get their textures
  from `character/settler_config.xml` instead (GFX-GR2-5.3).
  Class rule (checked 2026-10-24 over all 884 `.gr2` files: 244 of the textures not
  named in any spec rule are named by a model): a texture of these folders is drawn as
  the texture its models name. Named by nothing (no table, model, script or string of the executable) and taken as
  unused (medium): `buildings/Holzfaellerhaus.tga`, `PB_R_Headquarter01_L.tga`,
  `PB_R_Stone01.tga`, `marble1.tga`, `animals/Wolf.dds`, `items/GO_Warez01.dds`,
  `map/objects/RS_TreeBroadLeaf04.dds`, `ships/flagTexture.dds`. Textures named only by a
  settings file (the forester hut's colour index, four tree textures of the landscape
  elements) are not covered by this rule. Confidence: medium (the base-name lookup is inferred from
  every reference resolving; not traced).
  **Which copy (answered 2026-11-06, Q-1017; E-gfx-039).** The texture's path is the
  **model file's own folder** plus the base name; no other folder is searched. So a name that
  exists in two folders is always the copy beside the model: the rock models of `map/objects`
  take `map/objects/l_ground_00.dds`, `l_rock_00_n.dds`, `m_rock_01.dds`, `z_rock_00.dds` (the
  `map/textures` copies are the terrain patterns' own, GFX-TERR), the donkey model of
  `character/donkey` takes its own `EselTextue.dds` and an `animals` model the `animals` one. The
  file-open trace agrees (`FILE-TRACE.md`: `map/objects/z_rock_00.dds` and both `EselTextue.dds`
  read). Over all 884 models, 793 texture references resolve in the model's folder, 3 only
  elsewhere (`character/animations/settler_fishing` names `go_warez02.dds`, `LO_Gate_water`
  names `gate_water0000.dds`, `RS_Wheatfield01` names `house_decal.dds`, which exists only in `buildings`; the first is an
  animation file and the gate water frames are loaded by name from `map/textures` (GFX-OBJ-1.2a);
  and `RS_Wheatfield01.GR2` is an unused leftover: no landscape element, script or code names it
  and the original never reads it (`FILE-TRACE.md`); the wheat field is the element `Wheatfield`,
  whose mesh `LO_Wheat.GR2` names only `RS_Wheat02.dds` in its own folder (answered 2026-11-07,
  Q-1029; `settings/terrain_static_data.xml`), so no model the game draws takes `house_decal.dds`
  from another folder), 154 nowhere (the
  character art of the rule above). Confidence: high for buildings (the material builder joins
  the model folder and the name, code); medium-high for the other model kinds (their loader was
  not read; the trace and the survey agree). Covers (inventory): *texture* from
  `data/buildings/*`, `data/ships/*`, `data/animals/*`, `data/items/*`,
  `data/map/objects/*`, `data/character/donkey/*` except
  `data/buildings/holzfaellerhaus.tga`, `data/buildings/pb_r_headquarter01_l.tga`,
  `data/buildings/pb_r_stone01.tga`, `data/buildings/marble1.tga`,
  `data/buildings/pb_r_foresterhut01colorindex.*`, `data/animals/wolf.dds`,
  `data/items/go_warez01.dds`, `data/map/objects/rs_treebroadleaf04.dds`,
  `data/map/objects/rs_treebirch02.dds`, `data/map/objects/rs_treebirch03.dds`,
  `data/map/objects/rs_treebroadleaf02.dds`, `data/map/objects/rs_treebroadleaf03.dds`,
  `data/ships/flagtexture.dds`.
- **GFX-GR2-5.2** 159 texture records carry embedded images (encoding 3,
  256x256) of only about 208 bytes, evidently stubs; the game does not need them.
- **GFX-GR2-5.3** Character colouring: each character has a colour-index
  texture (`color = ...colorindices_luminance.tga`); its channels give a slot
  index and a luminance, looked up in the palette tables of
  `character/colormaps/*_table.tga` by slot (`character/color_slots.xml`: hair,
  jacket, skin, trousers, player colour, ...). This is done in the shader
  `skin_color_mesh.fx`.
  Class rule (2026-10-24): the colour tables named by `character/color_slots.xml` and the
  colour-index textures named by `character/settler_config.xml`; `color_map_0.tga` to `color_map_6.tga`
  (`color_map_1`, `color_map_2`, `color_map_3`, `color_map_4`, `color_map_5`, `color_map_6`),
  `settler_colorindices.tga` and `soldier_specular.tga` are named by nothing (no table,
  model, script or string of the executable) and are taken as unused. Confidence: medium.
  Covers (inventory): *texture* from `data/character/colormaps/*`,
  `data/character/meshes/*colorindices*` except
  `data/character/colormaps/color_map_?.tga`,
  `data/character/meshes/settler_colorindices.tga`.
- **GFX-GR2-5.4** DDS files in the data (503): DXT1 372, DXT5 68, DXT3 33,
  uncompressed 32-bit with alpha 6, 16-bit with alpha 16, 16-bit 7, 8-bit 1; one
  cube map; 350 with mip levels; sizes up to 1024x1024. One `.dds` file is not a
  DDS. Buildings have a colour map and an `_L` map (`PB_R_Sawmill01_L.dds`), the
  house light map bound through the material's `Self-Illumination` usage
  (`GFX-objects.md` GFX-OBJ-2.1; Q-049 closed 2026-11-08 b), and a shared
  `house_decal.dds`. `DiffuseColor0` is read where an object's shader rule says so
  (trees: light x vertex colour, GFX-OBJ tree shader); houses ignore it (GFX-OBJ-2.1).

### GFX-GR2-6 Shaders

- **GFX-GR2-6.1** The game ships its shaders as Direct3D 9 effect **source**
  (HLSL, `data/gfx/fx_20/*.fx` plus `shared.inc`, 33 files), shader model 2.0
  (`vs_2_0`/`ps_2_0`, a few `vs_1_1`): terrain, water, houses and decals,
  trees, doodads, stones, animals, donkey, ships, items, flags, streets,
  border, particles, shadow, post-processing, UI. A port must re-express them
  (behaviour only; the files are game data and stay out of the repository).
- **GFX-GR2-6.2** Skinning: a palette of at most 50 bone matrices per draw;
  the character shader blends 2 bones per vertex. Data has meshes with up to
  51 bone bindings and up to 4 weights per vertex, so the game either splits
  such meshes or uses only part of the bindings (not established).

### GFX-GR2-7 How the game uses Granny

- **GFX-GR2-7.1** `S2DNG.exe` imports 35 functions from `granny2.dll` 2.5.0.11:
  file loading (`ReadEntireFileFromMemory`, `GetFileInfo`, `FreeFile`),
  coordinate conversion (`ComputeBasisConversion`, `TransformFile`), mesh access
  (`GetMeshVertexCount`, `GetMeshIndexCount`, `GetMeshIndices`,
  `GetMeshBytesPerIndex`, `GetMeshVertices`, `GetMeshVertexType`,
  `GetMemberArrayWidth`, `GetMemberUnitSize`), and the animation run time
  (`InstantiateModel`, `FreeModelInstance`, `PlayControlledAnimation`,
  `SetControl{Active, LoopCount, Clock, Weight, Speed}`,
  `GetControlDurationLeft`, `FreeControl`, `SetModelClock`,
  `SampleModelAnimations`, `NewLocalPose`, `NewWorldPose`, `BuildWorldPose`,
  `GetWorldPose4x4Array`, `GetWorldPoseComposite4x4Array`,
  `GetLocalPoseTransform`, `GetSourceSkeleton`, `FindBoneByName`, `Free*Pose`).
  So the game reads the files from memory (after removing the rc00 container,
  `FMT-rc00-container.md`), converts them to its own axes and units, builds its
  own vertex buffers, and uses Granny to sample and blend animations into bone
  matrices.
- **GFX-GR2-7.2** A reimplementation therefore needs, besides decoding:
  (a) the basis conversion the game asks for: origin 0, right +z, up +y, back +x,
  units per metre = the kind's scale setting (`GFX-world-view.md` GFX-VIEW-1.2, 1.3),
  (b) curve evaluation (GFX-GR2-7.5), (c) local-to-world pose building with the
  rest transforms and `InverseWorldTransform`, (d) control timing (GFX-GR2-7.6).
- **GFX-GR2-7.5 Curve evaluation** (Q-179; E-gfx-008, read from the shipped
  `granny2.dll`). A curve is `{Degree d, Knots K[0..n-1], Controls P[0..n-1]}`
  with one control (of the track's dimension: 3 position, 4 orientation, 9
  scale-shear) per knot. In the game's data every multi-knot curve has K[0] = 0
  and K[n-1] = the clip's `Duration`; a curve with n = 1 is a constant. For a
  sample time t in [0, K[n-1]]:
  1. Knot index k = max(1, number of knots strictly less than t). (For t = K[j]
     exactly, k = j; for K[j-1] < t < K[j], k = j; t is never above K[n-1], see 7.6.)
  2. Window: the 2d knots and controls at indices k - d .. k + d - 1. Indices
     outside 0 .. n-1 are filled by clamping: an index below 0 repeats K[0] / P[0],
     an index at or above n repeats K[n-1] / P[n-1], **unless the clip loops**
     (7.6): then an index i below 0 takes K[i + n - 1] - K[n-1] with P[i + n - 1],
     and an index i at or above n - 1 takes K[n-1] + K[i - (n - 1)] with
     P[i - (n - 1)] (the last knot of one period is the first of the next).
  3. Blend (knots relative to k: t0 = K[k-2], t1 = K[k-1], t2 = K[k], t3 = K[k+1]
     from the window):
     - d = 0: value = P[k].
     - d = 1: u = (t - t1) / (t2 - t1); value = u P[k] + (1 - u) P[k-1].
     - d = 2: a = (t - t1) / (t2 - t1); b = (t - t0) / (t2 - t0); c = a + b - a b;
       e = a (t - t1) / (t3 - t1); value = e P[k] + (c - e) P[k-1] + (1 - c) P[k-2]
       (a quadratic B-spline on the stored knot vector).
     - d = 3 exists in the library but no game curve uses it.
  4. Orientation curves (dimension 4): the controls are blended component-wise as
     stored (the exporter stores them with consistent signs; no per-sample shortest
     arc test), except that controls copied into a clamped or wrapped window are
     negated when their dot product with the previous window control is negative.
     After the blend the quaternion is scaled by (3 - |q|^2) / 2 (the library's
     first-order renormalisation; an exact normalisation differs by less than the
     float precision for the data's near-unit results and is acceptable).
  5. The transform `Flags` are not read by the sampler; a track without a curve
     for a component contributes identity (0 position, unit quaternion, unit
     scale), and bones without a track keep their rest transform.
  Confidence: high (the sampler, knot search and window construction were read
  instruction by instruction; the knot rule was checked with a port against random
  knot vectors).
- **GFX-GR2-7.6 Control clock.** A playing clip has a local clock that advances
  by frame time x speed (the game's speed is `globalSpeed`, GFX-ANIM-2.1). A clip
  with loop count 0 (forever) or loops left wraps: whenever the clock reaches
  `Duration` it subtracts `Duration` (as often as needed) and counts one loop. A
  clip on its last (or only) pass keeps counting past `Duration` ("duration left"
  becomes negative, GFX-ANIM-3.3), but samples at the clock clamped to
  [0, Duration]; at t = Duration the window of 7.5 repeats the last knot and the
  pose is the last control. While a clip can still wrap (loop count 0, or not on
  its last pass) the knot window near the ends wraps into the neighbouring period
  (7.5 step 2), so a looping clip is continuous across its loop point; on the last
  pass the ends are clamped. Confidence: high (E-gfx-008).

- **GFX-GR2-7.3** Vertex buffers (Q-124; E-gfx-002). The game
  copies each mesh's vertices member by member, matching members by name:
  `Position` (3 floats, unchanged); `Normal` and `Tangent` (3 floats, scaled to
  unit length); `DiffuseColor0` (converted to a packed colour; see Q-049);
  `BoneWeights` (4 bytes become 4 floats: the first three bytes divided by 255,
  the fourth = 1 minus the first three; the fourth stored byte is not used);
  `BoneIndices` (4 bytes, unchanged); `TextureCoordinates0..3` (2 floats each,
  unchanged). Other members are ignored. Confidence: high (copy routine read
  directly).
- **GFX-GR2-7.4** No V flip. Texture coordinates reach the shaders as stored,
  and the shaders (`fx_20/house_shader.fx` and others) use them unchanged
  except for adding half of a per-texture offset parameter (the D3D9 half-texel
  convention). D3D9 samples with v = 0 at the first row stored in the DDS file.
  A renderer with the same origin (Direct3D, Vulkan, Metal, wgpu: v = 0 at the
  first uploaded row) must use V unchanged when it uploads DDS rows in file
  order; flipping V is right only for a bottom-left origin. This is separate
  from the basis conversion of Q-047, which concerns positions, normals and
  bone transforms. Confidence: high for the game's handling, medium that
  Granny's basis conversion leaves texture coordinates alone (not traced
  inside granny2.dll).

### GFX-GR2-8 Fallback: the user's granny2.dll

- **GFX-GR2-8.1** Not needed for decoding. If ever used (Windows only, 32-bit
  process, the user's own installed copy), the minimum is
  `GrannyReadEntireFileFromMemory` (or `GrannyReadEntireFile`),
  `GrannyGetFileInfo`, `GrannyFreeFile`; `GrannyDecompressData` alone (exported)
  would replace GFX-GR2-2; `GrannyConvertFileToRaw` writes an uncompressed copy.
  For animation, the run-time functions of GFX-GR2-7.1. A 32-bit DLL cannot be
  loaded into a 64-bit process, so a helper process would be required.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| magic | B867B0CA F86DB10F 84728C7E 5E19001E | | high | gr2_survey (all 884 files) |
| file format version | 6 | | high | gr2_survey |
| sections per file | 8 | | high | gr2_survey |
| compression used | Oodle1 (6188), none (884) | sections | high | gr2_survey |
| header size / type tag | 0x1B8 / 0x80000013 | | high | gr2_survey |
| fixup entry | 12 bytes | | high | gr2_check (all files decode) |
| marshalling entry | 16 bytes | | high | E-gfx-002 (262 files) |
| Oodle1 parameter blocks | 3 x 12 bytes, stream from byte 36 | | high | E-gfx-002 |
| Oodle1 symbol table total | 0x4000 | | high | E-gfx-002 (spec-only decoder matches all sections) |
| Oodle1 model start: escape weight, increment, table threshold | 4, 4, 8 | | high | E-gfx-002 |
| Oodle1 halving threshold | max(256, min(32 x M, 15160)) | weight | high | E-gfx-002 |
| Oodle1 increment cap | min(2 x M, threshold / 2 - 32) if M > 64, else 128 | weight | high | E-gfx-002 |
| `L` / `R` in the game's files | 256 / 262143 | | high | E-gfx-002 (every block) |
| copy lengths for size symbols 61..64 | 128, 192, 256, 512 | bytes | high | E-gfx-002 |
| member record | 32 bytes | | high | gr2_check |
| skin palette | 50 | matrices | high | `fx_20/shared.inc` |
| granny2.dll version | 2.5.0.11 | | high | DLL version resource, SHA-256 dffacb3d...ae29 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | all 884 `.gr2` in the data | decode every section | each fills exactly its expanded size; the Oodle1 decoder stops 1 to 3 bytes before the stored size |
| T9 | the sections below | decode (GFX-GR2-2) | SHA-256 of the stored bytes and of the expanded bytes as listed under "T9 vectors" |
| T10 | `buildings/PB_R_Sawmill01.GR2` section 0 | decode, log tokens | the tokens listed under "T10 tokens" |
| T11 | `buildings/PB_R_Sawmill01.GR2` section 2, part 0, literal model 0 | its first table recompute, at the start of its third symbol (the literal at output position 8) | before: values `[esc, 69, 70]`, weights `[6, 2, 2]`, `T` 10, table `[0, 0x4000]`, `I` 4, `Tt` 8; after: table `[0, 9830, 13106, 16384]`, `I` 8, `Tt` 18 (`Th` 8160, `Icap` 510, capacity 72) |
| T12 | `buildings/PB_R_Sawmill01.GR2` section 0, part 0, low model | its first halving | before: values `[esc, 0, 3, 1]`, weights `[10, 33, 208, 7]`, `T` 258, table `[0, 1260, 5040, 15624, 16384]`, `I` 128, `Tt` 258, `Th` 256, capacity 4; after: values `[esc, 0, 1, 3]`, weights `[5, 16, 3, 104]`, `T` 128, table `[0, 640, 2688, 3072, 16384]`, `I` 128, `Tt` 256 |
| T2 | `buildings/PB_R_Sawmill01.GR2` | read | meshes `GROUND` (72 vertices, 32 triangles) and `House` (4293 vertices, 2400 triangles, 33 bone bindings); skeleton `root` with 25 bones; textures `house_decal.dds`, `PB_R_Sawmill01_L.dds`, `PB_R_Sawmill01.dds` |
| T3 | `buildings/PB_R_Sawmill01/working.GR2` | read | one animation, duration 60 s, time step 0.02 s, 21 transform tracks |
| T4 | `character/meshes/asian_forester_mesh.GR2` | read | mesh with 1222 vertices (Position, BoneWeights, BoneIndices, Normal, DiffuseColor0, TextureCoordinates0), 1898 triangles in 2 groups, 44 bone bindings |
| T5 | `character/animations/settler_idle_01.gr2` | read | one animation, 3.333 s, time step 1/24 s, 44 tracks |
| T6 | `map/objects/RS_TreeBirch01.GR2` | read | mesh `RS_TreeBirch01`, 364 vertices, 246 triangles; texture `RS_TreeBirch05.dds` |
| T7 | `character/donkey/donkey_model.GR2` | read | skeleton `Bip01` with 39 bones, no mesh; `UnitsPerMeter` 39.37 |
| T8 | all files | totals | 3903 meshes, 856,538 vertices, 1207 bones, 376 animations; every index below its mesh's vertex count |

### T9 vectors

Parts = which of parts 0, 1, 2 are non-empty (GFX-GR2-2.10). No section in the
data has all three parts non-empty, so the vectors cover every layout that
occurs instead.

| File, section | Stored | Expanded | `E0` / `E1` | Parts | Stored SHA-256 | Expanded SHA-256 |
|---|---|---|---|---|---|---|
| `map/objects/LO_LavaFog01.GR2` s2 | 49 | 12 | 0 / 12 | 1 | 6ccd588c9c8f2c846683b282b0f19341aae6946fb85c1b45cb462a4b97974ac2 | b44387414aea2efef768a8b3ad17f3fd6f8cdb77e82e92ebba0a7b33f1d94d9a |
| `buildings/MilitaryFlag1.GR2` s2 | 632 | 2208 | 720 / 2208 | 0, 1 | 475587878a5875642b49863280531b89d6fae0daeb1c57ef4f05db21c29f56ee | 12587081ab69534993e1699aa946a7594238e51aa2f03f00c106f36a23f1484e |
| `buildings/PB_N_Well01.GR2` s4 | 4451 | 12936 | 12936 / 12936 | 0 | 947d8800c16cb48e7b64577014aeb97585d5e832344a3ca66e95596d3f543a31 | 1189490fb7c4f4fba0cfd7e0d1a9b4405f5d0896f106891f5221ded160994715 |
| `character/animations/work_well.GR2` s0 | 223919 | 381588 | 379036 / 379036 | 0, 2 | ddb91b14ff6810f59944719a63519d9eb453b416b71fac4bbd219f473eafe5b6 | 520bf8d3a86b5ba1a3351b94d8d3bd9624abe0a93e8deda98f2417d0c68afbce |
| `buildings/PB_R_Sawmill01.GR2` s0 | 3589 | 12616 | 8528 / 8528 | 0, 2 | 88cc43e947f1f7fb0e58f41b529d2f1cec18177f5a8ab1501fc8f514d6632ee0 | 59d4964555ae688e3d58c1023d79013f642a6fd0d88cdc5c20421d2ff95fdff1 |
| `buildings/PB_R_Sawmill01.GR2` s1 | 876 | 4320 | 4320 / 4320 | 0 | ac074ca76a2b8638ed8abf9ffca99f01deadb45b909b9505a58c29188aa6c3ec | c1059cce22b7b6d2d21d1815d438bba9af260c07bea289221757597cc3eec1b3 |
| `buildings/PB_R_Sawmill01.GR2` s2 | 211 | 384 | 384 / 384 | 0 | e1a361ff33b8feedcdfe82b0220c33de6aa14bac381bf23ed0bdab9451ca1d86 | 70bae7003762850ea391a77abc656a95146c763c4ff67ac405315b3b77595047 |
| `buildings/PB_R_Sawmill01.GR2` s3 | 105578 | 291936 | 0 / 0 | 2 | d84ea0e91a0b79695207299cfb467dfd61a9ac6e8a6a5df9b4f8b0d0a0266dd5 | 5701b1bcb4f068efc330cf8d1d3189ddae1f0a5b7408526d07372bff3b6f1ae6 |
| `buildings/PB_R_Sawmill01.GR2` s4 | 10102 | 28800 | 28800 / 28800 | 0 | 0088655f6c37057e1053941690e692ebec3ba03e6d32a4b93dd0c9df8c2aabbf | cb67b73bae2a7d47559503bb7dd2db34b10078413dff0b9a8a6ac86ffcf29276 |
| `buildings/PB_R_Sawmill01.GR2` s6 | 1359 | 18304 | 18304 / 18304 | 0 | b0c389ea128e449b223eaf20358d2bd39e298cfd3d80864eabb5ef56ef91b3c8 | 9accfad41d0bd0178c69de8ed3643e43a80ab95a3bbb5ba9e68e10ee5167a61b |

Sawmill s5 is stored uncompressed and s7 is Oodle1 with stored size 0; both
are empty (SHA-256 of nothing: e3b0c442...b855).

### T10 tokens

`buildings/PB_R_Sawmill01.GR2` section 0. Parameter blocks: part 0 `L` 256,
`R` 262143, literal capacity 138, high capacity 9, size capacities
`c0..c3` = 17, 4, 6, 25; part 1 (empty) 256, 262143, 1, 1, 1/1/1/1; part 2
256, 262143, 64, 5, 1/2/3/20. "ctx" = the size model used (previous size
symbol); "how" = hit (listed entry), pick (escape, then an unlisted entry),
new (escape, then a new entry).

| # | pos | ctx | size | token | how |
|---|---|---|---|---|---|
| 1 | 0 | 0 | 0 | literal 100 | new |
| 2 | 1 | 0 | 0 | literal 59 | new |
| 3 | 2 | 0 | 0 | literal 31 | new |
| 4 | 3 | 0 | 0 | literal 0 | new |
| 5 | 4 | 0 | 0 | literal 212 | new |
| 6 | 5 | 0 | 0 | literal 168 | new |
| 7 | 6 | 0 | 0 | literal 230 | new |
| 8 | 7 | 0 | 0 | literal 3 | new |
| 9 | 8 | 0 | 0 | literal 91 | new |
| 10 | 9 | 0 | 0 | literal 185 | new |
| 11 | 10 | 0 | 0 | literal 233 | new |
| 12 | 11 | 0 | 0 | literal 3 | hit |
| 13 | 12 | 0 | 0 | literal 3 | new |
| 14 | 13 | 0 | 0 | literal 0 | new |
| 15 | 14 | 0 | 1 | copy length 2, `l` 0, `h` 0, `md` 0, distance 1 | size, low, high, mid all new |
| 16 | 16 | 1 | 0 | literal 132 | new |
| 17 | 17 | 0 | 0 | literal 100 | new |
| 18 | 18 | 0 | 0 | literal 31 | hit |
| 19 | 19 | 0 | 0 | literal 0 | hit |
| 20 | 20 | 0 | 0 | literal 7 | new |

Further checkpoints in the same section: the first copy with `h` > 0 is at
pos 1748 (size 3, length 4, `l` 3, `h` 1, `md` 134, distance 1564). The last
token of part 0 is at pos 8524 (size 3, length 4, `l` 3, `h` 1, `md` 98,
distance 1420). Part 2 starts at pos 8528 with fresh models (part 1 is empty
and reads nothing): literals 51, 68, 32, 83, 116, 117 (all new, ctx 0); its
first copy is at pos 8542 (length 2, distance 1), the second at pos 8553
(length 2, `l` 2, `h` 0, `md` 1, distance 7). The section has 2883 tokens.

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-gfx-001 | tool run | survey script over the game's `data` files (container removed) | header, sections, compression census |
| E-gfx-002 | tool run | Oodle1 decoder trace (stats, hashes, tokens, models) | GFX-GR2-2 edge-case census over all 3134 non-empty Oodle1 sections; T9 to T12 |
| E-gfx-002 | code reading | `S2DNG.exe`, the vertex member name table | vertex buffer build (GFX-GR2-7.3, Q-124) |
| E-gfx-001 | tool run | the investigation's GR2 decoder and checker | decoder, type-driven reader, full check (0 errors) |
| E-gfx-001 | the program's import table | `S2DNG.exe`, granny2.dll imports | the 35 functions used |
| E-gfx-001 | code reading | `bin/granny2.dll` version resource and exports | 2.5.0.11; `GrannyDecompressData`, `GrannyConvertFileToRaw` exported |
| E-gfx-001 | data | `data/gfx/fx_20/*.fx`, `character/*.xml`, DDS headers | shaders, colouring, texture formats |
| reference | open source (investigation only) | nwn2mdk `gr2_decompress.cpp` / xoreos-tools | algorithm cross-check for GFX-GR2-2 |
| E-gfx-008 | code reading | granny2.dll (bin, sha256 dffacb3d...): curve evaluation (`GrannyEvaluateCurveAtT`), knot search, B-spline buffer set-up, sampling table, degree-2 samplers, sign fix, track sampling, control clock; data check over `data/character/animations` | curve evaluation and clock |
| E-gfx-039 | code reading | (session 2026-11-06): the material texture lookup (model folder + name); survey of all `.gr2` texture names against their folders (scratch script) | GFX-GR2-5.1 |

## Open questions

- Q-047: answered in `GFX-world-view.md` GFX-VIEW-1.2 / 1.3 (basis and units); the
  placement itself is GFX-VIEW-4.
- Q-048: meshes with more than 50 bone bindings or more than 2 weights per
  vertex: does the game split them or drop weights?
- Q-049: meaning of the building `_L` texture and of the `DiffuseColor0` vertex
  colour in building and object meshes.
- Q-179: answered by GFX-GR2-7.5 / 7.6.
