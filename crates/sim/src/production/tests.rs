use super::*;
use crate::movement::Walker;
use crate::requests::tests as req;
use crate::test_support::*;
use crate::{BuildingSize, BuildingType, GoodRules, PlayerId, Rules};

const WOOD: u32 = 6;
const STONE: u32 = req::STONE;
const PLANK: u32 = req::PLANK;
const CARPENTER: u32 = 20;
const WOODCUTTER: u32 = req::WOODCUTTER;
const STONECUTTER: u32 = 21;
const FORESTER: u32 = 22;
const PIG_FARMER: u32 = 23;
const HUT: u32 = 201;
const PIT: u32 = 202;
const NURSERY: u32 = 203;
const SAWMILL: u32 = 204;
const PIGGERY: u32 = 205;
const TREE: u32 = 300;
/// Tree families 0 to 3 and the default kind.
const FAMILY: [&[u32]; 4] = [
    &[310, 311],
    &[320, 321, 322, 323, 324, 325],
    &[330, 331],
    &[321, 331],
];
const DEFAULT_TREE: u32 = 320;
/// stone_01 .. stone_06
const STONES: [u32; 6] = [401, 402, 403, 404, 405, 406];

fn rules() -> Rules {
    let mut rules = req::rules(&req::full_stock());
    let building = |id, size, worker, product, radius, ticks, cycle| {
        let mut t = BuildingType::new(id, size);
        t.worker = Some(worker);
        t.product = product;
        t.working_radius = radius;
        t.production_ticks = ticks;
        t.cycle = cycle;
        t
    };
    let mut sawmill = building(
        SAWMILL,
        BuildingSize::Medium,
        CARPENTER,
        Some(PLANK),
        4,
        333,
        WorkCycle::Workshop,
    );
    sawmill.stock_limit = Some(6);
    let mut piggery = building(
        PIGGERY,
        BuildingSize::Medium,
        PIG_FARMER,
        Some(PLANK),
        4,
        333,
        WorkCycle::Workshop,
    );
    piggery.stock_limit = Some(6);
    rules.building_types.extend([
        building(
            HUT,
            BuildingSize::Small,
            WOODCUTTER,
            Some(WOOD),
            6,
            55,
            WorkCycle::Gatherer,
        ),
        building(
            PIT,
            BuildingSize::Small,
            STONECUTTER,
            Some(STONE),
            8,
            55,
            WorkCycle::Gatherer,
        ),
        building(
            NURSERY,
            BuildingSize::Small,
            FORESTER,
            None,
            6,
            55,
            WorkCycle::Forester,
        ),
        sawmill,
        piggery,
    ]);
    let rest = |r: u32| GoodRules {
        rest_ticks: r + 1,
        ..GoodRules::default()
    };
    rules.goods.insert(CARPENTER, rest(55));
    rules.goods.insert(PIG_FARMER, rest(222));
    for job in [WOODCUTTER, STONECUTTER, FORESTER] {
        rules.goods.entry(job).or_default().rest_ticks = 223;
    }
    let gathered = GoodRules {
        gathered: true,
        ..GoodRules::default()
    };
    rules.goods.insert(WOOD, gathered.clone());
    rules.goods.insert(STONE, gathered);
    rules.goods.insert(
        PLANK,
        GoodRules {
            costs: vec![(WOOD, 1)],
            ..GoodRules::default()
        },
    );
    let tree = |id| DepositKind {
        id,
        good: Some(WOOD),
        next: None,
        elevation: 1,
        grow_ticks: 1001,
        blocking: false,
        life_time: None,
        needs_plantable: false,
        random_offset: false,
    };
    rules.deposit_kinds = std::iter::once(TREE)
        .chain(FAMILY.iter().flat_map(|f| f.iter().copied()))
        .map(tree)
        .collect();
    for (i, &id) in STONES.iter().enumerate() {
        rules.deposit_kinds.push(DepositKind {
            id,
            good: Some(STONE),
            next: (i > 0).then(|| STONES[i - 1]),
            elevation: 2,
            grow_ticks: 0,
            blocking: false,
            life_time: None,
            needs_plantable: false,
            random_offset: false,
        });
    }
    rules.tree_families = FAMILY.iter().map(|f| f.to_vec()).collect();
    rules.default_tree = Some(DEFAULT_TREE);
    rules
}

fn open_world(width: u32, height: u32) -> World {
    World::with_map(5, flat(width, height), rules(), P0)
}

/// A finished building of `kind` on `at`.
fn finished(world: &mut World, kind: u32, at: Point) -> Target {
    assert_eq!(building(world, kind, at), Ok(()));
    world.sites.remove(&at);
    world.complete(at);
    world.target_of(at).unwrap()
}

/// A worker inside the building on `at`, starting its cycle.
fn worker(world: &mut World, at: Point, profession: u32) -> usize {
    let target = world.target_of(at).unwrap();
    let index = world.terrain.index(at);
    let b = world.buildings[index].as_mut().unwrap();
    b.workers = 1;
    b.active = true;
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession,
        list: crate::SettlerList::Workers,
        walker: Walker::new(at),
        job: SettlerJob::Worker {
            target,
            work: Work::Begin,
        },
        timer: 0,
        carrying: None,
    });
    world.file_input_orders(at);
    id
}

fn tree(world: &mut World, field: Point, kind: u32) {
    world.add_deposit(
        field,
        Deposit {
            kind,
            position: grid::field_sample(field),
            state: DepositState::Growing { ripe_at: 0 },
            reserved_by: None,
            life_left: None,
        },
    );
    world.changed(&[field]);
}

fn work_of(world: &World, id: usize) -> Work {
    match world.settlers[id].as_ref().unwrap().job {
        SettlerJob::Worker { work, .. } => work,
        ref other => panic!("not working: {other:?}"),
    }
}

/// Step `ticks` times and record each change of the worker's step.
fn trace(world: &mut World, id: usize, ticks: u32) -> Vec<(Tick, Work)> {
    let mut changes = Vec::new();
    let mut last = work_of(world, id);
    for _ in 0..ticks {
        world.step();
        let now = work_of(world, id);
        if std::mem::discriminant(&now) != std::mem::discriminant(&last)
            || matches!((now, last), (Work::Take { second: a }, Work::Take { second: b }) | (Work::PutDown { second: a }, Work::PutDown { second: b }) | (Work::TakeWare { second: a, .. }, Work::TakeWare { second: b, .. }) if a != b)
        {
            changes.push((world.tick() - 1, now));
        }
        last = now;
    }
    changes
}

/// Ticks at which the step became one matching `pred`.
fn when(changes: &[(Tick, Work)], pred: impl Fn(&Work) -> bool) -> Vec<Tick> {
    changes
        .iter()
        .filter(|(_, w)| pred(w))
        .map(|&(t, _)| t)
        .collect()
}

fn sawmill_with_logs(world: &mut World, logs: u32) -> (usize, Point) {
    let at = (10, 10);
    finished(world, SAWMILL, at);
    world.inputs.insert(at, vec![(WOOD, logs)]);
    (worker(world, at, CARPENTER), at)
}

// ECO-WORK T1, T2, T3, T4, T6, T10
// spec: ECO-WORK-3.2, ECO-WORK-3.3, ECO-WORK-3.9, ECO-WORK-4.2, ECO-WORK-4.3 (ECO-WORK T1, T2, T3, T4, T6, T10)
#[test]
fn workshop_cycle_ticks() {
    let mut world = open_world(30, 30);
    let (id, _) = sawmill_with_logs(&mut world, 6);
    let changes = trace(&mut world, id, 1400);
    let starts = when(&changes, |w| *w == Work::Working);
    let ends = when(&changes, |w| *w == Work::Take { second: false });
    let t0 = ends[0];
    // T1
    assert_eq!(t0 - starts[0], 333);
    // T4: ECO-WORK-4.1, from the end of the work action.
    let at = |pred: &dyn Fn(&Work) -> bool| {
        when(&changes, pred).into_iter().find(|&t| t >= t0).unwrap() - t0
    };
    assert_eq!(
        at(&|w| *w == Work::Take { second: true }),
        4,
        "T10: 3.3 lasts 4"
    );
    assert_eq!(
        at(&|w| matches!(w, Work::WaitRoom { .. })),
        8,
        "product taken"
    );
    assert_eq!(at(&|w| matches!(w, Work::Out { .. })), 10, "leaves");
    assert_eq!(at(&|w| *w == Work::AtFlag), 33, "at the flag");
    assert_eq!(
        at(&|w| *w == Work::PutDown { second: true }),
        38,
        "ware lies"
    );
    assert_eq!(at(&|w| *w == Work::In), 42, "turns back");
    assert_eq!(at(&|w| *w == Work::StepIn), 65, "back");
    assert_eq!(at(&|w| *w == Work::Rest), 66, "rest begins");
    assert_eq!(at(&|w| *w == Work::Begin), 122, "rest over");
    // T2 (ECO-WORK-3.4 as corrected in 77ed505: two standing ticks, one
    // more than the 122 of the table)
    assert_eq!(starts[1] - t0, 123);
    // T3: ware laid to ware laid.
    let laid = when(&changes, |w| *w == Work::PutDown { second: true });
    assert_eq!(laid[1] - laid[0], 456);
    // T6
    assert_eq!(starts[1] - laid[0], 85);
}

// ECO-WORK T5: no log when the rest ends; work starts within 1 tick of
// its arrival, without a second rest.
// spec: ECO-WORK-3.1, ECO-WORK-3.10 (ECO-WORK T5)
#[test]
fn workshop_waits_for_inputs() {
    let mut world = open_world(30, 30);
    let (id, at) = sawmill_with_logs(&mut world, 1);
    let changes = trace(&mut world, id, 333 + 122 + 50);
    assert_eq!(changes.last().unwrap().1, Work::WaitInputs);
    for _ in 0..100 {
        world.step();
        assert_eq!(work_of(&world, id), Work::WaitInputs);
    }
    world.receive_input(at, WOOD);
    let arrived = world.tick();
    let changes = trace(&mut world, id, 3);
    assert_eq!(changes[0], (arrived, Work::Working));
}

// ECO-WORK T11 (ECO-WORK-3.1b): while production is switched off the worker
// stands in a state of his own; in the tick after it is switched on again
// he only returns to waiting, his first check in the next: with inputs in
// stock the work starts two ticks after the switch.
// spec: ECO-WORK-3.1b (ECO-WORK T11), ECO-WORK-3.1
#[test]
fn t11_the_work_starts_two_ticks_after_production_is_on_again() {
    let mut world = open_world(30, 30);
    let (id, at) = sawmill_with_logs(&mut world, 1);
    world.receive_input(at, WOOD);
    let toggle = |world: &mut World| {
        world.submit(crate::Command::ProductionToggle { player: P0, at });
        world.step();
    };
    while work_of(&world, id) != Work::Begin {
        world.step();
        assert!(world.tick() < 2000, "home");
    }
    toggle(&mut world);
    for _ in 0..20 {
        world.step();
        assert_eq!(work_of(&world, id), Work::Off);
    }
    // A command takes effect at the tick's boundary: in the tick after it
    // he returns to waiting, the work starts in the next.
    toggle(&mut world);
    assert_eq!(work_of(&world, id), Work::Begin, "back to waiting");
    world.step();
    assert_eq!(work_of(&world, id), Work::Working, "the first check");
}

// ECO-WORK T-arrive-1 (ECO-WORK-3.1a): a worker arriving in a workshop with
// its inputs in stock stands in the arrival tick and in one more (the first
// waiting tick); its first check comes in the tick after that, so the work
// begins two ticks after the arrival tick.
// spec: ECO-WORK-3.1a (T-arrive-1), ECO-WORK-3.1, ECO-REQ-4.2
#[test]
fn t_arrive_1_the_first_check_two_ticks_after_arriving() {
    let mut world = open_world(30, 30);
    let at = (10, 10);
    let target = finished(&mut world, SAWMILL, at);
    world.inputs.insert(at, vec![(WOOD, 6)]);
    let index = world.terrain.index(at);
    world.buildings[index].as_mut().unwrap().workers = 1;
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: CARPENTER,
        list: crate::SettlerList::Workers,
        walker: Walker::new(at),
        job: SettlerJob::ToBuilding { target },
        timer: 0,
        carrying: None,
    });
    world.step();
    let arrival = world.tick() - 1;
    assert_eq!(work_of(&world, id), Work::Begin, "arrived in {arrival}");
    world.step();
    assert_eq!(work_of(&world, id), Work::Begin, "the first waiting tick");
    world.step();
    assert_eq!(work_of(&world, id), Work::Working);
    assert_eq!(world.tick() - 1, arrival + 2);
}

// ECO-REQ T25 (ECO-REQ-4.2): a building's input orders are filed by its own
// update, in the buildings' part of a tick: its worker arrives in the
// settlers' part, so the orders appear in the tick after his arrival, and
// the matching of the arrival tick does not see them.
// spec: ECO-REQ-4.2 (ECO-REQ T25), ECO-TIME-1.5
#[test]
fn t25_the_input_orders_come_a_tick_after_the_worker() {
    let mut world = open_world(30, 30);
    let at = (10, 10);
    let target = finished(&mut world, SAWMILL, at);
    let index = world.terrain.index(at);
    world.buildings[index].as_mut().unwrap().workers = 1;
    let orders = |world: &World| world.orders().iter().filter(|o| o.target.at == at).count();
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: CARPENTER,
        list: crate::SettlerList::Workers,
        walker: Walker::new(at),
        job: SettlerJob::ToBuilding { target },
        timer: 0,
        carrying: None,
    });
    world.step();
    assert_eq!(work_of(&world, id), Work::Begin, "arrived");
    assert_eq!(orders(&world), 0, "none in the arrival tick");
    world.step();
    assert!(orders(&world) > 0, "filed in the next");
}

// ECO-WORK T7: a full flag keeps the worker inside after taking the
// product; T-room-1: he is admitted by the entry check, a tick after the
// first check that finds room.
// spec: ECO-WORK-3.4, ECO-PROD-8.5 (ECO-WORK T7, T-room-1), TRN-CARRY-4.4
#[test]
fn workshop_waits_for_room() {
    let mut world = open_world(30, 30);
    let (id, at) = sawmill_with_logs(&mut world, 6);
    let flag = world.terrain.neighbour(at, 1).unwrap();
    let fillers: Vec<usize> = (0..6)
        .map(|_| world.new_ware_at(STONE, flag, Some((29, 29)), None))
        .collect();
    let changes = trace(&mut world, id, 600);
    assert!(matches!(changes.last().unwrap().1, Work::WaitRoom { .. }));
    // TRN-CARRY-4.2: a product without a destination takes the last
    // place freely, so with 5 wares left it may go out. ECO-WORK-3.4
    // (T-room-1): the first check that finds room raises the flag's bar,
    // the next admits the product: he walks out a tick later.
    world.remove_ware(fillers[0]);
    let freed = world.tick();
    let changes = trace(&mut world, id, 3);
    let out = changes
        .iter()
        .find(|(_, w)| matches!(w, Work::Out { .. }))
        .unwrap();
    assert_eq!(out.0, freed + 1, "{changes:?}");
}

// ECO-WORK T-room-2 (ECO-WORK-3.4): the check of the second standing tick is
// a whole entry check: when the flag's bar already holds the product's
// score (another ware's bid set it), it admits the product and the walk out
// starts in that tick, a tick before the third.
// spec: ECO-WORK-3.4 (ECO-WORK T-room-2), TRN-CARRY-4.4
#[test]
fn t_room_2_a_bar_set_by_another_bid_admits_in_the_second_tick() {
    let mut world = open_world(30, 30);
    let (id, at) = sawmill_with_logs(&mut world, 6);
    let flag = world.terrain.neighbour(at, 1).unwrap();
    let work = |w: &World| match &w.settlers[id].as_ref().unwrap().job {
        SettlerJob::Worker { work, .. } => *work,
        other => panic!("{other:?}"),
    };
    while !matches!(work(&world), Work::WaitRoom { .. }) {
        world.step();
        assert!(world.tick() < 600, "never took a product");
    }
    // Another ware's bid raised the bar to the product's score 0.
    world.flag_bar.insert(flag, 0);
    world.step();
    assert!(
        matches!(work(&world), Work::Out { .. }),
        "{:?}",
        work(&world)
    );
}

// ECO-WORK T8, T9
// spec: ECO-WORK-4.4, ECO-WORK-3.9 (ECO-WORK T8, T9)
#[test]
fn slope_and_rest_change_the_gap() {
    let gap = |raise: i32, kind: u32, profession: u32| {
        let mut terrain = flat(30, 30);
        let at = (10, 10);
        let flag = terrain.neighbour(at, 1).unwrap();
        set_height(&mut terrain, flag, raise);
        let mut world = World::with_map(5, terrain, rules(), P0);
        finished(&mut world, kind, at);
        world.inputs.insert(at, vec![(WOOD, 6)]);
        let id = worker(&mut world, at, profession);
        let changes = trace(&mut world, id, 1400);
        let starts = when(&changes, |w| *w == Work::Working);
        let ends = when(&changes, |w| *w == Work::Take { second: false });
        starts[1] - ends[0]
    };
    // ECO-WORK-3.4 as corrected (77ed505): one tick more than the table's
    // 122, 127 and 289.
    assert_eq!(gap(0, SAWMILL, CARPENTER), 123);
    assert_eq!(gap(400, SAWMILL, CARPENTER), 128, "T8");
    assert_eq!(gap(0, PIGGERY, PIG_FARMER), 290, "T9");
}

// ECO-REQ T8, T9: the stock limit is ordered once the worker is in; each
// plank orders one log again.
// spec: ECO-REQ-4.2, ECO-REQ-4.3, ECO-PROD-6.1 (ECO-REQ T8, T9)
#[test]
fn workshop_orders_its_inputs() {
    let mut world = open_world(30, 30);
    let at = (10, 10);
    finished(&mut world, SAWMILL, at);
    assert!(world.orders().is_empty());
    let id = worker(&mut world, at, CARPENTER);
    assert_eq!(world.open_orders(at, WOOD), 6, "T8");
    world.orders.clear();
    world.inputs.insert(at, vec![(WOOD, 6)]);
    let changes = trace(&mut world, id, 1000);
    let made = when(&changes, |w| *w == Work::Take { second: false }).len() as u32;
    assert_eq!(made, 2);
    assert_eq!(world.open_orders(at, WOOD), made, "T9");
    assert_eq!(world.inputs(at), &[(WOOD, 4)]);
}

// ECO-OUT T1, T2, T4
// spec: ECO-OUT-2.3, ECO-OUT-2.4, ECO-OUT-2.6 (ECO-OUT T1, T2, T4)
#[test]
fn target_search_in_flood_order() {
    let at = (10, 10);
    let search = |trees: &[Point]| {
        let mut world = open_world(30, 30);
        finished(&mut world, HUT, at);
        for &t in trees {
            tree(&mut world, t, TREE);
        }
        world.search_target(at, WOOD, 6)
    };
    let d2 = grid::ring(at, 2, 30, 30)[0];
    let d3 = grid::ring(at, 3, 30, 30)[0];
    assert_eq!(search(&[d3, d2]), Some(d2), "T1");
    let dir = |d| grid::neighbour(at, grid::Direction::new(d).unwrap(), 30, 30).unwrap();
    assert_eq!(search(&[dir(0), dir(5)]), Some(dir(5)), "T2");
    // T4: hut A's reserved tree is not B's.
    let mut world = open_world(30, 30);
    finished(&mut world, HUT, at);
    finished(&mut world, HUT, (14, 10));
    tree(&mut world, (12, 10), TREE);
    let a = world.target_of(at).unwrap();
    world.deposits.get_mut(&(12, 10)).unwrap().reserved_by = Some(a);
    assert_eq!(world.search_target((14, 10), WOOD, 6), None);
}

/// A hut on (10, 10) with its worker and the given trees.
fn hut_with(trees: &[Point]) -> (World, usize) {
    let mut world = open_world(30, 30);
    finished(&mut world, HUT, (10, 10));
    for &t in trees {
        tree(&mut world, t, TREE);
    }
    let id = worker(&mut world, (10, 10), WOODCUTTER);
    (world, id)
}

// TRN-FREE T48 (TRN-FREE-5.3, E-trn-041): the fine approach's steps each
// take their rate from the heights when they start: the ground ahead rising
// while the woodcutter walks (a site levelled nearby) slows the steps not
// yet begun, so he reaches the tree later than the heights at the order
// gave; the step under way keeps its rate.
// spec: TRN-FREE-5.3 (TRN-FREE T48)
#[test]
fn t48_the_approachs_steps_take_their_rates_when_they_start() {
    let trees = [(13, 8)];
    let (mut world, id) = hut_with(&trees);
    let mut ordered = None;
    for _ in 0..1200 {
        world.step();
        if matches!(work_of(&world, id), Work::Approach { .. }) {
            ordered = Some((world.tick(), world.settlers[id].as_ref().unwrap().timer));
            break;
        }
    }
    let (w, at_order) = ordered.expect("the approach");
    let field = world.settlers[id].as_ref().unwrap().walker.at;
    let steps = world
        .approach_steps(field, trees[0], false)
        .expect("a tree");
    assert!(steps.len() >= 2, "{steps:?}");
    // The first step under way; the ground ahead of it rises (class 2 a
    // field).
    world.step();
    let first = world.fine_position(world.settlers[id].as_ref().unwrap());
    let rate_before = first.expect("on the lattice").progress_bits;
    // A sample grid of the field heights (flat), then the samples after the
    // first step raised by a class 2 rise each.
    let (columns, rows) = world.terrain.sample_size();
    let base = world.terrain.sample_height(steps[0]);
    world.terrain.samples = vec![base; (columns * rows) as usize];
    for (k, &sample) in steps.iter().enumerate().skip(1) {
        world
            .terrain
            .set_sample_height(sample, base + 700 * k as i32);
    }
    world.step();
    let second = world.fine_position(world.settlers[id].as_ref().unwrap());
    assert_ne!(
        second.expect("on the lattice").progress_bits,
        rate_before,
        "the step under way goes on"
    );
    let mut reached = None;
    for _ in 0..400 {
        world.step();
        if matches!(work_of(&world, id), Work::AtSpot { .. }) {
            reached = Some(world.tick());
            break;
        }
    }
    let took = reached.expect("he works") - w;
    assert!(
        took > u64::from(at_order),
        "the later steps slower: {took} ticks, {at_order} at the order's heights"
    );
}

// TRN-FREE-5: the woodcutter walks the lattice to the tree, works and
// takes the log one lattice step (the tree's elevation size) from the
// tree's own sample, facing it; on his other steps he has no lattice
// place.
#[test]
fn woodcutter_stands_at_the_tree() {
    let trees = [(13, 8)];
    let (mut world, id) = hut_with(&trees);
    let tree_at = world.deposit(trees[0]).expect("a tree").position;
    let (columns, rows) = world.terrain.sample_size();
    let mut approach = Vec::new();
    let (mut worked, mut taken) = (0, 0);
    for _ in 0..1200 {
        world.step();
        let settler = world.settlers[id].as_ref().expect("the worker");
        let fine = world.fine_position(settler);
        match work_of(&world, id) {
            Work::Approach { .. } => approach.push(fine.expect("on the lattice")),
            Work::AtSpot { .. } | Work::TakeWare { .. } => {
                let fine = fine.expect("at the tree");
                let beside = (0..6).any(|d| {
                    grid::neighbour(fine.at, grid::Direction::new(d).unwrap(), columns, rows)
                        == Some(tree_at)
                });
                assert!(beside, "{fine:?} beside {tree_at:?}");
                assert_eq!(fine.next, None);
                if matches!(work_of(&world, id), Work::AtSpot { .. }) {
                    assert_eq!(
                        fine.facing,
                        Some(movement::direction_toward(fine.at, tree_at))
                    );
                    worked += 1;
                } else {
                    taken += 1;
                }
            }
            // TRN-FREE-5.6: the walk home's lattice part.
            Work::Return { .. } => assert!(fine.is_some()),
            _ => assert_eq!(fine, None),
        }
    }
    assert!(!approach.is_empty());
    assert!(approach.iter().all(|f| f.next.is_some()));
    assert_eq!((worked, taken), (55, 8));
    // ECO-OUT-3.5 (corrected Run D): the removal timer is 60 ticks
    // after the work began, 5 after it ended; it is gone 18 after the
    // harvest.
    let (mut world, id) = hut_with(&trees);
    let mut work_start = None;
    for _ in 0..1200 {
        world.step();
        if work_start.is_none() && matches!(work_of(&world, id), Work::AtSpot { .. }) {
            work_start = Some(world.tick());
        }
        if let Some((falls, gone)) = world.fall_of(trees[0]) {
            let start = work_start.expect("the work came first");
            // The work began in the tick before AtSpot was first seen.
            assert_eq!(falls, start - 1 + 60);
            assert_eq!(gone - falls, REMOVAL_TICKS - FALL_AFTER_WORK);
            return;
        }
    }
    panic!("the tree was not felled");
}

// GFX-ANIM-3.8a: the fellings of the whole map in one pass over the settlers
// are each tree's own felling start, tick by tick, while two woodcutters
// work at their trees, fell them and the trees lie and go; a forester's
// sapling is in neither.
// spec: GFX-ANIM-3.8a, ECO-OUT-4.7
#[test]
fn the_fellings_are_each_trees_felling_start() {
    let each = |world: &World| -> Vec<(Point, Tick)> {
        world
            .deposits()
            .filter_map(|(f, _)| Some((f, world.felling_began(f)?)))
            .collect()
    };
    let trees = [(13, 8), (14, 9)];
    let (mut world, _) = hut_with(&trees);
    let mut seen = 0;
    for _ in 0..1500 {
        world.step();
        assert_eq!(world.fellings(), each(&world), "tick {}", world.tick());
        seen += world.fellings().len();
    }
    assert!(seen > 100, "felling seen in {seen} tick-trees");
    let (mut world, _) = nursery(Some(1));
    for _ in 0..600 {
        world.step();
        assert_eq!(world.fellings(), each(&world));
    }
}

// GFX-ANIM-3.8 (Run D): the woodcutter's work arms a removal timer of
// work time + 5 (60) ticks on his tree: interrupted (his hut torn down
// mid-chop), the tree still starts its removal in W + 60 and is gone 19
// ticks later (the counter starts at 1 there, as ECO-OUT-6.5's).
#[test]
fn interrupted_felling_removes_the_tree() {
    let trees = [(13, 8)];
    let (mut world, id) = hut_with(&trees);
    // `world.tick()` after a step is one past the tick that step ran.
    let mut began = None;
    for _ in 0..1200 {
        world.step();
        if matches!(work_of(&world, id), Work::AtSpot { .. }) {
            began = Some(world.tick() - 1);
            break;
        }
    }
    let began = began.expect("the work began");
    for _ in 0..20 {
        world.step();
    }
    world.submit(crate::Command::RemoveBuilding {
        player: P0,
        at: (10, 10),
    });
    while world.tick() < began + 60 {
        world.step();
        assert!(
            world
                .deposit(trees[0])
                .is_some_and(|d| d.ripe(world.tick())),
            "tick {}",
            world.tick()
        );
    }
    world.step();
    assert_eq!(
        world.deposit(trees[0]).map(|d| d.state),
        Some(DepositState::Removing {
            since: began + 60,
            gone_at: began + 60 + 19,
        })
    );
}

// TRN-CARRY-7.2a (Q-230 a, b): a woodcutter carrying his log home when his
// hut is removed is sent home and loses the log at once: it goes into no
// warehouse and is not put down; a settler sent home never holds a ware.
// spec: TRN-CARRY-7.2a, TRN-FREE-6.4
#[test]
fn a_released_worker_loses_the_ware_in_his_hands_at_once() {
    let trees = [(13, 8)];
    let (mut world, id) = hut_with(&trees);
    let mut carrying = false;
    for _ in 0..3000 {
        world.step();
        if world.settlers[id]
            .as_ref()
            .is_some_and(|s| s.carrying.is_some())
        {
            carrying = true;
            break;
        }
    }
    assert!(carrying, "the woodcutter never carried his log");
    world.submit(crate::Command::RemoveBuilding {
        player: P0,
        at: (10, 10),
    });
    world.step();
    let s = world.settlers[id].as_ref().expect("the woodcutter");
    assert!(
        matches!(s.job, crate::SettlerJob::Leaving { .. }),
        "{:?}",
        s.job
    );
    assert_eq!(s.carrying, None, "the log is gone at once");
}

// ECO-OUT-5.4, 3.10, play-test: the woodcutter's door to door, from the
// door closing behind him (he steps in) to it opening again (he walks out):
// step in 1 + rest 223 + target check 51 = 275 ticks = 27.5 s of game time
// at 10 ticks a second (the wait begins in the tick the rest is over:
// E-trace-010, woodcutter 90392 in at 2419, out at 2694); the player timed
// about 27 s in the original. The same for every trip, whatever the tree.
// spec: ECO-OUT-5.4, ECO-OUT-3.10
#[test]
fn woodcutter_door_to_door() {
    let trees = [(13, 8), (14, 8), (8, 12)];
    let (mut world, id) = hut_with(&trees);
    let changes = trace(&mut world, id, 3000);
    let inside = when(&changes, |w| *w == Work::StepIn);
    let outside = when(&changes, |w| matches!(w, Work::Out { .. }));
    assert!(inside.len() >= 2, "{changes:?}");
    for &closed in &inside {
        let Some(&opened) = outside.iter().find(|&&t| t > closed) else {
            continue;
        };
        assert_eq!(opened - closed, 275, "door closed at {closed}");
        // ECO-TIME: a tick is 0.1 s of game time.
        let seconds = (opened - closed) as f64 * 0.1;
        assert!(
            (seconds - 27.0).abs() < 1.0,
            "{seconds} s against about 27 s"
        );
    }
}

// ECO-WORK T-door-4 (ECO-WORK-3.8, 3.8a, 3.8b): a workplace with a door
// field other than its own: coming back, the worker's walk ends on the
// building's field and a step of its own from progress 0 takes him to the
// door field, where he waits; going out again, a step of its own from the
// door onto the building's field comes first, then the walk to the flag
// from progress 0.
// spec: ECO-WORK-3.8 (ECO-WORK T-door-4), ECO-WORK-3.8a, ECO-WORK-3.8b
#[test]
fn t_door_4_a_worker_uses_the_door_field() {
    let trees = [(13, 8), (14, 8), (8, 12)];
    let (mut world, id) = hut_with(&trees);
    for kind in world.building_types.values_mut() {
        if kind.id == HUT {
            kind.doors = [Some(5); 3];
        }
    }
    let at = (10, 10);
    let door = world.terrain.neighbour(at, 5).unwrap();
    let flag = world.terrain.neighbour(at, 1).unwrap();
    let walker = |world: &World| world.settlers[id].as_ref().unwrap().walker.clone();
    // Back in after the first trip: on the door field.
    let mut last = work_of(&world, id);
    let mut seen_in = false;
    for _ in 0..3000 {
        world.step();
        let now = work_of(&world, id);
        if now == Work::StepIn && last != Work::StepIn {
            assert_eq!((walker(&world).at, walker(&world).next), (door, None));
            seen_in = true;
        }
        if seen_in && matches!(now, Work::Out { .. }) && !matches!(last, Work::Out { .. }) {
            // Out again: the door step onto the building's field, a pause
            // there, then the flag.
            let w = walker(&world);
            assert_eq!(w.at, door);
            let rest: Vec<Point> = w.next.into_iter().chain(w.path.iter().copied()).collect();
            assert_eq!(rest, [at, flag], "{w:?}");
            assert!(w.pauses.contains(&at), "{w:?}");
            return;
        }
        last = now;
    }
    panic!("no second trip");
}

// ECO-OUT T3: nothing within radius 6; the worker checks every 51 ticks
// and stays in.
// spec: ECO-OUT-2.1, ECO-OUT-3.2 (ECO-OUT T3)
#[test]
fn t3_no_target_in_range() {
    let far = grid::ring((10, 10), 7, 30, 30)[0];
    let (mut world, id) = hut_with(&[far]);
    let mut checks = Vec::new();
    for _ in 0..400 {
        world.step();
        let work = work_of(&world, id);
        assert!(matches!(work, Work::WaitTarget { .. }), "{work:?}");
        if work == (Work::WaitTarget { countdown: 50 }) {
            checks.push(world.tick() - 1);
        }
    }
    assert_eq!(world.reserved_target((10, 10)), None);
    assert!(checks.len() >= 7);
    assert!(checks.windows(2).all(|p| p[1] - p[0] == 51));
}

// ECO-OUT-2.1 (ECO-OUT T22): the building's target search runs on the
// search counter, every 51 ticks from the building's finishing (it gains 1
// a tick and 1 more while it is not 0), not on the 101-tick timer: a tree
// planted after a search is reserved at the next one.
// spec: ECO-OUT-2.1, ECO-OUT-2.2 (ECO-OUT T22)
#[test]
fn t22_the_target_search_runs_every_51_ticks() {
    let (mut world, _) = hut_with(&[]);
    let at = (10, 10);
    let t0 = world.tick();
    let mut searches = Vec::new();
    let mut reserved = None;
    for _ in 0..240 {
        world.step();
        let k = world.tick() - t0;
        if world.search_timers.get(&at) == Some(&0) {
            searches.push(k);
        }
        if k == 60 {
            tree(&mut world, (12, 8), TREE);
        }
        if reserved.is_none() && world.reserved_target(at).is_some() {
            reserved = Some(k);
        }
    }
    assert_eq!(searches, [51, 102, 153, 204]);
    assert_eq!(reserved, Some(102), "the next search after the planting");
}

// ECO-OUT T5: target reserved: the worker leaves 51 ticks after the
// cycle starts.
// spec: ECO-OUT-3.2, ECO-OUT-3.3 (ECO-OUT T5)
#[test]
fn t5_leaves_after_51_ticks() {
    let tree_at = (12, 8);
    let (mut world, id) = hut_with(&[tree_at]);
    let target = world.target_of((10, 10)).unwrap();
    world.deposits.get_mut(&tree_at).unwrap().reserved_by = Some(target);
    world.targets.insert((10, 10), tree_at);
    let changes = trace(&mut world, id, 200);
    let start = when(&changes, |w| {
        matches!(w, Work::WaitTarget { countdown: 50 })
    })[0];
    let out = when(&changes, |w| matches!(w, Work::Out { .. }))[0];
    assert_eq!(out - start, 51);
    let flag = when(&changes, |w| matches!(w, Work::ToSpot { .. }))[0];
    assert_eq!(flag - out, 23, "at the flag 23 ticks later");
}

// ECO-OUT T6, T14 and the whole woodcutter cycle on level ground.
// spec: ECO-OUT-3.4, ECO-OUT-3.5, ECO-OUT-3.6, ECO-OUT-3.7, ECO-OUT-3.8, ECO-OUT-3.9, ECO-OUT-3.10, ECO-OUT-6.4 (ECO-OUT T6, T14)
#[test]
fn t6_t14_felling_and_cycle() {
    let trees = [(13, 8), (14, 8)];
    let (mut world, id) = hut_with(&trees);
    let changes = trace(&mut world, id, 2400);
    // The building searched at its timer and the worker felled both.
    let work = when(&changes, |w| matches!(w, Work::AtSpot { .. }));
    let felled = when(&changes, |w| {
        matches!(w, Work::TakeWare { second: false, .. })
    });
    assert_eq!(felled.len(), 2);
    assert_eq!(felled[0] - work[0], 55, "T6: 55 ticks of work");
    // T6: gone 18 ticks after the harvest.
    let flag = world.terrain.neighbour((10, 10), 1).unwrap();
    assert!(world.deposit(trees[0]).is_none() && world.deposit(trees[1]).is_none());
    // T14: ware laid to ware laid = 394 + W1 + W2 of the second log
    // (ECO-WORK-3.6 put-down; ECO-OUT-5.1 says about 398, Q-115).
    let laid = when(&changes, |w| *w == Work::PutDown { second: true });
    assert_eq!(laid.len(), 2);
    let out = when(&changes, |w| matches!(w, Work::ToSpot { .. }));
    let at_tree = when(&changes, |w| matches!(w, Work::AtSpot { .. }));
    // ECO-OUT-5.4: W2 includes the fine walk back (TRN-FREE-5.6).
    let back_start = when(&changes, |w| matches!(w, Work::Return { .. }));
    let at_flag = when(&changes, |w| *w == Work::AtFlag);
    let w1 = at_tree[1] - out[1];
    let w2 = at_flag[1] - back_start[1];
    let start = when(&changes, |w| {
        matches!(w, Work::WaitTarget { countdown: 50 })
    });
    // The second cycle had its target reserved before it started.
    assert_eq!(laid[1] - start[1], 142 + w1 + w2);
    // ECO-OUT-3.10: the wait begins in the tick the rest is over, with no
    // Begin step between.
    assert!(when(&changes, |w| *w == Work::Begin).is_empty());
    let rest = when(&changes, |w| *w == Work::Rest);
    assert_eq!(start[1] - rest[0], 223, "the rest of 223 ticks");
    assert_eq!(start[1] - laid[0], 251);
    let _ = flag;
}

// ECO-OUT T23 (ECO-OUT-3.8a (1), (3)): the woodcutter carrying his log
// back makes one entry check for the flag as his walk back starts (it
// raises the flag's bar); his longer free walk to the flag is ordered
// without a check, and from the next tick he checks in every tick: the
// first admits him, his place is reserved while he still walks, and
// arriving he puts the log down at once.
// spec: ECO-OUT-3.8a (ECO-OUT T23), TRN-CARRY-4.3, TRN-CARRY-4.4
#[test]
fn t23_the_woodcutter_asks_for_his_place_on_the_way() {
    let (mut world, id) = hut_with(&[(13, 8)]);
    let flag = world.terrain.neighbour((10, 10), 1).unwrap();
    while !matches!(work_of(&world, id), Work::Back { .. }) {
        world.step();
        assert!(world.tick() < 2000, "walks back");
    }
    let number = world.settlers[id].as_ref().unwrap().number;
    assert!(world.settlers[id].as_ref().unwrap().walker.path.len() > 1);
    assert_eq!(world.flag_bar.get(&flag), Some(&0), "the take's check");
    assert!(!world.held_joins.contains_key(&number), "none as ordered");
    assert!(!world.worker_reservations().contains(&flag));
    world.step();
    assert!(world.held_joins.contains_key(&number), "admitted");
    assert!(world.worker_reservations().contains(&flag), "reserved");
    assert!(world.settlers[id].as_ref().unwrap().walker.is_walking());
    while world.settlers[id].as_ref().unwrap().walker.is_walking() {
        world.step();
    }
    assert_eq!(work_of(&world, id), Work::AtFlag, "the put-down at once");
}

// ECO-OUT T28 (ECO-OUT-3.8a (3), the view on admission): the woodcutter's
// longer walk back to a full flag holds him before it (standing); in the
// tick a place is free and his check admits him his walk shows again at
// once, still on his field and at progress 0; the step moves from the next
// tick (Q-1078 answered).
// spec: ECO-OUT-3.8a (ECO-OUT T28)
#[test]
fn t28_admitted_before_the_full_flag_his_walk_shows_at_once() {
    let (mut world, id) = hut_with(&[(13, 8)]);
    let flag = world.terrain.neighbour((10, 10), 1).unwrap();
    let fillers: Vec<usize> = (0..6)
        .map(|_| world.new_ware_at(STONE, flag, Some((29, 29)), None))
        .collect();
    let walker = |w: &World| w.settlers[id].as_ref().unwrap().walker.clone();
    let mut held = false;
    for _ in 0..3000 {
        world.step();
        if matches!(work_of(&world, id), Work::Back { .. }) && walker(&world).held {
            held = true;
            break;
        }
    }
    assert!(held, "held before the full flag");
    let number = world.settlers[id].as_ref().unwrap().number;
    for _ in 0..5 {
        world.step();
        assert!(!walker(&world).has_walk(), "he stands");
    }
    world.remove_ware(fillers[0]);
    while !world.held_joins.contains_key(&number) {
        world.step();
        assert!(world.tick() < 6000, "admitted");
    }
    let at = walker(&world).at;
    assert!(
        walker(&world).has_walk(),
        "his walk shows in the admission tick"
    );
    assert_eq!(walker(&world).next, None, "no step yet");
    world.step();
    let w = walker(&world);
    assert!(
        w.next.is_some() || w.at != at,
        "the step moves from the next tick"
    );
}

// ECO-OUT T24 (ECO-OUT-3.8a (2)): a walk back of a single step to the flag
// is ordered only once a check admits the ware. The check as the walk back
// starts raises the flag's bar; another ware bidding next with the same
// score is admitted and resets it, so the woodcutter's check at the end of
// his lattice walk raises it again: he stands on his field, and the next
// tick's check admits him and orders the step.
// spec: ECO-OUT-3.8a (ECO-OUT T24), TRN-CARRY-4.4
#[test]
fn t24_a_single_step_back_waits_for_the_admission() {
    let (mut world, id) = hut_with(&[(12, 9)]);
    let flag = world.terrain.neighbour((10, 10), 1).unwrap();
    while !matches!(work_of(&world, id), Work::Return { .. }) {
        world.step();
        assert!(world.tick() < 2000, "walks back");
    }
    assert_eq!(world.flag_bar.get(&flag), Some(&0), "the take's check");
    let good = world.settlers[id].as_ref().unwrap().carrying.unwrap();
    assert!(
        world.flag_bid(flag, good, None, &mut 0),
        "another ware first"
    );
    while matches!(work_of(&world, id), Work::Return { .. }) {
        world.step();
    }
    assert!(
        matches!(work_of(&world, id), Work::WaitBack { .. }),
        "he stands"
    );
    assert!(!world.settlers[id].as_ref().unwrap().walker.is_walking());
    world.step();
    assert!(matches!(work_of(&world, id), Work::Back { .. }));
    assert_eq!(world.settlers[id].as_ref().unwrap().walker.path.len(), 1);
}

// TRN-FREE-5.6, TRN-FREE T16, GFX-ANIM-3.8e (Run D): the walk home
// after a fine approach is a fine walk back along the lattice line from
// the stand to the point of the field F his walk out stopped on,
// ordered in the tick taking the log ends (E, no movement), first
// moving in E + 1 and arriving after the lattice walk's ticks; in that
// tick the free walk to the flag is ordered from F, first moving one
// tick later. He is never on a sample off that line.
#[test]
fn walk_home_goes_back_on_the_lattice() {
    for tree_at in [(13, 8), (12, 12), (8, 11), (12, 10)] {
        let (mut world, id) = hut_with(&[tree_at]);
        let (columns, rows) = world.terrain.sample_size();
        let mut out_field = None;
        let mut take_end = None;
        for _ in 0..1500 {
            let before = work_of(&world, id);
            world.step();
            let ran = world.tick() - 1;
            let settler = world.settlers[id].as_ref().unwrap();
            let work = work_of(&world, id);
            if matches!(work, Work::Approach { .. }) && out_field.is_none() {
                out_field = Some(settler.walker.at);
            }
            if let (
                Work::TakeWare {
                    second: true,
                    stand: Some(stand),
                },
                Work::Return { .. },
            ) = (before, work)
            {
                take_end = Some((ran, stand));
            }
            if take_end.is_some() {
                break;
            }
        }
        let (end, stand) = take_end.unwrap_or_else(|| panic!("tree {tree_at:?}: no return"));
        let f = out_field.expect("a walk out");
        let goal = grid::field_sample(f);
        let line = movement::fine_approach(&world.terrain, stand, goal, 0);
        assert_eq!(line.last(), Some(&goal), "tree {tree_at:?}");
        let ticks = movement::lattice_walk_ticks(
            &world.terrain,
            world.settings.settler_speed,
            stand,
            &line,
        );
        // E: still on the stand.
        let at_end = world
            .fine_position(world.settlers[id].as_ref().unwrap())
            .expect("on the lattice");
        assert_eq!(
            (at_end.at, at_end.progress_bits),
            (stand, 0),
            "tree {tree_at:?}"
        );
        let mut on_line: Vec<Point> = vec![stand];
        on_line.extend(&line);
        for k in 1..=ticks {
            world.step();
            assert_eq!(world.tick() - 1, end + u64::from(k));
            let settler = world.settlers[id].as_ref().unwrap();
            if k < ticks {
                let fine = world.fine_position(settler).expect("walking back");
                assert!(
                    on_line.contains(&fine.at),
                    "tree {tree_at:?} tick {k}: {fine:?}"
                );
                assert!(fine.next.is_some_and(|n| on_line.contains(&n)));
                assert!(matches!(work_of(&world, id), Work::Return { .. }));
            } else {
                // Arrived: on F, the free walk to the flag ordered.
                assert!(
                    matches!(work_of(&world, id), Work::Back { .. }),
                    "tree {tree_at:?}"
                );
                assert_eq!(settler.walker.at, f);
                assert_eq!(settler.walker.progress_bits, 0);
            }
        }
        world.step();
        let settler = world.settlers[id].as_ref().unwrap();
        assert!(
            settler.walker.progress_bits > 0 || settler.walker.at != f,
            "tree {tree_at:?}: the free walk moves one tick after it is ordered"
        );
        let _ = (columns, rows);
    }
}

// GFX-ANIM T16, T17 (Run D), TRN-FREE-5.6: level ground, even row 10,
// the hut's flag west along the row on (6, 10). x is his place relative
// to field (9, 10)'s point in world units (1.5 a sample step), from his
// fine position while on the lattice, else from the free walk.
#[test]
fn t16_t17_walk_home_table() {
    // T16: tree at (40, 40), he stands at (39, 40).
    let x16 = walk_home_x((40, 40));
    let want16: [(u64, f32); 13] = [
        (25, 4.5),
        (63, 4.5),
        (64, 4.365),
        (74, 3.015),
        (75, 2.88),
        (85, 1.53),
        (96, 0.045),
        (97, 0.0),
        (98, -0.27),
        (119, -5.94),
        (120, -6.21),
        (141, -11.88),
        (142, -12.15),
    ];
    for (k, want) in want16 {
        let got = x16[&k];
        assert!((got - want).abs() < 2e-3, "T16 W + {k}: {got}, want {want}");
    }
    // No tick moves him by more than one step's progress.
    for k in 25..142 {
        assert!((x16[&(k + 1)] - x16[&k]).abs() <= 0.27 + 1e-3, "W + {k}");
    }
    // T17: tree one sample east (41, 40), he stands on (40, 40), field
    // (10, 10)'s own point, yet walks back to (9, 10): 4 steps, 45
    // ticks, on (9, 10) in W + 108, the free walk from W + 109.
    let x17 = walk_home_x((41, 40));
    assert!((x17[&63] - 6.0).abs() < 1e-3);
    assert!(
        x17[&107] > 0.0 && x17[&108].abs() < 1e-3,
        "{} {}",
        x17[&107],
        x17[&108]
    );
    assert!(x17[&109] < 0.0);
}

/// GFX-ANIM T16, T17: the woodcutter's x relative to field (9, 10)'s
/// point, in world units, at the end of each tick W + k from his work's
/// start W, for a tree on field (10, 10) at sample `position`.
fn walk_home_x(position: Point) -> std::collections::BTreeMap<u64, f32> {
    let mut world = open_world(30, 30);
    let flag = (6, 10);
    let hut = (0..6)
        .filter_map(|d| world.terrain.neighbour(flag, d))
        .find(|&h| world.terrain.neighbour(h, 1) == Some(flag))
        .expect("a hut field");
    finished(&mut world, HUT, hut);
    tree(&mut world, (10, 10), TREE);
    world.deposits.get_mut(&(10, 10)).unwrap().position = position;
    let id = worker(&mut world, hut, WOODCUTTER);
    let origin = grid::field_sample((9, 10)).0 as f32;
    let mut out = std::collections::BTreeMap::new();
    let mut w = None;
    for _ in 0..3000 {
        world.step();
        let ran = world.tick() - 1;
        let settler = world.settlers[id].as_ref().unwrap();
        let work = work_of(&world, id);
        if w.is_none() && matches!(work, Work::AtSpot { .. }) {
            assert_eq!(settler.walker.at, (9, 10), "the walk out stops on (9, 10)");
            w = Some(ran);
        }
        let Some(w) = w else { continue };
        let column = |at: Point, next: Option<Point>, progress: u32, sample: fn(Point) -> Point| {
            let a = sample(at).0 as f32;
            let b = next.map_or(a, |n| sample(n).0 as f32);
            a + (b - a) * progress as f32 / movement::ONE as f32
        };
        let c = match world.fine_position(settler) {
            Some(f) => column(f.at, f.next, f.progress_millionths(), |p| p),
            None => {
                let walker = &settler.walker;
                column(
                    walker.at,
                    walker.next,
                    walker.progress_millionths(),
                    grid::field_sample,
                )
            }
        };
        out.insert(ran - w, (c - origin) * 1.5);
        if ran - w >= 142 {
            break;
        }
    }
    out
}

// ECO-OUT-6.4, T6: the felled tree is not ripe and disappears 18 ticks
// after the harvest, freeing its field.
#[test]
fn felled_tree_is_removed() {
    let mut world = open_world(20, 20);
    finished(&mut world, HUT, (10, 10));
    tree(&mut world, (12, 10), TREE);
    world.harvest((10, 10), (12, 10));
    let harvest = world.tick();
    assert!(!world.deposit((12, 10)).unwrap().ripe(harvest));
    while world.deposit((12, 10)).is_some() {
        world.step();
    }
    assert_eq!(world.tick() - 1, harvest + 18);
    assert!(!world.terrain.has((12, 10), state::DEPOSIT_BLOCKED));
}

// ECO-OUT T7: stone_06 gives 6 stones, shrinking to gone. Each smaller
// pile is a new deposit taking the next object number in the tick the
// work ends, the stone's own number coming later at the pick-up (8d46644).
// spec: ECO-OUT-1.3, ECO-OUT-3.6, ECO-OUT-6.4 (ECO-OUT T7)
#[test]
fn t7_stone_pile() {
    let mut world = open_world(30, 30);
    let pit = (10, 10);
    finished(&mut world, PIT, pit);
    let pile = (12, 10);
    tree(&mut world, pile, STONES[5]);
    let id = worker(&mut world, pit, STONECUTTER);
    let flag = world.terrain.neighbour(pit, 1).unwrap();
    let mut kinds = vec![STONES[5]];
    let mut harvests = 0;
    for _ in 0..6000 {
        let before = work_of(&world, id);
        let number = world.next_object();
        world.step();
        if matches!(work_of(&world, id), Work::TakeWare { second: false, .. })
            && !matches!(before, Work::TakeWare { second: false, .. })
        {
            harvests += 1;
        }
        if let Some(d) = world.deposit(pile) {
            if *kinds.last().unwrap() != d.kind {
                kinds.push(d.kind);
                assert_eq!(world.next_object(), number + 1, "the new pile's number");
            }
        }
        // No carrier comes: clear the flag now and then.
        if world.wares_at(flag).len() == 5 {
            let lying: Vec<usize> = world.wares_at(flag).to_vec();
            for w in lying {
                world.remove_ware(w);
            }
        }
    }
    assert_eq!(kinds, [406, 405, 404, 403, 402, 401]);
    assert!(world.deposit(pile).is_none());
    assert_eq!(harvests, 6, "stone_06 gives 6 stones");
    assert_eq!(world.search_target(pit, STONE, 8), None, "nothing left");
}

/// A nursery on (10, 10) with its forester, the ground of every field
/// of tree type `tree_type`.
fn nursery(tree_type: Option<u8>) -> (World, usize) {
    let mut world = open_world(30, 30);
    world.ground = vec![
        Ground {
            plantable: true,
            for_planting: true,
            tree_type,
        };
        world.terrain.len()
    ];
    finished(&mut world, NURSERY, (10, 10));
    let id = worker(&mut world, (10, 10), FORESTER);
    (world, id)
}

fn saplings(world: &World) -> Vec<(Point, u32)> {
    world.deposits().map(|(f, d)| (f, d.kind)).collect()
}

// ECO-OUT T8, T9, T10: kinds of the terrain's family; ripe 1001 ticks
// after planting.
// spec: ECO-OUT-4.5, ECO-OUT-6.1, ECO-OUT-6.2 (ECO-OUT T8, T9, T10)
#[test]
fn t8_t9_t10_planting_and_growth() {
    for (tree_type, family) in [
        (Some(1), FAMILY[1]),
        (Some(3), FAMILY[3]),
        (None, &[DEFAULT_TREE][..]),
    ] {
        let (mut world, id) = nursery(tree_type);
        let changes = trace(&mut world, id, 3000);
        // Planting ends where the walk home starts (TRN-FREE-5.6).
        let planted = when(&changes, |w| matches!(w, Work::Return { .. }));
        assert!(planted.len() >= 3);
        for (_, kind) in saplings(&world) {
            assert!(family.contains(&kind), "{tree_type:?} {kind}");
        }
    }
    let (mut world, id) = nursery(Some(1));
    let changes = trace(&mut world, id, 400);
    let planted = when(&changes, |w| matches!(w, Work::Return { .. }))[0];
    let (spot, _) = saplings(&world)[0];
    for t in planted..planted + 1002 {
        assert_eq!(
            world.deposit(spot).unwrap().ripe(t),
            t >= planted + 1001,
            "{t}"
        );
    }
    // T10 as the world counts it.
    while world.tick() <= planted + 1000 {
        world.step();
    }
    assert!(world.deposit(spot).unwrap().ripe(world.tick()));
}

// ECO-OUT-4.7: a forester planting is not felling a tree: the sapling
// under him never reports a felling (the view would drop it and play
// the tree-fall sound, SND-2.2).
#[test]
fn planting_is_not_felling() {
    let (mut world, id) = nursery(Some(1));
    let mut planting = 0;
    for _ in 0..600 {
        world.step();
        if let Work::AtSpot { spot } = work_of(&world, id) {
            planting += 1;
            assert!(world.deposit(spot).is_some(), "the sapling is there");
            assert_eq!(world.felling_began(spot), None);
        }
    }
    assert!(planting >= 55, "the forester planted ({planting} ticks)");
}

// ECO-OUT T16: the forester's hut reads -1 until its first planting ends,
// then 0.5, +0.25 at each later planting up to 1.0, and falls by 0.05 every
// 600 ticks like every producing building.
// spec: ECO-OUT-4.7b, ECO-OUT-3.11 (ECO-OUT T16)
#[test]
fn t16_forester_productivity() {
    let (mut world, id) = nursery(Some(1));
    let productivity = |world: &World| world.building_at((10, 10)).unwrap().productivity;
    let mut last = productivity(&world);
    assert_eq!(last, -100);
    let mut changes = Vec::new();
    for _ in 0..6000 {
        let planting = matches!(work_of(&world, id), Work::AtSpot { .. });
        world.step();
        let now = productivity(&world);
        if now != last {
            // Raised in the tick the planting action ends, or decayed on
            // the 600-tick counter.
            let ended = planting && matches!(work_of(&world, id), Work::Return { .. });
            changes.push((world.tick() - 1, now, ended));
            last = now;
        }
    }
    let raises: Vec<i32> = changes.iter().filter(|c| c.2).map(|c| c.1).collect();
    assert!(raises.len() >= 4, "{changes:?}");
    assert_eq!(raises[0], 50, "{changes:?}");
    for c in changes.iter().filter(|c| !c.2) {
        assert_eq!(c.0 % 600, 0, "a decay off the counter: {changes:?}");
    }
    let mut expected = -100;
    for &(_, now, ended) in &changes {
        expected = if ended {
            if expected < 0 {
                50
            } else {
                (expected + 25).min(100)
            }
        } else {
            (expected - 5).max(0)
        };
        assert_eq!(now, expected, "{changes:?}");
    }
}

// ECO-OUT T17: a forester who never finds a spot keeps -1.
// spec: ECO-OUT-4.7b (ECO-OUT T17)
#[test]
fn t17_forester_without_spots_keeps_no_productivity() {
    let (mut world, _) = nursery(Some(1));
    world.ground.iter_mut().for_each(|g| g.plantable = false);
    for _ in 0..3000 {
        world.step();
    }
    assert_eq!(world.building_at((10, 10)).unwrap().productivity, -100);
}

// ECO-OUT T11: nowhere to plant: no spot, searches every 51 ticks, no
// rest.
// spec: ECO-OUT-4.2, ECO-OUT-4.3 (ECO-OUT T11)
#[test]
fn t11_nowhere_to_plant() {
    let (mut world, id) = nursery(Some(1));
    world.ground.iter_mut().for_each(|g| g.plantable = false);
    let mut restarts = Vec::new();
    for _ in 0..300 {
        world.step();
        let work = work_of(&world, id);
        assert!(matches!(work, Work::WaitSpot { .. }), "{work:?}");
        if work == (Work::WaitSpot { countdown: 50 }) {
            restarts.push(world.tick() - 1);
        }
    }
    assert_eq!(restarts.len(), 6);
    assert!(restarts.windows(2).all(|p| p[1] - p[0] == 51));
    assert!(saplings(&world).is_empty());
}

// ECO-OUT T29: a forester switched off while he waits. With nowhere to
// plant, he is switched off during a countdown: the search that ends it
// still runs, finds nothing, and he stays inside (Begin), every tick, with no
// search and no draw. Switched on, the countdown starts in that tick and the
// next search is 51 ticks later. With a spot to plant, a switch-off during
// the countdown does not stop the search: he goes out and plants.
// spec: ECO-OUT-4.4, ECO-OUT-4.1, ECO-OUT-4.2 (ECO-OUT T29)
#[test]
fn t29_a_switched_off_forester_stops_searching() {
    let (mut world, id) = nursery(Some(1));
    world.ground.iter_mut().for_each(|g| g.plantable = false);
    // The first search ends a countdown; the next is under way.
    while work_of(&world, id) != (Work::WaitSpot { countdown: 50 }) {
        world.step();
    }
    world.step();
    assert_eq!(work_of(&world, id), Work::WaitSpot { countdown: 49 });
    world.toggle_production(P0, (10, 10)).unwrap();
    let mut searched = None;
    for n in 0..60 {
        world.step();
        if work_of(&world, id) == Work::Begin {
            searched = Some(n);
            break;
        }
    }
    assert_eq!(searched, Some(49), "the countdown's search still ran");
    let draws = world.rng().draws();
    for _ in 0..500 {
        world.step();
        assert_eq!(work_of(&world, id), Work::Begin, "off: inside, no search");
    }
    assert_eq!(world.rng().draws(), draws, "no draw while off");
    world.toggle_production(P0, (10, 10)).unwrap();
    world.step();
    assert_eq!(work_of(&world, id), Work::WaitSpot { countdown: 50 });
    let on = world.tick();
    while work_of(&world, id) != Work::Begin && world.tick() < on + 60 {
        world.step();
        if work_of(&world, id) == (Work::WaitSpot { countdown: 50 }) {
            break;
        }
    }
    assert_eq!(world.tick(), on + 51, "the next search 51 ticks later");

    // A spot to plant: switched off in the countdown, he still goes out.
    let (mut world, id) = nursery(Some(1));
    world.step();
    assert!(matches!(work_of(&world, id), Work::WaitSpot { .. }));
    world.toggle_production(P0, (10, 10)).unwrap();
    for _ in 0..60 {
        world.step();
    }
    assert!(
        matches!(work_of(&world, id), Work::Out { .. } | Work::ToSpot { .. }),
        "{:?}",
        work_of(&world, id)
    );
}

// ECO-OUT T12, T13: spots among the 11 first candidates, never next to
// a building.
// spec: ECO-OUT-4.3, ECO-OUT-4.4 (ECO-OUT T12, T13)
#[test]
fn t12_t13_spots() {
    let (world, _) = nursery(Some(1));
    let candidates = world.forester_candidates((10, 10), 6, false).unwrap();
    assert_eq!(candidates.len(), FORESTER_CANDIDATES);
    for &c in &candidates {
        for n in world.terrain.neighbours(c).into_iter().flatten() {
            assert!(!world.terrain.has(n, state::BLOCKED), "T13 {c:?}");
        }
        assert_ne!(Some(c), world.terrain.neighbour((10, 10), 1));
    }
    let mut chosen = std::collections::BTreeSet::new();
    for seed in 1..20 {
        let mut w = world.clone();
        w.rng = crate::Rng::new(seed);
        let nursery = w.building_types[&NURSERY].clone();
        let PlantSpot::Found(spot) = w.plant_spot((10, 10), &nursery) else {
            panic!("no spot");
        };
        assert!(candidates.contains(&spot), "{spot:?}");
        chosen.insert(spot);
    }
    assert!(chosen.len() > 3, "chosen at random: {chosen:?}");
}

// The forester's sapling grows into a tree the woodcutter fells: a
// log comes out of a planted tree.
// spec: ECO-OUT-4.7, ECO-OUT-6.2, ECO-OUT-3.6
#[test]
fn planted_trees_are_felled() {
    let (mut world, _) = nursery(Some(1));
    finished(&mut world, HUT, (14, 10));
    let cutter = worker(&mut world, (14, 10), WOODCUTTER);
    let flag = world.terrain.neighbour((14, 10), 1).unwrap();
    let mut logs = 0;
    for _ in 0..4000 {
        world.step();
        if work_of(&world, cutter) == (Work::PutDown { second: true })
            && world.settlers[cutter].as_ref().unwrap().timer == world.settings.pickup_ticks
        {
            logs += 1;
        }
    }
    assert!(logs >= 1, "logs {logs}");
    assert!(world
        .wares_at(flag)
        .iter()
        .all(|&w| world.ware(w).unwrap().good == WOOD));
    let _ = PlayerId(0);
}

// Same commands, same world, with production under way.
// spec: none (determinism of production: same commands, same world)
#[test]
fn production_is_deterministic() {
    let run = || {
        let (mut world, _) = nursery(Some(1));
        finished(&mut world, HUT, (14, 10));
        tree(&mut world, (16, 8), TREE);
        worker(&mut world, (14, 10), WOODCUTTER);
        for _ in 0..1500 {
            world.step();
        }
        world
    };
    assert_eq!(run(), run());
}

// ECO-OUT-3.11 (T26), 3.2: a new woodcutter without trees has the 0.5 of
// its placement, not below 0.5: no "No resources found." at its first
// failed checks; after the 600-tick fall below 0.5 the message comes, kept
// as one message.
// spec: ECO-OUT-3.11 (ECO-OUT T26), ECO-OUT-3.2
#[test]
fn no_resources_found() {
    let (mut world, _) = hut_with(&[]);
    for _ in 0..200 {
        world.step();
    }
    assert!(world.messages().is_empty());
    while world.tick() <= 600 {
        world.step();
    }
    for _ in 0..200 {
        world.step();
    }
    let texts: Vec<&str> = world.messages().iter().map(|m| m.text).collect();
    assert_eq!(texts, ["No resources found."]);
    let message = &world.messages()[0];
    assert!(
        message.until > message.tick + 600,
        "shown again, not repeated"
    );
}

// ECO-OUT-3.11 (T26): a type with a product has 0.5 from its site's
// placement on, which does not fall while it is under construction; +0.25
// each product, up to 1.0; -0.05 every 600 ticks once finished.
// spec: ECO-OUT-3.11 (ECO-OUT T26)
#[test]
fn productivity() {
    let mut world = open_world(30, 30);
    let at = (10, 10);
    assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
    let value = |w: &World| w.building_at(at).unwrap().productivity;
    assert_eq!(value(&world), 50, "the site's");
    while world.tick() <= 600 {
        world.step();
    }
    assert_eq!(value(&world), 50, "no fall under construction");
    world.sites.remove(&at);
    world.complete(at);
    let mut seen = Vec::new();
    for _ in 0..4 {
        world.produced(at);
        seen.push(value(&world));
    }
    assert_eq!(seen, [75, 100, 100, 100]);
    while !world.tick().is_multiple_of(600) {
        world.step();
    }
    world.step();
    assert_eq!(value(&world), 95);
}

// ECO-OUT-3.11 (AI-MIL T24): the statistics value is a float32, each step a
// float32 operation: from 1.0 eighteen falls of 0.05 leave 0.0999998, below
// 0.1, where the hundredths read 10.
// spec: ECO-OUT-3.11 (AI-MIL T24)
#[test]
fn t24_the_statistics_value_falls_in_float32_steps() {
    let mut world = open_world(30, 30);
    let at = (10, 10);
    finished(&mut world, SAWMILL, at);
    for _ in 0..2 {
        world.produced(at);
    }
    let value = |w: &World| f32::from_bits(w.building_at(at).unwrap().productivity_bits);
    assert_eq!(value(&world), 1.0);
    let mut falls = 0;
    while falls < 18 {
        if world.tick().is_multiple_of(600) && world.tick() > 0 {
            falls += 1;
        }
        world.step();
    }
    assert_eq!(world.building_at(at).unwrap().productivity, 10);
    assert!(value(&world) < 0.1, "{}", value(&world));
}

// TRN-FREE T7, T8: building to its flag and neighbour to neighbour are
// one step without a search; stop short 1 to a neighbour does not move.
// spec: TRN-FREE-2.2, TRN-FREE-4.2 (TRN-FREE T7, T8)
#[test]
fn t7_t8_single_steps() {
    let mut world = open_world(30, 30);
    let at = (10, 10);
    finished(&mut world, HUT, at);
    let flag = world.terrain.neighbour(at, 1).unwrap();
    assert_eq!(world.free_walk(at, flag, 0), Some(vec![flag]));
    assert_eq!(world.free_walk(flag, at, 0), Some(vec![at]));
    let ticks = movement::walk_ticks(&world.terrain, 45_000, at, &[flag]);
    assert_eq!(ticks, 23);
    assert_eq!(world.free_walk((5, 5), (6, 5), 0), Some(vec![(6, 5)]));
    assert_eq!(world.free_walk((5, 5), (6, 5), 1), Some(vec![]));
    let walk = world.free_walk((2, 20), (8, 20), 1).unwrap();
    assert_eq!(walk.len(), 5);
}

// TRN-FREE-5.4, ECO-OUT-4.5a: every sapling draws its lattice direction once
// more; a kind with the random offset stands one lattice step from its
// field's sample, one without on the sample.
// spec: TRN-FREE-5.4, ECO-OUT-4.5a
#[test]
fn saplings_stand_one_lattice_step_off() {
    for offset in [false, true] {
        let (mut world, id) = nursery(Some(1));
        for k in world.deposit_kinds.values_mut() {
            k.random_offset = offset;
        }
        world.record_draws();
        trace(&mut world, id, 400);
        let (spot, _) = saplings(&world)[0];
        let sample = grid::field_sample(spot);
        let position = world.deposit(spot).unwrap().position;
        if offset {
            assert_eq!(
                grid::distance(sample, position),
                1,
                "{sample:?} {position:?}"
            );
        } else {
            assert_eq!(position, sample);
        }
        // The spot draw, the kind draw and the offset draw, from one place.
        let record = world.take_draw_record();
        let planting = record
            .iter()
            // Windows writes the path with backslashes.
            .filter(|p| p.replace('\\', "/").contains("production/mod.rs"))
            .count();
        assert!(planting >= 3, "{record:?}");
    }
}

// SIM-START-4.4 f' (Q-263 d), T12: a produced ware takes its number in the
// first tick of the worker's pick-up action after the work (the plank as
// the sawyer takes it in hand), not when the work ends: none in the four
// ticks of the put-down before it.
// spec: SIM-START-4.4 (T12)
#[test]
fn a_product_takes_its_number_with_the_pick_up() {
    let mut world = open_world(30, 30);
    let (id, _) = sawmill_with_logs(&mut world, 6);
    while work_of(&world, id) != Work::Working {
        world.step();
    }
    let mut numbers = Vec::new();
    loop {
        let before = world.next_object();
        world.step();
        let work = work_of(&world, id);
        numbers.push((work, world.next_object() - before));
        if work == (Work::Take { second: true }) {
            break;
        }
    }
    let n = numbers.len();
    assert_eq!(numbers[n - 1], (Work::Take { second: true }, 1));
    assert!(
        numbers[..n - 1].iter().all(|&(_, taken)| taken == 0),
        "{numbers:?}"
    );
    assert!(numbers.contains(&(Work::Take { second: false }, 0)));
}

// ECO-OUT-4.3 with the grain flag, for the farm's site value (AI-CON-6.3):
// a free field passes; a flag, a tree's DepositBlocked or a Street on it
// fails; its neighbours do not matter.
// spec: ECO-OUT-4.3, AI-CON-6.3
#[test]
fn the_grain_planting_test_of_a_field() {
    let mut terrain = flat(20, 20);
    for (f, bits) in [((5, 5), state::DEPOSIT_BLOCKED), ((7, 5), state::STREET)] {
        let i = terrain.index(f);
        terrain.states[i] |= bits;
    }
    let neighbour = terrain.index((9, 6));
    terrain.states[neighbour] |= state::BLOCKED;
    let mut world = world(terrain);
    assert_eq!(flag(&mut world, (12, 12)), Ok(()));
    assert!(world.grain_plantable((10, 10)));
    assert!(!world.grain_plantable((5, 5)));
    assert!(!world.grain_plantable((7, 5)));
    assert!(!world.grain_plantable((12, 12)));
    assert!(
        world.grain_plantable((9, 5)),
        "a Blocked neighbour does not matter"
    );
}

// docs/snapshot.md, UI-GAME-8.5e, 8.5a: a game of ours saved in format 65 (the
// save fixture's world, after its header) had goods without their production
// counter, so it counted nothing for the rest of the game. Given this build's
// rules it counts from the load: a hut that produces after the load adds to
// its counter's current sample, the next cycle's sample starts at zero, and
// a coin made adds to series 5's total. The fixture's own huts make nothing
// in their economy, so here two finished huts produce by hand.
// spec: UI-GAME-8.5e, UI-GAME-8.5a
#[test]
fn a_world_of_format_65_counts_from_the_load() {
    use crate::snap::{Reader, MAGIC};
    use crate::statistics::{series, COIN_COUNTER, SAMPLE_TICKS};
    let bytes = include_bytes!("../../../../tests/saves/save1-world65.s2save");
    let start = bytes
        .windows(MAGIC.len())
        .position(|w| w == MAGIC)
        .expect("the world in the save");
    let mut r = Reader::new(&bytes[start..]);
    let mut world = World::read_snapshot(&mut r).expect("the world loads");
    assert_eq!(r.format(), 65);
    // Two finished huts with a product, and this build's rules counting
    // the first's product and making the second's a coin.
    let huts: Vec<(Point, PlayerId, u32)> = world
        .buildings()
        .filter(|b| !b.under_construction)
        .filter_map(|b| {
            let product = world.building_type(b.building_type)?.product?;
            Some((b.at, b.owner, product))
        })
        .collect();
    let (hut, owner, good) = huts[0];
    let (mint, mint_owner, coin) = *huts
        .iter()
        .find(|&&(_, _, g)| g != good)
        .expect("a hut of another product");
    let before = |world: &World| world.statistics(owner).unwrap().clone();
    // Without this build's rules: nothing counts, as before the fix.
    let mut unchanged = world.clone();
    unchanged.produced(hut);
    assert_eq!(before(&unchanged).production, before(&world).production);
    let mut rules = Rules::empty();
    rules.goods.insert(
        good,
        GoodRules {
            production_counter: Some(2),
            ..GoodRules::default()
        },
    );
    rules.goods.insert(
        coin,
        GoodRules {
            production_counter: Some(COIN_COUNTER),
            ..GoodRules::default()
        },
    );
    world.take_later_rules(r.format(), &rules);
    let cycle = |world: &World| world.statistics(owner).unwrap().production[2].len();
    let loaded = cycle(&world);
    assert!(loaded > 0, "the samples of the cycles before the load");
    world.produced(hut);
    world.produced(hut);
    world.produced(mint);
    assert_eq!(before(&world).production[2].back(), Some(&2));
    let coins = world.statistics(mint_owner).unwrap().coins;
    assert_eq!(coins, 1);
    // Played on to the next cycles: the count stays in its sample, the
    // next starts at zero and counts, the coins' total is sampled.
    // The next roll-over is in the update of tick `next - 1`.
    let next = world.tick().div_ceil(SAMPLE_TICKS) * SAMPLE_TICKS + 1;
    while world.tick() < next {
        world.step();
    }
    assert_eq!(cycle(&world), loaded + 1);
    world.produced(hut);
    while world.tick() < next + SAMPLE_TICKS {
        world.step();
    }
    let s = world.statistics(owner).unwrap();
    let last: Vec<u32> = s.production[2].iter().rev().take(3).copied().collect();
    assert_eq!(last, [0, 1, 2]);
    let m = world.statistics(mint_owner).unwrap();
    assert_eq!(m.series[series::COINS].back(), Some(&1));
    // A world of this build's format is left as it is.
    let mut current = world.clone();
    current.take_later_rules(crate::snap::VERSION, &Rules::empty());
    assert!(current == world);
}

/// Hook H6 (`docs/extending.md`, "New building types"): a building type of
/// a project's, with a recipe of its own (`Rules::recipes`), orders and
/// consumes that recipe, not its product's cost list, which the original's
/// producer of the good keeps; the original's game has no recipes, and its
/// state's `Debug` form (the state hash) has no trace of them.
// spec: none (hook H6: a project's recipe per building type; the cycle is ECO-WORK-3's, the orders ECO-PROD-6.1's)
#[test]
fn a_project_building_type_works_by_its_own_recipe() {
    const COAL: u32 = 7;
    const BURNER: u32 = 206;
    let plain = open_world(30, 30);
    assert!(!format!("{plain:?}").contains("recipes"));
    assert_eq!(plain.recipe(SAWMILL), None);
    let mut rules = rules();
    // Coal's own cost list (the original's producer, untouched): stone.
    rules.goods.insert(
        COAL,
        GoodRules {
            costs: vec![(STONE, 1)],
            ..GoodRules::default()
        },
    );
    let mut burner = BuildingType::new(BURNER, BuildingSize::Small);
    burner.worker = Some(CARPENTER);
    burner.product = Some(COAL);
    burner.production_ticks = 333;
    burner.cycle = WorkCycle::Workshop;
    burner.stock_limit = Some(6);
    rules.building_types.push(burner);
    rules.recipes.insert(BURNER, vec![(WOOD, 2)]);
    let mut world = World::with_map(5, flat(30, 30), rules, P0);
    assert!(format!("{world:?}").contains("recipes"));
    assert_eq!(world.recipe(BURNER), Some(&[(WOOD, 2)][..]));
    let at = (10, 10);
    finished(&mut world, BURNER, at);
    let id = worker(&mut world, at, CARPENTER);
    // ECO-PROD-6.1: up to the stock limit of each good of its recipe.
    assert_eq!(world.open_orders(at, WOOD), 6);
    assert_eq!(world.open_orders(at, STONE), 0);
    world.orders.clear();
    world.inputs.insert(at, vec![(WOOD, 6)]);
    let changes = trace(&mut world, id, 1000);
    let made = when(&changes, |w| *w == Work::Take { second: false }).len() as u32;
    assert_eq!(made, 2);
    assert_eq!(world.inputs(at), &[(WOOD, 2)]);
    // The coal it made lies at its flag.
    assert!(world.wares().any(|(_, w)| w.good == COAL));
}

/// Hook W20 (`docs/extending.md`, "Recipes a player chooses"): a type with
/// two recipes (`Rules::recipe_sets`) works its first until the player
/// chooses another (`Command::ChooseRecipe`): it orders and consumes the
/// chosen recipe's inputs and makes its product; the choice withdraws the
/// open orders of goods the new recipe does not take, is refused for an
/// index the set does not have, and is kept in the snapshot. The original's
/// game has neither sets nor choices, and its state's `Debug` form (the
/// state hash) has no trace of them.
// spec: none (hook W20: a recipe chosen per building; the cycle is ECO-WORK-3's, the orders ECO-PROD-6.1's)
#[test]
fn a_type_with_a_recipe_set_works_the_chosen_recipe() {
    use crate::rules::Recipe;
    use crate::{Command, Rejection};
    const COAL: u32 = 7;
    const POST: u32 = 207;
    let plain = open_world(30, 30);
    let plain_debug = format!("{plain:?}");
    assert!(!plain_debug.contains("recipe_sets") && !plain_debug.contains("chosen_recipes"));
    assert_eq!(plain.recipe_set(SAWMILL), None);
    let mut rules = rules();
    let mut post = BuildingType::new(POST, BuildingSize::Small);
    post.worker = Some(CARPENTER);
    post.product = Some(COAL);
    post.production_ticks = 333;
    post.cycle = WorkCycle::Workshop;
    post.stock_limit = Some(6);
    rules.building_types.push(post);
    let set = vec![
        Recipe {
            product: COAL,
            inputs: vec![(WOOD, 2)],
        },
        Recipe {
            product: PLANK,
            inputs: vec![(STONE, 1)],
        },
    ];
    rules.recipe_sets.insert(POST, set.clone());
    let mut world = World::with_map(5, flat(30, 30), rules, P0);
    assert_eq!(world.recipe_set(POST), Some(&set[..]));
    let at = (10, 10);
    finished(&mut world, POST, at);
    assert_eq!(world.chosen_recipe(at), Some(0));
    let id = worker(&mut world, at, CARPENTER);
    // The first recipe: wood ordered up to the stock limit, nothing else.
    assert_eq!(world.open_orders(at, WOOD), 6);
    assert_eq!(world.open_orders(at, STONE), 0);
    world.inputs.insert(at, vec![(WOOD, 2)]);
    let changes = trace(&mut world, id, 400);
    assert_eq!(
        when(&changes, |w| *w == Work::Take { second: false }).len(),
        1
    );
    assert!(world.wares().any(|(_, w)| w.good == COAL));
    // A recipe the set does not have is refused; another player's too.
    let choose = |index| Command::ChooseRecipe {
        player: P0,
        at,
        index,
    };
    assert_eq!(apply(&mut world, choose(2)), Err(Rejection::NotARecipe));
    let other = Command::ChooseRecipe {
        player: PlayerId(1),
        at,
        index: 1,
    };
    assert!(apply(&mut world, other).is_err());
    // The second: the wood orders are withdrawn, stone is ordered.
    assert_eq!(apply(&mut world, choose(1)), Ok(()));
    assert_eq!(world.chosen_recipe(at), Some(1));
    assert_eq!(world.open_orders(at, WOOD), 0);
    assert_eq!(world.open_orders(at, STONE), 6);
    assert!(format!("{world:?}").contains("chosen_recipes"));
    // The snapshot keeps the choice.
    let copy = World::from_snapshot(&world.snapshot()).unwrap();
    assert_eq!(copy.chosen_recipe(at), Some(1));
    assert!(copy == world);
    // It makes planks from stone now.
    world.inputs.insert(at, vec![(WOOD, 0), (STONE, 1)]);
    let changes = trace(&mut world, id, 800);
    assert!(!when(&changes, |w| *w == Work::Take { second: false }).is_empty());
    assert!(world.wares().any(|(_, w)| w.good == PLANK));
    // Back to the first: its own wood orders again.
    assert_eq!(apply(&mut world, choose(0)), Ok(()));
    assert_eq!(world.chosen_recipe(at), Some(0));
    assert_eq!(world.open_orders(at, STONE), 0);
    assert_eq!(world.open_orders(at, WOOD), 6);
}
