//! Military building upgrades (`specs/MIL-building-upgrade.md`).

use crate::grid::Point;
use crate::map::state;
use crate::{PlayerId, Rejection, World};

impl World {
    /// MIL-UPG-1.2: the type the finished military building of `player` on
    /// `at` can be upgraded to now.
    pub fn upgrade_target(&self, player: PlayerId, at: Point) -> Result<u32, Rejection> {
        let building = self
            .building_at(at)
            .filter(|b| b.owner == player)
            .ok_or(Rejection::NoBuilding)?;
        if !self.military.contains_key(&at) {
            return Err(Rejection::NotMilitary);
        }
        // MIL-UPG-1.2 (Q-367): not while the builder that finished it is
        // still linked to its construction.
        if building.under_construction
            || self.upgrades.contains_key(&at)
            || self.builder_links.contains_key(&at)
        {
            return Err(Rejection::NotUpgradable);
        }
        let kind = &self.building_types[&building.building_type];
        let next = kind.upgrade_to.ok_or(Rejection::NotUpgradable)?;
        let to = self
            .building_types
            .get(&next)
            .ok_or(Rejection::UnknownBuildingType)?;
        if to.size != building.size && self.check_building_replacing(player, at, to.size).is_err() {
            return Err(Rejection::NoSpaceToUpgrade);
        }
        Ok(next)
    }

    /// The type the building on `at` is being upgraded to, if it is.
    pub fn upgrading(&self, at: Point) -> Option<u32> {
        self.upgrades.get(&at).copied()
    }

    /// MIL-UPG-1.3, 2.1: the command. Nothing is paid; the building is
    /// marked, its blocked area laid out for the target's size class, and
    /// the construction of the target type begins (levelling first where
    /// the footprint needs it, MIL-UPG-2.2).
    pub(crate) fn upgrade_building(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<(), Rejection> {
        let next = self.upgrade_target(player, at)?;
        let old = *self.building_at(at).expect("a building");
        let kind = self.building_types[&next].clone();
        let new_footprint = self.footprint_of(at, kind.size);
        if kind.size != old.size {
            for &field in &new_footprint {
                self.set_bits(field, state::MOVEMENT_BLOCKED, true);
            }
            let index = self.terrain.index(at);
            self.buildings[index].as_mut().expect("a building").size = kind.size;
        }
        self.changed(&new_footprint);
        if kind.instant {
            // ECO-REQ-4.1
            self.change_building_type(at, next);
            return Ok(());
        }
        // MIL-UPG-2.2, ECO-HQ-4.8a (T6): the field's height is set again as
        // at a site's placement; the leveller brings the neighbours to it.
        let touched = self.level_site_height(at);
        self.changed(&touched);
        let site = self.new_site(at, &kind, kind.size);
        let levelled = site.levelling_needed;
        self.upgrades.insert(at, next);
        self.sites.insert(at, site);
        // ECO-PLACE-5.3: an upgrade's site as any site.
        if !levelled {
            self.clear_decorations(at);
        }
        Ok(())
    }
}
