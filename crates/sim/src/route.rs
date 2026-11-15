//! The road being built, click by click (`specs/TRN-roads-flags.md`
//! TRN-ROAD-8; the interface's side is UI-GAME-3.4).

use crate::grid::Point;
use crate::map::state;
use crate::{PlayerId, Rejection, World};

/// TRN-ROAD-8.1: a player's route: the fields from the start flag to the
/// current end, and which of them the route itself gave the Street state
/// (a field of another road keeps its own).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub fields: Vec<Point>,
    /// TRN-ROAD-7.1: a waterway's route.
    pub water: bool,
    marked: Vec<bool>,
}

impl Route {
    /// The route's current end.
    pub fn end(&self) -> Point {
        *self.fields.last().expect("a route has its start")
    }
}

impl World {
    /// TRN-ROAD-8: the road `player` is building, if any.
    pub fn route(&self, player: PlayerId) -> Option<&Route> {
        self.routes.get(&player)
    }

    /// TRN-ROAD-8.1 (`start-road`): a route of the one field
    /// of the player's flag on `at`, if the player has none.
    pub(crate) fn start_route(
        &mut self,
        player: PlayerId,
        at: Point,
        water: bool,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(at) != Some(player) {
            return Err(Rejection::NoFlagAtStart);
        }
        // TRN-ROAD-7.1: a waterway starts at a flag on the coast.
        if water && !self.terrain.has(at, state::COAST) {
            return Err(Rejection::NotCoast);
        }
        if self.routes.contains_key(&player) {
            return Err(Rejection::RouteExists);
        }
        self.routes.insert(
            player,
            Route {
                fields: vec![at],
                water,
                marked: vec![false],
            },
        );
        Ok(())
    }

    /// TRN-ROAD-8.2 (`extend-road`): a field of the route cuts
    /// it back to that field; any other field extends it from its current
    /// end along the cheapest route (TRN-ROAD-2, TRN-ROAD-7 for a
    /// waterway), each new field gaining the Street state, if the field may
    /// end the road (7.4) and a route exists. Then 8.3.
    pub(crate) fn extend_route(&mut self, player: PlayerId, to: Point) -> Result<(), Rejection> {
        let route = self.routes.get(&player).ok_or(Rejection::NoRoute)?;
        if !self.terrain.contains(to) {
            return Err(Rejection::OutsideMap);
        }
        if let Some(i) = route.fields.iter().position(|&f| f == to) {
            let mut route = self.routes.remove(&player).expect("checked");
            let dropped: Vec<Point> = route.fields.drain(i + 1..).collect();
            let marks: Vec<bool> = route.marked.drain(i + 1..).collect();
            self.unmark(&dropped, &marks);
            self.routes.insert(player, route);
            return Ok(());
        }
        let (start, end, water, points) = (
            route.fields[0],
            route.end(),
            route.water,
            route.fields.len(),
        );
        self.check_route_end(player, start, end, points, water, to)?;
        let (path, _) = if water {
            self.waterway_search(start, end, to)?
        } else {
            self.find_route(player, end, to)?
        };
        let added = &path[1..];
        let marks: Vec<bool> = added
            .iter()
            .map(|&f| !self.terrain.has(f, state::STREET) && self.flag_at(f).is_none())
            .collect();
        for (&f, &mark) in added.iter().zip(&marks) {
            if mark {
                self.set_bits(f, state::STREET, true);
            }
        }
        self.changed(added);
        if !water {
            // TRN-ROAD-3.3a: the stretch added evens the ground at once,
            // and nothing undoes it when the route is cut or cancelled.
            self.even_steps(&path, start);
        }
        let route = self.routes.get_mut(&player).expect("checked");
        route.fields.extend_from_slice(added);
        route.marked.extend(marks);
        self.complete_route(player);
        Ok(())
    }

    /// TRN-ROAD-7.4: whether `to` may end the road being built from the
    /// route's current `end` (`points` fields from the start flag `start`):
    /// inside the map and not Border, the start not DoodadBlocked; a target
    /// that is not Water must take a flag (or hold one of the player's);
    /// a waterway's length and end rules.
    fn check_route_end(
        &self,
        player: PlayerId,
        start: Point,
        end: Point,
        points: usize,
        water: bool,
        to: Point,
    ) -> Result<(), Rejection> {
        if self.terrain.has(to, state::BORDER) || self.terrain.has(start, state::DOODAD_BLOCKED) {
            return Err(Rejection::TargetNotGeneralSite);
        }
        if !self.terrain.has(to, state::WATER) && self.flag_at(to) != Some(player) {
            self.check_flag(player, to)?;
        }
        if water && !self.waterway_end_allowed(end, points, to) {
            return Err(Rejection::WaterwayTooLong);
        }
        Ok(())
    }

    /// TRN-ROAD-8.3 (`cancel-road`): the route goes and its fields are
    /// cleared.
    pub(crate) fn cancel_route(&mut self, player: PlayerId) -> Result<(), Rejection> {
        let route = self.routes.remove(&player).ok_or(Rejection::NoRoute)?;
        self.unmark(&route.fields, &route.marked);
        Ok(())
    }

    /// The Street state the route gave its fields is taken back.
    fn unmark(&mut self, fields: &[Point], marked: &[bool]) {
        for (&f, &mark) in fields.iter().zip(marked) {
            if mark && self.road_at(f).is_none() {
                self.set_bits(f, state::STREET, false);
            }
        }
        self.changed(fields);
    }

    /// TRN-ROAD-8.3: a complete route (its first field a flag, more than
    /// one field, its last field a flag or a field of another finished land
    /// road where a flag fits) becomes a road: a flag is placed on such a
    /// road field first (splitting that road), the route's fields become
    /// the road, which asks for its carrier, and the route is gone.
    fn complete_route(&mut self, player: PlayerId) {
        let Some(route) = self.routes.get(&player) else {
            return;
        };
        let (first, last) = (route.fields[0], route.end());
        if route.fields.len() < 2 || self.flag_at(first) != Some(player) {
            return;
        }
        let on_road = self.flag_at(last).is_none()
            && self.road_at(last).is_some_and(|r| !r.water)
            && self.check_flag(player, last).is_ok();
        if self.flag_at(last) != Some(player) && !on_road {
            return;
        }
        let route = self.routes.remove(&player).expect("checked");
        if on_road {
            self.put_flag(player, last);
            // ECO-HQ-4.8b: the flag's height ring.
            let touched = self.flag_height_ring(last);
            self.changed(&touched);
        }
        // The end flag's field carries no Street of the route.
        if route.marked.last() == Some(&true) && self.road_at(last).is_none() {
            self.set_bits(last, state::STREET, false);
        }
        self.add_road(player, route.fields.clone(), route.water);
        if !route.water {
            // TRN-ROAD-3.3: a land road evens the ground along it.
            self.even_road(&route.fields);
        }
        self.changed(&route.fields);
    }

    /// TRN-ROAD-8.3, every tick: a route whose start flag is gone, or one
    /// of whose fields can no longer be used, is dropped; a complete one
    /// becomes a road.
    pub(crate) fn update_routes(&mut self) {
        let players: Vec<PlayerId> = self.routes.keys().copied().collect();
        for player in players {
            let route = &self.routes[&player];
            let usable = |f: Point| {
                if route.water {
                    !self.terrain.has(f, state::BORDER | state::DOODAD_BLOCKED)
                } else {
                    self.is_general_site(player, f)
                }
            };
            let broken = self.flag_at(route.fields[0]) != Some(player)
                || route.fields[1..]
                    .iter()
                    .any(|&f| self.flag_at(f).is_none() && !usable(f));
            if broken {
                let _ = self.cancel_route(player);
            } else {
                self.complete_route(player);
            }
        }
    }

    /// TRN-ROAD-8.4: the fields of the routes being built, by player.
    pub fn routes(&self) -> impl Iterator<Item = (PlayerId, &Route)> {
        self.routes.iter().map(|(&p, r)| (p, r))
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
crate::snap_struct!(Route {
    fields,
    water,
    marked
});
