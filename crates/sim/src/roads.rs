//! Roads between flags (`specs/TRN-roads-flags.md`).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::grid::{self, Point};
use crate::map::state;
use crate::{PlayerId, Rejection, Road, World};

/// TRN-ROAD-2.2: cost of the climb from X to Y by the height class of
/// `height(Y) - height(X)`.
fn half_step_cost(class: i8) -> u32 {
    match class {
        -1 => 4,
        -2 => 5,
        -3 => 7,
        _ => 3,
    }
}

/// TRN-ROAD-2.2: harbour pavement doubles the cost of stepping onto it.
const HARBOR_FACTOR: u32 = 2;
/// TRN-ROAD-7.4: the most steps of a waterway.
const WATERWAY_MAX_STEPS: usize = 6;

/// SIM-START-4.3: the three transport kinds whose road networks a flag
/// holds an id of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Network {
    Carrier,
    Donkey,
    Boat,
}

impl World {
    /// TRN-ROAD-2.2: cost of one road step from `a` to its neighbour `b`.
    pub fn step_cost(&self, a: Point, b: Point) -> u32 {
        let (ha, hb) = (
            i64::from(self.terrain.height_at(a)),
            i64::from(self.terrain.height_at(b)),
        );
        let cost = half_step_cost(grid::height_class(hb - ha))
            + half_step_cost(grid::height_class(ha - hb));
        if self.terrain.has(b, state::FOR_HARBOR) {
            cost * HARBOR_FACTOR
        } else {
            cost
        }
    }

    /// TRN-ROAD-1.1 to 1.3: whether a road may pass through `field` (not as
    /// its target).
    fn road_may_enter(&self, player: PlayerId, field: Point) -> bool {
        self.is_general_site(player, field)
            && !self.terrain.has(field, state::STREET)
            && self.flag_at(field).is_none()
            && self.explored(player, field)
    }

    /// TRN-ROAD-2.1: the route from `from` (the start S) to `to` (the
    /// target T) and its cost: the search of 2.3, then the read-out of 2.4.
    /// The fields run from S to T, both included. It is usually, not
    /// always, the cheapest route.
    pub fn find_route(
        &self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Result<(Vec<Point>, u32), Rejection> {
        let terrain = &self.terrain;
        if !terrain.contains(from) || !terrain.contains(to) {
            return Err(Rejection::OutsideMap);
        }
        if from == to {
            return Err(Rejection::SameFlag);
        }
        // TRN-ROAD-1.1: the target too must be a general site.
        if !self.is_general_site(player, to) {
            return Err(Rejection::TargetNotGeneralSite);
        }
        // TRN-ROAD-2.5
        if grid::distance(from, to) == 1 {
            return Ok((vec![from, to], self.step_cost(to, from)));
        }
        let labels = self.route_labels(player, from, to)?;
        let path = self.read_out(&labels, from, to);
        // TRN-ROAD-2.2: the search entered each field from its neighbour
        // nearer T.
        let cost = path.windows(2).map(|w| self.step_cost(w[1], w[0])).sum();
        Ok((path, cost))
    }

    /// TRN-ROAD-2.3: the labels the search from T towards S leaves (`None`
    /// for a field without one).
    fn route_labels(
        &self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Result<Vec<Option<u32>>, Rejection> {
        let terrain = &self.terrain;
        let mut labels = vec![None; terrain.len()];
        // The open list by label plus distance to S, first in first out
        // among equal values. `place` holds the insertion number of a
        // field's entry while it is in the list: an entry with another
        // number was taken out when the field was relabelled.
        let mut list = BinaryHeap::new();
        let mut place = vec![None; terrain.len()];
        let mut inserted = 0u64;
        let target = terrain.index(to);
        labels[target] = Some(0);
        place[target] = Some(inserted);
        list.push(Reverse((grid::distance(to, from), inserted, target)));
        let field_of = |index: usize| {
            let width = terrain.width as usize;
            ((index % width) as u32, (index / width) as u32)
        };
        while let Some(Reverse((_, number, index))) = list.pop() {
            if place[index] != Some(number) {
                continue;
            }
            place[index] = None;
            let here = field_of(index);
            let label = labels[index].expect("a listed field has a label");
            for next in terrain.neighbours(here).into_iter().flatten() {
                // TRN-ROAD-1: only S is exempt from the street and flag
                // rule.
                let enterable = if next == from {
                    self.is_general_site(player, next) && self.explored(player, next)
                } else {
                    self.road_may_enter(player, next)
                };
                if !enterable {
                    continue;
                }
                let next_index = terrain.index(next);
                let cost = label + self.step_cost(here, next);
                if labels[next_index].is_some_and(|old| old <= cost) {
                    continue;
                }
                labels[next_index] = Some(cost);
                // The search ends the moment S has a label.
                if next == from {
                    return Ok(labels);
                }
                inserted += 1;
                place[next_index] = Some(inserted);
                list.push(Reverse((
                    cost + grid::distance(next, from),
                    inserted,
                    next_index,
                )));
            }
        }
        Err(Rejection::NoRoute)
    }

    /// TRN-ROAD-2.4: the road read from S to T over the labels: at each
    /// field the neighbours with the lowest label below the field's are the
    /// candidates; the previous step's direction is kept when it is one,
    /// else the lowest direction is taken (0 before the first step).
    fn read_out(&self, labels: &[Option<u32>], from: Point, to: Point) -> Vec<Point> {
        let terrain = &self.terrain;
        let label_at = |field: Point| labels[terrain.index(field)];
        let mut path = vec![from];
        let mut at = from;
        let mut direction = 0;
        while at != to {
            let here = label_at(at).expect("a field of the read-out has a label");
            let neighbours = terrain.neighbours(at);
            let lower = |d: usize| {
                neighbours[d]
                    .and_then(label_at)
                    .filter(|&label| label < here)
            };
            let least = (0..6).filter_map(lower).min().expect("a lower neighbour");
            if lower(direction) != Some(least) {
                direction = (0..6)
                    .find(|&d| lower(d) == Some(least))
                    .expect("a candidate");
            }
            at = neighbours[direction].expect("a candidate inside the map");
            path.push(at);
        }
        path
    }

    pub(crate) fn build_road(
        &mut self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(from) || !self.terrain.contains(to) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(from) != Some(player) {
            return Err(Rejection::NoFlagAtStart);
        }
        if self.flag_at(to) != Some(player) {
            return Err(Rejection::NoFlagAtTarget);
        }
        let (path, _) = self.find_route(player, from, to)?;
        self.add_road(player, path.clone(), false);
        self.even_road(&path);
        Ok(())
    }

    /// AI-CON-8.1 b, 8.3 c: the computer player's road builder from its flag
    /// `from` to its flag `to`. The one-field planning stub at `from` takes a
    /// number (SIM-START-4.4 c, Q-274); every field of the path found evens
    /// the ground as it is added (TRN-ROAD-3.3a, built or not); the road is
    /// made only when its steps are at most 2 x the distance of its ends.
    pub(crate) fn plan_road(
        &mut self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(from) || !self.terrain.contains(to) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(from) != Some(player) {
            return Err(Rejection::NoFlagAtStart);
        }
        self.new_number();
        if self.flag_at(to) != Some(player) {
            return Err(Rejection::NoFlagAtTarget);
        }
        let (path, _) = self.find_route(player, from, to)?;
        self.even_road(&path);
        if path.len() as u32 - 1 > 2 * grid::distance(from, to) {
            return Err(Rejection::PlannedRoadTooLong);
        }
        // Made, it is evened again with the same values (3.3).
        self.add_road(player, path, false);
        Ok(())
    }

    /// TRN-ROAD-3.3: a land road made along `path` evens the ground along
    /// it. Step by step in the road's order, on the heights as they are, each
    /// step is worked from its field whose direction to the other is 2, 3 or
    /// 4 (sample A; the other B, 4 sample steps away in direction d): with
    /// q = (hB - hA) / 4 and v0 = hA, vk = v(k-1) + q (float32), the sample
    /// k steps from A takes trunc(vk) and its neighbours in directions d + 2
    /// and d - 2 trunc((v(k-1) + vk) / 2), k = 1 to 3. Next to the road's
    /// first field (a) and to a flag that ends the step (b), the samples
    /// next to that field are left: all three at k = 1 when it is A, the
    /// centre at k = 3 when it is B.
    pub(crate) fn even_road(&mut self, path: &[Point]) {
        if let Some(&first) = path.first() {
            self.even_steps(path, first);
        }
    }

    /// TRN-ROAD-3.3, 3.3a: the steps along `path` evened as [`World::
    /// even_road`] does, for a stretch of a road or route whose first field
    /// is `first` (exception (a) applies only to the step next to it).
    pub(crate) fn even_steps(&mut self, path: &[Point], first: Point) {
        if self.terrain.samples.is_empty() {
            return;
        }
        let (columns, rows) = self.terrain.sample_size();
        let inside = |(c, r): (i32, i32)| {
            let (c, r) = (u32::try_from(c).ok()?, u32::try_from(r).ok()?);
            (c < columns && r < rows).then_some((c, r))
        };
        let mut touched = Vec::new();
        for (i, pair) in path.windows(2).enumerate() {
            let (here, second) = (pair[0], pair[1]);
            let Some(d) = (0..6u8).find(|&d| self.terrain.neighbour(here, d) == Some(second))
            else {
                continue;
            };
            let (a_field, b_field, d) = if (2..=4).contains(&d) {
                (here, second, d)
            } else {
                (second, here, (d + 3) % 6)
            };
            let (a, b) = (grid::field_sample(a_field), grid::field_sample(b_field));
            let ha = self.terrain.sample_height(a) as f32;
            let q = (self.terrain.sample_height(b) as f32 - ha) * 0.25;
            // (a) the road's first step, (b) a step ending on a flag.
            let starts = i == 0 && here == first;
            let mut leave_a = starts && a_field == first;
            let mut leave_b_centre = starts && b_field == first;
            if self.flag_at(second).is_some() {
                leave_b_centre |= second == b_field;
                leave_a |= second == a_field;
            }
            let dir = |n: u8| grid::Direction::ALL[usize::from(n % 6)];
            let mut at = (a.0 as i32, a.1 as i32);
            let mut v = ha;
            for k in 1..=3 {
                let before = v;
                v += q;
                at = grid::step(at.0, at.1, dir(d));
                if k == 1 && leave_a {
                    continue;
                }
                if !(k == 3 && leave_b_centre) {
                    if let Some(sample) = inside(at) {
                        self.set_sample_height(sample, v.trunc() as i32);
                    }
                }
                let side = ((before + v) / 2.0).trunc() as i32;
                for turn in [d + 2, d + 4] {
                    let next = grid::step(at.0, at.1, dir(turn));
                    if let Some(sample) = inside(next) {
                        self.set_sample_height(sample, side);
                    }
                }
            }
            touched.extend([here, second]);
        }
        self.changed(&touched);
    }

    /// A new road over `path`: its inner fields get the Street state.
    pub(crate) fn add_road(&mut self, player: PlayerId, path: Vec<Point>, water: bool) {
        // SIM-START-4.2: the road's number; SIM-START-4.3: then the
        // networks it joins are renumbered.
        let number = self.new_number();
        self.renumber_joined(path[0], path[path.len() - 1], water);
        let id = self.roads.len();
        let inner = path[1..path.len() - 1].to_vec();
        for &field in &inner {
            self.set_bits(field, state::STREET, true);
            let index = self.terrain.index(field);
            self.streets[index] = Some(id);
        }
        let length = self.road_length(&path);
        self.route_topology.clear();
        let ends = [path[0], path[path.len() - 1]];
        self.roads.push(Some(Road {
            owner: player,
            number,
            path,
            carrier: None,
            donkey: None,
            length,
            traffic: Default::default(),
            water,
        }));
        self.mark_flag_streets(&ends);
        // ECO-REQ-1.1: the road requests its carrier itself.
        self.changed(&inner);
    }

    /// TRN-ROAD-3.1 (T26): a flag's field carries the Street state while at
    /// least one road is attached to it, and loses it with its last road.
    pub(crate) fn mark_flag_streets(&mut self, flags: &[Point]) {
        for &flag in flags {
            let attached = self.flag_at(flag).is_some()
                && self
                    .roads
                    .iter()
                    .flatten()
                    .any(|r| r.path[0] == flag || r.path.last() == Some(&flag));
            self.set_bits(flag, state::STREET, attached);
        }
        self.changed(flags);
    }

    /// TRN-ROAD-7.2: whether a waterway may step from `q` onto `p`, with
    /// `from` its start flag and `to` its target.
    fn waterway_may_step(&self, from: Point, to: Point, q: Point, p: Point) -> bool {
        let terrain = &self.terrain;
        let water = |f: Point| terrain.has(f, state::WATER);
        if p != to {
            let object = terrain.has(p, state::STREET)
                || self.flag_at(p).is_some()
                || self.building_at(p).is_some()
                || self.deposits.contains_key(&p);
            if object || !water(p) || !(water(q) || q == from) {
                return false;
            }
        }
        // The water body: a path over water fields stays in the one it
        // starts in, so no further test is needed.
        (water(p) || water(q)) && !terrain.has(p, state::BORDER | state::DOODAD_BLOCKED)
    }

    /// TRN-ROAD-7.4, waterway terms: whether a route of `points` fields
    /// ending on `end` may be extended to `target`: the distance to the
    /// target plus the points already in the route minus 1 is at most 6,
    /// and a route of more than one point ends on a Water field.
    pub fn waterway_end_allowed(&self, end: Point, points: usize, target: Point) -> bool {
        let steps = grid::distance(end, target) as usize + points.saturating_sub(1);
        steps <= WATERWAY_MAX_STEPS && (points <= 1 || self.terrain.has(end, state::WATER))
    }

    /// TRN-ROAD-7: the cheapest waterway from the flag `from` to `to` and
    /// its cost: a search from the start with the field rules of 7.2 and
    /// the step cost of 7.3 (1, or 2 onto a Coast field), without the
    /// explored-field rule. Whether it runs from the target like the land
    /// search (TRN-ROAD-2.3, 2.4) is open (Q-254).
    pub fn find_waterway(&self, from: Point, to: Point) -> Result<(Vec<Point>, u32), Rejection> {
        let terrain = &self.terrain;
        if !terrain.contains(from) || !terrain.contains(to) {
            return Err(Rejection::OutsideMap);
        }
        if from == to {
            return Err(Rejection::SameFlag);
        }
        // TRN-ROAD-7.4: the target inside the map and not Border, the start
        // not DoodadBlocked.
        if terrain.has(to, state::BORDER) || terrain.has(from, state::DOODAD_BLOCKED) {
            return Err(Rejection::TargetNotGeneralSite);
        }
        // TRN-ROAD-7.4: from the start flag, a route of one point.
        if !self.waterway_end_allowed(from, 1, to) {
            return Err(Rejection::WaterwayTooLong);
        }
        self.waterway_search(from, from, to)
    }

    /// TRN-ROAD-7.2, 7.3: the cheapest waterway from `from` (the start
    /// flag `flag`, or the current end of a route that starts there) to
    /// `to`, and its cost.
    pub(crate) fn waterway_search(
        &self,
        flag: Point,
        from: Point,
        to: Point,
    ) -> Result<(Vec<Point>, u32), Rejection> {
        let terrain = &self.terrain;
        let step_cost = |p: Point| if terrain.has(p, state::COAST) { 2 } else { 1 };
        let (start, target) = (terrain.index(from), terrain.index(to));
        let mut cost = vec![u32::MAX; terrain.len()];
        let mut came_from = vec![usize::MAX; terrain.len()];
        let mut queue = BinaryHeap::new();
        let mut order = 0u64;
        cost[start] = 0;
        queue.push(Reverse((grid::distance(from, to), order, start)));
        let field_of = |index: usize| {
            let width = terrain.width as usize;
            ((index % width) as u32, (index / width) as u32)
        };
        while let Some(Reverse((_, _, index))) = queue.pop() {
            if index == target {
                let mut path = vec![to];
                let mut at = target;
                while at != start {
                    at = came_from[at];
                    path.push(field_of(at));
                }
                path.reverse();
                return Ok((path, cost[target]));
            }
            let here = field_of(index);
            for next in terrain.neighbours(here).into_iter().flatten() {
                if !self.waterway_may_step(flag, to, here, next) {
                    continue;
                }
                let next_index = terrain.index(next);
                let next_cost = cost[index] + step_cost(next);
                if next_cost < cost[next_index] {
                    cost[next_index] = next_cost;
                    came_from[next_index] = index;
                    order += 1;
                    // Every step costs at least 1: the field distance never
                    // overstates.
                    let estimate = next_cost + grid::distance(next, to);
                    queue.push(Reverse((estimate, order, next_index)));
                }
            }
        }
        Err(Rejection::NoRoute)
    }

    /// TRN-ROAD-7.1: a waterway between two of the player's flags, the
    /// first on a Coast field.
    pub(crate) fn build_waterway(
        &mut self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(from) || !self.terrain.contains(to) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(from) != Some(player) {
            return Err(Rejection::NoFlagAtStart);
        }
        if self.flag_at(to) != Some(player) {
            return Err(Rejection::NoFlagAtTarget);
        }
        if !self.terrain.has(from, state::COAST) {
            return Err(Rejection::NotCoast);
        }
        let (path, _) = self.find_waterway(from, to)?;
        self.add_road(player, path, true);
        Ok(())
    }

    /// TRN-ROAD-6.1, TER-5.3: the road is removed as when its player removes
    /// it. Its fields are free again; its carrier and donkey walk home
    /// (TRN-CARRY-7.1); a ware a carrier held is destroyed and wares it had
    /// claimed stay at their flag, free again (TRN-ROAD-6.3). The end flags
    /// stay. The road's requests for a carrier or donkey go with the road.
    pub(crate) fn remove_road(&mut self, id: usize) {
        let Some(road) = self.roads[id].take() else {
            return;
        };
        self.route_topology.clear();
        let inner = road.path[1..road.path.len() - 1].to_vec();
        for &field in &inner {
            self.set_bits(field, state::STREET, false);
            let index = self.terrain.index(field);
            self.streets[index] = None;
        }
        for (place, donkey) in [(road.carrier, false), (road.donkey, true)] {
            if let Some(carrier) = place {
                self.retire_carrier(carrier, donkey, road.owner, &road.path);
            }
        }
        // TRN-FREE-6.4b (T47a): a builder going home whose step in progress
        // runs along the road has his walk cut to that step: he ends it on
        // its point and stops there, nothing of the old walk left. (A
        // builder still walking to a standing site is not cut; other
        // settlers were not followed.)
        let builder = self.roles.builder;
        let along = |a: Point, b: Point| {
            road.path
                .windows(2)
                .any(|w| (w[0], w[1]) == (a, b) || (w[1], w[0]) == (a, b))
        };
        // T47b (E-trn-040): one on the walk home his own home search chose
        // is cut the same way (the excess dropped at the step's end) and
        // chooses his way home again in his arrival tick, with no standing
        // (`SettlerJob::Home`'s arrival short of its warehouse).
        // T54 (E-trn-046): when the step in progress is the road's last
        // step, onto its end flag that still stands, nothing is cut: he
        // reaches that flag as on any road walk and goes on by 6.5 (c), the
        // excess carried. Measured for the walk home (T47b) only.
        let ends: Vec<Point> = [road.path[0], road.path[road.path.len() - 1]]
            .into_iter()
            .filter(|&end| self.flag_at(end).is_some())
            .collect();
        for settler in self.settlers.iter_mut().flatten() {
            let going_home = matches!(settler.job, crate::SettlerJob::Leaving { .. })
                && Some(settler.profession) == builder;
            let walking_home = matches!(settler.job, crate::SettlerJob::Home { .. });
            if !(going_home || walking_home) || settler.player != road.owner {
                continue;
            }
            let walker = &mut settler.walker;
            if walking_home && walker.next.is_some_and(|next| ends.contains(&next)) {
                continue;
            }
            if walker.next.is_some_and(|next| along(walker.at, next)) {
                walker.path.clear();
                walker.pauses.clear();
                if walking_home {
                    walker.road_to = None;
                    walker.held = false;
                }
            }
        }
        self.renumber_removed(road.owner, road.carrier.is_some());
        // TRN-ROAD-6.1 (T26): an end flag keeps its Street while another
        // road is attached to it.
        self.mark_flag_streets(&[road.path[0], road.path[road.path.len() - 1]]);
        self.changed(&inner);
    }

    /// SIM-START-4.3: whether the flags `a` and `b` are in one road network
    /// of a kind (their ids for it are equal: a renumbering writes the
    /// whole network).
    pub(crate) fn same_network(&self, a: Point, b: Point, kind: Network) -> bool {
        let mut links: BTreeMap<Point, Vec<Point>> = BTreeMap::new();
        for road in self.roads.iter().flatten() {
            let member = match kind {
                Network::Boat => true,
                Network::Carrier => !road.water,
                Network::Donkey => road.carrier.is_some(),
            };
            if !member {
                continue;
            }
            let (from, to) = (road.path[0], road.path[road.path.len() - 1]);
            links.entry(from).or_default().push(to);
            links.entry(to).or_default().push(from);
        }
        let mut seen = BTreeSet::from([a]);
        let mut open = vec![a];
        while let Some(flag) = open.pop() {
            if flag == b {
                return true;
            }
            for &next in links.get(&flag).into_iter().flatten() {
                if seen.insert(next) {
                    open.push(next);
                }
            }
        }
        false
    }

    /// SIM-START-4.3 a: a road about to be made between the flags `a` and
    /// `b` joins their networks: where their ids differ, the joined network
    /// takes one fresh number, the boat network first, then the carrier
    /// network (a waterway is in the boat network only).
    fn renumber_joined(&mut self, a: Point, b: Point, water: bool) {
        if !self.same_network(a, b, Network::Boat) {
            self.new_number();
        }
        if !water && !self.same_network(a, b, Network::Carrier) {
            self.new_number();
        }
    }

    /// SIM-START-4.3 b: the road `id` gets its carrier: where its two ends'
    /// donkey ids differ (the donkey network joins the roads that have a
    /// carrier), the joined network takes a fresh number. A waterway's boat
    /// carrier joins it the same way (SIM-START-4.4 e, Q-263 c).
    pub(crate) fn renumber_carried(&mut self, id: usize) {
        let Some(road) = self.roads[id].as_ref() else {
            return;
        };
        if road.carrier.is_some() {
            return;
        }
        let (a, b, owner) = (road.path[0], road.path[road.path.len() - 1], road.owner);
        if !self.same_network(a, b, Network::Donkey) {
            self.new_number();
            // TRN-CARRY-5.3a: the ware renumbering empties the store.
            self.route_store.empty_ware(owner);
        }
    }

    /// SIM-START-4.3 c: a road is gone: if it had a carrier, the donkey
    /// network of each end is renumbered (two numbers); then the boat and
    /// the carrier network of one end, unconditionally (two more).
    fn renumber_removed(&mut self, owner: PlayerId, had_carrier: bool) {
        if had_carrier {
            self.new_number();
            self.new_number();
            // TRN-CARRY-5.3a: the ware renumberings empty the store.
            self.route_store.empty_ware(owner);
        }
        self.new_number();
        self.new_number();
    }

    /// TRN-ROAD-6.1: the player destroys the road that has `at` as an inner
    /// field.
    pub(crate) fn remove_road_command(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        let id = self.streets[self.terrain.index(at)]
            .filter(|&id| self.roads[id].as_ref().is_some_and(|r| r.owner == player))
            .ok_or(Rejection::NoRoad)?;
        self.remove_road(id);
        Ok(())
    }

    /// TRN-ROAD-6.2: the player destroys one of his flags.
    pub(crate) fn remove_flag_command(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(at) != Some(player) {
            return Err(Rejection::NotAFlag);
        }
        self.remove_flag(at);
        Ok(())
    }

    /// TRN-ROAD-4.2, SIM-START-4.4 j: a flag put on an inner field of the
    /// road `id` first removes the road (its numbers of SIM-START-4.3 c:
    /// the donkey ids of both ends if it had a carrier and no donkey, then
    /// the boat and carrier ids of one end); its carrier and donkey stay
    /// with it, unreleased. Returns the road for [`World::split_road`].
    pub(crate) fn take_split_road(&mut self, id: usize) -> Option<(usize, Road)> {
        let road = self.roads[id].take()?;
        self.route_topology.clear();
        // ECO-REQ-2.3a (T38): the old road's open requests lose their link
        // in their places (the first half takes the old id); the queue
        // deletes them on their turn.
        self.unlink_requests(road.owner, crate::Requester::Road(id));
        self.renumber_removed(road.owner, road.carrier.is_some() && road.donkey.is_none());
        Some((id, road))
    }

    /// TRN-ROAD-4.2, SIM-START-4.4 j: the removed road `road` (id `id`) is
    /// made again as two roads meeting at the new flag `at`: the first half
    /// from the old road's first flag to `at`, the second from its second
    /// flag to `at`, each made with its join renumberings (SIM-START-4.3 a);
    /// then the carrier goes to the half holding it, which joins the donkey
    /// networks (4.3 b). The first half keeps the old id here. Each half's
    /// traffic starts afresh, and the one without a carrier asks for one
    /// (ECO-REQ-1.1).
    pub(crate) fn split_road(&mut self, id: usize, road: Road, at: Point) {
        let index = self.terrain.index(at);
        let split = road
            .path
            .iter()
            .position(|&f| f == at)
            .expect("inner field on its road");
        let first = road.path[..=split].to_vec();
        let mut second = road.path[split..].to_vec();
        second.reverse();
        let (first_length, second_length) = (self.road_length(&first), self.road_length(&second));
        let first_number = self.new_number();
        self.renumber_joined(first[0], at, road.water);
        self.roads[id] = Some(Road {
            owner: road.owner,
            number: first_number,
            path: first,
            carrier: None,
            donkey: None,
            length: first_length,
            traffic: Default::default(),
            water: road.water,
        });
        let second_number = self.new_number();
        self.renumber_joined(second[0], at, road.water);
        let second_id = self.roads.len();
        for &field in &second[1..second.len() - 1] {
            let i = self.terrain.index(field);
            self.streets[i] = Some(second_id);
        }
        self.roads.push(Some(Road {
            owner: road.owner,
            number: second_number,
            path: second,
            carrier: None,
            donkey: None,
            length: second_length,
            traffic: Default::default(),
            water: road.water,
        }));
        // W19: both halves keep a pavement.
        self.paving_split(road.number, first_number, second_number);
        self.streets[index] = None;
        // TRN-ROAD-3.1 (T26): the new flag has the two halves attached.
        self.mark_flag_streets(&[at]);
        self.route_topology.clear();
        self.split_carriers(id, second_id, at, &road);
        // TRN-ROAD-3.3: the two new land roads even the ground again.
        if !road.water {
            for half in [id, second_id] {
                let path = self.roads[half].as_ref().expect("live road").path.clone();
                self.even_road(&path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::Command;

    // TRN-ROAD-3.3a, SIM-START-4.4 c, AI-CON-8.1 b: the computer player's
    // road builder takes its stub's number and evens the ground along the
    // path it finds, then makes the road only when its steps are at most
    // 2 x the distance of its ends: forced round a wall of road fields the
    // road is refused with the ground evened all the same; a straight one
    // is made. Without a flag at the start nothing is numbered.
    // spec: TRN-ROAD-3.3a, TRN-ROAD-3.3, SIM-START-4.4, AI-CON-8.1
    #[test]
    fn a_refused_planned_road_evens_the_ground() {
        let mut terrain = flat(20, 20);
        let (columns, rows) = terrain.sample_size();
        // A slope: each sample column 100 higher than the one before.
        terrain.samples = (0..columns * rows)
            .map(|i| ((i % columns) * 100) as i32)
            .collect();
        let mut world = world(terrain);
        let (a, b, c, d) = ((4, 8), (6, 8), (12, 8), (14, 8));
        for f in [a, b, c, d] {
            assert_eq!(flag(&mut world, f), Ok(()));
        }
        for y in 3..=13 {
            let index = world.terrain.index((5, y));
            world.terrain.states[index] |= state::STREET;
        }
        let (path, _) = world.find_route(P0, a, b).unwrap();
        assert!(path.len() - 1 > 4, "{path:?}");
        let mut expected = world.clone();
        expected.even_road(&path);
        assert_ne!(expected.terrain.samples, world.terrain.samples);
        let number = world.next_object;
        let plan = |from, to| Command::PlanRoad {
            player: P0,
            from,
            to,
        };
        assert_eq!(
            apply(&mut world, plan(a, b)),
            Err(Rejection::PlannedRoadTooLong)
        );
        assert_eq!(world.next_object, number + 1);
        assert_eq!(world.terrain.samples, expected.terrain.samples);
        assert_eq!(world.roads().count(), 0);
        assert_eq!(apply(&mut world, plan(c, d)), Ok(()));
        assert_eq!(world.roads().count(), 1);
        let number = world.next_object;
        assert_eq!(
            apply(&mut world, plan((9, 9), d)),
            Err(Rejection::NoFlagAtStart)
        );
        assert_eq!(world.next_object, number);
    }

    // TRN-ROAD-3.3a: a route being built evens the ground as each stretch
    // is added (the route's first field counting as the first field), and
    // cancelling the route does not undo it.
    // spec: TRN-ROAD-3.3a, TRN-ROAD-8
    #[test]
    fn a_route_evens_the_ground_as_it_grows() {
        let mut terrain = flat(20, 20);
        let (columns, rows) = terrain.sample_size();
        terrain.samples = (0..columns * rows)
            .map(|i| ((i % columns) * 100) as i32)
            .collect();
        let mut world = world(terrain);
        let a = (4, 8);
        assert_eq!(flag(&mut world, a), Ok(()));
        let start = Command::StartRoute {
            player: P0,
            at: a,
            waterway: false,
        };
        assert_eq!(apply(&mut world, start), Ok(()));
        let before = world.terrain.samples.clone();
        let to = (9, 8);
        assert_eq!(
            apply(&mut world, Command::ExtendRoute { player: P0, to }),
            Ok(())
        );
        let path = world.routes().next().unwrap().1.fields.clone();
        let mut expected = world.clone();
        expected.terrain.samples = before.clone();
        expected.even_steps(&path, a);
        assert_ne!(world.terrain.samples, before);
        assert_eq!(world.terrain.samples, expected.terrain.samples);
        let evened = world.terrain.samples.clone();
        assert_eq!(
            apply(&mut world, Command::CancelRoute { player: P0 }),
            Ok(())
        );
        assert_eq!(world.terrain.samples, evened);
    }

    // TRN T8
    // spec: TRN-ROAD-2.2 (TRN T8)
    #[test]
    fn step_costs_by_height_difference() {
        let mut terrain = flat(6, 6);
        let (a, b) = ((2, 2), (3, 2));
        let mut cost_for = |difference: i32| {
            set_height(&mut terrain, b, difference);
            let world = world(terrain.clone());
            (world.step_cost(a, b), world.step_cost(b, a))
        };
        assert_eq!(cost_for(0), (6, 6));
        assert_eq!(cost_for(201), (7, 7));
        assert_eq!(cost_for(600), (7, 7));
        assert_eq!(cost_for(601), (8, 8));
        assert_eq!(cost_for(1200), (8, 8));
        assert_eq!(cost_for(1201), (10, 10));
        assert_eq!(cost_for(-2400), (10, 10));
    }

    // TRN T9
    // spec: TRN-ROAD-2.2 (TRN T9)
    #[test]
    fn harbor_ground_doubles_the_step() {
        let mut terrain = flat(6, 6);
        set(
            &mut terrain,
            (3, 2),
            state::FOR_BUILDING | state::FOR_HARBOR,
        );
        let world = world(terrain);
        assert_eq!(world.step_cost((2, 2), (3, 2)), 12);
        assert_eq!(world.step_cost((3, 2), (2, 2)), 6);
    }

    // TRN T17: two routes of cost 18; at (8,9) both (9,9) and (9,10) are
    // candidates and the previous direction 5 is kept.
    // spec: TRN-ROAD-2.4 (TRN T17)
    #[test]
    fn trn_t17_the_read_out_keeps_its_direction() {
        let mut terrain = flat(16, 16);
        set_height(&mut terrain, (9, 8), 300);
        let world = world(terrain);
        let (path, cost) = world.find_route(P0, (8, 8), (10, 10)).unwrap();
        assert_eq!(path, [(8, 8), (8, 9), (9, 10), (10, 10)]);
        assert_eq!(cost, 18);
    }

    // TRN T18: the search reaches S over the steep field first and stops
    // there, so the road costs 20 where a detour of 18 exists.
    // spec: TRN-ROAD-2.1, TRN-ROAD-2.3, TRN-ROAD-2.4 (TRN T18)
    #[test]
    fn trn_t18_the_search_stops_at_the_first_label_of_the_start() {
        let mut terrain = flat(16, 16);
        set_height(&mut terrain, (9, 8), 1300);
        let world = world(terrain);
        assert_eq!(world.step_cost((8, 8), (9, 8)), 10);
        let (path, cost) = world.find_route(P0, (8, 8), (10, 8)).unwrap();
        assert_eq!(path, [(8, 8), (9, 8), (10, 8)]);
        assert_eq!(cost, 20);
        let detour = [(8, 8), (8, 7), (9, 7), (10, 8)];
        let detour_cost: u32 = detour.windows(2).map(|w| world.step_cost(w[1], w[0])).sum();
        assert_eq!(detour_cost, 18);
    }

    // spec: TRN-ROAD-3.1 (TRN-ROAD T26), TRN-ROAD-6.1
    #[test]
    fn a_road_between_two_flags() {
        let mut world = world(flat(12, 12));
        assert_eq!(flag(&mut world, (2, 5)), Ok(()));
        assert_eq!(flag(&mut world, (7, 5)), Ok(()));
        assert_eq!(road(&mut world, (2, 5), (7, 5)), Ok(()));
        let road = world.roads().next().unwrap().clone();
        assert_eq!(road.path.len(), 6);
        for &inner in &road.path[1..5] {
            assert!(world.terrain.has(inner, state::STREET));
        }
        // TRN-ROAD-3.1 (T26): a flag with a road attached carries Street
        // too, and loses it with its last road.
        assert!(world.terrain.has((2, 5), state::STREET));
        let removed = Command::RemoveRoad {
            player: P0,
            at: road.path[2],
        };
        assert_eq!(apply(&mut world, removed), Ok(()));
        assert!(!world.terrain.has((2, 5), state::STREET));
        assert!(world.flag_at((2, 5)).is_some());
    }

    // GFX-OBJ-1.4a: a road's fields (Street) remove the doodads that delete
    // by street, its flags every doodad; they stay removed when the road
    // and a flag go. A field nothing came over keeps its doodads.
    // spec: GFX-OBJ-1.4a, TRN-ROAD-3.1
    #[test]
    fn a_road_and_its_flags_remove_doodads_for_good() {
        use crate::{DOODADS_ALL, DOODADS_BY_STREET};
        let mut world = world(flat(12, 12));
        assert_eq!(flag(&mut world, (2, 5)), Ok(()));
        assert_eq!(flag(&mut world, (7, 5)), Ok(()));
        assert_eq!(road(&mut world, (2, 5), (7, 5)), Ok(()));
        let road = world.roads().next().unwrap().clone();
        let removed = world.doodads_removed().clone();
        for &inner in &road.path[1..5] {
            assert_eq!(removed.get(&inner), Some(&DOODADS_BY_STREET), "{inner:?}");
        }
        assert_eq!(
            removed.get(&(2, 5)),
            Some(&(DOODADS_ALL | DOODADS_BY_STREET))
        );
        assert_eq!(removed.get(&(4, 7)), None);
        let gone = Command::RemoveRoad {
            player: P0,
            at: road.path[2],
        };
        assert_eq!(apply(&mut world, gone), Ok(()));
        assert!(world.roads().next().is_none());
        assert_eq!(world.doodads_removed(), &removed);
        // Play test 16, item 46: saved and loaded, they stay gone.
        let loaded = World::from_snapshot(&world.snapshot()).unwrap();
        assert_eq!(loaded.doodads_removed(), &removed);
        // A game saved before the record: the fields as they stand.
        let mut older = world.clone();
        older.doodads_removed_from_state();
        assert_eq!(older.doodads_removed().get(&(2, 5)), Some(&DOODADS_ALL));
        assert_eq!(older.doodads_removed().get(&road.path[2]), None);
    }

    // spec: TRN-ROAD-1.1, ECO-PLACE-2.2
    #[test]
    fn roads_need_flags_at_both_ends() {
        let mut world = world(flat(12, 12));
        assert_eq!(flag(&mut world, (2, 5)), Ok(()));
        assert_eq!(
            road(&mut world, (2, 5), (7, 5)),
            Err(Rejection::NoFlagAtTarget)
        );
        assert_eq!(
            road(&mut world, (7, 5), (2, 5)),
            Err(Rejection::NoFlagAtStart)
        );
        assert_eq!(road(&mut world, (2, 5), (2, 5)), Err(Rejection::SameFlag));
        assert_eq!(
            road(&mut world, (2, 5), (20, 5)),
            Err(Rejection::OutsideMap)
        );
    }

    // TRN T7: a road may not cross another unless a flag is placed on the
    // crossing field first.
    // spec: TRN-ROAD-1.2 (TRN T7)
    #[test]
    fn roads_do_not_cross() {
        // A corridor one row high: the only way from west to east is along it.
        let mut terrain = flat(13, 9);
        for field in terrain.fields().collect::<Vec<_>>() {
            let walled = field.1 != 4 && field.0 != 6;
            if walled {
                set(&mut terrain, field, state::FOR_BUILDING | state::BLOCKED);
            }
        }
        let mut world = world(terrain);
        // North-south road along column 6.
        for f in [(6, 0), (6, 8), (0, 4), (12, 4)] {
            assert_eq!(flag(&mut world, f), Ok(()), "{f:?}");
        }
        assert_eq!(road(&mut world, (6, 0), (6, 8)), Ok(()));
        // It crosses the corridor on one field ((6,4) or a neighbour on
        // row 4, by the read-out of TRN-ROAD-2.4).
        let path = world.roads().next().unwrap().path.clone();
        let on_row: Vec<Point> = path.iter().copied().filter(|f| f.1 == 4).collect();
        let [crossing] = on_row[..] else {
            panic!("one field on row 4: {path:?}");
        };
        assert!(world.terrain.has(crossing, state::STREET));
        // West-east would have to cross it there.
        assert_eq!(road(&mut world, (0, 4), (12, 4)), Err(Rejection::NoRoute));
        // TRN T11: a flag on the crossing field splits the first road...
        assert_eq!(flag(&mut world, crossing), Ok(()));
        assert_eq!(world.roads().count(), 2);
        // TRN-ROAD-3.1 (T26): the new flag has the two halves attached.
        assert!(world.terrain.has(crossing, state::STREET));
        // ...and both new roads end there.
        for road in world.roads() {
            assert!(road.path.first() == Some(&crossing) || road.path.last() == Some(&crossing));
        }
        // Now the west-east road can be built in two parts.
        assert_eq!(road(&mut world, (0, 4), crossing), Ok(()));
        assert_eq!(road(&mut world, crossing, (12, 4)), Ok(()));
    }

    // TRN T11, T12; SIM-START-4.4 j: the second half runs from the old
    // road's second flag to the new flag.
    // spec: TRN-ROAD-4.1, TRN-ROAD-4.2 (TRN T11, T12), SIM-START-4.4
    #[test]
    fn flags_on_roads() {
        let mut world = world(flat(14, 10));
        assert_eq!(flag(&mut world, (2, 4)), Ok(()));
        assert_eq!(flag(&mut world, (8, 4)), Ok(()));
        assert_eq!(road(&mut world, (2, 4), (8, 4)), Ok(()));
        let path = world.roads().next().unwrap().path.clone();
        assert_eq!(path.len(), 7);
        // T12: next to an end flag: refused.
        assert_eq!(flag(&mut world, path[1]), Err(Rejection::FlagTooClose));
        // T11: two fields from both ends... (5,4) is three from (2,4) and three
        // from (8,4); (4,4) is two from (2,4) and four from (8,4).
        assert_eq!(flag(&mut world, (4, 4)), Ok(()));
        let mut paths: Vec<Vec<Point>> = world.roads().map(|r| r.path.clone()).collect();
        paths.sort();
        let mut second = path[2..].to_vec();
        second.reverse();
        assert_eq!(paths, [path[..=2].to_vec(), second]);
        for &inner in &path[3..6] {
            let road = world.road_at(inner).unwrap();
            assert_eq!(road.path.last(), Some(&(4, 4)));
        }
    }

    // TRN-ROAD-1.1: roads avoid buildings, trees and water.
    #[test]
    fn roads_go_around_obstacles() {
        let mut terrain = flat(12, 10);
        for y in 0..8 {
            set(
                &mut terrain,
                (6, y),
                state::FOR_BUILDING | state::DEPOSIT_BLOCKED,
            );
        }
        let mut world = world(terrain);
        assert_eq!(flag(&mut world, (3, 2)), Ok(()));
        assert_eq!(flag(&mut world, (9, 2)), Ok(()));
        assert_eq!(road(&mut world, (3, 2), (9, 2)), Ok(()));
        let road = world.roads().next().unwrap();
        assert!(road.path.iter().all(|&(x, y)| x != 6 || y >= 8));
    }

    // spec: none (determinism of roads: same commands, same world)
    #[test]
    fn same_commands_give_the_same_world() {
        let run = || {
            let mut world = world(flat(14, 14));
            for command in [
                Command::PlaceFlag {
                    player: P0,
                    at: (2, 2),
                },
                Command::PlaceFlag {
                    player: P0,
                    at: (10, 9),
                },
                Command::BuildRoad {
                    player: P0,
                    from: (2, 2),
                    to: (10, 9),
                },
                Command::PlaceBuilding {
                    player: P0,
                    building: MEDIUM,
                    at: (6, 11),
                },
            ] {
                world.submit(command);
            }
            world.step();
            world
        };
        assert_eq!(run(), run());
    }
}
