//! The original's autosave (`S2_SAVES_DIR`, with the install's rules from
//! `S2_GAME_DIR`; skipped when either is unset) continued with its computer
//! players: they play on from the restored world with their own records
//! (FMT-SAV-3.10a to 3.10g). What it prints is the measure of how they go on:
//! buildings planned in 3000 ticks, and why orders end.

use s2_ai::{AiConfig, AiSystem};
use s2_assets::{sav, GameDir};

// spec: FMT-SAV-3.10, FMT-SAV-3.10a, FMT-SAV-3.10d, FMT-SAV-3.10e, FMT-SAV-3.6, FMT-SAV-3.8, FMT-SAV-3.11, FMT-SAV-3.5a
#[test]
fn the_autosave_continues_with_its_computer_players() {
    let (Ok(dir), Some(saves)) = (GameDir::from_env(), std::env::var_os("S2_SAVES_DIR")) else {
        eprintln!("skipped: S2_GAME_DIR or S2_SAVES_DIR is not set");
        return;
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let path = std::path::Path::new(&saves).join("Autosave.sav");
    let bytes = std::fs::read(&path).expect("the autosave");
    let save = sav::load("Autosave.sav", &bytes).expect("it decodes");
    let (mut world, restored) = s2_rules::restore_game(&save, &rules).expect("restored");
    // What the save holds against what is taken over (for the report).
    let lists = [
        "workers",
        "constructors",
        "carriers",
        "bulldozers",
        "soldiers",
        "specialists",
    ];
    for (name, list) in lists.iter().zip(&save.settlers) {
        eprintln!("saved {name}: {}", list.len());
    }
    eprintln!(
        "saved wares {}, ware orders {}, building orders {}, ships {}, buildings {}, flags {}, streets {}",
        save.packages.len(),
        save.package_needs,
        save.building_needs,
        save.ships,
        save.buildings.len(),
        save.flags.len(),
        save.streets.len()
    );
    eprintln!("restored {restored:?}");
    eprintln!(
        "saved random {:08x?}, wares at flags {}, carried {}",
        save.random.map(s2_rules::random_words),
        s2_rules::saved_wares(&save).len(),
        save.settlers[2]
            .iter()
            .filter(|s| s.carried.is_some())
            .count()
    );
    // FMT-SAV-3.6, 3.9: every ware lying at a flag is put down there again.
    assert_eq!(restored.wares, s2_rules::saved_wares(&save).len());
    // FMT-SAV-3.6 (Q-1022, Q-1023): the packages in carriers' hands are in
    // their carriers' hands, each keeps its destination and order.
    let lying = save.packages.iter().filter(|p| p.lying).count();
    let in_hands = save.packages.len() - lying;
    let with_destination = save.packages.iter().filter(|p| p.has_target).count();
    let served = save
        .building_orders
        .iter()
        .filter(|o| o.package != sav::NO_ID)
        .count();
    eprintln!(
        "packages {}: lying {lying} -> at flags {}, in hands {in_hands} -> in hands {}, \
         skipped {}; with a destination {with_destination} -> {}; orders {} of which served \
         {served} -> served {}; ships being built {} -> {}; statistics cycle {:?} at tick {}, totals {:?}",
        save.packages.len(),
        restored.wares,
        restored.wares_carried,
        restored.wares_skipped,
        restored.wares_with_destination,
        save.building_orders.len(),
        restored.orders_served,
        save.saved_ships.len(),
        restored.ships,
        save.statistics_cycle,
        world.tick(),
        save.statistic_totals
    );
    for ship in &save.saved_ships {
        eprintln!("saved ship {ship:?}");
    }
    // FMT-SAV-3.5a (T10): the packages in named workers' hands are their
    // products again (the Fight autosave's two, workers 15264 and 18675).
    let products = s2_rules::saved_products(&save);
    eprintln!(
        "products in workers' hands {} -> {}; settlers going home {} -> {}",
        products.len(),
        restored.products,
        s2_rules::saved_homeward(&save).len(),
        restored.homeward
    );
    assert_eq!(restored.products, products.len());
    // FMT-SAV-3.5a (e): the 13 constructors the sites name are at them.
    eprintln!(
        "constructors naming a site {} -> at their sites {}",
        s2_rules::saved_builders(&save).len(),
        restored.builders
    );
    assert_eq!(restored.builders, s2_rules::saved_builders(&save).len());
    // (e, Q-1033 answered 2026-11-08 c): 8 are in a running unit (step 10)
    // with (c, r): c positions ordered, 1 .. 4, r ticks of the step left;
    // three at c = 4 (position 3) in mid-step. All go on with their units.
    // (b): the workers on the land coming back (36, 37) walk to their flag.
    let units = s2_rules::saved_builder_units(&save);
    eprintln!(
        "constructors in a running unit {units:?}; workers coming back {} -> walking {}",
        s2_rules::saved_workers_back(&save).len(),
        restored.workers_back
    );
    assert_eq!(units.len(), 8);
    assert!(units.iter().all(|&(_, p, _)| (1..=4).contains(&p)));
    assert_eq!(
        units.iter().filter(|&&(_, p, d)| p == 4 && d < 55).count(),
        3
    );
    assert_eq!(restored.builder_units, 8);
    assert_eq!(
        restored.workers_back,
        s2_rules::saved_workers_back(&save).len()
    );
    assert_eq!(restored.workers_back, 3);
    // (b, 2026-11-09, Q-257): the five indoor producers in state 33 go on
    // with their work action, 323, 312, 291, 112 and 103 ticks left (the
    // spec's samples row, corrected 2026-11-09 b).
    let mut working: Vec<u32> = s2_rules::saved_workers_working(&save)
        .iter()
        .map(|&(_, left)| left)
        .collect();
    working.sort_unstable();
    eprintln!(
        "producing inside {working:?} -> {}",
        restored.workers_working
    );
    assert_eq!(working, [103, 112, 291, 312, 323]);
    assert_eq!(restored.workers_working, 5);
    // (b, 2026-11-09 b, Q-257): out on the land in 33 / 34, three are at
    // their spot in their work action (a forester planting, 2 ticks left; a
    // woodcutter cutting, 52; a shipwright at his ship, 43) and two on their
    // fine approach (a forester and a woodcutter); all go on.
    let outdoors = s2_rules::saved_workers_outdoors(&save);
    let mut left: Vec<u32> = outdoors.iter().filter_map(|o| o.left).collect();
    left.sort_unstable();
    eprintln!(
        "out on the land {outdoors:?} -> at their spot {}, approaching {}",
        restored.workers_at_spot, restored.workers_approaching
    );
    assert_eq!(left, [2, 43, 52]);
    assert_eq!(outdoors.iter().filter(|o| o.walk.is_some()).count(), 2);
    assert_eq!(
        (restored.workers_at_spot, restored.workers_approaching),
        (3, 2)
    );
    // Each stands where the save has him: the forester one step short of
    // his sapling at (207, 233), the woodcutter of his tree at (199, 79),
    // the shipwright on his ship's coast field; the two approaching at the
    // saved point, stepping to the saved goal at the saved progress.
    let worker_of = |at: (u32, u32)| {
        world
            .settlers()
            .find_map(|(_, s)| match s.job {
                s2_sim::SettlerJob::Worker { target, work } if target.at == at => {
                    Some((work, s.timer, s.walker.at, world.fine_position(s)))
                }
                _ => None,
            })
            .expect("the building's worker")
    };
    let facing = |at: (u32, u32), to: (u32, u32)| Some(s2_sim::movement::direction_toward(at, to));
    for (building, stands, faces, timer) in [
        ((53, 62), (207, 234), (207, 233), 2),
        ((54, 16), (200, 78), (199, 79), 52),
    ] {
        let (work, t, _, fine) = worker_of(building);
        let fine = fine.expect("on the lattice");
        assert!(matches!(work, s2_sim::Work::AtSpot { .. }), "{work:?}");
        assert_eq!(
            (fine.at, fine.next, fine.facing, t),
            (stands, None, facing(stands, faces), timer)
        );
    }
    let (work, t, field, _) = worker_of((59, 48));
    assert_eq!(
        (work, t, field),
        (s2_sim::Work::AtSpot { spot: (62, 52) }, 43, (62, 52))
    );
    for (building, at, next, progress) in [
        ((52, 66), (218, 260), (219, 260), 0.18f32),
        ((50, 66), (187, 257), (187, 258), 0.8),
    ] {
        let (work, _, _, fine) = worker_of(building);
        let fine = fine.expect("on the lattice");
        assert!(matches!(work, s2_sim::Work::Approach { .. }), "{work:?}");
        assert_eq!((fine.at, fine.next), (at, Some(next)));
        assert!((f32::from_bits(fine.progress_bits) - progress).abs() < 0.01);
    }
    // (b, 2026-11-09 c, T14): on the rough part of their walk out, a forester
    // (21064) stepping (40, 67) -> (39, 67) at 0.77 to stop on (38, 67), one
    // short of his sapling's (37, 67), and a woodcutter (15985) stepping
    // (53, 63) -> (54, 63) to stop on (56, 64), one short of his tree's (56,
    // 65): both go on with the walk (k = 1) and make their fine approach.
    let rough: Vec<_> = outdoors.iter().filter(|o| o.rough.is_some()).collect();
    eprintln!("walking out {rough:?} -> {}", restored.workers_walking_out);
    for (spot, from, to, progress, stop) in [
        ((37, 67), (40, 67), (39, 67), 0.77f32, (38, 67)),
        ((56, 65), (53, 63), (54, 63), -1.0, (56, 64)),
    ] {
        let o = rough
            .iter()
            .find(|o| o.spot == spot)
            .unwrap_or_else(|| panic!("the walk out to {spot:?}"));
        let r = o.rough.as_ref().expect("rough");
        assert_eq!(
            (o.field, r.path[r.index as usize], r.stop_short),
            (from, to, 1)
        );
        if progress >= 0.0 {
            assert!((r.progress - progress).abs() < 0.01, "{r:?}");
        }
        let mut world = world.clone();
        let worker = |w: &s2_sim::World| {
            w.settlers()
                .find_map(|(_, s)| match s.job {
                    s2_sim::SettlerJob::Worker { target, work } if target.at == o.at => {
                        Some((work, s.walker.at, s.walker.next))
                    }
                    _ => None,
                })
                .expect("the building's worker")
        };
        assert_eq!(
            worker(&world),
            (s2_sim::Work::ToSpot { spot }, from, Some(to))
        );
        let mut ticks = 0;
        while matches!(worker(&world).0, s2_sim::Work::ToSpot { .. }) {
            world.step();
            ticks += 1;
            assert!(ticks < 300, "{spot:?}: never there");
        }
        let (work, field, _) = worker(&world);
        assert_eq!((work, field), (s2_sim::Work::Approach { spot }, stop));
    }
    assert_eq!(restored.workers_walking_out, rough.len());
    // ECO-OUT-2.6: the woodcutters' trees are their huts' targets.
    assert_eq!(world.reserved_target((54, 16)), Some((50, 20)));
    assert_eq!(world.reserved_target((50, 66)), Some((46, 65)));
    // (e, 2026-11-09 b): the four constructors approaching their sites
    // (step 13) and those walking to a construction position (step 10, r =
    // 55) go on with their fine walks.
    let walks = s2_rules::saved_builder_walks(&save);
    eprintln!(
        "constructors walking {} -> going on {}",
        walks.len(),
        restored.builder_walks
    );
    assert_eq!(
        save.settlers[1]
            .iter()
            .filter(|s| s.site_work.is_some_and(|w| w[2] == 13))
            .count(),
        4
    );
    assert_eq!((walks.len(), restored.builder_walks), (8, 8));
    // T26 (Q-1037): the four approaches' legs and the walks' targets: on the
    // way back (2) to the flag field's sample, on the way out (1) to it +
    // (1, 3).
    let approaches: std::collections::BTreeMap<(u32, u32), (u32, (u32, u32))> = walks
        .iter()
        .filter(|(site, _, _)| [(22, 49), (21, 50), (56, 20), (21, 41)].contains(site))
        .map(|(site, walk, leg)| (*site, (*leg, walk.path[0])))
        .collect();
    assert_eq!(
        approaches,
        [
            ((21, 41), (1, (89, 163))),
            ((21, 50), (2, (86, 196))),
            ((22, 49), (2, (92, 192))),
            ((56, 20), (1, (227, 79))),
        ]
        .into_iter()
        .collect()
    );
    // FMT-SAV-3.5a (b): the named workers on their way walk on.
    let states: std::collections::BTreeMap<u32, usize> =
        save.settlers[0]
            .iter()
            .fold(Default::default(), |mut m, s| {
                *m.entry(s.state).or_default() += 1;
                m
            });
    eprintln!(
        "workers by state {states:?}; on their way {} -> walking {}",
        s2_rules::saved_workers_on_way(&save).len(),
        restored.workers_on_way
    );
    assert_eq!(
        restored.workers_on_way,
        s2_rules::saved_workers_on_way(&save).len()
    );
    assert_eq!(
        restored.wares + restored.wares_carried + restored.products + restored.wares_skipped,
        save.packages.len()
    );
    assert!(restored.wares_carried <= in_hands);
    // FMT-SAV-3.6: the building orders (`Transport BuildingNeedGood`) our
    // buildings file again, at once and after a tick.
    eprintln!(
        "building orders back: {} of {} at once",
        orders_back(&save, &world),
        save.building_need_goods.len()
    );
    let slots = s2_rules::computer_players(&save);
    eprintln!("computer players {slots:?}");
    let config = AiConfig::new(&rules, &rules.sim_rules().expect("sim rules"));
    let mut ai = AiSystem::new(config, &world);
    ai.activate_all(&mut world, &slots);
    let mine = |w: &s2_sim::World| -> usize {
        slots
            .iter()
            .map(|(p, _)| w.buildings().filter(|b| b.owner == *p).count())
            .sum()
    };
    for (p, _) in &slots {
        assert!(ai.player(*p).is_some(), "slot {} plays", p.0);
    }
    let before = mine(&world);
    // FMT-SAV-3.10a: the records as the file has them, each player's.
    let taken = ai.take_over(&world, &save);
    eprintln!(
        "taken over {taken:?}, game clock {} s",
        world.game_seconds()
    );
    assert_eq!(taken.players, slots.len());
    for c in &save.computer_players {
        let p = ai
            .player(s2_sim::PlayerId(c.slot as u8))
            .expect("its player");
        assert_eq!(p.plan.needs.len(), c.needs.len() + c.food_needs.len());
        assert_eq!(p.plan.prods.len(), c.productions.len());
        assert_eq!(p.plan.waiting.len(), c.waiting.len());
        assert_eq!(p.cells.len(), c.cells.len());
        assert_eq!(
            p.conquered,
            c.conquered.iter().map(|&l| l as i32).collect::<Vec<_>>()
        );
        // A building named by the save is one the world has: the castle.
        assert!(p.plan.prods.iter().all(|x| x.building.is_some()));
    }
    // ECO-FOOD-5.11b: the game's clock goes on from the save's: every saved
    // order was made before now.
    for c in &save.computer_players {
        let needs = c.orders.needs.expect("the needs order");
        assert!(needs.time <= world.game_seconds());
        assert!(world.game_seconds() - needs.time < 60.0);
    }
    let mut runner = s2_ai::run::Runner::new(world, ai, Default::default());
    runner.step();
    eprintln!(
        "building orders back after a tick: {} of {}",
        orders_back(&save, &runner.world),
        save.building_need_goods.len()
    );
    for _ in 1..3000 {
        runner.step();
    }
    // 2 on the Fight autosave of 2026-11-01: 172 of the 174 orders end for
    // want of material, which no record of the player changes (Q-1001).
    eprintln!(
        "buildings planned in 3000 ticks: {}",
        runner
            .stats
            .issued
            .get("PlanBuilding")
            .copied()
            .unwrap_or(0)
    );
    eprintln!(
        "their buildings and sites {before} -> {} in 3000 ticks; commands {:?}, refused {:?}",
        mine(&runner.world),
        runner.stats.issued,
        runner.stats.rejected
    );
}

/// The autosave played on with every soak invariant after every tick.
// spec: FMT-SAV-3.10, FMT-SAV-3.4b
#[test]
fn the_autosave_plays_on_with_every_invariant() {
    let (Ok(dir), Some(saves)) = (GameDir::from_env(), std::env::var_os("S2_SAVES_DIR")) else {
        eprintln!("skipped: S2_GAME_DIR or S2_SAVES_DIR is not set");
        return;
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let path = std::path::Path::new(&saves).join("Autosave.sav");
    let bytes = std::fs::read(&path).expect("the autosave");
    let save = sav::load("Autosave.sav", &bytes).expect("it decodes");
    let (mut world, _) = s2_rules::restore_game(&save, &rules).expect("restored");
    let slots = s2_rules::computer_players(&save);
    let config = AiConfig::new(&rules, &rules.sim_rules().expect("sim rules"));
    let mut ai = AiSystem::new(config, &world);
    ai.activate_all(&mut world, &slots);
    ai.take_over(&world, &save);
    let mut checker = s2_sim::invariants::Checker::new(&world);
    eprintln!("from tick {}", world.tick());
    let mut broken = Vec::new();
    // FMT-SAV-3.4b: a site's stock is what lies there, and what its
    // work has used comes on top: nothing is ordered twice, nothing is left
    // over at the end (the plank of site (51, 19), gone at tick 36670).
    // FMT-SAV-3.5a (e): a builder going on with his running unit has its
    // material already (its rest of the work is the first unit left).
    for (at, site) in world.sites() {
        let running = site.builder.is_some_and(|b| b.in_unit);
        let units = (site.work_total - site.work_done).div_ceil(site.unit_ticks)
            - u32::from(running && site.work_done < site.work_total);
        let lying: u32 = site.on_site.iter().map(|&(_, n)| n).sum();
        let ordered: u32 = site.shortfall().iter().map(|&(_, n)| n).sum();
        eprintln!(
            "site {at:?}: work {}/{}, lying {:?}, to order {:?}",
            site.work_done,
            site.work_total,
            site.on_site,
            site.shortfall()
        );
        assert_eq!(lying + ordered, units, "site {at:?}");
    }
    for _ in 0..10_000 {
        ai.step(&mut world);
        broken.extend(checker.check(&world));
    }
    assert!(broken.is_empty(), "{broken:#?}");
}

/// FMT-SAV-3.6: how many of the save's building orders (building, good) the
/// world has an order of its own for (open, or served by a ware on its way),
/// each order counted once.
fn orders_back(save: &sav::SaveGame, world: &s2_sim::World) -> usize {
    let fields: std::collections::BTreeMap<u64, (u32, u32)> = save
        .buildings
        .iter()
        .map(|b| (b.id, (b.field.x, b.field.y)))
        .collect();
    // The open orders and those served by a ware on its way (ECO-REQ-5.3f).
    let served = world.wares().filter_map(|(_, w)| w.order);
    let mut open: Vec<((u32, u32), u32)> = world
        .orders()
        .iter()
        .copied()
        .chain(served)
        .filter(|o| !o.target.storage)
        .map(|o| (o.target.at, o.good))
        .collect();
    let mut back = 0;
    for (building, good) in &save.building_need_goods {
        let Some(&at) = fields.get(building) else {
            continue;
        };
        if let Some(i) = open.iter().position(|&o| o == (at, *good)) {
            open.swap_remove(i);
            back += 1;
        }
    }
    back
}
