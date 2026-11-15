//! FMT-SAV-2, 3: the player's own saves (`S2_SAVES_DIR`) read and continued
//! as games with the rules of the install (`S2_GAME_DIR`). Skipped (passes)
//! when either is unset.

use s2_assets::{sav, GameDir};

fn load(name: &str) -> Option<(sav::SaveGame, s2_rules::RuleSet)> {
    let (Ok(dir), Some(saves)) = (GameDir::from_env(), std::env::var_os("S2_SAVES_DIR")) else {
        eprintln!("skipped: S2_GAME_DIR or S2_SAVES_DIR is not set");
        return None;
    };
    let rules = s2_rules::load(&dir).unwrap_or_else(|e| panic!("{e}")).0;
    let path = std::path::Path::new(&saves).join(name);
    let file = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Some((
        sav::load(name, &file).unwrap_or_else(|e| panic!("{e}")),
        rules,
    ))
}

/// How far a save gets, printed for the report.
fn report(name: &str, save: &sav::SaveGame) {
    eprintln!("{name}: read {:?}", save.sections);
    for problem in &save.problems {
        eprintln!("  {problem}");
    }
}

// FMT-SAV T5: the 2023 save reads to its last byte, and the game continues
// with its buildings, flags, roads, territory and exploration, the workers
// and the road carriers.
// spec: FMT-SAV-2.1, FMT-SAV-2.2, FMT-SAV-3.1 (T5), FMT-SAV-3.4a, FMT-SAV-3.9, FMT-SAV-3.6, FMT-SAV-3.8, FMT-SAV-3.5a (T11), FMT-SAV-3.8 (T12), TRN-FREE-6.4
#[test]
fn coastal_storm_2023_continues() {
    let name = "17-03-2023 16-40-11 Coastal Storm.sav";
    let Some((save, rules)) = load(name) else {
        return;
    };
    report(name, &save);
    assert!(save.complete(), "{:?}", save.problems);
    assert_eq!(save.buildings.len(), 345);
    let sizes: Vec<usize> = save.settlers.iter().map(Vec::len).collect();
    assert_eq!(sizes, [296, 22, 1539, 7, 72, 16]);
    assert_eq!(
        (save.packages.len(), save.package_needs, save.building_needs),
        (303, 74, 390)
    );
    assert_eq!(
        (save.ships, save.flags.len(), save.streets.len()),
        (18, 947, 984)
    );
    assert_eq!(save.routing[0], [0, 114, 27]);
    assert_eq!(save.padding, Some(77));
    let (mut world, restored) = s2_rules::restore_game(&save, &rules).unwrap();
    eprintln!("  restored {restored:?}");
    assert_eq!(restored.buildings + restored.sites, 345);
    assert_eq!(restored.buildings_skipped, 0);
    assert_eq!(world.roads().count(), 984 - restored.roads_skipped);
    // FMT-SAV-3.4a, 3.9: the 154 workers the buildings name are in them,
    // every street's carrier and its 336 donkeys stand on their roads.
    assert_eq!(
        (restored.workers, restored.carriers, restored.donkeys),
        (154, 984, 336)
    );
    assert_eq!(world.carriers().len(), 984 + 336);
    // FMT-SAV-3.6, 3.8: every package lying at a flag or in a street
    // carrier's hands comes back with its destination; the ships being
    // built too.
    eprintln!(
        "  packages {}: at flags {}, in hands {}, skipped {}, with a destination {}, \
         orders served {} of {}; ships {} of which being built {}",
        save.packages.len(),
        restored.wares,
        restored.wares_carried,
        restored.wares_skipped,
        restored.wares_with_destination,
        restored.orders_served,
        save.building_orders.len(),
        save.ships,
        restored.ships
    );
    assert_eq!(
        restored.wares + restored.wares_carried + restored.products + restored.wares_skipped,
        save.packages.len()
    );
    // FMT-SAV-3.5a (c, T11): the settlers no building, street or soldier list
    // names are going home: 142 workers, 218 carriers, 21 constructors, 7
    // bulldozers, 16 specialists, 44 soldiers; the 396 of player 1 (beaten,
    // no warehouse) among them.
    let homeward = s2_rules::saved_homeward(&save);
    let of_one = homeward.iter().filter(|h| h.player.0 == 1).count();
    eprintln!(
        "  going home {} ({of_one} of player 1), sent {}; products in workers' hands {}",
        homeward.len(),
        restored.homeward,
        restored.products
    );
    assert_eq!(homeward.len(), 142 + 218 + 21 + 7 + 16 + 44);
    assert_eq!(of_one, 396);
    // FMT-SAV-3.5a (c), TRN-FREE-6.4 step 4 (Q-1032 c answered): their
    // going-home record keeps the wander failure count: 14 for 3 of them,
    // 15 .. 22 for the others (the spec's "15 .. 22" leaves out the 3);
    // every other settler going home has 0.
    let mut counts = std::collections::BTreeMap::new();
    for h in homeward.iter().filter(|h| h.player.0 == 1) {
        *counts.entry(h.failures).or_insert(0) += 1;
    }
    assert_eq!(
        counts.into_iter().collect::<Vec<_>>(),
        [
            (14, 3),
            (15, 22),
            (16, 24),
            (17, 45),
            (18, 61),
            (19, 59),
            (20, 56),
            (21, 76),
            (22, 50)
        ]
    );
    assert!(homeward
        .iter()
        .filter(|h| h.player.0 != 1)
        .all(|h| h.failures == 0));
    // FMT-SAV-3.8 (T12): 18 ships of player 1: numbers 0 .. 16 finished (16 in
    // state 1 with counters 5 .. 72, number 3 in state 4 with its expedition
    // flag and 4 + 1 + 6 goods), number 17 being built; none has a harbour.
    let numbers: Vec<u32> = save.saved_ships.iter().map(|s| s.number).collect();
    assert_eq!(numbers, (0..18).collect::<Vec<u32>>());
    assert!(save
        .saved_ships
        .iter()
        .all(|s| s.owner == 1 && s.harbour.is_none()));
    assert_eq!(restored.finished_ships, 17);
    // FMT-SAV-3.5a (b): the named workers on their way walk on.
    let states: std::collections::BTreeMap<u32, usize> =
        save.settlers[0]
            .iter()
            .fold(Default::default(), |mut m, s| {
                *m.entry(s.state).or_default() += 1;
                m
            });
    eprintln!(
        "  workers by state {states:?}; on their way {} -> walking {}",
        s2_rules::saved_workers_on_way(&save).len(),
        restored.workers_on_way
    );
    assert!(restored.workers_on_way <= s2_rules::saved_workers_on_way(&save).len());
    // (b, 2026-11-08 b): the workers on the land coming back walk to their
    // flags; (e): its one constructor is not in a running unit.
    assert_eq!(
        (restored.workers_back, restored.builder_units),
        (s2_rules::saved_workers_back(&save).len(), 0)
    );
    assert_eq!(restored.workers_back, 15);
    // (b, e, 2026-11-09 b): the workers out at their spot or on their fine
    // approach, and the constructors on the fine lattice, go on.
    let outdoors = s2_rules::saved_workers_outdoors(&save);
    let walks = s2_rules::saved_builder_walks(&save);
    eprintln!(
        "  out on the land {} -> at their spot {}, approaching {}; constructors walking {} -> {}",
        outdoors.len(),
        restored.workers_at_spot,
        restored.workers_approaching,
        walks.len(),
        restored.builder_walks
    );
    // Of the 22: 8 at their spot (6 farmers, 2 woodcutters), 1 woodcutter
    // approaching, 2 fishermen at the coast and a donkey breeder producing
    // inside on his building's door field (ECO-WORK-3.8a); (2026-11-09 c)
    // 6 on a rough walk out (two farmers sowing, a fisherman to his coast
    // spot with k = 0, three others with k = 1), all going on; (2026-11-11,
    // E-sav-014) 4 farmers going out to harvest (harvest step 8): 100504
    // and 298689 on the door step onto their farm's field (leg 1; before,
    // read as producers walking in and seated), 107387 and 111474 on the
    // walk from the farm's field to its flag (leg 0), all going on to their
    // farm's reserved grain.
    let leg = |l: u32| outdoors.iter().filter(|o| o.leg == Some(l)).count();
    assert_eq!((leg(1), leg(0)), (2, 2));
    let at_door = restored.workers_working - s2_rules::saved_workers_working(&save).len();
    eprintln!("  producing on the door field {at_door}");
    for o in outdoors.iter().filter(|o| o.rough.is_some()) {
        eprintln!("  walking out {o:?}");
    }
    eprintln!("  walking out -> {}", restored.workers_walking_out);
    assert_eq!(
        (
            restored.workers_at_spot,
            restored.workers_approaching,
            at_door
        ),
        (10, 1, 1)
    );
    assert_eq!(restored.workers_walking_out, 6 + 4);
    assert_eq!(
        restored.workers_at_spot
            + restored.workers_approaching
            + at_door
            + restored.workers_walking_out,
        outdoors.len()
    );
    assert_eq!(restored.builder_walks, walks.len());
    // (b, 2026-11-11, E-sav-014): the farmers waiting inside in their harvest
    // step (7) go on with their wait from the saved countdown.
    // The 9 workers in 33 whose harvest state is at step 7 go on: the 5
    // farmers (110877, 203134, 211736, 216019, 256071), each farm with its
    // reserved grain, and (2026-11-12, E-sav-016, T15) the 4 woodcutters
    // (104042, 228464, 253582, 271583; countdowns 24, 28, 9, 35), no tree
    // reserved.
    let waiting = s2_rules::saved_workers_waiting(&save);
    for w in &waiting {
        eprintln!("  waiting {w:?}");
    }
    let mut farms: Vec<u32> = waiting
        .iter()
        .filter(|w| w.target.is_some())
        .filter_map(|w| w.harvest)
        .collect();
    farms.sort();
    assert_eq!(farms, [4, 27, 34, 35, 45]);
    assert_eq!((waiting.len(), restored.workers_waiting), (9, 9));
    let ships = world.ships();
    assert_eq!(ships.len(), 18);
    let states: Vec<_> = ships[..17].iter().map(|s| s.vessel.state).collect();
    use s2_sim::navy::ShipState;
    assert_eq!(
        states.iter().filter(|&&s| s == ShipState::Search).count(),
        16
    );
    assert_eq!(states[3], ShipState::Waiting);
    assert!(ships[3].vessel.expedition);
    let goods: u32 = ships[3].vessel.aboard.iter().map(|&(_, n)| n).sum();
    assert_eq!(goods, 4 + 1 + 6);
    let counters: Vec<u32> = ships[..17]
        .iter()
        .filter(|s| s.vessel.state == ShipState::Search)
        .map(|s| s.vessel.counter)
        .collect();
    assert_eq!(
        (counters.iter().min(), counters.iter().max()),
        (Some(&5), Some(&72))
    );
    assert!(ships[..17]
        .iter()
        .all(|s| s.vessel.launched && s.owner.0 == 1));
    // TRN-FREE-6.4 step 4 (2026-11-08 b): player 1's homeless settlers (no
    // warehouse in reach) do not die at once: they wander between their
    // home searches, alive 100 ticks after the load.
    let homeless = |world: &s2_sim::World| {
        world
            .settler_views()
            .iter()
            .filter(|v| v.player.0 == 1 && v.activity == s2_sim::view::Activity::Home)
            .count()
    };
    let at_load = homeless(&world);
    assert!(at_load >= 396, "{at_load}");
    // Each goes on with its saved count: its first wander step counts it.
    let counts: Vec<u8> = world
        .settlers()
        .filter(|(_, s)| s.player.0 == 1)
        .filter_map(|(_, s)| match s.job {
            s2_sim::SettlerJob::Leaving {
                step: s2_sim::LeavingStep::Wander { tries },
                ..
            } => Some(tries),
            _ => None,
        })
        .collect();
    assert_eq!(counts.len(), 396);
    assert!(counts.iter().all(|t| (14..=22).contains(t)), "{counts:?}");
    let tick = world.tick();
    let mut checker = s2_sim::invariants::Checker::new(&world);
    let mut broken = Vec::new();
    for k in 0..1000 {
        world.step();
        broken.extend(checker.check(&world));
        if k == 99 {
            assert_eq!(homeless(&world), at_load, "alive after 100 ticks");
        }
    }
    assert_eq!(world.tick(), tick + 1000);
    assert!(broken.is_empty(), "{broken:#?}");
}

/// FMT-SAV T5b (Q-299): a soldier's rank is its profession id and its health is on
/// the rank's own scale; the list's `u32` is the owner. The Coastal Storm save: 2
/// privates at 3; 70 generals, 68 at 7, 1 at 6, 1 at 5. A restored game has them.
// spec: FMT-SAV-3.5, FMT-SAV-3.5a, FMT-SAV-3.5c (T5b)
#[test]
fn coastal_storm_2023_soldiers_rank_and_health() {
    let name = "17-03-2023 16-40-11 Coastal Storm.sav";
    let Some((save, rules)) = load(name) else {
        return;
    };
    const PRIVATE: u32 = 0x15E8_6843;
    const GENERAL: u32 = 0x559D_DEA3;
    let mut counts = std::collections::BTreeMap::new();
    for s in &save.settlers[4] {
        let points = s.live_points.expect("a soldier's live points");
        *counts
            .entry((s.profession, points.health as u32))
            .or_insert(0) += 1;
    }
    assert_eq!(
        counts.into_iter().collect::<Vec<_>>(),
        [
            ((PRIVATE, 3), 2),
            ((GENERAL, 5), 1),
            ((GENERAL, 6), 1),
            ((GENERAL, 7), 68),
        ]
    );
    assert!(save.settlers[0].iter().all(|s| s.live_points.is_none()));
    let (world, restored) = s2_rules::restore_game(&save, &rules).unwrap();
    let garrisons: Vec<_> = world
        .buildings()
        .flat_map(|b| world.garrison(b.at))
        .collect();
    assert_eq!(garrisons.len(), restored.soldiers);
    assert!(garrisons.iter().any(|s| s.rank == 4 && s.health == 7));
    assert!(garrisons
        .iter()
        .all(|s| (1..=7).contains(&s.health) && s.rank <= 4));
}

// FMT-SAV-3.4a, 3.4b, 3.9: the settlers a building or a street names are
// records of the matching settler list: a building's workers in the
// workers list, its soldiers in the soldiers list, a street's two carrier
// refs (after its two end flags) in the carriers list.
// spec: FMT-SAV-3.4a, FMT-SAV-3.4b, FMT-SAV-3.5, FMT-SAV-3.9
#[test]
fn coastal_storm_2023_settler_links() {
    let name = "17-03-2023 16-40-11 Coastal Storm.sav";
    let Some((save, _)) = load(name) else {
        return;
    };
    let ids = |list: usize| -> std::collections::HashSet<u64> {
        save.settlers[list].iter().map(|s| s.id).collect()
    };
    let (workers, carriers, soldiers) = (ids(0), ids(2), ids(4));
    let named = |refs: &mut dyn Iterator<Item = u64>, list: &std::collections::HashSet<u64>| {
        let refs: Vec<u64> = refs.filter(|&r| r != sav::NO_ID).collect();
        let found = refs.iter().filter(|r| list.contains(r)).count();
        (refs.len(), found)
    };
    let building_workers = named(
        &mut save
            .buildings
            .iter()
            .flat_map(|b| b.workers.iter().copied()),
        &workers,
    );
    let building_soldiers = named(
        &mut save
            .buildings
            .iter()
            .flat_map(|b| b.soldiers.iter().copied()),
        &soldiers,
    );
    let street_carriers = named(&mut save.streets.iter().map(|s| s.carriers[0]), &carriers);
    let street_donkeys = named(&mut save.streets.iter().map(|s| s.carriers[1]), &carriers);
    eprintln!(
        "workers {building_workers:?}, soldiers {building_soldiers:?}, \
         carriers {street_carriers:?}, donkeys {street_donkeys:?}"
    );
    for (refs, found) in [building_workers, building_soldiers, street_carriers] {
        assert!(refs > 0);
        assert_eq!(refs, found);
    }
    assert_eq!(street_donkeys.0, street_donkeys.1);
}

// The autosave (rewritten by the game; the newest is the four-player
// Fight game with three Hard computer players): every section reads, the
// active computer players' bodies too (FMT-SAV-3.10a to 3.10g), and the
// game continues from it.
// spec: FMT-SAV-3.10, FMT-SAV-3.10a, FMT-SAV-3.10c, FMT-SAV-3.10d, FMT-SAV-3.10e, FMT-SAV-3.10f, FMT-SAV-3.10g
#[test]
fn autosave_continues_as_far_as_it_reads() {
    let name = "Autosave.sav";
    let Some((save, rules)) = load(name) else {
        return;
    };
    report(name, &save);
    assert!(save.sections.contains(&"Net System"), "{:?}", save.sections);
    assert!(save.problems.is_empty(), "{:?}", save.problems);
    let computers = s2_rules::computer_players(&save);
    eprintln!("  computer players {computers:?}");
    assert_eq!(computers.len(), save.computer_players.len());
    let (mut world, restored) = s2_rules::restore_game(&save, &rules).unwrap();
    eprintln!("  restored {restored:?}");
    assert_eq!(restored.buildings_skipped, 0);
    for _ in 0..200 {
        world.step();
    }
}

/// The two saves of the original with a fight in progress (FMT-SAV-3.7a,
/// E-sav-017), as the investigation named them (`S2_SAVES_DIR` pointing at
/// a folder with them, outside the repository).
const FIGHT_FIRST: &str = "fight-13-11-2026 08-44-54 Keyhole.sav";
const FIGHT_SECOND: &str = "fight-13-11-2026 08-45-44 Keyhole.sav";

/// A fight save, or `None` (skipped) when the variables are unset or the
/// folder does not hold it (the player's own saves folder does not).
fn load_fight(name: &str) -> Option<(sav::SaveGame, s2_rules::RuleSet)> {
    let saves = std::env::var_os("S2_SAVES_DIR")?;
    if !std::path::Path::new(&saves).join(name).exists() {
        eprintln!("skipped: {name} is not in S2_SAVES_DIR");
        return None;
    }
    load(name)
}

/// Each record of a fight list: (attacker, opponent, target, counter, hit
/// pending, waiting field).
type FightRow = (u64, u64, u64, u32, bool, Option<(u32, u32)>);

fn fight_rows(list: &[sav::SavedFighters]) -> Vec<FightRow> {
    list.iter()
        .map(|f| {
            (
                f.attacker.unwrap_or(sav::NO_ID),
                f.opponent.unwrap_or(sav::NO_ID),
                f.target.unwrap_or(sav::NO_ID),
                f.counter,
                f.hit_pending,
                f.waiting.map(|w| (w.x, w.y)),
            )
        })
        .collect()
}

// FMT-SAV-3.7a (T17, reading): both fight saves read to the last byte, with
// fight lists of 2 / 2 / 1 and 1 / 2 / 0 records; the second holds attacker
// 16592 alone and unpaired in list 1 (waiting field (33, 20), round counter
// 21) and the duels 16600 against 18545 (counter 11, hit pending) and 17868
// against 18560 (counter 14) in list 2.
// spec: FMT-SAV-3.7, FMT-SAV-3.7a (T17)
#[test]
fn fight_saves_read_their_fights() {
    let (Some((first, _)), Some((second, _))) = (load_fight(FIGHT_FIRST), load_fight(FIGHT_SECOND))
    else {
        return;
    };
    for (name, save) in [(FIGHT_FIRST, &first), (FIGHT_SECOND, &second)] {
        report(name, save);
        assert!(save.sections.contains(&"Net System"), "{:?}", save.sections);
        assert!(save.problems.is_empty(), "{:?}", save.problems);
    }
    let lists = |s: &sav::SaveGame| {
        [
            s.fight_pairings.len(),
            s.fight_duels.len(),
            s.fight_defenders.len(),
        ]
    };
    assert_eq!(lists(&first), [2, 2, 1]);
    assert_eq!(lists(&second), [1, 2, 0]);
    eprintln!(
        "first: {:?} / {:?} / {:?}",
        fight_rows(&first.fight_pairings),
        fight_rows(&first.fight_duels),
        first.fight_defenders
    );
    let pairings = fight_rows(&second.fight_pairings);
    assert_eq!(pairings.len(), 1);
    let (attacker, opponent, _, counter, _, waiting) = pairings[0];
    assert_eq!(
        (attacker, opponent, counter, waiting),
        (16592, sav::NO_ID, 21, Some((33, 20)))
    );
    let duels: Vec<_> = fight_rows(&second.fight_duels)
        .into_iter()
        .map(|(a, o, _, c, hit, _)| (a, o, c, hit))
        .collect();
    assert_eq!(duels, [(16600, 18545, 11, true), (17868, 18560, 14, false)]);
}

// FMT-SAV-3.5c, 3.7a (T17, reading): in both fight saves every soldier's
// links agree with the fight records: a list-2 attacker's opponent link is
// the record's opponent and the opponent's is the attacker, the attacker's
// attack-target link is the record's target; a list-1 attacker has no
// opponent; the soldiers' state numbers are 18 (attack), 20 (wait) or 17
// (defend).
// spec: FMT-SAV-3.5c, FMT-SAV-3.7a (T17)
#[test]
fn fight_saves_links_agree() {
    let (Some((first, _)), Some((second, _))) = (load_fight(FIGHT_FIRST), load_fight(FIGHT_SECOND))
    else {
        return;
    };
    for save in [&first, &second] {
        let soldier = |id: u64| {
            save.settlers[4]
                .iter()
                .find(|s| s.id == id)
                .unwrap_or_else(|| panic!("soldier {id}"))
        };
        for (list, records) in [(1, &save.fight_pairings), (2, &save.fight_duels)] {
            for f in records.iter() {
                let a = soldier(f.attacker.expect("an attacker"));
                let links = a.soldier.expect("a soldier's links");
                eprintln!(
                    "list {list}: {} state {} at {:?} links {links:?} health {:?}",
                    a.id, a.state, a.movement.field, a.live_points
                );
                assert_eq!(links.attack_target, f.target);
                assert_eq!(links.opponent, f.opponent);
                assert!([18, 20].contains(&a.state));
                if let Some(o) = f.opponent {
                    let d = soldier(o);
                    let dl = d.soldier.expect("links");
                    eprintln!(
                        "   opponent {} state {} at {:?} links {dl:?} health {:?}",
                        d.id, d.state, d.movement.field, d.live_points
                    );
                    assert_eq!(dl.opponent, f.attacker);
                } else {
                    assert_eq!(list, 1);
                }
            }
        }
        for d in &save.fight_defenders {
            eprintln!("list 3: {d:?}");
        }
        for b in &save.buildings {
            let out: Vec<_> = b
                .soldiers
                .iter()
                .filter(|id| {
                    save.fight_pairings
                        .iter()
                        .chain(&save.fight_duels)
                        .any(|f| f.attacker == Some(**id) || f.opponent == Some(**id))
                })
                .collect();
            if !out.is_empty() {
                eprintln!(
                    "building {} at {:?} lists fighters {out:?} of {:?}",
                    b.id, b.field, b.soldiers
                );
            }
        }
    }
}

/// An attack record as (attacker, opponent, counter, hit, field), by saved
/// numbers.
type WorldFight = (u64, u64, u32, bool, Option<(u32, u32)>);

/// The world's attack records, in their order.
fn world_fights(world: &s2_sim::World) -> Vec<WorldFight> {
    let number = |id: usize| {
        world
            .settlers()
            .find(|(i, _)| *i == id)
            .map_or(sav::NO_ID, |(_, s)| u64::from(s.number))
    };
    world
        .attack_records()
        .iter()
        .map(|r| {
            (
                number(r.soldier),
                r.opponent.map_or(sav::NO_ID, number),
                r.counter,
                r.hit,
                r.field,
            )
        })
        .collect()
}

// FMT-SAV-3.7a (T17): the first fight save continues with its fights where
// they stood: the two duels (16579 against 16378, counter 12; 17868
// against the interceptor 16370, counter 5) and the two attackers not yet
// fighting (16600, counter 1; 16592, counter 9), each with its waiting
// field, none left out. Then 497 ticks are run and the fight lists are
// set beside the second save's (T17 expects them equal if the
// continuation is exact; printed, not asserted: the computer players'
// decisions are ours, not the original's).
// spec: FMT-SAV-3.7a (T17), MIL-ATK-6.2, MIL-ATK-6.3
#[test]
fn fight_save_continues_its_fights() {
    let (Some((first, rules)), Some((second, _))) =
        (load_fight(FIGHT_FIRST), load_fight(FIGHT_SECOND))
    else {
        return;
    };
    let (mut world, restored) = s2_rules::restore_game(&first, &rules).unwrap();
    eprintln!("restored {restored:?}");
    assert_eq!((restored.fights, restored.fights_skipped), (4, 0));
    let fights = world_fights(&world);
    eprintln!("after the load: {fights:?}");
    let none = sav::NO_ID;
    assert_eq!(
        fights,
        [
            (16579, 16378, 12, false, Some((32, 21))),
            (17868, 16370, 5, false, Some((31, 19))),
            (16600, none, 1, false, Some((32, 19))),
            (16592, none, 9, false, Some((33, 20))),
        ]
    );
    assert_eq!(world.duels().len(), 2);
    let start = world.tick();
    for _ in 0..497 {
        world.step();
    }
    eprintln!(
        "after 497 ticks (tick {} from {start}): {:?}",
        world.tick(),
        world_fights(&world)
    );
    eprintln!(
        "the second save: list 1 {:?}, list 2 {:?}, list 3 {:?}",
        fight_rows(&second.fight_pairings),
        fight_rows(&second.fight_duels),
        second.fight_defenders
    );
}
