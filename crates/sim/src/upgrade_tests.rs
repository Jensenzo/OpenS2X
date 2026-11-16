//! Test cases of `specs/MIL-building-upgrade.md`.

use crate::grid::Point;
use crate::map::state;
use crate::military_tests::*;
use crate::test_support::*;
use crate::{BuildingSize, Command, GameSetup, Rejection, Rules, World};

/// The military rules with the upgrade chain and a plank and a stone for
/// each step.
fn upgrade_rules() -> Rules {
    let mut rules = rules();
    rules.plank = Some(PLANK);
    for kind in &mut rules.building_types {
        let next = match kind.id {
            BARRACKS => Some(GUARD),
            GUARD => Some(TOWER),
            TOWER => Some(FORTRESS),
            _ => None,
        };
        kind.upgrade_to = next;
        if next.is_some() || kind.id == FORTRESS {
            kind.frame_costs = vec![(PLANK, 1)];
            kind.costs = vec![(STONE, 1)];
        }
    }
    rules
}

fn upgrade_game() -> World {
    let setup = GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        ..Default::default()
    };
    World::new_game(5, flat(80, 50), upgrade_rules(), &setup)
}

fn upgrade(world: &mut World, at: Point) -> Result<(), Rejection> {
    apply(world, Command::UpgradeBuilding { player: P0, at })
}

// T1: the order costs nothing, the soldiers stay, the type changes at the
// end and the claim runs again with the new range.
// spec: MIL-UPG-1.3, MIL-UPG-2.1, MIL-UPG-2.4, MIL-UPG-3.1, TER-2.3 (T1)
#[test]
fn t1_upgrade_a_barracks() {
    let mut world = upgrade_game();
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    assert_eq!(world.field_owner((36, 20)), None);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(world.upgrading(at), Some(GUARD));
    // Still a barracks, with a construction of the guard house beside it.
    assert_eq!(world.building_at(at).unwrap().building_type, BARRACKS);
    assert_eq!(
        world.site(at).unwrap().shortfall(),
        vec![(PLANK, 1), (STONE, 1)]
    );
    // MIL-UPG-1.2: not twice.
    assert_eq!(upgrade(&mut world, at), Err(Rejection::NotUpgradable));
    let shortfall = world.site(at).unwrap().shortfall();
    let outcome = apply(
        &mut world,
        Command::DebugSupplySite {
            player: P0,
            at,
            wares: shortfall,
            workers: true,
        },
    );
    assert_eq!(outcome, Ok(()));
    let mut done = false;
    for _ in 0..4000 {
        world.step();
        // MIL-UPG-3.1: the soldier stays throughout.
        assert_eq!(world.garrison(at).len(), 1);
        if world.upgrading(at).is_none() {
            done = true;
            break;
        }
    }
    assert!(done, "the upgrade finished");
    assert!(world.site(at).is_none());
    assert_eq!(world.building_at(at).unwrap().building_type, GUARD);
    assert!(!world.building_at(at).unwrap().under_construction);
    // MIL-UPG-2.4: the guard house's range of 9.
    assert_eq!(world.field_owner((36, 20)), Some(P0));
    assert!(world.is_explored(P0, (39, 20)));
    // And it goes on to the next step.
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(world.upgrading(at), Some(TOWER));
}

// TOOL-trace TRACE-3.5a: an upgrading building's trace row keeps its old
// type, finished and 1000 for the whole upgrade (levelling and building of
// the upgrade are not shown); the new type from the tick the type changes.
// spec: TRACE-3.5a, MIL-UPG-2.4
#[test]
fn an_upgrading_buildings_trace_row_stays_finished() {
    let mut world = upgrade_game();
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    let row = |world: &World| {
        let r = world
            .trace_buildings()
            .into_iter()
            .find(|r| r.field == at)
            .expect("its row");
        (r.building_type, r.state, r.progress)
    };
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(row(&world), (BARRACKS, "finished", 1000));
    let shortfall = world.site(at).unwrap().shortfall();
    let outcome = apply(
        &mut world,
        Command::DebugSupplySite {
            player: P0,
            at,
            wares: shortfall,
            workers: true,
        },
    );
    assert_eq!(outcome, Ok(()));
    let mut ticks = 0;
    while world.upgrading(at).is_some() {
        assert_eq!(row(&world), (BARRACKS, "finished", 1000));
        world.step();
        ticks += 1;
        assert!(ticks < 4000, "the upgrade finished");
    }
    assert_eq!(row(&world), (GUARD, "finished", 1000));
}

// MIL-UPG-2.2, ECO-HQ-4.8a (MIL-UPG T6): the start of an upgrade sets the
// building field's height again as a site's placement does: with its
// neighbours not all at its height, the integer mean of the 19 fields
// within 2 (one neighbour raised by 1900: the mean rises by 100).
// spec: MIL-UPG-2.2 (T6), ECO-HQ-4.8a
#[test]
fn t6_the_upgrade_start_sets_the_site_height() {
    let mut world = upgrade_game();
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    let base = world.terrain().height_at(at);
    let neighbour = world.terrain().neighbours(at)[0].unwrap();
    set_height(&mut world.terrain, neighbour, base + 1900);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(world.terrain().height_at(at), base + 100);
}

// T2: not finished, no next type, no room.
// spec: MIL-UPG-1.2 (T2)
#[test]
fn t2_refused() {
    let mut world = upgrade_game();
    // Under construction.
    let site = (27, 20);
    assert_eq!(
        apply(
            &mut world,
            Command::PlaceBuilding {
                player: P0,
                building: BARRACKS,
                at: site
            }
        ),
        Ok(())
    );
    assert!(upgrade(&mut world, site).is_err());
    // A fortress has no next type.
    let fortress = (27, 26);
    finished(&mut world, P0, FORTRESS, fortress);
    assert_eq!(upgrade(&mut world, fortress), Err(Rejection::NotUpgradable));
    // A tower whose fortress footprint is taken by a hut two fields away.
    let tower = (33, 20);
    finished(&mut world, P0, TOWER, tower);
    finished(&mut world, P0, HUT, (35, 20));
    assert_eq!(upgrade(&mut world, tower), Err(Rejection::NoSpaceToUpgrade));
    // Another player's building, or none.
    assert_eq!(upgrade(&mut world, (50, 40)), Err(Rejection::NoBuilding));
}

// The bigger footprint is laid out at the start (MIL-UPG-2.1).
#[test]
fn a_bigger_type_takes_its_footprint_at_once() {
    let mut world = upgrade_game();
    let at = (27, 20);
    occupied(&mut world, P0, TOWER, at);
    let west = world.terrain().neighbour(at, 3).unwrap();
    assert!(!world.terrain().has(west, state::MOVEMENT_BLOCKED));
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(world.building_at(at).unwrap().size, BuildingSize::Large);
    assert!(world.terrain().has(west, state::MOVEMENT_BLOCKED));
}

// T3: uneven ground is levelled first.
// spec: MIL-UPG-2.2 (T3)
#[test]
fn t3_levelling_first() {
    let mut world = upgrade_game();
    let at = (27, 20);
    occupied(&mut world, P0, TOWER, at);
    let neighbour = world.terrain().neighbour(at, 0).unwrap();
    world.set_field_height(neighbour, 5);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    let site = world.site(at).unwrap();
    assert!(site.levelling_needed && !site.levelled());
    // Flat ground: nothing to level.
    let mut flat_world = upgrade_game();
    occupied(&mut flat_world, P0, TOWER, at);
    assert_eq!(upgrade(&mut flat_world, at), Ok(()));
    assert!(flat_world.site(at).unwrap().levelled());
}

// MIL-UPG T9 (MIL-UPG-2.2a): an upgrade's update does one part per tick: on
// level ground its first update after the order only marks the levelling
// complete; the material orders and the builder request come in the next.
// spec: MIL-UPG-2.2a (MIL-UPG T9)
#[test]
fn t9_an_upgrade_on_level_ground_files_in_its_second_update() {
    let mut rules = upgrade_rules();
    rules.roles.builder = Some(WOODCUTTER);
    rules.start_stock.extend([(PLANK, 4), (STONE, 4)]);
    let setup = GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(5, flat(80, 50), rules, &setup);
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    road_between(&mut world, (20, 20), at);
    let filed = |w: &World| {
        let orders = w.orders().iter().filter(|o| o.target.at == at).count();
        let builder = w.requests(P0).iter().any(|r| r.profession == WOODCUTTER);
        (orders, builder)
    };
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert!(world.site(at).unwrap().levelled(), "levelling marked");
    assert_eq!(filed(&world), (0, false), "nothing filed yet");
    world.step();
    let (orders, builder) = filed(&world);
    assert!(orders > 0 && builder, "{orders} orders, builder {builder}");
}

// Removing an upgrading building takes the upgrade and its site with it.
// spec: MIL-UPG-2.1, TER-4.1
#[test]
fn removing_an_upgrading_building_clears_the_upgrade() {
    let mut world = upgrade_game();
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    let outcome = apply(&mut world, Command::RemoveBuilding { player: P0, at });
    assert_eq!(outcome, Ok(()));
    assert_eq!(world.upgrading(at), None);
    assert!(world.site(at).is_none());
}

// MIL-UPG-2.3: the materials and the builder come from the warehouse like for
// any site, over the roads.
#[test]
fn materials_and_builder_come_from_the_warehouse() {
    let mut rules = upgrade_rules();
    rules.roles.builder = Some(WOODCUTTER);
    rules
        .start_stock
        .extend([(WOODCUTTER, 2), (PLANK, 4), (STONE, 4)]);
    let setup = GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(5, flat(80, 50), rules, &setup);
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    road_between(&mut world, (20, 20), at);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    let mut done = false;
    for _ in 0..12_000 {
        world.step();
        if world.upgrading(at).is_none() {
            done = true;
            break;
        }
    }
    assert!(done, "the upgrade finished from the warehouse's stock");
    assert_eq!(world.building_at(at).unwrap().building_type, GUARD);
    assert_eq!(world.garrison(at).len(), 1);
}

// MIL T18 (MIL-SOL-4.2, 4.4 step 1, ECO-REQ-2.3a): a barracks with its one
// soldier present and no enemy (wanted 1) is upgraded on flat ground; with
// no road its site's builder request stays open and counts in R, so the
// allocation pass of tick 20 finds d = 1 - 1 - 1 = -1 and cancels it: it
// stays in the queue, unlinked. In tick 21 the upgrade files a builder
// request again (a new number, ECO-REQ-1.3) and the queue deletes the
// unlinked one on its turn. The soldier stays.
// spec: MIL-SOL-4.2, MIL-SOL-4.4, ECO-REQ-2.3a, ECO-REQ-1.3 (MIL T18)
#[test]
fn mil_t18_the_allocation_cancels_the_upgrades_builder_request() {
    use crate::Requester;
    let mut rules = upgrade_rules();
    rules.roles.builder = Some(WOODCUTTER);
    let setup = GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(5, flat(80, 50), rules, &setup);
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    let site = Requester::Site(world.target_of(at).unwrap());
    let count = |world: &World, requester: Requester| {
        world
            .requests(P0)
            .iter()
            .filter(|r| r.requester == requester)
            .count()
    };
    run_to(&mut world, 20);
    assert_eq!(world.wanted_soldiers(at), 1);
    assert_eq!(count(&world, site), 1, "the builder request is open");
    let next = world.next_object();
    // The pass of tick 20.
    world.step();
    assert_eq!(count(&world, site), 0, "cancelled");
    assert_eq!(count(&world, Requester::Unlinked), 1, "still in the queue");
    assert_eq!(world.next_object(), next, "no soldier request filed");
    world.step();
    assert_eq!(count(&world, site), 1, "filed again");
    assert_eq!(count(&world, Requester::Unlinked), 0, "deleted on its turn");
    assert_eq!(world.next_object(), next + 1, "with a new number");
    assert_eq!(world.garrison(at).len(), 1);
}

// MIL-UPG T7 (MIL-UPG-2.3a): the builder of an upgrade works at the four
// construction positions of the building's current type (here the
// barracks'), offsets from its flag field's sample, not at the target
// type's.
// spec: MIL-UPG-2.3a (MIL-UPG T7), TRN-FREE-5.5a
#[test]
fn t7_the_upgrades_builder_uses_the_current_types_positions() {
    let mut rules = upgrade_rules();
    for kind in &mut rules.building_types {
        if kind.id == BARRACKS {
            kind.construction_positions = [[(-1, 2), (-2, 1), (1, 2), (0, 2)]; 3];
        } else if kind.id == GUARD {
            kind.construction_positions = [[(-1, 2), (1, 2), (-2, 0), (-1, 1)]; 3];
        }
    }
    let setup = GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(5, flat(80, 50), rules, &setup);
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    let before = world.construction_samples(at);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(world.upgrading(at), Some(GUARD));
    let flag = world.terrain().neighbour(at, 1).unwrap();
    let (c, r) = crate::grid::field_sample(flag);
    let expected = [(-1, 2), (-2, 1), (1, 2), (0, 2)]
        .map(|(x, y): (i32, i32)| ((c as i32 + x) as u32, (r as i32 + y) as u32));
    assert_eq!(world.construction_samples(at), expected);
    assert_eq!(world.construction_samples(at), before);
}

// MIL-UPG T8 (MIL-UPG-2.4, 1.2, Q-367): the upgrade's construction is worked
// off with its builder still in his last step's action; the type changes in
// the building's update of the tick after his release, not with the
// completing decrement. A building just finished cannot be upgraded until
// its builder is released.
// spec: MIL-UPG-2.4 (MIL-UPG T8), MIL-UPG-1.2, ECO-HQ-4.7
#[test]
fn t8_the_type_changes_after_the_builders_release() {
    let mut world = upgrade_game();
    let at = (27, 20);
    finished(&mut world, P0, BARRACKS, at);
    garrison(&mut world, at, 0, 3);
    assert_eq!(upgrade(&mut world, at), Ok(()));
    let shortfall = world.site(at).unwrap().shortfall();
    let supply = Command::DebugSupplySite {
        player: P0,
        at,
        wares: shortfall,
        workers: true,
    };
    assert_eq!(apply(&mut world, supply), Ok(()));
    let mut completed = None;
    let mut changed = None;
    for _ in 0..4000 {
        world.step();
        let t = world.tick();
        if completed.is_none() && world.site(at).is_none() {
            completed = Some(t);
            // Done, but not yet the new type: its builder works on.
            assert_eq!(world.building_at(at).unwrap().building_type, BARRACKS);
            assert_eq!(world.upgrading(at), Some(GUARD));
        }
        if world.building_at(at).unwrap().building_type == GUARD {
            changed = Some(t);
            break;
        }
    }
    let (completed, changed) = (completed.unwrap(), changed.unwrap());
    assert!(changed > completed, "{completed} -> {changed}");
    assert!(changed <= completed + 56, "within his last action");
    // MIL-UPG-1.2: a building whose finishing builder is still linked to
    // its construction (until his release) is not upgradable.
    let other = (23, 25);
    finished(&mut world, P0, BARRACKS, other);
    assert_eq!(world.upgrade_target(P0, other), Ok(GUARD));
    let release = world.tick() + 3;
    world.builder_links.insert(other, release);
    assert_eq!(
        world.upgrade_target(P0, other),
        Err(Rejection::NotUpgradable)
    );
    while world.tick() <= release {
        world.step();
    }
    assert_eq!(world.upgrade_target(P0, other), Ok(GUARD));
}

// ECO-PLACE T20: an upgrade does not run the military clearance: a barracks
// five fields from the headquarters (its own field at distance 0) is
// upgraded.
// spec: ECO-PLACE-6.3 (T20)
#[test]
fn t20_an_upgrade_skips_the_military_clearance() {
    let mut world = upgrade_game();
    let at = (25, 20);
    occupied(&mut world, P0, BARRACKS, at);
    assert!(world.military_near(at));
    assert_eq!(upgrade(&mut world, at), Ok(()));
    assert_eq!(world.upgrading(at), Some(GUARD));
}
