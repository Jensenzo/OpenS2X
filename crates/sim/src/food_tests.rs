//! Test cases of `specs/ECO-food-mining.md` (T1 to T13) and the rules around
//! them: mine ore search, farm, fisherman, hunter, donkey farm, shipyard.

use crate::grid::Point;
use crate::map::state;
use crate::movement::Walker;
use crate::requests::tests as req;
use crate::test_support::*;
use crate::{
    BuildingSize, BuildingType, Deposit, DepositKind, DepositState, FieldResource, GoodRules,
    ProductionRules, Roles, Settler, SettlerJob, ShipRules, Work, WorkCycle, World,
};

const PLANK: u32 = req::PLANK;
const COAL: u32 = 31;
const FISH: u32 = 40;
const BREAD: u32 = 41;
const HAM: u32 = 42;
const WATER: u32 = 44;
const CORN: u32 = 45;
const DONKEY_WARE: u32 = 46;
const BOAT: u32 = 47;
const WOOD: u32 = 6;

const MINER: u32 = 21;
const FISHERMAN: u32 = 22;
const FARMER: u32 = 23;
const HUNTSMAN: u32 = 24;
const BREEDER: u32 = 25;
const SHIPWRIGHT: u32 = 26;
const GOOD_DONKEY: u32 = 27;

const COAL_MINE: u32 = 302;
const FISHERY: u32 = 310;
const HUNTER_HUT: u32 = 306;
const FARM: u32 = 311;
const DONKEY_FARM: u32 = 312;
const SHIPYARD: u32 = 313;
const GRAIN: u32 = 500;

fn kind(
    id: u32,
    job: u32,
    product: u32,
    ticks: u32,
    cycle: WorkCycle,
    radius: u32,
) -> BuildingType {
    let mut t = BuildingType::new(id, BuildingSize::Small);
    t.worker = Some(job);
    t.product = Some(product);
    t.production_ticks = ticks;
    t.cycle = cycle;
    t.working_radius = radius;
    t
}

fn rules() -> crate::Rules {
    let mut rules = req::rules(&req::full_stock());
    let mut mine = kind(COAL_MINE, MINER, COAL, 333, WorkCycle::Mine, 3);
    mine.stock_limit = Some(6);
    let fishery = kind(FISHERY, FISHERMAN, FISH, 333, WorkCycle::Fisher, 8);
    let hunter = kind(HUNTER_HUT, HUNTSMAN, HAM, 55, WorkCycle::Hunter, 8);
    let mut farm = kind(FARM, FARMER, CORN, 83, WorkCycle::Farm, 2);
    farm.planting_good = Some(CORN);
    let mut donkeys = kind(
        DONKEY_FARM,
        BREEDER,
        DONKEY_WARE,
        333,
        WorkCycle::DonkeyFarm,
        4,
    );
    donkeys.stock_limit = Some(6);
    let mut yard = kind(SHIPYARD, SHIPWRIGHT, BOAT, 333, WorkCycle::Shipyard, 8);
    yard.second_product = Some(BOAT);
    yard.switchable = true;
    yard.stock_limit = Some(6);
    rules
        .building_types
        .extend([mine, fishery, hunter, farm, donkeys, yard]);
    let rest = |ticks: u32| GoodRules {
        rest_ticks: ticks,
        ..GoodRules::default()
    };
    for (job, ticks) in [
        (MINER, 56),
        (FISHERMAN, 1112),
        (FARMER, 56),
        (HUNTSMAN, 223),
        (BREEDER, 223),
        (SHIPWRIGHT, 223),
    ] {
        rules.goods.insert(job, rest(ticks));
    }
    let resource = GoodRules {
        resource: true,
        ..GoodRules::default()
    };
    rules.goods.insert(COAL, resource.clone());
    rules.goods.insert(FISH, resource);
    rules.goods.insert(
        CORN,
        GoodRules {
            gathered: true,
            ..GoodRules::default()
        },
    );
    let costs = |list: &[(u32, u32)]| GoodRules {
        costs: list.to_vec(),
        ..GoodRules::default()
    };
    rules
        .goods
        .insert(DONKEY_WARE, costs(&[(CORN, 1), (WATER, 1)]));
    rules.goods.insert(BOAT, costs(&[(PLANK, 1)]));
    rules.roles = Roles {
        donkey: Some(GOOD_DONKEY),
        ..rules.roles
    };
    rules.deposit_kinds.push(DepositKind {
        id: GRAIN,
        good: Some(CORN),
        next: None,
        elevation: 1,
        grow_ticks: 1067,
        blocking: true,
        life_time: Some(3000),
        needs_plantable: true,
        random_offset: false,
    });
    rules.production = ProductionRules {
        mine_food: vec![FISH, BREAD, HAM],
        ..ProductionRules::default()
    };
    rules.tree_families = vec![vec![]; 4];
    rules.ship = ShipRules {
        frame_planks: 4,
        main_planks: 8,
        construct_ticks: 55,
        ..ShipRules::default()
    };
    let _ = (WOOD, BREAD);
    rules
}

fn world() -> World {
    World::with_map(5, flat(40, 40), rules(), P0)
}

fn finished(w: &mut World, kind: u32, at: Point) {
    assert_eq!(building(w, kind, at), Ok(()));
    w.sites.remove(&at);
    w.complete(at);
}

/// A finished building with its worker inside, starting its cycle. The
/// worker's number is its settler id.
fn working(w: &mut World, kind: u32, at: Point, job: u32) -> usize {
    finished(w, kind, at);
    staff(w, at, job)
}

/// A worker inside the finished building on `at`, starting its cycle.
fn staff(w: &mut World, at: Point, job: u32) -> usize {
    let target = w.target_of(at).unwrap();
    let index = w.terrain.index(at);
    let b = w.buildings[index].as_mut().unwrap();
    b.workers = 1;
    b.active = true;
    let id = w.add_settler(Settler {
        number: 0,
        player: P0,
        profession: job,
        list: crate::SettlerList::Workers,
        walker: Walker::new(at),
        job: SettlerJob::Worker {
            target,
            work: Work::Begin,
        },
        timer: 0,
        carrying: None,
    });
    w.file_input_orders(at);
    id
}

fn work_of(w: &World, id: usize) -> Work {
    match w.settlers[id].as_ref().unwrap().job {
        SettlerJob::Worker { work, .. } => work,
        ref other => panic!("not working: {other:?}"),
    }
}

fn ore(w: &mut World, field: Point, good: u32, amount: u8) {
    w.set_resource(field, Some(FieldResource { good, amount }));
}

fn stock(w: &mut World, at: Point, list: &[(u32, u32)]) {
    w.inputs.insert(at, list.to_vec());
}

fn clear_flags(w: &mut World) {
    let lying: Vec<usize> = w.wares().map(|(i, _)| i).collect();
    for ware in lying {
        w.remove_ware(ware);
    }
}

fn made(w: &World, at: Point) -> u32 {
    w.production_at(at).unwrap().count
}

fn message_count(w: &World, at: Point) -> usize {
    w.messages()
        .iter()
        .filter(|m| m.at == at && m.text == "No resources found.")
        .count()
}

fn flag_of(w: &World, at: Point) -> Point {
    w.terrain.neighbour(at, 1).unwrap()
}

/// The goods of the wares lying on the flag of the building on `at`.
fn at_flag(w: &World, at: Point) -> Vec<u32> {
    w.wares_at(flag_of(w, at))
        .iter()
        .map(|&i| w.wares().find(|(id, _)| *id == i).unwrap().1.good)
        .collect()
}

// ECO-FOOD T1: the nearest field in flood order, not the first found.
// spec: ECO-FOOD-2.4 (T1)
#[test]
fn t1_mine_takes_the_field_first_in_flood_order() {
    let mut w = world();
    let at = (10, 10);
    ore(&mut w, (12, 10), COAL, 7);
    ore(&mut w, (10, 11), COAL, 7);
    let _ = working(&mut w, COAL_MINE, at, MINER);
    let t0 = w.tick();
    let mut found = None;
    for _ in 0..101 {
        w.step();
        if found.is_none() {
            found = w.ore_field(at).map(|f| (f, w.tick() - t0));
        }
    }
    // ECO-FOOD-2.2: the first try is in the 101st update.
    assert_eq!(found, Some(((10, 11), 101)));
}

// ECO-FOOD T2, 2.7, 2.8: seven units, seven products, then idle.
#[test]
fn t2_a_field_gives_seven_products_then_the_mine_is_idle() {
    let mut w = world();
    let at = (10, 10);
    ore(&mut w, (10, 11), COAL, 7);
    let id = working(&mut w, COAL_MINE, at, MINER);
    stock(&mut w, at, &[(FISH, 20)]);
    for _ in 0..6000 {
        w.step();
        clear_flags(&mut w);
    }
    assert_eq!(made(&w, at), 7);
    assert_eq!(w.ore_field(at), None);
    assert_eq!(w.resource_at((10, 11)), None);
    // No further cycle: the worker waits for a field (ECO-PROD-8.4).
    assert_eq!(work_of(&w, id), Work::Begin);
    for _ in 0..1000 {
        w.step();
        clear_flags(&mut w);
    }
    assert_eq!(made(&w, at), 7);
}

// ECO-FOOD T3, 2.4, 2.6: a field at distance 4 is never found.
// ECO-OUT-3.11 (T26), 3.2: the message comes once the mine's 0.5 has
// fallen below 0.5, at the 600-tick fall.
// spec: ECO-FOOD-2.4 (ECO-FOOD T3), ECO-OUT-3.11 (ECO-OUT T26)
#[test]
fn t3_a_field_out_of_reach_is_never_found() {
    let mut w = world();
    let at = (10, 10);
    ore(&mut w, (14, 10), COAL, 3);
    let _ = working(&mut w, COAL_MINE, at, MINER);
    for _ in 0..450 {
        w.step();
    }
    assert_eq!(w.ore_field(at), None);
    assert_eq!(w.resource_at((14, 10)).unwrap().amount, 3);
    // Tries in the ticks 101, 202, 303, 404: no message at 0.5.
    assert_eq!(message_count(&w, at), 0);
    for _ in 0..300 {
        w.step();
    }
    // After the fall one message, shown until it runs out (ECO-REQ-3.8).
    assert!(message_count(&w, at) >= 1);
}

// ECO-FOOD T4, 2.5: no reservation.
#[test]
fn t4_two_mines_share_one_field() {
    let mut w = world();
    let (a, b) = ((10, 10), (14, 10));
    ore(&mut w, (12, 10), COAL, 7);
    let _ = working(&mut w, COAL_MINE, a, MINER);
    let _ = working(&mut w, COAL_MINE, b, MINER);
    stock(&mut w, a, &[(FISH, 20)]);
    stock(&mut w, b, &[(FISH, 20)]);
    let mut both_had_it = false;
    for _ in 0..6000 {
        w.step();
        clear_flags(&mut w);
        both_had_it |= w.ore_field(a).is_some() && w.ore_field(b).is_some();
    }
    assert!(both_had_it, "the field is not reserved");
    assert_eq!(made(&w, a) + made(&w, b), 7);
}

// ECO-FOOD-2.3: a mine whose field still holds ore never searches, and
// a mine without its worker does not either.
#[test]
fn mine_searches_only_without_a_field_and_with_its_worker() {
    let mut w = world();
    let at = (10, 10);
    ore(&mut w, (10, 11), COAL, 7);
    ore(&mut w, (12, 10), COAL, 7);
    let _ = working(&mut w, COAL_MINE, at, MINER);
    w.ore_fields.insert(at, (12, 10));
    for _ in 0..250 {
        w.step();
    }
    assert_eq!(w.ore_field(at), Some((12, 10)));
    let mut w = world();
    ore(&mut w, (10, 11), COAL, 7);
    finished(&mut w, COAL_MINE, at);
    for _ in 0..250 {
        w.step();
    }
    assert_eq!(w.ore_field(at), None);
}

// ECO-FOOD T5, 5.1: one search every 51 ticks; reserved at the first
// search after the animal is in reach.
#[test]
fn t5_hunter_searches_every_51_ticks() {
    let mut w = world();
    let at = (10, 10);
    let _ = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let t0 = w.tick();
    let mut searches = Vec::new();
    let mut reserved = None;
    let mut animal = None;
    for _ in 0..200 {
        w.step();
        let k = w.tick() - t0;
        if w.search_timers.get(&at) == Some(&0) {
            searches.push(k);
        }
        if k == 60 {
            animal = Some(w.add_animal(1, (15, 10)));
        }
        if reserved.is_none() && w.hunted_animal(at).is_some() {
            reserved = Some(k);
        }
    }
    assert_eq!(&searches[..3], &[51, 102, 153]);
    assert_eq!(reserved, Some(102));
    assert_eq!(w.hunted_animal(at), animal);
    assert!(w.animal(animal.unwrap()).unwrap().hunted_by.is_some());
}

// ECO-FOOD-5.2: only explored fields count, and an animal another hut
// has reserved is left alone.
#[test]
fn hunter_skips_unexplored_and_hunted_animals() {
    let mut w = world();
    let at = (10, 10);
    let _ = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let far = w.add_animal(1, (16, 10));
    let index = w.terrain.index((16, 10));
    w.explored[index] = 0;
    for _ in 0..120 {
        w.step();
    }
    assert_eq!(w.hunted_animal(at), None);
    w.explored[index] = crate::territory::player_bit(P0);
    let other = w.target_of(at).map(|mut t| {
        t.serial += 100;
        t
    });
    w.animals[far].as_mut().unwrap().hunted_by = other;
    for _ in 0..120 {
        w.step();
    }
    assert_eq!(w.hunted_animal(at), None);
}

// ECO-FOOD T50 (ECO-FOOD-5.2): the exploration test is made at the field
// the animal's walk ends on (its own field while it stands), not at the
// field it is registered on.
// spec: ECO-FOOD-5.2 (ECO-FOOD T50)
#[test]
fn t50_an_animal_counts_by_the_end_of_its_walk() {
    let mut w = world();
    let at = (10, 10);
    let _ = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let (here, goal) = ((16, 10), (17, 11));
    let id = w.add_animal(1, here);
    let bit = crate::territory::player_bit(P0);
    let explore = |w: &mut World, field: Point, on: bool| {
        let index = w.terrain.index(field);
        w.explored[index] = if on { bit } else { 0 };
    };
    // Walking from an unexplored field to an explored one: found.
    explore(&mut w, here, false);
    explore(&mut w, goal, true);
    w.animals[id]
        .as_mut()
        .unwrap()
        .walker
        .walk([(17, 10), goal]);
    assert_eq!(w.find_animal(at, P0, 10), Some(id));
    // Standing on the unexplored field: not found.
    w.animals[id].as_mut().unwrap().walker = Walker::new(here);
    assert_eq!(w.find_animal(at, P0, 10), None);
    // Its field explored, its walk's end not: not found.
    explore(&mut w, here, true);
    explore(&mut w, goal, false);
    w.animals[id]
        .as_mut()
        .unwrap()
        .walker
        .walk([(17, 10), goal]);
    assert_eq!(w.find_animal(at, P0, 10), None);
}

// ECO-FOOD T59 (ECO-FOOD-5.2, 5.10a): of several unreserved animals on the
// first field the search reaches, the hut takes the one registered there
// first, not the lowest number; an animal is registered on a field when its
// step onto it completes.
// spec: ECO-FOOD-5.2 (ECO-FOOD T59), ECO-FOOD-5.10a
#[test]
fn t59_of_several_animals_on_a_field_the_first_registered() {
    let mut w = world();
    let at = (10, 10);
    let field = (16, 10);
    let low = w.add_animal(1, (17, 10));
    let high = w.add_animal(1, field);
    // The lower number walks onto the field, where the higher one stands
    // since the start.
    let a = w.animals[low].as_mut().unwrap();
    a.state = crate::hunting::AnimalState::Walking {
        since: 0,
        speed: 45_000,
        idle_thousandths: 2000,
    };
    a.walker.walk([field]);
    while w.animals[low].as_ref().unwrap().at != field {
        w.step();
        assert!(w.tick() < 200, "it arrives");
    }
    assert_eq!(w.animals[low].as_ref().unwrap().registered, w.tick());
    assert_eq!(w.animals[high].as_ref().unwrap().registered, 0);
    assert_eq!(w.find_animal(at, P0, 10), Some(high), "registered first");
    // Registered in the same tick: the one updated first, the lower id.
    w.animals[high].as_mut().unwrap().registered = w.tick();
    assert_eq!(w.find_animal(at, P0, 10), Some(low));
}

fn lake(w: &mut World) -> (Point, Point) {
    // Coast field C and its water neighbour W, 2 and 3 fields east of the
    // flag of a hut on (10, 10).
    let flag = flag_of(w, (10, 10));
    let coast = (flag.0 + 2, flag.1);
    let water = (flag.0 + 3, flag.1);
    w.set_bits(coast, state::COAST, true);
    w.set_bits(water, state::WATER, true);
    (coast, water)
}

// ECO-FOOD T6, 1.3, 4.3: no spot: the message, then one try per 100 ticks
// in the ticks with (tick + n) mod 100 = 0.
#[test]
fn t6_fisherman_retries_on_his_own_grid() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    w.step();
    assert_eq!(work_of(&w, id), Work::FishRetry);
    assert_eq!(message_count(&w, at), 1);
    let (_, water) = lake(&mut w);
    ore(&mut w, water, FISH, 3);
    let mut started = None;
    for _ in 0..250 {
        let before = w.tick();
        w.step();
        if started.is_none() && matches!(work_of(&w, id), Work::OutNext { .. } | Work::Out { .. }) {
            started = Some(before);
        }
    }
    let started = started.expect("a trip");
    assert_eq!((started + id as u64) % 100, 0);
    assert!(started < w.tick() - 150 || started <= 101 + id as u64);
}

// ECO-FOOD T7, 4.2 to 4.6: the trip: the fish unit goes, a fish lies at
// the flag, the next trip needs another candidate. ECO-FOOD T51 (4.1a): the
// start tick only chooses the spot, he stands; the walk out comes a tick
// later.
// spec: ECO-FOOD-4.2 (ECO-FOOD T7), ECO-FOOD-4.1a (ECO-FOOD T51)
#[test]
fn t7_fisherman_one_trip() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (coast, water) = lake(&mut w);
    ore(&mut w, water, FISH, 1);
    // Candidates: only the Coast field; the fish field is the water next
    // to it (ECO-FOOD-4.3).
    let flag = flag_of(&w, at);
    assert_eq!(w.fishing_spots(flag, FISH, 8), vec![coast]);
    w.step();
    assert!(matches!(work_of(&w, id), Work::OutNext { spot } if spot == coast));
    assert!(
        !w.settlers[id].as_ref().unwrap().walker.is_walking(),
        "he stands"
    );
    assert_eq!(w.fish_field(at), Some(water));
    w.step();
    assert!(matches!(work_of(&w, id), Work::Out { spot: Some(s) } if s == coast));
    let mut stood = None;
    let mut action = 0;
    let mut fish_at_flag = false;
    let mut new_trips = 0;
    // To his next trip's start, the message (4.1a, 4.6a: a tick each).
    for _ in 0..1700 {
        w.step();
        match work_of(&w, id) {
            Work::AtSpot { .. } => {
                action += 1;
                stood = Some(w.settlers[id].as_ref().unwrap().walker.at);
            }
            Work::Out { spot: Some(_) } if w.resource_at(water).is_none() => new_trips += 1,
            _ => {}
        }
        fish_at_flag |= at_flag(&w, at) == [FISH];
    }
    assert_eq!(stood, Some(coast));
    assert_eq!(action, 333, "ECO-FOOD-4.4: the production time");
    assert_eq!(w.resource_at(water), None);
    assert!(fish_at_flag);
    assert_eq!(new_trips, 0, "no candidate is left");
    assert!(message_count(&w, at) >= 1);
    assert_eq!(
        made(&w, at),
        0,
        "a fisherman's catch is not a product count"
    );
}

// ECO-FOOD T52 (ECO-FOOD-4.6a): after the take the fisherman stands one tick
// on the spot; his walk back begins in the next tick.
// spec: ECO-FOOD-4.6a (ECO-FOOD T52), ECO-OUT-3.8a
#[test]
fn t52_the_fisherman_stands_a_tick_after_the_take() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (_, water) = lake(&mut w);
    ore(&mut w, water, FISH, 3);
    let mut took = false;
    for _ in 0..800 {
        let was = work_of(&w, id);
        w.step();
        if matches!(was, Work::TakeWare { second: true, .. })
            && !matches!(work_of(&w, id), Work::TakeWare { .. })
        {
            assert!(
                matches!(work_of(&w, id), Work::StandBack { .. }),
                "{:?}",
                work_of(&w, id)
            );
            assert!(
                !w.settlers[id].as_ref().unwrap().walker.is_walking(),
                "he stands"
            );
            w.step();
            assert!(
                w.settlers[id].as_ref().unwrap().walker.has_walk(),
                "walking back"
            );
            took = true;
            break;
        }
    }
    assert!(took, "a take");
}

// ECO-FOOD T53 (ECO-FOOD-4.6): the fisherman's next trip starts in the tick
// his rest is over (the spot chosen, he stands for the walk out of the next
// tick), with no tick between the rest and the start.
// spec: ECO-FOOD-4.6 (ECO-FOOD T53), ECO-OUT-3.10
#[test]
fn t53_the_next_trip_starts_in_the_tick_the_rest_ends() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (_, water) = lake(&mut w);
    ore(&mut w, water, FISH, 3);
    let mut seen = false;
    for _ in 0..2000 {
        let was = work_of(&w, id);
        w.step();
        if was == Work::Rest && work_of(&w, id) != Work::Rest {
            assert!(
                matches!(work_of(&w, id), Work::OutNext { .. }),
                "{:?}",
                work_of(&w, id)
            );
            seen = true;
            break;
        }
    }
    assert!(seen, "a rest ended");
}

// ECO-FOOD T8, T29, 4.5, 4.6: the fish field was emptied meanwhile: the
// unit comes from the first field around him (none within reach here), and
// the fish is made all the same.
#[test]
fn t8_a_fish_is_made_when_the_field_was_emptied_meanwhile() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (coast, water) = lake(&mut w);
    ore(&mut w, water, FISH, 1);
    // Fish out of reach of the catch: 2 fields from where he stands.
    let decoy = (coast.0 + 2, coast.1 + 1);
    ore(&mut w, decoy, FISH, 4);
    let mut emptied = false;
    let mut fish_at_flag = false;
    let mut rested = false;
    for _ in 0..1600 {
        w.step();
        if !emptied && matches!(work_of(&w, id), Work::AtSpot { .. }) {
            w.set_resource(water, None);
            emptied = true;
        }
        fish_at_flag |= at_flag(&w, at) == [FISH];
        rested |= emptied && work_of(&w, id) == Work::Rest;
    }
    assert!(emptied);
    assert!(fish_at_flag, "ECO-FOOD-4.6: the fish is made in every case");
    assert!(rested);
    assert_eq!(w.resource_at(decoy).unwrap().amount, 4);
    assert_eq!(w.fish_field(at), None);
}

// ECO-FOOD T63 (ECO-FOOD-4.5, corrected): the hut's remembered field
// emptied by another hut during the catch gives nothing: no unit is taken
// from the fish around him, the hut forgets the field, the fish is still
// made (4.6). Only a hut that remembers no field takes from the first field
// holding fish within the fishing distance of where he stands.
// spec: ECO-FOOD-4.5 (T63), ECO-FOOD-4.6
#[test]
fn t63_an_emptied_remembered_field_gives_nothing() {
    let run = |forget: bool| {
        let mut w = world();
        let at = (10, 10);
        let id = working(&mut w, FISHERY, at, FISHERMAN);
        let (coast, water) = lake(&mut w);
        ore(&mut w, water, FISH, 1);
        // Another field next to his coast spot, within the fishing distance.
        let near = (0..6)
            .filter_map(|d| w.terrain.neighbour(coast, d))
            .find(|&f| f != water)
            .unwrap();
        let mut emptied = false;
        let mut fish_at_flag = false;
        for _ in 0..800 {
            w.step();
            if !emptied && matches!(work_of(&w, id), Work::AtSpot { .. }) {
                w.set_resource(water, None);
                ore(&mut w, near, FISH, 3);
                if forget {
                    w.fish_fields.remove(&at);
                }
                emptied = true;
            }
            if at_flag(&w, at) == [FISH] {
                fish_at_flag = true;
                break;
            }
        }
        assert!(emptied);
        assert!(fish_at_flag, "ECO-FOOD-4.6: the fish is made in every case");
        // At the first fish's put-down, before a second trip.
        (w.resource_at(near).unwrap().amount, w.fish_field(at))
    };
    // Remembered, emptied: nothing taken, forgotten.
    assert_eq!(run(false), (3, None));
    // Nothing remembered: the first fish around him gives the unit.
    assert_eq!(run(true).0, 2);
}

// ECO-FOOD-4.5: a field with several units gives one per trip and stays
// the hut's fish field.
#[test]
fn fish_field_keeps_its_remaining_units() {
    let mut w = world();
    let at = (10, 10);
    let _ = working(&mut w, FISHERY, at, FISHERMAN);
    let (_, water) = lake(&mut w);
    ore(&mut w, water, FISH, 3);
    for _ in 0..450 {
        w.step();
        clear_flags(&mut w);
    }
    assert_eq!(w.resource_at(water).unwrap().amount, 2);
    assert_eq!(w.fish_field(at), Some(water));
}

fn grain(w: &mut World, field: Point, ripe_at: u64, life_left: Option<u32>) {
    w.add_deposit(
        field,
        Deposit {
            kind: GRAIN,
            position: crate::grid::field_sample(field),
            state: DepositState::Growing { ripe_at },
            reserved_by: None,
            life_left,
        },
    );
    w.changed(&[field]);
}

// ECO-FOOD T9, 3.3: sowing, 1067 ticks to ripe, then a harvest trip.
// ECO-FOOD-1.2 (ECO-OUT T26): the sowing does not raise the farm's
// productivity, the harvest does.
// spec: ECO-FOOD-3.3 (ECO-FOOD T9), ECO-FOOD-1.2 (ECO-OUT T26)
#[test]
fn t9_farm_sows_and_later_harvests() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FARM, at, FARMER);
    let mut sown = None;
    let mut first_field = None;
    let mut corn = false;
    let productivity = |w: &World| w.building_at(at).unwrap().productivity;
    let mut raises = Vec::new();
    for _ in 0..4000 {
        let before = w.tick();
        let was = productivity(&w);
        w.step();
        if productivity(&w) > was {
            raises.push(before);
        }
        clear_flags_keep(&mut w, &mut corn);
        if sown.is_none() {
            if let Some((f, d)) = w.deposits().next() {
                if let DepositState::Growing { ripe_at } = d.state {
                    assert_eq!(ripe_at, before + 1067, "ripe 1067 ticks after sowing");
                    // The deposit update of the same tick counted once.
                    assert_eq!(d.life_left, Some(2999));
                    assert!(d.reserved_by.is_none());
                    sown = Some(before);
                    first_field = Some(f);
                }
            }
        }
    }
    assert!(sown.is_some());
    assert!(w.terrain.has(first_field.unwrap(), state::DEPOSIT_BLOCKED) || corn);
    assert!(corn, "a harvest trip brought corn to the flag");
    assert!(!raises.contains(&sown.unwrap()), "not raised by the sowing");
    assert!(!raises.is_empty(), "the harvest raised it");
    let _ = id;
}

// ECO-OUT T19 (ECO-OUT-9.1): when the sowing ends the planting step makes
// the field's deposit once more, its lattice direction drawn: one draw in
// that tick, no object number, the field the same.
// spec: ECO-OUT-9.1 (ECO-OUT T19), TRN-FREE-5.4
#[test]
fn eco_out_t19_the_sowing_end_draws_once() {
    let mut w = world();
    let id = working(&mut w, FARM, (10, 10), FARMER);
    let _ = id;
    for _ in 0..2000 {
        let (draws, next) = (w.rng.draws(), w.next_object());
        let before: Vec<Point> = w.deposits().map(|(f, _)| f).collect();
        w.step();
        let sown = w
            .deposits()
            .find(|(_, d)| matches!(d.state, DepositState::Growing { .. }));
        if let Some((field, _)) = sown {
            assert!(before.contains(&field), "the same field");
            assert_eq!(w.rng.draws() - draws, 1, "one draw");
            assert_eq!(w.next_object(), next, "no number");
            return;
        }
    }
    panic!("no sowing ended");
}

// GFX-ANIM-3.8: the farmer sowing (the spot's field of size 0, placed for
// the sowing) plays WorkLoop1 at the spot; harvesting a grown field plays
// WorkLoop0.
// spec: GFX-ANIM-3.8, ECO-FOOD-3.3
#[test]
fn the_farmer_sows_with_work_loop_1_and_harvests_with_work_loop_0() {
    use crate::trace::Action;
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FARM, at, FARMER);
    let mut corn = false;
    let mut seen = Vec::new();
    for _ in 0..4000 {
        w.step();
        clear_flags_keep(&mut w, &mut corn);
        let number = w.settlers[id].as_ref().unwrap().number.to_string();
        let action = w
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == number)
            .map(|r| r.action);
        if let Some(a @ Action::WorkLoop(_)) = action {
            if seen.last() != Some(&a) {
                seen.push(a);
            }
        }
    }
    assert!(corn, "a harvest came");
    assert_eq!(seen.first(), Some(&Action::WorkLoop(1)), "sowing first");
    assert!(
        seen.contains(&Action::WorkLoop(0)),
        "then a harvest: {seen:?}"
    );
}

/// Remove the wares at the flags, remembering whether corn was among them.
fn clear_flags_keep(w: &mut World, corn: &mut bool) {
    for (_, ware) in w.wares() {
        *corn |= ware.good == CORN;
    }
    clear_flags(w);
}

// ECO-FOOD T10, 3.2: a reserved ripe field: the harvest takes the
// production time 83; the field is removed 18 ticks after. ECO-OUT T21:
// he leaves after the 51-tick target wait.
// spec: ECO-OUT-9.1 (ECO-OUT T21)
#[test]
fn t10_farm_harvests_a_reserved_field() {
    let mut w = world();
    let at = (10, 10);
    let field = (11, 10);
    let id = working(&mut w, FARM, at, FARMER);
    grain(&mut w, field, 0, None);
    let target = w.target_of(at).unwrap();
    w.deposits.get_mut(&field).unwrap().reserved_by = Some(target);
    w.targets.insert(at, field);
    let mut at_spot = 0;
    let mut harvest = None;
    let mut gone = None;
    let mut waited = 0;
    for _ in 0..320 {
        let before = w.tick();
        w.step();
        match work_of(&w, id) {
            Work::WaitTarget { .. } => waited += 1,
            Work::AtSpot { .. } if gone.is_none() => at_spot += 1,
            _ => {}
        }
        match w.deposit(field).map(|d| d.state) {
            Some(DepositState::Removing { .. }) if harvest.is_none() => harvest = Some(before),
            None if harvest.is_some() && gone.is_none() => gone = Some(before),
            _ => {}
        }
        clear_flags(&mut w);
    }
    assert_eq!(
        waited, 51,
        "ECO-OUT-9.1, 10 (T21): the 51-tick target wait before the harvest trip"
    );
    assert_eq!(at_spot, 83);
    assert_eq!(gone.unwrap() - harvest.unwrap(), 18);
    assert!(
        !w.terrain.has(field, state::BLOCKED),
        "ECO-OUT-1.5: the field is free again"
    );
}

// ECO-FOOD-3.3: grain fields block their field while they stand.
#[test]
fn grain_field_blocks_its_field() {
    let mut w = world();
    let field = (12, 12);
    grain(&mut w, field, 0, None);
    assert!(w.terrain.has(field, state::BLOCKED));
    assert!(w.terrain.has(field, state::DEPOSIT_BLOCKED));
}

// ECO-FOOD-3.4, ECO-OUT-6.5: an unharvested field withers after its
// lifetime; a reserved one does not count down.
#[test]
fn grain_field_withers_unless_reserved() {
    let mut w = world();
    grain(&mut w, (12, 12), 0, Some(3000));
    grain(&mut w, (16, 16), 0, Some(3000));
    let target = crate::Target {
        at: (1, 1),
        serial: 0,
        storage: false,
    };
    w.deposits.get_mut(&(16, 16)).unwrap().reserved_by = Some(target);
    let mut removing = None;
    for _ in 0..3100 {
        let before = w.tick();
        w.step();
        if removing.is_none()
            && matches!(
                w.deposit((12, 12)).map(|d| d.state),
                Some(DepositState::Removing { .. })
            )
        {
            removing = Some(before);
        }
    }
    assert_eq!(removing, Some(2999));
    assert!(w.deposit((12, 12)).is_none());
    assert_eq!(w.deposit((16, 16)).unwrap().life_left, Some(3000));
}

// ECO-FOOD T11, T58, 6.2: a donkey settler at the door, no ware; in the
// donkey's tick the breeder goes back to waiting, the next production starts
// in the tick after (with corn and water at hand): a cycle of 335 ticks.
// spec: ECO-FOOD-6.2 (ECO-FOOD T11, T58)
#[test]
fn t11_donkey_farm_makes_a_settler() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, DONKEY_FARM, at, BREEDER);
    stock(&mut w, at, &[(CORN, 2), (WATER, 2)]);
    let settlers = |w: &World| {
        w.settlers()
            .filter(|(_, s)| s.profession == GOOD_DONKEY)
            .count()
    };
    let mut ended = None;
    for _ in 0..400 {
        w.step();
        if work_of(&w, id) == Work::Donkey {
            ended = Some(w.tick());
            break;
        }
    }
    let ended = ended.expect("a production");
    assert_eq!(settlers(&w), 0);
    assert_eq!(w.inputs(at), [(CORN, 1), (WATER, 1)]);
    w.step();
    assert_eq!(w.tick(), ended + 1);
    assert_eq!(settlers(&w), 1, "created in the next tick");
    assert_eq!(
        work_of(&w, id),
        Work::WaitInputs,
        "he waits in the donkey's tick"
    );
    w.step();
    assert_eq!(work_of(&w, id), Work::Working, "the next production starts");
    assert!(at_flag(&w, at).is_empty());
    let donkey = w
        .settlers()
        .find(|(_, s)| s.profession == GOOD_DONKEY)
        .unwrap()
        .1;
    assert_eq!(donkey.walker.at, at);
    // 333 ticks later again, with no rest in between.
    let mut again = None;
    for _ in 0..400 {
        w.step();
        if work_of(&w, id) == Work::Donkey {
            again = Some(w.tick());
            break;
        }
    }
    assert_eq!(again, Some(ended + 2 + 333));
    assert_eq!(made(&w, at), 0, "no product is taken");
}

// ECO-FOOD T11a, 6.3: the new donkey walks out to the farm's flag and goes
// home to the headquarters, whose stock it joins.
#[test]
fn t11a_donkey_walks_home() {
    let setup = crate::GameSetup {
        players: vec![(P0, req::HQ)],
        local_player: P0,
        ..Default::default()
    };
    let mut w = World::new_game(9, flat(40, 40), rules(), &setup);
    let at = (16, 12);
    let flag = (16, 11);
    req::apply_all(&mut w, vec![req::place(DONKEY_FARM, at)]);
    req::apply_all(&mut w, vec![req::road_cmd(req::HQ_FLAG, flag)]);
    w.sites.remove(&at);
    w.complete(at);
    let id = staff(&mut w, at, BREEDER);
    stock(&mut w, at, &[(CORN, 1), (WATER, 1)]);
    let before = w.warehouse(req::HQ).unwrap().count(GOOD_DONKEY);
    let mut seen_at_flag = false;
    for _ in 0..1500 {
        w.step();
        clear_flags(&mut w);
        seen_at_flag |= w
            .settlers()
            .any(|(_, s)| s.profession == GOOD_DONKEY && s.walker.at == flag);
    }
    let _ = id;
    assert!(seen_at_flag, "out to the farm's flag first");
    assert_eq!(w.warehouse(req::HQ).unwrap().count(GOOD_DONKEY), before + 1);
    assert!(w.settlers().all(|(_, s)| s.profession != GOOD_DONKEY));
}

// ECO-WORK T-door-5 (3.8a, 3.8b), TRN-FREE T40 (6.4a), ECO-FOOD-6.2: the
// donkey breeding has a door field other than its own. The new donkey
// appears on the door field and steps onto the building's field as a walk of
// its own, then to the flag; it is sent home on arriving there (T): it stands
// on the flag in T, T + 1 and T + 2, and its walk home starts at progress 0
// in T + 3.
// spec: ECO-WORK-3.8a, ECO-WORK-3.8b (ECO-WORK T-door-5), TRN-FREE-6.4a (TRN-FREE T40), ECO-FOOD-6.2
#[test]
fn t_door_5_the_new_donkey_leaves_by_the_door_field() {
    let setup = crate::GameSetup {
        players: vec![(P0, req::HQ)],
        local_player: P0,
        ..Default::default()
    };
    let mut rules = rules();
    for kind in rules.building_types.iter_mut() {
        if kind.id == DONKEY_FARM {
            kind.doors = [Some(5); 3];
        }
    }
    let mut w = World::new_game(9, flat(40, 40), rules, &setup);
    let at = (16, 12);
    let flag = (16, 11);
    let door = w.terrain.neighbour(at, 5).unwrap();
    req::apply_all(&mut w, vec![req::place(DONKEY_FARM, at)]);
    req::apply_all(&mut w, vec![req::road_cmd(req::HQ_FLAG, flag)]);
    w.sites.remove(&at);
    w.complete(at);
    staff(&mut w, at, BREEDER);
    stock(&mut w, at, &[(CORN, 1), (WATER, 1)]);
    let donkey = |w: &World| {
        w.settlers()
            .find(|(_, s)| s.profession == GOOD_DONKEY)
            .map(|(_, s)| s.walker.clone())
    };
    let mut first = None;
    while first.is_none() {
        w.step();
        clear_flags(&mut w);
        first = donkey(&w);
        assert!(w.tick() < 1000, "a donkey is made");
    }
    // Made in this tick: on the door field, stepping onto the building's
    // field from progress 0, then to the flag.
    let walker = first.unwrap();
    assert_eq!(walker.at, door, "{walker:?}");
    let walk: Vec<Point> = walker
        .next
        .into_iter()
        .chain(walker.path.iter().copied())
        .collect();
    assert_eq!(walk, [at, flag], "{walker:?}");
    assert_eq!(walker.pauses, [at], "{walker:?}");
    // On the building's field, the step to the flag starts at progress 0.
    // On the flag (T) it stands three ticks; its walk home starts in T + 3.
    let mut on_field = false;
    let mut rows = Vec::new();
    for _ in 0..200 {
        w.step();
        clear_flags(&mut w);
        let Some(walker) = donkey(&w) else { break };
        if !on_field && walker.at == at {
            assert_eq!(walker.next, Some(flag));
            assert_eq!(walker.progress(), 0.0, "{walker:?}");
            on_field = true;
        }
        if walker.at == flag || !rows.is_empty() {
            rows.push((walker.at, walker.is_walking(), walker.progress()));
        }
        if rows.len() == 5 {
            break;
        }
    }
    assert!(on_field, "on the field");
    let standing = (flag, false, 0.0);
    assert_eq!(rows[..3], [standing, standing, standing], "{rows:?}");
    assert_eq!(rows[3], (flag, true, 0.0), "{rows:?}");
    assert!(rows[4].2 > 0.0, "{rows:?}");
}

fn coast_site(w: &mut World) -> Point {
    let site = (14, 10);
    w.set_bits(site, state::COAST, true);
    w.set_bits((15, 10), state::WATER, true);
    site
}

// ECO-FOOD T60 (ECO-FOOD-7.4a, 7.4b): the work parts are f32 values lowered
// once a tick of the step; the frame's last decrement takes a residue, so
// three of trip 4's 1/220 go to the main part, which holds 42/440
// (0.0954554) when the 12th step starts. That step's 43rd decrement empties
// it: the ship is finished and starts (SHIP-2.1) in that tick, 13 ticks
// before the step ends.
// spec: ECO-FOOD-7.4b (ECO-FOOD T60), ECO-FOOD-7.4a, SHIP-2.1
#[test]
fn t60_the_ship_is_finished_partway_through_the_last_trip() {
    let mut w = world();
    let at = (10, 10);
    working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    coast_site(&mut w);
    stock(&mut w, at, &[(PLANK, 12)]);
    assert_eq!(w.toggle_switch(P0, at), Ok(()));
    let (mut trips, mut stepping, mut main) = (0, false, 1.0f32);
    let (mut start, mut before, mut finished, mut end) = (None, None, None, None);
    for _ in 0..9000 {
        w.step();
        clear_flags(&mut w);
        let tick = w.tick() - 1;
        let Some(ship) = w.ships().first() else {
            continue;
        };
        if ship.step_amount.is_some() && !stepping {
            trips += 1;
            if trips == 12 {
                (start, before) = (Some(tick), Some(main));
            }
        }
        if ship.step_amount.is_none() && stepping && trips == 12 {
            end = Some(tick);
        }
        stepping = ship.step_amount.is_some();
        if ship.finished() && finished.is_none() {
            assert!(ship.vessel.launched, "it starts in that tick");
            finished = Some(tick);
        }
        main = f32::from_bits(ship.main_work);
    }
    let before = before.expect("a 12th step");
    assert!((before - 42.0 / 440.0).abs() < 1e-6, "{before}");
    let start = start.unwrap();
    assert_eq!(finished.map(|f| f - start), Some(42), "the 43rd decrement");
    assert_eq!(end.map(|e| e - start), Some(55), "the step's 56th");
}

// ECO-FOOD T12, 7.1, 7.3, 7.4: ship mode: 12 trips of one plank each;
// 7.4b: then "Ship constructed" at the ship's water field.
// spec: ECO-FOOD-7.4b (ECO-FOOD T23)
#[test]
fn t12_shipyard_builds_a_ship_in_twelve_trips() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    let site = coast_site(&mut w);
    stock(&mut w, at, &[(PLANK, 12)]);
    assert_eq!(w.toggle_switch(P0, at), Ok(()));
    let mut action = 0;
    let mut first_check = None;
    let t0 = w.tick();
    // The trip (step) in which each part is emptied (ECO-FOOD-7.4a).
    let (mut trips, mut stepping) = (0, false);
    let (mut frame_done, mut main_done) = (None, None);
    for _ in 0..9000 {
        w.step();
        clear_flags(&mut w);
        match work_of(&w, id) {
            Work::AtSpot { .. } => action += 1,
            Work::Out { spot: Some(_) } if first_check.is_none() => {
                first_check = Some(w.tick() - t0)
            }
            _ => {}
        }
        if let Some(ship) = w.ships().first() {
            if ship.step_amount.is_some() && !stepping {
                trips += 1;
            }
            stepping = ship.step_amount.is_some();
            if ship.frame_work == 0 && frame_done.is_none() {
                frame_done = Some(trips);
            }
            if ship.main_work == 0 && main_done.is_none() {
                main_done = Some(trips);
            }
        }
    }
    assert_eq!(
        first_check,
        Some(102),
        "the check is 101 ticks after the start"
    );
    assert_eq!((frame_done, main_done, trips), (Some(4), Some(12), 12));
    assert_eq!(w.ships().len(), 1);
    assert_eq!(w.ships()[0].at, site);
    assert!(w.ships()[0].finished());
    assert_eq!(action, 12 * 55, "ECO-FOOD-7.4: the construct time");
    assert_eq!(w.inputs(at).iter().map(|&(_, n)| n).sum::<u32>(), 0);
    // ECO-FOOD-7.4b: "Ship constructed" at the ship's water field.
    let water = w.ships()[0].water.expect("a water field");
    assert!(w
        .messages()
        .iter()
        .any(|m| m.at == water && m.text == "Ship constructed"));
}

// ECO-FOOD T54 (7.3): in ship mode the step starts only while production is
// on: switched off, the shipwright stays in his mode check, trying every
// tick. A check that finds no ship (no coast here) returns him to the mode
// check for one tick: the checks come 102 ticks apart.
// spec: ECO-FOOD-7.3 (ECO-FOOD T54)
#[test]
fn t54_the_ship_step_starts_only_while_production_is_on() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    stock(&mut w, at, &[(PLANK, 2)]);
    assert_eq!(w.toggle_switch(P0, at), Ok(()));
    let index = w.terrain.index(at);
    w.buildings[index].as_mut().unwrap().production.enabled = false;
    for _ in 0..300 {
        w.step();
        assert_eq!(work_of(&w, id), Work::Begin, "off: the mode check");
    }
    w.buildings[index].as_mut().unwrap().production.enabled = true;
    let mut checks = Vec::new();
    let mut last = work_of(&w, id);
    for _ in 0..500 {
        w.step();
        let now = work_of(&w, id);
        if now == Work::Begin && matches!(last, Work::WaitShip { .. }) {
            checks.push(w.tick());
        }
        last = now;
    }
    assert!(checks.len() >= 3, "{checks:?}");
    assert!(checks.windows(2).all(|p| p[1] - p[0] == 102), "{checks:?}");
}

// ECO-FOOD T61 (7.3b): a launched (finished) ship no longer holds its water
// field: the next ship can be started on the same building field; an
// unfinished one there keeps it.
// spec: ECO-FOOD-7.3b (ECO-FOOD T61)
#[test]
fn t61_a_launched_ship_frees_its_water_field() {
    let mut w = world();
    let at = (10, 10);
    working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    let site = coast_site(&mut w);
    let water = (15, 10);
    assert_eq!(w.ship_site(at, 8), Some((site, water)));
    w.ships.push(crate::Ship {
        owner: P0,
        at: site,
        water: Some(water),
        frame_work: 0,
        main_work: 0,
        step_amount: None,
        vessel: crate::navy::Vessel::new(site),
        name: None,
    });
    assert_eq!(w.ship_site(at, 8), Some((site, water)), "finished: free");
    w.ships[0].main_work = 1.0f32.to_bits();
    assert_eq!(w.ship_site(at, 8), None, "under construction: held");
}

// ECO-FOOD T55, T56, T57 (7.3b, 7.4): the ship is started on the coast
// field with its water field; the trip's plank takes an object number after
// the ship's; at the ship the shipwright puts the plank down (the drop for
// 4 ticks, the pick-up motion for 4), stands, works the step's 56 ticks and
// walks back: 66 ticks from his arrival.
// spec: ECO-FOOD-7.3b (ECO-FOOD T57), ECO-FOOD-7.4 (ECO-FOOD T55, T56), SIM-START-4.2
#[test]
fn t55_t56_a_trip_to_the_ship() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    let site = coast_site(&mut w);
    stock(&mut w, at, &[(PLANK, 12)]);
    assert_eq!(w.toggle_switch(P0, at), Ok(()));
    let mut numbers = None;
    let mut phases: Vec<(Work, u32)> = Vec::new();
    for _ in 0..2000 {
        let before = w.next_object;
        let was = work_of(&w, id);
        w.step();
        clear_flags(&mut w);
        let now = work_of(&w, id);
        if numbers.is_none()
            && matches!(now, Work::Out { spot: Some(_) })
            && !matches!(was, Work::Out { .. })
        {
            numbers = Some(w.next_object - before);
        }
        let key = match now {
            Work::AtShip { .. } | Work::AtSpot { .. } | Work::Back { .. } => now,
            _ => continue,
        };
        match phases.last_mut() {
            Some((k, n)) if *k == key => *n += 1,
            _ => phases.push((key, 1)),
        }
        if matches!(key, Work::Back { .. }) {
            break;
        }
    }
    assert_eq!(numbers, Some(2), "the ship's number, then the plank's");
    let ship = &w.ships()[0];
    assert_eq!((ship.at, ship.water), (site, Some((15, 10))));
    let spot = site;
    assert_eq!(
        phases[..4],
        [
            (Work::AtShip { spot, phase: 0 }, 4),
            (Work::AtShip { spot, phase: 1 }, 4),
            (Work::AtShip { spot, phase: 2 }, 2),
            // The 56th tick of the step orders the walk back.
            (Work::AtSpot { spot }, 55),
        ],
        "{phases:?}"
    );
    assert!(matches!(phases[4].0, Work::Back { .. }));
}

// ECO-FOOD-7.2: switched off (boat mode): the workshop cycle.
#[test]
fn shipyard_makes_boats_unless_switched() {
    let mut w = world();
    let at = (10, 10);
    let _ = working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    let _ = coast_site(&mut w);
    stock(&mut w, at, &[(PLANK, 1)]);
    let mut boat = false;
    for _ in 0..600 {
        w.step();
        boat |= at_flag(&w, at) == [BOAT];
    }
    assert!(boat);
    assert!(w.ships().is_empty());
}

// ECO-FOOD-7.3: ship mode without planks makes no ship site.
#[test]
fn shipyard_without_planks_starts_no_ship() {
    let mut w = world();
    let at = (10, 10);
    let _ = working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    let _ = coast_site(&mut w);
    assert_eq!(w.toggle_switch(P0, at), Ok(()));
    for _ in 0..500 {
        w.step();
    }
    assert!(w.ships().is_empty());
}

// ECO-FOOD T13, 5.4: the animal is gone on arrival.
#[test]
fn t13_hunter_still_brings_a_ham_when_the_animal_is_gone() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let animal = w.add_animal(1, (14, 10));
    let mut removed = false;
    let mut acted = false;
    let mut ham = false;
    for _ in 0..800 {
        w.step();
        if !removed && matches!(work_of(&w, id), Work::Out { .. } | Work::ToSpot { .. }) {
            w.remove_animal(animal);
            removed = true;
        }
        acted |= removed && matches!(work_of(&w, id), Work::AtSpot { .. });
        ham |= at_flag(&w, at) == [HAM];
    }
    assert!(removed);
    assert!(!acted, "no action");
    assert!(ham, "a ham is carried home");
    assert!(w.animal(animal).is_none());
}

// ECO-FOOD-5.3, 5.4: an animal that is there: 55 ticks, killed, ham.
#[test]
fn hunter_kills_and_brings_a_ham() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let animal = w.add_animal(1, (14, 10));
    let mut action = 0;
    let mut ham = false;
    for _ in 0..800 {
        w.step();
        if matches!(work_of(&w, id), Work::AtSpot { .. }) {
            action += 1;
            assert!(w.animal(animal).is_some());
        }
        ham |= at_flag(&w, at) == [HAM];
    }
    assert_eq!(action, 55);
    assert!(w.animal(animal).is_none());
    assert_eq!(w.hunted_animal(at), None);
    assert!(ham);
    // ECO-OUT-3.11 (T26): 0.5 from the placement, 0.25 for the product,
    // minus 0.05 per 600 ticks.
    assert_eq!(w.building_at(at).unwrap().productivity, 70);
}

// ECO-FOOD-1.3, ECO-PROD-1.2: a switched-off farm, fisherman and hunter
// do not start a trip.
#[test]
fn switched_off_outdoor_buildings_stay_inside() {
    let mut w = world();
    let (a, b, c) = ((10, 10), (16, 10), (22, 10));
    for (kind, at) in [(FARM, a), (FISHERY, b), (HUNTER_HUT, c)] {
        finished(&mut w, kind, at);
        assert_eq!(w.toggle_production(P0, at), Ok(()));
    }
    let farm = staff(&mut w, a, FARMER);
    let hut = staff(&mut w, b, FISHERMAN);
    let hunter = staff(&mut w, c, HUNTSMAN);
    let (_, water) = {
        let flag = flag_of(&w, b);
        let coast = (flag.0 + 2, flag.1);
        let water = (flag.0 + 3, flag.1);
        w.set_bits(coast, state::COAST, true);
        w.set_bits(water, state::WATER, true);
        ore(&mut w, water, FISH, 3);
        (coast, water)
    };
    w.add_animal(1, (c.0 + 3, c.1));
    for _ in 0..300 {
        w.step();
        assert_eq!(work_of(&w, farm), Work::Begin);
        assert!(matches!(work_of(&w, hut), Work::FishRetry));
        assert_eq!(work_of(&w, hunter), Work::Begin);
    }
    assert_eq!(w.resource_at(water).unwrap().amount, 3);
}

/// The worker's steps over `ticks` ticks, each with the tick it began in;
/// the wares at the flags are removed each tick.
fn trace(w: &mut World, id: usize, ticks: u32) -> Vec<(u64, Work)> {
    let mut changes = Vec::new();
    let mut last = work_of(w, id);
    for _ in 0..ticks {
        w.step();
        let now = work_of(w, id);
        if now != last {
            changes.push((w.tick(), now));
        }
        last = now;
        clear_flags(w);
    }
    changes
}

/// Door to door: for each time the door closes behind the worker (he steps
/// in), the ticks until it opens again (he walks out).
fn inside_times(changes: &[(u64, Work)]) -> Vec<u64> {
    let mut times = Vec::new();
    for (i, &(closed, work)) in changes.iter().enumerate() {
        if work != Work::StepIn {
            continue;
        }
        if let Some(&(opened, _)) = changes[i..]
            .iter()
            .find(|(_, w)| matches!(w, Work::Out { .. }))
        {
            times.push(opened - closed);
        }
    }
    times
}

// ECO-OUT-8.1, 10, 3.10: the hunter's door to door is the woodcutter's 275
// ticks (step in 1, rest 223, the 51-tick check from the rest's last tick);
// his free walk stops one field short of the animal, and a lattice walk
// (Approach) comes before the shot.
// spec: ECO-OUT-8.1, ECO-OUT-10, ECO-OUT-3.10
#[test]
fn hunter_door_to_door() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let animals = [(15, 10), (16, 12)];
    let ids: Vec<usize> = animals.iter().map(|&f| w.add_animal(1, f)).collect();
    let mut stood = Vec::new();
    let mut changes = Vec::new();
    let mut last = work_of(&w, id);
    for _ in 0..3000 {
        w.step();
        let now = work_of(&w, id);
        if now != last {
            changes.push((w.tick(), now));
            if let Work::AtSpot { spot } = now {
                assert!(matches!(last, Work::Approach { .. }), "{last:?}");
                stood.push((w.settlers[id].as_ref().unwrap().walker.at, spot));
            }
        }
        last = now;
        clear_flags(&mut w);
    }
    assert_eq!(stood.len(), 2, "{changes:?}");
    for &(field, animal) in &stood {
        assert_eq!(crate::grid::distance(field, animal), 1, "one field short");
    }
    assert!(ids.iter().all(|&a| w.animal(a).is_none()));
    let inside = inside_times(&changes);
    assert!(!inside.is_empty());
    assert!(inside.iter().all(|&t| t == 275), "{inside:?}");
}

// ECO-FOOD-4.4, ECO-OUT-7.1, 10: the fisherman stands on the Coast spot,
// fishes 333 ticks, and starts his next trip 1113 ticks after stepping in
// (step in 1, rest 1112, the trip's start in the rest's last tick, T53),
// with no 51-tick wait; ECO-FOOD-4.1a (T51): his walk out a tick after the
// start, 1114 ticks after stepping in.
// spec: ECO-FOOD-4.4, ECO-FOOD-4.6 (ECO-FOOD T53), ECO-FOOD-4.1a (ECO-FOOD T51)
#[test]
fn fisher_door_to_door() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (coast, water) = lake(&mut w);
    ore(&mut w, water, FISH, 3);
    let mut fished = 0;
    for _ in 0..1600 {
        w.step();
        if matches!(work_of(&w, id), Work::AtSpot { .. }) {
            let field = w.settlers[id].as_ref().unwrap().walker.at;
            assert_eq!(field, coast);
            fished += 1;
        }
        clear_flags(&mut w);
    }
    assert_eq!(fished, 333);
    let mut w = world();
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (_, water) = lake(&mut w);
    ore(&mut w, water, FISH, 3);
    let inside = inside_times(&trace(&mut w, id, 4500));
    assert!(inside.len() >= 2, "{inside:?}");
    assert!(inside.iter().all(|&t| t == 1114), "{inside:?}");
}

// ECO-OUT-9.1, 10, ECO-FOOD-3.2a, ECO-OUT T15, T20, T21: every sowing is
// one outing: 51 ticks inside (the sowing step's 50-tick wait, then the
// search), out, sow, back, in; after a harvest or a sowing he is inside
// 57 ticks (the step in, the 56-tick rest, and he decides in the tick it is
// over), + 51 for the spot search before he leaves to sow (T20: 108) or
// for the target wait before he leaves to harvest (T21: 108).
// spec: ECO-OUT-9.1, ECO-OUT-10 (ECO-OUT T15, T20, T21), ECO-FOOD-3.2a
#[test]
fn farmer_door_to_door() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FARM, at, FARMER);
    let changes = trace(&mut w, id, 1500);
    let sown = changes
        .iter()
        .filter(|(_, w)| matches!(w, Work::AtSpot { .. }))
        .count();
    let left: Vec<u64> = changes
        .iter()
        .filter(|(_, w)| matches!(w, Work::Out { .. }))
        .map(|&(t, _)| t)
        .collect();
    assert!(sown >= 3, "{changes:?}");
    // An outing still on its way when the trace ends has not sown yet.
    let last_sown = changes
        .iter()
        .rev()
        .find(|(_, w)| matches!(w, Work::AtSpot { .. }))
        .map_or(0, |&(t, _)| t);
    let finished = left.iter().filter(|&&t| t < last_sown).count();
    assert_eq!(finished, sown, "one outing per sowing: {changes:?}");
    // Inside before an outing that sows: 57 + 51 (the spot search); before
    // one that harvests (to a field an earlier outing sowed; the fields
    // sown first ripen within the trace now that the walk home has its
    // lattice part, TRN-FREE-5.6), 57 + 51 too (the target wait, T21).
    let mut checked = 0;
    let mut sown_spots = Vec::new();
    for (i, &(closed, work)) in changes.iter().enumerate() {
        if let Work::Out { spot: Some(spot) } = work {
            sown_spots.push(spot);
        }
        if work != Work::StepIn {
            continue;
        }
        let Some(j) = changes[i..]
            .iter()
            .position(|(_, w)| matches!(w, Work::Out { .. }))
        else {
            continue;
        };
        let (opened, out) = changes[i + j];
        let harvests = matches!(out, Work::Out { spot: Some(s) } if sown_spots.contains(&s));
        assert_eq!(opened - closed, 57 + 51, "harvest {harvests}: {changes:?}");
        checked += 1;
    }
    assert!(checked >= 2, "{changes:?}");
    // Harvest: a reserved ripe field: out after the target wait.
    let mut w = world();
    let field = (11, 10);
    let id = working(&mut w, FARM, at, FARMER);
    grain(&mut w, field, 0, None);
    let target = w.target_of(at).unwrap();
    w.deposits.get_mut(&field).unwrap().reserved_by = Some(target);
    w.targets.insert(at, field);
    let changes = trace(&mut w, id, 400);
    let closed = changes
        .iter()
        .find(|(_, w)| *w == Work::StepIn)
        .expect("home after the harvest")
        .0;
    let opened = changes
        .iter()
        .find(|(t, w)| *t > closed && matches!(w, Work::Out { .. }))
        .expect("out again to sow")
        .0;
    assert_eq!(opened - closed, 57 + 51, "{changes:?}");
}

// ECO-FOOD T30, 3.2a: no ripe field and no free spot ever: one search per
// 51 ticks, and he never leaves.
#[test]
fn t30_farmer_without_a_spot_searches_every_51_ticks() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FARM, at, FARMER);
    // No plantable field within the radius.
    for f in crate::grid::area(at, 4, w.terrain.width, w.terrain.height) {
        if f != at && f != flag_of(&w, at) {
            w.set_bits(f, state::BLOCKED, true);
        }
    }
    let mut searches = Vec::new();
    for _ in 0..400 {
        let before = work_of(&w, id);
        w.step();
        if before == (Work::WaitSpot { countdown: 0 }) {
            searches.push(w.tick());
        }
        assert!(!matches!(work_of(&w, id), Work::Out { .. }));
    }
    assert!(searches.len() >= 5, "{searches:?}");
    assert!(
        searches.windows(2).all(|p| p[1] - p[0] == 51),
        "{searches:?}"
    );
}

// ECO-FOOD T49, 3.2a (corrected): a sowing search that finishes with no
// spot (the fields around are walkable but take no grain) ends the sowing
// step in its own tick; the farmer decides in the next, so the searches
// come 52 ticks apart (51 with the decision at once).
// spec: ECO-FOOD-3.2a (ECO-FOOD T49)
#[test]
fn t49_the_farmer_decides_in_the_tick_after_a_failed_search() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FARM, at, FARMER);
    for f in crate::grid::area(at, 8, w.terrain.width, w.terrain.height) {
        if f != at && f != flag_of(&w, at) {
            w.set_bits(f, state::DEPOSIT_BLOCKED, true);
        }
    }
    let mut searches = Vec::new();
    for _ in 0..400 {
        let before = work_of(&w, id);
        w.step();
        if before == (Work::WaitSpot { countdown: 0 }) {
            assert_eq!(work_of(&w, id), Work::Begin, "the decision waits a tick");
            searches.push(w.tick());
        }
        assert!(!matches!(work_of(&w, id), Work::Out { .. }));
    }
    assert!(searches.len() >= 5, "{searches:?}");
    assert!(
        searches.windows(2).all(|p| p[1] - p[0] == 52),
        "{searches:?}"
    );
}

// ECO-OUT T25 (ECO-OUT-8.1): the hunter's walks aim at the field the
// reserved animal stands on when each walk starts, not the field it had
// when the hut reserved it.
// spec: ECO-OUT-8.1 (ECO-OUT T25)
#[test]
fn t25_the_hunter_aims_at_the_animals_field_now() {
    let mut w = world();
    let hut = (10, 10);
    let animal = w.add_animal(1, (14, 8));
    w.hunted.insert(hut, animal);
    assert_eq!(w.hunted_field(hut, (14, 8)), (14, 8));
    w.animals[animal].as_mut().unwrap().at = (15, 8);
    assert_eq!(w.hunted_field(hut, (14, 8)), (15, 8), "where it stands now");
    w.hunted.remove(&hut);
    assert_eq!(w.hunted_field(hut, (14, 8)), (14, 8));
}

// ECO-OUT T27 (ECO-OUT-8.1, corrected): the free walk aims at the reserved
// animal's walk goal when it starts (the last field of its current walk,
// its own field while it stands); the lattice walk at its position (the
// start field of a step in progress).
// spec: ECO-OUT-8.1 (ECO-OUT T27, T25)
#[test]
fn t27_the_free_walk_aims_at_the_animals_walk_goal() {
    let mut w = world();
    let hut = (10, 10);
    let animal = w.add_animal(1, (14, 8));
    w.hunted.insert(hut, animal);
    assert_eq!(w.hunted_goal(hut, (14, 8)), (14, 8), "standing: its field");
    let a = w.animals[animal].as_mut().unwrap();
    a.walker.walk([(15, 8), (16, 8)]);
    assert_eq!(w.hunted_goal(hut, (14, 8)), (16, 8), "walking: its goal");
    assert_eq!(w.hunted_field(hut, (14, 8)), (14, 8), "its position");
    // In its last step: the step's field is the goal.
    let a = w.animals[animal].as_mut().unwrap();
    a.walker.path.clear();
    a.walker.next = Some((15, 8));
    assert_eq!(w.hunted_goal(hut, (14, 8)), (15, 8));
    assert_eq!(w.hunted_field(hut, (14, 8)), (14, 8));
}

// ECO-FOOD-5.16 (3ede553): a new building removes the animals registered
// on its footprint, without a draw: a large building's field and its west,
// north-west and north-east neighbours; an animal on the flag's field or
// beside the footprint stays. A small building: its field only.
// spec: ECO-FOOD-5.16, ECO-PLACE-5.1
#[test]
fn a_new_building_removes_the_animals_on_its_footprint() {
    let mut w = world();
    let mut large = w.building_types[&FARM].clone();
    large.size = BuildingSize::Large;
    w.building_types.insert(FARM, large);
    let at = (20, 20);
    let t = &w.terrain;
    let footprint: Vec<Point> = [at]
        .into_iter()
        .chain([3, 4, 5].map(|d| t.neighbour(at, d).unwrap()))
        .collect();
    let flag = t.neighbour(at, 1).unwrap();
    let beside = t.neighbour(at, 0).unwrap();
    let on: Vec<usize> = footprint.iter().map(|&f| w.add_animal(1, f)).collect();
    let keep: Vec<usize> = [flag, beside].iter().map(|&f| w.add_animal(1, f)).collect();
    let draws = w.rng().draws();
    assert_eq!(building(&mut w, FARM, at), Ok(()));
    assert!(on.iter().all(|&a| w.animal(a).is_none()));
    assert!(keep.iter().all(|&a| w.animal(a).is_some()));
    assert_eq!(w.rng().draws(), draws, "no draw");
    let small_at = (30, 20);
    let near = w.terrain.neighbour(small_at, 3).unwrap();
    let (on_field, next_to) = (w.add_animal(1, small_at), w.add_animal(1, near));
    assert_eq!(building(&mut w, COAL_MINE, small_at), Ok(()));
    assert!(w.animal(on_field).is_none() && w.animal(next_to).is_some());
}

// FMT-SAV-3.5a (b, 2026-11-09 b, Q-257): a fisherman of a save fishing on
// his coast spot (state 33, 40 ticks of the action left) goes on there; the
// hut's fish field is the first fish around the spot, as at his trip's
// start (ECO-FOOD-4.1); in the 40th tick after the load the catch takes a
// unit from it (4.5) and the fish is made (4.6).
// spec: FMT-SAV-3.5a, ECO-FOOD-4.5
#[test]
fn a_fisherman_of_a_save_goes_on_fishing() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (coast, water) = lake(&mut w);
    ore(&mut w, water, FISH, 2);
    let saved = crate::RestoredOutdoor {
        at,
        field: coast,
        spot: coast,
        state: 33,
        left: Some(40),
        walk: None,
        rough: None,
        animal: None,
        leg: None,
    };
    assert_eq!(w.resume_outdoor(&saved), Some(true));
    assert_eq!(w.fish_field(at), Some(water));
    assert_eq!(work_of(&w, id), Work::AtSpot { spot: coast });
    assert_eq!(w.settlers[id].as_ref().unwrap().walker.at, coast);
    for _ in 1..40 {
        w.step();
        assert_eq!(work_of(&w, id), Work::AtSpot { spot: coast });
    }
    w.step();
    assert!(matches!(work_of(&w, id), Work::TakeWare { .. }));
    assert_eq!(w.resource_at(water).map(|r| r.amount), Some(1));
    assert_eq!(w.settlers[id].as_ref().unwrap().carrying, Some(FISH));
}

// FMT-SAV-3.5a (b, 2026-11-09 b, Q-257): a shipwright of a save at his ship
// (state 34, 10 ticks of the work action left) goes on with his work step:
// its amount is 1 / (construct time x main planks) with the frame done
// (ECO-FOOD-7.4a), a decrement in each of the next 9 ticks, the step's end
// in the 9th and his walk back in the 10th.
// spec: FMT-SAV-3.5a, ECO-FOOD-7.4a
#[test]
fn a_shipwright_of_a_save_goes_on_with_his_step() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, SHIPYARD, at, SHIPWRIGHT);
    let site = coast_site(&mut w);
    let main = 0.5f32;
    w.ships.push(crate::Ship {
        owner: P0,
        at: site,
        water: Some((15, 10)),
        frame_work: 0,
        main_work: main.to_bits(),
        step_amount: None,
        vessel: crate::navy::Vessel::new(site),
        name: None,
    });
    let saved = crate::RestoredOutdoor {
        at,
        field: site,
        spot: site,
        state: 34,
        left: Some(10),
        walk: None,
        rough: None,
        animal: None,
        leg: None,
    };
    assert_eq!(w.resume_outdoor(&saved), Some(true));
    let amount = 1.0f32 / (55 * 8) as f32;
    assert_eq!(w.ships[0].step_amount, Some(amount.to_bits()));
    let mut expected = main;
    for k in 1..=9 {
        w.step();
        expected -= amount;
        assert_eq!(f32::from_bits(w.ships[0].main_work), expected, "tick {k}");
        assert_eq!(work_of(&w, id), Work::AtSpot { spot: site });
    }
    assert_eq!(w.ships[0].step_amount, None, "the step ended in the 9th");
    w.step();
    assert_eq!(f32::from_bits(w.ships[0].main_work), expected);
    assert!(
        matches!(work_of(&w, id), Work::Back { .. }),
        "{:?}",
        work_of(&w, id)
    );
}

// FMT-SAV-3.5a (b, 2026-11-09 c, from code, no sample), ECO-OUT-8.1: a
// hunter of a save at work at his animal (hunt step 10, 20 ticks of the
// work left) goes on with it, the animal (his hut's reserved target) held
// for the hut and standing; when the work ends it is gone and he takes the
// ham. One on the lattice walk to it (hunt step 9, walk step 1) goes on from
// the saved step and progress, stopping 2 short of the animal's point, and
// then works. Without his animal he is seated.
// spec: FMT-SAV-3.5a, ECO-OUT-8.1
#[test]
fn a_hunter_of_a_save_goes_on_with_his_hunt() {
    let at = (10, 10);
    let animal_field = (15, 10);
    let field = (12, 10);
    let saved = |animal: usize, left: Option<u32>, walk: Option<crate::RestoredFineWalk>| {
        crate::RestoredOutdoor {
            at,
            field,
            spot: animal_field,
            state: 33,
            left,
            walk,
            rough: None,
            animal: Some(animal),
            leg: None,
        }
    };
    // Step 10: working.
    let mut w = world();
    let id = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let animal = w.add_animal(1, animal_field);
    assert_eq!(w.resume_outdoor(&saved(animal, Some(20), None)), Some(true));
    assert_eq!(w.hunted_animal(at), Some(animal));
    assert_eq!(w.animal(animal).unwrap().state, crate::AnimalState::Hunted);
    assert_eq!(work_of(&w, id), Work::AtSpot { spot: animal_field });
    for _ in 1..20 {
        w.step();
        assert_eq!(work_of(&w, id), Work::AtSpot { spot: animal_field });
        assert!(w.animal(animal).is_some());
    }
    w.step();
    assert!(matches!(work_of(&w, id), Work::TakeWare { .. }));
    assert!(w.animal(animal).is_none(), "killed");
    assert_eq!(w.settlers[id].as_ref().unwrap().carrying, Some(HAM));
    // Step 9, walk step 1: on the lattice walk.
    let mut w = world();
    let id = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    let animal = w.add_animal(1, animal_field);
    let from = crate::grid::field_sample(field);
    let point = crate::grid::field_sample(animal_field);
    let line = crate::movement::fine_approach(&w.terrain, from, point, 0);
    let steps = crate::movement::fine_approach(&w.terrain, from, point, 2);
    assert_eq!(steps.len() + 2, line.len());
    let speed = w.settings.settler_speed;
    let total = crate::movement::lattice_walk_ticks(&w.terrain, speed, from, &steps);
    let (_, next, progress) = crate::movement::lattice_walk_at(&w.terrain, speed, from, &steps, 15);
    let next = next.expect("walking");
    let j = line.iter().position(|&p| p == next).expect("on the line");
    let mut path: Vec<_> = line.iter().rev().copied().collect();
    path.push(from);
    let walk = crate::RestoredFineWalk {
        path,
        index: (line.len() - 1 - j) as u32,
        progress,
        rate: 0.09,
    };
    assert_eq!(
        w.resume_outdoor(&saved(animal, None, Some(walk))),
        Some(false)
    );
    assert!(
        matches!(work_of(&w, id), Work::Approach { spot, .. } if spot == animal_field),
        "{:?}",
        work_of(&w, id)
    );
    assert_eq!(w.settlers[id].as_ref().unwrap().timer, total - 15);
    for _ in 0..total - 15 {
        w.step();
    }
    assert_eq!(work_of(&w, id), Work::AtSpot { spot: animal_field });
    // Without his animal: seated.
    let mut w = world();
    let id = working(&mut w, HUNTER_HUT, at, HUNTSMAN);
    assert_eq!(w.resume_outdoor(&saved(7, Some(20), None)), None);
    assert_eq!(work_of(&w, id), Work::Begin);
}

// FMT-SAV-3.5a (b, 2026-11-09 c), ECO-FOOD-4.4: a fisherman of a save on the
// rough part of his walk out to his coast spot (stop-short 0) walks on onto
// the spot itself, the hut's fish field the first fish around it as at his
// trip's start, and fishes there.
// spec: FMT-SAV-3.5a, ECO-FOOD-4.4
#[test]
fn a_fisherman_of_a_save_walks_on_to_his_coast_spot() {
    let mut w = world();
    let at = (10, 10);
    let id = working(&mut w, FISHERY, at, FISHERMAN);
    let (coast, water) = lake(&mut w);
    ore(&mut w, water, FISH, 2);
    let flag = flag_of(&w, at);
    let middle = (flag.0 + 1, flag.1);
    let saved = crate::RestoredOutdoor {
        at,
        field: flag,
        spot: coast,
        state: 33,
        left: None,
        walk: None,
        rough: Some(crate::RestoredRoughWalk {
            path: vec![coast, middle, flag],
            index: 1,
            progress: 0.5,
            rate: 0.045,
            stop_short: 0,
        }),
        animal: None,
        leg: None,
    };
    assert_eq!(w.resume_outdoor(&saved), Some(false));
    assert_eq!(w.fish_field(at), Some(water));
    assert_eq!(work_of(&w, id), Work::ToSpot { spot: coast });
    let mut ticks = 0;
    while work_of(&w, id) == (Work::ToSpot { spot: coast }) {
        w.step();
        ticks += 1;
        assert!(ticks < 100, "never there");
    }
    assert_eq!(work_of(&w, id), Work::AtSpot { spot: coast });
    assert_eq!(w.settlers[id].as_ref().unwrap().walker.at, coast);
}
