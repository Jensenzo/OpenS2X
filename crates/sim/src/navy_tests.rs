//! Tests of `specs/SHIP-ships-expeditions.md`.
//!
//! The map is 40 x 30 of building ground with a sea between x = 14 and
//! x = 25; the land at 9..=13 and 26..=30 is harbour ground. Player 0 owns
//! and has explored everything unless a test says otherwise.

use std::collections::BTreeMap;

use crate::grid::{self, Point};
use crate::map::{state, Terrain};
use crate::military_tests::{road_between, run};
use crate::movement;
use crate::navy::{ExpeditionState, ShipState, Vessel};
use crate::test_support::{apply, flat, set, world, HARBOR, P0};
use crate::{
    BuildingSize, Command, GoodKind, GoodRules, PlayerId, Rejection, Requester, Ship, World,
};

const PLANK: u32 = 0x10;
const STONE: u32 = 0x12;
const BUILDER: u32 = 0x20;
const HELPER: u32 = 0x21;
const WAREHOUSE: u32 = 6;
const P1: PlayerId = PlayerId(1);

const WEST_HARBOUR: Point = (11, 10);
const EAST_HARBOUR: Point = (28, 10);

fn sea_terrain() -> Terrain {
    let mut terrain = flat(40, 30);
    for field in terrain.fields().collect::<Vec<_>>() {
        let (x, _) = field;
        if (14..=25).contains(&x) {
            set(&mut terrain, field, state::WATER);
        } else if (9..=13).contains(&x) || (26..=30).contains(&x) {
            set(&mut terrain, field, state::FOR_BUILDING | state::FOR_HARBOR);
        }
    }
    terrain
}

fn sea_world() -> World {
    let mut w = world(sea_terrain());
    let building_types = [
        crate::BuildingType {
            frame_costs: vec![(PLANK, 4)],
            costs: vec![(STONE, 6)],
            ..crate::BuildingType::new(HARBOR, BuildingSize::Harbor)
        },
        crate::BuildingType {
            warehouse: true,
            ..crate::BuildingType::new(WAREHOUSE, BuildingSize::Large)
        },
    ];
    for t in building_types {
        w.building_types.insert(t.id, t);
    }
    w.priorities = BTreeMap::from([(PLANK, 7), (STONE, 8)]);
    w.roles.builder = Some(BUILDER);
    w.roles.helper = Some(HELPER);
    for settler in [BUILDER, HELPER] {
        w.goods.insert(
            settler,
            GoodRules {
                kind: GoodKind::Settler,
                ..GoodRules::default()
            },
        );
    }
    w.ship_rules.speed = 180_000;
    w.ship_rules.stock_limit = 12;
    w
}

fn harbour(w: &mut World, player: PlayerId, at: Point) {
    w.put_building(player, HARBOR, BuildingSize::Harbor, at, false);
    w.complete(at);
}

fn warehouse(w: &mut World, at: Point) {
    w.put_building(P0, WAREHOUSE, BuildingSize::Large, at, false);
    w.complete(at);
}

/// A finished ship of `owner` on the water field `at`.
fn ship(w: &mut World, owner: PlayerId, at: Point) -> usize {
    w.ships.push(Ship {
        owner,
        at,
        water: None,
        frame_work: 0,
        main_work: 0,
        step_amount: None,
        name: None,
        vessel: Vessel::new(at),
    });
    w.ships.len() - 1
}

fn stock(w: &mut World, at: Point, good: u32, n: u32) {
    w.warehouses.get_mut(&at).expect("a warehouse").add(good, n);
}

fn count(w: &World, at: Point, good: u32) -> u32 {
    w.warehouses[&at].count(good)
}

fn state_of(w: &World, ship: usize) -> ShipState {
    w.ships()[ship].vessel.state
}

fn step_until(w: &mut World, limit: u32, done: impl Fn(&World) -> bool) -> u32 {
    for n in 0..limit {
        if done(w) {
            return n;
        }
        w.step();
    }
    panic!("not reached in {limit} ticks");
}

// SHIP-1.2, T3: docking places of a harbour whose ring has water at the
// positions 1, 2, 3: positions 1 and 3 only.
#[test]
fn ship_t3_docking_places_do_not_touch() {
    let mut terrain = flat(40, 30);
    let at = (20, 15);
    let ring = grid::ring(at, 3, 40, 30);
    for &f in &ring[0..3] {
        set(&mut terrain, f, state::WATER);
    }
    let mut w = world(terrain);
    harbour(&mut w, P0, at);
    let docks: Vec<Point> = w
        .harbour_at(at)
        .unwrap()
        .docks
        .iter()
        .map(|d| d.field)
        .collect();
    assert_eq!(docks, [ring[0], ring[2]]);
    assert!(w.terrain().has(ring[0], state::LANDING_POSITION));
    assert!(!w.terrain().has(ring[1], state::LANDING_POSITION));
    // A second harbour next to it makes no docking place touching these.
    assert!(w.warehouses.contains_key(&at), "a harbour is a warehouse");
}

// SHIP-1.2: water that is DoodadBlocked, or already a landing position, or
// next to one, is no docking place.
#[test]
fn ship_docks_skip_blocked_water() {
    let mut terrain = flat(40, 30);
    let at = (20, 15);
    let ring = grid::ring(at, 3, 40, 30);
    for &f in &ring[0..2] {
        set(&mut terrain, f, state::WATER | state::DOODAD_BLOCKED);
    }
    set(&mut terrain, ring[5], state::WATER);
    let mut w = world(terrain);
    harbour(&mut w, P0, at);
    let docks: Vec<Point> = w
        .harbour_at(at)
        .unwrap()
        .docks
        .iter()
        .map(|d| d.field)
        .collect();
    assert_eq!(docks, [ring[5]]);
}

// T1: the ship starts Idle, searches, and docks at the harbour. SHIP-2.7
// (T17): its route start cleared its last port position; docked, it is the
// harbour's own field, not the docking place.
// spec: SHIP-2.1, SHIP-2.3, SHIP-2.4, SHIP-2.5, SHIP-2.6, SHIP-2.7 (T1, T17)
#[test]
fn ship_t01_launch_search_dock() {
    let mut w = sea_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    let s = ship(&mut w, P0, (20, 10));
    w.step();
    // The first update: Idle, then search.
    assert_eq!(state_of(&w, s), ShipState::Search);
    // The first try comes 101 updates after the first.
    for n in 0..100 {
        w.step();
        assert_eq!(state_of(&w, s), ShipState::Search, "update {n}");
    }
    w.step();
    assert_eq!(state_of(&w, s), ShipState::ToHarbour);
    let v = &w.ships()[s].vessel;
    assert_eq!(v.harbour, Some(WEST_HARBOUR));
    assert_eq!(v.port, None, "cleared at the route start");
    let dock = w.harbour_at(WEST_HARBOUR).unwrap().docks[0].clone();
    assert_eq!(dock.reserved, Some(s));
    assert!(w.terrain().has(dock.field, state::LANDING_POSITION));
    step_until(&mut w, 2000, |w| state_of(w, s) == ShipState::Docked);
    let dock = &w.harbour_at(WEST_HARBOUR).unwrap().docks[0];
    assert_eq!(dock.docked, Some(s));
    assert_eq!(dock.reserved, None);
    assert_eq!(w.ships()[s].vessel.walker.at, dock.field);
    assert_ne!(dock.field, WEST_HARBOUR);
    assert_eq!(w.ships()[s].vessel.port, Some(WEST_HARBOUR));
    w.step();
    assert_eq!(state_of(&w, s), ShipState::Idle);
}

// T25 (SHIP-2.3 5, SHIP-2.4a, Q-1074): a ship docked at its harbour with
// nothing to do lies idle there (no search, its counter kept); a second ship
// called to that harbour finds no free docking place and pushes the idle
// one out: its counter the recorded 90, its link and berth dropped; it
// then searches and tries in the twelfth update after its turn.
// spec: SHIP-2.3, SHIP-2.4, SHIP-2.4a (T25)
#[test]
fn ship_t25_an_idle_ship_stays_until_pushed_out() {
    let mut w = sea_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    let s = ship(&mut w, P0, (20, 10));
    step_until(&mut w, 2000, |w| state_of(w, s) == ShipState::Docked);
    w.step();
    assert_eq!(state_of(&w, s), ShipState::Idle);
    let counter = w.ships()[s].vessel.counter;
    // Idle for longer than a search's period, its counter as it was.
    for _ in 0..300 {
        w.step();
        assert_eq!(state_of(&w, s), ShipState::Idle);
    }
    assert_eq!(w.ships()[s].vessel.counter, counter);
    assert_eq!(w.ships()[s].vessel.harbour, Some(WEST_HARBOUR));
    // Every other docking place taken: the idle ship's is the only one.
    let berth = w
        .harbour_at(WEST_HARBOUR)
        .unwrap()
        .docks
        .iter()
        .position(|d| d.docked == Some(s))
        .expect("docked");
    for (k, d) in w
        .harbours
        .iter_mut()
        .find(|h| h.at == WEST_HARBOUR)
        .unwrap()
        .docks
        .iter_mut()
        .enumerate()
    {
        if k != berth {
            d.reserved = Some(99);
        }
    }
    let caller = ship(&mut w, P0, (22, 12));
    w.ships[caller].vessel.state = ShipState::Idle;
    w.ships[caller].vessel.called = Some(WEST_HARBOUR);
    // Its first update launches it; the call is taken in its idle update.
    step_until(&mut w, 10, |w| state_of(w, caller) == ShipState::ToHarbour);
    let pushed = w.ships()[s].vessel.clone();
    assert_eq!(pushed.harbour, None);
    let dock = &w.harbour_at(WEST_HARBOUR).unwrap().docks[berth];
    assert_eq!(dock.docked, None);
    assert_eq!(dock.reserved, Some(caller));
    assert_eq!(state_of(&w, caller), ShipState::ToHarbour);
    // The recorded 90 is ours' 89 (SHIP-2.4, the counter as an
    // implementation sees it, (iv)): the next update turns it to searching,
    // and it tries in the twelfth update after that one.
    assert_eq!(pushed.counter, 89);
    w.step();
    assert_eq!(state_of(&w, s), ShipState::Search);
    for _ in 0..11 {
        w.step();
        assert_eq!(state_of(&w, s), ShipState::Search);
    }
    assert_eq!(w.ships()[s].vessel.counter, 100);
    w.step();
    assert_eq!(
        w.ships()[s].vessel.counter,
        0,
        "the try in the twelfth update"
    );
}

// SHIP-2.4: a ship picks the nearest harbour, the first in the list on a tie.
#[test]
fn ship_search_takes_the_nearest_harbour() {
    let mut w = sea_world();
    harbour(&mut w, P0, EAST_HARBOUR);
    harbour(&mut w, P0, WEST_HARBOUR);
    let s = ship(&mut w, P0, (16, 10));
    step_until(&mut w, 400, |w| state_of(w, s) == ShipState::ToHarbour);
    assert_eq!(w.ships()[s].vessel.harbour, Some(WEST_HARBOUR));
}

// T2: a ship standing on a docking place with no harbour of its own to go
// to moves to a free neighbour.
// spec: SHIP-3.3 (T2)
#[test]
fn ship_t02_making_room() {
    let mut w = sea_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    let dock = w.harbour_at(WEST_HARBOUR).unwrap().docks[0].field;
    let s = ship(&mut w, P1, dock);
    step_until(&mut w, 400, |w| state_of(w, s) == ShipState::MakingRoom);
    let to = w.ships()[s].vessel.walker.destination();
    assert_ne!(to, dock);
    assert!(grid::distance(dock, to) == 1);
    assert!(w.terrain().has(to, state::WATER));
    assert!(!w.terrain().has(to, state::LANDING_POSITION));
    step_until(&mut w, 400, |w| state_of(w, s) == ShipState::Search);
    assert_eq!(w.ships()[s].vessel.walker.at, to);
}

// SHIP-3.4: a moving ship explores radius 3 around fields not yet explored.
#[test]
fn ship_explores_while_sailing() {
    let mut w = sea_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    for e in w.explored.iter_mut() {
        *e = 0;
    }
    let s = ship(&mut w, P0, (24, 10));
    step_until(&mut w, 600, |w| state_of(w, s) == ShipState::ToHarbour);
    run(&mut w, 60);
    let now = w.ships()[s].vessel.walker.at;
    assert_ne!(now, (24, 10));
    assert!(w.is_explored(P0, now));
    assert!(w.is_explored(P0, (now.0, now.1 + 3)));
    assert!(!w.is_explored(P0, (24, 20)));
}

fn gather_world() -> (World, usize) {
    let mut w = sea_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    let s = ship(&mut w, P0, (20, 10));
    (w, s)
}

// T4: preparing an expedition with 2 planks, 6 stones and no builder.
// spec: SHIP-5.1, SHIP-5.2 (T4)
#[test]
fn ship_t04_gathering() {
    let (mut w, _) = gather_world();
    stock(&mut w, WEST_HARBOUR, PLANK, 2);
    stock(&mut w, WEST_HARBOUR, STONE, 6);
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStart {
                player: P0,
                at: WEST_HARBOUR
            }
        ),
        Ok(())
    );
    // Starting again changes nothing.
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStart {
                player: P0,
                at: WEST_HARBOUR
            }
        ),
        Ok(())
    );
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStart {
                player: P1,
                at: WEST_HARBOUR
            }
        ),
        Err(Rejection::NotAHarbour)
    );
    // Three commands took the ticks 0 to 2; the pass is in the 101st update
    // of the expedition, tick 100.
    run(&mut w, 97);
    assert!(w.orders().is_empty());
    run(&mut w, 4);
    let h = w.harbour_at(WEST_HARBOUR).unwrap();
    assert_eq!(h.expedition, ExpeditionState::Gathering);
    assert_eq!(h.aside, [(PLANK, 2), (STONE, 6)]);
    assert_eq!(count(&w, WEST_HARBOUR, PLANK), 0);
    let planks = w.orders().iter().filter(|o| o.good == PLANK).count();
    assert_eq!(planks, 2);
    let builders = w
        .requests(P0)
        .iter()
        .filter(|r| matches!(r.requester, Requester::Harbour(_)) && r.profession == BUILDER)
        .count();
    assert_eq!(builders, 1);
    // The next pass changes neither the orders nor the request.
    run(&mut w, 101);
    assert_eq!(w.orders().iter().filter(|o| o.good == PLANK).count(), 2);
    assert_eq!(w.requests(P0).len(), 1);
}

// SHIP T14 (SHIP-5.2, Q-385): the gathering pass counts all the harbour's
// orders of a good, the served ones whose ware is on its way too, also in a
// warehouse helper's hands; only open ones are cancelled. One of the two
// plank orders served (its plank still with the helper at the door): the
// next pass files none (four missing: three open, one on its way); a
// storage trip to the harbour does not count (Q-297).
// spec: SHIP-5.2 (SHIP T14)
#[test]
fn ship_t14_served_orders_on_their_way_count() {
    let (mut w, _) = gather_world();
    stock(&mut w, WEST_HARBOUR, STONE, 6);
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStart {
                player: P0,
                at: WEST_HARBOUR
            }
        ),
        Ok(())
    );
    run(&mut w, 101);
    let planks = |w: &World| w.orders().iter().filter(|o| o.good == PLANK).count();
    assert_eq!(planks(&w), 4, "the first pass: four missing");
    // One is served: its order goes with the helper carrying the plank out.
    let i = w.orders.iter().position(|o| o.good == PLANK).unwrap();
    let order = w.orders.remove(i);
    let warehouse = w.target_of(WEST_HARBOUR).unwrap();
    let number = w.new_number();
    w.add_settler(crate::Settler {
        number,
        player: P0,
        profession: w.roles.helper.unwrap_or(0),
        list: crate::SettlerList::Carriers,
        walker: crate::movement::Walker::new(w.door_field(WEST_HARBOUR)),
        job: crate::SettlerJob::CarryOut {
            warehouse,
            destination: Some(w.flag_of(WEST_HARBOUR)),
            target: Some(order.target),
            order: Some(order),
            step: crate::settlers::CarryOutStep::Door { score: 0 },
        },
        timer: 1000,
        carrying: Some(PLANK),
    });
    // Q-297: a plank on a storage trip to the harbour is no order of it.
    let mut storage = warehouse;
    storage.storage = true;
    let flag = w.flag_of(WEST_HARBOUR);
    w.new_ware_at(PLANK, flag, Some(flag), Some(storage));
    // The next pass, run at once.
    let h = w
        .harbours
        .iter()
        .position(|h| h.at == WEST_HARBOUR)
        .unwrap();
    w.gather_expedition(h);
    assert_eq!(planks(&w), 3, "3 open + 1 on its way: none filed");
    // With all four open again (the served one back), one is too many: an
    // open one is cancelled, not the one on its way.
    w.order(WEST_HARBOUR, PLANK);
    w.gather_expedition(h);
    assert_eq!(planks(&w), 3);
}

// T5, T6: with everything there the next pass gets a ship; when it is
// docked the goods and the builder go aboard.
// spec: SHIP-5.3, SHIP-5.4, SHIP-5.5, UI-GAME-11.1 (T5, T6)
#[test]
fn ship_t05_t06_getting_the_ship_and_loading() {
    let (mut w, s) = gather_world();
    stock(&mut w, WEST_HARBOUR, PLANK, 4);
    stock(&mut w, WEST_HARBOUR, STONE, 6);
    stock(&mut w, WEST_HARBOUR, BUILDER, 1);
    w.submit(Command::ExpeditionStart {
        player: P0,
        at: WEST_HARBOUR,
    });
    // First pass: everything is set aside, then the ship is looked for.
    step_until(&mut w, 300, |w| {
        w.harbour_at(WEST_HARBOUR).unwrap().expedition == ExpeditionState::GettingShip
    });
    let h = w.harbour_at(WEST_HARBOUR).unwrap();
    assert!(h.builder_aside);
    assert_eq!(count(&w, WEST_HARBOUR, BUILDER), 0);
    assert!(w.requests(P0).is_empty());
    // The next pass calls the nearest free ship.
    step_until(&mut w, 300, |w| {
        matches!(
            w.harbour_at(WEST_HARBOUR).unwrap().expedition,
            ExpeditionState::WaitingForShip(_)
        )
    });
    assert_eq!(
        w.harbour_at(WEST_HARBOUR).unwrap().expedition,
        ExpeditionState::WaitingForShip(s)
    );
    assert_eq!(w.ships()[s].vessel.called, Some(WEST_HARBOUR));
    // T6: the tick the ship is docked there the transfer is done.
    step_until(&mut w, 3000, |w| {
        w.harbour_at(WEST_HARBOUR).unwrap().expedition == ExpeditionState::Inactive
    });
    let v = &w.ships()[s].vessel;
    assert!(v.expedition);
    let mut aboard = v.aboard.clone();
    aboard.sort_unstable();
    assert_eq!(aboard, [(PLANK, 4), (STONE, 6), (BUILDER, 1)]);
    assert_eq!(v.harbour, None);
    assert_eq!(v.called, None);
    assert!(w.harbour_at(WEST_HARBOUR).unwrap().aside.is_empty());
    assert!(w
        .harbour_at(WEST_HARBOUR)
        .unwrap()
        .docks
        .iter()
        .all(|d| d.docked.is_none()));
    run(&mut w, 2);
    assert_eq!(state_of(&w, s), ShipState::Waiting);
    // UI-GAME-11.1, 4.5 c (E-trace-030): the message names the ship, whose
    // name follows its text in the row ("Ship is awaiting orders (Aurora)").
    assert!(w.messages().iter().any(|m| m.player == P0
        && m.text == "Ship waiting for Orders"
        && m.ship == Some(s as u32)));
}

/// A ship of player 0 on an expedition, waiting at (22, 12), with the
/// harbour spots A (30, 12), B (28, 12) and C (11, 12) on the west land.
fn voyage_world() -> (World, usize) {
    let mut w = sea_world();
    let s = ship(&mut w, P0, (22, 12));
    w.set_harbour_spots(vec![(30, 12), (28, 12), (11, 12)]);
    let v = &mut w.ships[s].vessel;
    v.launched = true;
    v.expedition = true;
    v.aboard = vec![(PLANK, 4), (STONE, 6), (BUILDER, 1)];
    w.step();
    assert_eq!(state_of(&w, s), ShipState::Waiting);
    (w, s)
}

fn direction(w: &World, s: usize, spot: Point) -> usize {
    usize::from(movement::direction_toward(
        w.ships()[s].vessel.walker.at,
        spot,
    ))
}

// T7: of two spots in one direction the nearer is the option; a spot on
// enemy land gives none.
// spec: SHIP-6.1 (T7)
#[test]
fn ship_t07_landing_options() {
    let (mut w, s) = voyage_world();
    let (a, b, c) = ((30, 12), (28, 12), (11, 12));
    assert_eq!(direction(&w, s, a), direction(&w, s, b));
    // Enemy land around C.
    for f in grid::area(c, 3, 40, 30) {
        let i = w.terrain().index(f);
        w.owners[i] = Some(P1);
    }
    let options = w.landing_options(s);
    let d = direction(&w, s, b);
    assert_eq!(options[d].map(|l| l.spot), Some(b));
    // Its landing field is the first field of the ship's sea around it.
    let landing = options[d].unwrap().field;
    assert_eq!(w.region_of(landing), w.region_of((22, 12)));
    assert!(grid::distance(landing, b) <= 3);
    assert_eq!(options.iter().flatten().count(), 1, "C is on enemy land");
}

// T8, T9: go; arrive; found the harbour.
// spec: SHIP-6.2, SHIP-6.3, SHIP-6.4, SHIP-7.1, SHIP-7.2 (T8, T9)
#[test]
fn ship_t08_t09_go_and_found() {
    let (mut w, s) = voyage_world();
    // The east land is unowned and unexplored.
    for x in 26..40 {
        for y in 0..30 {
            let i = w.terrain().index((x, y));
            w.owners[i] = None;
            w.explored[i] = 0;
        }
    }
    let b = (28, 12);
    w.ships[s].vessel.options = w.landing_options(s);
    let d = direction(&w, s, b);
    let landing = w.ships()[s].vessel.options[d].expect("option B");
    assert_eq!(landing.spot, b);
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionGo {
                player: P0,
                ship: s as u32,
                direction: 7
            }
        ),
        Err(Rejection::NoSuchDirection)
    );
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionGo {
                player: P0,
                ship: s as u32,
                direction: d as u8
            }
        ),
        Ok(())
    );
    // A second go is refused while there is a target.
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionGo {
                player: P0,
                ship: s as u32,
                direction: d as u8
            }
        ),
        Err(Rejection::NotOnExpedition)
    );
    step_until(&mut w, 100, |w| state_of(w, s) == ShipState::Sailing);
    step_until(&mut w, 2000, |w| state_of(w, s) == ShipState::Waiting);
    let v = &w.ships()[s].vessel;
    assert_eq!(v.walker.at, landing.field);
    assert_eq!(v.target, None);
    assert!(v.port.is_some());
    assert!(!w.terrain().has(v.port.unwrap(), state::WATER));
    // Radius 6 explored around the landing field.
    assert!(w.is_explored(P0, (landing.field.0 + 5, landing.field.1)));
    // Arriving again sent the message.
    assert!(
        w.messages()
            .iter()
            .filter(|m| m.text == "Ship waiting for Orders")
            .count()
            >= 1
    );
    // T9: the ship is closer than 5 fields to the spot.
    assert!(grid::distance(landing.field, b) < 5);
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionBuildHarbor {
                player: P0,
                ship: s as u32
            }
        ),
        Ok(())
    );
    let site = w.site(b).expect("a harbour site");
    // T22: the builder walks from the site's field to its flag first.
    assert!(site.builder_on_way);
    assert_eq!(site.delivered, [(PLANK, 4), (STONE, 6)]);
    assert_eq!(w.building_at(b).unwrap().owner, P0);
    // Range 4 claimed, radius 6 explored at once.
    for f in grid::area(b, 4, 40, 30) {
        assert_eq!(w.field_owner(f), Some(P0), "{f:?}");
    }
    assert!(w.is_explored(P0, (b.0 + 6, b.1)));
    // The expedition is over; the empty ship searches; no material orders.
    let v = &w.ships()[s].vessel;
    assert!(!v.expedition && v.aboard.is_empty());
    assert_eq!(v.options, [None; 6]);
    run(&mut w, 400);
    assert!(w
        .orders()
        .iter()
        .all(|o| o.target.at != b || (o.good != PLANK && o.good != STONE)));
    assert!(w.site(b).is_some());
}

// SHIP T20, T21 (SHIP-7.2, SHIP-2.4 corrected; E-ship-011, E-ship-012;
// Q-1073): the ship of a founding keeps waiting in the founding's update
// (F, never set idle); in F + 1 it turns to returning, in F + 2, standing
// still, to searching; the search's first try comes in its second update
// after that (F + 4), and it sails for the nearest finished harbour.
// spec: SHIP-7.2 (SHIP T20, SHIP T22), SHIP-2.4 (SHIP T21), SHIP-6.5, SIM-START-4.4 (T16)
#[test]
fn ship_t20_t21_the_founded_ship_searches_from_returning() {
    let (mut w, s) = voyage_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    for x in 26..40 {
        for y in 0..30 {
            let i = w.terrain().index((x, y));
            w.owners[i] = None;
            w.explored[i] = 0;
        }
    }
    let b = (28, 12);
    // Rough ground around the spot (T22: levelled at the founding anyway).
    for (k, n) in w.terrain().neighbours(b).into_iter().flatten().enumerate() {
        crate::test_support::set_height(&mut w.terrain, n, 1500 * (k as i32 % 2));
    }
    assert!(w.needs_levelling(b, crate::BuildingSize::Harbor));
    w.ships[s].vessel.options = w.landing_options(s);
    let d = direction(&w, s, b);
    let go = Command::ExpeditionGo {
        player: P0,
        ship: s as u32,
        direction: d as u8,
    };
    assert_eq!(apply(&mut w, go), Ok(()));
    step_until(&mut w, 100, |w| state_of(w, s) == ShipState::Sailing);
    step_until(&mut w, 2000, |w| state_of(w, s) == ShipState::Waiting);
    // Founded in the late part of tick F, as the computer player does
    // (AI-2.1a; a command at a tick's start meets the ship's update after
    // it in the same tick), with one record of its (T16).
    let before = w.next_object();
    w.step_then(move |_, _, _, _| {
        vec![Command::PlanHarbour {
            player: P0,
            ship: s as u32,
            records: 1,
        }]
    });
    assert!(w.late_results().iter().all(|r| r.outcome.is_ok()));
    assert!(w.site(b).is_some(), "founded");
    // SIM-START-4.4 b (T16): the site, its record, its flag and the flag's
    // three ids, then the builder.
    assert_eq!(w.building_at(b).unwrap().serial, before);
    let (id, builder) = w
        .settlers()
        .find(|(_, x)| matches!(x.job, crate::SettlerJob::ToSite { .. }))
        .map(|(i, x)| (i, x.clone()))
        .expect("the builder walks to the flag");
    assert_eq!(builder.number, before + 6);
    assert_eq!(w.next_object(), before + 7);
    assert_eq!(builder.walker.at, b);
    assert_eq!(builder.walker.destination(), w.flag_of(b));
    assert_eq!(state_of(&w, s), ShipState::Waiting, "F: still waiting");
    // T22: the site levelled from its creation, though its ground is not
    // level.
    let site = w.site(b).unwrap();
    assert!(
        site.levelled() && !site.levelling_needed,
        "levelled at once"
    );
    assert!(site.leveller.is_none());
    let expected = [
        ShipState::Returning,
        ShipState::Search,
        ShipState::Search,
        ShipState::ToHarbour,
    ];
    for (k, state) in expected.into_iter().enumerate() {
        w.step();
        assert_eq!(state_of(&w, s), state, "F + {}", k + 1);
    }
    // T22, Q-1075 (E-ship-014): on the flag in A he stands one tick; the
    // site has him as its builder from A + 1 (his first work step in A + 2).
    let flag = w.flag_of(b);
    for _ in 0..200 {
        let x = w.settlers[id].as_ref().expect("still walking");
        if x.walker.at == flag && !x.walker.is_walking() {
            break;
        }
        w.step();
    }
    assert!(
        w.site(b).unwrap().builder.is_none(),
        "A: standing on the flag"
    );
    w.step();
    assert!(w.settlers[id].is_none(), "A + 1: on the site");
    assert!(w.site(b).unwrap().builder.is_some());
}

// T17: a ship docked at a harbour, then loaded for an expedition: its
// options skip the spots of the harbour's continent within 3 fields of the
// ship, not the others.
// spec: SHIP-2.5, SHIP-2.7, SHIP-6.1 (T17)
#[test]
fn ship_t17_the_harbour_field_is_the_port() {
    let mut w = sea_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    let s = ship(&mut w, P0, (20, 10));
    step_until(&mut w, 2000, |w| state_of(w, s) == ShipState::Docked);
    assert_eq!(w.ships()[s].vessel.port, Some(WEST_HARBOUR));
    // Two spots on the west land where a harbour fits: one within 3 fields
    // of the ship, one farther; one on the east land.
    let at = w.ships()[s].vessel.walker.at;
    let near = (0..30)
        .map(|y| (13, y))
        .find(|&f| grid::distance(at, f) <= 3 && w.harbour_site_for(P0, f))
        .expect("a free spot near the ship");
    let far = (0..30)
        .rev()
        .map(|y| (13, y))
        .find(|&f| grid::distance(at, f) > 6 && w.harbour_site_for(P0, f))
        .expect("a free spot far from the ship");
    w.set_harbour_spots(vec![near, far, (28, 12)]);
    let options = w.landing_options(s);
    let spots: Vec<Point> = options.iter().flatten().map(|l| l.spot).collect();
    assert!(!spots.contains(&near), "{spots:?}");
    assert!(
        spots.contains(&far) && spots.contains(&(28, 12)),
        "{spots:?}"
    );
    // Without a port none is skipped.
    w.ships[s].vessel.port = None;
    let spots: Vec<Point> = w
        .landing_options(s)
        .iter()
        .flatten()
        .map(|l| l.spot)
        .collect();
    assert!(spots.contains(&near), "{spots:?}");
}

// T18: an expedition ship's arrival: its port is the first land neighbour
// of the stop field in the order of directions 4, 5, 0, 1, 2, 3; a field
// with six Water neighbours gives none, and no harbour can be founded.
// spec: SHIP-2.7, SHIP-6.4, SHIP-7.1 (T18), UI-GAME-4.5
#[test]
fn ship_t18_the_first_land_neighbour() {
    let mut terrain = sea_terrain();
    // The stop field F (25, 10) on the sea's east shore: its neighbours in
    // directions 4 and 5 made Water, the one in direction 0 is land.
    let f = (25, 10);
    let n = |d: u8| terrain.neighbour(f, d).unwrap();
    let (n4, n5, n0) = (n(4), n(5), n(0));
    set(&mut terrain, n4, state::WATER);
    set(&mut terrain, n5, state::WATER);
    assert!(!terrain.has(n0, state::WATER));
    let mut w = world(terrain);
    assert_eq!(w.first_land_neighbour(f), Some(n0));
    // Direction 4 tested first: from (14, 10) its land wins over the land
    // in direction 3, (13, 10).
    assert_eq!(
        w.first_land_neighbour((14, 10)),
        w.terrain().neighbour((14, 10), 4)
    );
    assert_eq!(w.first_land_neighbour((20, 10)), None);
    // The arrival sets it: an expedition ship sails to (25, 10)'s field.
    w.set_harbour_spots(vec![(28, 10)]);
    let s = ship(&mut w, P0, (20, 10));
    let v = &mut w.ships[s].vessel;
    v.launched = true;
    v.expedition = true;
    v.port = Some((11, 10));
    v.target = Some(crate::navy::Landing {
        spot: (28, 10),
        field: f,
    });
    v.state = ShipState::Sailing;
    v.walker = movement::Walker::new(f);
    w.step();
    let v = &w.ships()[s].vessel;
    assert_eq!(state_of(&w, s), ShipState::Waiting);
    assert_eq!(v.port, Some(n0));
    // Stopping in open water: no port, Build harbour not possible.
    let v = &mut w.ships[s].vessel;
    v.target = Some(crate::navy::Landing {
        spot: (28, 10),
        field: (20, 10),
    });
    v.state = ShipState::Sailing;
    v.walker = movement::Walker::new((20, 10));
    w.step();
    assert_eq!(w.ships()[s].vessel.port, None);
    assert!(!w.can_found_harbour(P0, s));
}

// T10: five fields away is too far.
// spec: SHIP-7.1 (T10), UI-GAME-4.5
#[test]
fn ship_t10_too_far_to_found() {
    let (mut w, s) = voyage_world();
    let b = (28, 12);
    let v = &mut w.ships[s].vessel;
    v.port = Some((26, 12));
    v.walker = movement::Walker::new((23, 12));
    assert_eq!(grid::distance((23, 12), b), 5);
    // UI-GAME-4.5 (f): the window's view says the same as the command.
    assert!(!w.can_found_harbour(P0, s));
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionBuildHarbor {
                player: P0,
                ship: s as u32
            }
        ),
        Err(Rejection::NoHarbourSpot)
    );
    w.ships[s].vessel.walker = movement::Walker::new((24, 12));
    assert_eq!(grid::distance((24, 12), b), 4);
    assert!(w.can_found_harbour(P0, s));
    assert!(!w.can_found_harbour(P1, s));
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionBuildHarbor {
                player: P0,
                ship: s as u32
            }
        ),
        Ok(())
    );
}

// T11: cancelling a waiting expedition: the ship goes back to the nearest
// reachable harbour and gives the goods to its stock.
// spec: SHIP-6.5 (T11)
#[test]
fn ship_t11_cancel() {
    let (mut w, s) = voyage_world();
    harbour(&mut w, P0, WEST_HARBOUR);
    harbour(&mut w, P0, EAST_HARBOUR);
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStop {
                player: P1,
                ship: s as u32
            }
        ),
        Err(Rejection::NoSuchShip)
    );
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStop {
                player: P0,
                ship: s as u32
            }
        ),
        Ok(())
    );
    assert_eq!(
        apply(
            &mut w,
            Command::ExpeditionStop {
                player: P0,
                ship: s as u32
            }
        ),
        Err(Rejection::NotOnExpedition)
    );
    let v = &w.ships()[s].vessel;
    assert!(!v.expedition);
    assert_eq!(v.options, [None; 6]);
    // (22, 12) is nearer to the west harbour.
    let nearer = if grid::distance((22, 12), WEST_HARBOUR) < grid::distance((22, 12), EAST_HARBOUR)
    {
        WEST_HARBOUR
    } else {
        EAST_HARBOUR
    };
    step_until(&mut w, 3000, |w| w.ships()[s].vessel.aboard.is_empty());
    assert_eq!(count(&w, nearer, PLANK), 4);
    assert_eq!(count(&w, nearer, STONE), 6);
    assert_eq!(count(&w, nearer, BUILDER), 1);
}

/// A small building whose site needs two planks (SHIP T19).
const HUT: u32 = 7;
/// Its field on island B, east of H2.
const HUT_AT: Point = (28, 16);

/// SHIP T19's set-up: island A (the west) with a warehouse holding planks
/// and helpers, its road to harbour H1, and a finished ship of player 0 on
/// the sea; island B (the east) with harbour H2 (two helpers in stock, no
/// planks) and a hut's site needing two planks, its flag on a road to H2.
/// The site's orders are filed; no pass has run.
fn t19_world() -> World {
    t19_world_with(2)
}

/// T19's world with `helpers` helpers in H2's stock.
fn t19_world_with(helpers: u32) -> World {
    let mut w = sea_world();
    w.building_types.insert(
        HUT,
        crate::BuildingType {
            frame_costs: vec![(PLANK, 2)],
            ..crate::BuildingType::new(HUT, BuildingSize::Small)
        },
    );
    warehouse(&mut w, (4, 10));
    harbour(&mut w, P0, WEST_HARBOUR);
    harbour(&mut w, P0, EAST_HARBOUR);
    stock(&mut w, (4, 10), PLANK, 3);
    stock(&mut w, (4, 10), HELPER, 10);
    stock(&mut w, EAST_HARBOUR, HELPER, helpers);
    road_between(&mut w, (4, 10), WEST_HARBOUR);
    assert_eq!(
        apply(
            &mut w,
            Command::PlaceBuilding {
                player: P0,
                building: HUT,
                at: HUT_AT,
            }
        ),
        Ok(())
    );
    road_between(&mut w, HUT_AT, EAST_HARBOUR);
    w
}

/// H2's need of `good`: its count of attached requesters (0: no entry).
fn need(w: &World, good: u32) -> u32 {
    w.harbour_at(EAST_HARBOUR)
        .unwrap()
        .needs
        .iter()
        .find(|n| n.good == good)
        .map_or(0, |n| n.count)
}

/// The hut site's open plank orders and the harbour each is attached to.
fn hut_orders(w: &World) -> Vec<Option<Point>> {
    w.orders()
        .iter()
        .filter(|o| o.target.at == HUT_AT && o.good == PLANK)
        .map(|o| o.harbour)
        .collect()
}

// T12: a harbour whose island needs planks gets them from another harbour's
// network by ship: the hut's two plank orders attach to H2 (SHIP-4.1), H2
// asks H1, the planks are gathered, shipped and unloaded into H2, and the
// next passes serve the orders from H2's stock, which detaches them: the
// need is gone and the hut gets its planks.
// spec: SHIP-4.1, SHIP-4.2, SHIP-4.3, SHIP-4.4, SHIP-4.6 (T12)
#[test]
fn ship_t12_transfer_between_harbours() {
    let mut w = t19_world();
    let s = ship(&mut w, P0, (20, 10));
    // H2 asks H1 in its first pass.
    run(&mut w, 102);
    assert_eq!(
        w.harbour_at(WEST_HARBOUR).unwrap().transfers,
        [crate::Transfer {
            good: PLANK,
            count: 2,
            destination: EAST_HARBOUR
        }]
    );
    // (The site's builder request is attached too: no warehouse on either
    // island has a builder.)
    let planks = |w: &World| {
        w.harbour_at(EAST_HARBOUR)
            .unwrap()
            .needs
            .iter()
            .find(|n| n.good == PLANK)
            .cloned()
    };
    assert_eq!(planks(&w).unwrap().supplier, Some(WEST_HARBOUR));
    assert_eq!(need(&w, BUILDER), 1);
    // Within a while: the planks arrive at H1, a ship is called, loads,
    // sails and unloads at H2; the passes send them on to the hut.
    step_until(&mut w, 8000, |w| hut_orders(w).is_empty());
    assert_eq!(need(&w, PLANK), 0);
    assert_eq!(count(&w, WEST_HARBOUR, PLANK), 0);
    assert_eq!(count(&w, (4, 10), PLANK), 1);
    assert!(w.harbour_at(WEST_HARBOUR).unwrap().transfers.is_empty());
    assert!(w.ships()[s].vessel.cargo.is_empty());
}

// T26 (SHIP-4 the passes' ticks, SHIP-4.2 a held good, SHIP-4.4
// corrections): harbours finished before the first tick make their first
// pass in the 102nd (no update in the finishing tick, the pass 101 updates
// later); H2 holds helpers, so its helper need is not asked; a ship lying
// docked at H1 is called in the pass in which a plank is ready, and nothing
// is loaded in that pass; the next pass loads as many planks as H2's plank
// need counts then (one requester detached in between: one plank).
// spec: SHIP-4, SHIP-4.2, SHIP-4.4 (T26)
#[test]
fn ship_t26_passes_held_goods_and_the_call() {
    let mut w = t19_world();
    w.attach_need(EAST_HARBOUR, HELPER);
    let s = ship(&mut w, P0, (16, 10));
    // (The harbours were finished before the world's first tick.)
    step_until(&mut w, 200, |w| {
        !w.harbour_at(WEST_HARBOUR).unwrap().transfers.is_empty()
    });
    assert_eq!(
        w.tick(),
        102,
        "the first update in the tick after the finishing, the first pass 101 later"
    );
    let transfers = &w.harbour_at(WEST_HARBOUR).unwrap().transfers;
    assert!(transfers.iter().any(|t| t.good == PLANK && t.count == 2));
    assert!(
        transfers.iter().all(|t| t.good != HELPER),
        "H2 holds helpers: its helper need is skipped"
    );
    // The ship searches, docks at H1 and lies idle there.
    step_until(&mut w, 2000, |w| {
        state_of(w, s) == ShipState::Idle && w.ships()[s].vessel.harbour == Some(WEST_HARBOUR)
    });
    // (A second docking place on the sea: the map's harbour has only the
    // one the ship lies at, and a call needs one free there.)
    w.harbours
        .iter_mut()
        .find(|h| h.at == WEST_HARBOUR)
        .unwrap()
        .docks
        .push(crate::navy::Dock {
            field: (15, 14),
            reserved: None,
            docked: None,
        });
    // The call: in the pass a plank is ready; it ends the pass.
    step_until(&mut w, 4000, |w| {
        w.harbour_at(WEST_HARBOUR).unwrap().called.is_some()
    });
    assert!(count(&w, WEST_HARBOUR, PLANK) >= 1);
    assert!(
        w.ships()[s].vessel.cargo.is_empty(),
        "nothing loaded in the calling pass"
    );
    assert_eq!(w.harbour_at(WEST_HARBOUR).unwrap().called, Some(s));
    w.detach_need(EAST_HARBOUR, PLANK);
    assert_eq!(need(&w, PLANK), 1);
    let calling = w.harbour_at(WEST_HARBOUR).unwrap().counter;
    assert_eq!(calling, 0);
    run(&mut w, 100);
    assert!(w.ships()[s].vessel.cargo.is_empty());
    w.step();
    assert_eq!(
        w.ships()[s].vessel.cargo,
        [crate::navy::Cargo {
            destination: EAST_HARBOUR,
            goods: vec![(PLANK, 1)]
        }],
        "the destination's need as it is at the loading"
    );
}

// T19 (SHIP-4.1, Q-063): each of the hut's plank orders finds no warehouse
// with planks over island B's roads and, at its turn in the ware order pass
// (every 11 ticks, one order chosen per pass), is attached to H2 (reachable
// from the site's flag, a ship on its sea): H2's plank need is 2. With no
// ship on the sea no order is attached and H2 has no plank need.
// spec: SHIP-4.1 (T19)
#[test]
fn ship_t19_orders_attach_to_the_harbour() {
    let mut w = t19_world();
    ship(&mut w, P0, (20, 10));
    assert_eq!(hut_orders(&w), [None, None]);
    run(&mut w, 23);
    assert_eq!(hut_orders(&w), [Some(EAST_HARBOUR), Some(EAST_HARBOUR)]);
    assert_eq!(need(&w, PLANK), 2);
    // A later pass attaches nothing again.
    run(&mut w, 22);
    assert_eq!(need(&w, PLANK), 2);
    // No ship: nothing is attached.
    let mut w = t19_world();
    run(&mut w, 45);
    assert_eq!(hut_orders(&w), [None, None]);
    assert_eq!(need(&w, PLANK), 0);
    assert!(w.harbour_at(EAST_HARBOUR).unwrap().needs.is_empty());
}

// T24 (SHIP-4.1, corrected 2026-11-21 eve): the harbour choice of a ware
// order asks the stored ware route cost from the representative order's flag
// (here the hut's, its network's only orders) to each eligible harbour's
// flag: after the orders attach to H2, the store holds that pair; nothing
// is stored for H1, which no road from the hut reaches.
// spec: SHIP-4.1 (T24)
#[test]
fn ship_t24_the_harbour_choice_asks_the_stored_cost() {
    let mut w = t19_world();
    ship(&mut w, P0, (20, 10));
    let (hut_flag, east, west) = (
        w.flag_of(HUT_AT),
        w.flag_of(EAST_HARBOUR),
        w.flag_of(WEST_HARBOUR),
    );
    assert_eq!(w.route_store.stored(P0, hut_flag, east), None);
    run(&mut w, 23);
    assert_eq!(hut_orders(&w), [Some(EAST_HARBOUR), Some(EAST_HARBOUR)]);
    let stored = w.route_store.stored(P0, hut_flag, east);
    assert!(matches!(stored, Some(Some(_))), "{stored:?}");
    assert_eq!(w.route_store.stored(P0, hut_flag, west), None);
}

// T19a (SHIP-4.1, Q-063 end): T19 after a ship has unloaded one plank into
// H2. At the next pass the first plank order is served from H2: it takes
// the plank and is detached in that step (2 -> 1, before the plank leaves
// H2). Then the site is torn down: the second order is cancelled and
// detached at once (1 -> 0). With H2 torn down instead, both orders are
// detached at once.
// spec: SHIP-4.1 (T19a)
#[test]
fn ship_t19a_orders_leave_the_need() {
    let mut w = t19_world();
    ship(&mut w, P0, (20, 10));
    run(&mut w, 23);
    assert_eq!(need(&w, PLANK), 2);
    // The plank the ship unloaded.
    stock(&mut w, EAST_HARBOUR, PLANK, 1);
    let ticks = step_until(&mut w, 12, |w| need(w, PLANK) == 1);
    assert!(ticks <= 11);
    assert_eq!(hut_orders(&w), [Some(EAST_HARBOUR)]);
    // The plank is on its way out of H2 (its helper carries it).
    assert_eq!(count(&w, EAST_HARBOUR, PLANK), 0);
    assert_eq!(
        apply(
            &mut w,
            Command::RemoveBuilding {
                player: P0,
                at: HUT_AT
            }
        ),
        Ok(())
    );
    assert_eq!(need(&w, PLANK), 0);
    assert!(w
        .harbour_at(EAST_HARBOUR)
        .unwrap()
        .needs
        .iter()
        .all(|n| n.good != PLANK));
    // H2 torn down instead: both orders detached at once.
    let mut w = t19_world();
    ship(&mut w, P0, (20, 10));
    run(&mut w, 23);
    assert_eq!(hut_orders(&w), [Some(EAST_HARBOUR), Some(EAST_HARBOUR)]);
    assert_eq!(
        apply(
            &mut w,
            Command::RemoveBuilding {
                player: P0,
                at: EAST_HARBOUR
            }
        ),
        Ok(())
    );
    assert_eq!(hut_orders(&w), [None, None]);
    assert!(w.harbour_at(EAST_HARBOUR).is_none());
}

/// The road from the hut's flag to H2's flag has a carrier assigned.
fn hut_road_has_carrier(w: &World) -> bool {
    let (a, b) = (w.flag_of(HUT_AT), w.flag_of(EAST_HARBOUR));
    w.roads.iter().flatten().any(|r| {
        let ends = (r.path[0], *r.path.last().unwrap());
        (ends == (a, b) || ends == (b, a)) && (r.carrier.is_some() || r.donkey.is_some())
    })
}

// T27 (SHIP-4.1, Q-1079 answered: the ware network for a settler request
// too): the hut site's builder request finds no warehouse with a builder;
// while its road to H2 has no carrier assigned (no helper on island B) the
// flags are in different ware networks and the request is not attached.
// Once a helper is in H2 and the road's carrier is sent out, the request is
// attached at its next turn.
// spec: SHIP-4.1 (T27)
#[test]
fn ship_t27_a_settler_request_over_the_ware_network() {
    let mut w = t19_world_with(0);
    ship(&mut w, P0, (20, 10));
    run(&mut w, 150);
    assert!(!hut_road_has_carrier(&w));
    assert_eq!(need(&w, BUILDER), 0);
    stock(&mut w, EAST_HARBOUR, HELPER, 1);
    let sent = step_until(&mut w, 200, |w| {
        hut_road_has_carrier(w) || need(w, BUILDER) > 0
    });
    assert!(
        hut_road_has_carrier(&w),
        "not attached before the carrier is sent"
    );
    let attached = step_until(&mut w, 200, |w| need(w, BUILDER) == 1);
    assert!(sent + attached <= 200);
}

// The run is deterministic.
// spec: none (determinism of ships: same commands, same world)
#[test]
fn ship_runs_are_deterministic() {
    let run_once = || {
        let mut w = sea_world();
        warehouse(&mut w, (4, 10));
        harbour(&mut w, P0, WEST_HARBOUR);
        harbour(&mut w, P0, EAST_HARBOUR);
        stock(&mut w, (4, 10), PLANK, 3);
        stock(&mut w, (4, 10), HELPER, 10);
        road_between(&mut w, (4, 10), WEST_HARBOUR);
        let _ = &mut w;
        let mut w = t19_world();
        ship(&mut w, P0, (20, 10));
        run(&mut w, 3000);
        w.state_hash()
    };
    assert_eq!(run_once(), run_once());
}

// UI-GAME-4.5 c: the default name's counter is the owner's count of
// earlier ships, back to 0 after 20; a player names only its own ship, and
// the name stays with the ship.
// spec: UI-GAME-4.5
#[test]
fn ship_names() {
    let mut w = sea_world();
    let ships: Vec<usize> = (0..23).map(|_| ship(&mut w, P0, (20, 10))).collect();
    let other = ship(&mut w, P1, (20, 12));
    let index = |w: &World, i: usize| w.ship_name_index(i).unwrap();
    assert_eq!(index(&w, ships[0]), 0);
    assert_eq!(index(&w, ships[20]), 20);
    assert_eq!(index(&w, ships[21]), 0);
    assert_eq!(index(&w, ships[22]), 1);
    assert_eq!(index(&w, other), 0);
    assert_eq!(w.ship_name_index(99), None);
    let name = |player, ship: usize, name: &str| Command::NameShip {
        player,
        ship: ship as u32,
        name: name.into(),
    };
    assert_eq!(apply(&mut w, name(P0, ships[1], "Aurora")), Ok(()));
    assert_eq!(w.ships()[ships[1]].name.as_deref(), Some("Aurora"));
    assert_eq!(
        apply(&mut w, name(P0, other, "Mine")),
        Err(Rejection::NoSuchShip)
    );
    assert_eq!(
        apply(&mut w, name(P0, 99, "None")),
        Err(Rejection::NoSuchShip)
    );
    assert_eq!(w.ships()[other].name, None);
}

// FMT-S2M-4.8, 4.9: a world made without a map file stands in for the
// stored continent layer with its computed regions (blocked water -1); a
// game's setup layer is used as loaded, and a layer of another size than
// the map is ignored.
// spec: FMT-S2M-4.8, FMT-S2M-4.9
#[test]
fn the_continent_layer_is_the_setups_as_loaded() {
    let mut w = sea_world();
    let (west, sea, east) = ((5, 5), (20, 5), (35, 5));
    assert_eq!(w.continent_of(west), Some(w.region_of(west) as i32));
    assert_eq!(w.continent_of(sea), Some(w.region_of(sea) as i32));
    assert_ne!(w.continent_of(west), w.continent_of(east));
    assert_eq!(w.continent_of((40, 0)), None);
    let mut layer = vec![3; 40 * 30];
    layer[5 * 40 + 5] = -1;
    layer[5 * 40 + 35] = 7;
    w.set_continents(layer);
    assert_eq!(w.continent_of(west), Some(-1));
    assert_eq!(w.continent_of(sea), Some(3));
    assert_eq!(w.continent_of(east), Some(7));
    w.set_continents(vec![1; 10]);
    assert_eq!(w.continent_of(sea), Some(3));
}

// SHIP T23 (SHIP-5.2, the requested builder's way): the expedition's builder
// leaves the serving warehouse as a builder going to his site, out of the
// door to its flag only. There, in his arrival tick A, he is sent home
// (TRN-FREE-6.4a), his home the harbour (TRN-FREE-6.4 step 1, not the
// warehouse he just left): he stands in A + 1 and A + 2 and starts his road
// walk at progress 0 in A + 3, and goes into the harbour, where he is set
// aside at once (Q-1077, reading (a)).
// spec: SHIP-5.2 (SHIP T23, Q-1077), TRN-FREE-6.4, TRN-FREE-6.4a
#[test]
fn ship_t23_the_expedition_builders_way() {
    let (mut w, _) = gather_world();
    let store = (4, 10);
    warehouse(&mut w, store);
    road_between(&mut w, store, WEST_HARBOUR);
    stock(&mut w, WEST_HARBOUR, PLANK, 4);
    stock(&mut w, WEST_HARBOUR, STONE, 6);
    stock(&mut w, store, BUILDER, 3);
    w.submit(Command::ExpeditionStart {
        player: P0,
        at: WEST_HARBOUR,
    });
    let builder = |w: &World| {
        w.settlers
            .iter()
            .flatten()
            .find(|s| s.profession == BUILDER)
            .cloned()
    };
    step_until(&mut w, 400, |w| builder(w).is_some());
    let b = builder(&w).unwrap();
    assert!(
        matches!(b.job, crate::SettlerJob::ToSite { target, builder: true } if target.at == WEST_HARBOUR),
        "{:?}",
        b.job
    );
    assert_eq!(
        b.walker.destination(),
        w.flag_of(store),
        "out to its flag only"
    );
    let flag = w.flag_of(store);
    step_until(&mut w, 200, |w| {
        builder(w).is_some_and(|b| matches!(b.job, crate::SettlerJob::Leaving { .. }))
    });
    let b = builder(&w).unwrap();
    assert_eq!(b.walker.at, flag, "A: sent home on the warehouse's flag");
    for tick in ["A + 1", "A + 2"] {
        w.step();
        let b = builder(&w).unwrap();
        assert_eq!(b.walker.at, flag);
        assert!(!b.walker.has_walk(), "he stands in {tick}");
    }
    w.step();
    let b = builder(&w).unwrap();
    assert!(b.walker.has_walk(), "A + 3: the walk home");
    assert_eq!(b.walker.progress_bits, 0);
    assert!(
        matches!(b.job, crate::SettlerJob::Home { target } if target.at == WEST_HARBOUR),
        "home: the harbour, {:?}",
        b.job
    );
    let number = b.number;
    step_until(&mut w, 400, |w| {
        w.settlers.iter().flatten().all(|s| s.number != number)
    });
    // Q-1077, reading (a): taken in by the harbour gathering without a
    // builder, he is set aside at once, never its stock (so no later pass
    // sends him out again for its own request).
    let h = w.harbour_at(WEST_HARBOUR).unwrap();
    assert!(h.builder_aside, "set aside as he is taken in");
    assert_eq!(count(&w, WEST_HARBOUR, BUILDER), 0);
}
