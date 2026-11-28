use super::*;
use crate::movement;
use crate::test_support::*;
use crate::Command;

const PLANK: u32 = 0x10;
const COIN: u32 = 0x11;
const STONE: u32 = 0x12;
const LOG: u32 = 0x13;

/// A world with the shipped settings and three goods.
fn transport_world(width: u32, height: u32) -> World {
    let mut world = world(flat(width, height));
    world.priorities = BTreeMap::from([(PLANK, 7), (COIN, 0), (STONE, 8), (LOG, 9)]);
    world
}

fn drop(world: &mut World, good: u32, at: Point, destination: Point) {
    let outcome = apply(
        world,
        Command::DropWare {
            player: P0,
            good,
            at,
            destination,
        },
    );
    assert_eq!(outcome, Ok(()), "drop at {at:?}");
}

/// Flags at A and B two fields apart with a road A-M-B and its carrier.
fn two_step_road() -> (World, Point, Point, Point) {
    let mut world = transport_world(14, 8);
    let (a, m, b) = ((3, 4), (4, 4), (5, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    (world, a, m, b)
}

/// A road from `b` to `to` whose carrier never works, so that wares
/// for `to` have a next flag and stay where they are.
fn away_road(world: &mut World, b: Point, to: Point) {
    assert_eq!(road(world, b, to), Ok(()));
    let last = world.carriers.len() - 1;
    world.carriers[last].task = CarrierTask::Retired;
}

/// Step until the next tick in which the carrier looks for work, so that
/// a ware dropped by a command in that step is found in it (tick 0 of the
/// spec's test cases). Returns that tick.
fn step_to_look_tick(world: &mut World, carrier: usize) -> Tick {
    let number = u64::from(world.carriers[carrier].number);
    while !(number + world.tick()).is_multiple_of(IDLE_LOOK_PERIOD) {
        world.step();
    }
    world.tick()
}

// TRN-CARRY-1.3
#[test]
fn idle_fields() {
    assert_eq!(idle_field(&[(0, 0), (1, 0), (2, 0)]), (1, 0));
    assert_eq!(idle_field(&[(0, 0), (1, 0), (2, 0), (3, 0)]), (2, 0));
}

// spec: TRN-CARRY-1.2, TRN-CARRY-1.3
#[test]
fn every_road_gets_a_carrier_on_its_idle_field() {
    let (world, _, m, _) = two_step_road();
    assert_eq!(world.carriers().len(), 1);
    assert_eq!(world.carriers()[0].walker.at, m);
    assert_eq!(world.roads().next().unwrap().carrier, Some(0));
}

// TRN-ROAD T13: the road is destroyed while its carrier holds a ware:
// the held ware is destroyed, the wares at A stay, the flags stay, the
// carrier is released.
// spec: TRN-ROAD-6.1, TRN-ROAD-6.3, TRN-CARRY-7.2 (TRN-ROAD T13)
#[test]
fn road_t13_destroy_a_road_with_a_carrier() {
    let (mut world, a, m, b) = two_step_road();
    step_to_look_tick(&mut world, 0);
    for _ in 0..3 {
        drop(&mut world, PLANK, a, b);
    }
    for _ in 0..400 {
        world.step();
        if world.carriers()[0].ware.is_some() && world.carriers()[0].walker.at == a {
            break;
        }
    }
    let held = world.carriers()[0].ware.expect("the carrier holds a ware");
    let at_a = world.wares_at(a).len();
    assert!(at_a >= 1, "wares are left at A");
    let total = world.wares().count();
    let outcome = apply(&mut world, Command::RemoveRoad { player: P0, at: m });
    assert_eq!(outcome, Ok(()));
    assert_eq!(world.roads().count(), 0);
    assert_eq!(world.flag_at(a), Some(P0));
    assert_eq!(world.flag_at(b), Some(P0));
    assert!(world.ware(held).is_none());
    assert_eq!(world.wares().count(), total - 1);
    assert_eq!(world.wares_at(a).len(), at_a);
    assert_eq!(world.carriers()[0].ware, None);
    assert_eq!(world.carriers()[0].task, CarrierTask::Retired);
    assert!(world.wares().all(|(_, w)| w.claimed_by.is_none()));
}

// GFX-ANIM-3.1, TRN-CARRY-3.1, 3.3: a carrier's pick-up and put-down each
// request PutDown in the drop-time action and Pickup in the pickup-time
// action, 4 ticks each (E-trace-005, carrier 90 204 at (140, 50)).
// spec: GFX-ANIM-3.1, TRN-CARRY-3.1, TRN-CARRY-3.3
#[test]
fn a_carrier_puts_down_then_picks_up() {
    let (mut world, a, _, b) = two_step_road();
    step_to_look_tick(&mut world, 0);
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    let mut seen = Vec::new();
    for _ in 0..140 {
        world.step();
        let row = world
            .trace_settlers()
            .into_iter()
            .find(|r| r.job.starts_with("Carrier.P"))
            .map(|r| (r.job.clone(), r.action));
        if let Some(row) = row {
            if seen.last() != Some(&row) {
                seen.push(row);
            }
        }
    }
    use crate::trace::Action::{Pickup, PutDown};
    assert_eq!(
        seen,
        [
            ("Carrier.PickUp".to_owned(), PutDown),
            ("Carrier.PickUp".to_owned(), Pickup),
            ("Carrier.PutDown".to_owned(), PutDown),
            ("Carrier.PutDown".to_owned(), Pickup),
        ]
    );
}

// TRN-CARRY T1, tick by tick (TRN-CARRY-3.3a, Q-104): at A in 23, the
// drop-time action 23 .. 26, the pickup-time action 27 .. 30, the walk with
// the plank at progress 0 in 31; at B in 76, standing; drop-time 77 .. 80, the
// plank lies at B from 81; pickup-time 81 .. 84; standing in 85; the walk to M
// at progress 0 in 86.
// spec: TRN-CARRY-2.1, TRN-CARRY-3.1, TRN-CARRY-3.2, TRN-CARRY-3.3, TRN-CARRY-3.3a (T1)
#[test]
fn t01_carry_one_plank() {
    let (mut world, a, m, b) = two_step_road();
    let start = step_to_look_tick(&mut world, 0);
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    let mut events: BTreeMap<&str, Tick> = BTreeMap::new();
    for _ in 0..140 {
        world.step();
        let tick = world.tick() - 1 - start;
        let c = &world.carriers()[0];
        let seen = |events: &BTreeMap<&str, Tick>, name: &str| events.contains_key(name);
        let checks = [
            ("at A", c.walker.at == a),
            (
                "drop-time action ends",
                // TRN-CARRY-3.1: the ware is held from here, off the flag.
                matches!(c.task, CarrierTask::PickUp { second: true, .. })
                    && c.ware.is_some()
                    && world.wares_at(a).is_empty(),
            ),
            (
                "has the ware",
                c.ware.is_some() && matches!(c.task, CarrierTask::Carry { .. }),
            ),
            ("at B", c.walker.at == b),
            ("stands at B", matches!(c.task, CarrierTask::Stop { .. })),
            (
                "drop-time at B",
                matches!(c.task, CarrierTask::PutDown { second: false, .. }),
            ),
            (
                "plank lies at B",
                !world.wares_at(b).is_empty() || world.ware(0).is_none(),
            ),
            (
                "stands after the pick-up",
                seen(&events, "at B") && matches!(c.task, CarrierTask::Out { .. }),
            ),
            (
                "walk to M",
                seen(&events, "at B")
                    && c.task == CarrierTask::Idle
                    && c.walker.has_walk()
                    && c.walker.next.is_none(),
            ),
            (
                "back at M",
                seen(&events, "walk to M") && c.walker.at == m && !c.walker.is_walking(),
            ),
        ];
        for (name, happened) in checks {
            if happened {
                events.entry(name).or_insert(tick);
            }
        }
    }
    let expected = [
        ("at A", 23),
        ("drop-time action ends", 27),
        ("has the ware", 31),
        ("at B", 76),
        ("stands at B", 76),
        ("drop-time at B", 77),
        ("plank lies at B", 81),
        ("stands after the pick-up", 85),
        ("walk to M", 86),
    ];
    for (name, tick) in expected {
        assert_eq!(events.get(name), Some(&tick), "{name}");
    }
    // One level step from 86 at progress 0.
    assert!(
        events["back at M"].abs_diff(109) <= 1,
        "{:?}",
        events["back at M"]
    );
    // The plank reached its destination flag and was delivered.
    assert_eq!(world.wares().count(), 0);
    assert_eq!(world.roads().next().unwrap().traffic.transport_count, 1);
}

// TRN-CARRY T2
// spec: TRN-CARRY-2.1 (T2)
#[test]
fn t02_reaction_waits_for_the_look_tick() {
    let (mut world, a, _, b) = two_step_road();
    let look = step_to_look_tick(&mut world, 0);
    // Run to a tick with (number + tick) mod 20 = 7.
    while (u64::from(world.carriers[0].number) + world.tick()) % IDLE_LOOK_PERIOD != 7 {
        world.step();
    }
    assert!(world.tick() > look);
    let dropped = world.tick();
    drop(&mut world, PLANK, a, b);
    while world.carriers[0].task == CarrierTask::Idle {
        world.step();
    }
    assert_eq!(world.tick() - 1 - dropped, 13);
}

// TRN-CARRY T3, T4: priority first, then the second flag on a tie.
// spec: TRN-CARRY-2.4, TRN-CARRY-2.5 (T3, T4)
#[test]
fn t03_t04_work_choice() {
    let (mut world, a, _, b) = two_step_road();
    drop(&mut world, PLANK, a, b);
    drop(&mut world, COIN, b, a);
    assert_eq!(world.choose_work(0), Some((1, b)));

    let (mut world, a, _, b) = two_step_road();
    drop(&mut world, COIN, a, b);
    drop(&mut world, PLANK, b, a);
    assert_eq!(world.choose_work(0), Some((0, a)));

    let (mut world, a, _, b) = two_step_road();
    drop(&mut world, PLANK, a, b);
    drop(&mut world, PLANK, b, a);
    let second_flag = world.roads().next().unwrap().path[2];
    assert_eq!(second_flag, b);
    assert_eq!(world.choose_work(0), Some((1, b)));
}

// TRN-CARRY-2.4: among equal priorities the first put down wins.
#[test]
fn first_listed_wins_among_equals() {
    let (mut world, a, _, b) = two_step_road();
    drop(&mut world, PLANK, a, b);
    drop(&mut world, PLANK, a, b);
    assert_eq!(world.best_ware(0, a), Some((0, 7)));
}

// TRN-CARRY T5
// spec: TRN-CARRY-3.5, TRN-CARRY-2.5 (T5)
#[test]
fn t05_takes_the_way_back_at_once() {
    let (mut world, a, m, b) = two_step_road();
    drop(&mut world, PLANK, a, b);
    // Run until the plank lies... is delivered at B, then drop a stone at B
    // for A while the carrier is still at B.
    while world.ware(0).is_some() {
        world.step();
    }
    drop(&mut world, STONE, b, a);
    let mut went_to_m = false;
    while world.carriers[0].ware.is_none() {
        world.step();
        let c = &world.carriers[0];
        went_to_m |= c.walker.at == m;
    }
    assert!(!went_to_m);
    assert_eq!(world.ware(1).unwrap().location, WareLocation::Carried(0));
}

// TRN-CARRY-2.2 (corrected, Q-107): a carrier walking back from B to its
// idle field does not look for work on the way: a ware dropped at B for the
// road while it is halfway into its first step does not turn it back; it
// walks on to the idle field M and claims the ware there, in a tick of its
// 20-tick grid (TRN-CARRY-2.1).
// spec: TRN-CARRY-2.2, TRN-CARRY-2.1
#[test]
fn walks_back_to_the_idle_field_without_looking() {
    let (mut world, a, m, b) = two_step_road();
    drop(&mut world, PLANK, a, b);
    while world.ware(0).is_some() {
        world.step();
    }
    // Walking back from B: wait until it is halfway into its first step.
    while world.carriers[0].walker.progress_millionths() < movement::ONE / 2 {
        world.step();
    }
    assert_eq!(world.carriers[0].walker.at, b);
    drop(&mut world, STONE, b, a);
    assert_eq!(world.carriers[0].task, CarrierTask::Idle);
    assert_eq!(world.carriers[0].walker.next, Some(m), "it walks on");
    let number = u64::from(world.carriers[0].number);
    loop {
        world.step();
        let c = &world.carriers[0];
        if matches!(c.task, CarrierTask::Fetch { .. }) {
            assert_eq!(c.walker.at, m, "claimed on the idle field");
            assert!((number + world.tick() - 1).is_multiple_of(IDLE_LOOK_PERIOD));
            return;
        }
        assert_eq!(c.task, CarrierTask::Idle);
        assert!(world.tick() < 200, "never claimed");
    }
}

/// Flags X, F, G in a row with roads X-F and F-G and their carriers; F
/// full of wares nobody takes.
fn blocked_flag() -> (World, Point, Point, Point) {
    let mut world = transport_world(16, 8);
    let (x, f, g) = ((2, 4), (6, 4), (10, 4));
    for at in [x, f, g] {
        assert_eq!(flag(&mut world, at), Ok(()));
    }
    assert_eq!(road(&mut world, x, f), Ok(()));
    assert_eq!(road(&mut world, f, g), Ok(()));
    for _ in 0..6 {
        world.new_ware_at(STONE, f, Some((14, 7)), None);
    }
    (world, x, f, g)
}

// TRN-CARRY-3.6 (a): the carrier of F-G is idle, so the waiting carrier
// takes over F-G and carries on; the other goes to X-F.
#[test]
fn road_swap_with_an_idle_carrier() {
    let (mut world, x, _, g) = blocked_flag();
    drop(&mut world, PLANK, x, g);
    let plank = world.wares().find(|(_, w)| w.good == PLANK).unwrap().0;
    for _ in 0..800 {
        world.step();
    }
    assert!(world.ware(plank).is_none(), "delivered past the full flag");
    assert_eq!(world.roads[0].as_ref().unwrap().carrier, Some(1));
    assert_eq!(world.roads[1].as_ref().unwrap().carrier, Some(0));
    let c1 = &world.carriers[1];
    assert_eq!(c1.road, 0);
    assert_eq!(
        c1.walker.at,
        idle_field(&world.roads[0].as_ref().unwrap().path)
    );
}

// AI-CON T95 (AI-CON-8.1e, the field's settler mark): a settler that comes
// to stand on a field marks it; any settler leaving the field clears the
// mark, whoever set it, so the first one standing there no longer holds it
// until it moves itself.
// spec: AI-CON-8.1e (AI-CON T95)
#[test]
fn t95_any_settler_leaving_a_field_clears_its_mark() {
    let mut world = transport_world(16, 8);
    let (a, b, c) = ((2, 4), (6, 4), (10, 4));
    for at in [a, b, c] {
        assert_eq!(flag(&mut world, at), Ok(()));
    }
    assert_eq!(road(&mut world, a, b), Ok(()));
    assert_eq!(road(&mut world, b, c), Ok(()));
    let field = (8, 6);
    let place = |w: &mut World, i: usize, at: Point| {
        w.carriers[i].walker = crate::movement::Walker::new(at);
    };
    place(&mut world, 0, field);
    place(&mut world, 1, (12, 6));
    world.update_field_marks();
    let first = world.carriers[0].number;
    assert_eq!(world.field_mark(field), Some(first), "it stands there");
    // The other carrier comes onto the field (walking on) and leaves it.
    world.carriers[1].walker = crate::movement::Walker::new(field);
    world.carriers[1].walker.walk(vec![(9, 6)]);
    world.update_field_marks();
    assert_eq!(world.field_mark(field), None, "walking onto it clears it");
    place(&mut world, 1, (9, 6));
    world.update_field_marks();
    assert_eq!(world.field_mark(field), None);
    assert_eq!(world.field_mark((9, 6)), Some(world.carriers[1].number));
    // The first carrier, still standing there, holds no mark until it moves.
    assert_eq!(world.carriers[0].walker.at, field);
    world.carriers[0].walker.walk(vec![(7, 6)]);
    world.update_field_marks();
    place(&mut world, 0, (7, 6));
    world.update_field_marks();
    assert_eq!(world.field_mark((7, 6)), Some(first));
}

// TRN-CARRY T44 (3.6 case a): the idle carrier's way to his new road is one
// walk along F-G to F and on along X-F to its idle field, ordered in the
// swap's tick at progress 0; he never stops on F.
// spec: TRN-CARRY-3.6 (TRN-CARRY T44)
#[test]
fn t44_the_swapped_idle_carrier_walks_through_f() {
    let (mut world, x, f, g) = blocked_flag();
    drop(&mut world, PLANK, x, g);
    for _ in 0..800 {
        world.step();
        if world.roads[1].as_ref().unwrap().carrier == Some(0) {
            break;
        }
    }
    assert_eq!(world.roads[1].as_ref().unwrap().carrier, Some(0), "swapped");
    let idle = idle_field(&world.roads[0].as_ref().unwrap().path);
    let c1 = &world.carriers[1];
    assert_eq!(
        c1.task,
        CarrierTask::Arrive {
            flag: idle,
            stand: false
        }
    );
    assert_eq!(c1.walker.progress_millionths(), 0, "ordered at progress 0");
    for _ in 0..200 {
        world.step();
        let c1 = &world.carriers[1];
        assert!(
            !(c1.walker.at == f && !c1.walker.is_walking()),
            "no stop on F"
        );
        if c1.walker.at == idle && !c1.walker.is_walking() {
            return;
        }
    }
    panic!("never reached the idle field of X-F");
}

// TRN-CARRY T54 (3.6 case a): the other carrier takes up its walk in its
// own next update. Here it comes before the swapping carrier in the list
// (F-G's road and carrier made first), so it has been updated in the swap's
// tick already: it stands through that tick, is at progress 0 in the next
// and advances from the one after (after the swapping carrier it would
// advance from the next tick, T44).
// spec: TRN-CARRY-3.6 (TRN-CARRY T54)
#[test]
fn t54_the_other_carrier_before_the_swapping_one_starts_a_tick_later() {
    let mut world = transport_world(16, 8);
    let (x, f, g) = ((2, 4), (6, 4), (10, 4));
    for at in [x, f, g] {
        assert_eq!(flag(&mut world, at), Ok(()));
    }
    assert_eq!(road(&mut world, f, g), Ok(()));
    assert_eq!(road(&mut world, x, f), Ok(()));
    for _ in 0..6 {
        world.new_ware_at(STONE, f, Some((14, 7)), None);
    }
    drop(&mut world, PLANK, x, g);
    for _ in 0..800 {
        world.step();
        if world.roads[0].as_ref().unwrap().carrier == Some(1) {
            break;
        }
    }
    assert_eq!(world.roads[0].as_ref().unwrap().carrier, Some(1), "swapped");
    let progress = |w: &World| w.carriers[0].walker.progress_millionths();
    assert!(matches!(world.carriers[0].task, CarrierTask::Arrive { .. }));
    assert_eq!(progress(&world), 0, "ordered in the swap's tick");
    world.step();
    assert_eq!(progress(&world), 0, "at progress 0 in the next tick");
    world.step();
    assert!(progress(&world) > 0, "advancing from the one after");
}

// SIM-START T15 (4.3 d): a road swap of carriers renumbers: R1, then R2 lose
// their carriers (no donkey: two numbers each), then R2 and R1 get theirs,
// one number each where the road's ends are apart then: here X-F and F-G
// join nothing else, so six numbers in the swap's tick.
// spec: SIM-START-4.3 (SIM-START T15), TRN-CARRY-3.6
#[test]
fn t15_a_road_swap_takes_its_numbers() {
    let (mut world, x, _, g) = blocked_flag();
    drop(&mut world, PLANK, x, g);
    for _ in 0..800 {
        let before = world.next_object();
        world.step();
        if world.roads[1].as_ref().unwrap().carrier == Some(0) {
            assert_eq!(world.next_object() - before, 6);
            return;
        }
    }
    panic!("no swap");
}

// TRN-CARRY T46 (3.5a): a carrier leaving a flag with a ware carries it
// towards the next flag the ware holds, not to his road's far end: the
// carrier of X-F, made to fetch a ware at F whose next flag is G, carries it
// along F-G (another carrier's road) to G.
// spec: TRN-CARRY-3.5a (TRN-CARRY T46)
#[test]
fn t46_a_carry_walk_goes_to_the_wares_next_flag() {
    let mut world = transport_world(16, 8);
    let (x, f, g) = ((2, 4), (6, 4), (10, 4));
    for at in [x, f, g] {
        assert_eq!(flag(&mut world, at), Ok(()));
    }
    assert_eq!(road(&mut world, x, f), Ok(()));
    assert_eq!(road(&mut world, f, g), Ok(()));
    let ware = world.new_ware_at(PLANK, f, Some(g), None);
    assert_eq!(world.wares[ware].as_ref().unwrap().next, Some(g));
    let tick = world.tick();
    world.start_fetch(0, ware, f, tick);
    for _ in 0..300 {
        world.step();
        if let CarrierTask::Carry { to, .. } = world.carriers[0].task {
            assert_eq!(to, g);
            assert_eq!(world.carriers[0].walker.destination(), g);
            return;
        }
    }
    panic!("never carried");
}

// TRN-CARRY T58 (3.5a): the carry walk's search counts the carrier's own
// road as 1: from A to the ware's next flag F his own road A-B (L 4) and
// B-F (L 6) cost 1 + 6 = 7 against A-F's 8, so he carries it over his road
// and the walk ends at B, the first flag reached (by every road's L it
// would go straight to F, 10 against 8).
// spec: TRN-CARRY-3.5a (TRN-CARRY T58)
#[test]
fn t58_the_carry_walk_counts_the_own_road_as_one() {
    let (a, b, f) = ((2, 4), (10, 4), (6, 8));
    let (mut world, own) = own_road_network(true, [4, 8, 6]);
    let ware = world.new_ware_at(PLANK, a, Some(f), None);
    assert_eq!(world.wares[ware].as_ref().unwrap().next, Some(f));
    let carrier = world
        .carriers
        .iter()
        .position(|c| c.road == own)
        .expect("the road's carrier");
    let tick = world.tick();
    world.start_fetch(carrier, ware, a, tick);
    for _ in 0..300 {
        world.step();
        if let CarrierTask::Carry { to, .. } = world.carriers[carrier].task {
            assert_eq!(to, b);
            assert_eq!(world.carriers[carrier].walker.destination(), b);
            return;
        }
    }
    panic!("never carried");
}

// TRN-CARRY T13, 3.6 (b): two carriers blocking each other at F swap
// roads and both wares go on.
// spec: TRN-CARRY-3.6 (T13)
#[test]
fn road_swap_of_two_blocked_carriers() {
    let (mut world, x, _, g) = blocked_flag();
    drop(&mut world, PLANK, x, g);
    drop(&mut world, COIN, g, x);
    for _ in 0..800 {
        world.step();
    }
    let left: Vec<u32> = world.wares().map(|(_, w)| w.good).collect();
    assert_eq!(left, [STONE; 6], "both delivered");
    assert_eq!(world.carriers[0].road, 1);
    assert_eq!(world.carriers[1].road, 0);
}

// TRN-CARRY-3.6 (b), 3.7, 2.6: the carrier of F-G walks towards F with a
// ware for X, but its entry is settled (an exchange ware at F is bound
// to it), so it is not waiting for room. Between two steps it stands on
// a field with its walk not done, as a stopped carrier does; that must
// not count as waiting. A swap there dropped its task and left the
// bound ware claimed by nobody's job: no carrier ever took it again, and
// the site it was for waited for good.
#[test]
fn no_road_swap_with_a_carrier_bound_for_an_exchange() {
    let (mut world, x, f, g) = blocked_flag();
    for _ in 0..50 {
        world.step();
    }
    let carry = |world: &mut World, carrier: usize, good: u32, to: Point| {
        let ware = world.new_ware_at(good, f, Some(to), None);
        world.flag_wares.get_mut(&f).unwrap().retain(|&w| w != ware);
        let w = world.wares[ware].as_mut().unwrap();
        w.location = WareLocation::Carried(carrier);
        w.claimed_by = Some(carrier);
        world.carriers[carrier].ware = Some(ware);
        let path = world.roads[world.carriers[carrier].road]
            .as_ref()
            .unwrap()
            .path
            .clone();
        let i = path.iter().position(|&p| p == f).unwrap();
        let before = if i == 0 { path[1] } else { path[i - 1] };
        world.carriers[carrier].walker = movement::Walker::new(before);
        world.carriers[carrier].walker.walk([f]);
        ware
    };
    // Carrier 0 (X-F) waits before F with a plank for G.
    carry(&mut world, 0, PLANK, g);
    world.carriers[0].task = CarrierTask::Carry {
        to: f,
        entry: Entry::Open { stop: true },
    };
    // Carrier 1 (F-G) brings a coin for X and has a stone at F bound.
    carry(&mut world, 1, COIN, x);
    let bound = world.new_ware_at(STONE, f, Some(g), None);
    world.wares[bound].as_mut().unwrap().claimed_by = Some(1);
    world.carriers[1].task = CarrierTask::Carry {
        to: f,
        entry: Entry::Exchange(bound),
    };
    world.road_swap(0, f);
    assert_eq!((world.carriers[0].road, world.carriers[1].road), (0, 1));
    assert_eq!(
        world.carriers[1].task,
        CarrierTask::Carry {
            to: f,
            entry: Entry::Exchange(bound)
        }
    );
    assert_eq!(world.ware(bound).unwrap().claimed_by, Some(1));
}

// TRN-CARRY-3.6a (Q-1008): a carrier may carry a ware along a neighbouring road
// (3.5a), so the full flag it waits for need not lie on its own road. The original
// swaps there too: this carrier takes R2 and starts its carry walk again from where
// it stands; the other only has its road changed, and in its own next update starts
// the ordinary road walk to its new idle field (TRN-FREE-6.1). The flag plays no part
// in that walk. It was a panic, "field on the road", in a game of campaign 5 at tick
// 43375, then a stand-in that did not swap here.
// spec: TRN-CARRY-3.5a, TRN-CARRY-3.6, TRN-CARRY-3.6a, TRN-FREE-6.1
#[test]
fn a_road_swap_at_a_flag_off_the_carriers_road() {
    let (mut world, _x, f, g) = blocked_flag();
    let h = (14, 4);
    assert_eq!(flag(&mut world, h), Ok(()));
    assert_eq!(road(&mut world, g, h), Ok(()));
    for _ in 0..200 {
        world.step();
    }
    // The carrier of G-H stands idle on its idle field: 3.6 (a) holds.
    let old_home = world.roads[0].as_ref().unwrap().path.clone();
    let path = world.roads[2].as_ref().unwrap().path.clone();
    assert_eq!(world.carriers[2].road, 2);
    assert_eq!(world.carriers[2].task, CarrierTask::Idle);
    assert_eq!(world.carriers[2].walker.at, idle_field(&path));
    assert!(!old_home.contains(&g), "G is not on the carrier's road");
    // The carrier of X-F waits before G with a plank for H, on the road F-G.
    let plank = world.new_ware_at(PLANK, g, Some(h), None);
    world
        .flag_wares
        .get_mut(&g)
        .unwrap()
        .retain(|&w| w != plank);
    let w = world.wares[plank].as_mut().unwrap();
    w.location = WareLocation::Carried(0);
    w.claimed_by = Some(0);
    world.carriers[0].ware = Some(plank);
    let before_g = {
        let on = world.roads[1].as_ref().unwrap().path.clone();
        let i = on.iter().position(|&p| p == g).unwrap();
        if i > 0 {
            on[i - 1]
        } else {
            on[1]
        }
    };
    world.carriers[0].walker = movement::Walker::new(before_g);
    world.carriers[0].walker.walk([g]);
    world.carriers[0].task = CarrierTask::Carry {
        to: g,
        entry: Entry::Open { stop: true },
    };
    world.road_swap(0, g);
    // The roads are exchanged.
    assert_eq!(world.carriers[0].road, 2);
    assert_eq!(world.carriers[2].road, 0);
    assert_eq!(world.roads[2].as_ref().unwrap().carrier, Some(0));
    assert_eq!(world.roads[0].as_ref().unwrap().carrier, Some(2));
    // This one goes on with its ware through G to H, without stopping at G.
    assert_eq!(
        world.carriers[0].task,
        CarrierTask::Carry {
            to: h,
            entry: Entry::Open { stop: false },
        }
    );
    assert_eq!(world.carriers[0].walker.destination(), h);
    // The other is still idle where it stood: only its road changed.
    assert_eq!(world.carriers[2].task, CarrierTask::Idle);
    assert_eq!(world.carriers[2].walker.at, idle_field(&path));
    assert!(!world.carriers[2].walker.is_walking());
    // In its next update it starts the road walk to X-F's idle field, passing
    // over F-G and F, and gets there. The plank gets to H.
    world.step();
    assert!(
        matches!(world.carriers[2].task, CarrierTask::Arrive { .. }),
        "{:?}",
        world.carriers[2].task
    );
    let goal = idle_field(&old_home);
    assert_eq!(world.carriers[2].walker.destination(), goal);
    let mut through_f = false;
    let mut arrived = false;
    for _ in 0..600 {
        world.step();
        through_f |= world.carriers[2].walker.at == f;
        if world.carriers[2].walker.at == goal && world.carriers[2].task == CarrierTask::Idle {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "at {:?}", world.carriers[2].walker.at);
    assert!(through_f, "over the roads");
    for _ in 0..300 {
        world.step();
    }
    assert!(world.ware(plank).is_none(), "delivered at H");
}

/// A plank for `to` in `carrier`'s hands, the carrier one step short of the
/// flag `at` on `road`, walking there and waiting for room (TRN-CARRY-3.2).
fn hand_over_waiting(world: &mut World, carrier: usize, at: Point, to: Point, road: usize) {
    let ware = world.new_ware_at(PLANK, at, Some(to), None);
    world
        .flag_wares
        .get_mut(&at)
        .unwrap()
        .retain(|&w| w != ware);
    let w = world.wares[ware].as_mut().unwrap();
    w.location = WareLocation::Carried(carrier);
    w.claimed_by = Some(carrier);
    world.carriers[carrier].ware = Some(ware);
    let on = world.roads[road].as_ref().unwrap().path.clone();
    let i = on.iter().position(|&p| p == at).unwrap();
    let before = if i > 0 { on[i - 1] } else { on[1] };
    world.carriers[carrier].walker = movement::Walker::new(before);
    world.carriers[carrier].walker.walk([at]);
    world.carriers[carrier].task = CarrierTask::Carry {
        to: at,
        entry: Entry::Open { stop: true },
    };
}

// TRN-CARRY-3.6a (Q-1012): case (b) off the carrier's road compares the other
// carrier's goal flag G and its ware's next flag NF with the two ends of this
// carrier's road. X-F's carrier waits at G over F-G with a plank for H; G-H's
// carrier waits at G with a plank for X, over a road of its own from G to X.
// G = F can never be an end of X-F: no swap, both keep waiting.
// spec: TRN-CARRY-3.6, TRN-CARRY-3.6a
#[test]
fn no_road_swap_off_the_road_when_the_others_leg_is_not_this_road() {
    let (mut world, x, _f, g) = blocked_flag();
    let h = (14, 4);
    assert_eq!(flag(&mut world, h), Ok(()));
    assert_eq!(road(&mut world, g, h), Ok(()));
    assert_eq!(road(&mut world, g, x), Ok(()));
    for _ in 0..200 {
        world.step();
    }
    hand_over_waiting(&mut world, 0, g, h, 1);
    hand_over_waiting(&mut world, 2, g, x, 2);
    let owner = world.roads[0].as_ref().unwrap().owner;
    assert_eq!(world.next_flag(owner, g, x), Some(x), "over the road G-X");
    world.road_swap(0, g);
    assert_eq!((world.carriers[0].road, world.carriers[2].road), (0, 2));
    let waiting = |to| CarrierTask::Carry {
        to,
        entry: Entry::Open { stop: true },
    };
    assert_eq!(world.carriers[0].task, waiting(g));
    assert_eq!(world.carriers[2].task, waiting(g));
}

// TRN-CARRY-3.6a (Q-1012): off the carrier's road case (b) swaps when the
// other carrier waits at R2's far end and this carrier's road begins there,
// the other's ware going on along it. G-H's carrier has come to F over X-F
// with a plank for H (next flag G, R2 = F-G); F-G's carrier waits at G with a
// plank for H, whose next leg is G-H. They swap: G-H's carrier carries on to
// G over F-G, F-G's carrier from G along G-H to H.
// spec: TRN-CARRY-3.6, TRN-CARRY-3.6a
#[test]
fn a_road_swap_off_the_road_when_the_others_next_leg_is_this_road() {
    let (mut world, _x, f, g) = blocked_flag();
    let h = (14, 4);
    assert_eq!(flag(&mut world, h), Ok(()));
    assert_eq!(road(&mut world, g, h), Ok(()));
    for _ in 0..200 {
        world.step();
    }
    assert_eq!((world.carriers[1].road, world.carriers[2].road), (1, 2));
    hand_over_waiting(&mut world, 2, f, h, 0);
    hand_over_waiting(&mut world, 1, g, h, 1);
    let owner = world.roads[0].as_ref().unwrap().owner;
    assert_eq!(world.next_flag(owner, f, h), Some(g));
    assert_eq!(world.next_flag(owner, g, h), Some(h));
    world.road_swap(2, f);
    assert_eq!((world.carriers[2].road, world.carriers[1].road), (1, 2));
    assert_eq!(world.carriers[2].walker.destination(), g);
    assert_eq!(world.carriers[1].walker.destination(), h);
    let open = |to| CarrierTask::Carry {
        to,
        entry: Entry::Open { stop: false },
    };
    assert_eq!(world.carriers[2].task, open(g));
    assert_eq!(world.carriers[1].task, open(h));
}

// TRN-CARRY T6
// spec: TRN-CARRY-3.2, TRN-CARRY-4.1, TRN-CARRY-4.2 (T6)
#[test]
fn t06_waits_before_a_full_flag() {
    let (mut world, a, m, b) = two_step_road();
    // A third flag beyond B whose road has no working carrier: wares at B
    // for it go nowhere.
    // A sixth for D fits because it leaves the other way (TRN-CARRY-4.2);
    // no ware at B heads to A, so there is nothing to exchange (T16).
    assert_eq!(flag(&mut world, (8, 4)), Ok(()));
    assert_eq!(flag(&mut world, (6, 7)), Ok(()));
    away_road(&mut world, b, (8, 4));
    for _ in 0..5 {
        drop(&mut world, STONE, b, (8, 4));
    }
    away_road(&mut world, b, (6, 7));
    drop(&mut world, STONE, b, (6, 7));
    assert_eq!(world.wares_at(b).len(), 6);
    // The plank at A (priority 7) goes before the stone at B (8).
    drop(&mut world, PLANK, a, b);
    // The carrier fetches the plank and stops at M, short of B.
    for _ in 0..200 {
        world.step();
    }
    let c = &world.carriers[0];
    assert_eq!(c.ware, Some(6));
    assert_eq!((c.walker.at, c.walker.next), (m, None));
    assert!(matches!(
        c.task,
        CarrierTask::Carry {
            entry: Entry::Open { stop: true },
            ..
        }
    ));
    // Make room: its first check with room sets the flag's bar, the second
    // one 10 ticks later admits it (TRN-CARRY-4.4, T16).
    let freed = world.tick();
    let stone = world.flag_wares.get_mut(&b).unwrap().remove(0);
    world.wares[stone] = None;
    while world.carriers[0].walker.next.is_none() {
        world.step();
    }
    // The tick in which it moves again: the retry tick decides, and a walk
    // ordered in one tick advances in the next (TRN-MOVE-2.1).
    let moved = world.tick() - 1;
    assert!(moved - freed <= 20, "{}", moved - freed);
}

// TRN-CARRY T36 (TRN-CARRY-3.2): a carrier's first entry check for the
// flag it carries to is in the tick after its walk is ordered: at a full
// flag the ware's wait score is still 0 in the ordering tick and grows
// from the next.
// spec: TRN-CARRY-3.2 (TRN-CARRY T36), TRN-CARRY-4.4
#[test]
fn t36_the_first_entry_check_is_in_the_tick_after_the_walk_is_ordered() {
    let (mut world, a, _, b) = two_step_road();
    assert_eq!(flag(&mut world, (8, 4)), Ok(()));
    assert_eq!(flag(&mut world, (6, 7)), Ok(()));
    away_road(&mut world, b, (8, 4));
    for _ in 0..5 {
        drop(&mut world, STONE, b, (8, 4));
    }
    away_road(&mut world, b, (6, 7));
    drop(&mut world, STONE, b, (6, 7));
    drop(&mut world, PLANK, a, b);
    let plank = world.wares().last().unwrap().0;
    while !matches!(world.carriers[0].task, CarrierTask::Carry { .. }) {
        world.step();
        assert!(world.tick() < 300, "carries");
    }
    // The ordering tick: no check yet.
    assert_eq!(world.ware(plank).unwrap().wait_score, 0);
    world.step();
    assert!(world.ware(plank).unwrap().wait_score > 0, "checked");
}

// TRN-CARRY T35 (TRN-CARRY-3.2, corrected): a carrier that may not enter
// the full flag F walks on over the road's inner fields and waits on the
// last field before F, not at the next field it reaches.
// spec: TRN-CARRY-3.2 (TRN-CARRY T35), TRN-CARRY-4.1, TRN-CARRY-4.2
#[test]
fn t35_a_refused_carrier_waits_on_the_last_field_before_the_flag() {
    let mut world = transport_world(16, 10);
    let (a, f) = ((3, 4), (7, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, f), Ok(()));
    assert_eq!(road(&mut world, a, f), Ok(()));
    let path = world.roads().next().unwrap().path.clone();
    assert_eq!(path.len(), 5, "four steps");
    let before_f = path[3];
    // F is full: five stones for one way out, a sixth for another.
    assert_eq!(flag(&mut world, (10, 4)), Ok(()));
    assert_eq!(flag(&mut world, (8, 7)), Ok(()));
    away_road(&mut world, f, (10, 4));
    for _ in 0..5 {
        drop(&mut world, STONE, f, (10, 4));
    }
    away_road(&mut world, f, (8, 7));
    drop(&mut world, STONE, f, (8, 7));
    assert_eq!(world.wares_at(f).len(), 6);
    drop(&mut world, PLANK, a, f);
    for _ in 0..300 {
        world.step();
    }
    let c = &world.carriers[0];
    assert!(c.ware.is_some(), "it carries the plank");
    assert_eq!((c.walker.at, c.walker.next), (before_f, None));
    assert!(matches!(
        c.task,
        CarrierTask::Carry {
            entry: Entry::Open { .. },
            ..
        }
    ));
}

// TRN-CARRY T56 (TRN-CARRY-3.2): a carrier refused while walking makes no
// entry check in the tick its step onto the last field before F ends and
// it stops there, even when that tick is on its 10-tick grid; its first
// check as a waiting carrier is the next grid tick.
// spec: TRN-CARRY-3.2 (TRN-CARRY T56)
#[test]
fn t56_no_check_in_the_tick_the_carrier_comes_to_a_stop() {
    let mut world = transport_world(16, 10);
    let (a, f) = ((3, 4), (7, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, f), Ok(()));
    assert_eq!(road(&mut world, a, f), Ok(()));
    let before_f = world.roads().next().unwrap().path[3];
    // F is full as in T35.
    assert_eq!(flag(&mut world, (10, 4)), Ok(()));
    assert_eq!(flag(&mut world, (8, 7)), Ok(()));
    away_road(&mut world, f, (10, 4));
    for _ in 0..5 {
        drop(&mut world, STONE, f, (10, 4));
    }
    away_road(&mut world, f, (8, 7));
    drop(&mut world, STONE, f, (8, 7));
    drop(&mut world, PLANK, a, f);
    let plank = world.wares().last().unwrap().0;
    while !matches!(world.carriers[0].task, CarrierTask::Carry { .. }) {
        world.step();
        assert!(world.tick() < 300, "carries");
    }
    // The update in which it comes to a stop on the field before F.
    let mut probe = world.clone();
    let stop = loop {
        let stepping = probe.carriers[0].walker.next.is_some();
        let tick = probe.tick();
        probe.step();
        let w = &probe.carriers[0].walker;
        if stepping && w.next.is_none() && w.is_walking() {
            assert_eq!(w.at, before_f);
            break tick;
        }
        assert!(probe.tick() < 600, "it stops");
    };
    // Its grid on that tick.
    let number = u64::from(world.carriers[0].number);
    world.carriers[0].number += ((10 - (number + stop) % 10) % 10) as u32;
    while world.tick() < stop {
        world.step();
    }
    let score = |w: &World| w.ware(plank).unwrap().wait_score;
    let before = score(&world);
    world.step();
    assert_eq!(world.carriers[0].walker.at, before_f);
    assert_eq!(score(&world), before, "no check in the stopping tick");
    for _ in 1..10 {
        world.step();
        assert_eq!(score(&world), before, "none off the grid");
    }
    world.step();
    assert_ne!(score(&world), before, "the next grid tick checks");
}

// TRN-CARRY-3.2a (T29): a stopped carrier admitted at one of its checks
// has its walk ordered again in that tick: it shows the walk (progress 0)
// where it stands from the admission tick, and its first step advances in
// the next.
// spec: TRN-CARRY-3.2a (T29), TRN-CARRY-3.2, TRN-MOVE-2.1
#[test]
fn t29_the_admitted_carrier_shows_its_walk_from_the_admission_tick() {
    let (mut world, a, m, b) = two_step_road();
    // B full as in T6: five stones one way, a sixth the other.
    assert_eq!(flag(&mut world, (8, 4)), Ok(()));
    assert_eq!(flag(&mut world, (6, 7)), Ok(()));
    away_road(&mut world, b, (8, 4));
    for _ in 0..5 {
        drop(&mut world, STONE, b, (8, 4));
    }
    away_road(&mut world, b, (6, 7));
    drop(&mut world, STONE, b, (6, 7));
    drop(&mut world, PLANK, a, b);
    for _ in 0..200 {
        world.step();
    }
    let c = &world.carriers[0];
    assert_eq!(
        (c.walker.at, c.walker.next, c.walker.has_walk()),
        (m, None, false)
    );
    let stone = world.flag_wares.get_mut(&b).unwrap().remove(0);
    world.wares[stone] = None;
    while matches!(
        world.carriers[0].task,
        CarrierTask::Carry {
            entry: Entry::Open { .. },
            ..
        }
    ) {
        world.step();
        assert!(world.tick() < 400, "admitted");
    }
    // The admission tick: still on M, the walk ordered.
    let w = &world.carriers[0].walker;
    assert_eq!((w.at, w.next, w.progress()), (m, None, 0.0));
    assert!(w.has_walk());
    world.step();
    let w = &world.carriers[0].walker;
    assert_eq!((w.at, w.next), (m, Some(b)));
    assert!(w.progress() > 0.0);
}

// TRN-CARRY T33 (TRN-CARRY-2.2): the tick a carrier's walk back to its
// idle field ends is no look tick, even when it falls on the carrier's
// grid: the ware waiting at its road's other end is claimed at the next
// tick of the grid, 20 ticks later.
// spec: TRN-CARRY-2.2 (TRN-CARRY T33)
#[test]
fn t33_the_arrival_on_the_idle_field_is_no_look_tick() {
    let (mut world, a, m, b) = two_step_road();
    // Walking back from A to its idle field M; a ware for A waits at B.
    let mut walker = crate::movement::Walker::new(a);
    walker.walk([m]);
    world.carriers[0].walker = walker;
    drop(&mut world, PLANK, b, a);
    // The tick of the step in which the walk ends.
    let arrival = |world: &mut World| {
        while world.carriers[0].walker.at != m {
            world.step();
            assert!(world.tick() < 200, "arrives");
        }
        world.tick() - 1
    };
    let t = arrival(&mut world.clone());
    let period = IDLE_LOOK_PERIOD as u32;
    world.carriers[0].number = 100 * period - (t % IDLE_LOOK_PERIOD) as u32;
    assert_eq!(arrival(&mut world), t);
    let number = u64::from(world.carriers[0].number);
    assert!((number + t).is_multiple_of(IDLE_LOOK_PERIOD), "a grid tick");
    assert!(!world.carriers[0].walker.is_walking());
    for _ in 1..IDLE_LOOK_PERIOD {
        assert_eq!(world.carriers[0].task, CarrierTask::Idle, "no look");
        world.step();
    }
    world.step();
    assert!(matches!(
        world.carriers[0].task,
        CarrierTask::Fetch { flag, .. } if flag == b
    ));
}

// ECO-REQ-5.3e (ECO-REQ T17, Q-282): the ware a carrier walks to fetch
// loses its next flag and the carrier's claim (redirected by the matching).
// The re-check computes the next flag again while the carrier walks on;
// arriving at its flag, the carrier finds no ware claimed by it and stands
// without picking it up, and in the next tick it chooses again and takes
// it, its pick-up starting in that tick.
// spec: ECO-REQ-5.3e (ECO-REQ T17), TRN-CARRY-2.5, ECO-DIST-1.6
#[test]
fn a_fetching_carrier_finds_the_ware_without_a_next_flag() {
    let (mut world, a, _, b) = two_step_road();
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    let ware = world.wares().last().unwrap().0;
    world.step();
    assert_eq!(
        world.carriers[0].task,
        CarrierTask::Fetch { ware, flag: a },
        "fetching"
    );
    let w = world.wares[ware].as_mut().unwrap();
    (w.next, w.claimed_by) = (None, None);
    let fetching = |world: &World| matches!(world.carriers[0].task, CarrierTask::Fetch { .. });
    while fetching(&world) {
        world.step();
        assert!(world.tick() < 200, "arrives");
        assert_eq!(
            world.ware(ware).unwrap().claimed_by,
            None,
            "nobody claims it"
        );
    }
    // The arrival tick: on A, no pick-up, the ware routed again by the
    // re-check and free.
    assert_eq!(world.carriers[0].walker.at, a);
    assert!(!matches!(
        world.carriers[0].task,
        CarrierTask::PickUp { .. }
    ));
    let w = world.ware(ware).unwrap();
    assert_eq!(
        (w.next, w.claimed_by, w.location),
        (Some(b), None, WareLocation::Flag(a))
    );
    world.step();
    assert!(matches!(
        world.carriers[0].task,
        CarrierTask::PickUp { second: false, .. }
    ));
    assert_eq!(world.ware(ware).unwrap().claimed_by, Some(0));
}

// ECO-REQ T21 (ECO-REQ-5.3e, Q-282): the ware a carrier walks to fetch is
// redirected over another road of its flag; its claim is gone at once and
// the re-check gives it that road's flag. The first carrier walks on,
// finds no ware of its own on arrival, stands there in the arrival tick and
// from the next walks back to its idle field; the other road's carrier
// takes the ware.
// spec: ECO-REQ-5.3e (ECO-REQ T21), TRN-CARRY-2.5, ECO-DIST-1.6
#[test]
fn t21_a_released_carrier_stands_on_arrival_and_goes_back() {
    let (mut world, a, m, b) = two_step_road();
    let c = (3, 6);
    assert_eq!(flag(&mut world, c), Ok(()));
    assert_eq!(road(&mut world, a, c), Ok(()));
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    let ware = world.wares().last().unwrap().0;
    world.step();
    assert_eq!(world.carriers[0].task, CarrierTask::Fetch { ware, flag: a });
    // The matching's redirect: over C now, the next flag and claim gone.
    let w = world.wares[ware].as_mut().unwrap();
    (w.destination, w.next, w.claimed_by) = (Some(c), None, None);
    while matches!(world.carriers[0].task, CarrierTask::Fetch { .. }) {
        world.step();
        assert!(world.tick() < 200, "arrives");
    }
    // The arrival tick: standing on A, no pick-up.
    let carrier = &world.carriers[0];
    assert_eq!((carrier.walker.at, carrier.task), (a, CarrierTask::Idle));
    assert!(!carrier.walker.is_walking());
    assert_ne!(world.ware(ware).unwrap().claimed_by, Some(0));
    world.step();
    assert!(
        world.carriers[0].walker.is_walking(),
        "back to its idle field"
    );
    while world.carriers[0].walker.is_walking() {
        world.step();
    }
    assert_eq!(world.carriers[0].walker.at, m);
    while world
        .ware(ware)
        .is_some_and(|w| w.location == WareLocation::Flag(a))
    {
        world.step();
        assert!(world.tick() < 400, "taken");
    }
    assert_eq!(
        world.ware(ware).unwrap().location,
        WareLocation::Carried(1),
        "by C's carrier"
    );
}

// ECO-REQ-5.3e (Q-285): a carrier whose claim is released while he picks
// the ware up (the matching's redirect, here over another road of the flag)
// takes nothing when the drop half ends, plays the pick-up half all the
// same and goes on without a ware, so no two carriers ever hold one ware;
// the other road's carrier takes it.
// spec: ECO-REQ-5.3e (Q-285)
#[test]
fn a_carrier_whose_claim_goes_during_the_pick_up_takes_nothing() {
    let (mut world, a, _, b) = two_step_road();
    let c = (3, 6);
    assert_eq!(flag(&mut world, c), Ok(()));
    assert_eq!(road(&mut world, a, c), Ok(()));
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    let ware = world.wares().last().unwrap().0;
    let picking = |world: &World| {
        matches!(
            world.carriers[0].task,
            CarrierTask::PickUp { second: false, .. }
        )
    };
    while !picking(&world) {
        world.step();
        assert!(world.tick() < 300, "picks it up");
    }
    // The redirect during the pick-up.
    let w = world.wares[ware].as_mut().unwrap();
    (w.destination, w.next, w.claimed_by) = (Some(c), None, None);
    while picking(&world) {
        world.step();
    }
    assert_eq!(world.carriers[0].ware, None, "nothing taken");
    assert_eq!(world.ware(ware).unwrap().location, WareLocation::Flag(a));
    assert!(
        matches!(
            world.carriers[0].task,
            CarrierTask::PickUp { second: true, .. }
        ),
        "the pick-up half all the same"
    );
    while matches!(world.carriers[0].task, CarrierTask::PickUp { .. }) {
        world.step();
    }
    assert_eq!(world.carriers[0].ware, None);
    while world.ware(ware).unwrap().location == WareLocation::Flag(a) {
        world.step();
        assert!(world.tick() < 600, "taken");
    }
    assert_eq!(world.ware(ware).unwrap().location, WareLocation::Carried(1));
}

// TRN-CARRY-3.3: a ware for the building at the next flag is handed over
// without the room check, so a full flag does not hold the carrier.
#[test]
fn a_ware_for_the_building_at_a_full_flag_is_delivered() {
    let (mut world, a, _, b) = two_step_road();
    let at = world.terrain.neighbour(b, 4).unwrap();
    assert_eq!(building(&mut world, crate::test_support::SMALL, at), Ok(()));
    assert_eq!(flag(&mut world, (8, 4)), Ok(()));
    away_road(&mut world, b, (8, 4));
    for _ in 0..5 {
        drop(&mut world, STONE, b, (8, 4));
    }
    drop(&mut world, STONE, b, a);
    assert_eq!(world.wares_at(b).len(), 6);
    let target = world.target_of(at).unwrap();
    let ware = world.new_ware_at(PLANK, a, Some(b), Some(target));
    for _ in 0..300 {
        world.step();
    }
    // Without the exception the carrier would wait with the plank for
    // room, and the stone at B needs that same carrier to leave.
    assert!(world.wares[ware].is_none(), "the plank went into the site");
    assert_eq!(world.site(at).unwrap().delivered, vec![(PLANK, 1)]);
}

// TRN-CARRY-3.3, 3.3a: into the building at the flag. The carrier stands
// on the building's field with its ware in the tick its walk ends (A) and
// in A + 1; in A + 2 the building receives the ware and the walk out to the
// flag is ordered at progress 0, advancing from A + 3 (E-trace-005: carrier
// 90237 into the hunter's site (140, 49), on its field from trace tick 773,
// standing 773 and 774, walking from 775).
// spec: TRN-CARRY-3.3, TRN-CARRY-3.3a
#[test]
fn a_carrier_stands_two_ticks_in_the_building_it_delivers_to() {
    let (mut world, a, _, b) = two_step_road();
    let at = world.terrain.neighbour(b, 4).unwrap();
    assert_eq!(building(&mut world, crate::test_support::SMALL, at), Ok(()));
    let target = world.target_of(at).unwrap();
    world.new_ware_at(PLANK, a, Some(b), Some(target));
    let mut steps = 0;
    while world.carriers[0].walker.at != at {
        world.step();
        steps += 1;
        assert!(steps < 400, "never reached the building");
    }
    let c = |w: &World| w.carriers[0].clone();
    let delivered = |w: &World| w.site(at).unwrap().delivered.clone();
    // A: standing, the ware still in hand.
    assert!(!c(&world).walker.is_walking());
    assert!(c(&world).ware.is_some());
    assert!(delivered(&world).is_empty());
    // A + 1: still standing with it.
    world.step();
    assert!(!c(&world).walker.is_walking());
    assert!(c(&world).ware.is_some());
    assert!(delivered(&world).is_empty());
    // A + 2: the building has it; the walk out is ordered, at progress 0 on
    // the building's field.
    world.step();
    assert_eq!(c(&world).ware, None);
    assert_eq!(delivered(&world), vec![(PLANK, 1)]);
    assert!(c(&world).walker.has_walk());
    assert_eq!(
        (c(&world).walker.at, c(&world).walker.progress()),
        (at, 0.0)
    );
    // A + 3: it advances.
    world.step();
    assert!(c(&world).walker.progress() > 0.0);
}

// TRN-CARRY-3.3 (TRN-CARRY T60, Q-230 c): the building removed while the
// carrier walks a ware into it. The walk in goes on to its field (A); in the
// hand-over tick, A + 1, nothing is delivered and the ware stays in his
// hands (the road counts the trip), and the walk back out is ordered at
// progress 0 in that tick: one standing row, the first move in A + 2 (R6:
// carrier 16778, Stand 0 on the site's field at trace tick 8414, Walk 0 at
// 8415, moving from 8416). He puts the ware down at the flag like any ware
// (drop and pickup actions, from his arrival tick), where it lies without a
// destination; he is not sent home, nothing is lost. TRN-CARRY-3.3a (the
// put-down of a ware a building did not take): back on the flag in A he
// stands, and from A + 1 he puts it down as at the end of a carrying walk:
// standing in A + 1, the drop-time action A + 2 .. A + 5, the ware lying at
// the flag from A + 6 (R6: Stand at trace ticks 8453 and 8454, PutDown 8455
// .. 8458, the ware at the flag from 8459).
// spec: TRN-CARRY-3.3 (TRN-CARRY T60), TRN-CARRY-3.3a, ECO-REQ-5.5
#[test]
fn t60_a_ware_for_a_building_removed_meanwhile_comes_back_out() {
    let (mut world, a, _, b) = two_step_road();
    let at = world.terrain.neighbour(b, 4).unwrap();
    assert_eq!(building(&mut world, crate::test_support::SMALL, at), Ok(()));
    let target = world.target_of(at).unwrap();
    let ware = world.new_ware_at(PLANK, a, Some(b), Some(target));
    let mut steps = 0;
    while !(matches!(
        world.carriers[0].task,
        CarrierTask::Deliver { back: false, .. }
    ) && world.carriers[0].walker.is_walking())
    {
        world.step();
        steps += 1;
        assert!(steps < 400, "never walked in");
    }
    let road = world.carriers[0].road;
    let trips = world.roads[road].as_ref().unwrap().traffic.transport_count;
    world.submit(Command::RemoveBuilding { player: P0, at });
    world.step();
    assert!(world.building_at(at).is_none(), "removed");
    // The walk in goes on to the building's field.
    while world.carriers[0].walker.at != at {
        world.step();
        assert!(world.wares[ware].is_some());
        steps += 1;
        assert!(steps < 600, "never reached the field");
    }
    // A standing with it; A + 1 the hand-over tick: no delivery, the ware
    // stays his, the trip counted, the walk out ordered at progress 0.
    world.step();
    let c = world.carriers[0].clone();
    assert_eq!(c.ware, Some(ware));
    assert_eq!(world.ware(ware).unwrap().location, WareLocation::Carried(0));
    assert_eq!(world.ware(ware).unwrap().target, None);
    assert_eq!(
        world.roads[road].as_ref().unwrap().traffic.transport_count,
        trips + 1
    );
    assert_eq!((c.walker.at, c.walker.progress()), (at, 0.0));
    assert!(c.walker.has_walk(), "the walk out ordered");
    // A + 2: the first move.
    world.step();
    let c = world.carriers[0].clone();
    assert!(c.walker.at != at || c.walker.progress() > 0.0, "moving");
    // Back on the flag: the put-down, then the ware lies there, no
    // destination; the carrier keeps his road.
    while world.carriers[0].walker.at != b || world.carriers[0].walker.is_walking() {
        world.step();
        steps += 1;
        assert!(steps < 900, "never back on the flag");
    }
    let task = |world: &World| world.carriers[0].task;
    assert!(
        matches!(task(&world), CarrierTask::Out { .. }),
        "A: standing"
    );
    world.step();
    assert!(
        matches!(task(&world), CarrierTask::Stop { exchange: None, .. }),
        "A + 1: standing, {:?}",
        task(&world)
    );
    world.step();
    assert!(
        matches!(task(&world), CarrierTask::PutDown { second: false, .. }),
        "A + 2: the drop-time action"
    );
    for _ in 0..3 {
        world.step();
        assert_eq!(world.ware(ware).unwrap().location, WareLocation::Carried(0));
    }
    world.step();
    assert_eq!(
        world.ware(ware).unwrap().location,
        WareLocation::Flag(b),
        "A + 6"
    );
    assert_eq!(world.ware(ware).unwrap().destination, None);
    assert_eq!(world.carriers[0].road, road);
    assert!(!matches!(world.carriers[0].task, CarrierTask::Retired));
}

/// Flags A, B and a road A-M-B, plus Z beyond A and C beyond B, each on a
/// road whose carrier never works (wares for them stay where they are).
fn exchange_world() -> (World, Point, Point, Point, Point) {
    let (mut world, a, _, b) = two_step_road();
    let (z, c) = ((1, 4), (8, 4));
    assert_eq!(flag(&mut world, z), Ok(()));
    away_road(&mut world, z, a);
    assert_eq!(flag(&mut world, c), Ok(()));
    away_road(&mut world, b, c);
    (world, a, b, z, c)
}

/// Step until the first carrier holds a ware and walks to `to`.
fn step_until_carrying(world: &mut World) {
    for _ in 0..400 {
        world.step();
        let c = &world.carriers[0];
        if c.ware.is_some() && matches!(c.task, CarrierTask::Carry { .. }) {
            return;
        }
    }
    panic!("the carrier never started");
}

// TRN-CARRY T38 (TRN-CARRY-3.7): the ware bound for an exchange loses its
// claim on the carrier's way (the matching's redirect, ECO-REQ-5.3e): the
// exchange is undone at once and, still walking, he checks again and binds
// the other ready ware.
// spec: TRN-CARRY-3.7 (TRN-CARRY T38), ECO-REQ-5.3e
#[test]
fn t38_an_exchange_whose_ware_is_redirected_on_the_way_is_undone() {
    let (mut world, a, b, z, c) = exchange_world();
    let log = world.new_ware_at(LOG, b, Some(z), None);
    let other = world.new_ware_at(LOG, b, Some(z), None);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    step_until_carrying(&mut world);
    world.step();
    let bound = |world: &World| match world.carriers[0].task {
        CarrierTask::Carry {
            entry: Entry::Exchange(w),
            ..
        } => Some(w),
        _ => None,
    };
    assert_eq!(bound(&world), Some(log));
    assert!(world.carriers[0].walker.is_walking());
    // The redirect while he walks.
    let w = world.wares[log].as_mut().unwrap();
    (w.claimed_by, w.next) = (None, None);
    world.step();
    assert_eq!(bound(&world), Some(other), "the other log bound instead");
    assert_eq!(world.ware(other).unwrap().claimed_by, Some(0));
}

// TRN-CARRY T57 (TRN-CARRY-3.7, Q-381): the exchange ware loses its claim in
// the carrier's arrival tick A. At A + 1 he has no exchange ware and looks
// again as one whose place was reserved: another ware ready for the way
// back is claimed and taken in the same two actions, his ware never having
// had a reserved place; with none ready his entry is asked again, as after
// a cancelled exchange.
// spec: TRN-CARRY-3.7 (TRN-CARRY T57)
#[test]
fn t57_an_exchange_ware_released_in_the_arrival_tick_counts_as_none() {
    for ready in [true, false] {
        let (mut world, a, b, z, c) = exchange_world();
        let log = world.new_ware_at(LOG, b, Some(z), None);
        step_to_look_tick(&mut world, 0);
        drop(&mut world, PLANK, a, c);
        while !matches!(world.carriers[0].task, CarrierTask::Stop { .. }) {
            world.step();
            assert!(world.tick() < 400, "arrives");
        }
        assert_eq!(
            world.carriers[0].task,
            CarrierTask::Stop {
                at: b,
                exchange: Some(log)
            }
        );
        // The matching takes the log away in A; another ready one lies there.
        let w = world.wares[log].as_mut().unwrap();
        (w.claimed_by, w.next) = (None, None);
        let other = ready.then(|| world.new_ware_at(LOG, b, Some(z), None));
        world.step();
        let Some(other) = other else {
            assert_eq!(
                world.carriers[0].task,
                CarrierTask::Carry {
                    to: b,
                    entry: Entry::Open { stop: false }
                },
                "{ready}"
            );
            continue;
        };
        assert!(
            matches!(
                world.carriers[0].task,
                CarrierTask::PutDown { exchange: Some(x), bid: false, .. } if x == other
            ),
            "{:?}",
            world.carriers[0].task
        );
        while matches!(world.carriers[0].task, CarrierTask::PutDown { .. }) {
            world.step();
        }
        assert_eq!(
            world.ware(other).unwrap().location,
            WareLocation::Carried(0)
        );
        assert_eq!(world.ware(log).unwrap().location, WareLocation::Flag(b));
    }
}

// ECO-REQ T24 (ECO-REQ-5.3e): the ware bound for an exchange loses its
// claim while the carrier puts his ware down at its flag: the pick-up half
// does not take it; he stands, chooses work afresh and may take it in a
// pick-up of its own.
// spec: ECO-REQ-5.3e (ECO-REQ T24), TRN-CARRY-3.7
#[test]
fn t24_an_exchange_ware_redirected_during_the_put_down_is_not_taken() {
    let (mut world, a, b, z, c) = exchange_world();
    let log = world.new_ware_at(LOG, b, Some(z), None);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    while !matches!(
        world.carriers[0].task,
        CarrierTask::PutDown { second: false, .. }
    ) {
        world.step();
        assert!(world.tick() < 400, "puts down");
    }
    // The redirect during the put-down: the claim goes, the ware stays.
    world.wares[log].as_mut().unwrap().claimed_by = None;
    while matches!(world.carriers[0].task, CarrierTask::PutDown { .. }) {
        world.step();
    }
    assert_eq!(world.carriers[0].ware, None, "the log not taken");
    assert_eq!(world.ware(log).unwrap().location, WareLocation::Flag(b));
    // Then a pick-up of its own.
    while world.ware(log).unwrap().location == WareLocation::Flag(b) {
        world.step();
        assert!(world.tick() < 800, "taken afresh");
    }
    assert_eq!(world.ware(log).unwrap().location, WareLocation::Carried(0));
}

// TRN-CARRY T14: B is full, but a log there is bound for A: the carrier
// claims it at its first check, in the tick after its walk is ordered
// (T36), never stops, and swaps at B.
// spec: TRN-CARRY-3.7 (T14), TRN-CARRY-3.2 (T36)
#[test]
fn t14_exchange_at_a_full_flag() {
    let (mut world, a, b, z, c) = exchange_world();
    for _ in 0..4 {
        world.new_ware_at(STONE, b, Some(c), None);
    }
    let log = world.new_ware_at(LOG, b, Some(z), None);
    world.new_ware_at(STONE, b, Some(c), None);
    assert_eq!(world.wares_at(b).len(), 6);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    step_until_carrying(&mut world);
    // TRN-CARRY-3.2 (T36): its first check in the tick after the walk.
    world.step();
    let plank = world.carriers[0].ware.expect("the plank");
    assert_eq!(
        world.carriers[0].task,
        CarrierTask::Carry {
            to: b,
            entry: Entry::Exchange(log)
        }
    );
    assert_eq!(world.ware(log).unwrap().claimed_by, Some(0));
    let mut arrived = None;
    for _ in 0..200 {
        world.step();
        let c = &world.carriers[0];
        if matches!(c.task, CarrierTask::Carry { to, .. } if to == a) {
            break;
        }
        assert!(!matches!(
            c.task,
            CarrierTask::Carry {
                entry: Entry::Open { .. },
                ..
            }
        ));
        assert_eq!(world.flag_load(b), 6, "plank in, log out");
        if arrived.is_none() && matches!(c.task, CarrierTask::PutDown { .. }) {
            arrived = Some(world.tick());
        }
    }
    let arrived = arrived.expect("reached B");
    // 4 + 4 ticks at B (the step in which the task changed is one before).
    assert_eq!(world.tick() - arrived, 8);
    assert_eq!(world.carriers[0].ware, Some(log));
    assert!(world.wares_at(b).contains(&plank));
    assert!(!world.wares_at(b).contains(&log));
}

// TRN-CARRY T15: the exchange also happens when B has room.
// spec: TRN-CARRY-3.7 (T15)
#[test]
fn t15_exchange_with_room() {
    let (mut world, a, b, z, c) = exchange_world();
    let stone = world.new_ware_at(STONE, b, Some(z), None);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    step_until_carrying(&mut world);
    // TRN-CARRY-3.2 (T36): its first check in the tick after the walk.
    world.step();
    assert_eq!(
        world.carriers[0].task,
        CarrierTask::Carry {
            to: b,
            entry: Entry::Exchange(stone)
        }
    );
    for _ in 0..200 {
        world.step();
        if matches!(world.carriers[0].task, CarrierTask::Carry { to, .. } if to == a) {
            break;
        }
    }
    // Back with the stone, no walk to M in between.
    assert_eq!(world.carriers[0].ware, Some(stone));
    assert_eq!(world.carriers[0].walker.at, b);
}

// TRN-CARRY T16 is `t06_waits_before_a_full_flag`; this adds the end: the
// carrier enters at its second successful check and puts the plank down.
// spec: TRN-CARRY-3.2, TRN-CARRY-4.4 (T16)
#[test]
fn t16_bid_after_room_appears() {
    let (mut world, a, b, _, c) = exchange_world();
    for _ in 0..6 {
        world.new_ware_at(STONE, b, Some(c), None);
    }
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    for _ in 0..300 {
        world.step();
    }
    let plank = world.carriers[0].ware.expect("waiting with the plank");
    assert!(matches!(
        world.carriers[0].task,
        CarrierTask::Carry {
            entry: Entry::Open { stop: true },
            ..
        }
    ));
    // Each refused check at the full flag adds 15 - 7 = 8.
    assert!(world.ware(plank).unwrap().wait_score >= 8 * 20);
    // Two places: the plank has the same next flag as the stones, so the
    // last place would not do (TRN-CARRY-4.2).
    for _ in 0..2 {
        let first = world.wares_at(b)[0];
        world.remove_ware(first);
    }
    for _ in 0..80 {
        world.step();
    }
    assert!(world.wares_at(b).contains(&plank));
    assert_eq!(world.carriers[0].ware, None);
}

// TRN-CARRY T17: a place reserved for a plank counts as lying at B, so a
// worker's product waits although only 5 wares lie there.
// spec: TRN-CARRY-4.3 (T17)
#[test]
fn t17_reserved_place_counts() {
    let (mut world, a, b, z, c) = exchange_world();
    let d = (6, 7);
    assert_eq!(flag(&mut world, d), Ok(()));
    away_road(&mut world, b, d);
    for _ in 0..4 {
        world.new_ware_at(STONE, b, Some(c), None);
    }
    world.new_ware_at(STONE, b, Some(d), None);
    let _ = z;
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    step_until_carrying(&mut world);
    for _ in 0..5 {
        world.step();
    }
    assert!(matches!(
        world.carriers[0].task,
        CarrierTask::Carry {
            entry: Entry::Reserved,
            ..
        }
    ));
    assert_eq!(world.wares_at(b).len(), 5);
    assert_eq!(world.flag_load(b), 6);
    // The worker's check (ECO-WORK-3.4) says no.
    assert!(!world.flag_has_room_for(b, None));
    assert!(!world.flag_reserve_direct(b, STONE, &mut 0));
}

// TRN-CARRY T18: the bar. B is full; X (coin) and Y (log) have each been
// refused 5 times.
// spec: TRN-CARRY-4.4 (T18)
#[test]
fn t18_wait_score_and_bar() {
    let (mut world, _, b, _, c) = exchange_world();
    for _ in 0..6 {
        world.new_ware_at(STONE, b, Some(c), None);
    }
    let (mut x, mut y) = (0, 0);
    for _ in 0..5 {
        assert!(!world.flag_bid(b, COIN, None, &mut x));
        assert!(!world.flag_bid(b, LOG, None, &mut y));
    }
    assert_eq!((x, y), (75, 30));
    let first = world.wares_at(b)[0];
    world.remove_ware(first);
    // A ware left: the bar is -1. X sets it to 75, Y's check fails, X's
    // next check admits it.
    assert!(!world.flag_bid(b, COIN, None, &mut x));
    assert!(!world.flag_bid(b, LOG, None, &mut y));
    assert!(world.flag_bid(b, COIN, None, &mut x));
    assert_eq!(world.flag_bar.get(&b), None);
    assert_eq!(world.flag_load(b), 6);
    // Y waits for the next place and bids again then.
    assert!(!world.flag_bid(b, LOG, None, &mut y));
    assert_eq!(y, 36);
    let next = world.wares_at(b)[0];
    world.remove_ware(next);
    assert!(!world.flag_bid(b, LOG, None, &mut y));
    assert!(world.flag_bid(b, LOG, None, &mut y));
}

// TRN-CARRY T19: the gridlock of Q-164 on one road.
// spec: TRN-CARRY-3.7, TRN-CARRY-4.1 (T19)
#[test]
fn t19_full_flags_flow_by_exchange() {
    let mut world = transport_world(20, 8);
    let (h, f, g) = ((2, 4), (6, 4), (10, 4));
    for at in [h, f, g] {
        assert_eq!(flag(&mut world, at), Ok(()));
    }
    assert_eq!(road(&mut world, h, f), Ok(()));
    assert_eq!(road(&mut world, f, g), Ok(()));
    world.carriers[1].task = CarrierTask::Retired;
    let logs: Vec<usize> = (0..6)
        .map(|_| world.new_ware_at(LOG, f, Some(h), None))
        .collect();
    let planks: Vec<usize> = (0..6)
        .map(|_| world.new_ware_at(PLANK, h, Some(g), None))
        .collect();
    // The first check at a flag only sets its bar (TRN-CARRY-4.4); the
    // carrier is never held for longer than that.
    let mut held = 0;
    for tick in 0..2000 {
        world.step();
        assert!(world.flag_load(h) <= 6 && world.flag_load(f) <= 6, "{tick}");
        let waiting = matches!(
            world.carriers[0].task,
            CarrierTask::Carry {
                entry: Entry::Open { stop: true },
                ..
            }
        );
        held = if waiting { held + 1 } else { 0 };
        assert!(held <= 2, "waiting at tick {tick}");
    }
    // Every log went in; every plank moved on to F.
    assert!(logs.iter().all(|&l| world.ware(l).is_none()));
    assert!(planks.iter().all(|&p| world.wares_at(f).contains(&p)));
}

// TRN-CARRY T7
// spec: TRN-CARRY-4.2 (T7)
#[test]
fn t07_last_place_needs_another_direction() {
    let mut world = transport_world(20, 8);
    let (a, b, c, d) = ((2, 4), (5, 4), (8, 4), (5, 7));
    for f in [a, b, c, d] {
        assert_eq!(flag(&mut world, f), Ok(()));
    }
    for (from, to) in [(a, b), (b, c), (b, d)] {
        assert_eq!(road(&mut world, from, to), Ok(()));
    }
    for _ in 0..5 {
        drop(&mut world, STONE, b, c);
    }
    drop(&mut world, PLANK, a, c);
    let plank = world.wares().last().unwrap().0;
    assert!(!world.flag_has_room(b, plank));
    // With one of the five going to D instead (its route computed for the
    // new destination, TRN-CARRY-5.4), the plank is accepted.
    let one = world.flag_wares[&b][0];
    world.wares[one].as_mut().unwrap().destination = Some(d);
    world.route_ware(one);
    assert!(world.flag_has_room(b, plank));
}

// TRN-CARRY-4.2a (T28): four wares at B all leaving towards C and a fifth
// place counted for a ware on its way. A road carrier's ware passes the
// direction test of the last place (its next flag is B itself); a ware a
// worker or a warehouse's helper is cleared to bring out has no next flag
// until it is put down and never passes it.
// spec: TRN-CARRY-4.2, TRN-CARRY-4.2a (T28), TRN-CARRY-4.3
#[test]
fn t28_a_ware_brought_out_has_no_next_flag_for_the_last_place() {
    let mut world = transport_world(20, 8);
    let (a, b, c, d) = ((2, 4), (5, 4), (8, 4), (5, 7));
    for f in [a, b, c, d] {
        assert_eq!(flag(&mut world, f), Ok(()));
    }
    for (from, to) in [(a, b), (b, c), (b, d)] {
        assert_eq!(road(&mut world, from, to), Ok(()));
    }
    for _ in 0..4 {
        drop(&mut world, STONE, b, c);
    }
    drop(&mut world, PLANK, a, c);
    let plank = world.wares().last().unwrap().0;
    // The fifth place is a building's: a helper or a worker cleared at B.
    world.flag_reserved.insert(b, 1);
    assert_eq!(world.flag_load(b), 5);
    assert!(!world.flag_has_room(b, plank));
    // The fifth place is a road carrier's instead (the road B-D's carrier
    // cleared to bring a ware to B).
    let carrier = world.roads[2].as_ref().unwrap().carrier.unwrap();
    world.carriers[carrier].task = CarrierTask::Carry {
        to: b,
        entry: Entry::Reserved,
    };
    assert!(world.flag_has_room(b, plank));
}

/// Flags A, B, C with roads A-B, B-C (2 steps each) and A-C (5 steps).
fn triangle() -> (World, Point, Point, Point) {
    let mut world = transport_world(20, 12);
    let (a, b, c) = ((4, 2), (6, 2), (8, 2));
    for f in [a, b, c] {
        assert_eq!(flag(&mut world, f), Ok(()));
    }
    assert_eq!(road(&mut world, a, b), Ok(()));
    assert_eq!(road(&mut world, b, c), Ok(()));
    // A five-step road from A to C around the others: block row 2 between.
    let mut long = world.clone();
    for x in 5..8 {
        let index = long.terrain.index((x, 2));
        long.terrain.states[index] |= crate::map::state::STREET;
    }
    let (path, _) = long.find_route(P0, a, c).unwrap();
    assert_eq!(path.len() - 1, 5, "{path:?}");
    let id = world.roads.len();
    let length = world.road_length(&path);
    let number = world.new_number();
    world.roads.push(Some(crate::Road {
        owner: P0,
        number,
        path,
        carrier: None,
        donkey: None,
        length,
        traffic: Traffic::default(),
        water: false,
    }));
    // A road made by hand: the route topology is made again.
    world.route_topology.clear();
    world.spawn_carrier(id);
    (world, a, b, c)
}

// TRN-CARRY T8, T9
// spec: TRN-CARRY-5.1, TRN-CARRY-5.2, TRN-CARRY-5.3 (T8, T9)
#[test]
fn t08_t09_routes() {
    let (mut world, a, b, c) = triangle();
    let lengths: Vec<u32> = world.roads().map(|r| r.length).collect();
    assert_eq!(lengths, [4, 4, 10]);
    assert_eq!(world.next_flag(P0, a, c), Some(b));
    // The A-B carrier is carrying something: A-B costs 3 x 4.
    world.carriers[0].ware = Some(99);
    assert_eq!(world.next_flag(P0, a, c), Some(c));
    assert_eq!(world.next_flag(P0, c, c), None);
}

// TRN-CARRY T31: in an exchange the put-down ware's next flag is computed
// before the carrier holds the other ware, so his own road counts him as
// free (L). The A-B carrier brings a plank for C to B and takes a stone
// there; B-C (here 6 long) has its carrier busy (18), so the plank goes
// back over A (4 + 10 = 14); with the A-B carrier counted as carrying the
// stone (12 + 10 = 22) it would go to C directly.
// spec: TRN-CARRY-3.7, TRN-CARRY-5.3, TRN-CARRY-5.4 (T31)
#[test]
fn t31_the_exchange_routes_the_put_down_ware_with_the_carrier_free() {
    let (mut world, a, b, c) = triangle();
    let bc = world
        .roads
        .iter()
        .position(|r| {
            r.as_ref()
                .is_some_and(|r| r.path[0] == b && *r.path.last().unwrap() == c)
        })
        .unwrap();
    world.roads[bc].as_mut().unwrap().length = 6;
    world.route_topology.clear();
    let bc_carrier = world.roads[bc].as_ref().unwrap().carrier.unwrap();
    world.carriers[bc_carrier].ware = Some(99);
    let ab_carrier = world.carriers.iter().position(|k| k.road == 0).unwrap();
    let stone = world.new_ware_at(STONE, b, Some(a), None);
    let plank = world.new_ware_at(PLANK, a, Some(c), None);
    world
        .flag_wares
        .get_mut(&a)
        .unwrap()
        .retain(|&w| w != plank);
    world.wares[plank].as_mut().unwrap().location = WareLocation::Carried(ab_carrier);
    world.carriers[ab_carrier].ware = Some(plank);
    // The exchange ware is claimed for him (TRN-CARRY-3.7).
    world.wares[stone].as_mut().unwrap().claimed_by = Some(ab_carrier);
    world.put_ware_down(ab_carrier, b, Some(stone), false);
    assert_eq!(world.carriers[ab_carrier].ware, Some(stone));
    assert_eq!(world.wares[plank].as_ref().unwrap().next, Some(a));
    assert_eq!(world.next_flag(P0, b, c), Some(c), "with him carrying");
}

// TRN-ROAD T27 (TRN-ROAD-4.2): a carrier walking to fetch a ware at an end
// of his half when his road is split by a flag on the field his step goes
// to: his walk ends on the new flag, he stands on it in the arrival tick,
// his walk on to the ware's flag is ordered in the next at progress 0 and
// first advances in the tick after; he still fetches the ware.
// spec: TRN-ROAD-4.2 (TRN-ROAD T27), TRN-FREE-6.5
#[test]
fn t27_a_fetching_carrier_stops_on_the_new_flag_ahead() {
    let mut world = transport_world(16, 8);
    // The ware lies at A, the road's first flag; the carrier walks there
    // from his idle field (7, 4), and the split (5, 4) is ahead: the field
    // his step goes to puts him on the first half, A to the new flag.
    let (a, b, split) = ((3, 4), (11, 4), (5, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    while world.carriers[0].walker.next != Some(split) {
        world.step();
        assert!(world.tick() < 300, "on his way");
    }
    assert!(matches!(world.carriers[0].task, CarrierTask::Fetch { flag, .. } if flag == a));
    assert_eq!(flag(&mut world, split), Ok(()));
    assert!(matches!(world.carriers[0].task, CarrierTask::Fetch { flag, .. } if flag == a));
    while world.carriers[0].walker.at != split {
        world.step();
        assert!(world.tick() < 400, "on the new flag");
    }
    let c = &world.carriers[0];
    assert!(!c.walker.has_walk(), "stands in the arrival tick");
    world.step();
    let c = &world.carriers[0];
    assert!(
        c.walker.has_walk() && c.walker.next.is_none(),
        "the walk on ordered"
    );
    assert_eq!((c.walker.at, c.walker.progress_bits), (split, 0));
    world.step();
    assert!(world.carriers[0].walker.next.is_some(), "moving");
    while !matches!(world.carriers[0].task, CarrierTask::PickUp { .. }) {
        world.step();
        assert!(world.tick() < 600, "fetches");
    }
    assert_eq!(world.carriers[0].walker.at, a);
}

// TRN-ROAD T30 (TRN-ROAD-4.2): a carrier walking to fetch a ware at an end
// of his half with the new flag behind him keeps his walk unchanged, as a
// carrying carrier: he is not put back on his field's point.
// spec: TRN-ROAD-4.2 (TRN-ROAD T30)
#[test]
fn t30_a_fetching_carrier_with_the_new_flag_behind_walks_on() {
    let mut world = transport_world(16, 8);
    let (a, b, split) = ((3, 4), (11, 4), (9, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    // Into his first step towards A from the idle field (7, 4).
    while world.carriers[0].walker.next.is_none() || world.carriers[0].walker.progress_bits == 0 {
        world.step();
        assert!(world.tick() < 300, "on his way");
    }
    let before = world.carriers[0].walker.clone();
    assert_eq!(flag(&mut world, split), Ok(()));
    let c = &world.carriers[0];
    assert!(matches!(c.task, CarrierTask::Fetch { flag, .. } if flag == a));
    // The flag command takes a tick, which he walks on.
    assert_eq!((c.walker.at, c.walker.next), (before.at, before.next));
    assert!(
        c.walker.progress_bits > before.progress_bits,
        "his walk goes on, not back to his field's point"
    );
    while !matches!(world.carriers[0].task, CarrierTask::PickUp { .. }) {
        world.step();
        assert!(world.tick() < 600, "fetches");
    }
    assert_eq!(world.carriers[0].walker.at, a);
}

// TRN-ROAD T31 (TRN-ROAD-4.2): a carrier walking to fetch a ware that lies
// at a flag of the other half, the new flag ahead of him: his walk ends on
// the new flag and he keeps his claim until he arrives there; in his arrival
// tick the claim goes and he stands; in the next his walk to his half's idle
// field is ordered at progress 0.
// spec: TRN-ROAD-4.2 (TRN-ROAD T31)
#[test]
fn t31_a_fetching_carrier_keeps_his_claim_to_the_new_flag() {
    let mut world = transport_world(16, 8);
    let (a, b, split) = ((3, 4), (11, 4), (9, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, b, a);
    while world.carriers[0].walker.next != Some((8, 4)) {
        world.step();
        assert!(world.tick() < 300, "on his way to B");
    }
    let CarrierTask::Fetch { ware, .. } = world.carriers[0].task else {
        panic!("fetching: {:?}", world.carriers[0].task);
    };
    assert_eq!(flag(&mut world, split), Ok(()));
    assert!(matches!(
        world.carriers[0].task,
        CarrierTask::Arrive { stand: true, .. }
    ));
    while world.carriers[0].walker.at != split || world.carriers[0].walker.is_walking() {
        assert_eq!(world.ware(ware).unwrap().claimed_by, Some(0), "claim kept");
        world.step();
        assert!(world.tick() < 400, "on the new flag");
    }
    assert_eq!(
        world.ware(ware).unwrap().claimed_by,
        None,
        "released on arrival"
    );
    assert!(
        !world.carriers[0].walker.has_walk(),
        "stands in the arrival tick"
    );
    world.step();
    let c = &world.carriers[0];
    assert!(c.walker.has_walk(), "the walk to the idle field ordered");
    assert_eq!((c.walker.at, c.walker.progress_bits), (split, 0));
}

// TRN-ROAD T33 (TRN-ROAD-4.2): a carrier walking back to his idle field
// after a put-down, without a ware, when a new flag is set on that field
// (his walk's goal, ahead of him): his walk ends on the new flag, he stands
// there in the arrival tick, and in the next his walk to his half's idle
// field is ordered at progress 0.
// spec: TRN-ROAD-4.2 (TRN-ROAD T33)
#[test]
fn t33_a_carrier_going_to_his_idle_field_ends_on_the_new_flag() {
    let mut world = transport_world(16, 8);
    let (a, b, split) = ((3, 4), (11, 4), (7, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    let path = world.road(0).unwrap().path.clone();
    assert_eq!(idle_field(&path), split);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    // Put down at B, then on his way back to the idle field.
    while !(world.carriers[0].task == CarrierTask::Idle
        && world.carriers[0].walker.next == Some((10, 4)))
    {
        world.step();
        assert!(world.tick() < 600, "on his way back");
    }
    assert_eq!(flag(&mut world, split), Ok(()));
    assert!(
        matches!(
            world.carriers[0].task,
            CarrierTask::Arrive { stand: true, .. }
        ),
        "{:?}",
        world.carriers[0].task
    );
    while world.carriers[0].walker.at != split || world.carriers[0].walker.is_walking() {
        world.step();
        assert!(world.tick() < 700, "on the new flag");
    }
    assert!(
        !world.carriers[0].walker.has_walk(),
        "stands in the arrival tick"
    );
    world.step();
    let c = &world.carriers[0];
    assert!(c.walker.has_walk(), "the walk to the idle field ordered");
    assert_eq!((c.walker.at, c.walker.progress_bits), (split, 0));
    let road = world.road(c.road).unwrap();
    assert_eq!(c.walker.destination(), idle_field(&road.path));
}

// TRN-ROAD T34 (TRN-ROAD-4.2): a carrier walking back to his idle field when
// a new flag splits his road beyond the walk's goal (the goal and his
// step's field on his half): the walk does not change; he stands on the old
// idle field in the arrival tick, and his walk to the half's idle field is
// ordered at progress 0 in the next, advancing from the tick after.
// spec: TRN-ROAD-4.2 (TRN-ROAD T34)
#[test]
fn t34_a_split_beyond_the_idle_walks_goal_keeps_the_walk() {
    let mut world = transport_world(16, 8);
    let (a, b, old_idle, split) = ((3, 4), (11, 4), (7, 4), (5, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    assert_eq!(idle_field(&world.road(0).unwrap().path), old_idle);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    while !(world.carriers[0].task == CarrierTask::Idle
        && world.carriers[0].walker.next == Some((10, 4)))
    {
        world.step();
        assert!(world.tick() < 600, "on his way back");
    }
    let progress = world.carriers[0].walker.progress();
    // The split (the helper's tick moves him on).
    assert_eq!(flag(&mut world, split), Ok(()));
    let c = &world.carriers[0];
    assert_eq!(
        (c.walker.next, c.walker.destination()),
        (Some((10, 4)), old_idle),
        "the walk unchanged"
    );
    assert!(c.walker.progress() > progress, "at its rate");
    let new_idle = idle_field(&world.road(c.road).unwrap().path);
    assert_ne!(new_idle, old_idle);
    while world.carriers[0].walker.at != old_idle || world.carriers[0].walker.is_walking() {
        world.step();
        assert!(world.tick() < 700, "on the old idle field");
    }
    assert!(
        !world.carriers[0].walker.has_walk(),
        "stands in the arrival tick"
    );
    world.step();
    let w = &world.carriers[0].walker;
    assert!(w.has_walk(), "the walk to the half's idle field ordered");
    assert_eq!(
        (w.at, w.progress_bits, w.destination()),
        (old_idle, 0, new_idle)
    );
    world.step();
    assert!(world.carriers[0].walker.next.is_some(), "it advances");
}

// TRN-FREE T43 (TRN-FREE-6.1): a road walk that starts on an inner field of
// a road searches from that road's first flag A: if the route's next flag
// is the road's other flag B he walks along the road to B, else to A and
// on; which end is nearer does not count.
// spec: TRN-FREE-6.1 (TRN-FREE T43)
#[test]
fn free_t43_a_walk_from_a_road_field_searches_from_the_first_flag() {
    let (a, b, f) = ((2, 4), (10, 4), (6, 8));
    // A to F directly is the cheapest route from A: to A, though the field
    // lies next to B.
    let (world, own) = own_road_network(false, [8, 5, 5]);
    let path = world.road(own).unwrap().path.clone();
    let field = path[path.len() - 2];
    let mut expected = along(&path, field, a);
    expected.extend(world.road_walk(P0, a, f).unwrap());
    assert_eq!(world.road_walk_from(P0, field, f), Some(expected));
    // A to F dear: the route from A goes over B, so straight to B.
    let (world, own) = own_road_network(false, [8, 30, 5]);
    let path = world.road(own).unwrap().path.clone();
    let field = path[1];
    let mut expected = along(&path, field, b);
    expected.extend(world.road_walk(P0, b, f).unwrap());
    assert_eq!(world.road_walk_from(P0, field, f), Some(expected));
}

// TRN-CARRY T40 (TRN-CARRY-4.3): a road carrier's reserved place counts
// without a gap until his put-down ends, his arrival tick included (he
// stands on the flag one tick before the put-down begins).
// spec: TRN-CARRY-4.3 (TRN-CARRY T40), TRN-CARRY-3.3a
#[test]
fn t40_a_reserved_place_counts_in_the_arrival_tick() {
    let (mut world, a, _, b) = two_step_road();
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, b);
    let mut seen = false;
    for _ in 0..300 {
        world.step();
        let task = world.carriers[0].task;
        let counted = world.flag_reserved.get(&b).copied().unwrap_or(0);
        if let CarrierTask::Stop { at, exchange: None } = task {
            assert_eq!(at, b);
            assert_eq!(counted, 1, "counted while standing");
            assert!(world.carrier_reserved(b));
            seen = true;
        }
        if matches!(task, CarrierTask::PutDown { second: true, .. }) {
            break;
        }
    }
    assert!(seen, "an arrival tick");
}

// TRN-CARRY T53 (TRN-CARRY-4.3, 3.7, Q-377): a carrier whose place at B was
// reserved and who claims a ware for the way back as his put-down begins
// (he looks again) keeps his place through the drop-time action, while the
// claimed ware still lies at B: B counts both, one more than before he came.
// When the drop-time action ends his ware lies at B, his place is used and
// the other ware is in his hands.
// spec: TRN-CARRY-4.3, TRN-CARRY-3.7 (TRN-CARRY T53)
#[test]
fn t53_a_reserved_exchange_keeps_its_place_through_the_drop() {
    let (mut world, a, _, b) = two_step_road();
    // His ware goes on beyond B, so it stays lying there.
    let c = (8, 4);
    assert_eq!(flag(&mut world, c), Ok(()));
    away_road(&mut world, b, c);
    step_to_look_tick(&mut world, 0);
    drop(&mut world, PLANK, a, c);
    // His place at B is granted at the entry check, with nothing at B yet.
    while !matches!(
        world.carriers[0].task,
        CarrierTask::Carry {
            entry: Entry::Reserved,
            ..
        }
    ) {
        world.step();
        assert!(world.tick() < 300, "cleared to B");
    }
    // A ware for the way back comes to lie at B after the entry check.
    drop(&mut world, PLANK, b, a);
    assert_eq!(world.flag_load(b), 2);
    let mut seen = false;
    for _ in 0..300 {
        world.step();
        match world.carriers[0].task {
            CarrierTask::PutDown {
                second: false,
                exchange: Some(_),
                bid: true,
                ..
            } => {
                assert_eq!(world.flag_reserved.get(&b).copied(), Some(1));
                assert_eq!(world.wares_at(b).len(), 1, "the claimed ware lies");
                assert_eq!(world.flag_load(b), 2, "both counted");
                assert!(world.carrier_reserved(b));
                seen = true;
            }
            CarrierTask::PutDown { second: true, .. } => {
                assert_eq!(world.flag_reserved.get(&b).copied().unwrap_or(0), 0);
                assert_eq!(world.flag_load(b), 1, "his ware lies, the other is held");
                assert!(world.carriers[0].ware.is_some());
                break;
            }
            _ => {}
        }
    }
    assert!(seen, "a drop-time action with the exchange claimed");
}

// TRN-CARRY T39 (TRN-CARRY-5.4): the flag a ware takes after a flag G (for
// the last-place test of 4.2 and the road swap of 3.6) is remembered with G:
// asked again from G it is the same answer although a fresh search would now
// go the other way (B-C made dear: from B over A); a ware renumbering of the
// player renews it, and so does the ware's own next-flag search.
// spec: TRN-CARRY-5.4 (TRN-CARRY T39)
#[test]
fn t39_the_flag_after_g_is_remembered() {
    let (mut world, a, b, c) = triangle();
    let plank = world.new_ware_at(PLANK, a, Some(c), None);
    assert_eq!(world.next_flag_after(plank, b), Some(c));
    let bc = world
        .roads
        .iter()
        .position(|r| {
            r.as_ref()
                .is_some_and(|r| r.path[0] == b && *r.path.last().unwrap() == c)
        })
        .unwrap();
    world.roads[bc].as_mut().unwrap().length = 40;
    world.route_topology.clear();
    assert_eq!(world.next_flag(P0, b, c), Some(a), "a fresh search");
    assert_eq!(world.next_flag_after(plank, b), Some(c), "remembered");
    world.route_store.empty_ware(P0);
    assert_eq!(world.next_flag_after(plank, b), Some(a), "renumbered");
    world.roads[bc].as_mut().unwrap().length = 2;
    world.route_topology.clear();
    world.route_ware(plank);
    assert_eq!(world.ware(plank).unwrap().next_after, None, "cleared");
    assert_eq!(world.next_flag_after(plank, b), Some(c));
}

// TRN-CARRY-5.3: "filled" means assigned. A carrier still on its way from
// the warehouse to its road, carrying nothing, is free: A-B costs L, not
// 3L; with a ware it is busy wherever it walks.
// spec: TRN-CARRY-5.3 (T9)
#[test]
fn an_assigned_carrier_on_its_way_is_free() {
    let (mut world, a, b, c) = triangle();
    world.carriers[0].task = CarrierTask::Arrive {
        flag: a,
        stand: false,
    };
    assert_eq!(world.route_cost(P0, a, c), Some(8));
    assert_eq!(world.next_flag(P0, a, c), Some(b));
    world.carriers[0].ware = Some(99);
    assert_eq!(world.route_cost(P0, a, c), Some(10));
    assert_eq!(world.next_flag(P0, a, c), Some(c));
}

// ECO-DIST-1.6, TRN-CARRY-5.4: a ware lying at a flag keeps the next flag
// computed when it came there. The A-B carrier then carrying (3L, TRN-CARRY-
// 5.3) makes A-C the cheaper way, but the re-checks find B still valid and
// leave it (counting 1 to 200); the 201st re-check computes it again (the
// test before the rise, 9d69dd5). A ware whose next flag's road is gone
// computes it at the next re-check.
// spec: ECO-DIST-1.6, TRN-CARRY-5.4, TRN-CARRY-5.3
#[test]
fn a_lying_ware_keeps_its_next_flag_while_the_costs_change() {
    let (mut world, a, b, c) = triangle();
    let ware = world.new_ware_at(PLANK, a, Some(c), None);
    let next = |w: &World| w.wares[ware].as_ref().unwrap().next;
    assert_eq!(next(&world), Some(b));
    world.carriers[0].ware = Some(99);
    assert_eq!(world.next_flag(P0, a, c), Some(c));
    for _ in 0..200 {
        world.recheck_wares();
    }
    assert_eq!(next(&world), Some(b));
    world.recheck_wares();
    assert_eq!(next(&world), Some(c));
    // The way over C's road removed: computed again at the next re-check.
    world.carriers[0].ware = None;
    world.recheck_wares();
    assert_eq!(next(&world), Some(c));
    let long = world
        .roads
        .iter()
        .position(|r| r.as_ref().is_some_and(|r| r.path.len() == 6))
        .unwrap();
    world.remove_road(long);
    world.recheck_wares();
    assert_eq!(next(&world), Some(b));
}

// AI-CON-8.3 c: the computer player's list of route flags counts a full
// flag (4L) but not a busy carrier (3L): with the A-B carrier carrying, a
// ware goes A-C directly (10 < 12), the list stays A, B, C (8 < 10).
// spec: AI-CON-8.3, TRN-CARRY-5.3 (T9)
#[test]
fn the_route_flag_list_leaves_busy_roads_at_their_length() {
    let (mut world, a, b, c) = triangle();
    assert_eq!(world.ware_route_flags(P0, a, c), Some(vec![a, b, c]));
    world.carriers[0].ware = Some(99);
    assert_eq!(world.next_flag(P0, a, c), Some(c));
    assert_eq!(world.ware_route_flags(P0, a, c), Some(vec![a, b, c]));
    assert_eq!(world.ware_route_flags(P0, c, c), Some(vec![c]));
    let lone = (14, 9);
    assert_eq!(flag(&mut world, lone), Ok(()));
    assert_eq!(world.ware_route_flags(P0, a, lone), None);
}

// AI-CON-8.3 c (AI-CON T55): of equally cheap routes the list of route
// flags is read out back from the destination by each flag's road order
// (TRN-CARRY-5.4): with A-C as long as A-B-C (8), C's older road wins.
// spec: AI-CON-8.3 (T55), TRN-CARRY-5.4
#[test]
fn t55_the_route_flag_list_on_equal_routes_follows_the_road_order() {
    let (mut world, a, b, c) = triangle();
    let long = world
        .roads
        .iter()
        .position(|r| r.as_ref().is_some_and(|r| r.path.len() == 6))
        .unwrap();
    world.roads[long].as_mut().unwrap().length = 8;
    world.route_topology.clear();
    assert_eq!(world.ware_route_flags(P0, a, c), Some(vec![a, b, c]));
    // The long road older than B-C: read out over it.
    let bc = world
        .roads
        .iter()
        .position(|r| {
            r.as_ref()
                .is_some_and(|r| r.path[0] == b && r.path.len() == 3)
        })
        .unwrap();
    let (n_bc, n_long) = (
        world.roads[bc].as_ref().unwrap().number,
        world.roads[long].as_ref().unwrap().number,
    );
    world.roads[bc].as_mut().unwrap().number = n_long;
    world.roads[long].as_mut().unwrap().number = n_bc;
    assert_eq!(world.ware_route_flags(P0, a, c), Some(vec![a, c]));
    assert_eq!(world.ware_route_flags(P0, c, c), Some(vec![c]));
}

// ECO-REQ-3.2, TRN-CARRY-5.3: the costs one search back from a flag finds
// equal the cheapest route's cost from each flag, with a busy road, and
// after a road was split and another removed (the searches follow the
// roads as they are).
// spec: TRN-CARRY-5.1, TRN-CARRY-5.3, ECO-REQ-3.2 (T8, T9)
#[test]
fn costs_to_a_flag_equal_each_route_cost() {
    let (mut world, a, b, c) = triangle();
    let check = |world: &World, flags: &[Point]| {
        for &to in flags {
            let costs = world.route_costs_to(P0, to, Search::Ware);
            for &from in flags {
                assert_eq!(
                    costs.from(from),
                    world.route_cost(P0, from, to),
                    "{from:?} -> {to:?}"
                );
            }
        }
    };
    check(&world, &[a, b, c]);
    // The A-B carrier is carrying something: A-B costs 3 x 4 (T9).
    world.carriers[0].ware = Some(99);
    assert_eq!(world.route_costs_to(P0, c, Search::Ware).from(a), Some(10));
    check(&world, &[a, b, c]);
    // A new road C-E, then a flag on its middle splits it.
    let (d, e) = ((11, 2), (14, 2));
    assert_eq!(flag(&mut world, e), Ok(()));
    assert_eq!(road(&mut world, c, e), Ok(()));
    check(&world, &[a, b, c, e]);
    assert_eq!(flag(&mut world, d), Ok(()));
    assert_eq!(world.roads().count(), 5);
    check(&world, &[a, b, c, d, e]);
    // Without the B-C road C is reached only over D.
    let bc = world.roads.iter().position(|r| {
        r.as_ref()
            .is_some_and(|r| r.path.first() == Some(&b) && r.path.last() == Some(&c))
    });
    world.remove_road(bc.unwrap());
    check(&world, &[a, b, c, d, e]);
    assert_eq!(world.next_flag(P0, b, c), Some(a));
    // A flag without roads: only itself.
    let lone = (14, 9);
    assert_eq!(flag(&mut world, lone), Ok(()));
    check(&world, &[a, b, c, d, e, lone]);
}

// TRN-CARRY-5.5
#[test]
fn no_route_no_next_flag() {
    let mut world = transport_world(14, 8);
    assert_eq!(flag(&mut world, (2, 2)), Ok(()));
    assert_eq!(flag(&mut world, (9, 5)), Ok(()));
    assert_eq!(world.next_flag(P0, (2, 2), (9, 5)), None);
}

// TRN-CARRY-5.2: a climbing step adds 1.25 + 1, a level one 2.
#[test]
fn road_length_with_slopes() {
    let mut terrain = flat(10, 4);
    set_height(&mut terrain, (2, 1), 400);
    let world = world(terrain);
    assert_eq!(world.road_length(&[(1, 1), (2, 1)]), 2);
    // 2.25 + 2.25 = 4.5, cut to 4.
    assert_eq!(world.road_length(&[(1, 1), (2, 1), (3, 1)]), 4);
    assert_eq!(world.road_length(&[(3, 1), (4, 1), (5, 1)]), 4);
}

// TRN-CARRY T50 (Q-374), in T1's ticks: the plank lies at B from 81, but
// it counts on the road only when the put-down's two actions are over, in
// the carrier's update of 85. When the road's traffic period ends in 85
// (its clock runs in the roads' update, before the carriers'), the
// transport belongs to the next period.
// spec: TRN-CARRY-3.4, TRN-CARRY-6.1 (T50)
#[test]
fn t50_a_put_down_counts_when_its_actions_are_over() {
    for period_ends in [false, true] {
        let (mut world, a, _, b) = two_step_road();
        let start = step_to_look_tick(&mut world, 0);
        world.submit(Command::DropWare {
            player: P0,
            good: PLANK,
            at: a,
            destination: b,
        });
        let mut counts = BTreeMap::new();
        for _ in 0..90 {
            if period_ends && world.tick() - start == 85 {
                let road = world.roads[0].as_mut().unwrap();
                road.traffic.ticks = TRAFFIC_PERIOD;
            }
            world.step();
            let tick = world.tick() - 1 - start;
            let traffic = &world.roads().next().unwrap().traffic;
            counts.insert(tick, (traffic.transport_count, traffic.average_bits));
        }
        assert!(world.ware(0).is_none(), "the plank was put down at B");
        assert_eq!(counts[&81], (0, 0), "lies at B, not counted");
        assert_eq!(counts[&84], (0, 0), "the pickup-time action runs");
        // Counted in 85, after the period closed with no transport in it.
        assert_eq!(counts[&85], (1, 0), "period ends: {period_ends}");
    }
}

// TRN-CARRY T10, T11, TRN-CARRY-6.3
#[test]
fn t10_t11_donkey_requests() {
    let period = |traffic: &mut Traffic, wares: u32| {
        traffic.transport_count = wares;
        for _ in 0..=TRAFFIC_PERIOD {
            traffic.tick(4);
        }
    };
    let mut t10 = Traffic::default();
    period(&mut t10, 70);
    assert_eq!(t10.average(), 140.0);
    assert!(t10.donkey_requested);

    let mut t11 = Traffic::default();
    period(&mut t11, 60);
    assert_eq!((t11.average(), t11.donkey_requested), (120.0, false));
    period(&mut t11, 60);
    assert_eq!((t11.average(), t11.donkey_requested), (180.0, true));

    for (wares, requested) in [(66, true), (65, false)] {
        let mut traffic = Traffic::default();
        period(&mut traffic, wares);
        assert_eq!(traffic.donkey_requested, requested, "{wares}");
    }
    // The period ends after the 6001st tick, not before.
    let mut traffic = Traffic {
        transport_count: 70,
        ..Traffic::default()
    };
    for _ in 0..TRAFFIC_PERIOD {
        traffic.tick(4);
    }
    assert!(!traffic.donkey_requested);
    traffic.tick(4);
    assert!(traffic.donkey_requested);
}

// TRN-CARRY T52 (TRN-CARRY-6.1): the average is a float32, not rounded. Road
// 90676 of E-trace-014 (L 4) with 17, 20, 24, 41, 35 transports in its
// periods: averages 34, 57, 76.5, 120.25, 130.125; the fifth passes 130
// and asks for a donkey (rounded down, 130 would not).
// spec: TRN-CARRY-6.1 (TRN-CARRY T52), TRN-CARRY-6.2
#[test]
fn t52_the_traffic_average_is_a_float() {
    let mut traffic = Traffic::default();
    let mut averages = Vec::new();
    for wares in [17, 20, 24, 41, 35] {
        assert!(!traffic.donkey_requested);
        traffic.transport_count = wares;
        for _ in 0..=TRAFFIC_PERIOD {
            traffic.tick(4);
        }
        averages.push(traffic.average());
    }
    assert_eq!(averages, [34.0, 57.0, 76.5, 120.25, 130.125]);
    assert!(traffic.donkey_requested);
}

// spec: TRN-CARRY-5.4
#[test]
fn wares_travel_over_several_roads() {
    let mut world = transport_world(24, 8);
    let flags = [(2, 4), (5, 4), (8, 4), (11, 4)];
    for f in flags {
        assert_eq!(flag(&mut world, f), Ok(()));
    }
    for pair in flags.windows(2) {
        assert_eq!(road(&mut world, pair[0], pair[1]), Ok(()));
    }
    drop(&mut world, PLANK, flags[0], flags[3]);
    drop(&mut world, COIN, flags[3], flags[0]);
    for _ in 0..800 {
        world.step();
    }
    assert_eq!(world.wares().count(), 0, "both delivered");
    let counts: Vec<u32> = world.roads().map(|r| r.traffic.transport_count).collect();
    assert_eq!(counts, [2, 2, 2]);
}

// spec: TRN-ROAD-5.2
#[test]
fn drop_ware_is_checked() {
    let (mut world, a, _, b) = two_step_road();
    let mut try_drop = |good, at, destination| {
        apply(
            &mut world,
            Command::DropWare {
                player: P0,
                good,
                at,
                destination,
            },
        )
    };
    assert_eq!(try_drop(PLANK, (0, 0), b), Err(Rejection::NotAFlag));
    assert_eq!(try_drop(PLANK, a, (9, 1)), Err(Rejection::NotAFlag));
    assert_eq!(try_drop(0xBAD, a, b), Err(Rejection::UnknownGood));
    assert_eq!(try_drop(PLANK, a, (90, 1)), Err(Rejection::OutsideMap));
    for _ in 0..5 {
        assert_eq!(try_drop(STONE, b, a), Ok(()));
    }
    // TRN-CARRY-4.2: a sixth ware leaving the same way does not fit.
    assert_eq!(try_drop(STONE, b, a), Err(Rejection::FlagFull));
}

// TRN-ROAD-4.2 with a carrier on the road (Q-102).
#[test]
fn splitting_a_road_keeps_one_carrier_per_road() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b) = ((2, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    drop(&mut world, PLANK, a, b);
    for _ in 0..30 {
        world.step();
    }
    assert_eq!(flag(&mut world, (5, 4)), Ok(()));
    let roads: Vec<_> = world.roads().collect();
    assert_eq!(roads.len(), 2);
    // The carrier stays on its part; the other part requests one
    // (ECO-REQ-1.1) instead of getting one placed.
    let manned: Vec<_> = roads.iter().filter(|r| r.carrier.is_some()).collect();
    assert_eq!(manned.len(), 1);
    let carrier = manned[0].carrier.unwrap();
    assert!(manned[0].path.contains(&world.carriers[carrier].walker.at));
    assert_eq!(world.carriers().len(), 1);
    let empty = (0..world.roads.len())
        .find(|&id| {
            world.roads[id]
                .as_ref()
                .is_some_and(|r| r.carrier.is_none())
        })
        .unwrap();
    world.step();
    assert_eq!(
        world.requests(P0)[0].requester,
        crate::Requester::Road(empty)
    );
    world.spawn_carrier(empty);
    for _ in 0..600 {
        world.step();
    }
    assert_eq!(world.wares().count(), 0, "the plank still arrives");
}

// TRN-ROAD-4.2, ECO-REQ-1.1, 2.3: a split makes two new roads; the old
// road's open request (here a donkey) goes with it, so the half left
// without the carrier asks for a helper (a kept donkey request blocked
// it for good: OBS-17).
#[test]
fn split_drops_the_old_roads_request() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    world.roles.donkey = Some(0x98);
    let (a, b) = ((2, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    let road0 = world.roads[0].as_mut().unwrap();
    road0.traffic.donkey_requested = true;
    world.request(P0, crate::Requester::Road(0), 0x98);
    // The carrier idles mid-road, on the second half of the split.
    assert_eq!(flag(&mut world, (4, 4)), Ok(()));
    let first = world.roads[0].as_ref().unwrap();
    assert_eq!(first.carrier, None);
    assert!(!first.traffic.donkey_requested, "a new road");
    world.step();
    assert!(world
        .requests(P0)
        .iter()
        .any(|r| r.requester == crate::Requester::Road(0) && r.profession == 0x99));
    assert!(!world
        .requests(P0)
        .iter()
        .any(|r| r.requester == crate::Requester::Road(0) && r.profession == 0x98));
}

/// A game with the headquarters at `HQ` and a level road from its flag
/// four steps east to a flag, whose carrier came from the headquarters
/// and stands on its idle field. Returns the world, the far flag, a field
/// of the road and the carrier's index.
fn road_from_headquarters() -> (World, Point, Point, usize) {
    use crate::requests::tests::{full_stock, game_with, HQ_FLAG};
    let mut world = game_with(flat(40, 30), &full_stock());
    let far = (HQ_FLAG.0 + 4, HQ_FLAG.1);
    assert_eq!(flag(&mut world, far), Ok(()));
    let outcome = apply(
        &mut world,
        Command::BuildRoad {
            player: P0,
            from: HQ_FLAG,
            to: far,
        },
    );
    assert_eq!(outcome, Ok(()));
    let mid = (HQ_FLAG.0 + 2, HQ_FLAG.1);
    for _ in 0..400 {
        world.step();
        let ready = world
            .carriers()
            .iter()
            .any(|c| c.task == CarrierTask::Idle && !c.walker.is_walking());
        if ready {
            break;
        }
    }
    let carrier = world
        .carriers()
        .iter()
        .position(|c| c.task == CarrierTask::Idle && !c.walker.is_walking())
        .expect("the road's carrier is idle");
    (world, far, mid, carrier)
}

/// The trace rows of the settler numbered `number`.
fn rows_of(world: &World, number: u32) -> usize {
    let id = number.to_string();
    world.trace_settlers().iter().filter(|r| r.id == id).count()
}

/// The settler that the released carrier numbered `number` became.
fn released(world: &World, number: u32) -> &crate::Settler {
    world
        .settlers()
        .map(|(_, s)| s)
        .find(|s| s.number == number)
        .expect("the released carrier is a settler")
}

// TRN-CARRY T22, T34, the walking case: the computer player replaces the
// road by a new one over the same fields (the last part of tick R0) while
// its carrier walks a step with a plank, three more steps to its flag. In
// its next update the carrier walks on at its rate, loses the plank and
// keeps to its walk, every step of it without a stop, as every field ahead
// is still a road field (T34); on the walk's last field it stands in A and
// A + 1, orders its way home in A + 2 (progress 0) and walks from A + 3. It
// keeps one trace row throughout.
// spec: TRN-CARRY-7.1a, TRN-CARRY-7.1b, TRN-ROAD-6.3, TRN-CARRY-7.2 (T22, T34)
#[test]
fn t22_t34_released_carrier_finishes_its_walk() {
    use crate::requests::tests::{HQ_FLAG, PLANK};
    let (mut world, far, mid, carrier) = road_from_headquarters();
    let number = world.carriers()[carrier].number;
    drop(&mut world, PLANK, far, HQ_FLAG);
    // Until the carrier walks a step toward the headquarters with it.
    let mut found = false;
    for _ in 0..400 {
        world.step();
        let c = &world.carriers()[carrier];
        if c.ware.is_some() && c.walker.next.is_some() && c.walker.progress() > 0.3 {
            found = true;
            break;
        }
    }
    assert!(found, "the carrier carries the plank");
    let held = world.carriers()[carrier].ware.expect("held");
    world.step_then(|_, _, _, _| {
        vec![
            Command::RemoveRoad {
                player: P0,
                at: mid,
            },
            Command::BuildRoad {
                player: P0,
                from: HQ_FLAG,
                to: far,
            },
        ]
    });
    assert_eq!(world.late_results()[0].outcome, Ok(()));
    assert_eq!(world.late_results()[1].outcome, Ok(()));
    let before = world.carriers()[carrier].walker.clone();
    // Tick R0, after the removal: nothing noticed yet.
    let s = released(&world, number);
    assert_eq!((s.walker.at, s.walker.next), (before.at, before.next));
    assert_eq!(s.walker.progress_bits, before.progress_bits);
    let view = world
        .settler_views()
        .into_iter()
        .find(|v| v.object == number);
    assert_eq!(
        view.and_then(|v| v.carrying),
        Some(PLANK),
        "it still holds the plank"
    );
    assert!(world.ware(held).is_some());
    assert_eq!(rows_of(&world, number), 1);
    // R0 + 1: it walks on at its rate, the plank is gone.
    world.step();
    let s = released(&world, number);
    let progress = before.progress() + f32::from_bits(before.rate_bits);
    assert_eq!((s.walker.at, s.walker.next), (before.at, before.next));
    assert_eq!(s.walker.progress_bits, progress.to_bits());
    let view = world
        .settler_views()
        .into_iter()
        .find(|v| v.object == number);
    assert_eq!(view.and_then(|v| v.carrying), None);
    assert!(world.ware(held).is_none());
    assert_eq!(rows_of(&world, number), 1);
    // A: the walk ends on its last field; no stop at the fields between.
    assert!(!before.path.is_empty(), "more than one step left");
    let end = before.path.back().copied().expect("the walk's last field");
    for _ in 0..200 {
        world.step();
        assert_eq!(rows_of(&world, number), 1);
        if released(&world, number).walker.next.is_none() {
            break;
        }
    }
    let s = released(&world, number);
    assert_eq!(s.walker.at, end);
    assert!(!s.walker.has_walk(), "it stands in A");
    world.step();
    assert!(!released(&world, number).walker.has_walk(), "and in A + 1");
    world.step();
    let s = released(&world, number);
    // A + 2: its way home is ordered at progress 0. From the flag the
    // walk ends on, a road walk home: here the door step into the
    // headquarters (TRN-CARRY-7.1b; the free walk from a field inside the
    // removed road: released_standing_carrier_walks_in_the_third_tick).
    assert!(
        s.walker.is_walking() && s.walker.next.is_none(),
        "A + 2: the walk is ordered"
    );
    assert_eq!((s.walker.at, s.walker.progress_bits), (end, 0));
    assert!(world.flag_at(end).is_some());
    assert!(
        matches!(s.job, crate::SettlerJob::Home { .. }),
        "{:?}",
        s.job
    );
    assert_eq!(rows_of(&world, number), 1);
}

// TRN-CARRY T49 (TRN-CARRY-7.1a): a released carrier's walk ends on the
// first field it reaches that is a flag or no longer a road field, the
// roads as they are when it notices. Its road removed while it carries
// towards the headquarters, three more steps to go: with nothing built
// instead it ends on the field of the step in progress; with a flag set on
// the old road's middle and a road from the far end to it, it walks on over
// the new road's field to that flag. Either way it stands there in A and
// A + 1.
// spec: TRN-CARRY-7.1a (TRN-CARRY T49)
#[test]
fn t49_a_released_carriers_walk_ends_where_the_road_is_gone() {
    use crate::requests::tests::{HQ_FLAG, PLANK};
    let cases: [(bool, Point); 2] = [(false, (13, 9)), (true, (12, 9))];
    for (rebuilt, end) in cases {
        let (mut world, far, mid, carrier) = road_from_headquarters();
        let number = world.carriers()[carrier].number;
        drop(&mut world, PLANK, far, HQ_FLAG);
        for _ in 0..400 {
            world.step();
            let c = &world.carriers()[carrier];
            if c.ware.is_some() && c.walker.next == Some((13, 9)) {
                break;
            }
        }
        let before = world.carriers()[carrier].walker.clone();
        assert_eq!(before.next, Some((13, 9)), "carrying west");
        assert_eq!(before.path.len(), 3, "three more steps");
        world.step_then(move |_, _, _, _| {
            let mut commands = vec![Command::RemoveRoad {
                player: P0,
                at: mid,
            }];
            if rebuilt {
                commands.push(Command::PlaceFlag {
                    player: P0,
                    at: mid,
                });
                commands.push(Command::BuildRoad {
                    player: P0,
                    from: far,
                    to: mid,
                });
            }
            commands
        });
        assert!(world.late_results().iter().all(|r| r.outcome.is_ok()));
        assert_eq!(world.road_at((13, 9)).is_some(), rebuilt);
        let mut a = None;
        for _ in 0..200 {
            world.step();
            let s = released(&world, number);
            if !s.walker.has_walk() {
                a = Some(s.walker.at);
                break;
            }
        }
        assert_eq!(a, Some(end), "rebuilt {rebuilt}");
        world.step();
        let s = released(&world, number);
        assert_eq!((s.walker.at, s.walker.has_walk()), (end, false), "A + 1");
    }
}

// TRN-CARRY T61 (TRN-CARRY-7.1a, corrected 2026-11-21, E-trn-042): a
// released carrier whose step in progress ends in the update in which it
// notices the release (R) tests the field that step reached first: no
// longer a road field (or a flag), its walk ends there although it had more
// steps; it stands in R, R + 1 and R + 2 and orders its walk home at
// progress 0 in R + 3, as a carrier standing when it notices.
// spec: TRN-CARRY-7.1a (TRN-CARRY T61)
#[test]
fn t61_a_step_ending_in_the_notice_update_stops_there() {
    use crate::requests::tests::{HQ_FLAG, PLANK};
    let (mut world, far, mid, carrier) = road_from_headquarters();
    let number = world.carriers()[carrier].number;
    drop(&mut world, PLANK, far, HQ_FLAG);
    // Removed in tick T (the step to (13, 9) goes on in T), noticed in R =
    // T + 1, whose movement ends that step.
    for _ in 0..400 {
        let w = &world.carriers()[carrier].walker;
        let rate = f32::from_bits(w.rate_bits);
        if world.carriers()[carrier].ware.is_some()
            && w.next == Some((13, 9))
            && w.progress() + rate <= 1.0
            && w.progress() + 2.0 * rate > 1.0
        {
            break;
        }
        world.step();
    }
    assert_eq!(world.carriers()[carrier].walker.path.len(), 3, "more steps");
    world.step_then(move |_, _, _, _| {
        vec![Command::RemoveRoad {
            player: P0,
            at: mid,
        }]
    });
    assert!(world.late_results().iter().all(|r| r.outcome.is_ok()));
    assert!(world.road_at((13, 9)).is_none());
    for tick in ["R", "R + 1", "R + 2"] {
        world.step();
        let s = released(&world, number);
        assert_eq!(
            s.walker.at,
            (13, 9),
            "{tick}: on the field the step reached"
        );
        assert!(!s.walker.has_walk(), "it stands in {tick}");
    }
    world.step();
    let s = released(&world, number);
    assert!(
        s.walker.has_walk() && s.walker.next.is_none(),
        "R + 3: the walk home is ordered"
    );
    assert_eq!(s.walker.progress_bits, 0);
}

// TRN-CARRY-7.1a, the standing case: a carrier standing on its idle field
// when it notices the removal (tick R) stands through R + 2 and orders its
// walk home at progress 0 in R + 3.
// spec: TRN-CARRY-7.1a
#[test]
fn released_standing_carrier_walks_in_the_third_tick() {
    let (mut world, _, mid, carrier) = road_from_headquarters();
    let number = world.carriers()[carrier].number;
    let at = world.carriers()[carrier].walker.at;
    world.step_then(|_, _, _, _| {
        vec![Command::RemoveRoad {
            player: P0,
            at: mid,
        }]
    });
    assert_eq!(world.late_results()[0].outcome, Ok(()));
    for tick in ["R", "R + 1", "R + 2"] {
        world.step();
        let s = released(&world, number);
        assert!(!s.walker.has_walk(), "it stands in {tick}");
        assert_eq!(s.walker.at, at);
    }
    world.step();
    let s = released(&world, number);
    assert!(
        s.walker.has_walk() && s.walker.next.is_none(),
        "R + 3: the walk is ordered"
    );
    assert_eq!(s.walker.progress_bits, 0);
    world.step();
    assert!(
        released(&world, number).walker.next.is_some(),
        "walking from R + 4"
    );
}

// TRN-CARRY T45, T48 (TRN-CARRY-7.1a): a carrier still on his way to his
// road over another road when it is removed: his walk ends on the next flag
// he reaches. Reaching it after the notice tick, he is the walking case: he
// stands in A and A + 1, his way home ordered in A + 2 (the extra tick is
// only for a step that ends in the notice tick).
// spec: TRN-CARRY-7.1a (TRN-CARRY T45, T48)
#[test]
fn t45_a_carrier_on_his_way_to_a_removed_road_stops_at_the_next_flag() {
    let (mut world, far, _, _) = road_from_headquarters();
    let beyond = (far.0, far.1 + 4);
    assert_eq!(flag(&mut world, beyond), Ok(()));
    let outcome = apply(
        &mut world,
        Command::BuildRoad {
            player: P0,
            from: far,
            to: beyond,
        },
    );
    assert_eq!(outcome, Ok(()));
    let road = world.roads.len() - 1;
    let path = world.roads[road].as_ref().unwrap().path.clone();
    let first = world.roads[0].as_ref().unwrap().path.clone();
    let inner = &first[1..first.len() - 1];
    // His new carrier comes out of the headquarters over the first road.
    let mut number = None;
    for _ in 0..400 {
        world.step();
        number = world
            .carriers()
            .iter()
            .find(|c| {
                c.road == road
                    && matches!(c.task, CarrierTask::Arrive { .. })
                    && c.walker.next.is_some()
                    && inner.contains(&c.walker.at)
            })
            .map(|c| c.number);
        if number.is_some() {
            break;
        }
    }
    let number = number.expect("on his way over the first road");
    world.step_then(|_, _, _, _| {
        vec![Command::RemoveRoad {
            player: P0,
            at: path[2],
        }]
    });
    assert_eq!(world.late_results()[0].outcome, Ok(()));
    world.step();
    assert_eq!(released(&world, number).walker.destination(), far);
    while released(&world, number).walker.is_walking() {
        world.step();
        assert!(world.tick() < 1000, "on the flag");
    }
    assert_eq!(released(&world, number).walker.at, far);
    world.step();
    assert!(
        !released(&world, number).walker.has_walk(),
        "stands in A + 1"
    );
    world.step();
    assert!(released(&world, number).walker.has_walk(), "A + 2: home");
}

// TRN-CARRY-5.1a: a road without an assigned carrier or donkey carries no
// ware: a ware finds no next flag and no route cost over it, while a
// settler walks it.
// spec: TRN-CARRY-5.1a
#[test]
fn wares_use_only_roads_with_a_carrier() {
    let mut world = transport_world(14, 8);
    let (a, b) = ((3, 4), (6, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    let outcome = apply(
        &mut world,
        Command::BuildRoad {
            player: P0,
            from: a,
            to: b,
        },
    );
    assert_eq!(outcome, Ok(()));
    assert_eq!(world.next_flag(P0, a, b), None);
    assert_eq!(world.route_cost(P0, a, b), None);
    assert!(world.road_walk(P0, a, b).is_some());
    world.spawn_carrier(world.roads.len() - 1);
    assert_eq!(world.next_flag(P0, a, b), Some(b));
    assert!(world.route_cost(P0, a, b).is_some());
}

// TRN-CARRY-7.1b (Q-266): a carrier released on its idle field, inside the
// removed road, finds nothing at the field or south-east of it, so in
// R + 3 it free-walks (TRN-FREE-2) to the network flag of the nearest
// warehouse nearest to it, the headquarters' flag; at the end of that walk
// the warehouse north-west of the flag takes it in (T55: its way in ordered
// in the arrival tick).
// spec: TRN-CARRY-7.1b (TRN-CARRY T55), TRN-FREE-6.4
#[test]
fn a_carrier_released_inside_its_road_free_walks_to_a_network_flag() {
    use crate::requests::tests::{HELPER, HQ, HQ_FLAG};
    let (mut world, _, mid, carrier) = road_from_headquarters();
    let number = world.carriers()[carrier].number;
    let at = world.carriers()[carrier].walker.at;
    assert!(
        world.road_at(at).is_some(),
        "the idle field is inside the road"
    );
    world.step_then(|_, _, _, _| {
        vec![Command::RemoveRoad {
            player: P0,
            at: mid,
        }]
    });
    for _ in 0..4 {
        world.step();
    }
    let s = released(&world, number);
    assert!(
        matches!(
            s.job,
            crate::SettlerJob::Released {
                step: crate::ReleasedStep::ToFlag,
                ..
            }
        ),
        "{:?}",
        s.job
    );
    assert_eq!(s.walker.path.back(), Some(&HQ_FLAG));
    assert!(
        s.walker.next.is_none() && s.walker.progress_bits == 0,
        "ordered in R + 3"
    );
    let helpers = world.warehouse(HQ).unwrap().count(HELPER);
    // TRN-CARRY T55: in the tick the walk ends on the flag, the way in is
    // ordered at progress 0 towards the warehouse, with no standing tick: it
    // advances from the next tick.
    let mut arrived = None;
    for _ in 0..200 {
        world.step();
        if world.settlers().all(|(_, s)| s.number != number) {
            break;
        }
        let s = released(&world, number);
        if arrived.is_none() && matches!(s.job, crate::SettlerJob::Home { .. }) {
            assert_eq!(s.walker.at, HQ_FLAG);
            assert_eq!(s.walker.path.front(), Some(&HQ), "the way in ordered");
            assert_eq!(s.walker.progress_bits, 0);
            arrived = Some(world.tick());
        } else if arrived == Some(world.tick() - 1) {
            assert!(
                s.walker.next == Some(HQ) && s.walker.progress_bits > 0,
                "no standing tick on the flag"
            );
        }
    }
    assert!(arrived.is_some(), "it went home from the flag");
    assert!(
        world.settlers().all(|(_, s)| s.number != number),
        "it went in"
    );
    assert_eq!(world.warehouse(HQ).unwrap().count(HELPER), helpers + 1);
}

// TRN-FREE-6.4 step 3: the warehouse must stand on the settler's landmass,
// equal stored continent indices (FMT-S2M-4.9). The released carrier of
// the test above, with the headquarters' field on another index, finds no
// warehouse and starts no walk to its flag in R + 3; with both on -1 (equal
// indices) it does.
// spec: TRN-FREE-6.4, FMT-S2M-4.9
#[test]
fn the_warehouse_of_step_3_stands_on_the_settlers_stored_continent() {
    use crate::requests::tests::{HQ, HQ_FLAG};
    for (hq_index, settler_index, walks) in [(2, 1, false), (-1, -1, true), (1, 1, true)] {
        let (mut world, _, mid, carrier) = road_from_headquarters();
        let number = world.carriers()[carrier].number;
        let at = world.carriers()[carrier].walker.at;
        let mut layer = vec![1; 40 * 30];
        layer[(HQ.1 * 40 + HQ.0) as usize] = hq_index;
        layer[(at.1 * 40 + at.0) as usize] = settler_index;
        world.set_continents(layer);
        world.step_then(|_, _, _, _| {
            vec![Command::RemoveRoad {
                player: P0,
                at: mid,
            }]
        });
        for _ in 0..4 {
            world.step();
        }
        let s = released(&world, number);
        assert_eq!(
            s.walker.path.back() == Some(&HQ_FLAG),
            walks,
            "{hq_index} {settler_index}: {:?}",
            s.job
        );
    }
}

// ECO-WORK T24 (ECO-WORK-3.8d, TRN-CARRY-1.4, Q-270): a new carrier for a
// road that does not touch its warehouse's flag stands one tick on that
// flag, orders its walk on at progress 0 in the next tick and walks over the
// roads to its road and on to its idle field in one walk, without stopping
// on its road's flag.
// spec: ECO-WORK-3.8d, TRN-CARRY-1.4 (T24)
#[test]
fn t24_a_new_carrier_stands_on_its_warehouses_flag() {
    use crate::requests::tests::{full_stock, game_with, HQ_FLAG};
    let mut world = game_with(flat(40, 30), &full_stock());
    let (middle, far) = ((HQ_FLAG.0 + 2, HQ_FLAG.1), (HQ_FLAG.0 + 4, HQ_FLAG.1));
    assert_eq!(flag(&mut world, middle), Ok(()));
    assert_eq!(flag(&mut world, far), Ok(()));
    // The first road gets its carrier at once; the second's comes from the
    // headquarters.
    assert_eq!(road(&mut world, HQ_FLAG, middle), Ok(()));
    let outcome = apply(
        &mut world,
        Command::BuildRoad {
            player: P0,
            from: middle,
            to: far,
        },
    );
    assert_eq!(outcome, Ok(()));
    let road = world.roads().count() - 1;
    let carrier = loop {
        world.step();
        if let Some(c) = world.carriers().iter().position(|c| c.road == road) {
            break c;
        }
        assert!(world.tick() < 300, "no carrier sent");
    };
    let c = |world: &World| world.carriers()[carrier].walker.clone();
    // Until it stands on the headquarters' flag.
    while !(c(&world).at == HQ_FLAG && c(&world).next.is_none()) {
        world.step();
        assert!(world.tick() < 500);
    }
    assert!(!c(&world).has_walk(), "it stands in the arrival tick");
    world.step();
    let w = c(&world);
    assert!(
        w.has_walk() && w.next.is_none() && w.progress_bits == 0,
        "ordered in the next"
    );
    world.step();
    assert!(c(&world).next.is_some(), "walking from the tick after");
    // One walk to the idle field: it never stands on its road's flag.
    let idle = idle_field(&world.road(road).unwrap().path);
    for _ in 0..400 {
        world.step();
        let w = c(&world);
        if w.next.is_none() && !w.is_walking() {
            assert_eq!(w.at, idle, "it stops only on its idle field");
            return;
        }
    }
    panic!("it did not reach its idle field");
}

// TRN-CARRY-2.4a: a ware joins the flag's list when its place is granted,
// not when it is put down. Ware A, carried from A to B, is admitted to B on
// the carrier's walk; ware B is dropped at B after that and lies there
// first; when A is put down the list is A, B and A is the best ware for the
// next road (same priority, same next flag).
// spec: TRN-CARRY-2.4a, TRN-CARRY-2.4
#[test]
fn a_ware_joins_the_flags_list_when_its_place_is_granted() {
    let (mut world, a, _, b) = two_step_road();
    let c = (b.0 + 2, b.1);
    assert_eq!(flag(&mut world, c), Ok(()));
    away_road(&mut world, b, c);
    let onward = world.roads.len() - 1;
    drop(&mut world, PLANK, a, c);
    let first = world.wares_at(a)[0];
    while !matches!(
        world.carriers[0].task,
        CarrierTask::Carry {
            entry: Entry::Reserved,
            ..
        }
    ) {
        world.step();
        assert!(world.tick() < 200, "never admitted");
    }
    drop(&mut world, PLANK, b, c);
    let second = *world.wares_at(b).last().unwrap();
    assert_eq!(world.wares_at(b), [second]);
    while world.wares_at(b).len() < 2 {
        world.step();
        assert!(world.tick() < 300, "never put down");
    }
    assert_eq!(world.wares_at(b), [first, second]);
    assert_eq!(world.best_ware(onward, b).map(|(w, _)| w), Some(first));
}

// TRN-ROAD-4.2 (corrected, Q-359): the old road's carrier is attached by
// the field it stands on, or the field its step goes to: on the first half
// (the old road's first flag to the new flag, that flag included) it gets
// the first half, anywhere else the second. Road (2, 4) .. (8, 4).
// spec: TRN-ROAD-4.2 (TRN T23)
#[test]
fn a_split_gives_the_carrier_the_half_of_its_field() {
    let cases: [(Point, Option<Point>, Point, bool); 4] = [
        // standing on the second half's side of the new flag
        ((5, 4), None, (4, 4), false),
        // standing on the first half's side
        ((5, 4), None, (6, 4), true),
        // stepping from the second half onto the new flag's field
        ((6, 4), Some((5, 4)), (5, 4), true),
        // stepping off the road (as on its way to it)
        ((5, 4), Some((5, 5)), (6, 4), false),
    ];
    for (at, next, split, first) in cases {
        let mut world = transport_world(16, 8);
        world.roles.helper = Some(0x99);
        let (a, b) = ((2, 4), (8, 4));
        assert_eq!(flag(&mut world, a), Ok(()));
        assert_eq!(flag(&mut world, b), Ok(()));
        assert_eq!(road(&mut world, a, b), Ok(()));
        for _ in 0..30 {
            world.step();
        }
        let w = &mut world.carriers[0].walker;
        w.walk([]);
        w.at = at;
        w.next = next;
        if next.is_some_and(|n| n.1 != a.1) {
            // Off the road: on its way to it.
            world.carriers[0].task = CarrierTask::Arrive {
                flag: a,
                stand: false,
            };
        }
        assert_eq!(flag(&mut world, split), Ok(()), "{split:?}");
        let road = world.carriers[0].road;
        let path = &world.road(road).unwrap().path;
        assert_eq!(
            path[0] == a,
            first,
            "{at:?} {next:?} split {split:?}: {path:?}"
        );
        assert_eq!(*path.last().unwrap(), split);
    }
}

// TRN-ROAD-4.2 (82d505f, TRN-ROAD T25): a carrier carrying a ware to an end
// of the half it gets goes on unchanged: the same step, path and entry
// check. Road (2, 4) .. (8, 4), a plank from A to B; split at (4, 4) while
// it steps (5, 4) -> (6, 4): the second half, whose end B is its flag.
// spec: TRN-ROAD-4.2 (TRN-ROAD T25), TRN-CARRY-3.2
#[test]
fn a_carrier_carrying_to_an_end_of_its_half_walks_on() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b) = ((2, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    while world.carriers[0].walker.next != Some((6, 4)) {
        world.step();
        assert!(world.tick() < 300, "never carried");
    }
    world.step();
    let (task, walker) = (world.carriers[0].task, world.carriers[0].walker.clone());
    assert!(matches!(task, CarrierTask::Carry { to, .. } if to == b));
    assert!(walker.progress_bits > 0);
    assert_eq!(flag(&mut world, (4, 4)), Ok(()));
    let path = &world.road(world.carriers[0].road).unwrap().path;
    assert_eq!((path[0], *path.last().unwrap()), (b, (4, 4)));
    // The flag is set at the tick boundary: one tick further on the step.
    let now = &world.carriers[0].walker;
    assert_eq!(world.carriers[0].task, task);
    assert_eq!(
        (now.at, now.next, &now.path),
        (walker.at, walker.next, &walker.path)
    );
    assert!(now.progress_bits > walker.progress_bits);
}

// TRN-ROAD T28 (TRN-ROAD-4.2): a carrier carrying a ware to a flag of the
// other half keeps his step: his walk is cut at the new flag ahead, he walks
// on to it and puts the ware down there, still bound for its flag, for the
// other half's carrier. Road (2, 4) .. (10, 4), a plank from A to B; split
// at (8, 4) while he steps (5, 4) -> (6, 4): the first half, B on the other.
// spec: TRN-ROAD-4.2 (TRN-ROAD T28)
#[test]
fn a_carrier_carrying_to_the_other_half_walks_on_to_the_new_flag() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b, split) = ((2, 4), (10, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    while world.carriers[0].walker.next != Some((6, 4)) {
        world.step();
        assert!(world.tick() < 300, "never carried");
    }
    world.step();
    let walker = world.carriers[0].walker.clone();
    assert!(walker.progress_bits > 0);
    assert_eq!(flag(&mut world, split), Ok(()));
    let path = &world.road(world.carriers[0].road).unwrap().path;
    assert_eq!((path[0], *path.last().unwrap()), (a, split));
    // The same step, one tick further; the walk ends on the new flag.
    let now = &world.carriers[0].walker;
    assert!(matches!(world.carriers[0].task, CarrierTask::Carry { to, .. } if to == split));
    assert_eq!((now.at, now.next), (walker.at, walker.next));
    assert!(now.progress_bits > walker.progress_bits);
    assert_eq!(now.destination(), split);
    let ware = world.carriers[0].ware.expect("his ware");
    while world.wares[ware].as_ref().unwrap().location != WareLocation::Flag(split) {
        world.step();
        assert!(world.tick() < 600, "never put down");
    }
    // It waits there, still bound for B (this test world sends no carrier
    // to the other half).
    assert_eq!(world.wares[ware].as_ref().unwrap().destination, Some(b));
}

// TRN-CARRY T42 (TRN-CARRY-3.2, 3.7): a carrier on its step onto the flag it
// carries to whose entry is refused there (its exchange undone on the way)
// cancels the step in the next tick: it stands on the last field before the
// flag, back on its point, and makes no check in that tick.
// spec: TRN-CARRY-3.2 (TRN-CARRY T42), TRN-CARRY-3.7
#[test]
fn t42_a_carrier_refused_on_its_step_onto_the_flag_steps_back() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b) = ((2, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    while world.carriers[0].walker.next != Some(b) {
        world.step();
        assert!(world.tick() < 600, "never stepped onto B");
    }
    let before = world.carriers[0].walker.at;
    // The answer for B becomes no while he steps onto it.
    world.carriers[0].task = CarrierTask::Carry {
        to: b,
        entry: Entry::Open { stop: true },
    };
    world.step();
    let c = &world.carriers[0];
    assert_eq!(
        (c.walker.at, c.walker.next),
        (before, None),
        "back on the last field"
    );
    assert_eq!(c.walker.progress_bits, 0);
    assert_eq!(
        c.walker.destination(),
        b,
        "the step onto B is kept for later"
    );
    assert_eq!(
        c.task,
        CarrierTask::Carry {
            to: b,
            entry: Entry::Open { stop: true }
        },
        "no check in that tick"
    );
}

// TRN-CARRY T51 (TRN-CARRY-3.2): refused on his step onto the flag in the
// tick before the movement completes it, the carrier does not step back: the
// step ends on the flag and he waits there in the waiting state, with no
// check in that tick and checks only on the 10-tick grid; when one admits
// him, the next tick is his arrival tick (3.3a: he stands), then the
// put-down.
// spec: TRN-CARRY-3.2 (TRN-CARRY T51), TRN-CARRY-3.3a
#[test]
fn t51_a_refused_step_the_movement_completes_ends_on_the_flag() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b) = ((2, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    // The last tick of his step onto B: the next movement completes it.
    while !(world.carriers[0].walker.next == Some(b) && world.carriers[0].walker.completes_step()) {
        world.step();
        assert!(world.tick() < 600, "never stepped onto B");
    }
    let refused = CarrierTask::Carry {
        to: b,
        entry: Entry::Open { stop: true },
    };
    world.carriers[0].task = refused;
    world.step();
    let c = &world.carriers[0];
    assert_eq!((c.walker.at, c.walker.next), (b, None), "on the flag");
    assert_eq!(c.task, refused, "no check in that tick");
    // Checks only on his 10-tick grid: the first admits him (B has room).
    let number = u64::from(c.number);
    let mut admitted = None;
    while world.carriers[0].task == refused {
        world.step();
        assert!(world.tick() < 1000, "never admitted");
        if world.carriers[0].task != refused {
            admitted = Some(world.tick() - 1);
        }
    }
    let admitted = admitted.unwrap();
    assert!(
        (number + admitted).is_multiple_of(ROOM_RETRY_PERIOD),
        "{admitted}"
    );
    assert_eq!(world.carriers[0].walker.at, b);
    // The next tick he stands (his arrival tick), then puts the plank down.
    world.step();
    assert!(matches!(world.carriers[0].task, CarrierTask::Stop { at, .. } if at == b));
    world.step();
    assert!(matches!(
        world.carriers[0].task,
        CarrierTask::PutDown { second: false, .. }
    ));
}

// TRN-ROAD-4.2: a carrier standing in the building at the old road's first
// flag (the headquarters) gets the first half.
// spec: TRN-ROAD-4.2
#[test]
fn a_carrier_in_the_first_flags_building_gets_the_first_half() {
    use crate::requests::tests::{HQ, HQ_FLAG};
    let (mut world, far, _, carrier) = road_from_headquarters();
    let road = world.carriers[carrier].road;
    assert_eq!(world.road(road).unwrap().path[0], HQ_FLAG);
    let w = &mut world.carriers[carrier].walker;
    w.walk([]);
    w.at = HQ;
    w.next = None;
    world.carriers[carrier].task = CarrierTask::Arrive {
        flag: HQ_FLAG,
        stand: true,
    };
    let split = (HQ_FLAG.0 + 2, HQ_FLAG.1);
    assert_eq!(flag(&mut world, split), Ok(()));
    let path = &world.road(world.carriers[carrier].road).unwrap().path;
    assert_eq!((path[0], *path.last().unwrap()), (HQ_FLAG, split));
    assert_ne!(path[0], far);
}

// TRN-FREE-6.5 (g) (Q-359, TRN-FREE T25): a new carrier walking over
// another road towards its road when its road is split gets the second
// half (its field is on neither half); its walk ends on the next flag it
// reaches (the flag between the two roads), it stands there in the arrival
// tick, its walk on to the new half's idle field is ordered in the next
// tick at progress 0 and moves from the tick after.
// spec: TRN-FREE-6.5, TRN-ROAD-4.2, TRN-CARRY-1.4
#[test]
fn a_carrier_whose_road_is_split_on_its_way_stops_on_the_next_flag() {
    use crate::requests::tests::{full_stock, game_with, HQ_FLAG};
    let mut world = game_with(flat(40, 30), &full_stock());
    let y = HQ_FLAG.1;
    let (middle, far, split) = ((HQ_FLAG.0 + 2, y), (HQ_FLAG.0 + 6, y), (HQ_FLAG.0 + 4, y));
    assert_eq!(flag(&mut world, middle), Ok(()));
    assert_eq!(flag(&mut world, far), Ok(()));
    assert_eq!(road(&mut world, HQ_FLAG, middle), Ok(()));
    let outcome = apply(
        &mut world,
        Command::BuildRoad {
            player: P0,
            from: middle,
            to: far,
        },
    );
    assert_eq!(outcome, Ok(()));
    let road = world.roads().count() - 1;
    let carrier = loop {
        world.step();
        if let Some(c) = world.carriers().iter().position(|c| c.road == road) {
            break c;
        }
        assert!(world.tick() < 300, "no carrier sent");
    };
    let c = |world: &World| world.carriers()[carrier].walker.clone();
    // In its step from the headquarters' flag on the first road (the field
    // it goes to is on neither half; the step onto middle, the old road's
    // first flag, would give it the first half).
    while c(&world).next != Some((HQ_FLAG.0 + 1, y)) {
        world.step();
        assert!(world.tick() < 500);
    }
    assert_eq!(flag(&mut world, split), Ok(()));
    let own = world.carriers()[carrier].road;
    let path = world.road(own).unwrap().path.clone();
    assert_eq!(
        (path[0], *path.last().unwrap()),
        (far, split),
        "the second half"
    );
    while c(&world).is_walking() {
        world.step();
        assert!(world.tick() < 600);
    }
    assert_eq!(c(&world).at, middle, "the walk ends on the next flag");
    assert!(!c(&world).has_walk(), "it stands in the arrival tick");
    world.step();
    let w = c(&world);
    assert!(
        w.has_walk() && w.next.is_none() && w.progress_bits == 0,
        "ordered in the next"
    );
    world.step();
    assert!(c(&world).next.is_some(), "walking from the tick after");
    let idle = idle_field(&path);
    for _ in 0..400 {
        world.step();
        let w = c(&world);
        if w.next.is_none() && !w.is_walking() {
            assert_eq!(w.at, idle);
            return;
        }
    }
    panic!("it did not reach its idle field");
}

// TRN-FREE-6.3, TRN-ROAD-4.2: a carrier that stands off its road with
// nothing to carry (it can happen when its road was split while it stood in
// the building at the old road's first flag on the door field: it gets the
// second half) does not look for work there; its idle step orders a road
// walk from where it stands to its road's idle field.
// spec: TRN-FREE-6.3, TRN-ROAD-4.2, TRN-CARRY-1.4
#[test]
fn a_carrier_off_its_road_walks_to_its_idle_field() {
    use crate::requests::tests::{full_stock, game_with, HQ_FLAG};
    let mut world = game_with(flat(40, 30), &full_stock());
    let (middle, far) = ((HQ_FLAG.0 + 2, HQ_FLAG.1), (HQ_FLAG.0 + 6, HQ_FLAG.1));
    assert_eq!(flag(&mut world, middle), Ok(()));
    assert_eq!(flag(&mut world, far), Ok(()));
    assert_eq!(road(&mut world, HQ_FLAG, middle), Ok(()));
    assert_eq!(road(&mut world, middle, far), Ok(()));
    let road = world.roads().count() - 1;
    let carrier = loop {
        world.step();
        let c = world
            .carriers()
            .iter()
            .position(|c| c.road == road && c.task == CarrierTask::Idle && !c.walker.is_walking());
        if let Some(c) = c {
            break c;
        }
        assert!(world.tick() < 600, "no carrier on its idle field");
    };
    let idle = idle_field(&world.road(road).unwrap().path);
    // Off its road, on the headquarters' flag, as after a delivery.
    world.carriers[carrier].walker = Walker::new(HQ_FLAG);
    world.carriers[carrier].task = CarrierTask::Out { flag: HQ_FLAG };
    world.step();
    assert!(matches!(
        world.carriers()[carrier].task,
        CarrierTask::Arrive { .. }
    ));
    assert!(world.carriers()[carrier].walker.has_walk());
    for _ in 0..300 {
        world.step();
        let w = &world.carriers()[carrier].walker;
        if w.at == idle && !w.is_walking() {
            return;
        }
    }
    panic!("it did not reach its idle field");
}

/// TRN-FREE-6.1: the flags A (2, 4), B (10, 4) and F (6, 8) with the roads
/// A - B (the walker's), F - A and F - B, the walker's made first or last;
/// their lengths L set to `ab`, `fa` and `fb`. The world and the walker's
/// road.
fn own_road_network(own_first: bool, [ab, fa, fb]: [u32; 3]) -> (World, usize) {
    let (a, b, f) = ((2, 4), (10, 4), (6, 8));
    let mut world = transport_world(16, 12);
    for at in [a, b, f] {
        assert_eq!(flag(&mut world, at), Ok(()), "{at:?}");
    }
    let mut made = vec![(a, b), (f, a), (f, b)];
    if !own_first {
        made.rotate_left(1);
    }
    for &(from, to) in &made {
        assert_eq!(road(&mut world, from, to), Ok(()));
    }
    let ids: Vec<usize> = [(a, b), (f, a), (f, b)]
        .into_iter()
        .map(|ends| {
            world
                .roads
                .iter()
                .position(|r| {
                    r.as_ref()
                        .is_some_and(|r| (r.path[0], *r.path.last().unwrap()) == ends)
                })
                .unwrap()
        })
        .collect();
    for (&id, length) in ids.iter().zip([ab, fa, fb]) {
        world.roads[id].as_mut().unwrap().length = length;
    }
    (world, ids[0])
}

/// The road's flag a walk to one of its fields heads for: the first of
/// its two flags on the walk.
fn end_taken(walk: &[Point], path: &[Point]) -> Option<Point> {
    let ends = [path[0], *path.last().unwrap()];
    walk.iter().copied().find(|f| ends.contains(f))
}

// TRN-FREE T39 (TRN-FREE-6.1, corrected by Q-378): from F both flags of the
// walker's road cost 8 (L 8 to A and to B), and the field next to B is
// nearer over B; the goal is A, by itself 8, over B and the walker's road
// (cost 1) 9: the walk heads for A and walks along the road from there.
// spec: TRN-FREE-6.1 (TRN-FREE T39, T42), TRN-FREE-6.5
#[test]
fn t39_a_walk_to_a_road_field_heads_for_the_first_flag() {
    let (f, a) = ((6, 8), (2, 4));
    let (world, own) = own_road_network(false, [8, 8, 8]);
    let path = world.road(own).unwrap().path.clone();
    let to = path[path.len() - 2];
    let walk = world.road_walk_to_field(P0, own, f, to).unwrap();
    assert_eq!(end_taken(&walk, &path), Some(a), "{walk:?}");
    let mut expected = world.road_walk(P0, f, a).unwrap();
    expected.extend(along(&path, a, to));
    assert_eq!(walk, expected);
}

// TRN-FREE T41 (TRN-FREE-6.1, corrected by Q-378): A costs 29 by itself and
// 28 + 1 over B and the walker's road: on equal costs the road into A
// attached there first wins. The walker's road made last: A (as measured);
// made first: B.
// spec: TRN-FREE-6.1 (TRN-FREE T41, T42, T31)
#[test]
fn t41_on_equal_costs_the_older_road_into_the_first_flag_wins() {
    let (a, b, f) = ((2, 4), (10, 4), (6, 8));
    for (own_first, end) in [(false, a), (true, b)] {
        let (world, own) = own_road_network(own_first, [5, 29, 28]);
        let path = world.road(own).unwrap().path.clone();
        let to = path[path.len() / 2];
        let walk = world.road_walk_to_field(P0, own, f, to).unwrap();
        assert_eq!(end_taken(&walk, &path), Some(end), "{own_first}: {walk:?}");
        assert_eq!(walk.last(), Some(&to));
    }
}

// TRN-FREE T42 (TRN-FREE-6.1, Q-378): the walker's own road costs 1 in the
// search to the road's first flag A, and the walk ends at the other flag B
// when the route comes into A from B. (a) A by itself 11, over B 8 + 1: B,
// although by the road's length (L 4) A is cheaper (11 against 12). (b) A
// 34, over B 36 + 1: A. (c) Standing on B, or on A, it walks along the road
// from there.
// spec: TRN-FREE-6.1 (TRN-FREE T42)
#[test]
fn free_t42_the_walkers_own_road_costs_one() {
    let (a, b, f) = ((2, 4), (10, 4), (6, 8));
    // (a)
    let (world, own) = own_road_network(false, [4, 11, 8]);
    let path = world.road(own).unwrap().path.clone();
    let to = path[1];
    assert!(
        !world.road_walk(P0, f, a).unwrap().contains(&b),
        "by L: A directly"
    );
    let walk = world.road_walk_to_field(P0, own, f, to).unwrap();
    let mut expected = world.road_walk(P0, f, b).unwrap();
    expected.extend(along(&path, b, to));
    assert_eq!(walk, expected);
    // (b)
    let (world, own) = own_road_network(false, [3, 34, 36]);
    let path = world.road(own).unwrap().path.clone();
    let to = path[path.len() - 2];
    let walk = world.road_walk_to_field(P0, own, f, to).unwrap();
    let mut expected = world.road_walk(P0, f, a).unwrap();
    expected.extend(along(&path, a, to));
    assert_eq!(walk, expected);
    // (c)
    for at in [a, b] {
        let walk = world.road_walk_to_field(P0, own, at, to).unwrap();
        assert_eq!(walk, along(&path, at, to), "from {at:?}");
    }
}

// TRN-CARRY T43 (TRN-CARRY-5.4): the flag-after-G memory is asked only by
// the last-place test, when the ware would take G's last place: an entry
// check at a full G asks and stores nothing; one with G at its limit less 1
// stores the answer.
// spec: TRN-CARRY-5.4 (TRN-CARRY T43), TRN-CARRY-4.2
#[test]
fn t43_the_next_flag_memory_is_asked_only_for_the_last_place() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b) = ((2, 4), (8, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: a,
        destination: b,
    });
    while !matches!(world.carriers[0].task, CarrierTask::Carry { .. }) {
        world.step();
        assert!(world.tick() < 600, "never carried");
    }
    let ware = world.carriers[0].ware.expect("his ware");
    let limit = world.settings.flag_package_limit as usize;
    let fillers: Vec<usize> = (0..limit)
        .map(|_| world.new_ware_at(STONE, b, Some((14, 6)), None))
        .collect();
    assert!(!world.entry_check(0, b));
    assert_eq!(
        world.wares[ware].as_ref().unwrap().next_after,
        None,
        "full: nothing"
    );
    world.remove_ware(fillers[0]);
    world.entry_check(0, b);
    assert!(
        world.wares[ware].as_ref().unwrap().next_after.is_some(),
        "the last place"
    );
}

// TRN-ROAD T29 (TRN-ROAD-4.2): a carrier in his step onto the new flag F of a
// split, carrying to the far end of the half he gets, has his walk cut at F
// and arrives there with a place, without an entry check; he puts the ware
// down at F and carries it on from there. Road (2, 4) .. (10, 4), a plank
// from B to A, split at (6, 4) while he steps (7, 4) -> (6, 4).
// spec: TRN-ROAD-4.2 (TRN-ROAD T29)
#[test]
fn t29_a_split_ahead_cuts_the_walk_at_the_new_flag() {
    let mut world = transport_world(16, 8);
    world.roles.helper = Some(0x99);
    let (a, b, split) = ((2, 4), (10, 4), (6, 4));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    for _ in 0..30 {
        world.step();
    }
    world.submit(Command::DropWare {
        player: P0,
        good: PLANK,
        at: b,
        destination: a,
    });
    while world.carriers[0].walker.next != Some(split)
        || !matches!(world.carriers[0].task, CarrierTask::Carry { .. })
    {
        world.step();
        assert!(world.tick() < 600, "never stepped onto (6, 4)");
    }
    let ware = world.carriers[0].ware.expect("his ware");
    assert_eq!(flag(&mut world, split), Ok(()));
    let c = &world.carriers[0];
    let path = &world.road(c.road).unwrap().path;
    assert!(path.contains(&a) && path.contains(&split), "the half to A");
    assert_eq!(
        c.task,
        CarrierTask::Carry {
            to: split,
            entry: Entry::Reserved
        },
        "a place at F"
    );
    assert_eq!(c.walker.destination(), split);
    let mut lay = false;
    for _ in 0..400 {
        world.step();
        let at = world.wares[ware].as_ref().unwrap().location;
        lay |= at == WareLocation::Flag(split);
        if lay && at == WareLocation::Carried(0) {
            return;
        }
    }
    panic!("not put down at F and carried on (lay {lay})");
}

// TRN-FREE T56 (TRN-FREE-6.3 carrier, Q-046): a carrier standing on a flag
// that has no road to its own road's flags searches for its walk to its
// idle field again in every update, does not move and stays on its road;
// once a ware waits at one of its road's flags it starts a road walk for
// it (which fails the same way); with its road removed it goes home.
// spec: TRN-FREE-6.3 (TRN-FREE T56)
#[test]
fn t56_a_carrier_cut_off_from_its_road_keeps_it_and_searches() {
    let (mut world, a, m, b) = two_step_road();
    let island = (10, 4);
    assert_eq!(flag(&mut world, island), Ok(()));
    for _ in 0..200 {
        world.step();
    }
    assert_eq!(world.carriers()[0].task, CarrierTask::Idle);
    world.carriers[0].walker = Walker::new(island);
    world.carriers[0].task = CarrierTask::Out { flag: island };
    for _ in 0..5 {
        world.step();
        let c = &world.carriers()[0];
        assert!(matches!(c.task, CarrierTask::Arrive { .. }), "{:?}", c.task);
        assert_eq!(c.walker.at, island);
        assert!(!c.walker.is_walking());
        assert_eq!(c.road, 0);
    }
    drop(&mut world, PLANK, a, b);
    world.step();
    let c = &world.carriers()[0];
    assert_eq!(
        c.task,
        CarrierTask::Fetch { ware: 0, flag: a },
        "a waiting ware sends it walking"
    );
    assert_eq!(c.walker.at, island);
    assert!(!c.walker.is_walking(), "no road walk can start");
    for _ in 0..3 {
        world.step();
        assert_eq!(world.carriers()[0].walker.at, island);
    }
    assert_eq!(
        apply(&mut world, Command::RemoveRoad { player: P0, at: m }),
        Ok(())
    );
    world.step();
    assert_eq!(world.carriers()[0].task, CarrierTask::Retired);
}
