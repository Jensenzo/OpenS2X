//! Checks of the world's consistency, run after every tick by the soak runs
//! (`s2-app soak`, `docs/test-coverage.md`). None of them is a game rule of
//! its own: each says what the rules, kept, never let happen.
//!
//! - `wares`: wares are conserved: what the world holds equals what it held
//!   at the start plus what was made minus what was used up (the ledger).
//! - `settler-place`: every settler is on the map, and one that works in or
//!   guards a building belongs to a building that stands.
//! - `one-job`: no two settlers or carriers claim one job: one worker per
//!   building, one builder and one leveller per site, one carrier and one
//!   donkey per road, one carrier per ware.
//! - `roads`: the road graph is consistent: flags at both ends, neighbouring
//!   fields, Street bits and the street index agree, wares at flags are where
//!   they say.
//! - `idle`: no builder or worker waits while its job has everything it
//!   needs for more than [`IDLE_LIMIT`] ticks.

use std::collections::{BTreeMap, BTreeSet};

use crate::grid::Point;
use crate::map::state;
use crate::{CarrierTask, Entry, SettlerJob, Target, Tick, WareLocation, Work, World};

/// Ticks a builder may stand at a site with material, or a worker wait for
/// inputs it has, before it counts as idle.
pub const IDLE_LIMIT: Tick = 600;

/// One broken invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub tick: Tick,
    /// The invariant's name (`wares`, `settler-place`, `one-job`, `roads`,
    /// `idle`).
    pub invariant: &'static str,
    /// The good concerned, for `wares`.
    pub good: Option<u32>,
    pub detail: String,
}

/// The checks with what they remember between ticks.
#[derive(Debug, Clone, Default)]
pub struct Checker {
    /// Per good: the wares held at the start less the ledger's balance then.
    baseline: BTreeMap<u32, i128>,
    /// Per site: its work when last changed and that tick.
    site_work: BTreeMap<Point, (u32, Tick)>,
    /// Per site: the tick its builder was first seen with usable material.
    site_ready: BTreeMap<Point, Tick>,
    /// Per building: the tick its worker was first seen waiting for inputs
    /// it had.
    worker_ready: BTreeMap<Point, Tick>,
    /// Idle reports already given, so a stall is reported once.
    reported: BTreeMap<Point, Tick>,
    /// The wares by place after the last check, to show where a lost or
    /// found ware was.
    places: Vec<(String, u32, u32)>,
}

impl Checker {
    /// Start checking `world` as it is now.
    pub fn new(world: &World) -> Self {
        let mut baseline: BTreeMap<u32, i128> = world
            .ware_holdings()
            .into_iter()
            .map(|(g, n)| (g, i128::from(n)))
            .collect();
        let ledger = world.ledger();
        for (&g, &n) in &ledger.made {
            *baseline.entry(g).or_default() -= i128::from(n);
        }
        for (&g, &n) in &ledger.used {
            *baseline.entry(g).or_default() += i128::from(n);
        }
        Self {
            baseline,
            places: world.ware_places(),
            ..Self::default()
        }
    }

    /// Every check on the world as it is after a step.
    pub fn check(&mut self, world: &World) -> Vec<Violation> {
        let mut out = Vec::new();
        let tick = world.tick();
        let mut fail = |invariant: &'static str, detail: String| {
            out.push(Violation {
                tick,
                invariant,
                good: None,
                detail,
            })
        };
        let wares = self.wares(world);
        settler_places(world, &mut fail);
        one_job(world, &mut fail);
        roads(world, &mut fail);
        self.idle(world, &mut fail);
        out.splice(0..0, wares);
        out
    }

    /// The conservation check. A difference is reported once, in the tick
    /// it appears, with the places of that good that changed; the count it
    /// is off by is then taken as the new start.
    fn wares(&mut self, world: &World) -> Vec<Violation> {
        let mut out = Vec::new();
        let held = world.ware_holdings();
        let ledger = world.ledger();
        let places = world.ware_places();
        let mut goods: Vec<u32> = held.keys().copied().collect();
        goods.extend(self.baseline.keys());
        goods.extend(ledger.made.keys());
        goods.extend(ledger.used.keys());
        goods.sort_unstable();
        goods.dedup();
        for g in goods {
            let get = |m: &BTreeMap<u32, u64>| i128::from(m.get(&g).copied().unwrap_or(0));
            let start = self.baseline.get(&g).copied().unwrap_or(0);
            let expected = start + get(&ledger.made) - get(&ledger.used);
            let have = get(&held);
            if have == expected {
                continue;
            }
            let of = |list: &[(String, u32, u32)]| -> BTreeMap<String, u32> {
                let mut m = BTreeMap::new();
                for (p, good, n) in list {
                    if *good == g {
                        *m.entry(p.clone()).or_default() += n;
                    }
                }
                m
            };
            let (before, after) = (of(&self.places), of(&places));
            let mut changed: Vec<String> = Vec::new();
            for p in before.keys().chain(after.keys()) {
                let (a, b) = (
                    before.get(p).copied().unwrap_or(0),
                    after.get(p).copied().unwrap_or(0),
                );
                if a != b && !changed.iter().any(|c| c.starts_with(&format!("{p}:"))) {
                    changed.push(format!("{p}: {a} -> {b}"));
                }
            }
            out.push(Violation {
                tick: world.tick(),
                invariant: "wares",
                good: Some(g),
                detail: format!(
                    "good {g}: {have} held, expected {expected} (start {start} + made {} - used {}); \
                     changed since the last tick: {}",
                    get(&ledger.made),
                    get(&ledger.used),
                    changed.join("; ")
                ),
            });
            *self.baseline.entry(g).or_default() += have - expected;
        }
        self.places = places;
        out
    }

    fn idle(&mut self, world: &World, fail: &mut impl FnMut(&'static str, String)) {
        let now = world.tick();
        // Builders: at the site, the ground levelled, a material it can
        // build in lying there, and no work done for IDLE_LIMIT ticks.
        self.site_work.retain(|at, _| world.sites.contains_key(at));
        self.site_ready.retain(|at, _| world.sites.contains_key(at));
        for (&at, site) in &world.sites {
            let usable = if site.frame_done() {
                !site.on_site.is_empty()
            } else {
                site.on_site
                    .iter()
                    .any(|&(g, _)| site.frame_costs.iter().any(|&(f, _)| f == g))
            };
            let entry = self.site_work.entry(at).or_insert((site.work_done, now));
            if entry.0 != site.work_done {
                *entry = (site.work_done, now);
                self.site_ready.remove(&at);
                continue;
            }
            if !(usable && site.builder.is_some() && site.levelled()) {
                self.site_ready.remove(&at);
                continue;
            }
            let since = *self.site_ready.entry(at).or_insert(now);
            if now - since > IDLE_LIMIT && self.reported.get(&at) != Some(&since) {
                self.reported.insert(at, since);
                fail(
                    "idle",
                    format!(
                        "builder at site {at:?} idle since tick {since}: work {}/{}, on site {:?}",
                        site.work_done, site.work_total, site.on_site
                    ),
                );
            }
        }
        // Workers: waiting for inputs the building has, production on.
        let mut waiting: Vec<Point> = Vec::new();
        for s in world.settlers.iter().flatten() {
            if let SettlerJob::Worker {
                target,
                work: Work::WaitInputs,
            } = s.job
            {
                let on = world
                    .target_building(target)
                    .is_some_and(|b| b.production.enabled && !b.under_construction);
                if on && world.can_produce(target.at) {
                    waiting.push(target.at);
                }
            }
        }
        self.worker_ready.retain(|at, _| waiting.contains(at));
        for at in waiting {
            let since = *self.worker_ready.entry(at).or_insert(now);
            if now - since > IDLE_LIMIT && self.reported.get(&at) != Some(&since) {
                self.reported.insert(at, since);
                fail(
                    "idle",
                    format!(
                        "worker of the building at {at:?} waits for inputs since tick {since} \
                         with {:?}",
                        world.inputs(at)
                    ),
                );
            }
        }
    }
}

/// Whether the building of `target` still stands.
fn stands(world: &World, target: Target) -> bool {
    world.target_building(target).is_some()
}

fn settler_places(world: &World, fail: &mut impl FnMut(&'static str, String)) {
    for (id, s) in world.settlers.iter().enumerate() {
        let Some(s) = s else { continue };
        if !world.terrain.contains(s.walker.at) {
            fail(
                "settler-place",
                format!("settler {id} off the map at {:?}", s.walker.at),
            );
        }
        match s.job {
            SettlerJob::Worker { target, .. } if !stands(world, target) => fail(
                "settler-place",
                format!("settler {id} works in a building that is gone: {target:?}"),
            ),
            SettlerJob::Garrison { target, .. } => {
                let listed = world
                    .military
                    .get(&target.at)
                    .is_some_and(|m| m.soldiers.contains(&id));
                if !stands(world, target) || !listed {
                    fail(
                        "settler-place",
                        format!(
                            "soldier {id} guards {target:?}, which {}",
                            if listed {
                                "is gone"
                            } else {
                                "does not list him"
                            }
                        ),
                    );
                }
            }
            _ => {}
        }
    }
    for (c, carrier) in world.carriers.iter().enumerate() {
        if !world.terrain.contains(carrier.walker.at) {
            fail(
                "settler-place",
                format!("carrier {c} off the map at {:?}", carrier.walker.at),
            );
        }
        if carrier.task == CarrierTask::Retired {
            continue;
        }
        let place = world
            .roads
            .get(carrier.road)
            .and_then(Option::as_ref)
            .map(|r| if carrier.donkey { r.donkey } else { r.carrier });
        if place != Some(Some(c)) {
            fail(
                "settler-place",
                format!(
                    "carrier {c} works road {}, whose place holds {place:?}",
                    carrier.road
                ),
            );
        }
    }
}

fn one_job(world: &World, fail: &mut impl FnMut(&'static str, String)) {
    // One worker per building, counting those on their way.
    let mut workers: BTreeMap<Target, Vec<usize>> = BTreeMap::new();
    let mut builders: BTreeMap<(Target, bool), Vec<usize>> = BTreeMap::new();
    for (id, s) in world.settlers.iter().enumerate() {
        match s.as_ref().map(|s| &s.job) {
            Some(&SettlerJob::Worker { target, .. }) | Some(&SettlerJob::ToBuilding { target }) => {
                workers.entry(target).or_default().push(id)
            }
            Some(&SettlerJob::ToSite { target, builder }) => {
                builders.entry((target, builder)).or_default().push(id)
            }
            _ => {}
        }
    }
    for (target, ids) in workers.into_iter().filter(|(_, v)| v.len() > 1) {
        fail(
            "one-job",
            format!("settlers {ids:?} are all the worker of {target:?}"),
        );
    }
    for ((target, builder), ids) in builders.into_iter().filter(|(_, v)| v.len() > 1) {
        let what = if builder { "builder" } else { "leveller" };
        fail(
            "one-job",
            format!("settlers {ids:?} all go to {target:?} as its {what}"),
        );
    }
    // One carrier per road place, one carrier per ware.
    let mut places: BTreeMap<usize, usize> = BTreeMap::new();
    for (id, road) in world.roads.iter().enumerate() {
        let Some(road) = road else { continue };
        for c in road.carrier.into_iter().chain(road.donkey) {
            if let Some(other) = places.insert(c, id) {
                fail(
                    "one-job",
                    format!("carrier {c} holds a place on roads {other} and {id}"),
                );
            }
        }
    }
    let mut carried: BTreeMap<usize, usize> = BTreeMap::new();
    for (c, carrier) in world.carriers.iter().enumerate() {
        let Some(ware) = carrier.ware else { continue };
        if let Some(other) = carried.insert(ware, c) {
            fail(
                "one-job",
                format!("ware {ware} is carried by carriers {other} and {c}"),
            );
        }
        let held = world
            .wares
            .get(ware)
            .and_then(Option::as_ref)
            .map(|w| w.location);
        if held != Some(WareLocation::Carried(c)) {
            fail(
                "one-job",
                format!("carrier {c} carries ware {ware}, which is at {held:?}"),
            );
        }
    }
    // TRN-CARRY-2.6: a ware lying at a flag is claimed only by a carrier
    // whose job is about it, or by a released carrier that has not noticed
    // yet (TRN-CARRY-7.1a: it lets go in its next update).
    let unnoticed: BTreeSet<usize> = world
        .settlers()
        .filter_map(|(_, s)| match s.job {
            crate::SettlerJob::Released {
                carrier,
                step: crate::ReleasedStep::Unnoticed { .. },
                ..
            } => Some(carrier),
            _ => None,
        })
        .collect();
    for (id, ware) in world.wares.iter().enumerate() {
        let Some(ware) = ware else { continue };
        let (WareLocation::Flag(_), Some(c)) = (ware.location, ware.claimed_by) else {
            continue;
        };
        let mine = world
            .carriers
            .get(c)
            .is_some_and(|carrier| match carrier.task {
                CarrierTask::Fetch { ware, .. } | CarrierTask::PickUp { ware, .. } => ware == id,
                CarrierTask::Carry {
                    entry: Entry::Exchange(ware),
                    ..
                } => ware == id,
                CarrierTask::PutDown { exchange, .. } | CarrierTask::Stop { exchange, .. } => {
                    exchange == Some(id)
                }
                // TRN-ROAD-6.3 (T32, Q-294): released, he stays bound to
                // his exchange ware until he is taken in or dies.
                CarrierTask::Retired => unnoticed.contains(&c) || carrier.bound == Some(id),
                // TRN-ROAD-4.2 (T31): on his way to the new flag after a
                // split, a fetching carrier keeps his claim until he arrives.
                CarrierTask::Arrive { stand: true, .. } => true,
                _ => false,
            });
        if !mine {
            fail(
                "one-job",
                format!(
                    "ware {id} is claimed by carrier {c}, whose task {:?} is not about it",
                    world.carriers.get(c).map(|c| c.task)
                ),
            );
        }
    }
}

fn roads(world: &World, fail: &mut impl FnMut(&'static str, String)) {
    let terrain = &world.terrain;
    for (id, road) in world.roads.iter().enumerate() {
        let Some(road) = road else { continue };
        let path = &road.path;
        if path.len() < 2 {
            fail("roads", format!("road {id} has {} fields", path.len()));
            continue;
        }
        for end in [path[0], path[path.len() - 1]] {
            if world.flag_at(end) != Some(road.owner) {
                fail(
                    "roads",
                    format!("road {id} ends at {end:?} without its owner's flag"),
                );
            }
        }
        for pair in path.windows(2) {
            if !terrain.neighbours(pair[0]).contains(&Some(pair[1])) {
                fail(
                    "roads",
                    format!(
                        "road {id}: {:?} and {:?} are not neighbours",
                        pair[0], pair[1]
                    ),
                );
            }
        }
        for &field in &path[1..path.len() - 1] {
            if world.streets[terrain.index(field)] != Some(id) || !terrain.has(field, state::STREET)
            {
                fail(
                    "roads",
                    format!("road {id}: its field {field:?} is not marked as its street"),
                );
            }
        }
    }
    for field in terrain.fields() {
        let Some(id) = world.streets[terrain.index(field)] else {
            continue;
        };
        let inner = world
            .roads
            .get(id)
            .and_then(Option::as_ref)
            .is_some_and(|r| r.path.len() > 2 && r.path[1..r.path.len() - 1].contains(&field));
        if !inner {
            fail(
                "roads",
                format!("field {field:?} names road {id}, which does not run over it"),
            );
        }
    }
    for (&flag, list) in &world.flag_wares {
        if !list.is_empty() && world.flag_at(flag).is_none() {
            fail(
                "roads",
                format!("wares {list:?} lie at {flag:?}, where no flag is"),
            );
        }
        for &w in list {
            let at = world
                .wares
                .get(w)
                .and_then(Option::as_ref)
                .map(|w| w.location);
            if at != Some(WareLocation::Flag(flag)) {
                fail(
                    "roads",
                    format!("flag {flag:?} lists ware {w}, which is at {at:?}"),
                );
            }
        }
    }
    for (id, ware) in world.wares.iter().enumerate() {
        if let Some(WareLocation::Flag(flag)) = ware.as_ref().map(|w| w.location) {
            if !world.flag_wares.get(&flag).is_some_and(|l| l.contains(&id)) {
                fail(
                    "roads",
                    format!("ware {id} lies at {flag:?}, which does not list it"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{GoodKind, GoodRules, Settler};

    const PLANK: u32 = 70;

    /// Two flags joined by a road with its carrier, a plank lying at the
    /// first flag (made through the ledger, as the rules make wares).
    fn small_world() -> World {
        let mut world = world(flat(24, 24));
        world.goods.insert(
            PLANK,
            GoodRules {
                kind: GoodKind::Ware,
                ..GoodRules::default()
            },
        );
        world.priorities.insert(PLANK, 1);
        flag(&mut world, (6, 6)).unwrap();
        flag(&mut world, (12, 6)).unwrap();
        road(&mut world, (6, 6), (12, 6)).unwrap();
        world.new_ware_at(PLANK, (6, 6), None, None);
        world.made(Some(PLANK), 1);
        world
    }

    fn names(found: &[Violation]) -> Vec<&'static str> {
        found.iter().map(|v| v.invariant).collect()
    }

    // spec: none (the soak's invariant checker on a sound world)
    #[test]
    fn a_world_that_keeps_the_rules_passes() {
        let mut world = small_world();
        let mut checker = Checker::new(&world);
        for _ in 0..300 {
            world.step();
            assert_eq!(checker.check(&world), vec![], "tick {}", world.tick());
        }
    }

    // spec: none (the soak's ware conservation check)
    #[test]
    fn a_ware_that_vanishes_is_reported_once_with_its_place() {
        let mut world = small_world();
        let mut checker = Checker::new(&world);
        let id = world.flag_wares[&(6, 6)][0];
        world.flag_wares.get_mut(&(6, 6)).unwrap().clear();
        world.wares[id] = None;
        let found = checker.check(&world);
        assert_eq!(names(&found), ["wares"]);
        assert_eq!(found[0].good, Some(PLANK));
        assert!(
            found[0].detail.contains("flag (6, 6)"),
            "{}",
            found[0].detail
        );
        // The new count is the start from now on.
        assert_eq!(checker.check(&world), vec![]);
    }

    // spec: none (the soak's ware conservation check)
    #[test]
    fn a_ware_made_outside_the_ledger_is_reported() {
        let mut world = small_world();
        let mut checker = Checker::new(&world);
        world.new_ware_at(PLANK, (12, 6), None, None);
        assert_eq!(names(&checker.check(&world)), ["wares"]);
    }

    // spec: none (the soak's road graph check)
    #[test]
    fn a_street_the_index_does_not_know_is_reported() {
        let mut world = small_world();
        let mut checker = Checker::new(&world);
        let inner = world.roads[0].as_ref().unwrap().path[2];
        let index = world.terrain.index(inner);
        world.streets[index] = None;
        assert_eq!(names(&checker.check(&world)), ["roads"]);
    }

    // spec: none (the soak's flag ware list check)
    #[test]
    fn a_ware_the_flag_does_not_list_is_reported() {
        let mut world = small_world();
        let mut checker = Checker::new(&world);
        world.flag_wares.get_mut(&(6, 6)).unwrap().clear();
        assert_eq!(names(&checker.check(&world)), ["roads"]);
    }

    // spec: none (the soak's one-job and settler-place checks)
    #[test]
    fn one_carrier_on_two_roads_is_reported() {
        let mut world = small_world();
        flag(&mut world, (12, 12)).unwrap();
        road(&mut world, (12, 6), (12, 12)).unwrap();
        let mut checker = Checker::new(&world);
        let first = world.roads[0].as_ref().unwrap().carrier;
        world.roads[1].as_mut().unwrap().carrier = first;
        let found = names(&checker.check(&world));
        assert!(found.contains(&"one-job"), "{found:?}");
        assert!(found.contains(&"settler-place"), "{found:?}");
    }

    // spec: none (the soak's settler-place check)
    #[test]
    fn a_settler_off_the_map_is_reported() {
        let mut world = small_world();
        let mut checker = Checker::new(&world);
        world.add_settler(Settler {
            number: 0,
            player: P0,
            profession: 1,
            list: crate::SettlerList::Workers,
            walker: crate::movement::Walker::new((40, 3)),
            job: SettlerJob::Dying,
            timer: 5,
            carrying: None,
        });
        assert_eq!(names(&checker.check(&world)), ["settler-place"]);
    }
}
