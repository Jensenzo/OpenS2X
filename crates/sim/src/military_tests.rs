//! Tests of `specs/TER-territory.md`, `specs/MIL-soldiers.md` and
//! `specs/MIL-coins-promotion.md`, on a small synthetic rule set.

use crate::grid::{self, Point};
use crate::map::state;
use crate::movement::Walker;
use crate::test_support::*;
use crate::{
    BuildingSize, BuildingType, Command, GameSetup, GoodKind, GoodRules, MilitaryRules, PlayerId,
    Rules, Settler, SettlerJob, World,
};

pub(crate) const CASTLE: u32 = 100;
pub(crate) const BARRACKS: u32 = 101;
pub(crate) const GUARD: u32 = 102;
pub(crate) const TOWER: u32 = 103;
pub(crate) const FORTRESS: u32 = 104;
pub(crate) const HUT: u32 = 105;
/// The five soldier professions.
pub(crate) const RANK: [u32; 5] = [50, 51, 52, 53, 54];
pub(crate) const HELPER: u32 = 60;
pub(crate) const SWORD: u32 = 61;
pub(crate) const SHIELD: u32 = 62;
pub(crate) const BEER: u32 = 63;
pub(crate) const COIN: u32 = 64;
pub(crate) const STONE: u32 = 65;
pub(crate) const WOODCUTTER: u32 = 66;
pub(crate) const PLANK: u32 = 67;
pub(crate) const CATAPULT: u32 = 106;
pub(crate) const P1: PlayerId = PlayerId(1);
pub(crate) const P2: PlayerId = PlayerId(2);

pub(crate) fn rules() -> Rules {
    let building = |id, size, range, radius, limit| {
        let mut t = BuildingType::new(id, size);
        t.territory_range = range;
        t.explore_radius = radius;
        t.soldier_limit = limit;
        t.worker_limit = 0;
        t
    };
    let mut castle = building(CASTLE, BuildingSize::Large, 9, 12, 0);
    castle.warehouse = true;
    let mut barracks = building(BARRACKS, BuildingSize::Medium, 8, 11, 2);
    barracks.stock_limit = Some(2);
    let mut guard = building(GUARD, BuildingSize::Medium, 9, 12, 3);
    guard.stock_limit = Some(2);
    let mut tower = building(TOWER, BuildingSize::Medium, 10, 13, 6);
    tower.stock_limit = Some(4);
    let mut fortress = building(FORTRESS, BuildingSize::Large, 11, 14, 9);
    fortress.stock_limit = Some(6);
    let hut = BuildingType::new(HUT, BuildingSize::Small);
    let mut catapult = BuildingType::new(CATAPULT, BuildingSize::Medium);
    catapult.catapult = true;
    catapult.stock_limit = Some(4);
    catapult.working_radius = 12;
    catapult.worker = Some(HELPER);
    let good = |kind, costs: Vec<(u32, u32)>| GoodRules {
        costs,
        kind,
        ..Default::default()
    };
    let mut goods = std::collections::BTreeMap::new();
    goods.insert(
        RANK[0],
        good(
            GoodKind::Settler,
            vec![(HELPER, 1), (SWORD, 1), (SHIELD, 1), (BEER, 1)],
        ),
    );
    for &r in &RANK[1..] {
        goods.insert(r, good(GoodKind::Settler, vec![]));
    }
    goods.insert(HELPER, good(GoodKind::Settler, vec![]));
    goods.insert(WOODCUTTER, good(GoodKind::Settler, vec![]));
    for g in [SWORD, SHIELD, BEER, COIN, STONE, PLANK] {
        goods.insert(g, good(GoodKind::Ware, vec![]));
    }
    let mut priorities = std::collections::BTreeMap::new();
    for g in [SWORD, SHIELD, BEER, COIN] {
        priorities.insert(g, 1);
    }
    Rules {
        building_types: vec![castle, barracks, guard, tower, fortress, hut, catapult],
        stone: Some(STONE),
        headquarters: Some(CASTLE),
        start_stock: vec![(RANK[0], 10), (HELPER, 20)],
        goods,
        transport_priorities: priorities,
        roles: crate::Roles {
            helper: Some(HELPER),
            coin: Some(COIN),
            ..Default::default()
        },
        military: MilitaryRules {
            soldiers: RANK.map(Some),
            max_health: [3, 4, 5, 6, 7],
            fighting_ticks: 17,
        },
        ..Rules::empty()
    }
}

/// A game on a flat map with one headquarters per entry.
pub(crate) fn game(width: u32, height: u32, players: &[(PlayerId, Point)]) -> World {
    let setup = GameSetup {
        players: players.to_vec(),
        local_player: P0,
        ..Default::default()
    };
    World::new_game(5, flat(width, height), rules(), &setup)
}

/// A world in which player 0 owns every field and nothing is Border, as
/// the test worlds of ECO-PLACE.
pub(crate) fn owned_world(width: u32, height: u32) -> World {
    World::with_map(5, flat(width, height), rules(), P0)
}

/// A finished building of `kind`, not occupied.
pub(crate) fn finished(world: &mut World, player: PlayerId, kind: u32, at: Point) {
    let size = world.building_types[&kind].size;
    world.put_building(player, kind, size, at, false);
    world.complete(at);
}

/// A soldier of `rank` linked to the building on `at` and present.
pub(crate) fn garrison(world: &mut World, at: Point, rank: usize, health: u32) -> usize {
    let target = world.target_of(at).expect("a building");
    let owner = world.building_at(at).expect("a building").owner;
    let id = world.add_settler(Settler {
        number: 0,
        player: owner,
        profession: RANK[rank],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(at),
        job: SettlerJob::Garrison {
            target,
            health,
            heal: 0,
        },
        timer: 0,
        carrying: None,
    });
    world
        .military
        .get_mut(&at)
        .expect("military")
        .soldiers
        .push(id);
    id
}

/// A finished building with one private present, occupied in the next
/// step's building update.
pub(crate) fn occupied(world: &mut World, player: PlayerId, kind: u32, at: Point) {
    finished(world, player, kind, at);
    garrison(world, at, 0, 3);
    world.step();
}

pub(crate) fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
}

pub(crate) fn run_to(world: &mut World, tick: u64) {
    while world.tick() < tick {
        world.step();
    }
}

pub(crate) fn road_between(world: &mut World, a: Point, b: Point) {
    let (from, to) = (world.flag_of(a), world.flag_of(b));
    world.submit(Command::BuildRoad {
        player: world.building_at(a).expect("a building").owner,
        from,
        to,
    });
    world.step();
    assert!(
        world.last_results()[0].outcome.is_ok(),
        "{:?}",
        world.last_results()
    );
}

fn lost_territory(world: &World, player: PlayerId) -> usize {
    world
        .messages()
        .iter()
        .filter(|m| m.player == player && m.text == "Lost Territory.")
        .count()
}

fn occupied_messages(world: &World) -> usize {
    world
        .messages()
        .iter()
        .filter(|m| m.text == "Military building occupied!")
        .count()
}

fn owned_within(world: &World, at: Point, range: u32) -> usize {
    world
        .terrain()
        .fields()
        .filter(|&f| grid::distance(at, f) <= range && world.field_owner(f) == Some(P0))
        .count()
}

// ---- TER-territory.md ----

// T1: the claim comes in the first tick a soldier is present.
// spec: TER-2.1, TER-3.2, TER-3.3 (TER T1)
#[test]
fn ter_t1_claim_in_the_first_tick_a_soldier_is_present() {
    let mut world = game(70, 50, &[(P0, (20, 20))]);
    let at = (27, 20);
    finished(&mut world, P0, BARRACKS, at);
    run(&mut world, 5);
    assert_eq!(world.field_owner((35, 20)), None);
    assert!(!world.is_explored(P0, (37, 20)));
    assert_eq!(occupied_messages(&world), 0);
    garrison(&mut world, at, 0, 3);
    world.step();
    assert_eq!(occupied_messages(&world), 1);
    // Range 8 claimed, radius 11 explored.
    assert_eq!(world.field_owner((35, 20)), Some(P0));
    assert_eq!(world.field_owner((36, 20)), None);
    assert!(world.is_explored(P0, (38, 20)));
    assert!(!world.is_explored(P0, (39, 20)));
    assert!(world.military_at(at).unwrap().applied);
}

// T2: a second soldier changes nothing.
// spec: TER-2.1 (TER T2)
#[test]
fn ter_t2_second_soldier_claims_nothing() {
    let mut world = game(70, 50, &[(P0, (20, 20))]);
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    let owners = world.owners.clone();
    let messages = world.messages().len();
    garrison(&mut world, at, 1, 4);
    run(&mut world, 3);
    assert_eq!(world.owners, owners);
    assert_eq!(world.messages().len(), messages);
}

/// Two players: headquarters at (22, 20) and (50, 20); returns the world.
fn two_players() -> World {
    game(80, 50, &[(P0, (22, 20)), (P1, (50, 20))])
}

// T3: the weaker claim keeps the field the stronger one has.
// spec: TER-3.1, TER-3.2, TER-3.3 (TER T3)
#[test]
fn ter_t3_strength_rule_keeps_the_field() {
    let mut world = two_players();
    occupied(&mut world, P0, GUARD, (30, 20));
    assert_eq!(world.field_owner((36, 20)), Some(P0));
    occupied(&mut world, P1, BARRACKS, (42, 20));
    // Distance 6 from both: 9/36 against 8/36.
    assert_eq!(world.field_owner((36, 20)), Some(P0));
    // Nearer to the barracks: 8/25 against 9/49.
    assert_eq!(world.field_owner((37, 20)), Some(P1));
    assert_eq!(lost_territory(&world, P0), 1);
}

// T4: within 2 of the claiming building the field is taken outright.
// spec: TER-3.2, TER-3.3 (TER T4)
#[test]
fn ter_t4_outright_within_two() {
    let mut world = two_players();
    // Guard house range 9 from (32, 20) reaches (41, 20).
    occupied(&mut world, P0, GUARD, (32, 20));
    assert_eq!(world.field_owner((41, 20)), Some(P0));
    assert_eq!(lost_territory(&world, P0), 0);
    occupied(&mut world, P1, BARRACKS, (43, 20));
    assert_eq!(world.field_owner((41, 20)), Some(P1));
    assert!(lost_territory(&world, P0) >= 1);
}

// T5: the only claimant burns: its land is released without messages.
// spec: TER-4.1, TER-5.1 (TER T5)
#[test]
fn ter_t5_release_of_the_only_claimant() {
    let mut world = owned_world(60, 50);
    let at = (30, 25);
    occupied(&mut world, P0, BARRACKS, at);
    assert_eq!(owned_within(&world, at, 8), 217);
    world.submit(Command::RemoveBuilding { player: P0, at });
    world.step();
    assert_eq!(owned_within(&world, at, 8), 0);
    assert_eq!(lost_territory(&world, P0), 0);
    // Border on the ring of owned fields around the hole, inside the
    // recomputed 21 x 21 square.
    for field in world.terrain().fields() {
        let d = grid::distance(at, field);
        assert_eq!(
            world.terrain().has(field, state::BORDER),
            d == 9,
            "{field:?} at {d}"
        );
    }
}

// T6: with a second claimant the shared fields stay.
// spec: TER-4.1 (TER T6)
#[test]
fn ter_t6_release_recomputes_over_the_others() {
    let mut world = owned_world(60, 50);
    let (a, b) = ((26, 25), (32, 25));
    occupied(&mut world, P0, BARRACKS, a);
    occupied(&mut world, P0, BARRACKS, b);
    world.submit(Command::RemoveBuilding { player: P0, at: a });
    world.step();
    for field in world.terrain().fields() {
        if grid::distance(a, field) <= 8 {
            let expected = grid::distance(b, field) <= 8;
            assert_eq!(world.field_owner(field) == Some(P0), expected, "{field:?}");
        }
    }
}

// T7: buildings on land that is lost or becomes Border are removed.
// spec: TER-6.1, TER-6.2 (TER T7)
#[test]
fn ter_t7_buildings_on_lost_land_are_removed() {
    let mut world = owned_world(60, 50);
    let at = (30, 25);
    occupied(&mut world, P0, BARRACKS, at);
    finished(&mut world, P0, HUT, (35, 25));
    // Distance 9 from the barracks: the new Border ring.
    finished(&mut world, P0, HUT, (39, 25));
    // Distance 12: stays.
    finished(&mut world, P0, HUT, (42, 25));
    run(&mut world, 3);
    assert!(world.building_at((35, 25)).is_some());
    world.submit(Command::RemoveBuilding { player: P0, at });
    run(&mut world, 3);
    assert!(world.building_at((35, 25)).is_none());
    assert!(world.building_at((39, 25)).is_none());
    assert!(world.building_at((42, 25)).is_some());
}

// TRN-FREE T49 (TRN-FREE-6.4a, E-trn-042): a building removed because its
// land is lost (TER-6.2, before the settlers' part of the tick) sends its
// worker home from the removal tick R itself: the going-home state from R,
// standing R, R + 1 and R + 2, the walk home at progress 0 in R + 3. (A
// removal after the settlers' part, the computer player's, gives the tick
// after, T28.)
// spec: TRN-FREE-6.4a (TRN-FREE T49), TER-6.2
#[test]
fn trn_free_t49_a_worker_of_lost_land_goes_home_from_the_removal_tick() {
    let mut world = owned_world(60, 50);
    let at = (30, 25);
    occupied(&mut world, P0, BARRACKS, at);
    let hut = (35, 25);
    finished(&mut world, P0, HUT, hut);
    let target = world.target_of(hut).unwrap();
    let worker = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[0],
        list: crate::SettlerList::Workers,
        walker: Walker::new(hut),
        job: SettlerJob::Worker {
            target,
            work: crate::production::Work::Off,
        },
        timer: 0,
        carrying: None,
    });
    run(&mut world, 3);
    world.submit(Command::RemoveBuilding { player: P0, at });
    let mut removed = None;
    for _ in 0..10 {
        world.step();
        if world.building_at(hut).is_none() {
            // The tick just run (the counter is raised after it).
            removed = Some(world.tick() - 1);
            break;
        }
    }
    let r = removed.expect("the hut goes with the land");
    let s = world.settlers[worker].as_ref().expect("the worker");
    assert!(
        matches!(s.job, SettlerJob::Leaving { at, .. } if at == r),
        "going home from R = {r}: {:?}",
        s.job
    );
    for tick in ["R + 1", "R + 2"] {
        world.step();
        let s = world.settlers[worker].as_ref().unwrap();
        assert!(!s.walker.has_walk(), "it stands in {tick}");
    }
    world.step();
    let s = world.settlers[worker].as_ref().unwrap();
    assert!(s.walker.has_walk(), "R + 3: the walk home ordered");
    assert_eq!(s.walker.progress_bits, 0);
}

// TRN-FREE T50 (TRN-FREE-6.4a, E-trn-042): a worker on a free walk (back to
// his building's flag from his work) when he is sent home does not stop
// after the step in progress: he walks every step of it, and from its
// arrival tick A stands in A and A + 1 and orders his walk home in A + 2.
// spec: TRN-FREE-6.4a (TRN-FREE T50)
#[test]
fn trn_free_t50_a_worker_on_a_free_walk_completes_it() {
    let mut world = owned_world(60, 50);
    occupied(&mut world, P0, BARRACKS, (30, 25));
    let hut = (35, 25);
    finished(&mut world, P0, HUT, hut);
    let target = world.target_of(hut).unwrap();
    let mut walker = Walker::new((30, 28));
    let path = [(31, 28), (32, 28), (33, 28), (34, 28)];
    walker.walk(path);
    let worker = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[0],
        list: crate::SettlerList::Workers,
        walker,
        job: SettlerJob::Worker {
            target,
            work: crate::production::Work::Back { score: 0 },
        },
        timer: 0,
        carrying: None,
    });
    world.step();
    world.step();
    let w = &world.settlers[worker].as_ref().unwrap().walker;
    assert!(
        w.next.is_some() && !w.path.is_empty(),
        "a step in progress, more to go"
    );
    world.submit(Command::RemoveBuilding {
        player: P0,
        at: hut,
    });
    let mut arrival = None;
    for _ in 0..200 {
        world.step();
        let s = world.settlers[worker].as_ref().expect("the worker");
        assert!(matches!(s.job, SettlerJob::Leaving { .. }), "{:?}", s.job);
        if !s.walker.is_walking() {
            arrival = Some(s.walker.at);
            break;
        }
    }
    assert_eq!(arrival, Some((34, 28)), "the whole walk, A");
    world.step();
    let s = world.settlers[worker].as_ref().unwrap();
    assert!(!s.walker.has_walk(), "it stands in A + 1");
    world.step();
    let s = world.settlers[worker].as_ref().unwrap();
    assert!(s.walker.has_walk(), "A + 2: the walk home ordered");
    assert_eq!(s.walker.progress_bits, 0);
}

// T8: a flag on a field that becomes Border goes with its roads.
// spec: TER-6.3, TER-6.4 (TER T8)
#[test]
fn ter_t8_flags_on_border_fields_are_removed() {
    let mut world = owned_world(60, 50);
    let at = (30, 25);
    occupied(&mut world, P0, BARRACKS, at);
    assert_eq!(flag(&mut world, (39, 25)), Ok(()));
    assert_eq!(flag(&mut world, (42, 25)), Ok(()));
    road(&mut world, (39, 25), (42, 25)).unwrap();
    assert!(world.roads().count() == 1);
    world.submit(Command::RemoveBuilding { player: P0, at });
    run(&mut world, 2);
    assert_eq!(world.flag_at((39, 25)), None);
    assert_eq!(world.roads().count(), 0);
    assert_eq!(world.flag_at((42, 25)), Some(P0));
    assert!(!world.terrain().has((40, 25), state::STREET));
}

// T9: a road over land that becomes foreign is removed during the claim.
// spec: TER-5.3, TER-5.4, TER-6.3 (TER T9)
#[test]
fn ter_t9_roads_over_foreign_land_are_removed() {
    let mut world = owned_world(60, 50);
    assert_eq!(flag(&mut world, (22, 25)), Ok(()));
    assert_eq!(flag(&mut world, (28, 25)), Ok(()));
    road(&mut world, (22, 25), (28, 25)).unwrap();
    assert_eq!(world.roads().count(), 1);
    // Player 1 occupies a barracks 5 fields east of the road's end.
    occupied(&mut world, P1, BARRACKS, (33, 25));
    assert_eq!(world.field_owner((25, 25)), Some(P1));
    assert_eq!(world.roads().count(), 0);
    assert!(!world.terrain().has((25, 25), state::STREET));
    // The flag on foreign land goes in the roads update.
    assert_eq!(world.flag_at((28, 25)), None);
    assert!(world.have_met(P0, P1));
}

// T10, T11: the Border bit.
// spec: TER-5.1 (TER T10, T11)
#[test]
fn ter_t10_t11_border_state() {
    let mut world = owned_world(40, 40);
    let at = (6, 20);
    occupied(&mut world, P0, BARRACKS, at);
    // All of the map is player 0's: only the outer ring of the map is
    // Border by the off-map neighbour rule, inside the recomputed square.
    assert!(world.terrain().has((0, 20), state::BORDER));
    assert!(!world.terrain().has((4, 20), state::BORDER));
    assert!(!world.terrain().has((14, 20), state::BORDER));
    let mut world = game(60, 50, &[(P0, (30, 25))]);
    assert!(world.terrain().has((39, 25), state::BORDER));
    assert!(!world.terrain().has((38, 25), state::BORDER));
    let _ = &mut world;
}

// T12: an upgrade claims again with the new range at once.
// spec: TER-2.3 (TER T12)
#[test]
fn ter_t12_upgrade_claims_again() {
    let mut world = game(70, 50, &[(P0, (20, 20))]);
    let at = (27, 20);
    occupied(&mut world, P0, BARRACKS, at);
    assert_eq!(world.field_owner((36, 20)), None);
    world.change_building_type(at, GUARD);
    assert_eq!(world.field_owner((36, 20)), Some(P0));
    assert!(world.is_explored(P0, (39, 20)));
}

// TER-6.2: a construction site on lost land is removed as well.
#[test]
fn ter_6_2_a_site_on_foreign_land_is_removed() {
    let mut world = owned_world(40, 40);
    let site = (20, 20);
    world.put_building(P0, HUT, BuildingSize::Small, site, true);
    world.step();
    assert!(world.building_at(site).is_some());
    let index = world.terrain().index(site);
    world.owners[index] = Some(P1);
    world.step();
    assert!(world.building_at(site).is_none());
}

// ---- MIL-soldiers.md ----

use crate::{Requester, SOLDIER_RANKS};

const HQ: Point = (20, 20);

/// A game with one headquarters that keeps no soldiers outside its stock.
fn quiet_game() -> World {
    let mut world = game(80, 40, &[(P0, HQ)]);
    world.military.get_mut(&HQ).unwrap().castle = [0; SOLDIER_RANKS];
    world
}

fn military_requests(world: &World, player: PlayerId) -> usize {
    world
        .requests(player)
        .iter()
        .filter(|r| matches!(r.requester, Requester::Military(_)))
        .count()
}

fn ranks_of(world: &World, at: Point) -> Vec<usize> {
    world.garrison(at).iter().map(|s| s.rank).collect()
}

fn set_stock(world: &mut World, at: Point, goods: &[(u32, u32)]) {
    let warehouse = world.warehouses.get_mut(&at).unwrap();
    warehouse.stock = goods.to_vec();
}

fn stock_of(world: &World, at: Point, good: u32) -> u32 {
    world.warehouse(at).unwrap().count(good)
}

// T1: far from enemies a guard house wants one soldier, once.
// spec: MIL-SOL-3.3, MIL-SOL-4.1, MIL-SOL-4.3, MIL-SOL-4.7 (MIL T1)
#[test]
fn mil_t1_one_request_when_no_enemy() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, GUARD, at);
    road_between(&mut world, at, HQ);
    run_to(&mut world, 21);
    assert_eq!(military_requests(&world, P0), 1, "filed in tick 20");
    // The headquarters has just sent the road's carrier: its 30-tick
    // cooldown (ECO-REQ-3.7) holds the soldier back.
    run_to(&mut world, 60);
    assert_eq!(military_requests(&world, P0), 0, "served");
    assert_eq!(world.military_at(at).unwrap().soldiers.len(), 1);
    run_to(&mut world, 100);
    assert_eq!(military_requests(&world, P0), 0);
    assert_eq!(world.military_at(at).unwrap().soldiers.len(), 1);
}

// TRN-FREE T52 (TRN-FREE-6.4, E-trn-042): all three tries also on a flag:
// a settler whose home search runs on a flag with no road finds no
// warehouse by the road try (1); try 2 or 3 follows in the same tick, a free
// walk to the warehouse's network flag at progress 0, and no wander step
// (no draw).
// spec: TRN-FREE-6.4 (TRN-FREE T52)
#[test]
fn trn_free_t52_on_a_flag_without_roads_the_free_walk_follows() {
    let mut world = quiet_game();
    let lone = (27, 22);
    world.submit(Command::PlaceFlag {
        player: P0,
        at: lone,
    });
    world.step();
    assert_eq!(world.flag_at(lone), Some(P0));
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(lone),
        job: SettlerJob::Leaving {
            step: crate::LeavingStep::Search,
            at: 0,
            stand: None,
            health: Some(3),
        },
        timer: 0,
        carrying: None,
    });
    let draws = world.rng().draws();
    world.step();
    let s = world.settlers[id].clone().expect("the settler");
    assert!(
        matches!(
            s.job,
            SettlerJob::Leaving {
                step: crate::LeavingStep::ToFlag,
                ..
            }
        ),
        "a free walk to the network flag: {:?}",
        s.job
    );
    assert_eq!(s.walker.destination(), world.flag_of(HQ));
    assert_eq!(s.walker.progress_bits, 0);
    assert!(
        world.rng().draws() > draws,
        "the free walk's draws (TRN-FREE-2), not a wander step"
    );
}

// TRN-FREE T51 (TRN-FREE-6.4a, E-trn-042, Q-1075): a reinforcement still on
// his walk out of the warehouse (to its own flag) when the building he goes
// to is removed is not sent home then: he walks on to the flag; in the
// update that walk ends there (A) he finds his target gone and shows the
// going-home state; he stands in A .. A + 3 and starts home at progress 0
// in A + 4 (Q-1075: built as going home from A, T = A; on the warehouse's
// own flag the home search of A + 3 orders a walk with no step,
// TRN-FREE-6.4c, which gives the fourth standing tick).
// spec: TRN-FREE-6.4a (TRN-FREE T51)
#[test]
fn trn_free_t51_a_reinforcement_walks_out_to_the_flag_first() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, GUARD, at);
    road_between(&mut world, at, HQ);
    let mut id = None;
    while id.is_none() {
        world.step();
        assert!(world.tick() < 100, "the soldier is sent");
        id = world
            .settlers()
            .find(|(_, s)| matches!(s.job, SettlerJob::ToMilitary { .. }))
            .map(|(i, _)| i);
    }
    let id = id.unwrap();
    world.submit(Command::RemoveBuilding { player: P0, at });
    let flag = world.flag_of(HQ);
    let s = |world: &World| world.settlers[id].clone().expect("the soldier");
    let mut a = false;
    for _ in 0..100 {
        world.step();
        let x = s(&world);
        if x.walker.at == flag {
            a = true;
            break;
        }
        assert!(
            matches!(x.job, SettlerJob::ToMilitary { .. }),
            "on his walk out: {:?}",
            x.job
        );
    }
    assert!(a, "he reaches the warehouse's flag");
    let x = s(&world);
    assert!(
        matches!(x.job, SettlerJob::Leaving { .. }),
        "A: {:?} {:?}",
        x.job,
        x.walker
    );
    assert!(!x.walker.has_walk(), "A: his walk stopped there");
    for tick in ["A + 1", "A + 2", "A + 3"] {
        world.step();
        let x = s(&world);
        assert_eq!(x.walker.at, flag);
        assert!(!x.walker.has_walk(), "he stands in {tick}");
    }
    world.step();
    let x = s(&world);
    assert!(x.walker.has_walk(), "A + 4: the walk home");
    assert_eq!(x.walker.progress_bits, 0);
}

// AI-MIL T33 (AI-MIL-5.4, E-ai-077; Q-1076 answered, E-ai-078): the
// computer player's def counts the soldiers on the building's list as the
// original keeps it: one inside, one out attacking (he stays on the list)
// and one on his step in from the flag count; one still walking to the
// flag does not (the original lists him from his arrival there).
// spec: AI-MIL-5.4 (AI-MIL T33)
#[test]
fn ai_mil_t33_def_counts_the_list_from_the_step_in() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, GUARD, at);
    road_between(&mut world, at, HQ);
    garrison(&mut world, at, 0, 3);
    let out = garrison(&mut world, at, 0, 3);
    let home = world.target_of(at).unwrap();
    world.settlers[out].as_mut().unwrap().job = SettlerJob::Attack {
        home,
        health: 3,
        step: crate::AttackStep::Walking,
    };
    assert_eq!(world.listed_soldiers(at), 2, "inside and out attacking");
    let flag = world.flag_of(at);
    let coming = |world: &mut World, walker: Walker| {
        let id = world.add_settler(Settler {
            number: 0,
            player: P0,
            profession: RANK[0],
            list: crate::SettlerList::Soldiers,
            walker,
            job: SettlerJob::ToMilitary {
                target: home,
                health: 3,
            },
            timer: 0,
            carrying: None,
        });
        world.military.get_mut(&at).unwrap().soldiers.push(id);
        id
    };
    let mut to_flag = Walker::new((24, 19));
    to_flag.walk([(25, 19), flag, at]);
    to_flag.next = Some((25, 19));
    to_flag.path.pop_front();
    coming(&mut world, to_flag);
    assert_eq!(world.listed_soldiers(at), 2, "not yet on the list");
    let mut stepping = Walker::new(flag);
    stepping.walk([at]);
    stepping.next = Some(at);
    stepping.path.clear();
    coming(&mut world, stepping);
    assert_eq!(world.listed_soldiers(at), 3, "on his step in");
}

// MIL-SOL-4.7a: the soldier sent for the request stands in its creation
// tick on the headquarters' door field (its walk not started), takes its
// first step in the next tick, ends its road walk exactly on the guard
// house's flag and starts the step inside from there at progress 0.
// spec: MIL-SOL-4.7a, MIL-SOL-4.7, ECO-WORK-3.8a, ECO-WORK-3.8c
#[test]
fn the_soldier_stands_in_its_first_tick_and_stops_on_the_flag() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, GUARD, at);
    road_between(&mut world, at, HQ);
    let door = world.door_field(HQ);
    let soldier = |world: &World| {
        world
            .settlers()
            .find(|(_, s)| matches!(s.job, SettlerJob::ToMilitary { .. }))
            .map(|(_, s)| s.walker.clone())
    };
    while soldier(&world).is_none() {
        world.step();
        assert!(world.tick() < 100, "the soldier is sent");
    }
    let first = soldier(&world).unwrap();
    assert_eq!((first.at, first.next, first.progress()), (door, None, 0.0));
    world.step();
    let second = soldier(&world).unwrap();
    assert_eq!(second.at, door);
    assert!(second.next.is_some() && second.progress() > 0.0);
    // On the guard house's flag point, the step inside at progress 0.
    let flag = world.flag_of(at);
    let mut on_flag = None;
    while let Some(w) = soldier(&world) {
        if w.at == flag {
            on_flag = Some(w);
            break;
        }
        world.step();
        assert!(world.tick() < 1000, "the soldier arrives");
    }
    let on_flag = on_flag.expect("seen on the flag");
    assert_eq!((on_flag.next, on_flag.progress()), (Some(at), 0.0));
}

/// Player 1's headquarters 36 fields east of the player-0 one, with an
/// occupied barracks `gap` fields from the building at (28, 20).
fn with_enemy(gap: u32) -> (World, Point) {
    let mut world = game(100, 40, &[(P0, HQ), (P1, (50, 20))]);
    world.military.get_mut(&HQ).unwrap().castle = [0; SOLDIER_RANKS];
    let enemy = (27 + gap, 20);
    occupied(&mut world, P1, BARRACKS, enemy);
    (world, (27, 20))
}

// T2
// spec: MIL-SOL-3.1, MIL-SOL-3.2 (MIL T2)
#[test]
fn mil_t2_medium_distance() {
    let (mut world, at) = with_enemy(20);
    finished(&mut world, P0, TOWER, at);
    run_to(&mut world, 12);
    let m = world.military_at(at).unwrap();
    assert_eq!(m.enemy_distance, 20);
    // trunc(0.3 x 6 + 0.5)
    assert_eq!(world.wanted_soldiers(at), 2);
}

// T3: near, a fortress wants all 9, one request per pass.
// spec: MIL-SOL-3.2, MIL-SOL-4.1, MIL-SOL-4.3 (MIL T3)
#[test]
fn mil_t3_near_one_request_per_pass() {
    let (mut world, at) = with_enemy(18);
    finished(&mut world, P0, FORTRESS, at);
    assert_eq!(world.wanted_soldiers(at), 9, "near while unset");
    run_to(&mut world, 12);
    assert_eq!(world.military_at(at).unwrap().enemy_distance, 18);
    assert_eq!(world.wanted_soldiers(at), 9);
    // No road: nothing is served. Passes at 20, 41, ..., 188.
    run_to(&mut world, 189);
    assert_eq!(military_requests(&world, P0), 9);
    run_to(&mut world, 300);
    assert_eq!(military_requests(&world, P0), 9, "never more than wanted");
}

// T4, T5: the enemy is gone: the weakest (or, preferring weak, the
// strongest) present soldier leaves; only one.
// spec: MIL-SOL-4.4, MIL-SOL-4.5 (MIL T4, T5)
#[test]
fn mil_t4_t5_release_choice() {
    for (strong, left) in [(true, vec![3]), (false, vec![0])] {
        let mut world = quiet_game();
        let at = (26, 20);
        finished(&mut world, P0, BARRACKS, at);
        garrison(&mut world, at, 0, 3);
        garrison(&mut world, at, 3, 6);
        world.submit(Command::MilitaryDefenseType { player: P0, strong });
        run_to(&mut world, 22);
        assert_eq!(world.military_at(at).unwrap().enemy_distance, 1000);
        assert_eq!(ranks_of(&world, at), left, "strong {strong}");
        run_to(&mut world, 120);
        assert_eq!(ranks_of(&world, at), left);
    }
}

// MIL T19 (MIL-SOL-4.5, TRN-FREE-6.4a): a released soldier first takes
// the one step from the building's field to its flag; only on arriving
// there (tick T) is it sent home: it stands in T, T + 1 and T + 2, and its
// home search runs from the flag in T + 3.
// spec: MIL-SOL-4.5 (MIL T19), TRN-FREE-6.4a, TRN-FREE-6.4 (TRN-FREE T52)
#[test]
fn t19_a_released_soldier_steps_out_to_the_flag_first() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, BARRACKS, at);
    garrison(&mut world, at, 0, 3);
    garrison(&mut world, at, 3, 6);
    let flag = world.flag_of(at);
    let leaving = |world: &World| {
        world.settlers().find_map(|(id, s)| match s.job {
            SettlerJob::Leaving {
                step: crate::LeavingStep::StepOut,
                ..
            } => Some(id),
            _ => None,
        })
    };
    let mut id = None;
    for _ in 0..200 {
        world.step();
        id = leaving(&world);
        if id.is_some() {
            break;
        }
    }
    let id = id.expect("a soldier released");
    let soldier = |world: &World| world.settlers().find(|(n, _)| *n == id).unwrap().1.clone();
    assert_eq!(
        soldier(&world).walker.destination(),
        flag,
        "the step to its flag"
    );
    while soldier(&world).walker.at != flag {
        world.step();
        assert!(world.tick() < 400, "arrives");
    }
    let arrived = world.tick() - 1;
    for k in 0..3 {
        let s = soldier(&world);
        assert!(matches!(s.job, SettlerJob::Leaving { .. }), "T + {k}");
        assert!(!s.walker.has_walk(), "stands in T + {k}");
        world.step();
    }
    // T + 3: the home search from the flag (here no road home: the road
    // try fails, and tries 2 and 3 follow, TRN-FREE T52: a free walk to the
    // headquarters' flag).
    assert!(matches!(
        soldier(&world).job,
        SettlerJob::Leaving {
            step: crate::LeavingStep::ToFlag,
            ..
        }
    ));
    assert_eq!(world.tick() - 1, arrived + 3);
}

// TRN-FREE-6.4 step 4 (2026-11-08 b, E-gfx-041): a settler with no
// warehouse in reach does not die at once. After each failed search it takes
// one wander step to a neighbour that is walkable and explored by its player
// (one draw of the generator picks it in ring order), and searches again in
// the tick after the step ends, the failure count going up by one; at the
// end of the 52nd step (the count above 50) it dies (TRN-CARRY-7.1: 28
// ticks, then gone).
// spec: TRN-FREE-6.4, TRN-CARRY-7.1
#[test]
fn a_homeless_settler_wanders_between_tries_then_dies() {
    let mut world = quiet_game();
    let profession = world.roles.helper.unwrap();
    let start = (50, 20);
    // Player 1 has no warehouse at all; the land about the settler is his.
    world.explore(P1, start, 2);
    let draws = world.rng.draws();
    world.send_home(P1, Some(profession), start, 777);
    assert_eq!(world.rng.draws(), draws + 1, "one draw for the step");
    let find = |world: &World| {
        world
            .settlers()
            .find(|(_, s)| s.number == 777)
            .map(|(_, s)| s.clone())
    };
    let s = find(&world).expect("alive");
    assert!(matches!(
        s.job,
        SettlerJob::Leaving {
            step: crate::LeavingStep::Wander { tries: 0 },
            ..
        }
    ));
    let mut steps = 1;
    let mut last = 0;
    let mut died = None;
    let mut stood = start;
    assert_eq!(s.walker.path.len(), 1);
    assert_eq!(grid::distance(stood, s.walker.path[0]), 1);
    for _ in 0..20_000 {
        world.step();
        let Some(s) = find(&world) else {
            break;
        };
        match s.job {
            SettlerJob::Leaving {
                step: crate::LeavingStep::Wander { tries },
                ..
            } if tries != last => {
                assert_eq!(tries, last + 1, "one more failure");
                assert_eq!(s.walker.path.len(), 1, "a single step");
                let to = s.walker.path[0];
                assert_eq!(grid::distance(stood, to), 1, "to a neighbour");
                assert!(world.is_explored(P1, to), "explored by his player");
                last = tries;
                steps += 1;
            }
            SettlerJob::Dying { .. } if died.is_none() => died = Some(world.tick()),
            _ => {}
        }
        if s.walker.next.is_none() {
            stood = s.walker.at;
        }
    }
    assert_eq!((steps, last), (52, 51), "52 searches and 52 wander steps");
    assert!(died.is_some(), "it dies after the 52nd step");
    assert!(find(&world).is_none(), "gone after dying");
}

// T44 (TRN-FREE-6.4 step 4, MIL-SOL-1.2; Q-1034): a homeless soldier makes
// no search for a military building. A private of health 3 with no
// warehouse in reach: after each 52nd failed search (the count 51 when the
// wander step ends) he loses 1 health and his count starts over at 0; at
// the 156th failure his health is 0 and he dies (TRN-CARRY-7.1). Nothing
// draws at those points: the only draws are the wander steps' (one each).
// spec: TRN-FREE-6.4, MIL-SOL-1.2, TRN-CARRY-7.1
#[test]
fn t44_a_homeless_soldier_loses_health_every_52nd_failure() {
    let mut world = quiet_game();
    let start = (50, 20);
    world.explore(P1, start, 2);
    let tick = world.tick();
    world.add_settler(Settler {
        number: 778,
        player: P1,
        profession: RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(start),
        job: SettlerJob::Leaving {
            step: crate::LeavingStep::Search,
            at: tick,
            stand: None,
            health: Some(3),
        },
        timer: 0,
        carrying: None,
    });
    let find = |world: &World| {
        world
            .settlers()
            .find(|(_, s)| s.number == 778)
            .map(|(_, s)| s.clone())
    };
    let draws = world.rng.draws();
    let mut steps: Vec<(u8, Option<u32>)> = Vec::new();
    let mut wandering = false;
    let mut died = None;
    for _ in 0..60_000 {
        world.step();
        let Some(s) = find(&world) else {
            break;
        };
        match s.job {
            SettlerJob::Leaving {
                step: crate::LeavingStep::Wander { tries },
                health,
                ..
            } => {
                if !wandering {
                    steps.push((tries, health));
                }
                wandering = true;
            }
            SettlerJob::Dying { .. } if died.is_none() => {
                died = Some(world.tick());
                wandering = false;
            }
            _ => wandering = false,
        }
    }
    assert_eq!(steps.len(), 156, "52 x 3 searches and wander steps");
    for (k, &(tries, health)) in steps.iter().enumerate() {
        assert_eq!(usize::from(tries), k % 52, "step {k}");
        assert_eq!(health, Some(3 - (k / 52) as u32), "step {k}");
    }
    assert!(died.is_some(), "he dies after the 156th step");
    assert!(find(&world).is_none(), "gone after dying");
    assert_eq!(world.rng.draws(), draws + 156, "one draw per wander step");
}

// T6: one departure per 42 ticks from a full fortress.
// spec: MIL-SOL-4.4 (MIL T6)
#[test]
fn mil_t6_release_cooldown() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, FORTRESS, at);
    for _ in 0..9 {
        garrison(&mut world, at, 1, 4);
    }
    let mut left = Vec::new();
    let mut before = 9;
    for _ in 0..140 {
        world.step();
        let now = world.military_at(at).unwrap().soldiers.len();
        if now < before {
            left.push(world.tick() - 1);
            before = now;
        }
    }
    assert_eq!(left, [20, 62, 104]);
}

// T7: soldiers still walking are released at once when nothing is wanted.
// spec: MIL-SOL-4.4, MIL-SOL-4.6 (MIL T7)
#[test]
fn mil_t7_walking_soldiers_released_at_once() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, BARRACKS, at);
    world.military.get_mut(&at).unwrap().applied = true;
    world.submit(Command::ToggleMoveOut { player: P0, at });
    let target = world.target_of(at).unwrap();
    for _ in 0..2 {
        let mut walker = Walker::new((30, 20));
        walker.walk((31..56).map(|x| (x, 20)));
        let id = world.add_settler(Settler {
            number: 0,
            player: P0,
            profession: RANK[0],
            list: crate::SettlerList::Soldiers,
            walker,
            job: SettlerJob::ToMilitary { target, health: 3 },
            timer: 0,
            carrying: None,
        });
        world.military.get_mut(&at).unwrap().soldiers.push(id);
    }
    run_to(&mut world, 21);
    assert!(world.military_at(at).unwrap().soldiers.is_empty());
    assert_eq!(world.wanted_soldiers(at), 0);
}

// T8: the warehouse sends its strongest soldier by default, its weakest
// when preferring weak; it never makes one.
// spec: MIL-SOL-4.7 (MIL T8)
#[test]
fn mil_t8_serving_picks_by_defense_type() {
    for (strong, sent) in [(true, RANK[4]), (false, RANK[0])] {
        let mut world = quiet_game();
        set_stock(&mut world, HQ, &[(RANK[0], 3), (RANK[4], 1), (HELPER, 5)]);
        let at = (26, 20);
        finished(&mut world, P0, BARRACKS, at);
        road_between(&mut world, at, HQ);
        world.submit(Command::MilitaryDefenseType { player: P0, strong });
        world.step();
        world.warehouses.get_mut(&HQ).unwrap().cooldown = 0;
        let target = world.target_of(at).unwrap();
        assert!(world.serve_soldier(P0, target));
        let id = world.military_at(at).unwrap().soldiers[0];
        assert_eq!(world.settlers[id].as_ref().unwrap().profession, sent);
    }
    // No soldier in stock: nothing is made from helper, sword and shield.
    let mut world = quiet_game();
    set_stock(
        &mut world,
        HQ,
        &[(HELPER, 5), (SWORD, 5), (SHIELD, 5), (BEER, 5)],
    );
    let at = (26, 20);
    finished(&mut world, P0, BARRACKS, at);
    road_between(&mut world, at, HQ);
    world.warehouses.get_mut(&HQ).unwrap().cooldown = 0;
    let target = world.target_of(at).unwrap();
    assert!(!world.serve_soldier(P0, target));
}

// T9, T10: recruiting every 101 ticks, up to 10 per warehouse.
// spec: MIL-SOL-5.1, MIL-SOL-5.3 (MIL T9)
#[test]
fn mil_t9_recruiting_uses_the_cost_list() {
    let mut world = quiet_game();
    set_stock(
        &mut world,
        HQ,
        &[(HELPER, 12), (SWORD, 3), (SHIELD, 5), (BEER, 9)],
    );
    run_to(&mut world, 100);
    assert_eq!(stock_of(&world, HQ, RANK[0]), 0, "first run in tick 100");
    run_to(&mut world, 101);
    assert_eq!(stock_of(&world, HQ, RANK[0]), 3);
    assert_eq!(stock_of(&world, HQ, HELPER), 9);
    assert_eq!(stock_of(&world, HQ, SWORD), 0);
    assert_eq!(stock_of(&world, HQ, SHIELD), 2);
    assert_eq!(stock_of(&world, HQ, BEER), 6);
}

// spec: MIL-SOL-5.1, MIL-SOL-5.2 (MIL T10)
#[test]
fn mil_t10_at_most_ten_per_run() {
    let mut world = quiet_game();
    set_stock(
        &mut world,
        HQ,
        &[(HELPER, 30), (SWORD, 30), (SHIELD, 30), (BEER, 30)],
    );
    run_to(&mut world, 101);
    assert_eq!(stock_of(&world, HQ, RANK[0]), 10);
    run_to(&mut world, 201);
    assert_eq!(stock_of(&world, HQ, RANK[0]), 10);
    run_to(&mut world, 202);
    assert_eq!(stock_of(&world, HQ, RANK[0]), 20);
}

// T11: the headquarters keeps one soldier of each rank outside its stock;
// T20: one comes out at a time, every 30 ticks (the door wait).
// spec: MIL-SOL-6.1, MIL-SOL-6.2 (MIL T11, T20)
#[test]
fn mil_t11_headquarters_keeps_one_of_each_rank() {
    let mut world = game(60, 40, &[(P0, HQ)]);
    let stock: Vec<(u32, u32)> = RANK.iter().map(|&r| (r, 3)).collect();
    set_stock(&mut world, HQ, &stock);
    world.step();
    assert_eq!(ranks_of(&world, HQ), [0]);
    for _ in 0..4 * crate::requests::SEND_COOLDOWN {
        world.step();
    }
    for &r in &RANK {
        assert_eq!(stock_of(&world, HQ, r), 2);
    }
    assert_eq!(ranks_of(&world, HQ), [0, 1, 2, 3, 4]);
    assert_eq!(world.present_soldiers(HQ), 5);
    for _ in 0..crate::requests::SEND_COOLDOWN {
        world.step();
    }
    // Lowering the amount puts one back into the stock.
    world.submit(Command::SetCastleSoldiers {
        player: P0,
        at: HQ,
        rank: 2,
        amount: 0,
    });
    world.step();
    assert_eq!(stock_of(&world, HQ, RANK[2]), 3);
    assert_eq!(ranks_of(&world, HQ), [0, 1, 3, 4]);
}

// T20 (MIL-SOL-6.2): a headquarters soldier comes out only while the
// warehouse's door is free, and coming out starts its 30-tick wait: with
// the reserve of privates raised to 4 they come out one every 30 ticks (R6:
// 599, 629, 659, 689), and the wait is the warehouse's send cooldown.
// spec: MIL-SOL-6.2 (MIL T20), ECO-REQ-3.7
#[test]
fn mil_t20_soldiers_come_out_one_every_30_ticks() {
    let mut world = game(60, 40, &[(P0, HQ)]);
    set_stock(&mut world, HQ, &[(RANK[0], 10)]);
    world.step();
    assert_eq!(world.present_soldiers(HQ), 1);
    for _ in 0..crate::requests::SEND_COOLDOWN {
        world.step();
    }
    world.submit(Command::SetCastleSoldiers {
        player: P0,
        at: HQ,
        rank: 0,
        amount: 4,
    });
    let mut came = Vec::new();
    for _ in 0..100 {
        let before = world.present_soldiers(HQ);
        world.step();
        if world.present_soldiers(HQ) > before {
            came.push(world.tick());
            assert_eq!(
                world.warehouses[&HQ].cooldown,
                crate::requests::SEND_COOLDOWN,
                "the door wait starts"
            );
        }
    }
    assert_eq!(came.len(), 3, "{came:?}");
    assert_eq!([came[1] - came[0], came[2] - came[1]], [30, 30]);
    assert_eq!(world.present_soldiers(HQ), 4);
}

// T12: one health point per 602 ticks inside.
// spec: MIL-SOL-1.2 (MIL T12)
#[test]
fn mil_t12_healing() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, BARRACKS, at);
    garrison(&mut world, at, 0, 1);
    run(&mut world, 601);
    assert_eq!(world.garrison(at)[0].health, 1);
    run(&mut world, 1);
    assert_eq!(world.garrison(at)[0].health, 2);
    run(&mut world, 602);
    assert_eq!(world.garrison(at)[0].health, 3);
    run(&mut world, 1204);
    assert_eq!(world.garrison(at)[0].health, 3, "not above the maximum");
}

// T13, T14: the duel numbers of MIL-SOL-1.3 (the fight itself is tested with combat).
#[test]
fn mil_t13_t14_duel_hit_chance() {
    assert_eq!(crate::military::duel_hit_outcomes(0, 0), 12);
    assert_eq!(crate::military::duel_hit_outcomes(4, 0), 24);
}

// T15: teams.
// spec: MIL-SOL-3.1 (MIL T15)
#[test]
fn mil_t15_enemy_distance_counts_enemies_only() {
    let setup = GameSetup {
        players: vec![(P0, (20, 20)), (P1, (50, 20)), (P2, (80, 20))],
        local_player: P0,
        teams: vec![(P0, 0), (P1, 0), (P2, -1)],
        ..Default::default()
    };
    let mut world = World::new_game(5, flat(110, 40), rules(), &setup);
    assert!(!world.are_enemies(P0, P1));
    assert!(world.are_enemies(P0, P2));
    assert!(!world.are_enemies(P2, P2));
    // Nobody's team (-1) is not a team of its own.
    assert!(world.are_enemies(P2, PlayerId(3)));
    let at = (27, 20);
    finished(&mut world, P0, BARRACKS, at);
    run_to(&mut world, 12);
    // Player 1's headquarters at distance 23 is no enemy; player 2's
    // headquarters at 53 is.
    assert_eq!(world.military_at(at).unwrap().enemy_distance, 53);
}

// The whole way: a request is filed, served, the soldier walks, arrives
// and the building claims its land (MIL-SOL-4.7, TER-2.1).
#[test]
fn mil_soldier_walks_in_and_claims() {
    let mut world = quiet_game();
    let at = (26, 20);
    finished(&mut world, P0, BARRACKS, at);
    road_between(&mut world, at, HQ);
    assert_eq!(world.field_owner((34, 20)), None);
    run_to(&mut world, 500);
    assert!(world.military_at(at).unwrap().applied);
    assert_eq!(world.field_owner((34, 20)), Some(P0));
    assert_eq!(world.present_soldiers(at), 1);
    assert_eq!(occupied_messages(&world), 1);
    // A private left the stock for it.
    assert_eq!(stock_of(&world, HQ, RANK[0]), 9);
}

// ---- MIL-coins-promotion.md ----

fn coins(world: &mut World, at: Point, n: u32) {
    world.inputs.insert(at, vec![(COIN, n)]);
}

fn coins_in(world: &World, at: Point) -> u32 {
    world
        .inputs
        .get(&at)
        .map_or(0, |l| l.iter().map(|&(_, n)| n).sum())
}

/// A barracks that keeps as many soldiers as the test puts in (the limit
/// and the "far" factor are raised so that the allocation lets none go).
fn barracks_with(world: &mut World, ranks: &[usize]) -> Point {
    let at = (26, 20);
    world
        .building_types
        .get_mut(&BARRACKS)
        .unwrap()
        .soldier_limit = 9;
    world.submit(Command::MilitaryAllocation {
        player: P0,
        class: 2,
        thousandths: 1000,
    });
    finished(world, P0, BARRACKS, at);
    for &r in ranks {
        garrison(world, at, r, 3);
    }
    at
}

// T1
// spec: MIL-PROM-4.1, MIL-PROM-4.2 (PROM T1)
#[test]
fn prom_t1_one_coin_promotes_one_private() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0, 0]);
    coins(&mut world, at, 1);
    run_to(&mut world, 301);
    assert_eq!(ranks_of(&world, at), [1, 0]);
    assert_eq!(coins_in(&world, at), 0);
}

// T2
// spec: MIL-PROM-4.2, MIL-PROM-4.3 (PROM T2)
#[test]
fn prom_t2_one_per_rank_with_one_coin() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0, 1, 2, 3, 4]);
    coins(&mut world, at, 1);
    run_to(&mut world, 301);
    assert_eq!(ranks_of(&world, at), [1, 2, 3, 4, 4]);
    assert_eq!(coins_in(&world, at), 0);
}

// T3
// spec: MIL-PROM-4.1, MIL-PROM-4.2 (PROM T3)
#[test]
fn prom_t3_three_runs() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0, 0, 0]);
    coins(&mut world, at, 3);
    run_to(&mut world, 301);
    assert_eq!(ranks_of(&world, at), [1, 0, 0]);
    run_to(&mut world, 602);
    assert_eq!(ranks_of(&world, at), [2, 1, 0]);
    run_to(&mut world, 903);
    assert_eq!(ranks_of(&world, at), [3, 2, 1]);
    assert_eq!(coins_in(&world, at), 0);
}

// T4
// spec: MIL-PROM-4.2 (PROM T4)
#[test]
fn prom_t4_generals_keep_their_coins() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[4, 4]);
    coins(&mut world, at, 2);
    run_to(&mut world, 310);
    assert_eq!(ranks_of(&world, at), [4, 4]);
    assert_eq!(coins_in(&world, at), 2);
}

// T5
// spec: MIL-PROM-3.2, MIL-SOL-7.2 (PROM T5)
#[test]
fn prom_t5_switch_off_cancels_orders() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0]);
    world.order(at, COIN);
    world.order(at, COIN);
    world.submit(Command::ToggleCoinSupply { player: P0, at });
    run(&mut world, 34);
    assert!(!world.military_at(at).unwrap().coin_switch);
    assert_eq!(world.orders.iter().filter(|o| o.good == COIN).count(), 0);
}

// T6, T7
// spec: MIL-PROM-3.1, MIL-PROM-3.2 (PROM T6, T7)
#[test]
fn prom_t6_t7_orders_to_the_stock_limit() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0]);
    run(&mut world, 34);
    assert_eq!(world.orders.iter().filter(|o| o.good == COIN).count(), 2);
    let mut world = quiet_game();
    let at2 = barracks_with(&mut world, &[0]);
    assert_eq!(at, at2);
    world.order(at, COIN);
    run(&mut world, 34);
    assert_eq!(world.orders.iter().filter(|o| o.good == COIN).count(), 1);
}

// PROM T13 (MIL-PROM-3.2 clarified, E-mil-023): "no open order" is the
// building's own order list empty; a served order stays on it until its ware
// is delivered (ECO-REQ-5.3f). While its coins are on their way (with the
// warehouse helper, then on the roads) the barracks orders none at its next
// checks, although none has arrived; it orders again only when all are in.
// spec: MIL-PROM-3.2 (PROM T13), ECO-REQ-5.3f
#[test]
fn prom_t13_a_coin_on_its_way_still_counts() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0]);
    road_between(&mut world, HQ, at);
    world.warehouses.get_mut(&HQ).unwrap().stock = vec![(COIN, 10), (HELPER, 5)];
    let open = |w: &World| w.orders.iter().filter(|o| o.target.at == at).count();
    // The first check orders the stock limit's coins; then they are served.
    let mut served = false;
    for _ in 0..300 {
        world.step();
        if open(&world) == 0 && world.orders_filed_for(at) > 0 {
            served = true;
            break;
        }
    }
    assert!(served, "the coins are on their way");
    let filed = world.orders_filed_for(at);
    // The next checks (every 33 ticks) order nothing while any is on its way.
    for _ in 0..70 {
        world.step();
        if coins_in(&world, at) > 0 || world.orders_filed_for(at) < filed {
            break;
        }
        assert_eq!(open(&world), 0, "tick {}", world.tick());
    }
}

// T8
// spec: MIL-PROM-2.1, MIL-SOL-6.4 (PROM T8)
#[test]
fn prom_t8_headquarters_does_not_promote() {
    let mut world = game(60, 40, &[(P0, HQ)]);
    let stock: Vec<(u32, u32)> = RANK.iter().map(|&r| (r, 3)).collect();
    set_stock(&mut world, HQ, &stock);
    coins(&mut world, HQ, 3);
    run_to(&mut world, 310);
    assert_eq!(ranks_of(&world, HQ), [0, 1, 2, 3, 4]);
    assert_eq!(coins_in(&world, HQ), 3);
}

// T9
// spec: MIL-PROM-4.1 (PROM T9)
#[test]
fn prom_t9_promotion_ticks() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[0, 0, 0]);
    coins(&mut world, at, 5);
    let mut seen = Vec::new();
    let mut before = 5;
    for _ in 0..910 {
        world.step();
        let now = coins_in(&world, at);
        if now < before {
            seen.push(world.tick() - 1);
            before = now;
        }
    }
    assert_eq!(seen, [300, 601, 902]);
}

// T10
// spec: MIL-PROM-3.1 (PROM T10)
#[test]
fn prom_t10_coin_check_is_phased_by_the_object_number() {
    let mut world = quiet_game();
    world.next_object = 5;
    let at = barracks_with(&mut world, &[0]);
    assert_eq!(world.building_at(at).unwrap().serial, 5);
    let mut seen = Vec::new();
    while seen.len() < 3 {
        world.step();
        if !world.orders.is_empty() {
            seen.push(world.tick() - 1);
            world.orders.clear();
        }
    }
    assert_eq!(seen, [28, 61, 94]);
}

// T11: the coin goes to the building with the nearer enemy, by accumulator.
// spec: MIL-PROM-3.3 (PROM T11)
#[test]
fn prom_t11_accumulators() {
    let mut world = quiet_game();
    let a = (26, 20);
    let b = (26, 24);
    for at in [a, b] {
        finished(&mut world, P0, BARRACKS, at);
        garrison(&mut world, at, 0, 3);
    }
    for (at, n) in [(a, 10), (b, 30)] {
        let m = world.military.get_mut(&at).unwrap();
        m.enemy_distance = n;
        m.accumulator = 0.0f32.to_bits();
        world.order(at, COIN);
    }
    let acc = |w: &World, at| w.military_at(at).unwrap().accumulator();
    // 0.1 against 0.0333: the first; its accumulator is reset.
    assert_eq!(world.pick_coin_order(&[0, 1]), Some(0));
    assert_eq!(acc(&world, a), 0.0);
    assert!((acc(&world, b) - 1.0 / 30.0).abs() < 1e-6);
    assert_eq!(world.pick_coin_order(&[0, 1]), Some(0));
    assert!((acc(&world, b) - 2.0 / 30.0).abs() < 1e-6);
    // ECO-TIME-5.1: 3 x (1/30) summed in float32 is 0.10000001, above 0.1, so
    // the second building is served and reset; the first stays at 0.1.
    assert_eq!(world.pick_coin_order(&[0, 1]), Some(1));
    assert_eq!(acc(&world, b), 0.0);
    assert_eq!(acc(&world, a), 0.1f32);
}

// T12
// spec: MIL-PROM-2.1, MIL-SOL-7.2 (PROM T12)
#[test]
fn prom_t12_coin_switch_is_on_for_a_new_building() {
    let mut world = quiet_game();
    let at = barracks_with(&mut world, &[]);
    assert!(world.military_at(at).unwrap().coin_switch);
    assert!(!world.military_at(at).unwrap().move_out);
}

// The index of building fields that replaced the per-tick map pass lists
// exactly the fields the pass would find, in the same order.
// spec: none (an index of the simulation's own: the building fields kept in map order)
#[test]
fn building_index_matches_a_map_pass() {
    let mut world = owned_world(40, 40);
    let scan = |w: &World| -> Vec<u32> {
        w.buildings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.is_some())
            .map(|(i, _)| i as u32)
            .collect()
    };
    for at in [(30, 10), (10, 20), (20, 5), (15, 30)] {
        finished(&mut world, P0, HUT, at);
        assert_eq!(world.building_index, scan(&world));
    }
    world.submit(Command::RemoveBuilding {
        player: P0,
        at: (10, 20),
    });
    world.step();
    assert_eq!(world.building_index.len(), 3);
    assert_eq!(world.building_index, scan(&world));
}

// TRN-FREE-6.3, 6.3a (T45): a soldier whose walk to his building ended
// elsewhere (here on the building's flag, a road walk that ended short)
// counts as arrived there; not standing on the building's field he does
// not enter and goes home, still linked: the building (a guard house that
// wants 1) counts him and asks for nobody while he walks. He is unlinked
// when his object goes, taken into the warehouse's stock in the tick after
// his walk home ends; the first allocation pass after that files one
// soldier request.
// spec: TRN-FREE-6.3, TRN-FREE-6.3a (T45), MIL-SOL-4.1, MIL-SOL-4.2
#[test]
fn t45_a_soldier_whose_walk_ended_short_stays_linked_until_he_goes() {
    let mut world = quiet_game();
    let at = (26, 20);
    world
        .building_types
        .get_mut(&BARRACKS)
        .unwrap()
        .soldier_limit = 1;
    finished(&mut world, P0, BARRACKS, at);
    road_between(&mut world, at, HQ);
    assert_eq!(world.wanted_soldiers(at), 1);
    let target = world.target_of(at).unwrap();
    let flag = world.flag_of(at);
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(flag),
        job: SettlerJob::ToMilitary { target, health: 3 },
        timer: 0,
        carrying: None,
    });
    world.military.get_mut(&at).unwrap().soldiers.push(id);
    world.step();
    let s = world.settlers[id].as_ref().unwrap();
    assert!(
        matches!(s.job, SettlerJob::Home { .. } | SettlerJob::Leaving { .. }),
        "{:?}",
        s.job
    );
    let stock = stock_of(&world, HQ, RANK[0]);
    // While he walks home: linked, counted, nobody asked for (the
    // allocation passes every 21 ticks meanwhile).
    let mut walked = 0;
    while world.settlers[id].is_some() {
        assert!(world.military_at(at).unwrap().soldiers.contains(&id));
        assert_eq!(military_requests(&world, P0), 0);
        world.step();
        walked += 1;
        assert!(walked < 2000, "he never got home");
    }
    assert!(
        walked > 21,
        "the test needs an allocation pass on his way: {walked}"
    );
    // Gone into the stock, and unlinked with it.
    assert_eq!(stock_of(&world, HQ, RANK[0]), stock + 1);
    assert!(!world.military_at(at).unwrap().soldiers.contains(&id));
    // The first allocation pass after that files one request.
    for _ in 0..21 {
        if military_requests(&world, P0) > 0 {
            break;
        }
        world.step();
    }
    assert_eq!(military_requests(&world, P0), 1);
}

// TRN-FREE T53 (TRN-FREE-6.4b, a work walk that ends in T, E-trn-044): a
// removed site's builder whose work walk under way ends in the removal tick
// T itself stands on its position in T; the line to his field's point is
// ordered at progress 0 in T + 1 and he moves from T + 2. (A walk ending
// after T orders the line in the tick he reaches it, T34.)
// spec: TRN-FREE-6.4b (TRN-FREE T53)
#[test]
fn trn_free_t53_a_work_walk_ending_in_t_orders_the_line_in_t_plus_1() {
    use crate::movement::LatticeWalk;
    let mut world = quiet_game();
    let field = (27, 22);
    let point = grid::field_sample(field);
    let speed = world.settings.settler_speed;
    let (from, to) = ((point.0 - 4, point.1), (point.0 - 2, point.1));
    let mut walk = LatticeWalk::new(&world.terrain, speed, from, to);
    while walk.ticks_left(&world.terrain, speed) > 1 {
        walk.advance(&world.terrain, speed);
    }
    let t = world.tick();
    let through = |walk| SettlerJob::Leaving {
        step: crate::LeavingStep::Through { walk, home: false },
        at: t,
        stand: Some(from),
        health: None,
    };
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: HELPER,
        list: crate::SettlerList::Carriers,
        walker: Walker::new(field),
        job: through(walk),
        timer: 0,
        carrying: None,
    });
    let line = |w: &World| match w.settlers[id].as_ref().unwrap().job {
        SettlerJob::Leaving {
            step: crate::LeavingStep::Through { walk, home },
            ..
        } => (walk, home),
        ref j => panic!("{j:?}"),
    };
    // T: he reaches his position; no line yet.
    world.step();
    let (walk, home) = line(&world);
    assert!(walk.is_done(&world.terrain) && !home);
    assert_eq!(walk.to, to);
    // T + 1: the line to his field's point at progress 0.
    world.step();
    let (walk, home) = line(&world);
    assert!(home);
    assert_eq!((walk.from, walk.to, walk.done), (to, point, 0));
    assert_eq!(walk.progress_bits, 0);
    // T + 2: he moves.
    world.step();
    let (walk, _) = line(&world);
    assert!(walk.done > 0 || walk.progress_bits != 0);
}
