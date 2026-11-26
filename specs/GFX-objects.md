# GFX-OBJ: Objects in the 3D view: model tables, shading, settler colouring, flags and border, wares at flags, roads

Status: draft (read from the shipped data tables and HLSL shaders, with the binary for the lookups and placements; not yet checked against the running game)
Owner question(s): play-test findings F13 to F17 (Run B); Q-174, Q-175, Q-176 (parts), Q-084, Q-049; new Q-091 to Q-094; Q-206, Q-210, Q-202 (d) answered 2026-10-17
Game build: 11757

## Summary

Every visible object is a Granny model named by a data table: buildings by
`buildings.xml`, trees, stones, doodads and the road flag by the
`LandscapeElements` table of `terrain_static_data.xml`, wares by `items.xml`,
settlers by `settler_config.xml`, animals and ships by their tables. Each table
also names the shader that draws the object. The shaders all share one lighting
model (one directional light, ambient, a shadow map, fog of war) and differ in
how they colour the surface: houses mix a light map and a per-tribe colour
variation, trees sway, stones have a gloss map, flags and border markers take
the owner's colour where their texture is transparent, settlers are painted
through a palette atlas, roads are a lit strip with a normal map. Wares at a
flag stand in up to six stacks around it.

## Rules

### GFX-OBJ-1 Model tables (Q-174)

- **GFX-OBJ-1.1 Buildings.** `settings/buildings.xml` is a list of `Building`
  entries with `name`, `mesh` (the house), `frame` (the construction site) and
  child elements `DoorOpen`, `DoorClose` (animation files) and optionally
  `Working` (an animation with event children). The name is
  `<Tribe>_<building>` with Tribe in Roman, African, Chinese for the
  simulation's tribes 0, 1, 2 (all 108 entries follow this). The root element
  carries `lodBias`, `lodScale`, `colorMap` (default
  `data/buildings/colors.tga`) and `numVariations` (see 2.2). Confidence: high.
  Measured (2026-10-25, E-heap-004): the running game holds a record for every one of
  the 108 entries, all three tribes, the never-drawn ones included, with its `name`,
  `mesh` and `frame` as written (the animations and the root's values were not compared).
  Class rule (2026-10-24): every entry is a building model of this form; the entries
  `Roman_ForesterInked`, `African_ForesterInked`, `Chinese_ForesterInked`,
  `African_Lumberjack` and `Chinese_Lumberjack` are named by no building type and are
  never drawn. Covers (inventory): *building model* from `settings/buildings.xml`.
  Covers (inventory): *building clip* from `data/buildings/*`.
- **GFX-OBJ-1.1b A building's own colour map (2026-10-24).** A `Building` entry may carry
  its own `colormap`, used for that building instead of the root's `colorMap`: the three
  forester huts name `data/buildings/PB_R_ForesterHut01ColorIndex.dds`. Confidence: high
  for the data; medium for "instead of" (the loader's use not read).
- **GFX-OBJ-1.1a Levels of detail of a building (2026-10-24, file trace).** Corrected the
  same day by the file-open trace. A building's levels of detail are model files: the
  loader takes the model itself, then `<name>LOD0`, `<name>LOD1`, ... (the model's file
  name up to the extension, `LOD<n>`, the extension) as long as such a file exists,
  stopping at the first that does not, and from each level it keeps the mesh `House` (the
  whole model when it has none); the model also gives the `GROUND` mesh (E-sim-008). The shipped buildings have `LOD0` files only,
  so each building has two levels. (The earlier reading took `LOD<n>` for a mesh name and the files for
  unused.) Measured with the file-open trace (TOOL-trace TRACE-7.5, the install's
  last-access times over the sessions of 2026-10-13 to 2026-10-23): each of the 22 Roman
  building types shown in those sessions had its `LOD0` file read in the same second as
  its model's first load, and the game's cache holds that file's `House` mesh as
  `<model>LOD0_House.msh`; the `LOD0` files of the Nubian and Chinese buildings and of the
  Roman catapult, fortress and gold mine, none of which was shown, were never read. Which
  of the two levels is drawn at which distance: 1.1c. Confidence: high (code and trace).
  Covers (inventory): *other file* from `data/buildings/*lod0.gr2`.
- **GFX-OBJ-1.1c Which level is drawn (2026-10-25, Q-351).** Every frame, before the
  buildings are drawn, each building's level is chosen:
  level = trunc((d x lodScale + lodBias) x (1 - p) x 0.05 + 0.5), clamped to 0 .. the
  number of levels - 1, where d is the distance from the camera's eye point to the
  building's position, `lodBias` and `lodScale` are the building entry's own values (a
  `Building` may state them; otherwise the root's; shipped: -70 and 1.2 everywhere, the
  Roman bakery restating them), and p is a value of the scene settings (`SET-settings.md`
  SET-4) that the graphic quality presets set to 0, 0.25, 0.5, 0.75 and 1 for VeryLow,
  Low, Medium, High and VeryHigh (which engine slider writes it is Q-100 c). Level 0 is
  the model, level 1 its `LOD0` file (1.1a). So at p = 1 (VeryHigh, the default) the
  factor is 0 and only the model is ever drawn; at p = 0 the `LOD0` model replaces it
  from d >= 66.7 (1.2 d - 70 >= 10), at p = 0.25 from 72.2, at 0.5 from 75, at 0.75 from
  91.7. The settings `lod0Range` .. `lod2Range` and `ForceLOD` belong to the settlers'
  table (`character/settler_config.xml`) and `lodDistance` to the landscape
  (`graphics.xml`); none of them changes a building's level. Confidence: medium (code
  only: E-gfx-032: the choice, the 0.05 and 0.5, the attributes into
  the entry's record, the presets, p from the
  scene settings; not observed: the game runs at VeryHigh on the recording PC).
- **GFX-OBJ-1.2 Landscape elements.** `settings/terrain_static_data.xml` holds,
  after the pattern list, a `LandscapeElements` list. Each `LandscapeElement`
  has `name`, `mesh`, `shader`, optional `rot` "min max" (degrees), `scaleX`,
  `scaleY`, `scaleZ` "min max" (default 1 1), `offset` (two numbers, default 0
  0), `numInstances` (default 50), `vbSize` (15000), `ibSize` (30000), `type`
  (one of flag, tree, stone, wheatField, gate; anything else is a plain doodad),
  `meshType` ("extended"), `shadowCast` and `shadowReceive` (default true),
  `forceShadowPass`, `priority`, `blendMode` (0 = alpha test at 127, no
  blending; 1 = alpha test and alpha blending; 2 = additive, no test) and
  `alignOnWater`. Entries are read in order; a repeated name is ignored (the
  first wins). Confidence: high. Measured (2026-10-25, E-heap-004): the running game
  holds one record for each of the 144 entries (those of every map, not only the one played), with `mesh`,
  `shader`, `forceShadowPass`, `rot` (as radians), the three scale ranges (each axis
  its own, as 1.4 says: tree_09's x range 0.9 .. 1.0 beside y and z 0.8 .. 1.0),
  `offset`, `numInstances`, `priority`, `blendMode`, `shadowCast` and `shadowReceive`
  as written, and the defaults above for every value an entry leaves out (scales 1 1,
  offset 0 0, 50, 15000, 30000, shadows on, blendMode 0). `type`, `meshType` and
  `alignOnWater` are kept in another form and were not compared.
  Covers (inventory): *landscape element* from `settings/terrain_static_data.xml`.
- **GFX-OBJ-1.2a Gate water (2026-10-24).** When the first landscape element of type gate
  is made, the game loads, once for all gates, the mesh `data/map/objects/LO_Gate_water.GR2`
  with the effect `gate_shader.fx` and 31 frame textures
  `data/map/textures/gate_water0000.dds` .. `gate_water0030.dds` (the name is the frame
  number in four digits; the count 31 is a constant of the executable). How the frames are
  stepped and drawn is not read. Confidence: high for the files and the count (E-sim-007); the animation is open (Q-348). Covers (inventory): *texture* from
  `data/map/textures/gate_water*`.
- **GFX-OBJ-1.3 Deposits and doodads look up by gfx name.** A deposit's
  `dp_setGfxName` value (`tree_01` .. `tree_14`, `tree_lava01..03`,
  `Wheatfield`, `stoneressource_01..06`, `medstoneressource_01..06`) and a
  doodad's name are the `name` of a landscape element; the element gives the
  mesh and shader. Shipped mapping: tree_01..03 = `RS_TreeBirch01..03`,
  tree_04..06 = `RS_TreeBroadLeaf01..03`, tree_07/08 = `RS_TreeFir01/02`,
  tree_09/10 = `RS_TreePalm01/02`, tree_11 = `RS_TreeCypress01`, tree_12 =
  `RS_TreeOlive01`, tree_13 = `RS_TreeAfrican01`, tree_14 =
  `RS_TreeCherryBlossom01`, tree_lava01..03 = `RS_TreeLava01..03` (all
  `tree_shader.fx`, type tree); stoneressource_NN = `RS_StoneRessourceNN`,
  medstoneressource_NN = `RS_MedStoneRessourceNN` (`stone_shader.fx`, type
  stone); Wheatfield = `LO_Wheat.GR2` (`weed_shader.fx`, type wheatField);
  `do_not_use_01` (the test deposits) = `RS_TreeBirch01`. All files are in
  `data/map/objects/`. Confidence: high.
- **GFX-OBJ-1.4 Random rotation and size (Q-084).** When an instance of an
  element is created, its yaw is drawn uniformly from the element's `rot` range
  and each scale axis from its `scaleX/Y/Z` range; `offset` is an extra
  placement offset (used by the resource signs: 1 1). Shipped values: every
  tree 0..360 degrees and 0.8..1.0 (tree_09 x 0.9..1.0; tree_13 and tree_14
  0.6..0.8); stones no rotation and no scaling; doodads 0..360 and mostly
  0.8..1.2 (grass 0.8..1.0; rock2 -20..20; the fingerposts and gate fixed).
  The growth scaling of a deposit (growing start size) multiplies this.
  Confidence: medium (ranges read from the table reader and the data; the
  random draw itself was not read).
  **The draw (answered 2026-11-06, Q-1021; E-gfx-039).** The view
  has a random generator of its own for map objects: the generator of `SIM-RNG.md` (Lehmer, 16807
  modulo 2^31 - 1, the same seed repair), seeded **once when the program starts** with 0x5432ACE0 and
  never reseeded (not by a new game, a load or a map change). A unit draw is (next mod 1000001) x
  0.000001. Each landscape-element instance the view creates (every doodad, deposit, tree, stone,
  field and road flag, in the order the game reports them to the view: a map's or save's objects when
  it is loaded, then every new one as it appears) makes exactly **four** draws in this order: yaw =
  rot.min + u x (rot.max - rot.min) (radians), then the x, y and z scale = min + u x (max - min) of
  each axis. The draws are made even when a range is empty (rot 0 0, scale 1 1), so every instance
  moves the stream by four. The instance's matrix is scale, then the turn about y, then the
  translation. Its place is the object's **fine position** (a sample of the height lattice,
  `MAP-grid-geometry.md` MAP-GRID-4, turned into world x, z by MAP-GRID's scale; corrected
  2026-11-07, see "Where a deposit is drawn" below) plus the element's `offset` (x, z) and nothing
  else (no random offset); its height is the ground height at that point (the water level when
  `alignOnWater` is set and the ground is lower). So a tree's turn and size depend on how many
  instances the view made before it in that program run; the farm tree of Q-1021 has the values
  its place in that sequence gives. Confidence: high (code).
  **The order of a load (answered 2026-11-07, Q-1027 a; E-gfx-040).** The view learns of a loaded map's or
  save's objects only after everything is read, when the game finishes the load system by system.
  The order of the instances, and so of their draws, is:
  1. **Road flags**: for player 0, then 1, ... 7, each player's flags in the order of that player's
     flag list in the file (FMT-SAV-3.9). A map file holds no flags.
  2. **Deposits** (trees, stone piles, grain fields): in the order of the file's deposit list
     (`FMT-s2m-map.md` resources, FMT-SAV for a save), each one only if its size is 0 or more (a
     deposit not yet started, size -1, makes no instance). On every shipped map that list is in
     ascending id order (Storm Coast: 3212 deposits, ids 11 .. 89913).
  3. **Doodads**: the file's first doodad list in ascending id order, then the second, then the
     third (the blocking rocks and the gate), each by id. A doodad whose type names no landscape
     element makes no instance and no draws.
  Nothing between makes draws (animals, buildings, settlers and roads use no landscape element).
  What the game creates after the load (a new game's headquarters flags, every later flag, tree,
  field, sign or doodad) follows in creation order. No map is loaded before the first game in a
  program run (the menus draw no map: FILE-TRACE has `Presentation.s2m` never read), so a free
  game started from the menus begins with the generator at its seed. Confidence: high (code; the
  file orders counted on all Storm Coast lists).
  **Where a deposit is drawn (answered 2026-11-07, Q-1027 b; E-gfx-040).** The view takes the deposit's own fine position, the one of `TRN-free-walk.md`
  TRN-FREE-5.4: for a deposit made in the game, one fine step from its field's sample in direction
  d when the kind has the random offset (trees) and that sample is on the height lattice, else the
  field's sample; for a map's or save's deposit, the fine position stored in the file (the shipped
  maps' trees are 0 or 1 fine step from their field's sample). So a tree stands where the woodcutter
  walks to it, not on the field's own point. Doodads are drawn at their stored fine position too
  (some doodad kinds are snapped to the nearest field's sample when made). Road flags stand on
  their field's sample. Confidence: high (code).
  **Flags (type flag) are shifted (E-gfx-039; index corrected 2026-11-07, E-gfx-040).** A road flag's instance also moves along world z by 1.3 x the `<Street>`
  width (1.0 in `graphics.xml`, so 1.3 units): with i = r x (4 x map width + 1) + c, where (c, r)
  is the flag field's **sample** on the height lattice (MAP-GRID-4.2: c = 4x + 2 on odd rows, else
  4x; r = 4y), not the field index; a = 58 i, u = a xor (a x 8192), v = (0x3434B6 - (u x u x 0x4BD9 +
  0x2E48D) x u) and 0x4B746F (32-bit arithmetic), the flag moves by -1.3 when v x 2.98105e-7 <=
  0.5, else by +1.3. Confidence: high for the arithmetic (code); medium for the axis (read from the
  stored coordinate, not from a picture).
  **Sway phase.** The tree sway (2.4) takes its phase from the instance's world translation and the
  clock; there is no random phase.
- **GFX-OBJ-1.4a Doodads removed by roads, flags and buildings (2026-11-11, Q-207 d; E-gfx-054).**
  A doodad type's "delete by street" switch (`dd_setDeleteByStreet`) is **on unless a script
  turns it off** (the type's constructor sets it; the shipped scripts turn it off only for the
  14 signs). Whenever the game rechecks the doodads at a field or over a box of fields after a
  change there, it **removes** each doodad (erases it from the map for good; it does not come
  back when the road or building goes) whose own field (the doodad's field, not its sample; for a
  doodad made in the game, such as a skeleton, the nearest field of its sample, MAP-GRID-4.4,
  `TRN-carriers-transport.md` TRN-CARRY-7.1, Q-1072) is
  covered: (i) its type deletes by street and the field has the Street state (`FMT-s2m-map.md`
  FMT-S2M-4.4 bit 1), or (ii) the field is MovementBlocked (bit 8), whatever the switch, or (iii)
  the field is DepositBlocked (bit 7) and a further test on the doodad holds, or (iv) an object
  of the field's object layer stands on it (a flag or building). Confidence: high for the
  default and for removal being permanent; medium for (iii), (iv) and the boxes rechecked.
- **GFX-OBJ-1.5 Flags and border.** The road flag is the landscape element
  `z_flag_00` = `map/objects/flag.gr2` with `flag_shader.fx`, type flag, fixed
  yaw 90 degrees. Border markers are `map/objects/limes.GR2` on land and
  `map/objects/buoy.GR2` on water, drawn with `border_shader.fx` as instanced
  meshes. Military buildings carry `buildings/MilitaryFlag1.GR2` or
  `MilitaryFlag2.GR2` (with `.dds` textures) drawn with `house_flag.fx`, whose
  texture is offset vertically by a per-flag value (which flag and which
  offset a building uses: Q-094). Confidence: high (models), medium (flag 1 / 2).
- **GFX-OBJ-1.6 Items, animals, ships.** `settings/items.xml`: `ItemDefinitions`
  with `Item name mesh` per good (Wood `GO_Log01`, Plank, Stone, Pig, Corn,
  Flour, Fish, Flesh, Bread, Water, Beer, Coal, Iron, IronOre, Gold, Anvil,
  Coin, Gripper, Axe, Saw, Hammer, Shovel, Pot, "Fishing Rod", Scythe, Hatchet,
  "Rolling Pin", Spear, Sword, Shield, Boat, Pickaxe, TestItem), `AnimatedItems`
  (none active), `DonkeyItems` (Wood = `dummy_item.gr2`) and `StackLayouts`
  (5.2). `settings/animals.xml`: Deer, Rabbit, Elk with `model`, `mesh`,
  `scale` and Stand / Walk / Dying animations. `settings/ships.xml`: `Freight`
  and `Expedition` meshes and one `Ship` per tribe with `scale`, `frame`,
  `mesh`, `model`, `idle`, `move`. Confidence: high. Measured (2026-10-25,
  E-heap-004): all 33 named `Item` entries are held with their `mesh`; Deer, Rabbit and
  Elk with their `model` and `scale` (their `mesh` is kept elsewhere; ships, the
  production animals and the stack layouts were not compared).
  Covers (inventory): *ship clip* from `data/ships/*`.
  Class rule (2026-10-24): the wild animals' clips are the Stand, Walk and Dying entries
  of `animals.xml`, played by GFX-ANIM-1.1. The production animals of the piggery and
  the donkey breeding (`Production` children of `buildings.xml`) are not specified here.
  Covers (inventory): *animal clip* from `data/animals/*` except
  `data/animals/donkey_stand.gr2`, `data/animals/farmpig_idle.gr2`.
- **GFX-OBJ-1.6b The animated fishing rod (2026-10-24).** `items.xml`'s only
  `AnimatedItem`, the fishing rod (`data/items/AO_FishingRod01.gr2` with
  `AO_FishingRod01_Model.gr2` and `AO_FishingRod01_Anim.gr2`), is commented out (marked
  "Buggy"), so `AnimatedItems` is empty and the three files are not used; the fisher's rod
  is the plain item `GO_FishingRod01`. Confidence: high (the shipped file).
- **GFX-OBJ-1.6a Production animals (2026-10-24).** A `Building` entry of
  `settings/buildings.xml` may hold one `Production` child (the donkey breeding: donkeys,
  5 for the Roman and Chinese entries, 4 for the African; the piggery: 4 pigs): `mesh`,
  `model`, `texture`, `numAnims` 1 with `anim0` (the one clip they play, `donkey_stand` or
  `farmpig_idle`, looped), and `numModels` N with `boneLink0` .. `boneLink<N-1>` naming
  bones of the building's model (`donkey01` .. `donkey05`, `pig01` .. `pig04`), one place
  per animal. **How many show (Q-349, 2026-10-24):** the building's view starts the
  clip, looping, for all N animals and places each at its bone, but shows only the first
  k of them in `boneLink` order: k = the building's productivity (the figure of its
  window, 0 .. 1, `ECO-outdoor-workers.md` ECO-OUT-3.11) x N, rounded to the nearest
  whole number (adding 0.5 and cutting off). The view checks this every frame while the
  building's production is on and its worker is in (the test that also gates its input
  orders), and changes the animals only when the productivity value has changed since
  its last check. A building that has never produced reads -1, so no animal shows until
  the first product (which sets 0.5: 3 of 5 donkeys, 2 of 4 pigs); at full productivity
  all N show. While production is off or the worker is away the shown animals stay as
  they were (the view only forgets its last value, so the count is set again when work
  resumes). Confidence: high (E-gfx-031);
  medium for the bones as the places. (An earlier citation of another function here was wrong:
  that function makes objects for the settler views.) Covers (inventory): *animal clip* from `data/animals/donkey_stand.gr2`,
  `data/animals/farmpig_idle.gr2`. Covers (inventory): *animal type* from `settings/buildings.xml`.
- **GFX-OBJ-1.7 Settlers.** `character/settler_config.xml` `Characters`
  (scale 1.2, evadeRadiusScale, evadeStart, boatOffset, goodOffset,
  goodRotation, lod0Range 40, lod1Range 50, lod2Range 60) holds one `Character`
  per `<Tribe>_<Profession>` with `mesh`, optional `mesh1..3` (LODs by the
  ranges), `color` (the colour-index map, 3.2) and `slotMap` (the colour map
  name, default "default"). Professions: Carrier, Constructor, Bulldozer,
  Woodcutter, Forester, Stonecutter, Fisher, Hunter, Cabinetmaker, Baker,
  Miller, Farmer, Pig Farmer, Slaughter, Helper, Miner, Soldier, Soldier1 ..
  Soldier4 (ranks), Explorer, Scholar, Locksmith, Brewer, Smith, Coiner,
  Smelter, DonkeyBreeder, ShipBuilder. `Models` names the shared skeleton file
  and `Donkey` the donkey (scale 1.0, mesh with LODs, model, `link` =
  `SLOT_Goods`, animations). Confidence: high.
  The donkey is one of these characters, with the mesh
  `data/character/donkey/donkey_mesh.GR2`.
- **GFX-OBJ-1.8 Rest pose facing (Q-176a).** The settler and animal model
  files use the same art-tool axes (right +x, up +z, back -y) and their rest
  poses face file +x (toe ahead of foot, head ahead of pelvis), which the basis
  conversion of `GFX-world-view.md` GFX-VIEW-1.2 turns into world +z. The
  heading table of GFX-VIEW-4.1 (direction 0 = +90 degrees) then turns +z to
  +x = direction 0. No per-kind correction is needed. Confidence: high
  (computed from the skeletons; test T6).

### GFX-OBJ-2 Shading (Q-175, Q-049)

Common to all object shaders: per-vertex light `L = saturate(N . lightDir) x
lightColour`, pixel `D = ambient + L x shadow` (shadow = 4-tap shadow map,
`GFX-world-view.md` GFX-VIEW-2.3/2.4), colour x fog-of-war alpha at the end
(GFX-VIEW-5.1). Alpha test where stated rejects alpha <= 127 / 255.

- **GFX-OBJ-2.1 Houses (house_shader).** Vertex: position by the bone matrix
  (one bone index per vertex), normal likewise; three texture coordinate sets.
  Pixel: `C` = colour texture (set 0), `lightMap` = the `_L` texture (set 1),
  `colorMap` = `colors.tga` sampled at set 2 shifted by the building's colour
  offset (2.2); `result = D x C x colorMap`; `lit = result x lightMap`;
  `result = lerp(result, lit, saturate(1.5 - D))` per channel (the light map
  shows fully in the dark, fades out where the direct light is strong); alpha
  = C.a, alpha test 127. The mesh's vertex colour is not used. The
  construction-site model (frame pass) and attached add-ons (skinned, 4 bones)
  are lit the same way with the colour texture only. Confidence: high.
  **Light map binding (play test 10, Q-219; E-gfx-021).** The light map is
  the texture the Granny material lists under the usage
  `Self-Illumination` (for the Roman headquarters `PB_R_Castle01_L.dds`, the
  baked "CompleteMap"); it is loaded raw (no x 2, no sRGB, all mip levels
  kept), sampled at texture coordinate set 1, and the term is applied in
  every technique the game uses (`Default`, `NoShadow` and the low variants
  alike), per channel. So the darkening the program computes on the HQ floor
  in full sun (e.g. (223, 194, 195) to about (198, 156, 158)) is the
  original's arithmetic; keep the term (`GFX-world-view.md` GFX-VIEW-2.6).
  **The house's light normal is not turned by the building (2026-11-01, T31).**
  In the house pass (not the construction-site frame pass or the skinned
  add-ons) the vertex's light term saturate(N . -L) x light colour is computed
  per vertex with the normal turned by its bone matrix only: the model's own
  frame after the load-time basis conversion (`GFX-world-view.md`
  GFX-VIEW-1.2), not turned by the building's heading of -30 degrees
  (GFX-VIEW-4.3), which turns the positions only. L is the world light
  direction. So a house is lit as if the sun stood 30 degrees further round,
  in the sense of the building's turn: at noon (heading 50) the apparent sun
  of a house moves from the screen's lower left toward the bottom. The frame
  pass and the skinned add-ons turn their normals with the building's matrix
  and are lit by the true sun. Confidence: medium-high (shipped
  `house_shader.fx`: the main vertex shader multiplies the normal by the
  instance matrix only, while the position also goes through the world
  matrix, which holds the building's turn and place; the frame and skinned
  shaders use the world matrix for both; measured, T31).
  **What else enters a house pixel (answered 2026-11-06, Q-1020; E-gfx-039).** Nothing: in full sun a
  house pixel is exactly the formula above, D = ambient + saturate(N . -L) x light colour x shadow
  (the 4-tap shadow factor of `GFX-world-view.md` GFX-VIEW-2.5, on the direct term only), times the
  colour texture and the colour column, mixed toward x light map by saturate(1.5 - D) with D
  including the ambient. There is no specular or gloss term in the house pass, and the normal is the
  unit model normal (models are renormalised on load and their bones carry no scale). The dome's
  difference on the look-compare pairs is T31's own: original / program over the dome's width runs
  0.97, 1.01, 1.10 .. 1.13, 1.24 .. 1.29 from left to right at tick 1500 (0.96 .. 1.24 at 6000), the
  gradient T31 predicts, and the program's dome pixels are the same before and after T31 was written
  (2026-10-31 and 2026-11-06 renders identical there): the program still lights the house with the
  normal turned by the building's heading. Building T31 is the change; a residual after it would be
  a new question. Confidence: high for the formula (shipped shader); the attribution is measured.
- **GFX-OBJ-2.2 Building colour variation.** `colors.tga` (64 x 256) is cut into
  `numVariations` columns; a building draws with the column `variation mod
  numVariations`, offsetting its third texture coordinate by `variation /
  numVariations`. **Corrected 2026-11-11 (Q-094, E-gfx-053): the variation is per building.**
  The 2026-11-08 b reading ("one value for the whole pass", column 0) took the wrong argument:
  the value handed down by the pass is a pointer to the view's parameters, not a number. The
  house draw takes the building's own **slot number** in the house renderer's table (the
  1-based handle under which the building's house view was registered, the same number it
  looks the house up by), and draws with column `slot mod numVariations` (`numVariations` = 8
  in the shipped `buildings.xml`): the third texture coordinate is offset by
  `(slot mod 8) x (1 / 8)`. So neighbouring buildings usually show different colour columns.
  How slots are given out is GFX-OBJ-2.2a. Confidence: high.
- **GFX-OBJ-2.2a How a building gets its slot (answered 2026-11-11 eve, Q-094; E-gfx-055).**
  The slots are one numbering for the whole map: every building of every player and of every
  type draws from the same table. The 3D view keeps a stack of free slots and a count of the
  slots ever made.
  1. **When.** A building takes its slot when the building object is created; for a building
     the player places that is the moment its construction site appears (the site's rising
     house is the same house view, GFX-OBJ-2.6b). Completion does not change the slot. The
     headquarters and every building a map or mission places at the start take theirs as they
     are created, in creation order (the order of their object numbers, `SIM-game-start.md`
     SIM-START-4).
  2. **Which number.** If the free stack is empty, the building gets a new slot: the count of
     slots ever made plus 1 (the first building of a game gets slot 1, so its column is 1, not
     0; slot 8 gets column 0). Otherwise it takes the slot freed **last**: a freed slot is
     always reused before a new number is made.
  3. **Freeing.** When a building goes, its slot is pushed onto the free stack:
     (a) a construction site (frame or main work still left) that is torn down, burnt or lost:
     at once;
     (b) a **finished** building (every shipped type has a ruin, `bp_setRemains` in
     `buildings.lua`): only after its collapse. The house stays in the view, with the
     `DemolitionDust` effect and the collapse sound (`SND-sound-music.md`), and sinks: a
     collapse clock starts at 0 and rises by 0.35 per second of the view's frame time
     (clamped at 1), and the house is lowered by its box height x clock squared. The slot is
     freed at the first view update after the clock reaches 1, so about 2.9 s after the
     removal; a building created during the collapse cannot get that slot. This holds for
     every removal of a finished building (burnt, destroyed, demolished, lost land) except
     the two in 4.
  4. **A building that changes type keeps its slot.** An upgrade (GFX-OBJ-8.4; the type
     changes when the upgrade ends) and a conquest (`MIL-conquest.md`: the old building is
     removed and one of the same type made for the new owner) free the old slot at once,
     without a collapse, and create the new house right after, which takes that same slot
     from the top of the stack. So a building's colour does not change when it is upgraded
     or captured. The upgrade in progress (GFX-OBJ-8.4) is drawn by the building's own house
     view and takes no further slot. (Corrected 2026-11-12, E-gfx-057: the separate view that
     the first reading took for an upgrade site is the ruin's view, 8.7 (5); it takes no slot
     either.)
  5. **A loaded game** builds its view afresh: the buildings take slots 1, 2, 3, ... in the
     order of the save's building list (FMT-SAV-3.4: all players in one list, in creation
     order), with an empty free stack. So after a load the slots are packed, and a building
     shows a different colour column than before the save whenever a slot below it was free
     at the time of the save or had been reused by a later building.
  6. **Only the house pass reads the column.** The construction frame pieces, the skinned
     add-ons, the ground decal, the flags (the building's flag, soldier and border flags),
     roads and every other object do not sample `colors.tga`; only the house mesh (also
     while it rises on a site) is multiplied by its column.
  Tests: T35. Confidence: high for 1 to 4 and 6 (code read: the slot table and its free
  stack, the house view's creation and removal, the finished-building test, the type change
  and conquest paths, the collapse clock; `house_shader.fx`); medium for 5 (the post-load
  order is read; that a load from inside a running game also starts from an empty table is
  inferred from the view being built for each game, not read).
- **GFX-OBJ-2.2b How the colour column is sampled (2026-11-12 night, E-gfx-058).**
  (a) **Lookup.** The colour map's coordinate is the vertex's third texture coordinate set plus
  the column offset `(slot mod 8) / 8` in u, plus **half a texel in both directions** (1/128 in
  u, 1/512 in v for the 64 x 256 `colors.tga`), sampled **bilinearly with wrap addressing** in
  u and v. Every part of a shipped house mesh carries one constant coordinate on all its
  vertices, so the filter never changes level (the top level is read) and each part takes one
  colour per column. The map has smooth gradients along u inside each 8-texel column and bands
  of 8 rows along v (white, orange, greys, blue, yellow, ...). A part whose coordinate lies
  within half a texel of a band's lower edge, or whose u lands in the last half texel of the
  map, therefore mixes in the next band or the map's first column; a nearest lookup without
  the half texel reads the band alone. Where this shows (largest difference over the 8
  columns, of 255): the Nubian harbour (a part at u 7.78, v 75.06 texels: 48 in column 7), the
  Nubian well (v 75.86: 36 in column 6), the Nubian locksmith (u 55.55, v 74.34: 34 in column
  1, the wrap to the first column), the Roman woodcutter's log pile (u 1.27, v 63.09: 9 % of
  the blue band below, up to 9 in each column); every other part of the shipped meshes stays
  within 3. Test T38.
  (b) **Meshes without the third set**: every Chinese house mesh, and the Roman and Nubian
  mines, the Roman military buildings (guard house, watch tower, barracks, fortress, catapult),
  the Nubian watch tower, donkey breeding and piggery (41 of the 99 house meshes named in
  `buildings.xml`). They read the coordinate (0, 0) plus the column offset: row 0, white in
  every column, so they show no tint whatever their slot. Confidence: medium (the value of a
  vertex input the mesh does not have is the device's default, not read).
  (c) **The woodcutter's roof (the look of 2026-11-12, `lookC-006000-town`).** The roof of the
  Roman woodcutter's hut in slot 6 (column 6) reads the orange band at about (218, 159, 94). The
  program's roof matches the original's pixel for pixel within 1 % (original / program 0.99 ..
  1.01 over the roof's 16 700 pixels). The 5 % / 11 % difference the roof box showed came from
  other pixels in the box: the hut's door-side wall, which the original shades
  (`GFX-world-view.md` GFX-VIEW-2.5 (f)), passed the box's colour filter only in the program's
  picture (2 000 pixels about 1.45 times brighter there). The colour column and the slot rule
  are not involved.
  Confidence: high for (a) (shader source and sampler states; the file's texels); measured for (c).
- **GFX-OBJ-2.3 Ground decal (corrected 2026-10-17, Q-202 d, Q-210).** Each
  building lays its type's `GROUND` mesh (from the house model file, e.g.
  `PB_R_WoodcutterHut01.GR2`; 44 vertices, 18 triangles for the woodcutter) on
  the ground with `buildings/house_decal.dds` and `house_decal.fx` (lit,
  alpha blended by the texture's alpha, depth-tested, no depth write). The
  geometry is built once (and again when the terrain under it changes):
  (a) the `GROUND` triangles are placed by the building's matrix (position,
  rotation); (b) every one of them is cut against every terrain lattice
  triangle under the decal's box (the cells the box covers plus one cell
  around), so each piece lies inside one terrain triangle; (c) every vertex
  of a piece is then put at the terrain height under it plus 0.01. A piece's
  texture coordinates are interpolated (barycentric) from the `GROUND`
  triangle's own first uv set (`TextureCoordinates0`); the world position is
  used only for the fog-of-war lookup. The texture is sampled with
  **wrapping** in both u and v, with v measured from the top row of the
  image (DirectX convention, no flip). `house_decal.dds` (1024 x 1024) is an
  atlas: its lower half is earth (left: open ground, right: four edge strips
  whose alpha fades out), its upper half paving (left) and paved edge strips
  (right). Every building's `GROUND` mesh spans u 0.002..0.998 and v
  -0.498..-0.002 or 0.502..0.998, which with wrapping is the earth half;
  only the Roman castle and the Roman fortress use v 0.002..0.498, the paved
  half. The mesh is a nine-patch: centre from the open ground, borders and
  corners from the fading strips, which gives the soft, ragged edge of
  `site-foundation.jpg` (OBS-18). A construction site shows the same decal as
  the finished building (it is created with the building, from the house
  model, not from the site model). Clamped sampling or a flipped v would
  sample the top rows (grey stripes); that is not the original's look.
  Confidence: high (E-pt5-001; `house_decal.fx`, `shared.inc` sampler 0; GROUND uv survey).
- **GFX-OBJ-2.3a Decal look (Run D, OBS-24, Q-212).** What makes the patch
  of `site-foundation-2.jpg`:
  (a) **Size and turn.** The decal is the `GROUND` mesh at its own size (the
  building's matrix: the -30 degree heading of GFX-VIEW-4.3 and the
  translation, no scale). The woodcutter's is an opaque centre of 5.3 x 4.3
  units (model x -2.73..2.57, z -2.53..1.80) with fading borders out to 6.7 x
  6.0 units (x -3.48..3.24, z -3.38..2.65), long side along the model's x,
  so on screen it is turned with the house, and the flag-to-door road strip
  runs from its front edge to the flag (the road strip to the door of 2.6).
  (b) **Soft edge.** Each border's alpha falls **linearly from 255 at the
  centre's edge to 0 at the outer edge**, i.e. over 0.68 to 0.85 world units
  (the atlas's edge strips are plain vertical gradients, 128 texel rows
  each, mirrored for the four sides by the mesh's uvs; T3g). The ragged look
  in the photos is the terrain (its texture and grass) showing through that
  wide fade plus the earth texture's own dark blotches; the strips
  themselves are not ragged. An edge that goes from opaque to clear in much
  less than 0.7 units (a few pixels at zoom 1) is not the original's look.
  **Widths and steps (2026-10-18, E-gfx-028):** for the woodcutter the fade
  runs from the opaque centre's edge to the mesh's outer edge over 0.68
  units on the +x side (x 2.57 to 3.24), 0.75 on the -x side (-2.73 to
  -3.48), 0.85 at the front, -z (-2.53 to -3.38) and 0.85 at the back
  (1.80 to 2.65). The alpha is linear in distance but stored as DXT3 4-bit
  values, so it falls in 15 steps of 17 (255, 238, ..., 17, 0), one step
  per about 0.05 units, softened by the bilinear filter between texels;
  there is no extra smoothing, curve or dithering. The pixel colour across
  the border is the straight blend of `GFX-world-view.md` GFX-VIEW-2.6 (c1)
  (test T26 there).
  (c) **Sampling.** Sampler 0: wrap in u and v, linear min / mag filter
  with linear mip filtering (trilinear, no anisotropy); the texture has 11
  mip levels (DXT3).
  (d) **Light.** Colour = (ambient + max(0, N . -L) x light colour x
  shadow) x texture colour, alpha = texture alpha, then fog of war. N is the
  `GROUND` mesh's own normal turned with the building (straight up for the
  shipped meshes), not the terrain's, so on a slope the patch is lit as if
  level and can look brighter or darker than the ground around it; it
  receives the shadow map (shadows of the house, trees and settlers fall
  on it). No other factor. Confidence: high (E-pt5-001; `house_decal.fx`,
  `shared.inc` CalcLight / CalcColor; the investigation's decal view script renders the
  rule top-down).
- **GFX-OBJ-2.6 Construction site (corrected 2026-10-17, Q-206, OBS-2,
  OBS-3).** From placement until the building is finished its field shows the
  door flag with the road strip to the door (2.3, ECO), the ground decal of
  2.3 and, depending on the work done (ECO-HQ-4.6, 4.7), the site model's
  pieces (2.6a) and the rising house (2.6b). With no work done (also during
  levelling, before the builder arrives or while he waits for material) only
  the decal and the flag are visible. "Frame progress" f = 1 - frame work
  remaining, "main progress" m = 1 - main work remaining, both 0..1.
  Confidence: high (E-pt5-001).
- **GFX-OBJ-2.6a Site model pieces.** The site model (the `frame` attribute of
  buildings.xml) consists of meshes named `0`, `1`, `2`, ... ; they are joined
  in **numeric name order** (not file order) and N = their number (the Roman
  woodcutter's site: 16, `PB_R_Generic01ConstructionSite.GR2`: 4). (A model
  without numbered meshes would be joined in file order.) The stage is
  s = min(N, floor((N + 1) x f + 0.5)), computed in 32-bit floats from the
  logic value of f (no interpolation between ticks); meshes `0` to `s - 1`
  are drawn whole, nothing at s = 0. During the main part f = 1, so the whole
  site model stands; it disappears in the tick the building is finished. Piece
  k first shows when f >= (k - 0.5) / (N + 1). For the Roman woodcutter the
  pieces are: 0 to 3 material piles on the ground, 4 to 7 the corner posts, 8
  to 11 the top beams, 12 to 14 the rafters, 15 the ridge beam.
  **Pieces 0 to 3 (Run D, OBS-25):** four low earth mounds around the
  frame, outside the posts; two of them carry the building material: piece 0
  (model x -4.9..-0.7, z -2.8..2.2) a mound with a small stack of planks and
  a tool handle stuck in it, piece 2 (x -2.3..4.9, z -3.3..-0.6) a mound with
  a plank stack and a shovel standing upright (up to 2.0 high); pieces 1
  and 3 (left side, about 2 x 2.5 units, 0.75 high) are bare mounds. They are
  part of the site model, so they do not depend on the wares delivered:
  with one frame unit all four appear during the builder's first work step
  (T3c: decrements 7, 20, 33, 46), the four posts during the second, the
  top beams and the first rafter during the third, the last rafters and the
  ridge during the fourth, and all of them stay until the building is
  finished, when the whole site model goes in one tick. **No ware is drawn
  at a site**: delivered wares are only in the site's stock (the only ware
  stacks in the world are the six-place stacks at flags, 5.1). So
  `site-frame-posts.jpg` shows stage 8 (pieces 0 to 7: mounds, planks,
  shovel, four posts; f between 0.441 and 0.5) and `site-frame-beams.jpg` a
  stage of 12 or more.
  Confidence: high (E-pt5-001).
  **How the pieces are shaded (2026-11-12, E-gfx-057, `house_shader.fx` pass
  RenderFrame; GFX-OBJ-2.2a (6)).** A pixel of the site model is
  rgb = (ambient + light colour x saturate(N . L) x s) x texel rgb, then the
  fog of war, with N . L taken per vertex from the mesh normal under the
  building's matrix and s the shadow factor (GFX-VIEW-2.5; 1 with shadows
  off). Nothing else: **no colour column** (`colors.tga` is not sampled, so
  every site looks the same whatever its slot), **no light map**, and the
  texture's alpha does nothing (no alpha test, no blending; the pieces are
  opaque and write depth). This differs from the house (2.1: colour column,
  light map, alpha test at 127), also from the rising house on the same site,
  which is a house and keeps its slot's column. Confidence: high.
- **GFX-OBJ-2.6b Rising house.** The house model is drawn while
  constructing only when m > 0, cut by a horizontal plane at height
  (H - 0.1) x m x 1.1111112 above the building's base (its field's ground
  height); H = top minus bottom of the house model's bounding box (the last
  LOD file loaded, i.e. `...LOD0.GR2` when it exists; Roman woodcutter
  4.297). Here m is interpolated between the previous and the current tick's
  value over the tick (like GFX-ANIM-4.1). The house therefore rises during
  the **main** part, inside the complete site model, and is complete in
  height from m = 0.8725 (woodcutter). So the order is: decal, then the site
  model piece by piece (frame part), then the walls (main part), then the
  site model goes. Confidence: high (E-pt5-001); medium for which LOD's box gives H.
- **GFX-OBJ-2.6c Fallback without shaders.** On the fixed-function render
  path (the renderer's capability level below 2) the house is not drawn
  until finished and the stage of 2.6a uses (f + m) / 2 instead of f. The
  reimplementation follows the shader path. Confidence: high (E-pt5-001).
- **GFX-OBJ-2.6d Builder.** Each work step (ECO-HQ-4.6, four per unit) sets
  the builder's request WorkLoop0 for the construct time (55 ticks) when the
  step's timed action starts, at that step's construction position; between
  steps he walks (fine walk, TRN-FREE-5.5) to the next position. The request
  is per step, not per unit (GFX-ANIM-3.8). Confidence: high (E-pt5-001).
- **GFX-OBJ-2.4 Trees (tree_shader).** Vertices sway: `s = saturate(|p|^2 x
  0.003) x m22` (p = vertex in model space, m22 = the instance matrix's y
  scale, so only the tops move), `x += s x sin(0.35 pi x Tx + t)`, `z += s x
  cos(0.35 pi x Tz + t)` with (Tx, Tz) the instance translation and t the
  frame clock in seconds. Light x the mesh's vertex colour; colour texture;
  alpha test 127. Confidence: high.
  **Precisely (F53, 2026-10-31, E-gfx-036, `tree_shader.fx`):** the vertex colour c
  multiplies only the direct term, per vertex: D = ambient + saturate(N . L) x light x c
  x s (s the shadow factor of GFX-VIEW-2.3d), then pixel = D x texel. A dark vertex
  colour still shows the texture lit by the full ambient; multiplying the whole D by c
  is wrong (GFX-VIEW T32). The normal pass draws the needles and leaves with the alpha
  test (texel alpha above 127 of 255 drawn, the rest discarded), depth write on and no
  blending. The shadow pass cuts at alpha 0.5 the same way.
- **GFX-OBJ-2.5 Doodads, rocks, resource signs (doodads_shader,
  rock_shader, signs_shader).** Plain lit texture with alpha test 127 (no
  vertex colour). The sign and rock variants add the underwater haze: `a =
  saturate((y + 7.05) / 7)` with y the world height, `haze = lerp((0, 0, 0.05),
  (0, 0.8, 0.3), a - 0.3)`, `result = lerp(haze, result, a)`. Confidence: high.
- **GFX-OBJ-2.6 Stones (stone_shader).** Per-pixel lighting with the
  interpolated normal, no alpha test, plus a gloss: `S = pow(saturate(reflect
  . view), 16)`, `result += S x tex.a x tex.rgb`, where reflect = normalize(2 x
  D x N + lightDir) (D the lit colour; the shader uses it as written) and view
  the vector to the camera. Confidence: high (formula as shipped).
- **GFX-OBJ-2.7 Wheat (weed_shader).** The field mesh is drawn two-sided with
  alpha test and blending; its colour = `lerp((0.5, 8, 0.2), white, m22)`
  with m22 the instance's y scale (young fields are strongly green),
  multiplied into `(lightColour - ambient) x shadow x texture`. Confidence:
  high.
- **GFX-OBJ-2.8 Items, animals, donkey, ships (item_shader, animals,
  donkey, ships).** Lit colour texture; items alpha test 127, animals none.
  Ships: `C = tex x D`; `P = C x lerp(1, playerColour x 1.8, C.a)`; `result =
  lerp(C, P, playerColourWeight)` (the weight is set per part of the ship,
  2.8a; the construction frame pass is rigid, the ship pass skinned).
  Confidence: high.
- **GFX-OBJ-2.8a Which parts of a ship take the player colour (2026-11-15,
  Q-182 e; E-gfx-070, code reading and the shipped models, textures and
  shader).** Only the sails and the masthead banner take the owner's colour,
  and only as a tint. The hull, deck, oars, rudder and mast never do.
  (a) **Two parts.** Each tribe's ship model (`PV_R_Ship01.GR2`,
  `PV_N_Ship01.GR2`, `PV_C_Ship01.GR2`) has two material groups, in the
  file's order:
  - **part 0**: the hull with deck, oars, rudder and mast, textured with
    `PV_<t>_Ship01.dds` (512 x 512, DXT1: opaque, or fully transparent where
    unused);
  - **part 1**: the sails and the banner, textured with
    `PV_<t>_Ship01PC.dds` (256 x 256, DXT3 with a real alpha channel).

  The materials are `15 - Default` / `16 - Default` (Roman), `11 - Default`
  / `assa` (African) and `ship` / `sail` (Chinese). Part 0 has 1972 / 2656 /
  1602 triangles, part 1 has 462 / 156 / 432.
  (b) **The weight is the part's number.** The ship is drawn part by part,
  each part with its own texture. Before a part is drawn,
  `playerColourWeight` is set to the part's index as a number: 0 for the
  hull, 1 for the sails. So:
  - **part 0**: rgb = tex x D, with no player colour at all. Alpha = tex.a
    with the alpha test at 127, so the unused (transparent) texels are
    discarded.
  - **part 1**: rgb = tex x D x lerp(1, 1.8 x playerColour, tex.a), alpha 1
    (nothing discarded).

  D is the lit term of 2.8: ambient + light colour x saturate(N . -L) x the
  shadow factor. The colour is the owner's player colour (2.11).
  (c) **The tint is partial.** The sail textures' alpha is never 1:
  - Roman: at most 102/255 (0.40) on the sail's middle band, 51/255 (0.20)
    on most of the rest;
  - African: mostly 153/255 (0.60);
  - Chinese: mostly 170/255 (0.67).

  A white sail texel of a blue player is therefore pale blue, and a texel
  with alpha 0 (the seams and edges) keeps its own colour.
  (d) **Not coloured either:** the freight and crow's-nest add-ons
  (`PV_Ship_Freight.GR2`, `PV_Ship_Crowsnest.GR2`, texture
  `PV_Ship_Addon.dds`). They are drawn as plain meshes: lit texture, alpha
  test 127, no player colour. Also not coloured: the ship's construction
  frame (`PV_R_Ship01ConstructionSite.GR2`, the frame pass of 2.8) and a
  waterway's buoys (6.2b d). The **border** buoys on water are different
  objects and keep the player colour of 2.10 where their texture's alpha is 0.
  Confidence: high (code reading of the per-part draw and its weight; data
  for the parts, materials, textures and alpha values). Test T48. **Seen
  (2026-11-15 day, E-trace-030, the Seafaring tutorial, the blue player's
  Roman ships Aurora and Bellona close up by day, 2560 x 1440):** hull, deck,
  rails, oars, rudder, mast and the cargo (crates, barrels, sacks) show their
  own wood and cloth colours with no blue at all; the furled sail lying on
  the deck and the cloth hung along both sides are pale blue to lavender,
  never the flags' saturated blue; the crow's nest is plain wood. The border
  buoys on the water next to them are saturated blue on their top with a blue
  flag (2.10), and the land border stones are saturated blue too. That is
  the rule above. Confidence: high (code, data and pictures).
- **GFX-OBJ-2.9 Flags (flag_shader, house_flag).** Cloth wave in the vertex
  shader: `s = 0.33 x saturate(-0.02 + x^2 + z^2)` (model space, so the pole
  at the origin stays), `w = Tx + Tz + x + y + t`, `c = 0.5 + 0.5 cos(4 w)`,
  `z += s c`, `x += s (1 - c)`. Pixel: `rgb = lerp(flagColour x 1.8, tex, tex.a)
  x D`, alpha 1: where the flag texture is transparent the owner's colour (x
  1.8) shows. **t, the shaders' clock, is game time in seconds (the world's
  elapsed time, ECO-TIME-1.4), so it stops while the game is paused and runs 2
  and 3 times as fast at speeds 2 and 3** (corrected 2026-10-18 by live trace 1,
  Q-313; Run D had read the scene's real-time branch, which no caller uses,
  TOOL-trace TRACE-5.2). So a flag waves once every 2 pi / 4 = 1.571 s of game
  time; the tree sway (2.4), the water and the terrain wobble use the same clock.
  Confidence: high (clock measured, E-trace-001: game time and clip clocks frozen
  while paused with frames running, Jens saw flags, trees, water and animals stop
  and move exactly 2x and 3x at speeds 2 and 3; formula E-gfx-016).
- **GFX-OBJ-2.10 Border markers (border_shader).** Instanced by position
  vectors; `rgb = D x lerp(playerColour x 1.8, tex.rgb, tex.a)`, alpha = tex.a
  x fog-of-war alpha, alpha test 127 and blending. Confidence: high.
  **Seen (2026-11-17 night, Q-207 c; E-trace-033, pictures of the original at 2560 x 1440):** a border
  stone is drawn whole: a grey stone foot and a **player-coloured dome** on it (blue, red, green or
  yellow, flat-shaded), standing on the border fields. So the player-coloured part (tex.a 0) is **not**
  cut away: the alpha test of 127 above does not act on the border markers as the game draws them
  (which render state turns it off was not traced). Drawing them without the alpha test, as the flag
  (2.9), gives the seen picture (high for the look).
- **GFX-OBJ-2.11 Player colours.** The eight player colours are set by
  `scripts/menu.cfg`: 0 (0.01, 0.10, 0.60), 1 (0.60, 0, 0), 2 (0.20, 0.50, 0),
  3 (0.75, 0.46, 0.05), 4 (0.50, 0.50, 0.50), 5 (0.09, 0.09, 0.09), 6 (0.45,
  0, 0.35), 7 (0.20, 0.60, 0.60). Flags, border markers, ships and settlers
  use them (x 1.8 in the flag / border / ship formulas). Confidence: high.

### GFX-OBJ-3 Settler colouring (Q-176d)

- **GFX-OBJ-3.1 Colour slots.** `character/color_slots.xml`: `SlotEnumeration`
  lists slot names; `ColorRamps` defines a 16-step ramp per slot either as
  `solid` (one colour, 16 equal steps), as a `bright` and `dark` pair
  (gradient), as a `texture` whose rows (16 texels each; `rows` = how many,
  default the image height) are alternative ramps (hair 16 rows, Roman skin 6,
  others 16 or 1), or as 16 child `value` elements; `ColorMaps` name a map and
  assign a slot to each material index 1 .. 16. Confidence: high.
- **GFX-OBJ-3.2 Atlas.** For each colour map the renderer builds a 16 x 16
  block (row = material index, column = luminance step) in a 1024 x 1024 atlas,
  one block per ramp-row variant; a character is given the block offset
  (`TEXTURE_ATLAS_OFFSET`) and the block scale (`COLOR_INDEX_SCALE`).
  Confidence: medium (the block layout follows from the shader and the reader;
  the variant choice per settler was not traced).
  **How a settler's block is made (2026-11-13, Q-180 d, e; E-gfx-062, code reading).**
  (1) *Column order.* The stored ramps run from bright (texel 0) to dark (texel 15);
  column c of a block row holds ramp step 15 - c, so the pixel rule's u (bright
  pixels at high u) reads the bright end. Confidence: high.
  (2) *Rows.* The colour-index maps' red byte is the colour map's entry number:
  red 1 and 2 the eyes, red 3 the hair, red 4 the jacket (the player-colour part,
  alpha 0; its slot `Jacket` is the player colour ramp), red 5 face and hands
  (`Skin`), and so on; the soldiers' maps use red 16 for their `Black` entry. The
  block's row r therefore holds the slot of entry r. Confidence: medium (from the
  data and the original's picture of a woodcutter; the code's row order was not
  read to the end).
  (3) *Variants.* Each new settler figure gets its own block. For the 16 entries
  of its colour map in order (entry 1 first), the program draws one random
  number per entry, also for a slot with a single ramp row, and takes ramp row
  `draw mod rows` of that slot (`rows` as in 3.1: hair 16, Roman skin 6, `Yellow`
  and `Red` 8, `EyeOuter`, `_empty_` and `White_Cloth` 1). The entries are
  independent: hair, skin, clothes and hat each vary on their own. The numbers
  come from one generator of the view, shared by all settler figures of the
  program run and never touched by the simulation: the "minimal standard"
  generator, state = 16807 x state mod (2^31 - 1), the draw being the new state,
  seeded once with 1412607200 when the first figure is made. A block equal to
  one already in the atlas shares that place. Because the draws follow the order
  in which the view makes figures, an implementation need only reproduce the
  rule (independent, uniform rows per entry), not the original's exact sequence.
  Confidence: high for the rule and the generator, medium for which events make
  a new figure (a settler that leaves the view and comes back may be drawn
  again). Test T42.
  (3a) *The atlas's places (2026-11-14, Q-1043; E-gfx-067, code reading).* Each block in the atlas
  has a **use count**. A new figure whose block equals one already placed shares it (count + 1).
  When a figure is removed it gives its block back (count - 1); at 0 the block's place is freed
  and can be taken again by a later block. Places are handed out on a 64 x 64 grid of 16 x 16
  places, the first free place in row order (left to right, top row first), so a freed place is
  the next one taken. **When no place is free** the block gets none: the figure is still made,
  its block is not given back when it is removed, an equal block made later tries again for a
  place, and the figure's draw looks up the place it does not have without any check, so what
  it shows is not defined (the original has no rule for a full atlas; it fills only with more
  than 4096 different blocks in use at once). An implementation may choose any visible fallback
  for that case. **The atlas is new at every game start** (it is made with the game's view, which
  is set up for each new or loaded game), so it starts empty; the generator of (3) is not reset:
  it is seeded once per program run and goes on from game to game. Confidence: high for the use
  count, the freeing, the grid order and the missing check (code); medium for the atlas being
  made anew for each game (the view's set-up is made at every game start; its removal when a game
  is left was not read). Test T45.
- **GFX-OBJ-3.3 Pixel rule (skin_color_mesh).** The character's `color` map
  (`*_colorindices_luminance.tga`, point sampled) gives per texel: red =
  material row, green = luminance, blue = specular mask, alpha = 0 where the
  player colour applies. With the scalar `D = saturate(N . L) x lum(light) x
  shadow + lum(ambient)` (corrected Run A, E-gfx-011: lum = 0.299 r + 0.587 g
  + 0.114 b, so lum(light) = 0.8965 and lum(ambient) = 0.5413 in the shipped
  data; `shadow` is GFX-VIEW-2.3's factor, 0.1 in a shadow; a settler on flat
  open ground has D = 1.228, in a shadow 0.610):
  `u = saturate(green x D) x scale.x`, `v = red x scale.y`, plus the atlas
  offset and half a texel; `colour = atlas(u, v)`; `colour = lerp(playerColour x
  green x D x 2, colour, alpha)`; `colour += pow(saturate(reflect . view), 16) x
  blue`; x fog-of-war; output alpha = alpha. So the palette already contains the
  shading (the luminance axis is lit luminance). Confidence: high.
  **The terms exactly (2026-11-12 night, Q-180 f; E-gfx-061, `skin_color_mesh.fx`).** (1) The
  diffuse dot takes the interpolated vertex normal **as it is** (not renormalised):
  saturate(N_i . -L) x lum(light), times the shadow factor whenever the frame draws shadows
  (the 4-tap test of `GFX-world-view.md` GFX-VIEW-2.5, or the one-tap low variant at shader detail
  2), then + lum(ambient). (2) The highlight is **not** a mirror reflection of the light: with N
  the renormalised normal, L the light's travel direction (pointing down) and D the scalar of (1),
  R = normalize(2 x D x N + L), S = pow(saturate(R . V), 16) with V the unit vector from the
  pixel to the camera, and the colour becomes saturate(colour + S x blue) (clamped before the
  fog of war). Because D includes the ambient and the shadow (1.228 on flat lit ground, 0.610 in
  a shadow) instead of N . -L, the highlight sits elsewhere than a reflect(L, N) highlight (on flat lit
  ground seen from above it is 24 times stronger) and nearly vanishes in a shadow. (3) The atlas lookup adds half a texel of the atlas to (u, v) after
  the block offset. Test T41.
- **GFX-OBJ-3.4 Carried goods.** Superseded by GFX-OBJ-7 (Q-192 b, Q-207 a). The
  earlier reading (the ware turned by `goodRotation` and shifted by `goodOffset` at
  the hand) was wrong: those two values apply only to a ware lying in a carrier's
  boat (GFX-OBJ-7.6).

### GFX-OBJ-4 Build help icons and cursor sprites: see `GFX-world-view.md` GFX-VIEW-6

### GFX-OBJ-5 Wares at flags (Q-176f)

- **GFX-OBJ-5.1 Stacks.** A flag shows its wares in up to 6 stacks, one per
  good type, each holding up to 6 wares. A ware joins the stack of its type or
  the first free stack. Stack i (0 .. 5) stands at angle i x 60 degrees around
  the flag at radius r = (depth of the good's mesh bounding box along z) +
  `distance` (`StackLayouts`, 0.5): base = flag position + (sin a x r, 0, cos a
  x r), at the ground height there. Confidence: high.
- **GFX-OBJ-5.2 Layout.** The k-th ware of a stack (k = 0 .. 5) sits at base +
  rotate(a) x (offset_k x bounding box extent), with offset_k from the good's
  `Layout` in `items.xml`: layout A (goods Wood, Stone, Corn, Iron): (-1, 0, 0),
  (0, 0, 0), (1, 0, 0), (-0.5, 1, 0), (0.5, 1, 0), (0, 2, 0) (a pyramid: three
  on the ground, two above, one on top); layout B (Water, Flour, Pig, Coal,
  IronOre, Gold): (-0.5, 0, -1), (0.5, 0, -1), (-0.5, 0, 0), (0.5, 0, 0),
  (-0.5, 0, 1), (0.5, 0, 1) (two by three on the ground). Goods in no layout
  use layout A (medium). The ware's own heading is the stack angle. When a ware
  leaves, the later wares of the stack move down one slot; an empty stack is
  freed. Confidence: high (positions), medium (default layout, heading).

### GFX-OBJ-6 Roads (street renderer)

- **GFX-OBJ-6.1 Settings.** `graphics.xml` `<Street>`: numSegments 8, width
  1.0, texStretch 0.25, crossingRadius 2.25, buoyDistance 1.7, textures
  `simple_street.dds` / `street.dds` (with `_n` normal maps), crossing textures
  `simple_crossing.dds` / `crossing.dds`, buoy texture `street_buoy.dds`,
  shader `street_shader.fx`. Confidence: high.
- **GFX-OBJ-6.2 Geometry.** A road runs through its field points (flag to
  flag); each step between two points is split into numSegments pieces (so a
  road of n points has (n - 1) x 8 + 1 samples), with the tangent at a point
  taken toward the next point (one-sided at the ends); the strip is `width`
  wide, its texture v advances by texStretch per world unit; at the end flags a
  crossing disc of crossingRadius uses the crossing texture; a road over water
  keeps a fixed height (the water level) and is drawn as pairs of buoys,
  one pair per step (6.2b). **Texture (play test 10, E-gfx-017):** a road
  that has been upgraded to a donkey road is drawn with texture 1
  (`street.dds` / `street_n.dds`), every other land road with texture 0
  (`simple_street.dds` / `simple_street_n.dds`); donkey roads are drawn
  first. **When a road turns into a donkey road (Q-221, E-gfx-024):** at the moment the road is
  marked busy and asks for a donkey (TRN-CARRY-6.2), before any donkey has
  been sent or arrived, the road's visual moves from the plain list to the
  donkey list (texture 1 from the next frame), and each of its two end
  flags' crossing discs moves to crossing texture 1 (`crossing.dds`) if it
  is not there already. The mark is never cleared, so a road never goes
  back to texture 0. The flag-to-door strip (6.4) has no road and is always
  texture 0. **`width` is the half width**: the strip reaches `width` to each
  side of the centre line, 2.0 units in all (read for the flag-to-door strip,
  6.4; the general road mesh builder uses the same value, E-gfx-029).
  Confidence: high (texture choice, width).
- **GFX-OBJ-6.2a Road curve and tessellation (Q-092; E-gfx-029).** A land
  road with field points P0 .. Pn-1 (the field sample positions with their
  heights, GFX-VIEW-4.3) and terrain normals N0 .. Nn-1 is a smooth curve,
  not a polyline:
  (a) **Point tangents.** T_i = normalise(P_i+1 - P_i-1) for inner points,
  normalise(P1 - P0) at the first and normalise(Pn-1 - Pn-2) at the last
  (3D, heights included).
  (b) **Step lengths, two passes.** First L_i = the length of the vector
  that gave T_i (|P_i+1 - P_i-1| inside, the one-step chord at the ends).
  Then, for every step i, L_i is replaced by an arc estimate of that step's
  curve (c) evaluated with the first-pass L_i: the sum of the 31 chord
  lengths between the curve points at t = 0, 1/32, ..., 31/32 (the last
  1/32 is not counted). The last point's L is 0 (it has no step).
  (c) **Curve of step i** (P_i to P_i+1), 0 <= t <= 1: cubic Hermite
  P(t) = h0 P_i + h1 P_i+1 + h2 m0 + h3 m1 with h0 = 2t^3 - 3t^2 + 1,
  h1 = -2t^3 + 3t^2, h2 = t^3 - 2t^2 + t, h3 = t^3 - t^2, and both
  end tangents scaled by the same factor: m0 = T_i x 1.3 L_i, m1 = T_i+1 x
  1.3 L_i (L_i of b). The direction D(t) is the normalised derivative
  (exactly T_i at t = 0 and T_i+1 at t = 1); the normal N(t) = N_i + t
  (N_i+1 - N_i), linear and not normalised. At t = 0 and t = 1 the point,
  normal and direction are the field's own.
  (d) **Samples.** Each step is sampled at t = k / numSegments, k = 1 ..
  8, after a first sample at t = 0 of step 0 (so (n - 1) x 8 + 1 samples;
  the point at t = 1 of a step is the next step's start). **At an end flag
  where more than one road ends** (the flag's crossing holds at least two
  roads) the road stops one segment short: it starts at t = 1/8 of the
  first step, or ends at t = 7/8 of the last, so the crossing disc of
  6.2 covers the flag itself; at a flag with only this road it runs to the
  flag point. **When the count is taken (2026-11-14 b, Q-1048; E-gfx-069, code reading):** the
  test uses the number of roads attached to that flag's crossing **at the moment this road's strip
  is built**, this road included. A road's strip is built when the road is made (and on a loaded
  game when its visual is made) and again each time one of its two ends is attached to a crossing,
  first the start, then the end; nothing else rebuilds it. Attaching another road to the same flag,
  or removing one, rebuilds only the crossing (6.2c), **not the strips of the roads already there**.
  So at each flag the first road attached to it keeps running to the flag point, every road attached
  after it stops one segment short, and a strip keeps its shape when later roads come or go (a flag
  left with one road can show it stopping short; a road becomes "first" only if it is rebuilt while
  alone at that flag). In a loaded game the roads are made in the save's road order, so the first of
  them at a flag runs to it. With the first road's 2-unit-wide end on the flag point, the junction
  reads as one worn area even where the flat crossing is hidden under the ground (6.2d a). Test T47.
  **Seen in the original (2026-11-15 night, part 2, E-trace-029, zoom 1 at 2560 x 1440):** a flag F made
  at the end of a dirt road R1, then R2 from F to a new flag and R3 from F to a flag on a paved road;
  the three strips meet at F in one worn area; after R1 was destroyed through the road menu, the
  picture changed only along R1's own course: around F the strips of R2 and R3 are pixel for pixel
  unchanged (not rebuilt), as (d) says. Whether R1 ran to F's point could not be seen (a building hid
  that side of F). Confidence for the "not rebuilt" part: high (a picture difference).
  (e) **Strip.** At each sample the side vector is S = N(t) x D(t) (not
  normalised again: on a slope across the road it is a little shorter than
  1), the edges are centre -/+ S x width. Each pair of consecutive samples
  makes one quad that is draped on the terrain exactly as the flag-to-door
  strip (6.4 b): the terrain triangles under it, clipped to the quad's four
  edge planes, no lift. Texture coordinates per vertex p of a piece
  (centre samples A then B, sides S_A, S_B): u = clamp(((p - A) / width .
  (S_A + S_B) / 2 + 1) / 2, 0, 1); v = V_A + f x |B - A| x texStretch,
  f the fraction of the way from A's edge line to B's, V the running sum
  of |B - A| x texStretch from the road's first sample (v 0 there); the
  vertex tangent is (S_A + S_B) / 2. So the texture runs continuously
  along the whole road at 4 units per repeat; u = 0 is the S side.
  (f) Waterways use the same curve with the water level as every point's
  height and are drawn as buoys only (6.2b).
  Confidence: high for (a) to (e) (E-gfx-029 for the
  crossing's road count); test values from the investigation's road curve
  script (T27b, T27c).
- **GFX-OBJ-6.2b Waterway buoys (Q-229 a; E-sim-002).** A road over water
  (a waterway) draws no strip and no quads at all: its visual is a set of
  buoys built once from the road's field points.
  (a) **Curve.** The same curve as 6.2a (a) to (c) over the road's points
  P0 .. Pn-1 (both flags included), with every point's height replaced by
  the water level (GFX-WATER-1.x) before the tangents, the step lengths and
  the curve are computed, so the curve is flat and horizontal, and with the
  normal N(t) = (0, 1, 0) at every point (no terrain normals).
  (b) **Placement.** For every step i = 0 .. n-2 (no step is skipped and
  none is shortened at a crossing, unlike 6.2a d) take the curve point P and
  the normalised direction D at t = 0.5 and the side vector S = N x D =
  (D.z, 0, -D.x). The step gets two buoys, at P + S x buoyDistance (1.7)
  and at P - S x buoyDistance, in that order. Nothing is placed at the
  flags or at the field points themselves.
  (c) **Model.** Each buoy is `data/map/objects/street_buoy.GR2` (loaded
  once, GR2 read as GFX-OBJ-1.x) copied vertex by vertex with the position
  moved by the buoy's place and everything else (normal, colour, texture
  coordinates, tangent) unchanged: the buoy is not turned along the step,
  not scaled, and does not bob (the border buoys of TER-7.3 bob; these do
  not: the mesh is static and its shader adds no time term).
  (d) **Shading.** `street_shader.fx` pass `RenderBuoys`: lit by the
  vertex normal against the main light (diffuse only, no normal map, no
  shadow map), times the buoy texture (`street_buoy.dds`), then the fog of
  war; depth test and depth write on, no alpha blending, no alpha test.
  Confidence: high for (a) to (d) (E-sim-002 model load and
  construction, placement, curve point with the water
  level and the up normal, tangents and lengths at
  the water level, draw of the water road list, road
  visual with the water level); the buoy pass from `street_shader.fx`.
  Test: a straight waterway of 3 points along +x at water level w, points
  (0, w, 0), (1.5, w, 0), (3, w, 0): D = (1, 0, 0), S = (0, 0, -1); buoys at
  (0.75, w, -1.7), (0.75, w, 1.7), (2.25, w, -1.7), (2.25, w, 1.7).
  **Seen (2026-11-15 night, part 2, E-trace-029, waterways at zoom 1 and 2, 2560 x 1440):** no strip on
  the water or the shore; brown-orange buoys in pairs on both sides of the course, the carrier standing
  on a log raft between them; the waterway's flag stands on the shore with its land roads.
- **GFX-OBJ-6.2c The crossing at a flag (2026-11-12 night, Q-183 a; E-gfx-061, code reading).**
  The crossing is not a round disc but a **hexagon of 6 triangles** around the flag (7 vertices):
  the centre at the flag's own sample point (its height), and six corners at radius
  crossingRadius (2.25) in the directions (sin a, 0, cos a) for a = 60, 120, 180, 240, 300, 360
  degrees, in that order, so two corners lie on the flag's z line (straight up and down the
  screen) and the six road directions of MAP-GRID (30, 90, 150, ... degrees) meet the middles of
  the hexagon's sides. Each corner is put on the terrain: its height is the plane of the terrain
  triangle under it (`GFX-world-view.md` GFX-VIEW-1.4) and its normal that triangle's normal;
  only a corner whose side holds a waterway keeps the flag's height. **Which corner (2026-11-13,
  E-gfx-065, code reading; was medium):** the flag's six road places are the map directions k =
  0 .. 5 of MAP-GRID (0 east, 1 south-east, 2 south-west, 3 west, 4 north-west, 5 north-east), at
  the angles 90 + 60 k degrees of the (sin a, 0, cos a) convention (world +z is north,
  GFX-VIEW-1.1), so the road in direction k leaves through the hexagon side from corner k (at 60
  (k + 1) degrees) to corner k + 1. Corner k (k = 0 .. 5 in the order above) is **not** put on the
  terrain when the road attached in direction k is a waterway (a road over water, drawn as buoys,
  6.2b): it keeps its reset place, the flag's centre point moved by 2.25 in its
  direction, so its height is the flag's own sample height; and its normal is not computed but
  left as the corner before it got (corner k - 1's terrain normal, or that corner's inherited one;
  for corner 0 nothing sets it: an implementation takes (0, 1, 0)). Only that one corner of the
  waterway's side is affected; the other corner of the side (k + 1) is placed on the terrain unless
  its own direction holds a waterway. A land road changes nothing. The uv stays planar (from the
  corner's x and z, which do not change). Test T44. The centre's
  normal is (0, 1, 0). Triangles (centre, corner k + 1, corner k) for k = 1 .. 6, wrapping. The
  crossing texture is laid **flat in world x and z** over the square around the flag: u = (dx /
  2.25 + 1) / 2, v = (dz / 2.25 + 1) / 2, dx and dz the vertex's offset from the flag (the centre
  0.5, 0.5; the corner at a = 180 degrees (0.5, 0); the one at 60 degrees (0.933, 0.75)); so the
  hexagon shows the texture's middle and cuts off its four corners and the strips beside the
  corners at 0 and 180 degrees. Tangent (1, 0, 0) on every vertex, vertex colour white; the
  shading is 6.3's. The mesh is built when the crossing is made and again whenever a road is
  attached to or removed from the flag. Test T40. Confidence: high (the crossing's mesh builder,
  its corner function and the angle table set at start-up).
- **GFX-OBJ-6.2d What lies at a flag, as the original shows it (2026-11-14 b, F58, F60; E-trace-026,
  pictures of the running original at 2560 x 1440, zoom 1 to 3, by day).** The pictures agree with
  6.2 to 6.2c and 6.4; no rule there changes. What they show, for an implementation to compare with:
  (a) **Dirt roads (texture 0).** At a flag with one, two or three dirt roads the worn surface is
  **continuous up to the flag**: no bare ground shows around the pole. Where two or more roads meet,
  the first road attached to the flag runs to its point and the later ones stop one segment short
  (6.2a d, "When the count is taken"); the crossing's soft dirt patch adds to it where it is not
  hidden under the ground (6.3): the
  plain crossing texture (`simple_crossing.dds`) holds a roughly round patch over about the middle
  half of its square, i.e. about 1.1 units around the flag, with an alpha of up to 0.6 like the road
  strip's centre (6.4 d), so the junction reads as one trodden area of the road's own colour.
  (b) **Donkey roads (texture 1).** The crossing is the stone texture (`crossing.dds`): an opaque,
  irregular, roughly round blob of large flat stones about 3 units across (its stones lie within
  about 1.5 units of the flag, inside the hexagon's inner radius of 1.95), the rest of the square
  transparent. So **no hexagon outline shows**: the hexagon only carries the texture. The blob is the
  **same light stone colour as the cobbled road** (measured on one picture: stones of the crossing
  about (194, 167, 145), of the road beside it about (182, 156, 136); the road shows more dark joints
  between smaller stones). At the headquarters' flag, where four donkey roads meet, the roads and the
  crossing merge into one cobbled area.
  (c) **A flag with dirt and donkey roads** gets the stone blob as soon as one of its roads is a donkey
  road (6.2); a dirt road ends at the blob's edge.
  (d) **Between a building's flag and its door** lies the flag-to-door strip of 6.4: a faint
  semi-transparent band of trodden earth (texture 0, also when the flag's roads are cobbled) from the
  flag to the building; at the headquarters most of it is under the building's own paved base.
  Test T46. Confidence: high (pictures; the textures' alpha measured from the files).
- **GFX-OBJ-6.3 Shading.** Lit per pixel with the normal map in tangent space,
  colour texture, fog of war; drawn with alpha blending and no depth write
  (so it lies on the terrain); buoys are lit rigid meshes. Confidence: high.
  **Depth (2026-11-14 b, Q-1046; E-gfx-068, the effect files):** the street pass (road strips,
  flag-to-door strips and crossings alike) tests depth with less-or-equal and writes none, blends
  source alpha over inverse source alpha, and has no alpha test; there is **no depth bias** (none in
  the effect, none set by the code) and **no lift** (the vertex shader passes the world position
  unchanged). The strips are draped on the ground triangles (6.2a e, 6.4 b) and so pass the test
  everywhere; the crossing's six triangles are flat (6.2c), so where the fine ground rises above a
  triangle between its corners that part fails the test and is **hidden**, in the original as in an
  implementation of this rule. Do not drape or lift the crossing. Confidence: high.

- **GFX-OBJ-6.4 Flag-to-door strip (play test 10, F14, OBS-32; E-gfx-017).**
  Every building with a door flag (a site from its placement, a finished
  building, the headquarters, military buildings, harbours: there is no type
  or state test) has a worn earth strip that is a **plain road segment** from
  its flag to the building, drawn by the road renderer together with the
  ordinary roads. (The earlier reading "no path object joins door and flag"
  is withdrawn.)
  (a) **End points.** P0 = the flag field's point, P1 = the building field's
  point (the field sample positions with their heights, not the door), so on
  flat ground the strip is 6.0 units long (the flag is the building's
  neighbour toward world (+3, -5.196), GFX-VIEW-4.3) and its far half lies
  under the house, which hides it; what shows runs from the flag to the
  front wall and door.
  (b) **Shape.** Direction d = normalised P1 - P0 (in 3D). At each end the
  side vector is s = n x d with n that field's terrain normal (`MAP` normal of
  GFX-VIEW-2.2); the four corners are P0 -/+ s0 x w and P1 -/+ s1 x w with
  w = `Street width` = 1.0, so the strip is **2.0 units wide**. The mesh is the
  terrain surface itself cut to that quad (the terrain triangles clipped by
  the four planes through its edges that contain the end normals): it lies
  exactly on the ground, with no lift.
  (c) **Texture coordinates.** u runs across: u = clamp((((p - P0) / w) .
  t + 1) / 2, 0, 1) with t = (s0 + s1) / 2, so u = 0 on the P0 - s side and 1 on
  the P0 + s side; v runs along: v = f x |P1 - P0| x texStretch, f the point's
  fraction of the way from the P0 end line to the P1 end line, so v = 0 at the
  flag and 1.5 at the building point on flat ground (texStretch 0.25, 6.4
  shows a quarter of the texture per unit). Sampler: u clamped, v wrapped,
  trilinear. The tangent is t (along u).
  (d) **Look.** Texture 0 (`simple_street.dds`, normal map
  `simple_street_n.dds`), the plain-road texture: its alpha is 0 in the outer
  3/32 of u on each side, rises over about 6/32 to a half-transparent centre
  (alpha about 150 of 255, 0.59), so the strip is a soft-edged, partly
  see-through band of trodden earth (centre colour about (86, 71, 49)) over the
  terrain, not an opaque slab. No crossing disc is added for the strip (the
  flag's own crossing belongs to the flag).
  (e) **Light.** As every road (6.3): per pixel, N from the normal map in the
  frame (t, t x n, n) of the vertex; D = ambient + light colour x saturate(N .
  L) x shadow (2.3, 2.5, shadows received); colour = D x texel, alpha =
  texel alpha, x fog-of-war factor; blended over the terrain (source alpha,
  inverse source alpha), depth test on, no depth write.
  (f) **Lifetime.** Made when the building's view object is made (the
  building is placed or loaded) if its door flag exists, otherwise in the
  first frame in which the flag exists; removed with the view object.
  Drawn when its box is in view. Confidence: high (E-gfx-017;
  `street_shader.fx`, `graphics.xml` `<Street>`).

### GFX-OBJ-8 Building decorations: soldier flags, status icons, upgrades (Run B, F13 to F17; E-gfx-012)

- **GFX-OBJ-8.1 Soldier flag (F13).** Every finished building whose type holds
  soldiers (both soldier counts of the type at least 1: the military
  buildings) and the headquarters carry one flag pole, `MilitaryFlag1.GR2`
  with `MilitaryFlag1.dds` and `house_flag.fx`. The file holds six meshes
  named `0` to `5`: `0` the pole with its finial, `1` to `3` three
  swallow-tailed banners near the top (largest at the top), `4` and `5` two
  small pennants lower on the pole. The flag shows meshes `0` to `n`
  (numeric order of the names, like the site pieces of 2.6a) with
  **n = the number of soldiers inside the building, at most 5**; the
  headquarters always shows n = 5. A soldier counts as inside when the
  building is his home, he stands (no step under way) on the building's
  field and has no order pending (not leaving, not fighting). So an empty
  barracks shows the bare pole, one soldier adds the top banner, and so on.
  The count is read every frame; there is no animation of a banner
  appearing.
  (a) **Place.** In the building's own frame (before the heading of
  `GFX-world-view.md` GFX-VIEW-4.3): x = box centre x + 0.33 x box width,
  y = 0 (the building's base), z = box centre z - 0.55 x box depth, where
  the box is the bounding box of the type's house model (the `House` mesh
  of the `...LOD0.GR2` file when there is one, else of the main file). This
  is the front right corner, just outside the front face. The flag is turned
  by 90 degrees about +y (the pole model's +x, the direction its banners
  fly, turns to the building's -z, its front), then the building's world
  matrix (heading -30 degrees and position) applies. Scale 1.0.
  (b) **Colour.** The flag colour is the owner's player colour (GFX-OBJ-2.11);
  pixel = lerp(flagColour x 1.8, texel, texel alpha) x light (as the road
  flags, 2.9); lit with shadow; fog of war applies. Texture offset 0.
  (c) **Wave.** In the vertex shader, per vertex of the model in its own
  frame: strength = 0.33 x saturate(x^2 + z^2 - 0.02) (zero on the pole's
  axis, growing outward along the banner), w = Tx + Tz + x + y + clock
  (T = the flag's world translation; clock = real seconds, 2.9), c = 0.5 +
  0.5 cos(4 w); z += strength x c and x += strength x (1 - c). Its shadow
  uses the same displacement. Confidence: high (rule, place, wave, and the
  mesh order: the flag model is loaded through the same numbered merge as
  the site models, Run D, E-gfx-016).
- **GFX-OBJ-8.2 Border-distance flag.** Military buildings (not the
  headquarters) that have been occupied (they claim land, TER-2.1) carry a
  second pole, `MilitaryFlag2.GR2` / `MilitaryFlag2.dds`, at the front
  left corner: x = box centre x - 0.33 x box width, same y, z and turn as
  8.1. Its texture is shifted down by i / 3 of its height, i chosen from the
  building's enemy distance n (`MIL-soldiers.md` MIL-SOL-3.1, 3.2): i = 0
  far (n > 24), 1 medium (19..24), 2 near (n <= 18, also while n is still
  -1). Same player colour, lighting and wave as 8.1. Confidence: high.
- **GFX-OBJ-8.3 Status icons (F16).** A finished building of the local
  player that is not sinking shows a floating icon effect above it while it
  has a reason to idle; the effects are particle scripts
  `data/scripts/effects/<name>.pfx` drawing one icon of
  `data/gfx/particles_texture.tga` (512 x 512), started at the centre of the
  building's box (moved with the building's matrix):
  | state | effect | icon (texture rectangle, u0 v0 u1 v1) |
  |---|---|---|
  | production building with production on, productivity below 25 % for 10 s | `Sleep` | "Z" (0, 0.3125, 0.0625, 0.375) |
  | production building switched off | `Sleep_ProductionOff` | gear with a red stop mark (0.0625, 0.3125, 0.125, 0.375) |
  | military building, "move soldiers out" on, coins on | `Sleep_NoSoldier` | soldier's head (0.125, 0.3125, 0.1875, 0.375) |
  | military building, coin supply off | `Sleep_NoCoin` | coins with a red stop mark (0.1875, 0.3125, 0.25, 0.375) |
  | military building, both off | `Sleep_NoSoldierAndCoin` | both icons (0.125, 0.3125, 0.25, 0.375) |
  The idle timer counts the game step (frozen while paused; TRACE-5.3, aligned 2026-11-08 b, Q-231 b) while productivity is below 0.25 and
  resets otherwise; the effect is made when the state changes and removed
  when it ends (no state: no effect).
  **Precisely (play test 11, E-ui-011):** each frame,
  for a finished, own, not sinking production building with production on,
  the timer runs while the productivity is **at most 0.25**, and a building
  that has never produced has **no productivity entry, which reads as -1 and
  so counts as idle**; above 0.25 the timer is cleared. The `Sleep` icon is
  up while the timer has reached 10 s and goes in the frame the productivity
  rises above 0.25 (the first product sets 0.5, `ECO-outdoor-workers.md`
  ECO-OUT-3.11). So every new production building shows "Z" from 10 s after
  its view starts updating until its first product; a forester's "product" is
  a finished planting (ECO-OUT-4.7b), so a forester with spots loses the icon
  at his first planting's end and keeps it off while he plants at least once
  per 50 min (the decay from 0.5 to 0.25 takes 5 x 600 ticks); a forester with
  no spot keeps it. Confidence: high. So "coins stopped" is shown by the
  coin-and-stop-mark icon floating over the building, not by a sign model.
  "Coins on" means the coin switch is on and the coin accumulator is set
  (`MIL-coins-promotion.md`; it is -1 until the building's first enemy
  distance, at most 11 ticks after it is occupied), so a new military
  building briefly shows `Sleep_NoCoin`.
  The five files differ only in the icon rectangle; the motion is 8.3a
  (`Sleep_NoSoldierAndCoin` stores v1 = 0.4125, not 0.375, so its picture
  reaches 0.0375 lower in the texture). Confidence: high.
- **GFX-OBJ-8.3a Particle effects (`.pfx`, Run D, Q-213 a).** A `.pfx` is
  little-endian: a string (u32 length + bytes, no terminator) "0002"; the
  emitter class name ("Default") and its values in this order with no
  padding: maxLoops i32, pos, posVar, vel, velVar (3 f32 each), size,
  sizeVar, ttl, ttlVar, spawn, spawnVar (f32 each), color, colorVar (4 f32
  each); the particle class name ("Decal") and its texture rectangle (u0, v0,
  u1, v1); the particle count (i32); the curve set's class name ("Default")
  and four curves, each an i32 type followed by its values; for version
  "0002" a final i32. Curve types: 0 constant (c), 1 linear (start, end):
  start + (end - start) t, 2 eased (start, end): start + (end - start) (1 -
  cos(pi t)) / 2, 3 cubic (a, b, c, d): a + (c - b) t + ((b - a) - K) t^2 + K
  t^3 with K = d - c - b + a (so a at t = 0, c at t = 1), 4 the same with
  tension and bias (6 values), 5 and 6 periodic (offset, phase, scale, bias;
  not used by the icons). The four curves **multiply** a particle's start
  values over its life, t = age / life in 0..1: curve 0 the velocity (each
  axis), 1 the size, 2 the colour's r, g, b, 3 its alpha. A particle's
  position is start + (velocity x curve 0) x age (no integration).
  Particles are a fixed pool of `count`; particle i is born at (i + 1) x
  spawn after the effect starts and is reborn (new random values within
  the variances) each `ttl` later; maxLoops 0 means no end.
  **The five Sleep icons:** vel (0.5, 2.5, 0) +- (0.3, 0.25, 0.1) per s, size
  1.0 +- 0.2, ttl 6 s, spawn 0.8 +- 0.1 s, 4 particles, colour white;
  curves: velocity x (1 - 0.4 t), size x (4 t - 3 t^2 + t^3) (0 at birth, 2
  at the end), colour x 1, alpha x (1 - t^3). So each icon starts at the
  box centre at size 0, grows while it rises 9 units and drifts 1.8 units
  sideways over 6 s, and fades out mostly in its last two seconds; four
  icons rise 0.8 s apart, then 2.8 s pass before the first is reborn. The
  icon is drawn as a quad of the particle's size (the "Decal" particle; its
  facing was not traced, the program's camera-facing quad stands in).
  Confidence: high for the format and the values (E-gfx-016, curve classes), medium for the axes of vel (taken as
  world x, y, z).
  Covers (inventory): *particle effect* from `data/scripts/effects/*`.
- **GFX-OBJ-8.3b Which settlers make particles (2026-11-15 day, for W18 / S2N-006; data).**
  A settler shows a particle effect only where its work clip names one in
  `character/settler_animations.xml` (a `Particle` child, started at its time in the clip
  like the clip's sounds). The shipped file names exactly two:
  - the leveller's shovel clip (`WorkLoop0Bulldozer`) starts `StoneDust` at 0.5 s;
  - the forester's planting clip (`WorkLoop0Forester`) starts `StoneDust` at 0.0 s
    (GFX-ANIM T20).

  Every other settler clip has sounds only, or nothing. That includes the builder's
  hammering (`WorkLoop0Constructor`, also the shipwright's), the woodcutter's chopping
  (`WorkLoop0Axe`), the stonecutter's, the geologist's and the farmer's clips. So the
  original shows **no dust over a construction site while its builder hammers** and **no
  chips where a woodcutter fells**. A site shows dust only while it is being levelled. The
  buildings' own clips (`buildings.xml` `Particle` children: smoke, saw dust, fire, sparks)
  are the rest of the original's particles. Confidence: high (data).
  **GFX-OBJ-8.3c A clip's particle effect, its place and its end (2026-11-16 night, Q-1062; E-gfx-071,
  code reading, and E-trace-031, pictures).**
  - **One effect per play of the clip.** When the settler's model starts the clip, each `Particle`
    entry becomes one event. The event starts its effect once, when the clip's own time (wall-clock
    seconds since the play began x the global animation speed, 1.8) reaches the entry's `start`.
    A loop of the same play does not start it again; a new play (the next work step) starts a new one.
  - **Place.** An entry without a bone (both shipped `StoneDust` entries) puts the effect at the
    model's own origin: the settler's feet. Its place is computed again every frame from the model's
    matrix, so it follows him while the event lives. An entry naming a bone uses that bone's pose.
    Offsets from the entry's attributes are added (none are set).
  - **End.** An entry with a duration stops emitting once start + duration has passed (the shipped
    entries set none). Otherwise the effect emits until its event ends with the clip's play (the
    model starts another clip). The particles already made then live out their own life
    (`StoneDust.pfx`: 3 +- 1 s), so the cloud fades rather than vanishing.
  - **Seen** (the forester, game speed 3, pictures about 35 ticks apart): no cloud while he walks to the
    spot; a grey-white cloud about two fields across around his feet in two pictures 35 ticks apart
    while he plants; no cloud 35 ticks later, the sapling planted and the forester walking off.
  - So a planting (55 ticks, the 10 s clip at speed 1.8) gives one cloud that lasts the planting and
    fades about 3 s after it. The leveller's clip starts its cloud 0.5 s into each of his shovel plays.
  - **Seen, the leveller (2026-11-16 day, E-trace-032, game speed 1, zoom 1 and 2, a medium site on a
    gentle slope):** while he shovels, a translucent grey-violet cloud stands at his feet, about one
    to two fields across, darker at its centre and low over the ground (not above his knees); when he
    walks on to the next point of the site, the cloud stays behind where it was and thins out within
    about two seconds (pictures about 0.7 s apart); walking between points he raises no new cloud.
    This agrees with the rules above (one cloud per shovel play, the made particles living out their
    3 +- 1 s). Confidence: high for the look.
  Confidence: high for the once-per-play start, the place and the end rule (code); medium for "ends with
  the clip's play" (the event's owner, the pooled model entry, was not followed to its release); the
  look agrees. While a military building is
  upgraded (`MIL-building-upgrade.md`) its visual keeps the old house and
  adds the new type's house and the new type's construction-site model, all
  with the building's matrix. With u = the upgrade progress (1 - remaining
  upgrade work, interpolated within the tick like 2.6b) and H = the new
  house's box height, a horizontal plane at base + (H - 0.1) x u / 0.9
  splits the two houses: the **old house is drawn only above** the plane,
  the **new house only below** it, so the new building grows up from the
  ground through the old one, which disappears from the bottom up, and is
  complete in height at u = 0.9. The new type's site model (scaffolding)
  stands around them the whole time. The flags of 8.1 / 8.2 follow the
  type the building has now. Confidence: medium (which mesh is kept on
  which side is read from the plane signs; not compared with the game).
- **GFX-OBJ-8.5 Ground under the headquarters (F14; corrected 2026-10-18,
  Q-221).** The paved stone floor of the Roman headquarters (and the Roman
  fortress) is the paved half of its ground decal (2.3; their `GROUND`
  meshes map v 0.002..0.498); turned with the building (-30 degrees) its
  front edge faces the flag. Between the flag and the door lies the
  flag-to-door strip of 6.4 (a plain road segment, texture 0, 2.0 units
  wide; it is drawn before the decal, so where the two overlap the decal
  is mixed on top of it, GFX-VIEW-2.7); the walk in is the logic step of GFX-VIEW-4.3
  (e). (The earlier sentence "no separate path object joins a building's
  door and its flag" is withdrawn.) Confidence: high (E-gfx-017, 6.4).
- **GFX-OBJ-8.6 A soldier leaving (F15).** A soldier leaves like any
  settler: his first step goes from the building field to the flag
  (direction 1, heading 150 degrees, GFX-VIEW-4.1) in a straight line,
  which with the -30 degree heading runs straight out of the door; the door
  opens at the start of the step (leave distance 0), plays its open clip
  and closes 0.6 s later (`GFX-animations.md` door rules); he plays Walk.
  Confidence: high.
- **GFX-OBJ-8.7 A finished building's collapse and its ruin (2026-11-12; E-gfx-057, code
  reading and data).** When a **finished** building leaves the world (burnt, destroyed by a
  catapult, on lost land, torn down; not a site, not an upgrade or a capture, which change the
  building in place, 2.2a (4)), its house view does not vanish but collapses, and where the
  simulation leaves remains (`MIL-conquest.md` MIL-CON-4.1, 4.1a) a ruin view appears in the
  same tick. In the removal's frame:
  1. **Gone at once:** the ground decal (2.3), the status icon (8.3) and the soldier and
     border-distance flags (8.1, 8.2). The house's state updates stop as well (no new door
     opening, door sound or work clip is started).
  2. **The house sinks.** The house mesh stays, with its slot's colour column (2.2a) and its
     light, and is lowered every frame: collapse clock c starts at 0 and grows by 0.35 x the
     frame's time in seconds, clamped at 1; the house's base height is the ground height at its
     position, read again each frame, minus H x c^2, H = the house model's box height (top minus
     bottom, as 2.6b). So it goes slowly at first and fastest at the end: a quarter of H after
     1.43 s, all of H after 2.857 s, when its top is at the ground. The view is removed (and its
     slot freed, 2.2a (3 b)) at the first view update after c reaches 1.
     **Its pose (2026-11-12 night, E-gfx-060, code reading).** The house keeps animating: every
     frame, collapsing or not, the view sets its model's clock to the frame time, samples the
     clips on the model and builds the bone pose from them, and only then is drawn. What stops at
     the removal is the house's state update, which is what starts clips. So the clips playing at
     the removal go on exactly as before: a work clip keeps playing (to its end, see below; what it carries
     on its bones, such as the chimney smoke of a working building, keeps going and sinks with
     the house), and a door clip that had begun runs to its end. No clip starts after the removal,
     and the house is **never reset to its rest pose** for the collapse. A building that was idle
     sinks in its idle pose. Test T39.
     **The work clip, the animals and the clips' sounds (2026-11-13, E-gfx-064, code reading;
     corrects "keeps looping" above).** (a) *The work clip.* `Working` is started to play once
     (GFX-ANIM-6.2); in normal play the state update stops it when the work's duration ends.
     Collapsing, that stop no longer comes either, so a work clip in progress at the removal runs
     on to its own end and then stays on its last frame; it does not start again. (A clip that
     loops by itself, such as the production animals' below, keeps looping.) (b) *Production
     animals* (1.6a: the donkeys of a donkey breeder, the pigs of a piggery) stay with the house:
     their looping clip goes on (their clocks advance in the same per-frame step), they are drawn
     every frame on their bones of the house's current pose and placement, so they sink with it,
     and they go with the house view at the end. How many show stays as it was at the removal
     (that count is set by the state update, which has stopped). (c) *Sounds and particles of
     the clips.* A clip's `Sound` and `Particle` events (GFX-ANIM-2.3) are fired from the same
     per-frame step that advances the clip, so they keep firing during the collapse at their
     times for the part of the clip still to play: a work clip's events after the point it had
     reached at the removal still sound (e.g. a donkey breeder's `donkey` at 10, 30 or 50
     animation-seconds, a slaughterhouse's `slaughterhouse` at 19.4 or 40.4), each at most once,
     under the usual visibility rule of SND-2.2; none repeats because nothing restarts. The door
     sounds are not clip events: the opening sound came with the door's start, and the closing
     clip and its sound (started by the state update 0.6 s after opening) never come, so an
     opening door stays open and silent. The production animals' clips carry no events. The
     collapse sound (4) comes on top. Confidence: high for (b) and for (c)'s firing path and the
     door; medium for (a)'s "stays on its last frame" (what the animation library does with a
     finished play-once clip that is never stopped) and for the sound's place following the sink.
     Test T43.
  3. **Dust.** The particle effect `DemolitionDust` (`data/scripts/effects/DemolitionDust.pfx`,
     format 8.3a) starts once at the house's position (its origin on the ground, not the box
     centre): 50 particles, one round only (no rebirth), all born within the first 0.05 s
     (spawn 0.001 +- 0.001 s), each at a random offset of up to +-5 units in x and z (0 in height)
     from that point; velocity (0, 0.8, 0) +- (1, 0.5, 1) units per second, times (1 - 0.6 t);
     size 7 +- 3, times 0.1 + 0.5 t + 2.8 t^2 - 1.4 t^3 (a tenth at birth, twice at the end);
     life 5 +- 3 s; colour (0.6, 0.3, 0.0) and alpha 0.3 +- 0.15, alpha times
     0.5 + 3 t - 8 t^2 + 4.5 t^3 (half at birth, at most 0.82 near t = 0.23, back to 0 at the
     end; it dips to -0.01 near t = 0.95, so clamp at 0); t = age / life. Texture: the rectangle
     (0.25, 0, 0.5, 0.25) of `data/gfx/particles_texture.tga` (a soft grey puff, mean alpha 76 of
     255), multiplied by the particle's colour, blended with its alpha, no depth write. So a
     brown cloud about 10 units across rises a few units and fades within 2 to 8 s, well after
     the house has gone.
  4. **Sound:** `sound_building_collapsed` at the house's position, only when the building is
     visible to the player (`SND-sound-music.md` SND-2.2, the same condition as its door
     sounds).
  5. **The ruin.** The ruin's model is `<tribe>_<ruin>` where `<ruin>` is the remains type's
     model name (`remains.lua`: `RuinSmall01`, `RuinMedium01`, `RuinLarge01`, by the building's
     `bp_setRemains`) and `<tribe>` the building's tribe (e.g. `Roman_RuinSmall01`). It stands
     on the building's field, on the
     ground height there, and lies on a copy of the building type's ground decal (2.3), draped
     on the terrain the same way (each vertex 0.01 above the ground). The ruin has no fire,
     smoke or other particles (no code path gives it one; `Fire.pfx` is not used for buildings)
     and no colour column: the ruin is drawn in the same pass as a site's frame pieces, with
     2.6a's formula (ambient + light x saturate(N . L) per vertex x s) x texel, then the fog of
     war; no colour map, no light map, opaque, depth written; its texture is its mesh's own. The
     heading is the ruin entry's own, which is the default every `Building` entry gets (-30
     degrees; the ruin entries name only a mesh). It stays while the remains burn (`rm_setBurningTime` 333 ticks,
     MIL-CON-4.1a) and goes, with its decal, in the tick they have burnt out.
  So for about 2.9 s the sinking house and the ruin stand in the same place, the house
  sinking through the ruin inside the dust. A site that is torn down or burnt simply vanishes
  (no collapse, no dust, no ruin, 2.2a (3 a), MIL-CON-4.1a); a headquarters never leaves a ruin
  (MIL-CON-4.1) but its house still collapses.
  Confidence: high for 1 to 3 and 5 (the collapse start, the per-frame sink, the effect file,
  the ruin view's creation); medium for the dust's colour order (taken as red, green, blue,
  alpha as for the status icons) and for "visible" in 4.

### GFX-OBJ-7 Carried wares and tools (Q-192 b, Q-207 a; E-gfx-010)

- **GFX-OBJ-7.1 Attach bone.** Every settler has one attach bone, named by its
  `Character`'s `link` attribute in `settler_config.xml`, default `SLOT_HandR`. No
  shipped `Character` sets `link`, so all settlers of all tribes use `SLOT_HandR`
  (a child of the right hand bone; the carry and work clips animate it). The donkey
  uses its own `link`, `SLOT_Goods`. The carried ware and the tool are both drawn
  at this one bone. Confidence: high.
- **GFX-OBJ-7.2 Transform chain.** Every frame, for a ware or tool attached to a
  settler (column vectors; the same as Direct3D's `v x B x W` with row vectors):
  `world vertex = W x B(t) x v`, where
  - `v` is the item mesh vertex (`items.xml` mesh) after the basis conversion of
    GFX-VIEW-1.2 at 1.0 units per metre (items keep their file size);
  - `B(t)` is the attach bone's model-space matrix in the settler's current pose:
    rest transforms of `settler_model.GR2` overridden by the playing clip's tracks
    (GFX-GR2-7.5 / 7.6), composed parent before child from the root, everything
    converted at the settler scale 1.2. It is rigid (rotation and translation, no
    scale). The bone's full orientation is used, not only its position;
  - `W` is the settler's own world matrix, the one its body is drawn with
    (position, heading, evade offset; GFX-ANIM-4).

  Nothing else enters the chain: no `goodOffset`, no `goodRotation`, no per-ware,
  per-tool or per-tribe offset, no extra scale. The shoulder position of a plank,
  log or stone, and the hold of an axe, come entirely from the clip: the carry clips
  (WalkPlank, WalkWood, StandStone, ...) move `SLOT_HandR` itself onto the grip of the
  ware, so the same rule gives "plank on the shoulder" for WalkPlank and "axe in the
  hands" for WorkLoop0Axe. The pose is the shared pose of GFX-ANIM-2.4 (built with
  the identity offset, so `B` is relative to the settler's root). Confidence: high.
- **GFX-OBJ-7.3 Frames.** After the basis conversion the model frame of a settler is:
  +x the settler's right, +y up, +z forward (the rest pose faces file +x,
  GFX-OBJ-1.8), origin on the ground under the root. The item frame is the item
  file's own axes after the same conversion (for the long wares and tools the long
  axis is item +z). The test values of 7.8 are in these frames.
- **GFX-OBJ-7.4 Which object.** The ware drawn in a hand is the ware object itself:
  while a settler carries it, it is attached to him and placed by 7.2; when he puts
  it down it is detached and placed by GFX-OBJ-5. The tool is a separate object
  each settler of a working kind owns for its whole life. Its good is the first good
  in the job's cost list (`Jobs.lua` `gp_addCost`, in file order) that has an
  `items.xml` entry. Workers and specialists use their own job; the builder kind
  always uses the job `Constructor`, the soldier kind always `Private` (every rank
  holds the Sword), the leveller kind `Bulldozer`; carriers and donkeys have none.
  From the shipped data:

  | job | tool | job | tool |
  |---|---|---|---|
  | Woodcutter | Axe | Stonecutter | Pickaxe |
  | Miner | Pickaxe | Forester | Shovel |
  | Fisher | Fishing Rod | Hunter | Spear |
  | Farmer | Scythe | Baker | Rolling Pin |
  | Cabinetmaker | Saw | Locksmith | Gripper |
  | Slaughter | Hatchet | Smith, ShipBuilder, Scholar | Hammer |
  | Coiner, Smelter | Pot | Explorer | Spear |
  | builders (Constructor) | Hammer | levellers (Bulldozer) | Shovel |
  | soldiers (Private) | Sword | Miller, Brewer, Pig Farmer, DonkeyBreeder, Helper, carriers | none |

  Confidence: high (rule), high (table: computed from the data with the rule).
- **GFX-OBJ-7.4a A ware used up in the hand hides the tool (Q-355).** The settler's
  lookup good (GFX-ANIM-1.3) and whether its tool is drawn are a state of the settler's
  view, changed only by these events:
  - (a) a ware is put in his hand (attached, 7.4): lookup good = that ware's good, tool
    hidden;
  - (b) the ware in his hand is attached somewhere else (put down on a flag or in a
    building, handed over, laid in a boat): lookup good = his tool's good, tool drawn
    (subject to 7.5 (a), (c));
  - (c) the ware in his hand is destroyed while he still holds it (used up in place, not
    put anywhere): lookup good = none, and the tool **stays hidden**. Nothing but a later
    (a) or (b) changes this.

  The shipwright in ship mode is the case that occurs: each plank he carries to the ship
  is used up in his hand at the ship (ECO-FOOD-7.4), so after his first plank he has the
  lookup good none (Stand and Walk, the base clips) and no hammer drawn whenever he
  carries nothing, for the rest of his life; while he carries a plank it is the Plank
  (WalkPlank); WorkLoop0ShipBuilder is a profession clip and unaffected. Every worker
  that puts its goods down (on a flag or in a building) gets its tool back by (b).
  Measured on the one shipwright of E-trace-002 (settler 93488): StandHammer and
  WalkHammer (559 rows) only from trace tick 33749 to 39338; first WalkPlank at 39339,
  first work at the ship 39549; from 39599 to 149999 never a hammer clip: Stand 3991,
  Walk 3858, WalkPlank 2420, Pickup 49, PutDown 47, WorkLoop0ShipBuilder 676 rows
  (every 10th tick, E-heap-002 clip names). Code (E-gfx-033): the ware's view, when
  destroyed while attached to a settler's view, sends the settler's view a "held ware
  gone" notice that clears the held ware and the lookup good and does not touch the
  tool; (a) and (b) both go through the view's "held ware changed" update, which shows
  the tool again only when nothing is held.
  - (d) A detail of the same update (code only, medium; not seen in a recording): while
    the settler plays WorkLoop0 or WorkLoop1 its tool is drawn as an animated copy
    (made when the loop starts, removed when the request ends); an (a) or (b) during
    that time changes the lookup good (to the ware's good, or to none) but not the tool,
    and when the animated copy is removed the tool is drawn again while the lookup good
    stays as it was (none after a (b)).

  Confidence: high for the shipwright (measured) and for (a) to (c) (code, and every
  other settler's clips of E-trace-002 agree with (a), (b): TRC-29); medium for (d).
- **GFX-OBJ-7.5 When the tool shows.** The tool is drawn exactly while (a) the
  settler itself is drawn (outside its building, in view), (b) it carries no ware,
  and (c) its field is not under fog of war for the local player, except after a ware
  was used up in its hand (7.4a (c)). Nothing about the
  playing clip matters. So a woodcutter has the axe walking out, standing at the
  tree, chopping and walking back empty; it disappears the moment he takes the log
  and he walks home with the log in its place; inside the hut neither is drawn.
  Builders and levellers at a site show the hammer / shovel. The state is
  re-evaluated when the carried ware changes or the settler's visibility changes
  (so a settler that walks out of fog keeps its last state until then; medium).
  This replaces the earlier "only while the clip marks the item as shown"
  (GFX-ANIM-3.5 of 2026-10-17, E-pt2-001): the flag read there is the fog test
  of (c), not a clip property. Confidence: high (a, b), medium (c timing).
- **GFX-OBJ-7.6 Carrier in a boat.** While a carrier rows on a waterway his ware
  is not attached to the bone. It lies in the boat: matrix = `W x R_y(goodRotation)`
  (10 degrees about the vertical), with the translation moved by `goodOffset` taken
  as (x, 0, z) = (-0.5, 0, 0.4) in the settler's frame (turned by `W`'s rotation;
  only the horizontal parts are added). The boat model itself sits at `W` moved by
  `boatOffset` (-0.25, 0.2) the same way. This is the only use of `goodOffset`,
  `goodRotation` and `boatOffset`. Confidence: high (chain), medium (the waterway
  test and the boat's own bobbing are not specified here).
- **GFX-OBJ-7.7 Clip choice.** Which clip the settler plays (and so where
  `SLOT_HandR` goes) is GFX-ANIM-1.3: the good used for the lookup is the carried
  ware's good, else the tool's good (also while the tool is hidden by fog), and the
  Boat for a rowing carrier.
- **GFX-OBJ-7.8 Test values.** `B(t)` of 7.2 for six cases at three clip times
  each: origin = the attach point in the model frame; quaternion = the rotation from
  item axes to model axes (w >= 0); item centre and ends = the centre of the item
  mesh's bounding box and the two ends of the box along its long axis, in the model
  frame. Clip times are clip-local animation seconds (before `globalSpeed`); the
  walk clips loop (1.0 s), the work clips play once; the work times are the clips'
  hit sounds. The settler's own `W` is not included: in the world, apply the
  settler's heading and position on top. An implementation must match to 0.005.
  Computed with the investigation's carry pose script.

| ID | case (clip, item) | clip t | origin | quaternion (x, y, z, w) | item centre | end - | end + |
|---|---|---|---|---|---|---|---|
| C1 | carrier walking with a plank (settler_walk_plank, GO_Plank01) | 0.00 | (0.330, 1.334, 0.096) | (0.0580, -0.0386, 0.6356, 0.7689) | (0.277, 1.353, 0.070) | (0.262, 1.497, -0.959) | (0.292, 1.209, 1.099) |
| C2 | carrier walking with a plank (settler_walk_plank, GO_Plank01) | 0.25 | (0.284, 1.417, 0.201) | (0.1004, -0.1391, 0.6605, 0.7310) | (0.232, 1.437, 0.173) | (0.306, 1.781, -0.805) | (0.158, 1.094, 1.151) |
| C3 | carrier walking with a plank (settler_walk_plank, GO_Plank01) | 0.50 | (0.202, 1.368, 0.272) | (-0.0229, -0.1856, 0.6520, 0.7348) | (0.159, 1.387, 0.231) | (0.474, 1.603, -0.736) | (-0.155, 1.171, 1.197) |
| C4 | woodcutter walking out (settler_walk_pickaxe, GO_Axe01) | 0.00 | (0.278, 1.305, 0.180) | (0.1353, 0.0879, 0.6804, 0.7149) | (0.239, 1.418, 0.063) | (0.100, 1.451, -0.363) | (0.378, 1.384, 0.488) |
| C5 | woodcutter walking out (settler_walk_pickaxe, GO_Axe01) | 0.25 | (0.215, 1.426, 0.224) | (0.0240, 0.1109, 0.7658, 0.6329) | (0.167, 1.507, 0.087) | (0.087, 1.445, -0.351) | (0.246, 1.570, 0.524) |
| C6 | woodcutter walking out (settler_walk_pickaxe, GO_Axe01) | 0.50 | (0.174, 1.349, 0.360) | (0.0053, -0.0211, 0.7289, 0.6842) | (0.166, 1.456, 0.233) | (0.175, 1.473, -0.215) | (0.156, 1.439, 0.681) |
| C7 | woodcutter chopping (woodcutter_chop, GO_Axe01) | 1.24 | (0.062, 0.849, 0.964) | (0.5998, -0.7924, 0.0680, 0.0880) | (0.038, 0.781, 1.115) | (0.064, 0.877, 1.552) | (0.012, 0.686, 0.677) |
| C8 | woodcutter chopping (woodcutter_chop, GO_Axe01) | 2.88 | (0.244, 0.899, 0.907) | (-0.1401, -0.9649, 0.1331, 0.1778) | (0.202, 0.963, 1.055) | (0.373, 1.056, 1.460) | (0.032, 0.870, 0.651) |
| C9 | woodcutter chopping (woodcutter_chop, GO_Axe01) | 4.56 | (0.243, 0.901, 0.907) | (-0.1399, -0.9651, 0.1328, 0.1771) | (0.201, 0.965, 1.056) | (0.371, 1.058, 1.461) | (0.031, 0.872, 0.651) |
| C10 | stonemason walking out (settler_walk_pickaxe, GO_PickAxe01) | 0.00 | (0.278, 1.305, 0.180) | (0.1353, 0.0879, 0.6804, 0.7149) | (0.250, 1.354, 0.106) | (0.111, 1.387, -0.318) | (0.388, 1.321, 0.529) |
| C11 | stonemason walking out (settler_walk_pickaxe, GO_PickAxe01) | 0.25 | (0.215, 1.426, 0.224) | (0.0240, 0.1109, 0.7658, 0.6329) | (0.186, 1.455, 0.140) | (0.107, 1.393, -0.295) | (0.265, 1.517, 0.575) |
| C12 | stonemason walking out (settler_walk_pickaxe, GO_PickAxe01) | 0.50 | (0.174, 1.349, 0.360) | (0.0053, -0.0211, 0.7289, 0.6842) | (0.167, 1.394, 0.279) | (0.177, 1.411, -0.167) | (0.158, 1.377, 0.725) |
| C13 | stonemason quarrying (stonecutter_crush, GO_PickAxe01) | 1.56 | (-0.259, 0.508, 0.685) | (0.1665, 0.9629, 0.0736, 0.1995) | (-0.328, 0.522, 0.746) | (-0.511, 0.488, 1.153) | (-0.146, 0.555, 0.340) |
| C14 | stonemason quarrying (stonecutter_crush, GO_PickAxe01) | 4.08 | (-0.242, 0.544, 0.752) | (0.1787, 0.9686, 0.0655, 0.1602) | (-0.305, 0.559, 0.819) | (-0.455, 0.528, 1.239) | (-0.156, 0.591, 0.399) |
| C15 | stonemason quarrying (stonecutter_crush, GO_PickAxe01) | 6.84 | (-0.233, 0.548, 0.651) | (0.1584, 0.9669, 0.0429, 0.1956) | (-0.301, 0.565, 0.712) | (-0.476, 0.556, 1.123) | (-0.126, 0.575, 0.301) |
| C16 | woodcutter walking home with a log (settler_walk_log, GO_Log01) | 0.00 | (0.390, 1.399, 0.155) | (0.0876, 0.0765, 0.5600, 0.8203) | (0.361, 1.410, 0.147) | (0.132, 1.469, -0.849) | (0.590, 1.350, 1.143) |
| C17 | woodcutter walking home with a log (settler_walk_log, GO_Log01) | 0.25 | (0.347, 1.479, 0.273) | (0.1437, -0.0351, 0.5864, 0.7964) | (0.318, 1.491, 0.264) | (0.203, 1.768, -0.715) | (0.434, 1.215, 1.243) |
| C18 | woodcutter walking home with a log (settler_walk_log, GO_Log01) | 0.50 | (0.251, 1.429, 0.355) | (0.0238, -0.0774, 0.5897, 0.8036) | (0.225, 1.440, 0.339) | (0.324, 1.572, -0.672) | (0.126, 1.307, 1.349) |

  Item boxes after the conversion (item frame, lo .. hi), to check the item side
  alone: GO_Plank01 (-0.178, -0.022, -1.068) .. (0.188, 0.131, 1.010); GO_Axe01
  (-0.123, -0.054, -0.580) .. (0.330, 0.063, 0.318); GO_PickAxe01 (-0.291, -0.050,
  -0.530) .. (0.377, 0.062, 0.364); GO_Log01 (-0.172, -0.181, -1.039) .. (0.172,
  0.238, 1.010). Rest pose (no clip) for the skeleton side: SLOT_HandR at (0.932,
  1.270, -0.026), right hand bone at (0.735, 1.319, -0.022), pelvis at (0, 0.686, 0).
  Reading: C1-C3 the plank lies on the right shoulder (y about 1.35, the shoulder
  is at 1.32, x +0.2..+0.33) along the walk direction (z from about -0.9 to +1.1);
  C4-C6 and C10-C12 the axe / pickaxe rests on the right shoulder, blade forward;
  C7-C9 the axe is held in front at waist height (the tree is ahead), C13-C15 the
  pickaxe is held low in front, slightly left; C16-C18 the log lies on the right
  shoulder like the plank.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| element defaults | rot 0 0, scale 1 1, offset 0 0, numInstances 50, vb 15000, ib 30000 | | high | E-gfx-007 |
| tree rot / scale | 0..360 deg; 0.8..1.0 (x0.9 palm; 0.6..0.8 african, cherry) | | high | data |
| house light map mix | lerp(result, result x L, saturate(1.5 - D)) | | high | house_shader.fx |
| player colour gain | x 1.8 where texture alpha = 0 | | high | shared.inc |
| tree sway | strength sat(r^2 x 0.003) x m22; phase 0.35 pi per unit | | high | tree_shader.fx |
| flag wave | 0.33 x sat(-0.02 + x^2 + z^2); cos(4 w) | | high | flag_shader.fx |
| stone gloss power | 16 | | high | stone_shader.fx |
| haze | top (0, 0.8, 0.3), bottom (0, 0, 0.05), range 7 | world | high | doodads_shader.fx |
| settler specular power | 16 | | high | skin_color_mesh.fx |
| palette atlas | 1024 x 1024, blocks 16 x 16 | texels | medium | E-gfx-007 |
| goodOffset / goodRotation | (-0.5, 0.4) / 10 deg, ware in a carrier's boat only (7.6) | | high | settler_config.xml; E-gfx-010 |
| attach bone | `SLOT_HandR` (Character `link`, none set); donkey `SLOT_Goods` | | high | E-gfx-010 |
| item / settler units per metre | 1.0 / 1.2 | | high | E-gfx-010 |
| flag stacks | 6 stacks x 6 wares, 60 deg apart, radius bbox.z + 0.5 | | high | E-gfx-007, items.xml |
| street | segments 8, width 1.0, texStretch 0.25, crossing 2.25, buoy 1.7 | world | high | graphics.xml |
| player colours | see GFX-OBJ-2.11 | | high | menu.cfg |
| decal lift / sampler | 0.01 above the terrain; wrap u and v, v from the top row | world | high | E-pt5-001; shared.inc |
| site stage | min(N, floor((N + 1) f + 0.5)); N = numbered site meshes (woodcutter 16, generic 4) | | high | E-pt5-001 |
| house rise | (H - 0.1) x m x 1.1111112; H woodcutter 4.297 | world | high (formula), medium (H's LOD) | E-pt5-001 |
| builder request | WorkLoop0, 55 ticks, per work step | ticks | high | E-pt5-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | deposit with gfx name `tree_14` | model lookup | `map/objects/RS_TreeCherryBlossom01.GR2`, tree_shader, yaw in 0..360, scale 0.6..0.8 |
| T2 | deposit `medstoneressource_03` | model lookup | `RS_MedStoneRessource03.GR2`, stone_shader, no random yaw or scale |
| T3 | building type Roman tribe, woodcutter | buildings.xml entry | `Roman_Lumberjack`: mesh `PB_R_WoodcutterHut01.gr2`, frame `...ConstructionSite.GR2` |
| T3a | woodcutter site just placed, no builder yet | draw | only the ground decal and the door flag; no site model piece, no house (2.6) |
| T3b | Roman woodcutter site (frame 1 plank, main 1 plank, N = 16, H = 4.297), builder present, materials as given | after k work decrements (ECO-HQ-4.6a, float32) | k = 0 (0 %, nothing delivered or 1 plank waiting): decal + flag only. k = 110 (25 %, 1st plank taken at unit 1's start): f = 0.5 in float32 just below one half, stage 8: meshes 0..7 (piles and posts), no house. k = 221 (50 %, end of unit 1; 2nd plank must be on site for unit 2 to start): f = 1, stage 16: all 16 meshes, no house (m = 0). k = 332 (75 %, 2nd plank taken): m = 0.504545, all 16 meshes, house cut at 2.353 above the base. k = 442 (100 %): finished, site model gone, house complete, decal stays |
| T3c | same site | frame decrement at which each stage first shows | stage 1..16 at decrements 7, 20, 33, 46, 59, 72, 85, 98, 111, 123, 136, 149, 162, 175, 188, 201 of unit 1 (steps: 1..55 at position 0, 56..110 position 1, 111..165 position 2, 166..220 position 3, 221 the extra one) |
| T3d | generic site model (N = 4), frame 1 plank | same | stages 1..4 at decrements 23, 67, 111, 155; with 2 frame planks 45, 133, 221 (unit 1) and 309 (unit 2's 88th) |
| T3e | woodcutter GROUND vertex 0: uv (0.498, -0.002); vertex 5: uv (0.502, -0.124) | nearest texel | vertex 0: column 509, row 1021 (v wraps to 0.998), open ground, alpha 255; vertex 5: column 514, row 897, start of an edge strip, alpha 0 (the strips in the lower right quarter are 128 rows each, alpha rising 0 to 255 down the strip) |
| T3f | Roman castle GROUND | uv range | v 0.002..0.498: paved half of the atlas |
| T3g | woodcutter GROUND, bilinear on mip 0 | alpha along model x at z = -0.4 / along z at x = 0 | x 2.565: 255, 2.7: 204, 2.9: 102, 3.1: 34, 3.244: 0; x -2.729: 255, -2.9: 187, -3.1: 102, -3.3: 34; z -2.525: 255, -2.8: 170, -3.1: 51, -3.376: 0; z 1.76: 255, 2.0: 170, 2.3: 68, 2.57: 0; centre 255 (2.3a b) |
| T3h | woodcutter GROUND, bilinear on mip 0, along z at x = 0 every 0.05 units from -2.50 to -3.35 | alpha | 255, 243, 238, 221, 204, 187, 170, 136, 119, 102, 85, 68, 51, 34, 34, 17, 17, 0 (DXT3 steps of 17, with bilinear in-betweens such as 243); along x at z = -0.3 from 2.55 to 3.20: 255, 238, 221, 204, 170, 153, 134, 102, 85, 68, 46, 34, 17, 11 (2.3a b) |
| T3h | woodcutter site on a slope, ambient a, light colour c, N . -L = 0.8 for an upward normal, no shadow | decal colour | (a + 0.8 c) x texture colour, independent of the slope (2.3a d) |
| T3i | woodcutter site, stage 4 (frame decrements 46 to 58) | draw | mounds 0 to 3 with the two plank stacks and the upright shovel, no posts; no extra plank objects however many planks are in the site's stock (2.6a) |
| T4 | flag of player 1 | draw | `flag.gr2` with flag_shader; transparent texels show (0.6, 0, 0) x 1.8 x light |
| T5 | wares at a flag: 2 Wood, 1 Stone | placement | Wood stack at angle 0, wares at offsets (-1,0,0) and (0,0,0) x bbox; Stone stack at 60 deg |
| T6 | settler walking in direction 0 | facing | model +z (rest pose) rotated 90 deg -> faces +x; no extra turn |
| T7 | house pixel with D = (1, 1, 1) | light map mix | result = C x colorMap x D x lerp(1, L, 0.5) |
| T8 | road of 4 field points | samples | 25 samples; width 1.0; crossing radius 2.25 at both flags |
| T10 | carrier walks with a plank, heading 0 | frame | plank drawn with W x B(t) of WalkPlank; values C1-C3 of 7.8 before W; no goodOffset |
| T11 | woodcutter leaves his hut, walks to a tree, chops, takes the log, walks home | tool / ware | axe shown from the door on (WalkAxe, C4-C6), during WorkLoop0Axe (C7-C9); hidden from the pickup on; log on the shoulder (WalkWood, C16-C18) |
| T12 | carrier with a ware rows a boat | frame | ware at W x R_y(10 deg), shifted by (-0.5, 0, 0.4) in the settler frame; not at the bone |
| T13 | builder at a site, hammering | tool | Hammer at SLOT_HandR (job Constructor); leveller: Shovel |
| T14 | soldier of rank 3 walking | tool | Sword (the soldier kind always uses the Private cost list) |
| T9 | building on a slope | decal | every decal vertex lies 0.01 above the terrain under it; no gap on the downhill side (2.3) |
| T15 | settler standing on flat open ground, texel green 0.8, shipped daytimes | palette u | D = 0.766 x 0.8965 + 0.5413 = 1.228; u = saturate(0.8 x 1.228) = 0.982 x scale.x; in a building's shadow D = 0.610, u = 0.488 x scale.x |
| T16 | Roman woodcutter's hut finished on field (11, 11) (GFX-VIEW T18) | decal | the `GROUND` mesh turned by -30 deg with the house: its corner (-3.478, 0, -3.38) lands at (-232.322, -, -207.316) before draping |
| T17 | Roman guard house (LOD0 box x -2.929..2.929, z -3.268..3.416) on field (11, 11), raw height 4200 | soldier flag | local (1.9331, 0, -3.6022), world (-227.525, 4.2, -204.803); border flag local (-1.9331, 0, -3.6022), world (-230.873, 4.2, -206.736); banners fly toward world (0.5, 0, -0.866) |
| T18 | same guard house with 0 / 1 / 3 / 7 soldiers inside | soldier flag meshes | `0` / `0`-`1` / `0`-`3` / `0`-`5` |
| T19 | Roman headquarters (box x -6.432..6.501, z -1.548..8.119) on field (10, 10), raw 5000 | soldier flag | local (4.3024, 0, -2.0314), world (-235.258, 5.0, -207.454), all six meshes; no border flag |
| T20 | occupied guard house, enemy distance 30 / 20 / 10 / -1 | border flag texture offset | 0 / 1/3 / 2/3 / 2/3 |
| T21 | own barracks, coin supply switched off, move-out off | status effect | `Sleep_NoCoin` (coins with a stop mark) over the box centre; switching coins on removes it; move-out on as well: `Sleep_NoSoldierAndCoin` |
| T22 | own woodcutter's hut, productivity 0.2 for 9.9 s, then 10.0 s | status effect | none, then `Sleep` ("Z") |
| T24 | headquarters on flat ground at height 5, building field point B = (bx, 5, bz), flag at B + (3, 0, -5.196) | flag-to-door strip (6.4) | corners B + (3.866, 0, -4.696) (u 0, v 0), B + (2.134, 0, -5.696) (u 1, v 0), B + (0.866, 0, 0.5) (u 0, v 1.5), B + (-0.866, 0, -0.5) (u 1, v 1.5); the half near B under the house |
| T25 | woodcutter's site just placed, same ground | strip | identical to T24 (no state or type test); present from the placement, before the builder |
| T26 | finished woodcutter's hut, building point 0.6 higher than the flag point | strip | d = normalise(-3, 0.6, 5.196) from the flag; v at B = sqrt(36.36) x 0.25 = 1.507; the mesh follows the terrain between the two |
| T27 | a road upgraded to a donkey road | road texture | `street.dds`, alpha 255 in the middle; a plain road and every flag-to-door strip `simple_street.dds`, centre alpha about 0.59 |
| T27a | level 2-step road between flags F and G (L = 4), 66 wares carried in its first 6000-tick period (TRN-CARRY-6.3); F also ends a plain road | road and crossings per frame | texture 0 and crossing 0 at F and G until the period ends; from the tick the average passes 130 (the donkey request) texture 1 and crossing texture 1 at F and G, with no donkey on the road yet; F's crossing stays texture 1 although its other road is plain; nothing changes back later (6.2) |
| T27b | flat road (normals up) through (0, 0, 0), (6, 0, 0), (12, 0, 0), no other roads at either flag | samples | T = +x at all three; L = 5.7614, 5.5400, 0; centres x = 0, 0.8722, 1.6397, 2.3373, 3.0, 3.6627, 4.3603, 5.1278, 6.0, 6.8486, 7.6127, 8.3204, 9.0, 9.6796, 10.3873, 11.1514, 12.0 (17 samples, y = z = 0); S = (0, 0, -1); v = 0, 0.2181, 0.4099, ..., 1.5 at x 6, 3.0 at x 12. With two roads at both flags: 15 samples from x 0.8722 (v 0) to 11.1514 (v 2.5698) (6.2a) |
| T27c | flat road (0, 0, 0), (6, 0, 0), (9, 0, -5.196) (a 60 degree bend) | samples | T = (1, 0, 0), (0.866, 0, -0.5), (0.5, 0, -0.866); L = 5.9212, 6.0328; centres (0.9033, 0, 0.0526) at t 1/8, (3.1289, 0, 0.4811) at t 1/2, (4.4859, 0, 0.5412) at t 3/4, (6, 0, 0); step 1 (7.8588, 0, -2.2392) at t 1/2; S at step 0 t 1/2 = (0.1751, 0, -0.9845), at P1 (-0.5, 0, -0.866); v at P1 1.5376, at the end 3.0764. The road swings out by up to about 0.56 to +z before the bend: the 1.3 tangent scale, not a polyline (6.2a) |
| T23 | guard house upgrading to a watch tower, u = 0.45, new box height H | upgrade cut | plane at base + (H - 0.1) x 0.5; old house above, new house below, watch-tower site model shown |
| T28 | forester's hut finished at real time 0, worker arrives, first planting ends at 90 s, no further planting | Sleep icon | shown from 10 s (productivity -1) to the frame of 90 s (0.5); off until the productivity has decayed to 0.25 (five 600-tick decays, 300 s later), then shown again 10 s after that decay (8.3) |
| T29 | production building switched off | icon | `Sleep_ProductionOff` at once, whatever the productivity; `Sleep` never while off (8.3) |
| T30 | a Roman woodcutter's hut (lodBias -70, lodScale 1.2, two levels) at distance d from the camera's eye point | level drawn (1.1c) | p = 1 (VeryHigh): level 0 (the model) at every d; p = 0: level 0 at d = 66, level 1 (`LOD0`) at d = 67; p = 0.5: level 0 at d = 74.9, level 1 at 75.1; any p: level 0 at d <= 58 (1.2 d - 70 < 0 gives a value below 0.5, clamped at 0) |
| T32 | 1.4 (Q-1027 a): the first free game of a program run on MP_2P_Storm_Coast | the view generator's draws for the first two instances | the first instance is the file's first deposit (id 11, field (65, 103), fine position (262, 412)), not a flag or doodad: its unit draws are 0.491558 (yaw), 0.170283, 0.4644, 0.828969 (x, y, z scale; for a 0.8 .. 1.0 range 0.834057, 0.892880, 0.965794); the second is the file's second deposit (id 12, (66, 104)) with 0.719698, 0.099176, 0.037281, 0.218381. The first doodad's instance comes after all 3212 deposits (its draws start at draw 12849) |
| T33 | 1.4 (Q-1027 b): the map's deposit 2093 of Storm Coast (field (136, 52), stored fine position (544, 207)) | where its tree stands | at the sample (544, 207), one fine step from the field's sample (544, 208), plus the element's offset; not at the field's sample |
| T34 | 1.4 flag shift (index corrected 2026-11-07): road flags on Storm Coast (width 170, lattice width 681) | z shift | flag on (10, 11): sample (42, 44), i = 30006, -1.3; flag on (130, 40): sample (520, 160), i = 109480, v = 4211814, **+1.3** (the field index 6930 would give -1.3) |
| T31 | 2.1 (the house's light normal): the Roman HQ at noon, `look-compare-auto/2026-10-31-f53` pair weakfogoff-006000-default (2560 x 1440), the dome region (1000, 250, 260, 240) | original / ours luminance per pixel | a smooth left-to-right gradient, not a uniform factor: by quarter columns 0.958, 1.019, 1.142, 1.172 (the right of the dome up to 1.3 brighter in the original, the left down to 0.92). Lighting the model-frame normal against ours' turned normal gives 0.986, 0.979, 1.100, 1.122 on the dome's camera-facing vertices (light map 0.6); ours' region mean 1.06 .. 1.08 darker comes from this. Flat ground cannot show it (only the sun's elevation counts there) |
| T36 | 8.7: a finished Roman woodcutter's hut (H 4.297, 2.6b) burnt by a catapult at time 0, the camera on it, 25 frames a second | the house's sink, the dust, the ruin | the decal, status icon and flags gone in the first frame; sink H x (0.35 t)^2: 0.13 at 0.5 s, 0.53 at 1 s, 2.11 at 2 s, 4.297 from 2.857 s, the view gone at the first update after that; 50 dust particles all born by 0.05 s, none after; `Roman_RuinSmall01` on the hut's field at -30 degrees on the hut's decal from the same tick, no flames; the ruin gone 333 ticks later. A torn-down site: nothing of this (it vanishes) |
| T37 | 2.6a: a site's frame piece and the rising house of the same site, slot 3 | the pixel shading | the frame piece: (ambient + light x N . L x s) x texel, the same whatever the slot; the rising house: x the column of slot 3 and its light map (2.1) |
| T35 | 2.2a (Q-094): a new free game, one human player, no other buildings on the map; the headquarters is created at the start; then place construction sites A, B, C (in that order) | (1) read each house's slot and column; (2) tear down site B (not built yet), then place D; (3) place E; (4) tear down sites C and A, in that order, then place F, G, H; (5) finish E, then demolish it and place J at once, place K 3.0 s later; (6) upgrade a finished guard house in slot s; (7) save and load after step (4) | (1) HQ slot 1 (column 1), A 2, B 3, C 4 (columns 2, 3, 4); (2) D gets 3 (column 3); (3) E gets 5; (4) free stack 4 then 2: F gets 2, G gets 4, H gets 6; (5) J gets 7 (E's slot 5 is still held by the collapse), K gets 5 (freed after about 2.9 s); (6) the watch tower keeps slot s and its column; (7) the buildings then standing are HQ 1, F 2, D 3, G 4, E 5, H 6; the save lists them in creation order HQ, D, E, F, G, H, so after the load HQ 1, D 2, E 3, F 4, G 5, H 6 (columns change for D, E, F, G) |
| T38 | 2.2b: colour map lookups (texel units of the 64 x 256 map, before the column offset) | the colour (0 .. 255) for a column | Roman woodcutter's roof (0.970, 54.925): column 6 (218.0, 159.0, 94.0), column 0 (234, 212, 116). Its log pile (1.270, 63.088): column 6 (179.2, 180.1, 185.4) where a nearest lookup without the half texel gives (182, 182, 183); column 7 (243.2, 245.0, 249.6) against (252, 253, 253). Nubian locksmith (55.552, 74.343): column 1 (255.0, 227.7, 148.3) against (255, 194, 134); column 0 (255, 197, 135) both ways. A mesh without the third set: white (255, 255, 255) in every column |
| T39 | 8.7 (2) pose: a Roman quarry working (its work clip looping, smoke at its chimney bone) torn down at time 0, and a woodcutter's hut torn down 0.2 s after its door began to open | the house's pose from 0 to 2.857 s | the quarry's work clip keeps playing at its own speed while the house sinks (frames differ from frame to frame), to its end and not again (corrected 2026-11-13, 8.7 (2) (a)), its smoke stays at the sinking chimney; the hut's door opens to the clip's end and holds it; no door or work clip starts after 0; neither house jumps to its rest pose at 0 |
| T40 | 6.2c: a flag on flat ground at height 0 at world (0, 0, 0), crossingRadius 2.25 | the crossing's vertices | centre (0, 0, 0) uv (0.5, 0.5); corners (1.9486, 0, 1.125) uv (0.933, 0.75); (1.9486, 0, -1.125) (0.933, 0.25); (0, 0, -2.25) (0.5, 0); (-1.9486, 0, -1.125) (0.067, 0.25); (-1.9486, 0, 1.125) (0.067, 0.75); (0, 0, 2.25) (0.5, 1); six triangles (0, 2, 1), (0, 3, 2), (0, 4, 3), (0, 5, 4), (0, 6, 5), (0, 1, 6); on a slope each corner's height is the terrain triangle's plane there |
| T41 | 3.3 terms: a settler pixel on flat ground lit at noon (N = (0, 1, 0), L = (0.4924, -0.7660, 0.4132), lum(light) 0.8965, lum(ambient) 0.5413, no shadow), the camera straight above (V = (0, 1, 0)), blue mask 1 | D, R and S | N . -L = 0.7660, D = 1.2280; 2 D N + L = (0.4924, 1.6900, 0.4132), R = (0.2723, 0.9347, 0.2285), S = 0.339 (a reflect(L, N) highlight would give R = (0.4924, 0.7660, 0.4132), S = 0.014); in a shadow (factor 0.1): D = 0.6100, R = (0.6257, 0.5769, 0.5251), S = 0.0002 |
| T42 | 3.2 (3) variants: the colour map `default` (entries 1 .. 16: EyeInner 16 rows, EyeOuter 1, Hair 16, Jacket 16, Skin 6, Trousers 16, Belt 16, Shoes 16, Details 16, Apron 16, Hat_brown 16, Hat_green 16, Hat_grey 16, Yellow 8, Red 8, _empty_ 1), the generator seeded 1412607200, three figures made one after the other | the ramp row per entry | first figure 15, 0, 14, 14, 3, 11, 11, 2, 1, 10, 3, 10, 14, 4, 2, 0; second 4, 0, 12, 1, 3, 14, 6, 11, 1, 9, 10, 10, 6, 2, 7, 0; third 12, 0, 9, 15, 3, 2, 2, 4, 12, 8, 5, 4, 5, 1, 6, 0 (16 draws per figure, one-row entries included); block column c of an entry = its ramp's texel 15 - c |
| T43 | 8.7 (2) work clip, animals, sounds: a Roman donkey breeder at productivity 1 (5 donkeys shown), its `Working` clip started 5.0 animation-seconds before the building is torn down (events `donkey` at 10, 30, 50), the house visible | the 2.857 s of the collapse (about 5.1 animation-seconds at globalSpeed 1.8) | the five donkeys keep their looping clip and sink with the house, on their bones, until the view goes; one `donkey` sound at animation-second 10 (about 2.8 s after the removal), none at 30 or 50 within the collapse; no door sound; the collapse sound at 0; the `Working` clip not restarted |
| T44 | 6.2c waterway corner: a flag at world x 0, z 0 whose sample height is 0.5, a waterway attached in direction 1 (south-east), land roads in 0 and 3, the terrain under the corners sloping so that its plane gives heights 0.6 (corner 0), 0.7 (1), 0.4 (2), 0.3 (3), 0.45 (4), 0.55 (5) with normals n0 .. n5 | the six corners | corner 0 at (1.9486, 0.6, 1.125), normal n0; **corner 1 at (1.9486, 0.5, -1.125)** (the flag's height), normal n0 (inherited from corner 0); corner 2 at (0, 0.4, -2.25), normal n2; corners 3, 4, 5 on the terrain with n3, n4, n5; uv as T40; with the waterway in direction 0 instead: corner 0 at the flag's height with the normal (0, 1, 0) |
| T45 | 3.2 (3a) (Q-1043): figures A, B, C made with three different blocks, then A removed, then figure D with a fourth block; then a new game | the atlas places | A (0, 0), B (16, 0), C (32, 0); after A's removal D takes (0, 0); a figure made with a block equal to B's shares (16, 0) and B's place stays until both are removed; after the new game's start the first block takes (0, 0) again, while the colour draws go on from the generator's state |
| T46 | 6.2d: one player's flags by day, close up: a flag with three dirt roads; a flag with one dirt road at a building; a flag with two donkey roads; a flag with one dirt and one donkey road | look at each flag | three dirt roads: their worn surface meets at the flag with no bare ground (the crossing's soft patch covers the last segment); one dirt road at a building: the road runs to the flag, and a faint earth strip runs on to the door; two donkey roads: a round blob of the same light stones as the roads, no hexagon edge, no colour step between road and blob; dirt plus donkey: the stone blob, the dirt road ending at it |
| T48 | 2.8a: a ship drawn lit with D = (1, 1, 1) and no shadow; owner blue (0.01, 0.10, 0.60), then red (0.6, 0, 0) | the pixel colour (before the fog of war, clamped to 0 .. 1) | blue: a hull texel (0.5, 0.4, 0.3) of `PV_R_Ship01.dds` gives (0.5, 0.4, 0.3) (no tint); a white Roman sail texel with alpha 102/255 gives (0.6072, 0.672, 1.032), clamped to (0.6072, 0.672, 1.0); with alpha 51/255: (0.8036, 0.836, 1.016), clamped to (0.8036, 0.836, 1.0); with alpha 0: (1, 1, 1). Red: a white African sail texel with alpha 153/255 gives (1.048, 0.4, 0.4), clamped to (1.0, 0.4, 0.4); its hull stays untinted. The freight crate and the crow's nest are never tinted |
| T47 | 6.2a d: flag F with no road; build road R1 from F to G, then R2 from F to H, then R3 from F to K (dirt) | look at the strips' ends at F; then remove R1 | R1 runs to F's point; R2 and R3 stop one segment (1/8 of their first step) short of F; F's crossing holds 3 roads; after R1 is removed, R2 and R3 still stop short (their strips are not rebuilt) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-gfx-060 | code reading | (session 2026-11-12 night) the house view's per-frame matrix update (collapse clock, sink, then the model clock, clip sampling and the bone pose every frame), its per-update state step (skipped while collapsing), its extra per-frame step (empty for houses) | 8.7 (2), T39 |
| E-gfx-061 | code reading | (session 2026-11-12 night) the crossing's mesh builder (7 vertices, 18 indices, the planar texture coordinates), its corner function (on the terrain plane unless a waterway holds that side), the corner directions set at start-up (60 .. 360 degrees), the crossing's corner reset (radius crossingRadius); `skin_color_mesh.fx` (the settler pixel's diffuse dot, shadow, highlight and atlas lookup) | 6.2c, T40; 3.3, T41 |
| E-gfx-062 | code reading, data, and an earlier photograph of the original (kept outside the repository) | (session 2026-11-13) the settler block's builder (a block per new figure, one draw per colour-map entry), the row writer (column c = ramp texel 15 - c), the view's random generator and its seed, the atlas packer (exact 16 x 16 places, equal blocks shared); `color_slots.xml`, the colour-index maps' red bytes; the woodcutter photograph of the carried-goods comparison | 3.2, T42 |
| E-gfx-064 | code reading, data | (session 2026-11-13) the production house view's per-frame step (house step, then each animal's clock and pose), its bounds and draw callbacks (each animal on its bone of the house's current pose and placement), its state update (the shown count) and destructor; the collapse start (animals untouched); the house's work clip start (played once, events registered) and the model's per-frame advance (attached event dispatch); `buildings.xml` work clip events | 8.7 (2), T43 |
| E-gfx-065 | code reading | (session 2026-11-13) the crossing's corner function (the waterway test on the road in the corner's own direction, the terrain placement skipped), the corner loop of the mesh builder (one normal value carried from corner to corner), the corner reset (the centre's height), the road attach (the slot is the map direction from the flag to the road's first field), the street renderer's list choice (the same waterway flag) | 6.2c, T44 |
| E-gfx-067 | code reading | (session 2026-11-14) | 3.2 (3a) (Q-1043): the block records' use count, the release at a figure's removal, the atlas's place search (first free in row order) and freeing, the missing check for a block without a place, the atlas made with each game's view |
| E-gfx-068 | data files, code reading | `data/gfx/fx_20/street_shader.fx` and `shared.inc` (the street pass's render states, the vertex shader); no depth-bias render state in the code (session 2026-11-14 b) | GFX-OBJ-6.3 depth (Q-1046) |
| E-trace-026 | pictures of the running original | Jens's 2023 save of MP_2P_Storm_Coast loaded, 2560 x 1440, zoom 1 to 3, day / night cycle off (session 2026-11-14 b; pictures kept outside the repository) | GFX-OBJ-6.2d (F58, F60) |
| E-trace-029 | pictures of the running original | the same save, roads built and one destroyed in the look, waterways at zoom 1 and 2 (session 2026-11-15 night, part 2) | GFX-OBJ-6.2a (d) seen, 6.2b seen |
| E-trace-030 | pictures of the running original | the Seafaring tutorial's ships and border buoys close up by day, 2560 x 1440 (session 2026-11-15 day; pictures kept outside the repository) | GFX-OBJ-2.8a seen |
| E-gfx-069 | code reading | (session 2026-11-14 b) | GFX-OBJ-6.2a d (Q-1048): the strip's count test at its build, the road visual's rebuild only at creation and at its own end attaches, the crossing's attach rebuilding only the crossing; the crossing builder makes seven vertices and drapes nothing |
| E-gfx-058 | code reading, data, and the 2026-11-12 look of the original (pictures kept outside the repository) | (session 2026-11-12 night) `house_shader.fx` (the lookup with the half texel, the sampler states), `colors.tga`, every house mesh's third texture coordinate set; the town picture at tick 6000 compared with the program's at the same camera, each house's faces projected and grouped by direction | GFX-OBJ-2.2b, T38; `GFX-world-view.md` GFX-VIEW-2.5 (f), T43 |
| E-gfx-055 | code reading | (session 2026-11-11 eve): the house view's creation (slot from the free stack or a new one), its removal (at once, or after the collapse for a finished building), the collapse clock, the type change and conquest paths, the post-load order of buildings, the upgrade site's own list; `house_shader.fx` (which passes read the colour map); `buildings.lua` / `remains.lua` (every type has a ruin) | GFX-OBJ-2.2a, T35 |
| E-gfx-057 | code reading and data | (session 2026-11-12) the house view's collapse start (effect, sound, what it frees), its per-frame sink, the view list's update; the ruin view's creation from the remains (model name, heading, draped decal) and its removal; `DemolitionDust.pfx` decoded; `particles_texture.tga`; `particle_shader.fx`; `house_shader.fx` pass RenderFrame and `shared.inc` (CalcLight, CalcColor) | 2.2a (4), 2.6a, 8.7, T36, T37 |
| E-gfx-054 | code reading | (session 2026-11-11): the doodad type's delete-by-street switch (its Lua setter, its default in the type's constructor), the doodad cover test and its callers (one field, a box, all doodads), which erase covered doodads | GFX-OBJ-1.4a |
| E-gfx-053 | code reading | (session 2026-11-11): the house pass and the house draw (which argument becomes the colour column; the stack read again), `buildings.xml` numVariations 8 | GFX-OBJ-2.2 |
| E-gfx-031 | code reading | | GFX-OBJ-1.6a (Q-349) |
| E-gfx-033 | code reading and live read of the running game | E-trace-002 `settlers.csv.gz` settler 93488 with E-heap-002's clip names (offset 391) | GFX-OBJ-7.4a, 7.5; GFX-ANIM-1.3 |
| E-sim-008 | code reading | the levels of detail (the model, then `<name>LOD<n>` files while they exist, `House` and `GROUND` meshes); with E-files-001 (the install's last-access times) | GFX-OBJ-1.1a |
| E-trace-001 | live read of the running game | pause and speed log (`pause-speed.csv`): game time and clip clocks frozen while paused, 2x / 3x at speeds 2 / 3; Jens watched flags, trees, water | GFX-OBJ-2.9 clock |
| E-gfx-012 | code reading | | soldier and border flags |
| E-gfx-012 | code reading | `data/scripts/effects/Sleep*.pfx`, `particles_texture.tga` | status icons |
| E-gfx-012 | code reading | | upgrade visual |
| E-gfx-011 | code reading | `skin_color_mesh.fx` | settler light: luminance of light and ambient, shadow factor |
| E-gfx-007 | code reading | `S2DNG.exe` | landscape element reader, instance renderers |
| E-gfx-007 | code reading | | flag, border, building renderers; table loading |
| E-gfx-007 | code reading | | colour slots, settler config, palette parameters, carried good |
| E-gfx-007 | code reading | | ware stacks at flags |
| E-gfx-007 | code reading | | road config and geometry |
| E-gfx-007 | code reading | | menu Lua bindings (setPlayerColor) |
| E-gfx-010 | code reading | | attach bone, transform chain, tool choice and visibility, boat; test values by the investigation's carry pose script |
| E-sim-002 | code reading | | waterway buoys (6.2b) |
| E-gfx-029 | code reading | | road curve and tessellation (6.2a, Q-092) |
| E-gfx-024 | code reading | | donkey texture switch (6.2, Q-221) |
| E-gfx-017 | code reading | `simple_street.dds` alpha profile | flag-to-door strip, road texture per kind |
| E-pt2-001 | code reading | | ground decal draped on the terrain |
| E-pt5-001 | code reading | | decal from the GROUND mesh, site pieces, rising house, builder request; test values by the investigation's construction view script |
| E-ui-011 | code reading | | GFX-OBJ-8.3 precise rule (play test 11) |
| E-gfx-070 | code reading and data | (session 2026-11-15 day): the ship's per-part draw and its colour weight; the ship models' material groups, `PV_<t>_Ship01.dds` / `PV_<t>_Ship01PC.dds` and their alpha; `ships.fx`, `mesh_shader.fx` | GFX-OBJ-2.8a, T48 |
| E-gfx-071 | code reading and pictures | (session 2026-11-16 night): a clip's Particle event (start once per play, place at the model or a bone every frame, end by duration or with the play); E-trace-031's forester pictures | GFX-OBJ-8.3c |
| E-trace-032 | pictures of the original | (session 2026-11-16 day): the leveller's dust on a sloped medium site, speed 1, zoom 1 and 2 | GFX-OBJ-8.3c |
| E-gfx-040 | code reading | (session 2026-11-07; also filed as E-sav-010, E-ui-020) | GFX-OBJ-1.4 (Q-1027) |
| E-gfx-039 | code reading | (session 2026-11-06): `house_shader.fx` | GFX-OBJ-1.4, 2.1 |
| data | tables | `settings/terrain_static_data.xml` (LandscapeElements), `buildings.xml`, `items.xml`, `animals.xml`, `ships.xml`, `character/settler_config.xml`, `character/color_slots.xml`, `scripts/menu.cfg` | |
| data | shaders | `gfx/fx_20/house_shader.fx`, `house_decal.fx`, `house_flag.fx`, `flag_shader.fx`, `border_shader.fx`, `tree_shader.fx`, `doodads_shader.fx`, `rock_shader.fx`, `signs_shader.fx`, `stone_shader.fx`, `weed_shader.fx`, `item_shader.fx`, `skin_color_mesh.fx`, `animals.fx`, `donkey.fx`, `ships.fx`, `street_shader.fx`, `shared.inc` | |
| data | models | `character/models/settler_model.GR2`, `animals/deer_model.GR2` (skeleton rest poses, read on the investigation side) | facing |

## Open questions

- Q-091: the transition-mask scale (see `GFX-terrain-patterns.md`).
- Q-092: which roads use the second street texture; the exact strip
  tessellation and crossing geometry.
- Q-093: camera near / far planes and the option defaults (scroll speed).
- Q-094: a building's colour-variation seed, and which military flag model
  and texture offset a building uses.
