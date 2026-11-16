//! Test cases of `specs/ECO-building-placement.md` ECO-PLACE-6, the
//! military clearance.

use crate::military_tests::*;
use crate::test_support::*;
use crate::{BuildingSize, Command, Rejection, World};

fn place(
    world: &mut World,
    player: crate::PlayerId,
    building: u32,
    at: crate::grid::Point,
) -> Result<(), Rejection> {
    apply(
        world,
        Command::PlaceBuilding {
            player,
            building,
            at,
        },
    )
}

/// Player 0 owns every field; his headquarters stands on (20, 20).
fn hq_world() -> World {
    let mut world = owned_world(80, 50);
    finished(&mut world, P0, CASTLE, (20, 20));
    world
}

// T18: the headquarters keeps the military types four fields off; at five
// they fit. The types without a territory range (a hut, the catapult) are
// not held back.
// spec: ECO-PLACE-6.1, ECO-PLACE-6.2, ECO-PLACE-6.3 (T18)
#[test]
fn t18_military_types_keep_four_fields_off_the_headquarters() {
    for at in [(24, 20), (20, 24)] {
        let world = hq_world();
        assert!(world.military_near(at), "{at:?}");
        for kind in [BARRACKS, GUARD, TOWER, FORTRESS] {
            let mut world = hq_world();
            assert_eq!(
                place(&mut world, P0, kind, at),
                Err(Rejection::TooCloseToMilitary),
                "{kind} on {at:?}"
            );
            assert!(world.building_at(at).is_none());
        }
        for kind in [HUT, CATAPULT] {
            let mut world = hq_world();
            assert_eq!(place(&mut world, P0, kind, at), Ok(()), "{kind} on {at:?}");
        }
    }
    for at in [(25, 20), (20, 25)] {
        let mut world = hq_world();
        assert!(!world.military_near(at), "{at:?}");
        assert_eq!(place(&mut world, P0, BARRACKS, at), Ok(()), "{at:?}");
        assert!(world.site(at).is_some());
    }
}

// T19: another player's construction site counts as soon as it is placed;
// a building without a territory range does not.
// spec: ECO-PLACE-6.2 (T19)
#[test]
fn t19_an_enemy_site_counts_a_building_without_range_does_not() {
    let mut world = hq_world();
    world.put_building(P1, GUARD, BuildingSize::Medium, (40, 20), true);
    assert!(world.building_at((40, 20)).unwrap().under_construction);
    assert!(world.military_near((36, 20)));
    assert_eq!(
        place(&mut world, P0, BARRACKS, (36, 20)),
        Err(Rejection::TooCloseToMilitary)
    );
    assert!(!world.military_near((35, 20)));
    finished(&mut world, P0, HUT, (60, 20));
    assert!(!world.military_near((62, 20)));
    assert_eq!(place(&mut world, P0, GUARD, (62, 20)), Ok(()));
}

// ECO-PLACE-6.3: the clearance comes first, before the other placement
// rules and before the mission's lock (a locked type that is too close is
// refused as too close); only each building's own field counts, and a
// field outside the map is skipped.
// spec: ECO-PLACE-6.2, ECO-PLACE-6.3
#[test]
fn the_clearance_is_checked_first_on_own_fields_only() {
    let mut world = hq_world();
    assert_eq!(
        apply(
            &mut world,
            Command::LockBuilding {
                player: P0,
                building: BARRACKS
            }
        ),
        Ok(())
    );
    assert_eq!(
        place(&mut world, P0, BARRACKS, (24, 20)),
        Err(Rejection::TooCloseToMilitary)
    );
    assert_eq!(
        place(&mut world, P0, BARRACKS, (30, 20)),
        Err(Rejection::BuildingLocked)
    );
    // On the headquarters' own field: too close, not "too close to a
    // building".
    assert_eq!(
        place(&mut world, P0, GUARD, (20, 20)),
        Err(Rejection::TooCloseToMilitary)
    );
    // The castle is large: its other footprint fields do not count, so a
    // field four from one of them but five from its own field is clear.
    let footprint: Vec<_> = world
        .footprint_of((20, 20), BuildingSize::Large)
        .into_iter()
        .filter(|&f| f != (20, 20))
        .collect();
    assert!(!footprint.is_empty());
    let far = (0..80u32)
        .flat_map(|x| (0..50u32).map(move |y| (x, y)))
        .find(|&f| {
            crate::grid::distance(f, (20, 20)) == 5
                && footprint.iter().any(|&g| crate::grid::distance(f, g) <= 4)
        })
        .expect("a field five from the castle and four from its footprint");
    assert!(!world.military_near(far), "{far:?}");
    // Near the map's edge the missing fields are skipped, not refused.
    assert!(!world.military_near((0, 0)));
}
