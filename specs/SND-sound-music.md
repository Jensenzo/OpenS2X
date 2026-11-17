# SND: Sound effects, ambient sounds and music

Status: draft (read from the data files and the binary; not checked in the running game)
Owner question(s): plan 6.6 (investigation chunk 3); Q-098, Q-099 raised
Game build: 11757

## Summary

All sounds are defined in one Lua table (`data/scripts/properties/sounds.lua`):
104 named sounds with a type (ambient, music, GUI, sound), one or more files,
loop flag, priority, volume and a 3D minimum distance. Sound effects are
fired by animation events (the `Sound` children of the animation XML) or by a
few game events in code (steps, doors, tree fall, catapult, collapse). Music
is a playlist of the tribes present in the game, interrupted by a battle loop
while the player's soldiers fight. Ambient loops are emitters placed in the
map. Four volume settings exist: master, music, UI, sound.

## Rules

### SND-1 Definitions

- **SND-1.1** A sound is identified by the 32-bit hash of its Lua name (the
  `sn_create` argument). Its attributes: display name, type (0 ambient,
  1 music, 2 GUI, 3 sound), file list, looping, priority, volume (0..255,
  default assumed 255), 3D flag, 3D minimum distance, 3D maximum distance,
  2D distance, music tribe (0 Roman, 1 African, 2 Chinese), percussion flag.
  Confidence: high. The full table is in the investigation's notes.
  Class rule: every entry of `sounds.lua` is a sound of this form, played by the rules
  SND-2 to SND-8 that name it. Covers (inventory): *sound* from
  `scripts/properties/sounds.lua`.
- **SND-1.2** When a sound with several files is played, one file is chosen
  at random (uniform) each time. Confidence: high.
- **SND-1.3** Files live under `data/sound/` (`ambients/`, `music/`,
  `Human/<profession>/`, `Buildings/<building>/`, root for the GUI files);
  the file names in the table match case-insensitively. Confidence: high.
  Class rule (2026-10-24): every file under `data/sound/` is played only as one of the
  files of a definition; four files are named by no definition and are never played:
  `Buildings/shipyard/openingChest.wav`, `Human/Fisherman/fish_squishing.wav`,
  `uiCancel.wav`, `music/RomePer1.mp3` (the definitions name other files for these
  purposes). (Corrected 2026-10-24: `music/Credits.mp3` and `music/Jingle_Bad.mp3` are
  named, by the credits music and the lost jingle; the first list compared the names
  with the wrong case.) The file-open trace (TOOL-trace TRACE-7.5) agrees: over the
  sessions of 2026-10-13 to 2026-10-23 the original never read the four, while it read
  every other file under `data/sound/` it had a use for; at start-up it opens most music
  files at once (the other tribes' tracks, briefing, credits, jingles, battle loop), and
  the Roman in-game tracks one at a time as they start. Covers (inventory): *sound file*
  from `data/sound/*` except `data/sound/buildings/shipyard/openingchest.wav`,
  `data/sound/human/fisherman/fish_squishing.wav`, `data/sound/uicancel.wav`. Covers
  (inventory): *music file* from `data/sound/music/*` except
  `data/sound/music/romeper1.mp3`.

### SND-2 Sound effects

- **SND-2.1 Animation events.** A `Sound` child of an animation
  (`character/settler_animations.xml`, `settings/buildings.xml`) names a
  sound by its display name (compared without case, for example
  `scholar hammer`, `chopping Wood`, `mining wheel loop` with a `duration`);
  it plays at the animation's object when the clip reaches `start`
  (GFX-ANIM-2.3). Confidence: high.
- **SND-2.2 Code events.** Independently of animations:
  - footsteps (`sound_steps`) while a visible settler walks, every
    0.5 s divided by the animation global speed (`globalSpeed` 1.8, so every
    0.278 s of logic time; GFX-ANIM-3.6), at the settler;
  - `sound_dooropen` when a building door opens and `sound_doorclose` when
    it closes (GFX-ANIM-6.1), only for visible buildings;
  - `sound_treefall` once when a tree starts falling;
  - `sound_catapult_shoot` at a catapult's launch and `sound_catapult_impact`
    at the impact field;
  - `sound_building_collapsed` with the demolition dust when a building is
    destroyed.
  Confidence: high.
  **Which definition a code event plays (2026-11-08 b, Q-186 a; E-gfx-041, constants in the
  executable checked against `scripts/properties/sounds.lua`):** the code names a sound by its
  definition id (the hash given to `sn_create`), never by display name. Exactly 19 definition
  ids occur in the executable: the music `Intro Music`, `Rome` / `Africans` / `Chinese Music`
  and `... Percussion`, `Battle Music` (twice), `Briefing Music`, `Winning Jingle`, `Loosing
  Jingle` and `Credits Music`, and the sounds `steps`, `opening Door`, `closing Door`, `falling
  Tree`, `catapult shoot`, `catapult impact`, `building collapsing`. So pairing by the
  definitions' names gives the same sounds. Intro, briefing, won, lost and credits ids are kept
  together by one interface object as five slots, 0 intro, 1 briefing, 2 won, 3
  lost, 4 credits, played by one call that does nothing when that slot is already playing and
  otherwise stops the current music and starts the slot's. **When (2026-11-08 b):**
  the book dialog of briefings and debriefings (`uiBriefing`, Next / Previous Page, CONTINUE)
  starts `Briefing Music`; the end dialog starts `Winning Jingle` on VICTORY and
  `Loosing Jingle` on DEFEAT, and so do the stand-alone victory screen
  (`ui_briefing_screen_won`) and defeat screen (`!You have lost the Map!`,
  `ui_briefing_screen_lost`); the credits start `Credits Music`; the
  menus start `Intro Music`. Confidence: high (code).
  **Which files (2026-11-17 day, E-snd-005: code reading and the definitions):** each of these
  definitions names one file, so no choice is made (SND-1.2): `Winning Jingle` plays
  `music/Jingle_Good.mp3`, `Loosing Jingle` `music/Jingle_Bad.mp3`, `Battle Music`
  `music/Battle_loop.wav` (looping, SND-3.4). An attack on the local player brings two sounds:
  the "You are under attack!" row is a military message, so it plays `uiMessage1`
  (`uiMessage1.wav`) when it is added, only while the messages window is shown (`UI-ingame.md`
  UI-GAME-11.4; a repeated attack within 60 timer units only resets the row and adds nothing, so
  it sounds once); and the fight itself starts the battle loop (SND-3.4). The original keeps no
  sound log (its log files are crash dumps and network logs), and the file-open trace cannot tell
  them apart (it opens these files at start-up), so the files rest on the code and the
  definitions; not heard in a recording.
- **SND-2.3 Audibility (corrected 2026-10-17, E-pt5-001).** A positioned
  sound is played only when the **game speed** setting is at most 3 (always
  true for the three speeds of the game; looping sounds skip this test) and
  the listener (SND-8.1) is closer than 30 x the sound's 3D minimum distance
  to the source (full 3D distance, squared values compared); otherwise the
  event is dropped. The camera's zoom level does **not** gate sounds: the
  earlier reading "zoom at most 3" took the game-speed field for the zoom.
  Confidence: high (E-pt5-001).
- **SND-2.4 Position.** A sound fired for a moving object follows it (its
  position is re-read every audio update); a sound fired for a place keeps
  that position. The listener is the point of SND-8.1, not the camera
  itself. Confidence: high.
- **SND-2.5 Volume.** A channel's volume is the definition's volume scaled
  by the type's volume setting (SND-5) and the master volume; the exact
  product was not read (Q-098). Confidence: medium.
- **SND-2.6 Channels (answered 2026-11-04, Q-1005; E-snd-002).** The sound library is
  opened at 44100 Hz with **32 channels**, one pool, and the game never changes that.
  Finished channels are released at the next audio update. Confidence: high for the number
  and for 2.6a to 2.6d, read in the game and in the library the game ships (FMOD 3.75); not
  heard in the running original.
  - **2.6a Who holds channels.** Each sound that plays holds one channel. The music stream
    holds one and at most 4 ambient loops hold one each; those are set aside and no other
    sound can take them. Everything else (effects, interface sounds) shares what is left, so
    at least 27 channels.
  - **2.6b When a sound starts.** Channels are tried in a ring, beginning after the last
    channel handed out, skipping the ones set aside. The first free channel is taken. If
    none is free, the candidate is the channel with the **lowest priority** playing; among
    equal priorities the **quietest** (its present level after distance and pan, 0 .. 255:
    the mean of its left and right level, SND-8.3a 4),
    and among equal levels the first in the ring. If the candidate's priority is **at most**
    the new sound's priority it is cut at once and the new sound takes its channel;
    otherwise the **new sound is refused**: it is lost, with no retry and no queue. The ring
    position moves on only when a channel was handed out.
  - **2.6c What the priorities do.** Higher is more important (SND-1.1, table in the notes):
    steps 0, doors and a few work sounds 8, most work sounds 9 to 11, hits and deaths 12,
    interface 100 to 200. So with every channel busy a step can only replace another step,
    and is refused as soon as no step is playing; a hammer (11) replaces anything up to 11,
    steps first; an interface sound replaces any effect. Priority is compared before
    distance: a near door (8) goes before a far hammer (11).
  - **2.6d One sound repeating.** Nothing limits it: a sound may start again while it is
    still playing, as often as events come, up to the channels available. There is no limit
    per sound or per file and no "only one at a time" mode, in the game or in the way it uses
    the library; each start picks a file again (SND-1.2), so the same file may overlap
    itself.
  - **2.6e Game speed.** Sound events run on logic time (GFX-ANIM-2.3, 3.6), so at speed 2
    and 3 they come 2 and 3 times as often. The samples keep their length and pitch (the
    game never sets a channel's frequency), so more of them overlap and the limit of 2.6a
    is reached sooner. The speed test of SND-2.3 always passes.
  - **2.6f Hardware voices.** The library asks DirectSound for free hardware 3D buffers and,
    if there are any, plays 3D effects on those in a second pool of that size. Windows since
    Vista offers none through DirectSound on ordinary drivers, so 32 is the whole budget.
    The 32 is high; that no hardware pool exists on a given PC is general knowledge, not
    measured here (medium). Implement 32, no second pool.
  - **Not read:** whether a looping work sound (a mine's or smeltery's loop) is started
    again after being cut.

### SND-3 Music

- **SND-3.1 Menus.** The menu system plays `music_main` (looping) in the
  main menu, `music_briefing` in a mission briefing, `music_won` /
  `music_lost` (once) on the end screen, `music_credits` in the credits; a
  state change replaces the music. Confidence: high.
- **SND-3.2 In-game playlist.** When a game runs, every present player slot
  adds its tribe's two sets to the playlist, once per slot: Roman
  `music_roman` (Rome1..4) + `music_romanPer` (RomePer2..4), African
  `music_africans` (Nubia1..3) + `music_africansPer` (NubiaPer1..3), Chinese
  `music_chinese` (China1..3) + `music_chinesePer` (ChinaPer1, 1a, 1b, 2, 3).
  The playlist is therefore the union over the tribes in the game.
  Confidence: high.
- **SND-3.3 Playback.** One stream plays at a time; a new track starts at
  volume 0 and fades up to the music volume; when a track ends the next
  entry is chosen and a short gap elapses before it starts (gap length and
  the order rule, sequential or random, open: Q-098). Confidence: medium.
- **SND-3.4 Battle music.** Every frame the game tests whether the local
  player is in a fight. While it is, a hold timer is pushed to now + 20 s,
  and if no music change is pending `music_battle` (a loop) is started once.
  When no fight has been seen for 20 s the battle loop is stopped (only if
  it is the current music) and the playlist resumes. Confidence: high.

### SND-4 Interface sounds

- **SND-4.1** `uiClick1` on every button release (UI-GAME-1.4); `uiClick0`
  on list and tab controls; `uiDialog0` when a dialog opens; `uiCancel`
  with a refused action (error box; HUD and flag-menu refusals);
  `uiMessage0` / `uiMessage1` when a message arrives (two message kinds);
  `uiKey0` for a typed key in a text input; `uiRemove` when a message is
  removed. GUI sounds are 2D and use the UI volume. Confidence: high.

### SND-5 Volume settings

- **SND-5.1** Four settings: master (the library's SFX master, 0..1),
  music, UI, sound, each 0..1, shown as sliders in the sound settings page
  (UI-MENU-4.2); the ambient volume (a fifth internal value) is not exposed.
  Defaults: master 1.0, ambient 1.0, sound 1.0, music 0.3, UI 1.0.
  Changing a setting re-applies it to the playing channels of that type.
  Confidence: high for defaults and behaviour; where the settings are saved
  is not traced (Q-098).

### SND-6 Ambient sounds

- **SND-6.1** A map carries ambient emitters (FMT-S2M-6.3): a sound id of
  the `ambient_*` set and a field. The thirteen ambients are looping, not
  3D, priority 100, 2D distance 50, volume 64 (lava 32): Beach, three
  desert winds, two forests, two meadows, four waters (stream, swamp, waves,
  river), lava. Confidence: high.
- **SND-6.2** The emitters are handed, with their positions, to the sound
  side each update; which emitters play and how the 2D distance attenuates
  them: SND-7.4. Confidence: high.

### SND-7 Answers to Q-098 and Q-099 (2026-10-17)

- **SND-7.1 Volume product (closes SND-2.5).** A channel plays at
  `int(definition volume (0..255) x type volume (0..1))`; the master setting is
  the library's SFX master volume (`master x 255`) applied on top by the library.
  The music stream plays at `music volume x 255`. Changing a type's slider
  re-applies the product to every playing channel of that type. Confidence: high.
- **SND-7.2 Track order and gap (closes SND-3.3).** The playlist is played in
  order; when a track ends the player advances to the next entry and starts it at
  the first audio update after the end: the gap timer is created at 0 and never
  set, so there is no pause between tracks. Confidence: medium-high (no writer of
  the timer found).
- **SND-7.3 Storage (closes SND-5.1).** The five volumes are fields 11 to 15 of
  the settings file (SET-2), in the order master, music, UI, sound, ambient.
  Confidence: high.
- **SND-7.4 Ambients (SND-6.2; answered 2026-11-05, Q-099; E-snd-003).** Every audio update, with L the listener position of SND-8.1
  and t the last frame's real time in seconds (not game time; 0.025 in fixed-step mode):
  1. **Choice.** Over all of the map's emitters, the horizontal distance r = sqrt((Lx - Ex)^2 +
     (Lz - Ez)^2) (height ignored) ranks them; the **four nearest with different sounds** are
     chosen: per sound only its nearest emitter counts (a farther emitter of a sound already in the
     list is skipped, a nearer one replaces it), ties keep the earlier emitter, and there is **no
     distance cut** here: the four nearest are chosen however far they are.
  2. **Keep or hand over.** A chosen emitter whose sound already plays keeps that channel; if the
     channel was playing for another emitter of the same sound, it is handed to the chosen one
     without restarting (the sound continues, its level now follows the new emitter).
  3. **Fade out.** A playing ambient channel that is not kept has its wanted level set to 0; once
     both its wanted and its current level are below 1 it is stopped and its channel freed.
  4. **Start.** For each chosen emitter whose sound does not play, while fewer than four ambient
     channels play: its looping sound starts on a free channel (SND-2.6a: reserved while it
     plays) at current level 0, paused when the ambient player is paused. A refused start is
     tried again in the next update.
  5. **Level.** For each playing ambient, with D its definition's 2D distance (50 for all
     thirteen) and V its definition volume (0 .. 255): wanted = int(V x min(1, 2 - r / D) x
     max(0, 1 - 0.0125 |Ly - Ey|)), so **full up to r = D, falling linearly to 0 at r = 2 D**
     (negative beyond, i.e. silent) and fading with the height difference between the listener
     and the emitter (silent at 80 units). The current level moves toward the wanted one by at
     most **255 x t per update** (it jumps to it when closer). The channel volume is
     int(int(current) x the ambient type volume, SND-7.1) (a negative value plays as silence).
  The listener's height L.y comes from SND-8.1 (the eye plus the offset); the emitter's from its
  field's height (medium: the emitter position's y was not traced to its source). So up to four
  loops play at once, each the nearest emitter of its sound; a fifth sound waits until one of the
  four drops out of the four nearest. Confidence: high for the choice, the hand-over, the formula
  and the fade rate; medium for the emitter's height.

### SND-8 Sound at a distance (2026-10-17, Q-099 part, Q-186 c)

- **SND-8.1 Listener.** Every audio update the listener is placed at
  L = E + D x (0.8 x h / |D|^2), where E is the camera's eye position, h is
  the eye's height above the terrain directly below it, and D is built from
  the camera's smoothed unit view direction v (GFX-VIEW-3.3) as
  D = (v.y, v.y, v.z): the original puts the direction's **y** component into
  the x slot as well (a slip in the original; reproduce it, it shifts the
  listener sideways). If |D|^2 is 0 the offset is 0. The listener's forward
  vector is f = D / |D|; its velocity is zero; its **up vector** is world up with
  the forward part taken out, u = f x (Y x f) = Y - (f . Y) f with Y = (0, 1, 0)
  (answered 2026-11-06, Q-1015 a; E-snd-004; when f . Y is exactly 0
  the game uses another fixed axis in place of Y, which no game camera reaches).
  Confidence: high (E-pt5-001).
- **SND-8.2 3D sounds (corrected 2026-11-04, Q-1005; E-snd-002).** A definition is 3D
  unless it sets `set3DSound(false)`: the 13 ambients, the 12 music sets and the 8 GUI sounds
  do, so every in-game effect (the 71 of type Sound) is 3D. A 3D sound uses its definition's
  3D minimum distance (default 1) and 3D maximum distance (default 1e9; no definition sets
  one). **The game sets the library's roll-off factor to 4** once, when the sound system
  starts (the earlier "no roll-off factor, default 1.0" was wrong: the call goes through a
  jump stub that the search missed). It sets no distance factor, so one world unit is one
  metre. Confidence: high (E-snd-002; E-pt5-001).
- **SND-8.3 Falloff (corrected 2026-11-04, Q-1005; E-snd-002).** The distance factor of a 3D
  sound is **g = 1 within the minimum distance, else g = min / (min + 4 x (dist - min))**,
  with dist the 3D distance from the listener to the source, capped at the maximum distance
  (never reached in practice). The library keeps it as a whole level, g x 255 cut to an
  integer, and multiplies the channel volume of SND-7.1 (definition volume x type volume,
  under the master) by that level / 255. So a sound falls off much faster than the 1 / dist
  stated here before: a sound at twice its minimum distance plays at 1/5, not 1/2. Together
  with SND-2.3 a sound is heard up to 30 x min and is then cut, at g = 1/117 there (level 2
  of 255) whatever its minimum distance: chop (min 20) up to 600 units, steps (min 3) up to
  90. Confidence: high for the gain law, read in the library (E-snd-002).
- **SND-8.3a Pan (answered 2026-11-06, Q-1015 b, c; E-snd-004: a reading of fmod.dll).** A 3D sound's pan is computed by the
  library every update from the listener of 8.1:
  1. R = u x f (the listener's up crossed with its forward, the screen's right in the game's
     left-handed frame; a unit vector since u and f are unit and perpendicular). n = the unit
     vector from the listener point L to the sound (the full 3D vector; a sound exactly at L
     gives n = 0).
  2. pan = 128 + trunc(128 x (n . R)), limited to 0 .. 255 (0 full left, 255 full right, 128
     centre). The library's pan separation is never set by the game (1.0), so this stays.
  3. Gains: with v = trunc(channel volume x distance level / 255) (8.3, 0 .. 255) and the table
     T(p) = min(255, floor(16 sqrt(p))), left = trunc(T(255 - pan) x v / 255), right =
     trunc(T(pan) x v / 255). This is an equal-power law: a centred sound (pan 128) plays at
     180 / 255 on the left and 181 / 255 on the right (about -3 dB each), a sound fully to one
     side at v on that side and 0 on the other.
  4. The channel's "level" of SND-2.6b (the quietest-first choice) is the **mean** (left + right)
     / 2, cut to an integer.
  2D sounds (ambients, music, interface) are not panned by this rule; the pan the game gives them
  was not read (the library's special "stereo" pan plays both sides at v, any other pan uses the
  table of 3). Confidence: high (library code; the table was compared with the formula for all
  256 entries); medium that the game never changes the separation (no call found).
- **SND-8.4 Ambients and footsteps.** Footsteps are ordinary 3D sounds
  (`sound_steps`, min 3, SND-8.3) fired at the walking settler (SND-2.2).
  Ambient definitions are 2D: the library applies no distance attenuation to
  them; which emitters play and how their 2D distance (50) is used is still
  open (Q-099). Confidence: high for footsteps, low for ambients.
- **SND-8.5 What is heard zoomed out (corrected 2026-11-04, Q-1006).** With SND-8.1 to 8.3
  the animation events and the code events of SND-2.2 stay audible at all five zoom levels
  when looking at their source (8.T2), though quietly when zoomed out. **Footsteps are the
  exception:** the formula would let a step be heard (8.T3), but a step fires only near the
  camera's eye (GFX-ANIM-3.6), which decides. See SND-8.6. A program that drops sounds at zoom
  4 (the old SND-2.3 reading) or measures the cut-off from the camera's eye with a smaller
  radius goes silent when zoomed out. Confidence: high.
- **SND-8.6 Footsteps by camera (answered 2026-11-04, Q-1006; E-snd-002).** The test of
  GFX-ANIM-3.6 measures from the camera's eye and is the only gate that decides whether a
  step exists; SND-8.3 and SND-2.3 then apply to a step that has fired. Since the eye stays at
  least 10 units above the ground below it (GFX-VIEW-3.3) and the test asks for 7.07 at most,
  **on level ground no settler ever steps audibly, at any zoom**, zoom 0 included.
  | Camera | Steps fire | Share of camera positions with a walkable field in reach |
  |---|---|---|
  | zoom 0 | only where the ground rises steeply next to the eye | 3.2 % over all maps (0 to 7.9 % by map) |
  | zoom 1 | hardly ever | 0.07 % |
  | zoom 2, 3, 4 | never | 0 |
  | following a settler (O key, camera mode 3) | yes: the eye trails him about 4.6 (behind) or 5.5 (ahead) units away, so he and those next to him step; the pose is `GFX-world-view.md` GFX-VIEW-3.7 | always, for him |

  So the sound belongs to the follow camera and to slopes at zoom 0. Shares from
  the investigation's footstep reach script over 34 maps, 565,100 camera positions: settled
  camera at yaw 0, one target per field; not modelled are the ground smoothing, the eased eye
  and the other two yaws. Confidence: high for the rule and for level ground; medium for the
  shares; medium for the follow distance, which eases 5 % per frame and so lags a walking
  settler a little. A look at the original would confirm it: silence on level ground at zoom
  0, steps with O.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| sound_count | 104 (13 ambient, 12 music, 71 sound, 8 GUI) | | high | E-snd-001 data |
| audibility_factor | 30 x 3D min distance | world units | high | E-snd-001 |
| audibility_speed_gate | game speed <= 3 (looping sounds exempt); not the zoom | | high | E-pt5-001 |
| listener_offset | 0.8 x camera height above ground, along (v.y, v.y, v.z) / |D|^2 | world | high | E-pt5-001 |
| 3D defaults | min 1, max 1e9, distance factor 1; chop min 20, steps min 3 | world | high | E-pt5-001; sounds.lua |
| rolloff_factor | 4 (corrected 2026-11-04) | | high | E-snd-002 |
| channels | 32 at 44100 Hz; music 1, ambient loops at most 4 set aside | | high | E-snd-002 |
| step_gate | squared distance to the camera's eye <= 50; counter +5 / -3, 0 .. 100 | world | high | E-snd-002 |
| step_period | 0.5 / speed scale | s | high | E-snd-001 |
| battle_hold | 20 | s | high | E-snd-001 |
| volume_defaults | master 1.0, ambient 1.0, sound 1.0, music 0.3, UI 1.0 | | high | E-snd-001 |
| music_fade_scale | 256 | volume units | medium | E-snd-001 |
| ambient_2d_distance | 50 | world units | high | sounds.lua |
| gui_volumes | click 190, dialog 255, message 200, cancel 128 | 0..255 | high | sounds.lua |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | woodcutter at distance 500 from the listener | chop event | `chopping Wood` plays (30 x 20 = 600 > 500) with g = 20 / (20 + 4 x 480) = 0.0103 |
| T2 | same at distance 700 | chop event | nothing plays |
| T3 | any sound event at zoom 4, source under the camera target | event | plays (the zoom does not gate sounds; 8.T2) |
| T3a | game speed 3, a chop | event | plays (3 <= 3) |
| 8.T1 | flat ground at height 0, low water, camera settled at yaw 0 on target (0, 5, 0) | listener per zoom 0..4 | eye (0, 10, 0) / (0, 20.21, -14.48) / (0, 42.02, -19.83) / (0, 60.53, -29.75) / (0, 79.04, -39.67); listener (-2.20, 7.80, 7.00) / (-7.13, 13.08, -4.97) / (-16.40, 25.62, -7.53) / (-23.62, 36.91, -12.03) / (-30.85, 48.20, -16.53) |
| 8.T2 | same, woodcutter chopping at (0, 0, 0) (on the ground under the target) | volume factor g of `chopping Wood` (min 20) at zoom 0..4 | distance 10.71 / 15.70 / 31.34 / 45.44 / 59.56; g = 1.000 / 1.000 / 0.306 / 0.164 / 0.112; channel volume = round(255 x sound setting) x g under the master |
| 8.T3 | same, a step fired there anyway (only the follow camera or a slope does that, SND-8.6) | `sound_steps` (min 3) at zoom 0..4 | g = 0.089 / 0.056 / 0.026 / 0.017 / 0.013; all within the 90-unit cut-off |
| 8.T4 | same flat ground, a settler walking under the camera at zoom 0 to 4 | watch for steps | none: the eye is 10 or more units away, squared distance over 50 |
| 8.T5 | camera following a walking settler (O), settled | watch for steps | one every 0.278 s of logic time, from him |
| 2.T1 | 27 effects playing, music and 4 ambients on, every effect priority 11 | a step (0) starts | refused; nothing is cut |
| 2.T2 | same, one effect priority 8 at any level, the rest 11 | a hammer (11) starts | the priority 8 one is cut, however close it is |
| 2.T3 | same, all priority 11, levels 40, 12, 12 and the rest higher | a hammer (11) starts | the first level 12 channel in the ring is cut |
| 2.T4 | one woodcutter, nothing else playing | the same sound fires three times faster than it ends | all three overlap, no limit per sound |
| 2.T5 | speed 3 against speed 1, same scene | count sound starts per real second | three times as many, samples unchanged in length |
| 7.T1 | 7.4: listener at (0, 20, 0); emitters ambient_forest1 at r = 10, 30, 70 (all height 20), ambient_water1 at r = 60, ambient_meadow1 at r = 120, ambient_beach at r = 99; all V 100, D 50, ambient volume 1.0; nothing playing | one update with t = 0.02, then many | chosen: forest1 (10), water1 (60), beach (99), meadow1 (120) (the two farther forest1 emitters are skipped); all four start at level 0; wanted levels: forest1 100, water1 int(100 x 0.8) = 80, beach int(100 x 0.02) = 2, meadow1 negative (silent); after the first update the current levels are 5.1, 5.1, 2, -5.1 (255 x 0.02 per update); forest1 reaches 100 after 20 updates |
| 7.T2 | 7.4: as 7.T1 settled; the listener moves so that a fifth sound's emitter (lava) becomes the nearest | updates | lava enters the four, the farthest of the four (meadow1) leaves: its wanted level goes to 0 and it stops when below 1; lava starts once a channel is free (at most four play at once) |
| T4 | settler walking, speed scale 1 | 2 s | four step sounds, each a random one of 10 files |
| T5 | game with a Roman and a Chinese player | start | playlist = Rome1..4, RomePer2..4, China1..3, ChinaPer1, 1a, 1b, 2, 3 |
| T6 | playlist running, a fight starts | 0 s / 25 s after the last blow | `Battle_loop` starts within the frame / stops 20 s after the fight |
| T7 | settings defaults | open sound settings | sliders at 1.0, 0.3, 1.0, 1.0 (master, music, UI, sound) |
| T8 | any dialog | open | `uiDialog0` plays once |
| T9 | SND-8.3a: a 3D sound at full volume within its minimum distance (v = 255): (a) at the listener's right (n = R), (b) straight ahead or below the listener (n . R = 0), (c) n . R = 0.5 | pan and the two levels | (a) pan 255: left 0, right 255; (b) pan 128: left 180, right 181, level 180; (c) pan 192: left T(63) = 126, right T(192) = 221, level 173 |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-snd-001 | code reading | | sound manager, play, defaults, volume set, device update, mover channels, random file, music, step, doors, tree, catapult, collapse, settings, ambient chunk, click, dialog, error box |
| E-pt5-001 | code reading | | speed gate, listener, 3D sample setup and defaults; test values by the investigation's listener script |
| E-snd-002 | code reading | `fmod.dll` 3.75 | channels, roll-off, channel choice, footstep gate, follow camera |
| E-snd-003 | code reading | the ambient player, the choice of the four, the level and its easing, the frame seconds, the sound update | SND-7.4, 7.T1, 7.T2 |
| E-snd-004 | code reading | (= E-gfx-039): the listener up vector; fmod.dll: the listener attributes call (right = up x forward), software 3D update (pan = 128 + 128 n . R), gains (mixed level (L + R) / 2), pan table = min(255, floor(16 sqrt p)) | SND-8.1, 8.3a, 2.6b, T9 |
| E-snd-005 | code reading and data | (session 2026-11-17 day): the music slots' definitions in `sounds.lua` (one file each), the message sound by kind, the original's log files | SND-2.2 "Which files" |
| E-snd-001a | data | `data/scripts/properties/sounds.lua`, `data/sound/**` (241 files), `character/settler_animations.xml`, `settings/buildings.xml` | definitions, files, animation events |
| E-snd-001b | the program's data | `S2DNG.exe`: the Lua binding table, the factor 30.0 | setters and the audibility factor |

## Open questions

- Q-098: the music track order and the gap between tracks; the volume
  product per channel; where the four volume settings are stored.
- Q-099: the ambient emitter playback (which emitters play, the 2D distance
  attenuation, how many at once).
