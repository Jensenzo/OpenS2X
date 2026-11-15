//! The ship and expedition controls of the viewer (`specs/SHIP-ships-expeditions.md`).

use s2_sim::{Command, ExpeditionState, PlayerId, ShipState, World};

/// The ships of `player` that are finished, as indices into `World::ships`.
pub fn fleet(world: &World, player: PlayerId) -> Vec<usize> {
    world
        .ships()
        .iter()
        .enumerate()
        .filter(|(_, s)| s.owner == player && s.vessel.launched)
        .map(|(i, _)| i)
        .collect()
}

/// The ship `choice` picks among the player's fleet.
pub fn chosen(world: &World, player: PlayerId, choice: usize) -> Option<usize> {
    let fleet = fleet(world, player);
    (!fleet.is_empty()).then(|| fleet[choice % fleet.len()])
}

/// Start an expedition at the harbour under the cursor.
pub fn start_command(world: &World, player: PlayerId, at: Option<(u32, u32)>) -> Option<Command> {
    let at = at.filter(|&f| {
        world.harbour_at(f).is_some() && world.building_at(f).is_some_and(|b| b.owner == player)
    })?;
    Some(Command::ExpeditionStart { player, at })
}

fn state_name(state: ShipState) -> &'static str {
    match state {
        ShipState::Idle => "idle",
        ShipState::Search => "search",
        ShipState::ToHarbour => "to harbour",
        ShipState::Docked => "docked",
        ShipState::Waiting => "waiting for orders",
        ShipState::Sailing => "sailing",
        ShipState::Unloading => "unloading",
        ShipState::MakingRoom => "making room",
        ShipState::Returning => "returning",
    }
}

/// Text lines for the panel: the harbours' expeditions and the fleet, the
/// chosen ship marked.
pub fn lines(world: &World, player: PlayerId, choice: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let fleet = fleet(world, player);
    let picked = chosen(world, player, choice);
    for h in world.harbours() {
        if world.building_at(h.at).is_none_or(|b| b.owner != player) {
            continue;
        }
        let expedition = match h.expedition {
            ExpeditionState::Inactive => continue,
            ExpeditionState::Gathering => "gathering goods".to_string(),
            ExpeditionState::GettingShip => "getting a ship".to_string(),
            ExpeditionState::WaitingForShip(s) => format!("waiting for ship {s}"),
        };
        lines.push(format!(
            "harbour ({}, {}): expedition, {expedition}",
            h.at.0, h.at.1
        ));
    }
    if fleet.is_empty() {
        return lines;
    }
    lines.push(
        "ships (Insert choose, Home start expedition at harbour, Numpad 1-6 go, End found harbour, Delete cancel):"
            .into(),
    );
    for &i in &fleet {
        let v = &world.ships()[i].vessel;
        let options: String = v
            .options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.is_some())
            .map(|(d, _)| (d + 1).to_string())
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!(
            "{} ship {i} ({}, {}) {}{}{}{}",
            if picked == Some(i) { ">" } else { " " },
            v.step().at.0,
            v.step().at.1,
            state_name(v.state),
            if v.expedition { ", expedition" } else { "" },
            if v.cargo.is_empty() {
                String::new()
            } else {
                format!(", cargo {}", v.cargo.len())
            },
            if options.is_empty() {
                String::new()
            } else {
                format!(", go: {options}")
            },
        ));
    }
    lines
}

/// What became of a ship command, for the status line.
pub fn describe(command: &Command) -> Option<String> {
    Some(match command {
        Command::ExpeditionStart { at, .. } => {
            format!("expedition prepared at ({}, {})", at.0, at.1)
        }
        Command::ExpeditionStop { ship, .. } => format!("ship {ship}: expedition cancelled"),
        Command::ExpeditionBuildHarbor { ship, .. } => {
            format!("ship {ship}: harbour construction site placed")
        }
        Command::ExpeditionGo {
            ship, direction, ..
        } => format!("ship {ship}: sailing in direction {}", direction + 1),
        _ => return None,
    })
}
