# Films: from the original's Bink files to the screen

The original plays Bink videos (`data/video/*.bik`). There is no Bink spec, and
OpenS2X may not take a decoder from another project. So the films are converted
(UI-MENU-0.1) **at install, from the player's own game files, with FFmpeg**. FFmpeg runs as a separate tool; none of its code (or any Bink
code) is in this repository or in the program.

## Which films

| Film | File | Shown | Spec |
|---|---|---|---|
| `title` | `title.bik`, 616 x 665, 25 fps, 1308 frames, no sound | looping on the main menu's left sheet at panel (56, 62), 1:1 | UI-MENU-1.1a, T8 |
| `intro` | `Intro.bik`, 1024 x 576, 25 fps, 3366 frames, sound | at start-up, full screen, shape kept, centred on black, subtitles from `Intro.xml`; a key or left click ends it | UI-MENU-0.1, 0.3, T14, T15 |

The three company logo films (`Ubisoft`, `BB_Logo`, `funaticsAlpha`) are not
converted or shown: a project decision (UI-MENU-0.1). `extro.bik` is not used yet.

## Converting the films on Windows

With the download only (no source code): install [FFmpeg](https://ffmpeg.org)
so that `ffmpeg` runs in a Command Prompt, open a Command Prompt in the
folder of `OpenS2X.exe`, and run these four commands. Put your own game
folder (the one that contains `data`) in place of the GOG default:

```bat
mkdir video\title video\intro
ffmpeg -i "C:\GOG Games\Settlers 2 10th Anniversary\data\video\title.bik" -an -c:v mjpeg -q:v 4 -pix_fmt yuvj420p -f mjpeg video\title\frames.mjpeg
ffmpeg -i "C:\GOG Games\Settlers 2 10th Anniversary\data\video\Intro.bik" -an -c:v mjpeg -q:v 4 -pix_fmt yuvj420p -f mjpeg video\intro\frames.mjpeg
ffmpeg -i "C:\GOG Games\Settlers 2 10th Anniversary\data\video\Intro.bik" -filter_complex "[0:a:0][0:a:2]amix=inputs=2:normalize=0:duration=longest[a]" -map "[a]" -c:a pcm_s16le video\intro\audio.wav
```

`[0:a:2]` is the English voice; for German take `[0:a:1]`, for French
`[0:a:3]`. Then write two small text files with Notepad, each line as shown,
for the GOG edition's films:

`video\title\info.txt`:

```text
fps=25/1
width=616
height=665
frames=1308
```

`video\intro\info.txt`:

```text
fps=25/1
width=1024
height=576
frames=3366
```

The next start of OpenS2X plays the intro and shows the town on the main
menu's left sheet. The films take about 500 MB. The source code's
`scripts/convert-films.sh` does the same on Linux or WSL and reads these
values from the files.

## The converted form

`scripts/convert-films.sh [--force] <game dir> <out dir>` writes, per film,
`<out>/<name>/`:

- `frames.mjpeg`: every frame as a baseline JPEG, one after the other (FFmpeg's
  `-c:v mjpeg -q:v 4 -pix_fmt yuvj420p -f mjpeg`). About 130 MB for the title
  and 360 MB for the intro.
- `audio.wav`: 16-bit PCM, only when the film has sound. By UI-MENU-0.3a the
  intro's four tracks are track 0 (music and effects) and three voice tracks, one
  per language; the original plays track 0 and the current language's voice track
  together at one volume. The script asks `s2-app voice-track
  <game>/data/scripts/menu.cfg` for that track (`menu.setLanguage` looked up in the
  `menu.setVideoVoiceStream` map: de 1, gb 2, fr 3; any other language 2; the
  shipped English game: 2) and sums the two with FFmpeg's
  `amix=inputs=2:normalize=0` (no halving). `S2_APP` may name a built `s2-app`;
  else the script runs it with cargo from the repo, and without either it takes
  track 2. A film with fewer than four tracks plays every track it has. The
  shipped intro's sum (tracks 0 + 2) reaches full scale in 0.013 % of its samples,
  which are clipped, as a mixer summing the two would.
- `info.txt`: `fps=` (as FFprobe gives it, e.g. `25/1`), `width=`, `height=`,
  `frames=` (packets counted).

Run it once into a `video` folder beside the program (`<out dir>` =
`<folder of OpenS2X.exe>/video`). Without the films the program shows the
parchment where the town would be and starts at the main menu.

Why Motion-JPEG: every frame is a self-contained JPEG, so playing needs nothing
but a JPEG decoder; the decoder is `zune-jpeg` (pure Rust, MIT OR Apache-2.0 OR
Zlib, pinned in `crates/viewer/Cargo.toml`), which builds for the Windows target
with no native libraries. Sound is plain WAV, which Bevy's audio already plays. A
modern codec (VP9, AV1) would be ten times smaller but needs a large decoder or a
native library.

## Playing (`crates/viewer/src/films/video.rs`)

- The program looks for the films in `S2_VIDEO_DIR`, else `video\` beside the exe.
- `Film::load` indexes the JPEG images of `frames.mjpeg` in one pass (start of
  image FF D8 FF to end of image FF D9) without keeping the file in memory; each
  frame shown is read from the file and decoded (`zune-jpeg`, to RGBA) into an
  image the interface draws as `Atlas::Film`.
- Frame f shows from f / fps seconds after the film started (real time); the
  title wraps around (frame 0 again at 52.32 s), the intro ends after its last
  frame.
- The intro's sound plays at the master volume through the program's audio
  output; the menu music waits until it ends. The pointer is hidden while it runs.
- Subtitles: `s2_assets::subtitles` reads `Intro.xml` (frame, text key) cues; the
  key's line of `language/<lang>/intro.txt` (its `<hcenter>` mark removed) is drawn
  in `font_big_white`, wrapped and centred in 80 % of the film's width, placed by
  UI-MENU-0.3 (in the bar under the film when it is taller than the text, 16 px
  above the screen's bottom when there is no bar, else on the film's bottom edge).

## Tests

`cargo test -p s2-viewer video` checks the frame splitting, `info.txt`, the
timing (T8), the layout (T14, T15) and the cues; with `S2_VIDEO_DIR` set it also
loads and decodes both converted films.
