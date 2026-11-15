//! The military settings panel and the soldier lines of a building
//! (`specs/MIL-soldiers.md` MIL-SOL-7, MIL-SOL-2 to 6,
//! `specs/MIL-coins-promotion.md`). Plain functions of the simulation state,
//! testable without a window.

use s2_sim::grid::Point;
use s2_sim::{Command, PlayerId, World, SOLDIER_RANKS};

/// MIL-SOL-1.1, the dialog's names of the ranks.
pub const RANK_NAMES: [&str; SOLDIER_RANKS] = [
    "private",
    "private first class",
    "sergeant",
    "officer",
    "general",
];

/// The colour of a rank's pip: private grey, then green, blue, purple, gold.
pub fn rank_color(rank: usize) -> [f32; 3] {
    const COLORS: [[f32; 3]; SOLDIER_RANKS] = [
        [0.85, 0.85, 0.85],
        [0.3, 0.9, 0.35],
        [0.3, 0.6, 1.0],
        [0.75, 0.4, 0.95],
        [1.0, 0.8, 0.1],
    ];
    COLORS[rank.min(SOLDIER_RANKS - 1)]
}

/// The entries of the settings panel, in order: the three allocation
/// factors, attack strength, intercepting defenders, defense type and the
/// headquarters' soldier amount per rank.
pub const SETTINGS: usize = 6 + SOLDIER_RANKS;

fn setting_name(index: usize) -> String {
    match index {
        0 => "Allocation Close".into(),
        1 => "Allocation Medium".into(),
        2 => "Allocation Far".into(),
        3 => "Attack Strength".into(),
        4 => "Intercepting Defenders".into(),
        5 => "Defense type".into(),
        n => format!("Headquarters keeps: {}", RANK_NAMES[n - 6]),
    }
}

/// The headquarters the castle amounts act on: the building under the
/// cursor if it is the player's, else the player's first warehouse that
/// keeps soldiers.
pub fn headquarters_target(
    world: &World,
    player: PlayerId,
    hovered: Option<Point>,
) -> Option<Point> {
    let is_own = |at: Point| {
        world
            .building_at(at)
            .is_some_and(|b| b.owner == player && !b.under_construction)
            && world.stock(at).is_some()
            && world.military_at(at).is_some()
    };
    hovered
        .filter(|&at| is_own(at))
        .or_else(|| world.warehouses().map(|(at, _)| at).find(|&at| is_own(at)))
}

/// The lines of the panel with `selected` marked.
pub fn settings_lines(
    world: &World,
    player: PlayerId,
    selected: usize,
    hovered: Option<Point>,
) -> Vec<String> {
    let settings = world.military_settings(player);
    let hq = headquarters_target(world, player, hovered);
    let mut lines = vec![
        "Military Settings (F1; [ ] choose, U/J +/- 0.1, Enter defense type, Y reset):".into(),
    ];
    for index in 0..SETTINGS {
        let value = match index {
            0..=2 => format!("{:.1}", settings.allocation(index)),
            3 => format!("{:.1}", settings.attack_strength()),
            4 => format!("{:.1}", settings.defenders()),
            5 => if settings.prefer_strong {
                "prefer strong soldiers"
            } else {
                "prefer weak soldiers"
            }
            .to_owned(),
            n => match hq.and_then(|at| world.military_at(at)) {
                Some(m) => m.castle[n - 6].to_string(),
                None => "-".into(),
            },
        };
        let mark = if index == selected { ">" } else { " " };
        lines.push(format!("{mark} {}: {value}", setting_name(index)));
    }
    lines
}

/// The command a step of `steps` tenths (or, for the defense type, a flip)
/// on setting `selected` gives, if it changes anything.
pub fn setting_command(
    world: &World,
    player: PlayerId,
    selected: usize,
    steps: i32,
    hovered: Option<Point>,
) -> Option<Command> {
    let settings = world.military_settings(player);
    let thousandths = |value: f32| {
        ((value * 1000.0).round() as i64 + 100 * i64::from(steps)).clamp(0, 1000) as u32
    };
    match selected {
        0..=2 => Some(Command::MilitaryAllocation {
            player,
            class: selected as u8,
            thousandths: thousandths(settings.allocation(selected)),
        }),
        3 => Some(Command::MilitaryAttackStrength {
            player,
            thousandths: thousandths(settings.attack_strength()),
        }),
        4 => Some(Command::MilitaryDefenders {
            player,
            thousandths: thousandths(settings.defenders()),
        }),
        5 => (steps != 0).then_some(Command::MilitaryDefenseType {
            player,
            strong: !settings.prefer_strong,
        }),
        n => {
            let at = headquarters_target(world, player, hovered)?;
            let rank = n - 6;
            let amount = world.military_at(at)?.castle[rank];
            let amount = (i64::from(amount) + i64::from(steps.signum())).max(0) as u32;
            Some(Command::SetCastleSoldiers {
                player,
                at,
                rank: rank as u8,
                amount,
            })
        }
    }
}

/// MIL-ATK-1.4: the names of the dialog's four count buttons.
pub const COUNT_NAMES: [&str; 4] = ["Low", "Medium", "High", "Highest number of soldiers"];

/// Whether the building on `at` is an enemy military building `player` could
/// open the attack dialog on (MIL-ATK-1.3).
pub fn attackable(world: &World, player: PlayerId, at: Point) -> bool {
    world
        .building_at(at)
        .is_some_and(|b| !b.under_construction && world.are_enemies(player, b.owner))
        && world.military_at(at).is_some()
}

/// The attack dialog on `target` (MIL-ATK-1.3, 1.4).
pub fn attack_lines(world: &World, player: PlayerId, target: Point, strong: bool) -> Vec<String> {
    let available = world.attack_available(player, target);
    let mut lines = vec![format!(
        "Attack ({}, {}): {available} soldiers available",
        target.0, target.1
    )];
    if available == 0 {
        lines.push("  Can not attack this building!".into());
        return lines;
    }
    let counts = world.attack_counts(player, target);
    for (i, (name, n)) in COUNT_NAMES.iter().zip(counts).enumerate() {
        lines.push(format!("  {} {name}: {n}", i + 1));
    }
    lines.push(format!(
        "  F9: {}   0 / right click: close",
        if strong {
            "Use strong units first"
        } else {
            "Use weak units first"
        }
    ));
    lines
}

/// The order a count button gives (`button` 0 to 3).
pub fn attack_command(
    world: &World,
    player: PlayerId,
    target: Point,
    button: usize,
    strong: bool,
) -> Option<Command> {
    let count = *world.attack_counts(player, target).get(button)?;
    (world.attack_available(player, target) > 0).then_some(Command::Attack {
        player,
        target,
        count,
        strong_first: strong,
    })
}

/// The newest messages that are still shown, newest first, as lines.
pub fn message_lines(world: &World, limit: usize) -> Vec<String> {
    let now = world.tick();
    world
        .messages()
        .iter()
        .rev()
        .filter(|m| m.until > now)
        .take(limit)
        .map(|m| {
            format!(
                "  [{}] P{} ({}, {}): {}",
                m.tick, m.player.0, m.at.0, m.at.1, m.text
            )
        })
        .collect()
}

/// MIL-SOL-2 to 4, MIL-PROM-3: the soldiers, wanted number, enemy distance
/// and coins of the military building on `at`.
pub fn building_lines(world: &World, at: Point, coin: Option<u32>) -> Vec<String> {
    let (Some(m), Some(building)) = (world.military_at(at), world.building_at(at)) else {
        return Vec::new();
    };
    let present = |s: &s2_sim::SoldierView| {
        let name = RANK_NAMES[s.rank];
        if s.present {
            format!("{name} {}hp", s.health)
        } else {
            format!("{name} {}hp (walking)", s.health)
        }
    };
    let soldiers: Vec<String> = world.garrison(at).iter().map(present).collect();
    let mut lines = Vec::new();
    let headquarters = world.stock(at).is_some();
    if headquarters {
        let stock = world.stock_soldiers(at);
        lines.push(format!(
            "  soldiers in stock by rank: {}",
            stock.map(|n| n.to_string()).join(" ")
        ));
    } else {
        lines.push(format!(
            "  soldiers {} of {} wanted; enemy distance {}",
            soldiers.len(),
            world.wanted_soldiers(at),
            if m.enemy_distance < 0 {
                "-".to_owned()
            } else {
                m.enemy_distance.to_string()
            }
        ));
    }
    if !soldiers.is_empty() {
        lines.push(format!("  inside: {}", soldiers.join(", ")));
    }
    if !headquarters {
        let coins = coin.map_or(0, |good| {
            world
                .inputs(at)
                .iter()
                .filter(|&&(g, _)| g == good)
                .map(|&(_, n)| n)
                .sum::<u32>()
        });
        lines.push(format!(
            "  coins {coins}   coin supply {}   moving out {}   (F2 / F3)",
            if m.coin_switch { "on" } else { "off" },
            if m.move_out { "on" } else { "off" },
        ));
    }
    if !m.applied && !building.under_construction {
        lines.push("  not yet occupied: no land claimed".into());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the viewer's debug military panel)
    #[test]
    fn rank_colours_are_distinct() {
        for a in 0..SOLDIER_RANKS {
            for b in a + 1..SOLDIER_RANKS {
                assert_ne!(rank_color(a), rank_color(b));
            }
        }
    }

    // spec: MIL-SOL-7.1
    #[test]
    fn settings_have_a_name_each() {
        for index in 0..SETTINGS {
            assert!(!setting_name(index).is_empty());
        }
        assert_eq!(setting_name(6), "Headquarters keeps: private");
    }
}
