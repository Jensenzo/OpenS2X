//! The destruction system (AI-CON-9): tearing down useless buildings and
//! flags.

use s2_sim::Command;

use crate::player::{AiPlayer, Ctx};
use crate::sites::SiteEnv;
use crate::view::flag_of;

impl AiPlayer {
    /// AI-CON-9.2 (every 41 ticks): the next building in id order; resource
    /// buildings and hunters are torn down when their site has lost its value.
    pub(crate) fn building_destruction(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let cfg = ctx.config;
        let n = &cfg.names;
        // AI-2.6 (T41): "id order" is the order the buildings were created
        // in; the cursor holds the last handled id + 1, and a run that finds
        // no building from it on only resets it.
        let mut records: Vec<_> = self.records.values().collect();
        records.sort_by_key(|r| r.serial);
        let Some(rec) = records
            .iter()
            .find(|r| r.serial >= self.destruction_cursor)
            .copied()
            .cloned()
        else {
            self.destruction_cursor = 0;
            return;
        };
        self.destruction_cursor = rec.serial.wrapping_add(1);
        // AI-CON-9.1.
        if rec.protect.any() {
            return;
        }
        let Some(t) = world.building_type(rec.building_type) else {
            return;
        };
        let resource_product = t.product.is_some_and(|g| cfg.is_resource_good(g));
        if !resource_product && rec.building_type != n.hunter {
            return;
        }
        let Some(b) = ctx.snapshot.building(self.id, rec.at) else {
            return;
        };
        // The building is unfinished or its worker is at home (T41).
        if !(b.under_construction || ctx.snapshot.at_home.contains(&rec.at)) {
            return;
        }
        let env = SiteEnv {
            world,
            config: cfg,
            player: self.id,
            own: ctx.snapshot.of(self.id),
        };
        let value = env.value(t, rec.at);
        let fisher_without_catch = rec.building_type == n.fisher && b.productivity == 0;
        let candidate = value <= 0.0 || fisher_without_catch;
        if !candidate {
            return;
        }
        // A woodcutter is kept only if its cell has a forester and exactly
        // one wood production (its good wood or of wood's group).
        if rec.building_type == n.woodcutter {
            let forester = self
                .cells
                .get(rec.cell)
                .is_some_and(|c| c.forester.is_some());
            let wood = self
                .plan
                .prods
                .iter()
                .filter(|p| p.cell == rec.cell && cfg.supplies(p.good, n.wood))
                .count();
            if forester && wood == 1 {
                return;
            }
        }
        // Hook W25 (reborn.ai_setDeepen): an exhausted mine of a type with
        // a deepening is deepened, or kept for one, instead (none in the
        // original's rules: torn down as below).
        if let Some(deepening) = world.deepening(rec.building_type) {
            let done = world.mine_deepening(rec.at);
            let other_under_way = self
                .records
                .values()
                .any(|r| r.at != rec.at && world.mine_deepening(r.at).is_some_and(|d| d.under_way));
            let n = &cfg.names;
            let food = [n.fish, n.flesh, n.bread]
                .iter()
                .map(|&g| self.econ.stock(g))
                .sum();
            let fate = crate::project::mine_fate(
                cfg.project_rules.deepen_food,
                done.map_or(0, |d| d.done) < deepening.most,
                done.is_some_and(|d| d.under_way),
                food,
                other_under_way,
                world.can_deepen(self.id, rec.at),
            );
            match fate {
                crate::project::MineFate::TearDown => {}
                crate::project::MineFate::Keep => return,
                crate::project::MineFate::Deepen => {
                    ctx.out.push(Command::DeepenMine {
                        player: self.id,
                        at: rec.at,
                    });
                    return;
                }
            }
        }
        // AI-CON-9.3: torn down with remains (T41).
        ctx.out.push(Command::TearDown {
            player: self.id,
            at: rec.at,
        });
    }

    /// AI-CON-9.4 (every 59 ticks): in every cell with the clearing marker
    /// the first ordinary building of the marker's size is torn down.
    pub(crate) fn cell_clearing(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let marked: Vec<_> = self
            .cells
            .iter()
            .filter_map(|c| c.clearing.map(|(_, size)| (c.pos, size)))
            .collect();
        for (pos, size) in marked {
            let victim = self.cells.get(pos).and_then(|c| {
                c.buildings.iter().copied().find(|at| {
                    self.records
                        .get(at)
                        .is_some_and(|r| !r.protect.depot && !r.protect.military)
                        && world.building_at(*at).is_some_and(|b| b.size == size)
                })
            });
            if let Some(at) = victim {
                // AI-CON-9.3: torn down with remains.
                ctx.out.push(Command::TearDown {
                    player: self.id,
                    at,
                });
                if let Some(c) = self.cells.get_mut(pos) {
                    c.clearing = None;
                }
            }
        }
    }

    /// AI-CON-9.5 (every tick that is not a multiple of 41 or 59): the next
    /// flag of the player in list order; a flag with no building, no wares and
    /// no road, or with a single road whose carrier or donkey carries nothing,
    /// is removed.
    pub(crate) fn flag_destruction(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let terrain = world.terrain();
        let flags = world.flags_in_order(self.id);
        // T50: the list as it is now, with the flags this think's commands
        // made (they act at once in the original, AI-2.1a), appended.
        let mut made: Vec<s2_sim::grid::Point> = Vec::new();
        for command in ctx.out.iter() {
            let field = match *command {
                Command::PlanBuilding { player, at, .. } if player == self.id => {
                    terrain.neighbour(at, crate::view::FLAG_DIRECTION)
                }
                Command::PlaceFlag { player, at } if player == self.id => Some(at),
                _ => None,
            };
            if let Some(f) = field.filter(|&f| world.flag_at(f).is_none() && !made.contains(&f)) {
                made.push(f);
            }
        }
        let len = flags.len() + made.len();
        if len == 0 {
            return;
        }
        // A cursor at or past the end starts again at the first flag; the
        // flag at it is tested, then it moves on by one.
        let index = match self.flag_cursor as usize {
            i if i >= len => 0,
            i => i,
        };
        self.flag_cursor = (index + 1) as u32;
        // A flag made in this think is not tested.
        let Some(&flag) = flags.get(index) else {
            return;
        };
        let has_building = terrain
            .neighbour(flag, 4)
            .and_then(|b| world.building_at(b))
            .is_some_and(|b| b.owner == self.id && flag_of(world, b.at) == Some(flag));
        if has_building || !world.wares_at(flag).is_empty() {
            return;
        }
        let roads: Vec<&s2_sim::Road> = world
            .roads()
            .filter(|r| r.owner == self.id)
            .filter(|r| r.path.first() == Some(&flag) || r.path.last() == Some(&flag))
            .collect();
        // AI-CON-9.5 (corrected, T47): one road keeps the flag only while its
        // carrier or donkey carries a ware (holds one that is not lying); an
        // assigned carrier walking empty does not.
        let carrying = |carrier: Option<usize>| {
            carrier
                .and_then(|c| world.carriers().get(c))
                .and_then(|c| c.ware)
                .and_then(|w| world.ware(w))
                .is_some_and(|w| matches!(w.location, s2_sim::WareLocation::Carried(_)))
        };
        let remove = match roads.as_slice() {
            [] => true,
            [road] => !carrying(road.carrier) && !carrying(road.donkey),
            _ => false,
        };
        if remove {
            ctx.out.push(Command::RemoveFlag {
                player: self.id,
                at: flag,
            });
        }
    }
}
