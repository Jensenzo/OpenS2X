//! The game with a project's extensions (docs/extending.md): a rules
//! overlay, an asset folder, sharper shadows, a system of its own, a main
//! menu entry with its page, lines of its own on the start screen and the
//! main menu, no intro film, the music's switches reported (the
//! original's music still plays), a camera that drifts while its switch
//! is on, its own window title, the player's profile file left
//! unwritten (changed settings are logged and kept in a copy of its own
//! in `Documents\S2\extended`), the thing under the pointer
//! logged at a Ctrl-click, F12's screenshots in `screens/extended`, and
//! its saves in `Documents\S2\extended\saves`, marked as its own.
//!
//!     cargo run -p s2-viewer --example extended -- --menu
//!
//! The asset folder is `S2_NEXT_ASSETS` when set (laid out like the
//! install: `data/...`). With `EXTENDED_AO` set the map has screen-space
//! ambient occlusion (hook W17b).

use s2_viewer::bevy::prelude::*;
use s2_viewer::extend::{
    CameraFollow, CurrentSettings, MapThings, MenuEntry, MusicSlot, PageWidget,
};
use s2_viewer::{Captions, EffectRule, Extensions, ProjectEvent, RenderOptions, SettingsOptions};

/// The woodcutter's hut costs one more stone: an overlay sees the
/// originals' globals and calls the same `properties` functions (LUA-PROP).
const DEARER_WOODCUTTERS: &str = "properties.bp_addCost(woodCutter, stone, 1)";

/// The project's own switches, shown on its page.
#[derive(Resource, Default)]
struct Switches {
    follow_camera: bool,
    music: usize,
}

/// The page of the main menu's EXTENDED entry: read from the app's world
/// every frame it is shown; the callbacks change the world.
fn page(world: &World) -> Vec<PageWidget> {
    let switches = world.resource::<Switches>();
    vec![
        PageWidget::Caption("Extended".into()),
        PageWidget::Toggle {
            label: "Follow camera".into(),
            value: switches.follow_camera,
            on_change: Box::new(|world, on| world.resource_mut::<Switches>().follow_camera = on),
        },
        PageWidget::Choice {
            label: "Music".into(),
            options: vec!["Original".into(), "Own".into(), "Shuffle".into()],
            index: switches.music,
            on_change: Box::new(|world, n| world.resource_mut::<Switches>().music = n),
        },
        PageWidget::Button {
            label: "Defaults".into(),
            on_click: Box::new(|world| *world.resource_mut::<Switches>() = Switches::default()),
        },
    ]
}

/// The music hook: what the viewer's player switched to, and the volume a
/// project's own player would play it at.
fn music(mut slots: MessageReader<MusicSlot>, settings: Res<CurrentSettings>) {
    for slot in slots.read() {
        info!(
            "extended: music {slot:?} at volume {}",
            settings.get().music_volume()
        );
    }
}

/// The camera hook: while the page's "Follow camera" is on, the camera
/// follows a point drifting east one unit a second; the player's own
/// camera input takes it back and the switch goes off.
fn drift(time: Res<Time>, mut switches: ResMut<Switches>, mut follow: ResMut<CameraFollow>) {
    if follow.cleared {
        switches.follow_camera = false;
    }
    if !switches.follow_camera {
        if follow.target.is_some() {
            follow.target = None;
        }
        return;
    }
    let at = follow.target.unwrap_or(Vec3::new(64.0, 0.0, -64.0));
    follow.target = Some(at + Vec3::X * time.delta_secs());
}

/// The map things hook: a Ctrl-click logs the nearest thing under the
/// pointer (a settler with the ware he carries, a ware, an animal) and
/// where it is drawn.
fn pointed(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    things: Res<MapThings>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !(ctrl && buttons.just_pressed(MouseButton::Left) && things.over_map) {
        return;
    }
    if let Some(hit) = things.under_pointer.first() {
        let at = things.position(hit.thing);
        info!(
            "extended: {:?} (carrying {:?}) {:.1} away, at {at:?}",
            hit.thing, hit.carrying, hit.distance
        );
    }
}

fn main() -> std::process::ExitCode {
    let extensions = Extensions {
        rules_overlays: vec![s2_rules::Overlay {
            name: "woodcutter-costs-one-more-stone".into(),
            code: DEARER_WOODCUTTERS.as_bytes().to_vec(),
        }],
        asset_folders: std::env::var_os("S2_NEXT_ASSETS")
            .map(Into::into)
            .into_iter()
            .collect(),
        render: RenderOptions {
            shadow_map_size: Some(4096),
            ambient_occlusion: std::env::var_os("EXTENDED_AO").is_some(),
            ..RenderOptions::default()
        },
        plugins: vec![Box::new(|app: &mut App| {
            app.init_resource::<Switches>()
                .add_systems(Startup, || {
                    // The player's folders and the game's, as the game
                    // finds them.
                    let profiles = s2_viewer::paths::users_folder();
                    let game = s2_viewer::paths::game_dir();
                    info!("extended: started, profiles in {profiles:?}, the game in {game:?}");
                })
                .add_systems(Update, (music, drift, pointed));
        })],
        menu: vec![MenuEntry {
            label: "EXTENDED".into(),
            page: Box::new(page),
        }],
        captions: Captions {
            start_screen_title: Some("EXTENDED - BUILT ON OPENS2X".into()),
            main_menu_footer: Some(format!(
                "extended example, OpenS2X {}",
                env!("CARGO_PKG_VERSION")
            )),
        },
        skip_launch_films: true,
        // true: the project's own player plays on `MusicSlot`.
        music_external: false,
        window_title: Some("OpenS2X, extended".into()),
        settings: SettingsOptions {
            profile_write: false,
            app_settings_dir: None,
            // The settings pages' changes kept from start to start.
            profile_dir: s2_viewer::paths::documents().map(|d| d.join("S2").join("extended")),
        },
        on_settings_change: Some(Box::new(|settings| {
            info!(
                "extended: settings changed, music volume {}",
                settings.music_volume()
            );
        })),
        map_things: true,
        // Dust over a site while its builder hammers (the original shows
        // none there, GFX-OBJ-8.3b; the leveller's StoneDust taken).
        effects: vec![EffectRule {
            effect: "data/scripts/effects/StoneDust.pfx".into(),
            profession: "good_constructor".into(),
            work_loop: Some(0),
        }],
        screenshot_dir: Some("screens/extended".into()),
        // Its games apart from OpenS2X's, marked with its name and the
        // overlay above: OpenS2X does not load them, nor this program
        // OpenS2X's.
        save_dir: s2_viewer::paths::documents()
            .map(|d| d.join("S2").join("extended").join("saves")),
        project_name: Some("extended".into()),
        // Hook W24: a message when a project's rule finishes (this example
        // has no deepening or paving overlay, so none comes).
        messages: vec![(ProjectEvent::RoadPaved, "Road paved".into())],
        // The hooks' buttons keep the boat-and-ship switch's picture.
        button_pictures: Default::default(),
    };
    s2_viewer::run_with(true, extensions)
}
