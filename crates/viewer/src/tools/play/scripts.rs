//! The play scripts: the steps of `first-ten-minutes` and `select-map`,
//! each naming the spec rules it checks.

use super::*;

pub(crate) fn field(
    name: &'static str,
    f: impl Fn(&PlayView, &Named) -> Result<Point, String> + Send + Sync + 'static,
) -> Act {
    Act::Field(name, Box::new(f))
}

pub(crate) fn button(
    name: &'static str,
    f: impl Fn(&PlayView, &Named, &Action, Option<&str>) -> bool + Send + Sync + 'static,
) -> Act {
    Act::Button(name, Box::new(f))
}

pub(crate) fn named(named: &Named, name: &str) -> Result<Point, String> {
    named
        .get(name)
        .copied()
        .ok_or_else(|| format!("no field {name} was chosen"))
}

/// GFX-VIEW-3.1: the 3D camera is at zoom step `want`.
pub(crate) fn zoom_is(v: &PlayView, want: u8) -> Result<(), String> {
    let zoom = v.view3d.rig.zoom;
    if zoom == want {
        Ok(())
    } else {
        Err(format!("the zoom step is {zoom}, not {want}"))
    }
}

pub(crate) fn keys(code: KeyCode, key: Key, n: usize) -> Vec<Act> {
    std::iter::repeat_with(|| Act::Key(code, key.clone()))
        .take(n)
        .collect()
}

pub(crate) fn hq_flag(v: &PlayView, _: &Named) -> Result<Point, String> {
    Ok(v.flag_of(v.headquarters()?))
}

/// A placed flag is one of the player's.
pub(crate) fn flag_placed(v: &PlayView, at: Point) -> Result<(), String> {
    if v.world.flag_at(at) == Some(v.player) {
        Ok(())
    } else {
        Err(format!("no flag at ({}, {})", at.0, at.1))
    }
}

/// The context panel offers an enabled button that places a flag on `at`.
pub(crate) fn flag_button_offered(v: &PlayView, at: Point) -> Result<(), String> {
    let found = v
        .button(&|a, _| matches!(a, Action::Command(Command::PlaceFlag { at: f, .. }) if *f == at));
    match found {
        Some(_) => Ok(()),
        None => Err(format!(
            "the dialog offers no flag button for ({}, {}); context {:?}",
            at.0, at.1, v.ui.context
        )),
    }
}

/// UI-GAME-1.10a (F41): the system's pointer is on the screen point of
/// `at` (within 3 px), as a map menu leaves it when it closes.
pub(crate) fn pointer_on(v: &PlayView, at: Point, what: &str) -> Result<(), String> {
    let p = v
        .screen(at)
        .ok_or_else(|| format!("{what} is not on screen"))?;
    match v.cursor {
        Some(c) if (c[0] - p[0]).abs() <= 3.0 && (c[1] - p[1]).abs() <= 3.0 => Ok(()),
        c => Err(format!("the pointer is at {c:?}, {what} at {p:?}")),
    }
}

/// The "first ten minutes" of a free game: what a player does first, each
/// step checked against the game state.
pub fn first_ten_minutes() -> Vec<Step> {
    let place_flag_at = |name: &'static str| {
        button("the flag button", move |_, n, a, _| {
            named(n, name).is_ok_and(
                |at| matches!(a, Action::Command(Command::PlaceFlag { at: f, .. }) if *f == at),
            )
        })
    };
    vec![
        step(
            "the game is running, in its frame (UI-GAME-11)",
            vec![],
            1,
            Box::new(|v, _| {
                v.headquarters()?;
                if !v.panels.iter().any(|p| p.id == PanelId::Frame) {
                    return Err("the screen frame is not drawn".into());
                }
                Ok(())
            }),
        ),
        // GFX-VIEW-3.1 / 3.2: five zoom steps 0 .. 4, the game starts at 3
        // (observed in the original too, specs/OBSERVED.md).
        step(
            "the game starts at zoom step 3 of 0 to 4 (GFX-VIEW-3.1)",
            vec![],
            1,
            Box::new(|v, _| zoom_is(v, 3)),
        ),
        step(
            "the wheel back zooms out one step, to 4 (GFX-VIEW-3.2)",
            vec![Act::Wheel(-1.0)],
            5,
            Box::new(|v, _| zoom_is(v, 4)),
        ),
        step(
            "PAGE UP at the farthest step stays there (GFX-VIEW-3.2, UI-GAME-12.1)",
            vec![Act::Key(KeyCode::PageUp, Key::PageUp)],
            5,
            Box::new(|v, _| zoom_is(v, 4)),
        ),
        step(
            "PAGE DOWN zooms in step by step to the closest, 0 (GFX-VIEW-3.2, UI-GAME-12.1)",
            keys(KeyCode::PageDown, Key::PageDown, 5),
            5,
            Box::new(|v, _| zoom_is(v, 0)),
        ),
        step(
            "the wheel forward at the closest step stays there (GFX-VIEW-3.2)",
            vec![Act::Wheel(1.0)],
            5,
            Box::new(|v, _| zoom_is(v, 0)),
        ),
        step(
            "PAGE UP zooms out again to the start step (GFX-VIEW-3.2, UI-GAME-12.1)",
            keys(KeyCode::PageUp, Key::PageUp, 3),
            5,
            Box::new(|v, _| zoom_is(v, 3)),
        ),
        step(
            "the construction view is switched on (UI-GAME-9.1)",
            vec![
                button("the HUD's construction view button", |_, _, a, _| {
                    *a == Action::Hud(3)
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, _| {
                if v.overlay {
                    Ok(())
                } else {
                    Err("the construction view is off".into())
                }
            }),
        ),
        step(
            "SPACE switches the construction view off (UI-GAME-12.1)",
            vec![Act::Key(KeyCode::Space, Key::Space)],
            10,
            Box::new(|v, _| {
                if v.overlay {
                    Err("the construction view is still on".into())
                } else {
                    Ok(())
                }
            }),
        ),
        step(
            "SPACE switches it on again (UI-GAME-9.1, UI-GAME-12.1)",
            vec![Act::Key(KeyCode::Space, Key::Space)],
            10,
            Box::new(|v, _| {
                if v.overlay {
                    Ok(())
                } else {
                    Err("the construction view is off".into())
                }
            }),
        ),
        // GFX-VIEW-3.3: the game starts with the headquarters at the
        // screen's centre, its flag low, near the bar at 1024 x 768; the
        // player scrolls down a little first.
        step(
            "the down arrow scrolls the view: the headquarters' flag is clear of the bar (UI-GAME-12.1)",
            keys(KeyCode::ArrowDown, Key::ArrowDown, 10),
            10,
            Box::new(|v, n| {
                let flag = hq_flag(v, n)?;
                let p = v
                    .screen(flag)
                    .ok_or_else(|| format!("the headquarters' flag {flag:?} is not on screen"))?;
                if p[1] < v.window[1] * 0.75 {
                    Ok(())
                } else {
                    Err(format!(
                        "the headquarters' flag is at {p:?}, low on the screen"
                    ))
                }
            }),
        ),
        step(
            "a click on a building spot near the headquarters opens the build menu (UI-GAME-3.2)",
            vec![
                field("woodcutter", |v, _| {
                    let home = hq_flag(v, &Named::new())?;
                    v.own_fields_on_screen(home, 6)
                        .into_iter()
                        .find(|&f| {
                            grid::distance(home, f) >= 4
                                && matches!(
                                    v.world.placement_at(f),
                                    Placement::Small | Placement::Medium | Placement::Large
                                )
                                && v.screen(v.flag_of(f)).is_some()
                        })
                        .ok_or_else(|| {
                            "no building spot 4 to 6 fields from the headquarters' flag on screen"
                                .into()
                        })
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "woodcutter")?;
                match &v.ui.context {
                    Some(Context::Build { field, .. }) if *field == at => Ok(()),
                    other => Err(format!("the build menu is not open: {other:?}")),
                }
            }),
        ),
        step(
            "the woodcutter is placed: its construction site exists (UI-GAME-3.2, ECO-HQ-4.1)",
            vec![
                button("the woodcutter's button", |v, n, a, _| {
                    let Ok(id) = v.building_id("WoodcutterHut") else {
                        return false;
                    };
                    named(n, "woodcutter").is_ok_and(|at| {
                        matches!(a, Action::Command(Command::PlaceBuilding { building, at: f, .. }) if *building == id && *f == at)
                    })
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "woodcutter")?;
                let site = v.world.site(at).is_some();
                let building = v
                    .world
                    .building_at(at)
                    .is_some_and(|b| b.owner == v.player && b.under_construction);
                if !(site && building) {
                    return Err(format!("no construction site at ({}, {})", at.0, at.1));
                }
                // GFX-OBJ-1.1: the site's frame model stands at once, before
                // any builder arrives.
                if !v.placements.has_building(at) {
                    return Err(format!("the site at ({}, {}) is not drawn", at.0, at.1));
                }
                Ok(())
            }),
        ),
        // UI-GAME-1.10a: the build menu closed by a type, the pointer on
        // its field.
        step(
            "after the woodcutter's button the pointer is on its field (UI-GAME-1.10a)",
            vec![],
            10,
            Box::new(|v, n| pointer_on(v, named(n, "woodcutter")?, "the woodcutter's field")),
        ),
        step(
            "a click on the headquarters' flag opens the flag menu (UI-GAME-3.1)",
            vec![field("hq_flag", hq_flag), Act::Click],
            10,
            Box::new(|v, n| {
                let at = named(n, "hq_flag")?;
                match &v.ui.context {
                    Some(Context::Flag(f)) if *f == at => Ok(()),
                    other => Err(format!("the flag menu is not open: {other:?}")),
                }
            }),
        ),
        step(
            "\"Build road\" starts road building from the flag (UI-GAME-3.4, TRN-ROAD-8.1)",
            vec![
                button("the build road button", |_, _, a, _| {
                    matches!(a, Action::StartRoad(_))
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "hq_flag")?;
                if v.ui.road_from == Some(at) {
                    Ok(())
                } else {
                    Err(format!(
                        "road building did not start: road_from {:?}",
                        v.ui.road_from
                    ))
                }
            }),
        ),
        step(
            "a click on the woodcutter's flag builds the road (UI-GAME-3.4, TRN-ROAD-8.3)",
            vec![
                field("woodcutter_flag", |v, n| {
                    Ok(v.flag_of(named(n, "woodcutter")?))
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let (a, b) = (named(n, "hq_flag")?, named(n, "woodcutter_flag")?);
                let joined = v.world.roads().any(|r| {
                    r.owner == v.player
                        && (r.path.first() == Some(&a) && r.path.last() == Some(&b)
                            || r.path.first() == Some(&b) && r.path.last() == Some(&a))
                });
                if joined {
                    Ok(())
                } else {
                    Err(format!(
                        "no road between ({}, {}) and ({}, {}); {} roads",
                        a.0,
                        a.1,
                        b.0,
                        b.1,
                        v.world.roads().count()
                    ))
                }
            }),
        ),
        step(
            "a click on the road opens the road menu (UI-GAME-3.3)",
            vec![
                field("road_field", |v, n| {
                    let (a, b) = (named(n, "hq_flag")?, named(n, "woodcutter_flag")?);
                    let road = v
                        .world
                        .roads()
                        .find(|r| {
                            r.path.first() == Some(&a) && r.path.last() == Some(&b)
                                || r.path.first() == Some(&b) && r.path.last() == Some(&a)
                        })
                        .ok_or("the road is gone")?;
                    let inner = road
                        .path
                        .get(2..road.path.len().saturating_sub(2))
                        .unwrap_or(&[]);
                    inner
                        .iter()
                        .copied()
                        .find(|&f| {
                            v.world.placement_at(f) != Placement::Nothing && v.screen(f).is_some()
                        })
                        .ok_or_else(|| {
                            format!(
                                "no field of the road takes a flag (road of {} fields)",
                                road.path.len()
                            )
                        })
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "road_field")?;
                match &v.ui.context {
                    Some(Context::Road(f)) if *f == at => Ok(()),
                    other => Err(format!("the road menu is not open: {other:?}")),
                }
            }),
        ),
        step(
            "\"Deploy flag\" puts a flag on the road, which splits it (UI-GAME-3.3, TRN-ROAD-4.2)",
            vec![place_flag_at("road_field"), Act::Click],
            10,
            Box::new(|v, n| {
                let at = named(n, "road_field")?;
                flag_placed(v, at)?;
                let ends = v
                    .world
                    .roads()
                    .filter(|r| r.path.first() == Some(&at) || r.path.last() == Some(&at))
                    .count();
                if ends == 2 {
                    Ok(())
                } else {
                    Err(format!("the flag ends {ends} roads, not 2"))
                }
            }),
        ),
        // UI-GAME-1.10b: the road menu warps onto its field when it closes.
        step(
            "after \"Deploy flag\" the pointer is on the new flag (UI-GAME-1.10b)",
            vec![],
            10,
            Box::new(|v, n| pointer_on(v, named(n, "road_field")?, "the new flag")),
        ),
        step(
            "a road to a field next to a flag is refused, and the reason is shown (UI-GAME-1.4)",
            vec![
                field("hq_flag", hq_flag),
                Act::Click,
                button("the build road button", |_, _, a, _| {
                    matches!(a, Action::StartRoad(_))
                }),
                Act::Click,
                field("bad_target", |v, n| {
                    let home = named(n, "hq_flag")?;
                    v.world
                        .terrain()
                        .neighbours(home)
                        .into_iter()
                        .flatten()
                        .find(|&f| {
                            v.world.field_owner(f) == Some(v.player)
                                && v.world.flag_at(f).is_none()
                                && v.world.building_at(f).is_none()
                                && v.screen(f).is_some()
                        })
                        .ok_or_else(|| {
                            "no free field next to the headquarters' flag on screen".into()
                        })
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "bad_target")?;
                if v.world.flag_at(at) == Some(v.player) {
                    return Err(format!(
                        "a flag was placed next to a flag at ({}, {})",
                        at.0, at.1
                    ));
                }
                // UI-GAME-3.4, TRN-ROAD-7.4: the click extends nothing and
                // road building goes on.
                if v.ui.road_from.is_none() {
                    return Err("road building ended".into());
                }
                let points = v.world.route(v.player).map(|r| r.fields.len());
                if points != Some(1) {
                    return Err(format!("the route has {points:?} fields, not 1"));
                }
                match v.refusal() {
                    Some(_) => Ok(()),
                    None => Err("no refusal is shown".into()),
                }
            }),
        ),
        step(
            "a click on the road's start flag cancels road building (UI-GAME-3.4)",
            vec![field("hq_flag", hq_flag), Act::Click],
            10,
            Box::new(|v, _| {
                if v.ui.road_from.is_some() || v.world.route(v.player).is_some() {
                    Err("road building is still on".into())
                } else {
                    Ok(())
                }
            }),
        ),
        step(
            "a click on a free field extends the road being built there (UI-GAME-3.4)",
            vec![
                field("hq_flag", hq_flag),
                Act::Click,
                button("the build road button", |_, _, a, _| {
                    matches!(a, Action::StartRoad(_))
                }),
                Act::Click,
                field("road_end", |v, n| {
                    let home = named(n, "hq_flag")?;
                    v.own_fields_on_screen(home, 5)
                        .into_iter()
                        .find(|&f| {
                            grid::distance(home, f) >= 3
                                && v.world.check_flag(v.player, f).is_ok()
                                && v.world.find_route(v.player, home, f).is_ok()
                        })
                        .ok_or_else(|| "no free field 3 to 5 fields away on screen".into())
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "road_end")?;
                match v.world.route(v.player) {
                    Some(r) if r.end() == at && r.fields.len() > 1 => Ok(()),
                    other => Err(format!(
                        "the route does not end at ({}, {}): {:?}",
                        at.0,
                        at.1,
                        other.map(|r| r.fields.clone())
                    )),
                }
            }),
        ),
        step(
            "a click on the road's end opens the deploy-flag menu (UI-GAME-3.6)",
            vec![field("road_end", |_, n| named(n, "road_end")), Act::Click],
            10,
            Box::new(|v, n| {
                let at = named(n, "road_end")?;
                match &v.ui.context {
                    Some(Context::DeployFlag(f)) if *f == at => flag_button_offered(v, at),
                    other => Err(format!("not the deploy-flag menu: {other:?}")),
                }
            }),
        ),
        step(
            "\"Create Flag\" ends the road there (UI-GAME-8.6)",
            vec![place_flag_at("road_end"), Act::Click],
            10,
            Box::new(|v, n| {
                let (home, at) = (named(n, "hq_flag")?, named(n, "road_end")?);
                let built = v
                    .world
                    .roads()
                    .any(|r| r.path.first() == Some(&home) && r.path.last() == Some(&at));
                if !built {
                    return Err("no road from the headquarters' flag to the new flag".into());
                }
                if v.ui.road_from.is_some() {
                    return Err("road building is still on".into());
                }
                Ok(())
            }),
        ),
        // UI-GAME-1.10a (play test 11): the deploy-flag menu closed, the
        // pointer is on the new flag.
        step(
            "after \"Create Flag\" the pointer is on the new flag (UI-GAME-1.10a)",
            vec![],
            10,
            Box::new(|v, n| pointer_on(v, named(n, "road_end")?, "the new flag")),
        ),
        step(
            "a click on a flag-only spot offers a flag (UI-GAME-3.2, ECO-PLACE-2.2)",
            vec![
                field("flag_spot", |v, n| {
                    let home = named(n, "hq_flag")?;
                    v.own_fields_on_screen(home, 9)
                        .into_iter()
                        .find(|&f| {
                            v.world.placement_at(f) == Placement::Flag
                                && v.world.road_at(f).is_none()
                        })
                        .ok_or_else(|| "no flag-only spot on screen".into())
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "flag_spot")?;
                // UI-GAME-3.3 T23: the road menu with the one flag button.
                match &v.ui.context {
                    Some(Context::Road(f)) if *f == at => {}
                    other => return Err(format!("not the road menu: {other:?}")),
                }
                flag_button_offered(v, at)?;
                if v.button(&|a, _| matches!(a, Action::Command(Command::RemoveRoad { .. })))
                    .is_some()
                {
                    return Err("the flag-only spot's menu offers to destroy a road".into());
                }
                Ok(())
            }),
        ),
        step(
            "the flag is placed on the flag-only spot (ECO-PLACE-2.2)",
            vec![place_flag_at("flag_spot"), Act::Click],
            10,
            Box::new(|v, n| flag_placed(v, named(n, "flag_spot")?)),
        ),
        step(
            "after the flag-only spot's flag the pointer is on it (UI-GAME-1.10b)",
            vec![],
            10,
            Box::new(|v, n| pointer_on(v, named(n, "flag_spot")?, "the new flag")),
        ),
        step(
            "a click on a building spot offers a flag too (UI-GAME-3.2)",
            vec![
                field("house_spot", |v, n| {
                    let home = named(n, "hq_flag")?;
                    v.own_fields_on_screen(home, 9)
                        .into_iter()
                        .find(|&f| {
                            matches!(
                                v.world.placement_at(f),
                                Placement::Small | Placement::Medium | Placement::Large
                            ) && v.world.check_flag(v.player, f).is_ok()
                        })
                        .ok_or_else(|| "no building spot with room for a flag on screen".into())
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| flag_button_offered(v, named(n, "house_spot")?)),
        ),
        step(
            "the flag is placed on the building spot (ECO-PLACE-2.2)",
            vec![place_flag_at("house_spot"), Act::Click],
            10,
            Box::new(|v, n| flag_placed(v, named(n, "house_spot")?)),
        ),
        // F41: the build menu's flag button left the pointer on the button.
        step(
            "after the build menu's flag button the pointer is on the new flag (UI-GAME-1.10b)",
            vec![],
            10,
            Box::new(|v, n| pointer_on(v, named(n, "house_spot")?, "the new flag")),
        ),
        step(
            "a building spot where no flag fits offers no flag (UI-GAME-3.2 T5)",
            vec![
                field("no_flag_spot", |v, n| {
                    let home = named(n, "hq_flag")?;
                    v.own_fields_on_screen(home, 9)
                        .into_iter()
                        .find(|&f| {
                            // A building spot (a flag-only spot opens the
                            // road menu instead, UI-GAME-3.3).
                            matches!(
                                v.world.placement_at(f),
                                Placement::Small | Placement::Medium | Placement::Large
                            ) && v.world.road_at(f).is_none()
                                && v.world.check_flag(v.player, f).is_err()
                        })
                        .ok_or_else(|| "no building spot without room for a flag on screen".into())
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, n| {
                let at = named(n, "no_flag_spot")?;
                match &v.ui.context {
                    Some(Context::Build { field, .. }) if *field == at => {}
                    other => {
                        return Err(format!(
                            "the build menu is not open: {other:?}; {at:?} now takes {:?}",
                            v.world.placement_at(at)
                        ));
                    }
                }
                match flag_button_offered(v, at) {
                    Ok(()) => Err("the flag button is enabled although no flag fits".into()),
                    Err(_) => Ok(()),
                }
            }),
        ),
        step(
            "\"Call a scholar\" on the headquarters' flag: a geologist leaves the headquarters (UI-GAME-3.1, ECO-SPC-1.1)",
            vec![
                // The build menu of the step before is still open, maybe
                // over the flag: the right button closes it (UI-GAME-1.10a).
                Act::RightClick,
                field("hq_flag", hq_flag),
                Act::Click,
                button("the scholar button", |_, _, a, _| {
                    matches!(a, Action::Command(Command::OrderGeologist { .. }))
                }),
                Act::Click,
            ],
            400,
            Box::new(|v, n| {
                let at = named(n, "hq_flag")?;
                let good = v.good_id("good_scholar")?;
                if v.specialists_at(at, good) == 0 {
                    return Err(format!(
                        "no geologist is out; the context is {:?}",
                        v.ui.context
                    ));
                }
                if v.ui.context.is_some() {
                    return Err("the flag menu is still open".into());
                }
                Ok(())
            }),
        ),
        step(
            "\"Call the explorer\" on the headquarters' flag: a scout leaves the headquarters (UI-GAME-3.1, ECO-SPC-1.1)",
            vec![
                field("hq_flag", hq_flag),
                Act::Click,
                button("the explorer button", |_, _, a, _| {
                    matches!(a, Action::Command(Command::OrderScout { .. }))
                }),
                Act::Click,
            ],
            400,
            Box::new(|v, n| {
                let at = named(n, "hq_flag")?;
                let good = v.good_id("good_explorer")?;
                if v.specialists_at(at, good) == 0 {
                    return Err(format!(
                        "no scout is out; the context is {:?}",
                        v.ui.context
                    ));
                }
                if v.ui.context.is_some() {
                    return Err("the flag menu is still open".into());
                }
                Ok(())
            }),
        ),
        // OBS-10: the smart cursor takes the pointer back to the flag.
        step(
            "after the flag menu's action the pointer is back on the flag (OBS-10)",
            vec![],
            10,
            Box::new(|v, n| pointer_on(v, named(n, "hq_flag")?, "the headquarters' flag")),
        ),
        // UI-GAME-9.3, T32: the pointer on the headquarters' flag shows
        // "Flag"; on the headquarters, its name (no productivity).
        step(
            "the pointer on the headquarters' flag: the tooltip \"Flag\" (UI-GAME-9.3)",
            // The pointer was taken back onto the flag (OBS-10): it moves
            // off and onto the flag again, as the hand would.
            vec![
                field("hq", |v, _| v.headquarters()),
                field("hq_flag", hq_flag),
            ],
            30,
            Box::new(|v, _| match v.map_tooltip {
                Some(t) if t.eq_ignore_ascii_case("flag") => Ok(()),
                t => Err(format!("the tooltip is {t:?}")),
            }),
        ),
        step(
            "the pointer on the headquarters: its name as the tooltip (UI-GAME-9.3)",
            vec![field("hq", |v, _| v.headquarters())],
            10,
            Box::new(|v, _| match v.map_tooltip {
                Some(t) if !t.is_empty() && !t.contains('%') && !t.starts_with('!') => Ok(()),
                t => Err(format!("the tooltip is {t:?}")),
            }),
        ),
        // UI-GAME-2.2, 8.5, 12.1: C opens the player statistics page; the family's
        // second tab the production statistics.
        step(
            "C opens the player statistics page, not a menu (UI-GAME-2.2, 8.5, 12.1)",
            vec![Act::Key(KeyCode::KeyC, Key::Character("c".into()))],
            10,
            Box::new(|v, _| match v.ui.global {
                Some(Global::Statistics(_)) => Ok(()),
                other => Err(format!("the open dialog is {other:?}")),
            }),
        ),
        step(
            "the family's second tab opens the production statistics (UI-GAME-8.5, 8.5b)",
            vec![
                button("the production tab", |_, _, a, _| {
                    *a == Action::OpenGlobal(Global::Production)
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, _| match v.ui.global {
                Some(Global::Production) => Ok(()),
                other => Err(format!("the open dialog is {other:?}")),
            }),
        ),
        step(
            "the family's Inventory tab opens the inventory (UI-GAME-7.1, 8.5)",
            vec![
                button("the Inventory tab", |_, _, a, _| {
                    *a == Action::OpenGlobal(Global::Inventory(false))
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, _| {
                if v.ui.global != Some(Global::Inventory(false)) {
                    return Err(format!("the open dialog is {:?}", v.ui.global));
                }
                // A count per slot, each the sum over the warehouses
                // (UI-GAME-8.5c: the slots are buttons).
                let all = crate::ui::game::inventory(v.world, v.player);
                let slots = global_widgets(v)
                    .filter(
                        |w| matches!(w, Widget::Button { rect, .. } if *rect == [0, 80, 48, 56]),
                    )
                    .count();
                if slots == 0 || all.is_empty() {
                    return Err(format!("{slots} slots, {} goods in stock", all.len()));
                }
                let shown: u32 = global_widgets(v)
                    .filter_map(|w| match w {
                        Widget::Text { text, .. } => text.parse::<u32>().ok(),
                        _ => None,
                    })
                    .sum();
                let wares: u32 = v
                    .catalog
                    .map(|c| {
                        c.slots(&c.wares)
                            .iter()
                            .filter_map(|(_, g)| all.get(g))
                            .sum()
                    })
                    .unwrap_or(0);
                if shown != wares {
                    return Err(format!(
                        "the slots show {shown} goods, the stock has {wares}"
                    ));
                }
                Ok(())
            }),
        ),
        step(
            "the fleet tab opens the fleet list with its picture (UI-GAME-8.5)",
            vec![
                button("the fleet tab", |_, _, a, _| {
                    *a == Action::OpenGlobal(Global::Fleet)
                }),
                Act::Click,
            ],
            10,
            Box::new(|v, _| {
                if v.ui.global != Some(Global::Fleet) {
                    return Err(format!("the open dialog is {:?}", v.ui.global));
                }
                let ships = crate::world::ships::fleet(v.world, v.player).len().min(11);
                let rows = global_widgets(v)
                    .filter(|w| matches!(w, Widget::TextButton { .. }))
                    .count();
                let picture = global_widgets(v).any(|w| {
                    matches!(w, Widget::Sprite { rect, at, .. } if *rect == [136, 440, 344, 184] && *at == [6.0, 255.0])
                });
                if rows != ships || !picture {
                    return Err(format!("{rows} rows for {ships} ships, picture {picture}"));
                }
                Ok(())
            }),
        ),
        step(
            "the fleet list's close button closes it (UI-GAME-1.8)",
            vec![
                button("the close button", |_, _, a, _| *a == Action::CloseGlobal),
                Act::Click,
            ],
            10,
            Box::new(|v, _| match v.ui.global {
                None => Ok(()),
                g => Err(format!("the open dialog is {g:?}")),
            }),
        ),
        // UI-GAME-12.1, 8.5d: B toggles the buildings overview.
        step(
            "B opens the buildings overview with a cell per type (UI-GAME-8.5d)",
            vec![Act::Key(KeyCode::KeyB, Key::Character("b".into()))],
            10,
            Box::new(|v, _| {
                if v.ui.global != Some(Global::Buildings) {
                    return Err(format!("the open dialog is {:?}", v.ui.global));
                }
                let cells = global_widgets(v)
                    .filter(|w| matches!(w, Widget::Button { rect, .. } if *rect == [0, 0, 80, 80]))
                    .count();
                let types = v
                    .catalog
                    .map_or(0, |c| crate::ui::game::buildings_page_types(c).len());
                if cells == 0 || cells != types {
                    return Err(format!("{cells} cells for {types} types"));
                }
                Ok(())
            }),
        ),
        step(
            "B closes it again (UI-GAME-12.1)",
            vec![Act::Key(KeyCode::KeyB, Key::Character("b".into()))],
            10,
            Box::new(|v, _| match v.ui.global {
                None => Ok(()),
                g => Err(format!("the open dialog is {g:?}")),
            }),
        ),
        // UI-GAME-12.1: , jumps to the next warehouse and opens its window.
        step(
            ", jumps to the headquarters and opens its window (UI-GAME-12.1)",
            vec![Act::Key(KeyCode::Comma, Key::Character(",".into()))],
            10,
            Box::new(|v, _| {
                let hq = v.headquarters()?;
                match &v.ui.context {
                    Some(Context::Building { at, .. }) if *at == hq => Ok(()),
                    c => Err(format!(
                        "the open window is {c:?}, the headquarters at {hq:?}"
                    )),
                }
            }),
        ),
        step(
            "the window's close button closes it (UI-GAME-1.8)",
            vec![
                button("the close button", |_, _, a, _| *a == Action::CloseContext),
                Act::Click,
            ],
            10,
            Box::new(|v, _| match &v.ui.context {
                None => Ok(()),
                c => Err(format!("the open window is {c:?}")),
            }),
        ),
        // UI-GAME-1.10b: a building window warps onto its field when it
        // closes.
        step(
            "after the window closes the pointer is on the headquarters (UI-GAME-1.10b)",
            vec![],
            10,
            Box::new(|v, _| pointer_on(v, v.headquarters()?, "the headquarters")),
        ),
    ]
}

/// The widgets of the open global dialog.
pub(crate) fn global_widgets<'a>(v: &'a PlayView) -> impl Iterator<Item = &'a Widget> + 'a {
    v.panels
        .iter()
        .filter(|p| p.id == PanelId::Global)
        .flat_map(|p| p.widgets.iter())
}

/// UI-MENU-2.1: Select Map from the single-player page (`--menu-screen
/// single`): the map button opens it, the pointer over a row shows that
/// map, a click takes it and closes the dialog.
pub fn select_map() -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction;
    vec![
        menu_step(
            "the map button opens Select Map with its rows (UI-MENU-2.1)",
            vec![
                button("the map button", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::OpenSelectMap)
                }),
                Act::Click,
            ],
            30,
            Box::new(|v, _| {
                if v.menu.screen != Screen::SelectMap {
                    return Err(format!("the screen is {:?}", v.menu.screen));
                }
                let rows = v
                    .panels
                    .iter()
                    .flat_map(|p| p.widgets.iter())
                    .filter(|w| {
                        matches!(
                            w,
                            Widget::TextButton {
                                action: Action::Menu(MenuAction::SelectMap(_)),
                                ..
                            }
                        )
                    })
                    .count();
                let want = v.menu.maps.len().min(crate::menus::menu::MAP_ROWS);
                if rows != want || want == 0 {
                    return Err(format!("{rows} rows, want {want}"));
                }
                Ok(())
            }),
        ),
        menu_step(
            "the pointer over a row of Select Map shows that map (UI-MENU-2.1)",
            vec![button("the third map", |_, _, a, _| {
                *a == Action::Menu(MenuAction::SelectMap(2))
            })],
            30,
            Box::new(|v, _| match crate::menus::menu::shown_map(v.menu) {
                Some(s) if s.path == v.menu.maps[2] => Ok(()),
                s => Err(format!("shown {:?}", s.map(|s| &s.name))),
            }),
        ),
        menu_step(
            "a click on a row takes its map and closes Select Map (UI-MENU-2.1)",
            vec![
                button("the second map", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::SelectMap(1))
                }),
                Act::Click,
            ],
            30,
            Box::new(|v, _| match &v.menu.summary {
                Some(s)
                    if v.menu.screen == Screen::SinglePlayer
                        && v.menu.map_choice == 1
                        && s.path == v.menu.maps[1] =>
                {
                    Ok(())
                }
                s => Err(format!(
                    "screen {:?}, map {}, summary {:?}",
                    v.menu.screen,
                    v.menu.map_choice,
                    s.as_ref().map(|s| &s.name)
                )),
            }),
        ),
    ]
}

/// The name the save-quit-load scripts save under.
pub const HARNESS_SAVE: &str = "harness";

/// The keys that type `text` (lower-case letters).
fn type_keys(text: &str) -> Vec<Act> {
    text.chars()
        .filter_map(|c| {
            let code = match c {
                'a' => KeyCode::KeyA,
                'e' => KeyCode::KeyE,
                'h' => KeyCode::KeyH,
                'n' => KeyCode::KeyN,
                'r' => KeyCode::KeyR,
                's' => KeyCode::KeyS,
                _ => return None,
            };
            Some(Act::Key(code, Key::Character(c.to_string().into())))
        })
        .collect()
}

/// UI-MENU-3.1, 5.1: the first half of save-quit-load, in a free game: it
/// runs a while, Escape opens the game menu, Save Game the save dialog with
/// the proposed name, the name is typed over, `!Save` writes the game and
/// goes back to it. The script's end quits the program.
pub fn save_game() -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::{Input, MenuAction};
    let mut type_name = vec![
        button("the save name input", |_, _, a, _| {
            matches!(a, Action::FocusInput(Input::SaveName, _))
        }),
        Act::Click,
        Act::Key(KeyCode::End, Key::End),
    ];
    type_name.extend(keys(KeyCode::Backspace, Key::Backspace, 64));
    type_name.extend(type_keys(HARNESS_SAVE));
    vec![
        step(
            "the game runs (ECO-TIME-1.2)",
            Vec::new(),
            3000,
            Box::new(|v, _| {
                if v.world.tick() >= 400 {
                    Ok(())
                } else {
                    Err(format!("tick {}", v.world.tick()))
                }
            }),
        ),
        menu_step(
            "Escape opens the game menu (UI-MENU-3.1)",
            vec![Act::Key(KeyCode::Escape, Key::Escape)],
            60,
            Box::new(|v, _| match v.menu.screen {
                Screen::GameMenu => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "Save Game opens the save dialog with the proposed name (UI-MENU-5.1, FMT-SAV-1.3)",
            vec![
                button("Save Game", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::SaveGame)
                }),
                Act::Click,
            ],
            60,
            Box::new(|v, _| {
                if v.menu.screen != Screen::Save {
                    return Err(format!("the screen is {:?}", v.menu.screen));
                }
                if v.menu.save_name.len() < 20 {
                    return Err(format!("the proposed name is {:?}", v.menu.save_name));
                }
                Ok(())
            }),
        ),
        menu_step(
            "the name is typed over (UI-MENU-5.3)",
            type_name,
            60,
            Box::new(|v, _| {
                if v.menu.save_name == HARNESS_SAVE {
                    Ok(())
                } else {
                    Err(format!("the name is {:?}", v.menu.save_name))
                }
            }),
        ),
        menu_step(
            "Save writes the game and goes back to it (UI-MENU-5.1)",
            vec![
                button("Save", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::WriteSave)
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match &v.menu.last_save {
                Some(saved)
                    if saved.path.is_file()
                        && crate::game::map_name(&saved.path) == HARNESS_SAVE
                        && v.menu.screen == Screen::None =>
                {
                    eprintln!(
                        "saved tick {} state hash {:016x} computer players {:?}",
                        saved.tick, saved.world_hash, saved.ai_hash
                    );
                    Ok(())
                }
                s => Err(format!("saved {s:?}, the screen {:?}", v.menu.screen)),
            }),
        ),
        step(
            "the autosave is written after its interval (FMT-SAV-5.1, 5.2; --autosave 60)",
            Vec::new(),
            1500,
            Box::new(|v, _| {
                let folder = crate::menus::menu::save_folder().ok_or("no save folder")?;
                let name = crate::menus::menu::autosave_name(v.menu);
                let path = crate::game::save::path_of(&folder, &name);
                if path.is_file() {
                    Ok(())
                } else {
                    Err(format!("no {} at tick {}", path.display(), v.world.tick()))
                }
            }),
        ),
    ]
}

/// UI-MENU-1.5: a third start of the program: CONTINUE on the
/// single-player page loads the autosave, the game as it was autosaved.
pub fn continue_game() -> Vec<Step> {
    use crate::ui::widgets::MenuAction;
    vec![
        menu_step(
            "CONTINUE loads the autosave as it was saved (UI-MENU-1.5, FMT-SAV-5.1)",
            vec![
                button("CONTINUE", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::Continue)
                }),
                Act::Click,
            ],
            600,
            Box::new(|v, _| match &v.menu.last_load {
                Some(check)
                    if check.same()
                        && crate::game::map_name(&check.path)
                            == crate::menus::menu::autosave_name(v.menu) =>
                {
                    eprintln!(
                        "continued tick {} state hash {:016x}",
                        check.tick, check.loaded.0
                    );
                    Ok(())
                }
                Some(check) => Err(format!("loaded {check:?}")),
                None => Err(format!("nothing loaded; the notice {:?}", v.menu.notice)),
            }),
        ),
        step(
            "the continued game plays on (UI-MENU-1.5, ECO-TIME-1.2)",
            Vec::new(),
            3000,
            Box::new(|v, _| {
                let from = v.menu.last_load.as_ref().map_or(0, |c| c.tick);
                if v.world.tick() >= from + 200 {
                    Ok(())
                } else {
                    Err(format!("tick {} (loaded at {from})", v.world.tick()))
                }
            }),
        ),
    ]
}

/// UI-MENU-1.5, 5.2: the second half of save-quit-load, from the
/// single-player page of a new start of the program: LOAD GAME lists the
/// save, Load continues it, and the game loaded has the state hashes the
/// game had when it was saved; then it plays on.
pub fn load_game() -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction;
    vec![
        menu_step(
            "LOAD GAME lists the saves (UI-MENU-1.5, 5.2, 6.1)",
            vec![
                button("LOAD GAME", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::LoadGame)
                }),
                Act::Click,
            ],
            60,
            Box::new(|v, _| {
                if !matches!(v.menu.screen, Screen::Load { .. }) {
                    return Err(format!("the screen is {:?}", v.menu.screen));
                }
                if v.menu
                    .saves
                    .iter()
                    .any(|e| e.ours && e.name == HARNESS_SAVE)
                {
                    Ok(())
                } else {
                    Err(format!("the list is {:?}", v.menu.saves))
                }
            }),
        ),
        menu_step(
            "a click chooses the save (UI-MENU-5.2)",
            vec![
                button("the harness's save", |v, _, a, _| match a {
                    Action::Menu(MenuAction::SelectSave(i)) => v
                        .menu
                        .saves
                        .get(*i)
                        .is_some_and(|e| e.ours && e.name == HARNESS_SAVE),
                    _ => false,
                }),
                Act::Click,
            ],
            30,
            Box::new(|v, _| match v.menu.saves.get(v.menu.save_choice) {
                Some(e) if e.name == HARNESS_SAVE => Ok(()),
                e => Err(format!("chosen {e:?}")),
            }),
        ),
        menu_step(
            "Load continues the game as it was saved: the same state hashes (UI-MENU-5.2)",
            vec![
                button("Load", |_, _, a, _| {
                    *a == Action::Menu(MenuAction::LoadSave)
                }),
                Act::Click,
            ],
            600,
            Box::new(|v, _| match &v.menu.last_load {
                Some(check) if check.same() => {
                    eprintln!(
                        "loaded tick {} state hash {:016x} computer players {:?}",
                        check.tick, check.loaded.0, check.loaded.1
                    );
                    Ok(())
                }
                Some(check) => Err(format!(
                    "saved with {:?}, loaded as {:?}",
                    check.saved, check.loaded
                )),
                None => Err(format!("nothing loaded; the notice {:?}", v.menu.notice)),
            }),
        ),
        step(
            "the loaded game plays on (UI-MENU-5.2, ECO-TIME-1.2)",
            Vec::new(),
            3000,
            Box::new(|v, _| {
                let from = v.menu.last_load.as_ref().map_or(0, |c| c.tick);
                if v.world.tick() >= from + 200 {
                    Ok(())
                } else {
                    Err(format!("tick {} (loaded at {from})", v.world.tick()))
                }
            }),
        ),
    ]
}

/// CAM-5.2: the tutorial is at this step, and its window shows the step's
/// text from the language file (not the key).
fn tutorial_at(v: &PlayView, step: u8) -> Result<(), String> {
    use crate::menus::campaign::step_key;
    let Some(t) = &v.mission.tutorial else {
        return Err("no tutorial is running".into());
    };
    if t.step != step {
        return Err(format!(
            "the tutorial is at {} (0x{:02x}), not {}: {:?}",
            step_key(t.step),
            t.step,
            step_key(step),
            t.error
        ));
    }
    let window = v
        .panels
        .iter()
        .find(|p| p.id == PanelId::Tutorial)
        .ok_or("the tutorial window is not on screen")?;
    let shown = window
        .widgets
        .iter()
        .any(|w| matches!(w, Widget::Paragraph { text, .. } if !text.is_empty() && text != step_key(step)));
    if !shown {
        return Err(format!("the window does not show {}", step_key(step)));
    }
    Ok(())
}

/// The tutorial's next step after one of its button steps: the pointer on
/// the button its text asks for, a click.
fn click_button(
    name: &'static str,
    step: u8,
    what: &'static str,
    wanted: impl Fn(&PlayView, &Named, &Action, Option<&str>) -> bool + Send + Sync + 'static,
) -> Step {
    self::step(
        name,
        vec![button(what, wanted), Act::Click],
        60,
        Box::new(move |v, _| tutorial_at(v, step + 1)),
    )
}

/// A map step of the tutorial: as a person does, the marked field scrolled into
/// view, the pointer on it, a click.
fn click_field(name: &'static str, step: u8) -> Step {
    self::step(
        name,
        vec![
            Act::Reveal(
                "the marked field",
                Box::new(move |_, _| {
                    crate::menus::campaign::pointer(step)
                        .ok_or_else(|| "no marked field".to_owned())
                }),
            ),
            field("the marked field", move |_, _| {
                crate::menus::campaign::pointer(step).ok_or_else(|| "no marked field".to_owned())
            }),
            Act::Click,
        ],
        60,
        Box::new(move |v, _| tutorial_at(v, step + 1)),
    )
}

/// The build menu's button that places the building named `name`.
fn places(name: &'static str) -> impl Fn(&PlayView, &Named, &Action, Option<&str>) -> bool {
    move |v, _, a, _| {
        matches!(a, Action::Command(Command::PlaceBuilding { building, .. })
            if v.building_id(name).is_ok_and(|id| id == *building))
    }
}

/// `Hud(3)`: the bar's construction view button.
fn construction_view(_: &PlayView, _: &Named, a: &Action, _: Option<&str>) -> bool {
    matches!(a, Action::Hud(3))
}

/// The flag menu's "Build road".
fn starts_road(_: &PlayView, _: &Named, a: &Action, _: Option<&str>) -> bool {
    matches!(a, Action::StartRoad(_))
}

/// CAM-5, T8: lesson 1 as a person plays it: from the single-player page's
/// TUTORIAL and the lesson's button, each instruction followed with the click
/// it asks for, to the lesson's end; then Play, and the village gets built.
pub fn tutorial_one() -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    let medium_tab = |_: &PlayView, _: &Named, a: &Action, _: Option<&str>| {
        // UI-GAME-3.2: the tabs small, medium, large.
        matches!(a, Action::BuildTab(6))
    };
    vec![
        menu_step(
            "TUTORIAL opens the lesson page (UI-MENU-1.6)",
            vec![
                button("TUTORIAL", |_, _, a, _| {
                    matches!(a, Action::Menu(M::Tutorial))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                if v.menu.screen == Screen::Tutorial {
                    Ok(())
                } else {
                    Err(format!("the menu is on {:?}", v.menu.screen))
                }
            }),
        ),
        menu_step(
            "the first lesson's button starts map tutorial01 (UI-MENU-1.6, CAM-5.1)",
            vec![
                button("Lesson 1", |_, _, a, _| {
                    matches!(a, Action::Menu(M::StartLesson(1)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                if v.menu.screen == Screen::None {
                    Ok(())
                } else {
                    Err(format!("the menu is on {:?}", v.menu.screen))
                }
            }),
        ),
        step(
            "the lesson starts at T01_Introduction with its window (CAM-5.2, T8)",
            vec![],
            60,
            Box::new(|v, _| {
                if v.mission.map != "tutorial01" || v.mission.kind != 2 {
                    return Err(format!("the map is {} {}", v.mission.map, v.mission.kind));
                }
                tutorial_at(v, 0x1d)
            }),
        ),
        click_button(
            "T01_Introduction: the window's button goes on (CAM-5.2)",
            0x1d,
            "the window's button",
            |_, _, a, _| matches!(a, Action::Menu(M::TutorialNext)),
        ),
        click_button(
            "T01_ShowConstructionMap: the bar's construction view (CAM-5.3)",
            0x1e,
            "the construction view button",
            construction_view,
        ),
        step(
            "a click off the marked field is refused and keeps the step (CAM-5.3, T8)",
            vec![field("a field not marked", |_, _| Ok((41, 38))), Act::Click],
            60,
            Box::new(|v, _| {
                tutorial_at(v, 0x1f)?;
                let t = v.mission.tutorial.as_ref().expect("checked");
                if t.error != Some("T01_ERROR_SelectMarkedPosition") {
                    return Err(format!("the error is {:?}", t.error));
                }
                if v.ui.context.is_some() {
                    return Err("the click opened a dialog".into());
                }
                Ok(())
            }),
        ),
        click_field(
            "T01_WoodCutter_OpenSelectBuilding: the marked field opens the build menu (CAM-5.3)",
            0x1f,
        ),
        step(
            "the build menu opens at (20, 20) during the tutorial (UI-GAME-1.9, T15)",
            vec![],
            5,
            Box::new(|v, _| {
                let open = matches!(v.ui.context, Some(Context::Build { .. }));
                let at = v.panels.iter().find(|p| p.id == PanelId::Context);
                match at {
                    Some(p) if open && p.at == [20.0, 20.0] => Ok(()),
                    Some(p) => Err(format!("{:?} at {:?}", v.ui.context, p.at)),
                    None => Err("no context dialog".into()),
                }
            }),
        ),
        click_button(
            "T01_WoodCutter_SelectBuilding: the woodcutter's button (CAM-5.3)",
            0x20,
            "the woodcutter's button",
            places("WoodcutterHut"),
        ),
        click_field(
            "T01_WoodCutter_StartStreet: the site's flag opens the flag menu (CAM-5.3)",
            0x21,
        ),
        click_button(
            "T01_WoodCutter_SelectCreateStreet: Build road (CAM-5.3)",
            0x22,
            "Build road",
            starts_road,
        ),
        click_field(
            "T01_WoodCutter_EndStreet: the headquarters' flag ends the road (CAM-5.3, UI-GAME-3.4)",
            0x23,
        ),
        step(
            "the woodcutter's road stands (TRN-ROAD-2)",
            vec![],
            60,
            Box::new(|v, _| {
                let n = v.world.roads().count();
                if n == 1 {
                    Ok(())
                } else {
                    Err(format!("{n} roads"))
                }
            }),
        ),
        click_field(
            "T01_SawMill_OpenSelectBuilding: the marked field (CAM-5.3)",
            0x24,
        ),
        click_button(
            "T01_SawMill_SelectPage: the medium buildings' tab (CAM-5.3, UI-GAME-3.2)",
            0x25,
            "the medium tab",
            medium_tab,
        ),
        click_button(
            "T01_SawMill_SelectBuilding: the sawmill's button (CAM-5.3)",
            0x26,
            "the sawmill's button",
            places("Sawmill"),
        ),
        click_field(
            "T01_Forester_OpenSelectBuilding: the marked field (CAM-5.3)",
            0x27,
        ),
        click_button(
            "T01_Forester_SelectBuilding: the forester's button (CAM-5.3)",
            0x28,
            "the forester's button",
            places("ForesterHouse"),
        ),
        click_field("T01_Forester_StartStreet: the site's flag (CAM-5.3)", 0x29),
        click_button(
            "T01_Forester_SelectCreateStreet: Build road (CAM-5.3)",
            0x2a,
            "Build road",
            starts_road,
        ),
        click_field(
            "T01_Forester_EndStreet: the headquarters' flag (CAM-5.3)",
            0x2b,
        ),
        click_field(
            "T01_StoneMason_OpenSelectBuilding: the marked field (CAM-5.3)",
            0x2c,
        ),
        click_button(
            "T01_StoneMason_SelectBuilding: the stonemason's button (CAM-5.3)",
            0x2d,
            "the stonemason's button",
            places("Stone Pit"),
        ),
        click_field(
            "T01_StoneMason_StartStreet: the site's flag (CAM-5.3)",
            0x2e,
        ),
        click_button(
            "T01_StoneMason_SelectCreateStreet: Build road (CAM-5.3)",
            0x2f,
            "Build road",
            starts_road,
        ),
        click_field(
            "T01_StoneMason_EndStreet: the marked field on the road (CAM-5.3)",
            0x30,
        ),
        click_field(
            "T01_StoneMason_OpenFlagDialog: the marked field on the road (CAM-5.3)",
            0x31,
        ),
        click_button(
            "T01_StoneMason_SelectFlag: the road menu's flag (CAM-5.3, TRN-ROAD-4)",
            0x32,
            "the flag button",
            |_, _, a, _| matches!(a, Action::Command(Command::PlaceFlag { .. })),
        ),
        click_button(
            "T01_HideConstructionMap: the bar's construction view (CAM-5.3)",
            0x33,
            "the construction view button",
            construction_view,
        ),
        step(
            "T01_End offers Play and EXIT MAP; the village is laid out (CAM-5.3)",
            vec![],
            60,
            Box::new(|v, _| {
                tutorial_at(v, 0x34)?;
                let offers = |wanted: fn(&Action) -> bool| v.button(&|a, _| wanted(a)).is_some();
                if !offers(|a| matches!(a, Action::Menu(M::TutorialPlay)))
                    || !offers(|a| matches!(a, Action::Menu(M::EndExit)))
                {
                    return Err("Play or EXIT MAP is not offered".into());
                }
                let sites = [(42, 35), (43, 37), (46, 35), (40, 42)];
                if let Some(missing) = sites.iter().find(|&&f| v.world.building_at(f).is_none()) {
                    return Err(format!("no building on {missing:?}"));
                }
                if v.overlay {
                    return Err("the construction view is still on".into());
                }
                Ok(())
            }),
        ),
        click_play(),
        step(
            "the village gets built: the woodcutter's hut stands (ECO-HQ-4, CAM-5.3)",
            vec![],
            4000,
            Box::new(|v, _| match v.world.building_at((42, 35)) {
                Some(b) if !b.under_construction => Ok(()),
                Some(_) => Err("the hut is still a site".into()),
                None => Err("no hut".into()),
            }),
        ),
    ]
}

/// CAM-5: the steps from the single-player page to a lesson's first instruction.
fn start_lesson(
    open: &'static str,
    press: &'static str,
    window: &'static str,
    lesson: u8,
    map: &'static str,
    first: u8,
) -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    vec![
        menu_step(
            open,
            vec![
                button("TUTORIAL", |_, _, a, _| {
                    matches!(a, Action::Menu(M::Tutorial))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                if v.menu.screen == Screen::Tutorial {
                    Ok(())
                } else {
                    Err(format!("the menu is on {:?}", v.menu.screen))
                }
            }),
        ),
        menu_step(
            press,
            vec![
                button(
                    "the lesson's button",
                    move |_, _, a, _| matches!(a, Action::Menu(M::StartLesson(n)) if *n == lesson),
                ),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                if v.menu.screen == Screen::None {
                    Ok(())
                } else {
                    Err(format!("the menu is on {:?}", v.menu.screen))
                }
            }),
        ),
        step(
            window,
            vec![],
            60,
            Box::new(move |v, _| {
                if v.mission.map != map || v.mission.kind != 2 {
                    return Err(format!("the map is {} {}", v.mission.map, v.mission.kind));
                }
                tutorial_at(v, first)
            }),
        ),
    ]
}

/// A hand on the wheel: the camera a step further out, so that a lesson's marked
/// fields are on screen in the harness's small window (GFX-VIEW-3.1).
fn zoom_out_over(name: &'static str, at: Point) -> Step {
    step(
        name,
        vec![
            // CAM-5.3b: the step just begun may move the camera; no camera
            // input is read until the move is over (GFX-VIEW-3.6 e), so the
            // hand waits for it, then finds the flag.
            Act::Reveal("the headquarters' flag", Box::new(move |_, _| Ok(at))),
            field("the headquarters' flag", move |_, _| Ok(at)),
            Act::Wheel(-1.0),
        ],
        30,
        Box::new(|v, _| zoom_is(v, 4)),
    )
}

/// A step that only explains: the window's button goes on (CAM-5.2).
fn explain(name: &'static str, step: u8) -> Step {
    use crate::ui::widgets::MenuAction as M;
    click_button(name, step, "the window's button", |_, _, a, _| {
        matches!(a, Action::Menu(M::TutorialNext))
    })
}

/// The tab of the build menu that lists the building `name`, by its size
/// (UI-GAME-3.2, 3.5: 5 small, 6 medium, 7 large).
fn tab_holding(name: &'static str) -> impl Fn(&PlayView, &Named, &Action, Option<&str>) -> bool {
    use s2_sim::BuildingSize as Size;
    move |v, _, a, _| {
        let size = v
            .building_id(name)
            .ok()
            .and_then(|id| v.world.building_type(id))
            .map(|kind| kind.size);
        let tab = match size {
            Some(Size::Small) => 5,
            Some(Size::Medium) => 6,
            Some(Size::Large | Size::Harbor) => 7,
            _ => return false,
        };
        *a == Action::BuildTab(tab)
    }
}

/// CAM-5, T8: lesson 2 as a person plays it: each instruction followed with the
/// click it asks for, in the order of CAM-5.2's table, to the lesson's end.
pub fn tutorial_two() -> Vec<Step> {
    let mut steps = start_lesson(
        "TUTORIAL opens the lesson page for lesson 2 (UI-MENU-1.6)",
        "the second lesson's button starts map tutorial02 (UI-MENU-1.6, CAM-5.1)",
        "lesson 2 starts at T02_Introduction with its window (CAM-5.2)",
        2,
        "tutorial02",
        0x35,
    );
    steps.extend([
        explain(
            "T02_Introduction: the window's button goes on (CAM-5.2)",
            0x35,
        ),
        zoom_out_over(
            "lesson 2: the wheel takes the camera a step out (GFX-VIEW-3.1)",
            (45, 37),
        ),
        click_field(
            "T02_Flag_OpenSelectBuilding: the marked field opens its menu (CAM-5.3)",
            0x36,
        ),
        click_button(
            "T02_Flag_SelectBuilding: the menu's flag (CAM-5.3, TRN-ROAD-4)",
            0x37,
            "the flag button",
            |_, _, a, _| matches!(a, Action::Command(Command::PlaceFlag { .. })),
        ),
        click_field("T02_Flag_StartStreet: the new flag (CAM-5.3)", 0x38),
        click_button(
            "T02_Flag_SelectCreateStreet: Build road (CAM-5.3)",
            0x39,
            "Build road",
            starts_road,
        ),
        click_field(
            "T02_Flag_EndStreet: the headquarters' flag ends the road (CAM-5.3, UI-GAME-3.4)",
            0x3a,
        ),
        click_field(
            "T02_Scholar_OpenFlagDialog: the flag again (CAM-5.2, CAM-5.3)",
            0x3b,
        ),
        click_button(
            "T02_Scholar_CallScholar: the flag menu's geologist (CAM-5.2, ECO-SPC-3)",
            0x3c,
            "the geologist's button",
            |_, _, a, _| matches!(a, Action::Command(Command::OrderGeologist { .. })),
        ),
        click_field(
            "T02_Coal_OpenSelectBuildingDialog: the marked field (CAM-5.3)",
            0x3d,
        ),
        click_button(
            "T02_Coal_SelectBuilding: the coal mine's button (CAM-5.3)",
            0x3e,
            "the coal mine's button",
            places("Coal Mine"),
        ),
        click_field("T02_Coal_StartStreet: the mine's flag (CAM-5.3)", 0x3f),
        click_button(
            "T02_Coal_SelectCreateStreet: Build road (CAM-5.3)",
            0x40,
            "Build road",
            starts_road,
        ),
        click_field(
            "T02_Coal_EndStreet: the flag on the road (CAM-5.2, CAM-5.3)",
            0x41,
        ),
        click_field(
            "T02_Iron_OpenSelectBuildingDialog: the marked field (CAM-5.3)",
            0x42,
        ),
        click_button(
            "T02_Iron_SelectBuilding: the iron mine's button (CAM-5.3)",
            0x43,
            "the iron mine's button",
            places("Iron Mine"),
        ),
        click_field("T02_Iron_StartStreet: the mine's flag (CAM-5.3)", 0x44),
        click_button(
            "T02_Iron_SelectCreateStreet: Build road (CAM-5.3)",
            0x45,
            "Build road",
            starts_road,
        ),
        click_field(
            "T02_Iron_EndStreet: the coal mine's flag (CAM-5.2, CAM-5.3)",
            0x46,
        ),
        click_field(
            "T02_Smeltery_OpenSelectBuilding: the marked field (CAM-5.3)",
            0x47,
        ),
        click_button(
            "T02_Smeltery_SelectPage: a tab of the build menu (CAM-5.3, UI-GAME-3.2)",
            0x48,
            "the smeltery's tab",
            tab_holding("Smeltery"),
        ),
        click_button(
            "T02_Smeltery_SelectBuilding: the smeltery's button (CAM-5.3)",
            0x49,
            "the smeltery's button",
            places("Smeltery"),
        ),
        click_field(
            "T02_Smith_OpenSelectBuildingDialog: the marked field (CAM-5.3)",
            0x4a,
        ),
        click_button(
            "T02_Smith_SelectPage: a tab of the build menu (CAM-5.3, UI-GAME-3.2)",
            0x4b,
            "the smithy's tab",
            tab_holding("Smithy"),
        ),
        click_button(
            "T02_Smith_SelectBuilding: the smithy's button (CAM-5.3)",
            0x4c,
            "the smithy's button",
            places("Smithy"),
        ),
        click_field("T02_Smith_StartStreet: the smithy's flag (CAM-5.3)", 0x4d),
        click_button(
            "T02_Smith_SelectCreateStreet: Build road (CAM-5.3)",
            0x4e,
            "Build road",
            starts_road,
        ),
        click_field(
            "T02_Smith_EndStreet: the headquarters' flag (CAM-5.2, CAM-5.3)",
            0x4f,
        ),
        step(
            "T02_End: the lesson's last step (CAM-5.3)",
            vec![],
            60,
            Box::new(|v, _| tutorial_at(v, 0x50)),
        ),
        click_play(),
    ]);
    steps
}

/// CAM-5, T8: lesson 3 as a person plays it, to its attack and the military
/// dialog (CAM-5.1: the script's guard house, watchtower and enemy barracks).
pub fn tutorial_three() -> Vec<Step> {
    use crate::ui::widgets::Global;
    let mut steps = start_lesson(
        "TUTORIAL opens the lesson page for lesson 3 (UI-MENU-1.6)",
        "the third lesson's button starts map tutorial03 (UI-MENU-1.6, CAM-5.1)",
        "lesson 3 starts at T03_Introduction with its window (CAM-5.2)",
        3,
        "tutorial03",
        0x0e,
    );
    steps.extend([
        explain(
            "T03_Introduction: the window's button goes on (CAM-5.2)",
            0x0e,
        ),
        zoom_out_over(
            "lesson 3: the wheel takes the camera a step out (GFX-VIEW-3.1)",
            (45, 37),
        ),
        click_field(
            "T03_Barracks_OpenSelectBuildingDialog: the marked field (CAM-5.3)",
            0x0f,
        ),
        click_button(
            "T03_Barracks_SelectBuilding: the barracks' button (CAM-5.3)",
            0x10,
            "the barracks' button",
            places("Barracks"),
        ),
        click_field("T03_Barracks_StartStreet: the site's flag (CAM-5.3)", 0x11),
        click_button(
            "T03_Barracks_SelectCreateStreet: Build road (CAM-5.3)",
            0x12,
            "Build road",
            starts_road,
        ),
        click_field("T03_Barracks_EndStreet: the marked flag (CAM-5.3)", 0x13),
        click_field(
            "T03_Catapult_OpenSelectBuildingDialog: the marked field (CAM-5.3)",
            0x14,
        ),
        click_button(
            "T03_Catapult_SelectPage: a tab of the build menu (CAM-5.3, UI-GAME-3.2)",
            0x15,
            "the catapult's tab",
            tab_holding("Catapult"),
        ),
        click_button(
            "T03_Catapult_SelectBuilding: the catapult's button (CAM-5.3)",
            0x16,
            "the catapult's button",
            places("Catapult"),
        ),
        click_field(
            "T03_Attack_OpenAttackDialog: the enemy's barracks (CAM-5.1, CAM-5.3)",
            0x17,
        ),
        // A person waits: until a building of the player's has a soldier to spare,
        // the dialog says 0 and its order is off.
        step(
            "soldiers can be sent at the enemy's barracks (MIL-ATK-1.4, CAM-5.1)",
            vec![],
            20000,
            Box::new(|v, _| {
                let target = crate::menus::campaign::pointer(0x17).expect("a marked field");
                match v.world.attack_available(v.player, target) {
                    0 => Err("no soldier to spare".to_owned()),
                    _ => Ok(()),
                }
            }),
        ),
        click_button(
            "T03_Attack_StartAttack: the attack dialog's order (CAM-5.3, MIL-ATK-1.5)",
            0x18,
            "the attack button",
            |_, _, a, _| matches!(a, Action::AttackGo),
        ),
        click_button(
            "T03_Misc_OpenMilitaryDialog: the bar's tools (CAM-5.3)",
            0x19,
            "the bar's button",
            |_, _, a, _| matches!(a, Action::Hud(6)),
        ),
        click_button(
            "T03_Misc_SelectPage: the military page (CAM-5.3)",
            0x1a,
            "the military page",
            |_, _, a, _| matches!(a, Action::OpenGlobal(Global::Military)),
        ),
        click_button(
            "T03_Misc_CloseDialog: its close button (CAM-5.3)",
            0x1b,
            "the close button",
            |_, _, a, _| matches!(a, Action::CloseGlobal | Action::CloseContext),
        ),
        step(
            "T03_End: the lesson's last step (CAM-5.3)",
            vec![],
            60,
            Box::new(|v, _| tutorial_at(v, 0x1c)),
        ),
        click_play(),
    ]);
    steps
}

/// CAM-5, T8: lesson 4 as a person plays it: a ship, an expedition, a harbour
/// on the far shore (CAM-5.1: the script's harbour, shipyard and two ships).
pub fn tutorial_four() -> Vec<Step> {
    let close = |_: &PlayView, _: &Named, a: &Action, _: Option<&str>| {
        matches!(a, Action::CloseGlobal | Action::CloseContext)
    };
    let mut steps = start_lesson(
        "TUTORIAL opens the lesson page for lesson 4 (UI-MENU-1.6)",
        "the fourth lesson's button starts map tutorial04 (UI-MENU-1.6, CAM-5.1)",
        "lesson 4 starts at T04_Introduction with its window (CAM-5.2)",
        4,
        "tutorial04",
        0x00,
    );
    steps.extend([
        explain(
            "T04_Introduction: the window's button goes on (CAM-5.2)",
            0x00,
        ),
        click_field("T04_BuildShip_SelectShipYard: the shipyard (CAM-5.3)", 0x01),
        click_button(
            "T04_BuildShip_ChangeProduction: its boat / ship switch (CAM-5.3c)",
            0x02,
            "the boat / ship switch",
            |_, _, a, _| matches!(a, Action::Command(Command::ProductionSwitch { .. })),
        ),
        click_button(
            "T04_BuildShip_CloseDialog: its close button (CAM-5.3)",
            0x03,
            "the close button",
            close,
        ),
        click_field(
            "T04_StartExpedition_SelectPort: the harbour (CAM-5.3)",
            0x04,
        ),
        click_button(
            "T04_StartExpedition_SelectPage: the dialog's tab (CAM-5.3)",
            0x05,
            "the expedition tab",
            |_, _, a, _| matches!(a, Action::BuildingTab(12)),
        ),
        click_button(
            "T04_StartExpedition_SelectIcon: the expedition's button (CAM-5.3, SHIP-7)",
            0x06,
            "the expedition's button",
            |_, _, a, _| matches!(a, Action::Command(Command::ExpeditionStart { .. })),
        ),
        click_button(
            "T04_StartExpedition_CloseDialog: its close button (CAM-5.3)",
            0x07,
            "the close button",
            close,
        ),
        step(
            "the expedition is ready at the harbour (SHIP-7, CAM-5.1)",
            vec![],
            6000,
            Box::new(|v, _| {
                let at = crate::menus::campaign::pointer(0x08).expect("a marked field");
                match crate::ui::game::context_for(v.world, crate::game::PLAYER, at) {
                    Some(Context::Ship(_)) => Ok(()),
                    other => Err(format!("the marked field gives {other:?}")),
                }
            }),
        ),
        click_field(
            "T04_StartSailing_SelectShip: the ship on the marked field (CAM-5.3a)",
            0x08,
        ),
        click_button(
            "T04_StartSailing_SelectArrow: the SouthWest compass button (CAM-5.3a, SHIP-6.2)",
            0x09,
            "the compass button of direction 2",
            |_, _, a, _| {
                matches!(
                    a,
                    Action::Command(Command::ExpeditionGo { direction: 2, .. })
                )
            },
        ),
        step(
            "T04_BuildPort_SelectShip: the right fleet arrow shows the second ship and jumps there (CAM-5.3a, UI-GAME-4.5)",
            vec![
                button("the right fleet arrow", |_, _, a, _| {
                    matches!(a, Action::ShipArrow { right: true, .. })
                }),
                Act::Click,
            ],
            60,
            Box::new(|v, _| {
                tutorial_at(v, 0x0b)?;
                let Some(Context::Ship(i)) = v.ui.context else {
                    return Err(format!("the window shows {:?}", v.ui.context));
                };
                let at = v.world.ships()[i].position();
                if at != (127, 36) {
                    return Err(format!("the window shows the ship at {at:?}, not (127, 36)"));
                }
                // The jump: the ship's field is on screen.
                if !v.in_view(at) {
                    return Err(format!("the camera did not jump to the ship at {at:?}"));
                }
                Ok(())
            }),
        ),
        click_button(
            "T04_BuildPort_SelectIcon: the harbour's button (CAM-5.3, SHIP-7.2)",
            0x0b,
            "the harbour's button",
            |_, _, a, _| matches!(a, Action::Command(Command::ExpeditionBuildHarbor { .. })),
        ),
        click_button(
            "T04_BuildPort_CloseDialog: its close button (CAM-5.3)",
            0x0c,
            "the close button",
            close,
        ),
        step(
            "T04_EndTutorial: the lesson's last step (CAM-5.3)",
            vec![],
            60,
            Box::new(|v, _| tutorial_at(v, 0x0d)),
        ),
        click_play(),
    ]);
    steps
}

/// CAM-5.3: Play ends the guidance and the game goes on.
fn click_play() -> Step {
    use crate::ui::widgets::MenuAction as M;
    step(
        "Play ends the guidance and the game goes on (CAM-5.3)",
        vec![
            button("Play", |_, _, a, _| {
                matches!(a, Action::Menu(M::TutorialPlay))
            }),
            Act::Click,
        ],
        60,
        Box::new(|v, _| {
            if v.mission.tutorial.is_some() {
                return Err("the tutorial is still running".into());
            }
            if v.panels.iter().any(|p| p.id == PanelId::Tutorial) {
                return Err("its window is still on screen".into());
            }
            Ok(())
        }),
    )
}

/// A book is open with these pages, and its panel on screen (CAM-1.3).
fn book_is(v: &PlayView, first: &str, debriefing: bool) -> Result<(), String> {
    let Some(book) = &v.mission.book else {
        return Err(format!("no book at tick {}", v.world.tick()));
    };
    if book.pages.first().map(String::as_str) != Some(first) || book.debriefing != debriefing {
        return Err(format!("the book is {book:?}"));
    }
    if !v.panels.iter().any(|p| p.id == PanelId::Book) {
        return Err("its panel is not on screen".into());
    }
    Ok(())
}

/// The book's next page, then its CONTINUE (CAM-1.3): the pointer and clicks.
fn book_through() -> Vec<Act> {
    use crate::ui::widgets::MenuAction as M;
    vec![
        button("Next Page", |_, _, a, _| {
            matches!(a, Action::Menu(M::BookPage(1)))
        }),
        Act::Click,
        button("CONTINUE", |_, _, a, _| {
            matches!(a, Action::Menu(M::CloseBook))
        }),
        Act::Click,
    ]
}

/// CAM-3.5, 3.6b, 4.2: mission 1 from the island map, its briefing read,
/// its completion prepared (CAM-3.7's `INeedSomeHelp`: playing to the gate
/// is the headless test's, `crates/viewer/src/menus/campaign_tests.rs`), the
/// victory book with the debriefing, the island map with mission 2 open, and
/// mission 2 started from its eagle. The profile is not written (`--play`).
pub fn mission_end() -> Vec<Step> {
    use crate::menus::campaign::{unlocked, EndScreen};
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    vec![
        menu_step(
            "START CAMPAIGN opens the island map (UI-MENU-1.5, UI-MENU-1.7)",
            vec![
                button("START CAMPAIGN", |_, _, a, _| {
                    matches!(a, Action::Menu(M::Campaign))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::Campaign => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "the eagle on island I starts mission 1 (UI-MENU-1.7, CAM-4.2)",
            vec![
                button("mission 1's eagle", |_, _, a, _| {
                    matches!(a, Action::Menu(M::StartMission(1)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::None => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        step(
            "mission 1 runs: its target, triggers and locks (CAM-3.1, CAM-4.3, CAM-4.4)",
            vec![],
            100,
            Box::new(|v, _| {
                if v.mission.map != "campaign01" || v.mission.kind != 1 {
                    return Err(format!("the map is {} {}", v.mission.map, v.mission.kind));
                }
                let mission = v.world.mission().ok_or("no mission")?;
                if mission.target != s2_sim::mission::target::PORTAL
                    || mission.triggers.len() != 2
                    || mission.locks.len() != 10
                {
                    return Err(format!("the mission is {mission:?}"));
                }
                Ok(())
            }),
        ),
        step(
            "T opens the goal panel (UI-GAME-2.4, UI-GAME-12.1)",
            vec![Act::Key(KeyCode::KeyT, Key::Character("t".into()))],
            30,
            Box::new(|v, _| {
                if v.panels.iter().any(|p| p.id == PanelId::Briefing) {
                    Ok(())
                } else {
                    Err("the goal panel is not on screen".into())
                }
            }),
        ),
        step(
            "the briefing opens in the 26th second (CAM-1.2, CAM-4.4, T1)",
            vec![],
            400,
            Box::new(|v, _| book_is(v, "b01_A_00", false)),
        ),
        step(
            "the book is read and closed, and the game goes on (CAM-1.3)",
            book_through(),
            100,
            Box::new(|v, _| {
                if v.mission.book.is_some() {
                    Err("the book is still open".into())
                } else {
                    Ok(())
                }
            }),
        ),
        step(
            "the mission completed: VICTORY on the end screen, the map state kept (CAM-3.5, CAM-3.6)",
            vec![Act::Prepare(Box::new(|v| {
                Ok(Command::Cheat {
                    player: v.player,
                    cheat: s2_sim::mission::Cheat::INeedSomeHelp,
                })
            }))],
            100,
            Box::new(|v, _| {
                if v.mission.end != Some(EndScreen::Victory(4)) {
                    return Err(format!("the end screen is {:?}", v.mission.end));
                }
                if !v.panels.iter().any(|p| p.id == PanelId::Book) {
                    return Err("its panel is not on screen".into());
                }
                if !v.menu.settings.completed("campaign01") {
                    return Err("the map state is not completed".into());
                }
                Ok(())
            }),
        ),
        menu_step(
            "the victory book shows the debriefing, its next page (CAM-3.6b, CAM-1.3c, CAM-4.4)",
            vec![
                button("Next Page", |_, _, a, _| {
                    matches!(a, Action::Menu(M::BookPage(1)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                let Some(book) = &v.mission.book else {
                    return Err("no victory book".into());
                };
                if book.end != Some(EndScreen::Victory(4))
                    || book.pages.first().map(String::as_str) != Some("b01_C_00")
                    || book.page != 1
                {
                    return Err(format!("the book is {book:?}"));
                }
                Ok(())
            }),
        ),
        menu_step(
            "EXIT MAP leads to the island map with mission 2 open (CAM-3.6b, CAM-4.2, T3)",
            vec![
                button("EXIT MAP", |_, _, a, _| {
                    matches!(a, Action::Menu(M::EndExit))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                if v.menu.screen != Screen::Campaign {
                    return Err(format!("the screen is {:?}", v.menu.screen));
                }
                if !unlocked(&v.menu.settings, 2) {
                    return Err("mission 2 is not open".into());
                }
                Ok(())
            }),
        ),
        menu_step(
            "the eagle on island II starts mission 2 (UI-MENU-1.7, CAM-4.2)",
            vec![
                button("mission 2's eagle", |_, _, a, _| {
                    matches!(a, Action::Menu(M::StartMission(2)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::None => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        step(
            "mission 2 runs with its computer player and its briefing (CAM-4.3, CAM-4.4)",
            vec![],
            400,
            Box::new(|v, _| {
                if v.mission.map != "campaign02" {
                    return Err(format!("the map is {}", v.mission.map));
                }
                if !v.world.is_player(PlayerId(1)) {
                    return Err("no second player".into());
                }
                book_is(v, "b02_A_00", false)
            }),
        ),
    ]
}

/// CAM-3.2, 3.6: the headquarters gone on a prepared world: the defeat
/// screen, EXIT MAP to the main menu.
pub fn mission_defeat() -> Vec<Step> {
    use crate::menus::campaign::EndScreen;
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    vec![
        step(
            "mission 1 runs (CAM-4.3)",
            vec![],
            100,
            Box::new(|v, _| {
                if v.mission.map == "campaign01" {
                    Ok(())
                } else {
                    Err(format!("the map is {}", v.mission.map))
                }
            }),
        ),
        step(
            "the headquarters gone: DEFEAT on the end screen (CAM-3.2, CAM-3.6, T6)",
            vec![Act::Prepare(Box::new(|v| {
                Ok(Command::RemoveBuilding {
                    player: v.player,
                    at: v.headquarters()?,
                })
            }))],
            100,
            Box::new(|v, _| {
                if v.mission.end != Some(EndScreen::Defeat) {
                    return Err(format!("the end screen is {:?}", v.mission.end));
                }
                if !v.panels.iter().any(|p| p.id == PanelId::Book) {
                    return Err("its panel is not on screen".into());
                }
                Ok(())
            }),
        ),
        menu_step(
            "EXIT MAP leads to the main menu (CAM-3.6)",
            vec![
                button("EXIT MAP", |_, _, a, _| {
                    matches!(a, Action::Menu(M::EndExit))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::Main if v.mission.book.is_none() => Ok(()),
                s => Err(format!(
                    "the screen is {s:?}, the book {:?}",
                    v.mission.book
                )),
            }),
        ),
    ]
}

/// FMT-SAV-3.12, CAM-1.1, first half: mission 1 with its briefing read, saved
/// from the game menu. The script's end quits the program.
pub fn mission_save() -> Vec<Step> {
    let mut steps = vec![
        step(
            "the briefing of the mission to save opens (CAM-4.4)",
            vec![],
            400,
            Box::new(|v, _| book_is(v, "b01_A_00", false)),
        ),
        step(
            "the book is read and closed before the save (CAM-1.3)",
            book_through(),
            100,
            Box::new(|v, _| {
                if v.mission.book.is_none() && v.world.tick() >= 300 {
                    Ok(())
                } else {
                    Err(format!(
                        "the book {:?} at tick {}",
                        v.mission.book,
                        v.world.tick()
                    ))
                }
            }),
        ),
    ];
    // `save_game` without its wait and its autosave.
    let mut saving = save_game();
    saving.pop();
    steps.extend(saving.into_iter().skip(1));
    steps
}

/// FMT-SAV-3.12, CAM-1.1, second half: from a new start of the program, the
/// save loaded (its state hashes as saved): the script's variables back, the
/// goals as before, and the briefing it has read does not come again.
pub fn mission_load() -> Vec<Step> {
    let mut steps = load_game();
    steps.push(step(
        "the loaded mission is as it was: script, goal, triggers, locks (CAM-1.1, FMT-SAV-3.12, T8)",
        vec![],
        60,
        Box::new(|v, _| {
            if v.mission.map != "campaign01" || v.mission.kind != 1 {
                return Err(format!("the map is {} {}", v.mission.map, v.mission.kind));
            }
            if !v.mission.script_variables.iter().any(|(n, _)| n == "player0") {
                return Err(format!("the variables are {:?}", v.mission.script_variables));
            }
            if v.mission.debriefing != ["b01_C_00", "b01_C_01"] {
                return Err(format!("the debriefing is {:?}", v.mission.debriefing));
            }
            let mission = v.world.mission().ok_or("no mission")?;
            if mission.target != s2_sim::mission::target::PORTAL
                || mission.triggers.len() != 2
                || mission.locks.len() != 10
            {
                return Err(format!("the mission is {mission:?}"));
            }
            Ok(())
        }),
    ));
    steps.push(step(
        "the briefing read before the save does not come again (CAM-1.2, FMT-SAV-3.12)",
        vec![],
        400,
        Box::new(|v, _| {
            if v.mission.book.is_some() {
                return Err("a book opened".into());
            }
            let from = v.menu.last_load.as_ref().map_or(0, |c| c.tick);
            if v.world.tick() >= from + 300 {
                Ok(())
            } else {
                Err(format!("tick {} (loaded at {from})", v.world.tick()))
            }
        }),
    ));
    steps
}

/// CAM-3.5, SET-2.2, first half: mission 1 completed by CAM-3.7's command in a
/// program with a profile folder of its own (`--profile-dir`, never the
/// player's): the program writes the map state to its profile file.
pub fn campaign_state_write() -> Vec<Step> {
    use crate::menus::campaign::EndScreen;
    vec![
        step(
            "mission 1 runs, not yet completed (CAM-4.3, SET-2.2)",
            vec![],
            100,
            Box::new(|v, _| {
                if v.mission.map != "campaign01" {
                    return Err(format!("the map is {}", v.mission.map));
                }
                if v.menu.settings.completed("campaign01") {
                    return Err("the profile folder is not a new one".into());
                }
                Ok(())
            }),
        ),
        step(
            "the mission completed: the map state kept (CAM-3.5, CAM-3.7, SET-2.2)",
            vec![Act::Prepare(Box::new(|v| {
                Ok(Command::Cheat {
                    player: v.player,
                    cheat: s2_sim::mission::Cheat::INeedSomeHelp,
                })
            }))],
            100,
            Box::new(|v, _| {
                if v.mission.end != Some(EndScreen::Victory(4)) {
                    return Err(format!("the end screen is {:?}", v.mission.end));
                }
                if !v.menu.settings.completed("campaign01") {
                    return Err("the map state is not completed".into());
                }
                Ok(())
            }),
        ),
    ]
}

/// CAM-4.2, SET-2.2, second half: a new start of the program reads the profile
/// file the first half wrote: mission 2 is open, mission 3 is not.
pub fn campaign_state_read() -> Vec<Step> {
    use crate::menus::campaign::unlocked;
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    vec![
        menu_step(
            "the profile file is read back: campaign01 completed (SET-2.2, CAM-3.5)",
            vec![],
            120,
            Box::new(|v, _| {
                if v.menu.settings.completed("campaign01") {
                    Ok(())
                } else {
                    Err("the new start does not know the mission was completed".into())
                }
            }),
        ),
        menu_step(
            "START CAMPAIGN opens the island map (UI-MENU-1.5, UI-MENU-1.7)",
            vec![
                button("START CAMPAIGN", |_, _, a, _| {
                    matches!(a, Action::Menu(M::Campaign))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::Campaign => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "the eagle on island II is open, the one on III is not (CAM-4.2, T3, T9)",
            vec![
                button("mission 2's eagle", |_, _, a, _| {
                    matches!(a, Action::Menu(M::StartMission(2)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| {
                if !unlocked(&v.menu.settings, 2) || unlocked(&v.menu.settings, 3) {
                    return Err("missions 2 and 3 are not as the map state says".into());
                }
                match v.menu.screen {
                    Screen::None => Ok(()),
                    s => Err(format!("the screen is {s:?}")),
                }
            }),
        ),
        step(
            "mission 2 starts (CAM-4.3)",
            vec![],
            100,
            Box::new(|v, _| {
                if v.mission.map == "campaign02" {
                    Ok(())
                } else {
                    Err(format!("the map is {}", v.mission.map))
                }
            }),
        ),
    ]
}

/// The planks in the player's headquarters.
fn headquarters_planks(v: &PlayView) -> Result<u32, String> {
    let plank = v.good_id("plank")?;
    let stock = v
        .world
        .stock(v.headquarters()?)
        .ok_or("the headquarters has no stock")?;
    Ok(stock
        .iter()
        .find(|(good, _)| *good == plank)
        .map_or(0, |(_, n)| *n))
}

/// CAM-4.3, ECO-HQ-2.2: a free game on High (x 4), left for the main menu; then
/// mission 1, whose headquarters holds what setting 0 gives (Q-1002): nothing
/// carries over from the earlier game.
pub fn mission_after_free_game() -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    vec![
        step(
            "a free game on High: 88 planks in the headquarters (ECO-HQ-2.2)",
            vec![],
            100,
            Box::new(|v, _| match headquarters_planks(v)? {
                88 => Ok(()),
                n => Err(format!("{n} planks")),
            }),
        ),
        menu_step(
            "Escape opens the game menu (UI-MENU-3.1)",
            vec![Act::Key(KeyCode::Escape, Key::Escape)],
            60,
            Box::new(|v, _| match v.menu.screen {
                Screen::GameMenu => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "Exit to Title asks first (UI-MENU-3.3)",
            vec![
                button("Exit to Title", |_, _, a, _| {
                    matches!(a, Action::Menu(M::AskLeave(false)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::Leave { desktop: false, .. } => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "the Autosave... box goes on (UI-MENU-3.3, setting 22, SET-2)",
            vec![
                button("Autosave...", |_, _, a, _| {
                    matches!(a, Action::Menu(M::AutosaveOnLeave))
                }),
                Act::Click,
            ],
            60,
            Box::new(|v, _| {
                if v.menu.settings.autosave_on_leave {
                    Ok(())
                } else {
                    Err("the box is off".to_owned())
                }
            }),
        ),
        menu_step(
            "Yes leaves the game and writes the autosave, the interval off (UI-MENU-3.1, 3.3, FMT-SAV-5.3)",
            vec![
                button("Yes", |_, _, a, _| {
                    matches!(a, Action::Menu(M::ExitToTitle))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::Main => {
                    let folder = crate::menus::menu::save_folder().ok_or("no save folder")?;
                    let name = crate::menus::menu::autosave_name(v.menu);
                    let path = crate::game::save::path_of(&folder, &name);
                    if path.is_file() {
                        Ok(())
                    } else {
                        Err(format!("no {}", path.display()))
                    }
                }
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "SINGLE PLAYER opens its page on CAMPAIGN, the island map (UI-MENU-1.2, UI-MENU-1.5, UI-MENU-1.7)",
            vec![
                button("SINGLE PLAYER", |_, _, a, _| {
                    matches!(a, Action::Menu(M::SinglePlayer))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::Campaign => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        menu_step(
            "the eagle on island I starts mission 1 (UI-MENU-1.7, CAM-4.2)",
            vec![
                button("mission 1's eagle", |_, _, a, _| {
                    matches!(a, Action::Menu(M::StartMission(1)))
                }),
                Act::Click,
            ],
            120,
            Box::new(|v, _| match v.menu.screen {
                Screen::None => Ok(()),
                s => Err(format!("the screen is {s:?}")),
            }),
        ),
        step(
            "mission 1's headquarters holds 44 planks, not 88 (CAM-4.3, ECO-HQ-2.2)",
            vec![],
            100,
            Box::new(|v, _| {
                if v.mission.map != "campaign01" {
                    return Err(format!("the map is {}", v.mission.map));
                }
                match headquarters_planks(v)? {
                    44 => Ok(()),
                    n => Err(format!("{n} planks")),
                }
            }),
        ),
    ]
}

/// The frames allowed while the camera pans at full speed over the late game
/// of `panning` (F55), on this PC, held to the screen's refresh (16.7 ms): the
/// median shows every object placed again every tick (25.6 ms before that was
/// fixed); the others show the spikes, road meshes built again for the whole map
/// a few times a second. A frame that just misses the refresh takes 33.3 ms, so
/// the spikes are counted above that. Each step judges all the frames of the pan
/// so far: taken alone, a leg over a busy part of the map runs higher.
const PAN_MEDIAN_MS: f64 = 20.0;
const PAN_P99_MS: f64 = 50.0;
const PAN_WORST_MS: f64 = 150.0;
const PAN_SPIKE_MS: f64 = 50.0;
const PAN_SPIKES_SHARE: f64 = 0.01;

/// GFX-VIEW-3.2 (F55): the arrow keys held, the camera crossing the map at full
/// speed: the frames stay short and the camera moves. A late game of computer
/// players at speed 3 (`crates/viewer/tests/play.rs`).
pub fn panning() -> Vec<Step> {
    let pan = |name, code, key| {
        step(
            name,
            vec![Act::Hold(code, key, 400)],
            10,
            Box::new(|v: &PlayView, _: &Named| {
                let mut frames = v.held_frames.to_vec();
                if frames.len() < 300 {
                    return Err(format!("{} frames were timed", frames.len()));
                }
                frames.sort_by(f64::total_cmp);
                let at = |q: f64| frames[((frames.len() - 1) as f64 * q).round() as usize];
                let (p99, worst) = (at(0.99), at(1.0));
                let spikes = frames.iter().filter(|&&ms| ms > PAN_SPIKE_MS).count();
                let share = spikes as f64 / frames.len() as f64;
                info!(
                    "play: {} frames panning: median {:.1} ms, p99 {p99:.1} ms, worst {worst:.1} ms, \
                     {spikes} over {PAN_SPIKE_MS} ms",
                    frames.len(),
                    at(0.5)
                );
                if at(0.5) > PAN_MEDIAN_MS
                    || p99 > PAN_P99_MS
                    || worst > PAN_WORST_MS
                    || share > PAN_SPIKES_SHARE
                {
                    return Err(format!(
                        "panning is slow: median {:.1} ms (limit {PAN_MEDIAN_MS}), p99 {p99:.1} ms \
                         (limit {PAN_P99_MS}), worst {worst:.1} ms (limit {PAN_WORST_MS}), {spikes} of \
                         {} frames over {PAN_SPIKE_MS} ms (limit {:.0} %)",
                        at(0.5),
                        frames.len(),
                        PAN_SPIKES_SHARE * 100.0
                    ));
                }
                Ok(())
            }),
        )
    };
    vec![
        pan(
            "panning right at full speed stays smooth (GFX-VIEW-3.2)",
            KeyCode::ArrowRight,
            Key::ArrowRight,
        ),
        pan(
            "panning down at full speed stays smooth (GFX-VIEW-3.2)",
            KeyCode::ArrowDown,
            Key::ArrowDown,
        ),
        pan(
            "panning left at full speed stays smooth (GFX-VIEW-3.2)",
            KeyCode::ArrowLeft,
            Key::ArrowLeft,
        ),
    ]
}

/// UI-MENU T17 (3.3a c): on the main menu without a game, `!EXIT` asks first:
/// the leave dialog in its desktop mode over the main menu, `!No` keeps the main
/// menu, and `!EXIT` while the dialog is open closes it. (`!Yes` would end the
/// program: the harness ends it itself.)
pub fn main_exit() -> Vec<Step> {
    use crate::menus::menu::Screen;
    use crate::ui::widgets::MenuAction as M;
    let exit = || button("EXIT", |_, _, a, _| matches!(a, Action::Menu(M::ExitAsked)));
    let asking = || -> Check {
        Box::new(|v, _| match v.menu.screen {
            Screen::Leave {
                desktop: true,
                over_main: true,
            } => Ok(()),
            s => Err(format!("the screen is {s:?}")),
        })
    };
    let main = || -> Check {
        Box::new(|v, _| match v.menu.screen {
            Screen::Main => Ok(()),
            s => Err(format!("the screen is {s:?}")),
        })
    };
    vec![
        menu_step(
            "EXIT on the main menu asks first (UI-MENU-3.3a)",
            vec![exit(), Act::Click],
            60,
            asking(),
        ),
        menu_step(
            "No closes the question and the main menu stays (UI-MENU-3.3a)",
            vec![
                button("No", |_, _, a, _| matches!(a, Action::Menu(M::StayInGame))),
                Act::Click,
            ],
            60,
            main(),
        ),
        menu_step(
            "EXIT asks again (UI-MENU-3.3a)",
            vec![exit(), Act::Click],
            60,
            asking(),
        ),
        menu_step(
            "EXIT while the question is open closes it (UI-MENU-3.3a)",
            vec![exit(), Act::Click],
            60,
            main(),
        ),
    ]
}

/// The scripts by name.
pub fn script(name: &str) -> Option<Vec<Step>> {
    match name {
        "main-exit" => Some(main_exit()),
        "panning" => Some(panning()),
        "campaign-state-write" => Some(campaign_state_write()),
        "campaign-state-read" => Some(campaign_state_read()),
        "mission-after-free-game" => Some(mission_after_free_game()),
        "tutorial-one" => Some(tutorial_one()),
        "tutorial-two" => Some(tutorial_two()),
        "tutorial-three" => Some(tutorial_three()),
        "tutorial-four" => Some(tutorial_four()),
        "mission-end" => Some(mission_end()),
        "mission-defeat" => Some(mission_defeat()),
        "mission-save" => Some(mission_save()),
        "mission-load" => Some(mission_load()),
        "first-ten-minutes" => Some(first_ten_minutes()),
        "select-map" => Some(select_map()),
        "save-game" => Some(save_game()),
        "load-game" => Some(load_game()),
        "continue-game" => Some(continue_game()),
        _ => None,
    }
}
