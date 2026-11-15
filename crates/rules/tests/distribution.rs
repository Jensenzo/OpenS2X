//! ECO-DIST on a real game with the original rules of the player's install
//! (`S2_GAME_DIR`; skipped when unset): the default tables and ranks come
//! out of the scripts and the commands act on them.

use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::{Command, PlayerId, World};

const P0: PlayerId = PlayerId(0);

fn game() -> Option<(s2_rules::RuleSet, World, s2_sim::grid::Point)> {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let bytes = std::fs::read(dir.root().join(rel)).expect("map file");
    let map = s2m::load(rel, &bytes).expect("map");
    let hq = (map.info.start_positions[0].x, map.info.start_positions[0].y);
    let world = s2_rules::new_game(7, &map, &rules, 0).expect("game");
    Some((rules, world, hq))
}

// spec: ECO-DIST-2.1, ECO-DIST-4.1
#[test]
fn default_tables_and_ranks_come_from_the_scripts() {
    let Some((rules, world, _)) = game() else {
        return;
    };
    let id = |name: &str| rules.id_named(name).unwrap_or_else(|| panic!("{name}"));
    let tables = world.distribution(P0);
    assert_eq!(tables.len(), 6);
    // ECO-DIST-2.1: grain: mill 0.5, piggery 0.3, donkey farm 0.2, brewery 0.3.
    let grain: Vec<(u32, f32)> = tables[0].iter().map(|e| (e.key, e.weight())).collect();
    assert_eq!(
        grain,
        [
            (id("mill"), 0.5),
            (id("piggery"), 0.3),
            (id("donkeyBreeding"), 0.2),
            (id("brewery"), 0.3)
        ]
    );
    // Planks: construction first.
    assert_eq!(tables[3][0].key, id("construction"));
    assert_eq!(tables[3][0].weight(), 1.0);
    // The food table serves fish, meat and bread.
    let food = &world.distribution_rules()[4];
    assert!(["fish", "flesh", "bread"]
        .iter()
        .all(|n| food.goods.contains(&id(n))));
    // ECO-DIST-4.1: Goods.lua ranks.
    let rank = |n: &str| world.rank(P0, id(n));
    assert_eq!(
        [
            rank("coin"),
            rank("plank"),
            rank("stone"),
            rank("wood"),
            rank("water")
        ],
        [0, 7, 8, 9, 10]
    );
}

// spec: ECO-DIST-1.3, ECO-DIST-2.2, ECO-DIST-4.2
#[test]
fn commands_act_on_the_headquarters_and_the_player() {
    let Some((rules, mut world, hq)) = game() else {
        return;
    };
    let plank = rules.id_named("plank").unwrap();
    let wood = rules.id_named("wood").unwrap();
    let click = Command::WarehouseCycle {
        player: P0,
        at: hq,
        good: plank,
    };
    for expect in [(true, false), (false, true), (false, false)] {
        world.submit(click.clone());
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()));
        let f = world.warehouse(hq).unwrap().flags_of(plank);
        assert_eq!((f.stop, f.out), expect);
    }
    // All wares out: every ware good has a record, settlers none.
    world.submit(Command::WarehouseAll {
        player: P0,
        at: hq,
        mode: 2,
        settlers: false,
    });
    world.step();
    // Q-033: goods without a type are wares.
    let wares = rules.wares().count() + rules.untyped_goods().count();
    assert_eq!(world.warehouse(hq).unwrap().flags.len(), wares);
    assert!(world.warehouse(hq).unwrap().flags_of(wood).out);
    // Move wood to the front.
    world.submit(Command::PriorityMove {
        player: P0,
        from: 9,
        to: 0,
    });
    world.step();
    assert_eq!(world.rank(P0, wood), 0);
    assert_eq!(world.rank(PlayerId(1), wood), 9);
}
