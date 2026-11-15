//! What the AI reads of the world once per tick: every player's buildings.
//! The world is only read; nothing here changes it.

use s2_sim::grid::Point;
use s2_sim::{Building, PlayerId, World};

/// Player slots (AI-2.1: 0 to 7).
pub const SLOTS: usize = 8;

/// The buildings of every player at the start of the tick, in the world's
/// building order (row by row of their fields).
pub struct Snapshot {
    pub buildings: Vec<Vec<Building>>,
    /// AI-CON-9.2 (T41): the buildings with a worker standing at home.
    pub at_home: std::collections::BTreeSet<Point>,
    present: Vec<bool>,
}

impl Snapshot {
    pub fn take(world: &World) -> Self {
        let mut buildings = vec![Vec::new(); SLOTS];
        for b in world.buildings() {
            if let Some(list) = buildings.get_mut(usize::from(b.owner.0)) {
                list.push(*b);
            }
        }
        let at_home = world.workers_at_home();
        let present = (0..SLOTS)
            .map(|slot| world.is_player(PlayerId(slot as u8)) || !buildings[slot].is_empty())
            .collect();
        Self {
            buildings,
            at_home,
            present,
        }
    }

    /// AI-2.1 "still in the game": the slot holds a player object, set when
    /// the player is created; nothing clears it on defeat (Q-159). A slot
    /// with a building is a player as well.
    pub fn alive(&self, player: PlayerId) -> bool {
        self.present
            .get(usize::from(player.0))
            .copied()
            .unwrap_or(false)
    }

    pub fn of(&self, player: PlayerId) -> &[Building] {
        self.buildings
            .get(usize::from(player.0))
            .map_or(&[], Vec::as_slice)
    }

    pub fn building(&self, player: PlayerId, at: Point) -> Option<&Building> {
        self.of(player).iter().find(|b| b.at == at)
    }
}
