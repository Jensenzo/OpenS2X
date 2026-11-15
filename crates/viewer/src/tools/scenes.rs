//! `--scene`: set-ups for screenshots, made only of player commands.

use s2_sim::grid::{self, Point};
use s2_sim::map::state;
use s2_sim::{Command, PlayerId, World};

/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;

/// Apply `commands` in one step; true if all were carried out.
fn apply(world: &mut World, commands: Vec<Command>) -> bool {
    for c in commands {
        world.submit(c);
    }
    world.step();
    world.last_results().iter().all(|r| r.outcome.is_ok())
}

/// The flag of the player's first warehouse (the headquarters).
fn home_flag(world: &World, player: PlayerId) -> Option<Point> {
    let (at, _) = world
        .warehouses()
        .find(|(at, _)| world.building_at(*at).is_some_and(|b| b.owner == player))?;
    world.terrain().neighbour(at, FLAG_DIRECTION)
}

/// The player's own fields within `radius` of `center`, nearest first.
fn own_fields(world: &World, player: PlayerId, center: Point, radius: u32) -> Vec<Point> {
    let t = world.terrain();
    let mut fields = grid::area(center, radius, t.width, t.height);
    fields.retain(|&f| world.field_owner(f) == Some(player));
    fields.sort_by_key(|&f| (grid::distance(center, f), t.index(f)));
    fields
}

/// A flag 3 to 6 fields from the headquarters' flag joined to it by road,
/// and a geologist ordered there; the flag.
pub fn geologist(world: &mut World, player: PlayerId) -> Option<Point> {
    let home = home_flag(world, player)?;
    for at in own_fields(world, player, home, 6) {
        if grid::distance(home, at) < 3 {
            continue;
        }
        let mut trial = world.clone();
        let placed = apply(&mut trial, vec![Command::PlaceFlag { player, at }])
            && apply(
                &mut trial,
                vec![Command::BuildRoad {
                    player,
                    from: home,
                    to: at,
                }],
            )
            && apply(&mut trial, vec![Command::OrderGeologist { player, at }]);
        if placed {
            *world = trial;
            return Some(at);
        }
    }
    None
}

/// A Coast flag joined to the headquarters by road and a waterway from it
/// over the water to another Coast flag of the player; the waterway's
/// middle field.
pub fn waterway(world: &mut World, player: PlayerId) -> Option<Point> {
    let home = home_flag(world, player)?;
    let coast = |w: &World, f: Point| w.terrain().has(f, state::COAST);
    let starts: Vec<Point> = own_fields(world, player, home, 14)
        .into_iter()
        .filter(|&f| coast(world, f))
        .take(40)
        .collect();
    for a in starts {
        let mut trial = world.clone();
        let joined = (trial.flag_at(a) == Some(player)
            || apply(&mut trial, vec![Command::PlaceFlag { player, at: a }]))
            && apply(
                &mut trial,
                vec![Command::BuildRoad {
                    player,
                    from: home,
                    to: a,
                }],
            );
        if !joined {
            continue;
        }
        let ends: Vec<Point> = own_fields(&trial, player, a, 14)
            .into_iter()
            .filter(|&b| grid::distance(a, b) >= 4 && coast(&trial, b))
            .collect();
        for b in ends {
            let Ok((path, _)) = trial.find_waterway(a, b) else {
                continue;
            };
            let mut built = trial.clone();
            if apply(&mut built, vec![Command::PlaceFlag { player, at: b }])
                && apply(
                    &mut built,
                    vec![Command::BuildWaterway {
                        player,
                        from: a,
                        to: b,
                    }],
                )
            {
                *world = built;
                return Some(path[path.len() / 2]);
            }
        }
    }
    None
}

/// DEBUG ONLY: a launched ship of the player on the nearest water field
/// with water all around it, within 14 fields of the headquarters' flag
/// (the ship window, UI-GAME-4.5); its field.
pub fn ship(world: &mut World, player: PlayerId) -> Option<Point> {
    let home = home_flag(world, player)?;
    let t = world.terrain();
    let mut fields = grid::area(home, 14, t.width, t.height);
    fields.sort_by_key(|&f| (grid::distance(home, f), t.index(f)));
    let open_water = |f: Point| {
        t.has(f, state::WATER)
            && t.neighbours(f)
                .into_iter()
                .all(|n| n.is_some_and(|n| t.has(n, state::WATER)))
    };
    let at = fields.into_iter().find(|&f| open_water(f))?;
    let i = world.debug_add_ship(player, at);
    world.debug_launch_ship(i);
    Some(at)
}

/// A building of type `building` on the nearest spot of the player's land
/// within 8 fields of the headquarters' flag that takes it, joined to that
/// flag by road; its field.
pub fn building(world: &mut World, player: PlayerId, building: u32) -> Option<Point> {
    let home = home_flag(world, player)?;
    for at in own_fields(world, player, home, 8) {
        if grid::distance(home, at) < 3 {
            continue;
        }
        let Some(flag) = world.terrain().neighbour(at, FLAG_DIRECTION) else {
            continue;
        };
        let mut trial = world.clone();
        let placed = apply(
            &mut trial,
            vec![Command::PlaceBuilding {
                player,
                building,
                at,
            }],
        ) && apply(
            &mut trial,
            vec![Command::BuildRoad {
                player,
                from: home,
                to: flag,
            }],
        );
        if placed {
            *world = trial;
            return Some(at);
        }
    }
    None
}

/// A finished military building of type `building` by the headquarters
/// (as `building`, but made with `CreateBuilding`), the game then played on
/// until `soldiers` are inside (at most 6000 ticks); with `coins_off` its
/// coin supply is then switched off. For the soldier flag and the status
/// icons (GFX-OBJ-8.1, 8.3). Its field.
pub fn military(
    world: &mut World,
    player: PlayerId,
    building: u32,
    soldiers: usize,
    coins_off: bool,
) -> Option<Point> {
    let home = home_flag(world, player)?;
    // MIL-SOL-7.1: every distance class asks for its full complement.
    let full = (0..3)
        .map(|class| Command::MilitaryAllocation {
            player,
            class,
            thousandths: 1000,
        })
        .collect();
    apply(world, full);
    let mut found = None;
    for at in own_fields(world, player, home, 8) {
        if grid::distance(home, at) < 3 {
            continue;
        }
        let Some(flag) = world.terrain().neighbour(at, FLAG_DIRECTION) else {
            continue;
        };
        let mut trial = world.clone();
        let placed = apply(
            &mut trial,
            vec![Command::CreateBuilding {
                player,
                building,
                at,
                settler: None,
            }],
        ) && apply(
            &mut trial,
            vec![Command::BuildRoad {
                player,
                from: home,
                to: flag,
            }],
        );
        if placed {
            *world = trial;
            found = Some(at);
            break;
        }
    }
    let at = found?;
    let inside = |w: &World| w.garrison(at).iter().filter(|s| s.present).count();
    let until = world.tick() + 6000;
    while inside(world) < soldiers && world.tick() < until {
        world.step();
    }
    if coins_off {
        apply(world, vec![Command::ToggleCoinSupply { player, at }]);
    }
    Some(at)
}
