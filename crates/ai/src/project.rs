//! Hook W25 (`docs/extending.md`, "The computer players and a project's
//! rules"): the computer players use a project's rules when its overlay
//! says when (`reborn.ai_setDeepen`, `reborn.ai_setPaving`,
//! `reborn.ai_setRecipeRule`, `AiConfig::project_rules`). None of this runs
//! for the original's rules, which have none: their game is the original's.

use s2_sim::Command;

use crate::player::{AiPlayer, Ctx};

/// W25: how often (counts) a computer player looks at its roads and its
/// buildings with a recipe rule; the shift of its slot as every part's.
pub const PROJECT_PERIOD: u64 = 97;

/// W25: what the destruction run does with an exhausted mine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineFate {
    /// As the original's computer player (AI-CON-9.3).
    TearDown,
    /// Kept for a later deepening, or while one is under way.
    Keep,
    /// `Command::DeepenMine` now.
    Deepen,
}

/// What the deepening rule does with an exhausted mine the destruction run
/// would tear down: `food_at_least` the rule (none: no rule), `left` the
/// mine's type has a deepening and the mine has deepenings left, `under_way`
/// one of it is being done, `food` the player's fish, meat and bread in
/// stock, `other_under_way` another mine of his is being deepened, `can`
/// `World::can_deepen`.
pub fn mine_fate(
    food_at_least: Option<u32>,
    left: bool,
    under_way: bool,
    food: u32,
    other_under_way: bool,
    can: bool,
) -> MineFate {
    let Some(at_least) = food_at_least else {
        return MineFate::TearDown;
    };
    if under_way {
        return MineFate::Keep;
    }
    if !left {
        return MineFate::TearDown;
    }
    if can && !other_under_way && food >= at_least {
        MineFate::Deepen
    } else {
        MineFate::Keep
    }
}

/// One of the player's roads as the paving rule sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadCase {
    pub number: u32,
    /// TRN-CARRY-6.1's traffic average.
    pub traffic: f32,
    /// The paving good its pavement takes.
    pub cost: u32,
    /// `World::can_pave` through one of its inner fields.
    pub pavable: bool,
}

/// The road the paving rule paves now, if any: none while one of the
/// player's roads is being paved; else the pavable road of the highest
/// traffic average (the lowest number of equal ones) when that average is
/// at least `traffic_at_least` and the stock of the good is more than
/// `reserve` plus its cost.
pub fn road_to_pave(
    roads: &[RoadCase],
    stock: u32,
    traffic_at_least: u32,
    reserve: u32,
    any_under_way: bool,
) -> Option<u32> {
    if any_under_way {
        return None;
    }
    let best = roads
        .iter()
        .filter(|r| r.pavable && r.traffic >= traffic_at_least as f32)
        .min_by(|a, b| {
            b.traffic
                .total_cmp(&a.traffic)
                .then(a.number.cmp(&b.number))
        })?;
    (u64::from(stock) > u64::from(reserve) + u64::from(best.cost)).then_some(best.number)
}

/// The recipe a recipe rule wants: `index` while the player holds more
/// than `above`, else the first.
pub fn wanted_recipe(have: u32, above: u32, index: u32) -> u32 {
    if have > above {
        index
    } else {
        0
    }
}

impl AiPlayer {
    /// W25: the paving and recipe rules, every [`PROJECT_PERIOD`] counts
    /// (the deepening rule runs in the destruction run, `building_destruction`).
    pub(crate) fn project_rules(&mut self, ctx: &mut Ctx) {
        self.paving_rule(ctx);
        self.recipe_rules(ctx);
    }

    fn paving_rule(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let (Some((traffic_at_least, reserve)), Some(rule)) =
            (ctx.config.project_rules.paving, world.paving_rule())
        else {
            return;
        };
        let own: Vec<_> = world
            .roads()
            .filter(|r| r.owner == self.id && !r.water)
            .collect();
        let any_under_way = own.iter().any(|r| world.paving_work(r.number).is_some());
        let inner = |path: &[s2_sim::grid::Point]| path.get(1).filter(|_| path.len() > 2).copied();
        let cases: Vec<RoadCase> = own
            .iter()
            .map(|r| RoadCase {
                number: r.number,
                traffic: r.traffic.average(),
                cost: rule.per_field.saturating_mul(r.path.len() as u32),
                pavable: inner(&r.path).is_some_and(|at| world.can_pave(self.id, at)),
            })
            .collect();
        let stock = self.econ.stock(rule.good);
        let Some(number) = road_to_pave(&cases, stock, traffic_at_least, reserve, any_under_way)
        else {
            return;
        };
        if let Some(at) = own
            .iter()
            .find(|r| r.number == number)
            .and_then(|r| inner(&r.path))
        {
            ctx.out.push(Command::PaveRoad {
                player: self.id,
                at,
            });
        }
    }

    fn recipe_rules(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let cfg = ctx.config;
        // The last rule given for a type wins.
        let mut rules: Vec<s2_rules::model::AiRecipeRule> = Vec::new();
        for r in &cfg.project_rules.recipes {
            rules.retain(|k| k.building != r.building);
            rules.push(*r);
        }
        for rule in rules {
            let Some(set) = world.recipe_set(rule.building) else {
                continue;
            };
            if rule.index as usize >= set.len() {
                continue;
            }
            // A group good counts its members.
            let have: u32 = match cfg.sim.recipe_groups.get(&rule.good) {
                Some(members) => members.iter().map(|&g| self.econ.stock(g)).sum(),
                None => self.econ.stock(rule.good),
            };
            let wanted = wanted_recipe(have, rule.above, rule.index);
            let mine: Vec<_> = world
                .buildings()
                .filter(|b| {
                    b.owner == self.id && b.building_type == rule.building && !b.under_construction
                })
                .map(|b| b.at)
                .collect();
            for at in mine {
                if world.chosen_recipe(at).is_some_and(|c| c != wanted) {
                    ctx.out.push(Command::ChooseRecipe {
                        player: self.id,
                        at,
                        index: wanted,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // W25 (reborn.ai_setDeepen): no rule, the original's teardown; with it
    // a mine with deepenings left is deepened when the food is there and
    // no other is under way, else kept; one being deepened is kept; one
    // at its most deepenings is torn down.
    // spec: none (hook W25, a project's rule for the computer players)
    #[test]
    fn the_deepening_rule_decides_an_exhausted_mines_fate() {
        use MineFate::*;
        assert_eq!(mine_fate(None, true, false, 50, false, true), TearDown);
        assert_eq!(mine_fate(Some(6), true, false, 6, false, true), Deepen);
        assert_eq!(mine_fate(Some(6), true, false, 5, false, true), Keep);
        assert_eq!(mine_fate(Some(6), true, false, 9, true, true), Keep);
        assert_eq!(mine_fate(Some(6), true, false, 9, false, false), Keep);
        assert_eq!(mine_fate(Some(6), true, true, 9, false, false), Keep);
        assert_eq!(mine_fate(Some(6), false, false, 9, false, false), TearDown);
    }

    // W25 (reborn.ai_setPaving): the busiest pavable road at or above the
    // traffic, while the stock is more than the reserve plus its cost; one
    // at a time; ties to the lower number.
    // spec: none (hook W25, a project's rule for the computer players)
    #[test]
    fn the_paving_rule_picks_the_busiest_road() {
        let road = |number, traffic, cost, pavable| RoadCase {
            number,
            traffic,
            cost,
            pavable,
        };
        let roads = [
            road(7, 4.0, 5, true),
            road(3, 6.5, 4, false),
            road(9, 5.0, 6, true),
            road(2, 5.0, 6, true),
            road(4, 1.0, 1, true),
        ];
        assert_eq!(road_to_pave(&roads, 17, 3, 10, false), Some(2));
        // The stock must be more than reserve + cost.
        assert_eq!(road_to_pave(&roads, 16, 3, 10, false), None);
        assert_eq!(road_to_pave(&roads, 100, 3, 10, true), None);
        assert_eq!(road_to_pave(&roads, 100, 6, 10, false), None);
        assert_eq!(road_to_pave(&[], 100, 0, 0, false), None);
    }

    // W25 (reborn.ai_setPaving) on the AI fixture: with a paving rule in
    // the game and the computer player's rule, a stock of stone above the
    // reserve plus the cost has its road paved through an inner field;
    // without the computer player's rule nothing is sent.
    // spec: none (hook W25, a project's rule for the computer players)
    #[test]
    fn a_computer_player_paves_its_road_on_the_fixture() {
        use crate::testutil::*;
        use crate::AiSystem;
        use s2_sim::PlayerId;
        let p0 = PlayerId(0);
        let mut rules = rules();
        rules.paving = Some(s2_sim::PavingRule {
            good: STONE,
            per_field: 1,
            speed_percent: 130,
        });
        let mut world = world_on(1, &[0], &Extras::default(), rules.clone());
        for at in [(24, 20), (28, 20)] {
            world.submit(Command::PlaceFlag { player: p0, at });
        }
        world.submit(Command::BuildRoad {
            player: p0,
            from: (24, 20),
            to: (28, 20),
        });
        world.step();
        let road = world
            .roads()
            .find(|r| r.path.first() == Some(&(24, 20)))
            .expect("the road")
            .clone();
        let run = |paving: Option<(u32, u32)>, stone: u32| {
            let mut config = config_for(&rules);
            config.project_rules.paving = paving;
            let mut ai = AiSystem::new(config, &world);
            assert!(ai.activate_with(&world, &mut s2_sim::Numbers::new(0), p0, 2));
            ai.player_mut(p0).econ = crate::view::Economy::from_counts(&[(STONE, stone)], &[]);
            ai.with_ctx(&world, p0, |p, ctx| p.project_rules(ctx)).1
        };
        let cost = road.path.len() as u32;
        let paves = |c: &[Command]| {
            c.iter()
                .any(|c| matches!(c, Command::PaveRoad { at, .. } if road.path[1..road.path.len() - 1].contains(at)))
        };
        assert!(paves(&run(Some((0, 10)), 11 + cost)));
        assert!(!paves(&run(Some((0, 10)), 10 + cost)));
        assert!(run(None, 100).is_empty());
    }

    // W25 (reborn.ai_setRecipeRule): the rule's recipe above the stock,
    // else the first.
    // spec: none (hook W25, a project's rule for the computer players)
    #[test]
    fn the_recipe_rule_follows_the_stock() {
        assert_eq!(wanted_recipe(41, 40, 1), 1);
        assert_eq!(wanted_recipe(40, 40, 1), 0);
        assert_eq!(wanted_recipe(0, 0, 2), 0);
    }
}
