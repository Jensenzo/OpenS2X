//! The command-line options and the usage text.

use crate::game::START_RESOURCES;
use crate::tools::shots::SHOT_SETTLE_FRAMES;
use bevy::prelude::*;
use s2_sim::PlayerId;
use std::path::PathBuf;

pub(crate) const USAGE: &str =
    "usage: s2-viewer [--map <file> | --map-name <name>] [--game-dir <dir>]
                 [--screenshot-dir <dir>] [--shot <file.png>]

  --version               print the program's version and quit
  --map <file>            a map file to show
  --map-name <name>       a map of the install by name, e.g. campaign01
  --save <file.sav>       continue a save game of the original
  --game-dir <dir>        the game install (default: S2_GAME_DIR)
  --assets <dir>          a folder laid out like the install whose files
                          take the place of the install's (repeatable, the
                          last wins; docs/extending.md)
  --rules-overlay <file>  a Lua file run after the property scripts
                          (repeatable; docs/extending.md)
  --screenshot-dir <dir>  where F12 saves screenshots (default: screens)
  --save-dir <dir>        the save folder (default: Documents\\S2\\saves)
  --autosave <seconds>    the autosave interval for this run (0: off),
                          in place of the profile's
  --profile-dir <dir>     keep the settings and the profile file here, not in
                          the player's folders
  --shot <file.png>       save one screenshot of the first map and quit
  --shot-after <frames>   frames the map is on screen before the shot
  --bench <frames>        time that many frames of the map once its 3D
                          scene is built and drawn for a while, then quit
                          (the frame-time benchmark, docs/test-coverage.md)
  --bench-report <file>   where --bench writes its times
  --bench-pan             with --bench: the arrow keys held, the camera crossing
                          the map at full speed; reports the worst frames
  --bench-drag <pixels>   with --bench: the camera dragged with the middle button
                          instead, that many pixels a frame
  --bench-vsync           with --bench: frames held to the screen's refresh,
                          as a player sees the game (default: as fast as
                          they come)
  --bench-seconds <s>     time the frames for this many seconds instead of a
                          count of frames (a long run, docs/test-coverage.md)
  --bench-log <file>      with --bench or --bench-seconds: a line a second
                          (frames, ticks, memory) and one per frame over
                          33 ms with its parts, written as the run goes
  --bench-invariants      with --bench: every soak invariant after every tick;
                          their time is left out of the frame times
  --mute                  all volumes 0 for this run: the sounds run as ever
                          and nobody hears them (long runs at night)
  --camera <file>         with --compare: the 3D camera at the position and
                          view direction of a look-comparison camera file
                          (JSON with eye and dir, TOOL-trace TRACE-8)
  --compare <file.sav>    the look comparison (docs/compare.md): continue an
                          original save paused in the 3D view, the camera
                          at the headquarters by the start rule, no
                          interface; with --shot the shot is taken one frame
                          after the scene is built (use with --zoom, --size);
                          with a map (.s2m or a name) instead of a save, the
                          same for a new game on it (golden images)
                          (default 30; to catch a clip later on)
  --pause-layer           with --compare: the paused view's layer over the
                          shot (UI-GAME-1.11), for a picture the original
                          took paused
  --no-fog                draw every field as explored (the look comparison
                          of a game the original played with fog of war
                          off; the game itself is unchanged, Q-290)
  --still                 start paused (tree sway, flags, water stand) with
                          the frame clock standing still, so that a --shot
                          is the same in every run (golden images)
  --overlay               start with the placement overlay on
  --model <name>          start in the model view with this model (a file
                          stem such as PB_R_Sawmill01, or a .gr2 path)
  --model-shots <dir>     the model view with the game camera (GFX-VIEW-3
                          at --zoom, default 3) and no interface: save one
                          <file stem>.png of every model into <dir>, then
                          quit (catalogue thumbnails; use with --size)
  --model-list <file>     with --model-shots: only the .gr2 files listed
                          there, one path per line (relative to the game
                          directory or absolute)
  --expand <ticks>        first run a scripted expansion of every player
                          towards its nearest enemy for this many ticks
  --start-resources <n>   start resources setting 0 to 3 (default 0)
  --focus <x,y,scale>     centre the camera on a field, at a zoom (world
                          units per pixel; 0.2 is close)
  --military-panel        start with the military settings panel open
  --attack fight|conquest after the expansion, player 1 attacks until the
                          first duel or the first building it takes
  --ai <slot>=<profile>   make a player a computer player, profile 1 weak,
                          2 normal, 3 strong (repeatable, e.g. --ai 1=2)
  --paused                start with the game paused
  --compact               a short text panel: map, status, attack dialog,
                          messages
  --debug                 the keyboard panels and text overlay instead of
                          the original interface (the keys below)
  --menu                  start at the main menu (UI-MENU-1)
  --menu-screen <name>    start at select, setup, settings, sound, engine,
                          graphics, input, network, save, credits, single,
                          campaign, tutorial or start (screenshots)
  --free-game <map>       start a free game on a free game map through the
                          menus, with the default player slots
  --mission <name>        start campaign mission campaignNN or tutorial
                          lesson tutorialNN as its menu does
  --open <x,y>            open the dialog of a field at start (screenshots)
  --open-tab <n>          with --open: the building window's tab (9 goods,
                          10 settlers, 11 military, 12 expedition)
  --open-global <name>    open tools, military, distribution, transport,
                          statistics, production, inventory, buildings,
                          settlers, fleet, minimap, messages or game-menu at
                          start (repeatable)
  --open-building         open the window of the player's first production
                          building (screenshots)
  --building-type <name>  with --open-building: the first of this type, by
                          its rules name (e.g. WoodcutterHut)
  --context-at <x,y>      with --open-building: open its window as if clicked
                          at this screen point (its bottom-right corner there)
  --watch <name>          the 3D camera follows the worker of the player's
                          first building of this type, or with Carrier the
                          first road carrier carrying a ware (screenshots)
  --until-work <step>     with --run and --watch: play on until that worker
                          is in this step (e.g. AtSpot, Approach, TakeWare)
  --until-door <type>     with --run: play on until a settler walks through
                          the door of the player's first building of that
                          type (its door opens in the first frame; F52)
  --until-felled <ticks>  with --run: play on until a tree of the player's
                          was felled that many ticks ago, the camera on it
                          (the sinking tree, GFX-ANIM-3.8c)
  --until-built <percent> with --scene woodcutter-site: play on until the
                          site's work is that far (100: finished)
  --run <ticks>           play the game this many ticks (with its computer
                          players) before showing it
  --speed <1-3>           the game speed at start (UI-GAME-2.7)
  --hour <h>              start the environment clock at this hour instead
                          of 12 (GFX-VIEW-2.3c; pictures of the night)
  --3d                    start in the 3D view
  --tutorial-step <n>     start a tutorial at this step (0x00 to 0x50;
                          screenshots)
  --end-screen <victory|defeat>
                          show the game as just ended (CAM-3.6; screenshots)
  --size <WxH>            the window's size (default: the profile's
                          resolution for the game, 1280x900 for the viewer)
  --play <script>         the play harness: drive the game with pointer,
                          clicks and keys through its own input systems and
                          check each step (first-ten-minutes); screenshots
                          of the steps go to --screenshot-dir; exit code 1
                          when a step failed
  --play-report <file>    where --play writes its report
  --colour-probe <report> draw quads of known texels under the map's light,
                          read their pixels back and check them against
                          min(255, v x D) (docs/colour.md); exit code 1
                          when one is off
  --log-builders          log every frame a builder is drawn still while the
                          game has him in a work step (clip, duration, loop)
  --zoom <0-4>            the 3D view's zoom step (default 3; 4 is farthest)

keys: N / P next / previous map, WASD or arrows pan, mouse drag pan,
      wheel zoom, F12 screenshot, Esc quit
      B placement overlay; click a field to place a flag, click two flags
      for a road; Q / E page and 1-9 choose a building, 0 or right click
      clears; X removes the building under the cursor; U upgrades the
      military building under the cursor (barracks, guard house, tower,
      fortress)
      I warehouse stock; C (debug) puts the missing materials, a leveller
      and a builder on the construction site under the cursor
      F1 military settings, F2 coin supply and F3 moving soldiers out of the
      building under the cursor, F4 territory tint, F8 borders, F10 messages
      A over an enemy military building: attack dialog (1-4 count, F9 weak /
      strong units first)
      Home at a harbour: prepare an expedition; Insert choose a ship, Numpad
      1-6 sail in a compass direction, End found a harbour, Delete cancel
      M model view (there: N / P model, drag orbit, wheel zoom, M back)
      F11 3D / 2D view; there arrows / WASD pan, middle drag pan, wheel
      zoom, Page Up / Page Down rotate";

pub(crate) struct Options {
    /// `--version`: print the version line and quit (`version.rs`).
    pub(crate) version: bool,
    pub(crate) map: Option<PathBuf>,
    /// `--save`: continue a save game (the original's or ours).
    pub(crate) save: Option<PathBuf>,
    /// `--save-dir`: the save folder in place of `<Documents>\S2\saves`.
    pub(crate) save_dir: Option<PathBuf>,
    /// `--autosave`: the autosave interval in seconds of game time in place
    /// of the profile's (which it does not change).
    pub(crate) autosave: Option<u32>,
    /// `--compare`: the look comparison (docs/compare.md).
    pub(crate) compare: bool,
    /// `--still` (and `--compare`): paused, the real-time clock at 0.
    pub(crate) still: bool,
    /// `--no-fog`: every field drawn as explored.
    pub(crate) no_fog: bool,
    /// `--pause-layer`: the paused view's layer over a `--compare` shot.
    pub(crate) pause_layer: bool,
    pub(crate) map_name: Option<String>,
    pub(crate) game_dir: Option<PathBuf>,
    /// `--assets`: asset folders in front of the install.
    pub(crate) asset_folders: Vec<PathBuf>,
    /// `--rules-overlay`: Lua run after the property scripts.
    pub(crate) rules_overlays: Vec<PathBuf>,
    pub(crate) screenshot_dir: PathBuf,
    pub(crate) shot: Option<PathBuf>,
    /// `--shot-after`: frames on screen before the shot.
    pub(crate) shot_after: u32,
    /// `--profile-dir`: the settings and the profile file live here, not in
    /// the player's folders (the harness's own, `docs/test-coverage.md`).
    pub(crate) profile_dir: Option<PathBuf>,
    /// `--bench`, `--bench-report`: the frame-time benchmark.
    pub(crate) bench: Option<u32>,
    /// `--camera`: the look comparison's camera file (TRACE-8).
    pub(crate) camera: Option<PathBuf>,
    pub(crate) bench_report: Option<PathBuf>,
    /// `--bench-vsync`: the benchmark's frames held to the refresh.
    pub(crate) bench_vsync: bool,
    /// `--bench-pan`: the benchmark holds the arrow keys (F55).
    pub(crate) bench_pan: bool,
    /// `--bench-seconds`: the benchmark's length in seconds.
    pub(crate) bench_seconds: Option<u32>,
    /// `--bench-log`: the running log of a long benchmark.
    pub(crate) bench_log: Option<PathBuf>,
    /// `--bench-invariants`: the soak's invariants after every tick.
    pub(crate) bench_invariants: bool,
    /// `--mute`: all volumes 0 for this run.
    pub(crate) mute: bool,
    /// `--bench-drag`: the benchmark drags the camera with the middle button,
    /// this many pixels a frame (F55).
    pub(crate) bench_drag: Option<f32>,
    pub(crate) overlay: bool,
    pub(crate) model: Option<String>,
    /// `--model-shots <dir>`: a picture of every model, then quit.
    pub(crate) model_shots: Option<PathBuf>,
    /// `--model-list <file>`: the models `--model-shots` takes.
    pub(crate) model_list: Option<PathBuf>,
    /// `--expand <ticks>`: run the scripted expansion of all players first.
    pub(crate) scenario: Option<u64>,
    /// `--ai <slot>=<profile>`: computer players.
    pub(crate) ai: Vec<(PlayerId, u8)>,
    pub(crate) start_resources: u8,
    /// `--focus x,y,scale`: centre the camera on a field at a zoom.
    pub(crate) focus: Option<(u32, u32, f32)>,
    /// `--military-panel`: start with the military settings panel open.
    pub(crate) military_panel: bool,
    /// `--attack fight|conquest`: after the expansion, player 1 attacks
    /// until the first duel or the first building it takes.
    pub(crate) attack: Option<s2_rules::scenario::AttackStop>,
    /// `--paused`: start with the game paused.
    pub(crate) paused: bool,
    /// `--compact`: the text panel shows only the map, the status, the
    /// attack dialog and the messages.
    pub(crate) compact: bool,
    pub(crate) debug: bool,
    /// `--menu`: start at the main menu; `--menu-screen` names another
    /// menu screen to start at (screenshots).
    pub(crate) menu: bool,
    pub(crate) menu_screen: Option<String>,
    /// `--free-game <map>`: start a free game through the menus.
    pub(crate) free_game: Option<String>,
    /// `--mission <campaignNN|tutorialNN>`: start a mission as its menu
    /// does.
    pub(crate) mission: Option<String>,
    /// `--open x,y`: the context dialog of a field opened at start.
    pub(crate) open: Option<(u32, u32)>,
    /// `--open-tab n`: the tab `--open`'s building window shows.
    pub(crate) open_tab: Option<u8>,
    /// `--open-global tools|military|distribution|transport|minimap|messages`.
    pub(crate) open_global: Vec<String>,
    /// `--open-building`: open the window of the player's first production
    /// building once there is one.
    pub(crate) open_building: bool,
    /// `--building-type`: the type `--open-building` looks for.
    pub(crate) building_type: Option<String>,
    /// `--context-at`: the screen point `--open-building`'s window opens at,
    /// as if clicked there (UI-GAME-1.9).
    pub(crate) context_at: Option<[f32; 2]>,
    /// `--watch`: the 3D camera follows the worker of the player's first
    /// building of this type.
    pub(crate) watch: Option<String>,
    /// `--until-work`: `--run` goes on until the watched worker is in this
    /// step.
    pub(crate) until_work: Option<String>,
    /// `--until-felled`: `--run` goes on until a tree of the player's was
    /// felled this many ticks ago; the 3D camera follows it.
    pub(crate) until_felled: Option<u64>,
    /// `--until-door`: `--run` goes on until a settler holds the door of the
    /// player's first building of this type open.
    pub(crate) until_door: Option<String>,
    /// `--until-built`: `--run` goes on until the scene's construction site
    /// has done this percentage of its work.
    pub(crate) until_built: Option<u32>,
    /// `--scene geologist | waterway | barracks | woodcutter-site | ship |
    /// ship-shore | building:<type> | military:<type>:<n>`: a set-up for
    /// screenshots.
    pub(crate) scene: Option<String>,
    /// `--3d`: start in the 3D view; `--zoom`: its zoom step.
    pub(crate) three_d: bool,
    pub(crate) zoom: Option<u8>,
    /// `--run <ticks>`: play this many ticks at load.
    pub(crate) run: u64,
    /// `--speed`: the game speed at start, 1 to 3.
    pub(crate) speed: u32,
    /// `--hour`: the environment clock's start hour (GFX-VIEW-2.3c).
    pub(crate) hour: Option<f32>,
    /// `--size WxH`: the window's size.
    pub(crate) size: Option<(u32, u32)>,
    /// `--tutorial-step <n>`: the tutorial step to start at (screenshots).
    pub(crate) tutorial_step: Option<u8>,
    /// `--end-screen victory|defeat`: the game shown as just ended
    /// (CAM-3.6: screenshots of the end and defeat screens).
    pub(crate) end_screen: Option<crate::menus::campaign::EndScreen>,
    /// `--play <script>`: the play harness's script (`play`).
    pub(crate) play: Option<String>,
    /// `--play-report <file>`: where the harness writes its report.
    pub(crate) play_report: Option<PathBuf>,
    /// `--colour-probe <report>`: check the colour pipeline and quit.
    pub(crate) colour_probe: Option<PathBuf>,
    /// `--log-builders`: log builders shown still while they work.
    pub(crate) log_builders: bool,
    /// The game opens with the intro film (UI-MENU-0.1).
    pub(crate) intro: bool,
}

pub(crate) fn parse_args(game: bool) -> Result<Options, String> {
    parse_from(game, std::env::args().skip(1))
}

pub(crate) fn parse_from(
    game: bool,
    args: impl IntoIterator<Item = String>,
) -> Result<Options, String> {
    let mut options = Options {
        version: false,
        map: None,
        save: None,
        save_dir: None,
        autosave: None,
        compare: false,
        still: false,
        no_fog: false,
        pause_layer: false,
        map_name: None,
        game_dir: std::env::var_os("S2_GAME_DIR").map(PathBuf::from),
        asset_folders: Vec::new(),
        rules_overlays: Vec::new(),
        screenshot_dir: PathBuf::from("screens"),
        shot: None,
        shot_after: SHOT_SETTLE_FRAMES,
        profile_dir: None,
        bench: None,
        bench_pan: false,
        bench_seconds: None,
        bench_log: None,
        bench_invariants: false,
        mute: false,
        bench_drag: None,
        bench_report: None,
        bench_vsync: false,
        camera: None,
        overlay: false,
        model: None,
        model_shots: None,
        model_list: None,
        scenario: None,
        ai: Vec::new(),
        start_resources: START_RESOURCES,
        focus: None,
        military_panel: false,
        attack: None,
        paused: false,
        compact: false,
        debug: false,
        menu: false,
        menu_screen: None,
        free_game: None,
        mission: None,
        size: None,
        tutorial_step: None,
        end_screen: None,
        open: None,
        open_tab: None,
        open_global: Vec::new(),
        open_building: false,
        building_type: None,
        context_at: None,
        watch: None,
        until_work: None,
        until_felled: None,
        until_door: None,
        until_built: None,
        scene: None,
        three_d: game,
        zoom: None,
        run: 0,
        speed: 1,
        hour: None,
        play: None,
        colour_probe: None,
        log_builders: false,
        intro: false,
        play_report: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--map" => options.map = Some(value()?.into()),
            "--map-name" => options.map_name = Some(value()?),
            "--save" => options.save = Some(value()?.into()),
            // docs/compare.md: the save as the original continues it
            // (GFX-VIEW-3.6: the camera made afresh at the HQ), the game
            // standing still, the 3D view without the interface.
            // The same still shot of a new game on a map (an .s2m file or
            // a map name): the golden images (docs/test-coverage.md).
            "--compare" => {
                let what = value()?;
                if what.to_ascii_lowercase().ends_with(".sav") {
                    options.save = Some(what.into());
                } else if what.to_ascii_lowercase().ends_with(".s2m") {
                    options.map = Some(what.into());
                } else {
                    options.map_name = Some(what);
                }
                options.compare = true;
                options.still = true;
                options.three_d = true;
                options.paused = true;
            }
            // The golden images: a picture that is the same in every run.
            "--still" => {
                options.still = true;
                options.paused = true;
            }
            "--no-fog" => options.no_fog = true,
            "--pause-layer" => options.pause_layer = true,
            "--game-dir" => options.game_dir = Some(value()?.into()),
            "--assets" => options.asset_folders.push(value()?.into()),
            "--rules-overlay" => options.rules_overlays.push(value()?.into()),
            "--screenshot-dir" => options.screenshot_dir = value()?.into(),
            "--save-dir" => options.save_dir = Some(value()?.into()),
            "--profile-dir" => options.profile_dir = Some(value()?.into()),
            "--autosave" => {
                options.autosave = Some(value()?.parse().map_err(|e| format!("--autosave: {e}"))?)
            }
            "--shot" => options.shot = Some(value()?.into()),
            "--shot-after" => {
                let frames = value()?;
                options.shot_after = frames
                    .parse()
                    .map_err(|_| format!("bad frame count {frames}"))?;
            }
            "--bench" => {
                let frames = value()?;
                options.bench = Some(
                    frames
                        .parse()
                        .map_err(|_| format!("bad frame count {frames}"))?,
                );
            }
            "--bench-report" => options.bench_report = Some(value()?.into()),
            "--bench-vsync" => options.bench_vsync = true,
            "--bench-pan" => options.bench_pan = true,
            "--bench-seconds" => {
                let seconds = value()?;
                options.bench_seconds = Some(
                    seconds
                        .parse()
                        .map_err(|_| format!("bad number of seconds {seconds}"))?,
                );
                options.bench.get_or_insert(u32::MAX);
            }
            "--bench-log" => options.bench_log = Some(value()?.into()),
            "--bench-invariants" => options.bench_invariants = true,
            "--mute" => options.mute = true,
            "--bench-drag" => {
                let pixels = value()?;
                options.bench_drag = Some(
                    pixels
                        .parse()
                        .map_err(|_| format!("bad pixel count {pixels}"))?,
                );
            }
            "--camera" => options.camera = Some(value()?.into()),
            "--overlay" => options.overlay = true,
            "--model" => options.model = Some(value()?),
            "--model-shots" => options.model_shots = Some(value()?.into()),
            "--model-list" => options.model_list = Some(value()?.into()),
            "--expand" => {
                let ticks = value()?;
                options.scenario = Some(
                    ticks
                        .parse()
                        .map_err(|_| format!("bad tick count {ticks}"))?,
                );
            }
            "--ai" => {
                let text = value()?;
                let bad = || format!("bad --ai {text}, want <slot>=<profile> with profile 1 to 3");
                let (slot, profile) = text.split_once('=').ok_or_else(bad)?;
                let slot: u8 = slot.parse().map_err(|_| bad())?;
                let profile: u8 = profile.parse().map_err(|_| bad())?;
                if slot >= 8 || !(1..=3).contains(&profile) {
                    return Err(bad());
                }
                options.ai.push((PlayerId(slot), profile));
            }
            "--focus" => {
                let text = value()?;
                let bad = || format!("bad focus {text}, want x,y,scale");
                let parts: Vec<&str> = text.split(',').collect();
                let [x, y, scale] = parts[..] else {
                    return Err(bad());
                };
                options.focus = Some((
                    x.parse().map_err(|_| bad())?,
                    y.parse().map_err(|_| bad())?,
                    scale.parse().map_err(|_| bad())?,
                ));
            }
            "--military-panel" => options.military_panel = true,
            "--paused" => options.paused = true,
            "--compact" => options.compact = true,
            "--debug" => options.debug = true,
            "--menu" => options.menu = true,
            "--free-game" => {
                options.menu = true;
                options.free_game = Some(value()?);
            }
            "--mission" => {
                options.menu = true;
                options.mission = Some(value()?);
            }
            "--menu-screen" => {
                options.menu = true;
                options.menu_screen = Some(value()?);
            }
            "--open" => {
                let text = value()?;
                let (x, y) = text
                    .split_once(',')
                    .and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)))
                    .ok_or_else(|| format!("bad field {text}, want x,y"))?;
                options.open = Some((x, y));
            }
            "--open-tab" => {
                options.open_tab = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--open-tab takes a tab number".to_owned())?,
                )
            }
            "--open-global" => options.open_global.push(value()?),
            "--open-building" => options.open_building = true,
            "--building-type" => options.building_type = Some(value()?),
            "--context-at" => {
                let text = value()?;
                let point = text
                    .split_once(',')
                    .and_then(|(x, y)| Some([x.parse().ok()?, y.parse().ok()?]));
                options.context_at =
                    Some(point.ok_or_else(|| format!("bad point {text}, want x,y"))?);
            }
            "--watch" => options.watch = Some(value()?),
            "--until-work" => options.until_work = Some(value()?),
            "--until-door" => options.until_door = Some(value()?),
            "--until-felled" => {
                options.until_felled = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--until-felled takes a tick count".to_owned())?,
                )
            }
            "--until-built" => {
                options.until_built = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--until-built takes a percentage".to_owned())?,
                )
            }
            "--scene" => options.scene = Some(value()?),
            "--3d" => options.three_d = true,
            "--speed" => {
                let speed = value()?;
                options.speed = speed
                    .parse()
                    .ok()
                    .filter(|s| (1..=3).contains(s))
                    .ok_or_else(|| format!("bad speed {speed}, want 1 to 3"))?;
            }
            "--hour" => {
                let hour = value()?;
                options.hour = Some(
                    hour.parse()
                        .ok()
                        .filter(|h: &f32| h.is_finite())
                        .ok_or_else(|| format!("bad hour {hour}"))?,
                );
            }
            "--run" => {
                let ticks = value()?;
                options.run = ticks
                    .parse()
                    .map_err(|_| format!("bad tick count {ticks}"))?;
            }
            "--end-screen" => {
                options.end_screen = match value()?.as_str() {
                    "victory" => Some(crate::menus::campaign::EndScreen::Victory(1)),
                    "defeat" => Some(crate::menus::campaign::EndScreen::Defeat),
                    other => return Err(format!("bad end screen {other}, want victory or defeat")),
                };
            }
            "--tutorial-step" => {
                let text = value()?;
                let step = match text.strip_prefix("0x") {
                    Some(hex) => u8::from_str_radix(hex, 16).ok(),
                    None => text.parse().ok(),
                };
                match step {
                    Some(n) if usize::from(n) < crate::menus::campaign::STEPS.len() => {
                        options.tutorial_step = Some(n)
                    }
                    _ => return Err(format!("bad tutorial step {text}, want 0 to 0x50")),
                }
            }
            "--play" => {
                let name = value()?;
                if crate::tools::play::script(&name).is_none() {
                    return Err(format!("no play script named {name}"));
                }
                options.play = Some(name);
            }
            "--play-report" => options.play_report = Some(value()?.into()),
            "--colour-probe" => options.colour_probe = Some(value()?.into()),
            "--log-builders" => options.log_builders = true,
            "--size" => {
                let text = value()?;
                let size = text
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
                match size {
                    Some(s) => options.size = Some(s),
                    None => return Err(format!("bad size {text}, want WxH")),
                }
            }
            "--zoom" => {
                let zoom = value()?;
                match zoom.parse() {
                    Ok(z) if z <= crate::render::world3d::MAX_ZOOM => options.zoom = Some(z),
                    _ => return Err(format!("bad zoom {zoom}, want 0 to 4")),
                }
            }
            "--attack" => {
                options.attack = Some(match value()?.as_str() {
                    "fight" => s2_rules::scenario::AttackStop::Fight,
                    "conquest" => s2_rules::scenario::AttackStop::Conquest,
                    other => return Err(format!("--attack wants fight or conquest, not {other}")),
                });
            }
            "--start-resources" => {
                let setting = value()?;
                options.start_resources = setting
                    .parse()
                    .map_err(|_| format!("bad start resources {setting}"))?;
            }
            "--version" | "-V" => options.version = true,
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// docs/compare.md: `--compare` continues the save paused in the 3D
    /// view with the interface hidden; zoom, size and shot as given.
    // spec: none (the look comparison's options, docs/compare.md)
    #[test]
    fn compare_options() {
        let args = [
            "--compare",
            "a.sav",
            "--zoom",
            "2",
            "--size",
            "1024x768",
            "--shot",
            "out.png",
        ]
        .map(String::from);
        let o = parse_from(false, args).expect("parses");
        assert_eq!(o.save.as_deref(), Some(Path::new("a.sav")));
        assert!(o.compare && o.three_d && o.paused);
        assert_eq!(o.zoom, Some(2));
        assert_eq!(o.size, Some((1024, 768)));
        assert_eq!(o.shot.as_deref(), Some(Path::new("out.png")));
        let plain = parse_from(false, ["--save".to_string(), "a.sav".to_string()]).unwrap();
        assert!(!plain.compare && !plain.paused);
        // A map instead of a save: a new game on it, still.
        let map = parse_from(false, ["--compare", "MP_2P_Storm_Coast"].map(String::from)).unwrap();
        assert_eq!(map.map_name.as_deref(), Some("MP_2P_Storm_Coast"));
        assert!(map.compare && map.save.is_none());
        let file = parse_from(false, ["--compare", "x/m.S2M"].map(String::from)).unwrap();
        assert_eq!(file.map.as_deref(), Some(Path::new("x/m.S2M")));
        // A picture of a game the original played with fog of war off
        // (Q-290): every field drawn as explored.
        let open = parse_from(false, ["--compare", "m", "--no-fog"].map(String::from)).unwrap();
        assert!(open.no_fog && !map.no_fog);
        // A picture the original took paused: the paused view's layer.
        let paused =
            parse_from(false, ["--compare", "m", "--pause-layer"].map(String::from)).unwrap();
        assert!(paused.pause_layer && !map.pause_layer);
    }

    /// `--bench-vsync`: the benchmark's frames held to the screen's
    /// refresh, as a player sees the game (F51).
    // spec: none (the frame-time benchmark's options, docs/test-coverage.md)
    #[test]
    fn bench_options() {
        let args = ["--bench", "3600", "--bench-vsync"].map(String::from);
        let o = parse_from(false, args).expect("parses");
        assert_eq!(o.bench, Some(3600));
        assert!(o.bench_vsync);
        let plain = parse_from(false, ["--bench", "600"].map(String::from)).unwrap();
        assert!(!plain.bench_vsync);
    }

    /// `--context-at x,y`: the screen point `--open-building`'s window is
    /// opened at, as if clicked there (UI-GAME-1.9: its bottom-right
    /// corner on that point), so a picture can show it beside its building
    /// and beside a global window. Without it, the screen's centre.
    // spec: UI-GAME-1.9 (the screenshot tool's click point)
    #[test]
    fn context_at_option() {
        let args = ["--open-building", "--context-at", "1180,900"].map(String::from);
        let o = parse_from(false, args).expect("parses");
        assert!(o.open_building);
        assert_eq!(o.context_at, Some([1180.0, 900.0]));
        let plain = parse_from(false, ["--open-building".to_string()]).unwrap();
        assert_eq!(plain.context_at, None);
        assert!(parse_from(false, ["--context-at", "12"].map(String::from)).is_err());
    }

    /// `--version` (or `-V`): the program prints its version line and
    /// quits; not given, it runs as ever.
    // spec: none (the program's version, docs/releasing.md)
    #[test]
    fn version_option() {
        assert!(parse_from(true, ["--version".to_string()]).unwrap().version);
        assert!(parse_from(false, ["-V".to_string()]).unwrap().version);
        assert!(!parse_from(true, Vec::<String>::new()).unwrap().version);
    }

    /// `--open-tab n`: the tab of `--open`'s building window, so a picture
    /// can show the headquarters' military page (UI-GAME-4.4).
    // spec: UI-GAME-4.4 (the screenshot tool's tab)
    #[test]
    fn open_tab_option() {
        let args = ["--open", "29,51", "--open-tab", "11"].map(String::from);
        let o = parse_from(false, args).expect("parses");
        assert_eq!((o.open, o.open_tab), (Some((29, 51)), Some(11)));
        assert!(parse_from(false, ["--open-tab", "x"].map(String::from)).is_err());
    }
}
