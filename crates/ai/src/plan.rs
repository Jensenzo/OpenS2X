//! The AI's planning objects (AI-CON-1.2): needs and productions.

use s2_sim::grid::Point;

use crate::cells::CellPos;
use crate::resources::Square;

pub type NeedId = u32;
pub type ProdId = u32;

/// A need: a good the AI wants produced, for a consumer building.
#[derive(Debug, Clone, PartialEq)]
pub struct Need {
    pub id: NeedId,
    pub good: u32,
    /// The consumer building's field (the headquarters, or a smithy or mint
    /// for the needs of AI-MIL-6, or the new building of AI-CON-2.5); none
    /// for an input need of AI-CON-3.2 (T26).
    pub consumer: Option<Point>,
    pub cell: Option<CellPos>,
    /// The resource square inside the cell and its kind (AI-CON-3.4).
    pub square: Option<(Square, u8)>,
    pub production: Option<ProdId>,
    /// AI-CON-3.4: no square was found; the cell system drives expansion.
    pub waiting: bool,
    /// AI-CON-3.1: the linked food need.
    pub food: Option<NeedId>,
    /// Game seconds at creation.
    pub created: f32,
    /// AI-CON-8.3 b (T51): when it was tied to its consumer; the
    /// consumer's need list is in this order (0: no consumer).
    pub tied: u32,
    /// AI-CON-9.3 (T84): its producer was torn down and it waits to be
    /// placed again.
    pub again: bool,
    /// AI-CON-2.5b (T102): its place in its cell's need list, which holds
    /// the needs in the order they were given the cell (0: no cell, or
    /// given it in a game saved before snapshot format 83, ordered by id).
    pub celled: u32,
}

/// A planned or existing producer of a good in a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Production {
    pub id: ProdId,
    pub good: u32,
    pub cell: CellPos,
    /// The building once it is placed.
    pub building: Option<Point>,
    pub need: Option<NeedId>,
}

/// All needs and productions of one player.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub needs: Vec<Need>,
    pub prods: Vec<Production>,
    next_id: u32,
    /// SIM-START-4.2, 4.4 b, g: the need and production records made so
    /// far, each an object that takes a number from the simulation's
    /// counter.
    made: u32,
    /// AI-CON-3.4, 3.5: the waiting list, in the order the needs were put
    /// on it.
    pub waiting: Vec<NeedId>,
    /// AI-CON-8.3 b (T51): the needs tied to a consumer so far.
    ties: u32,
    /// AI-CON-2.5b (T102): the needs given a cell so far.
    celled: u32,
}

impl Plan {
    /// FMT-SAV-3.10d, 3.10g: a plan from a save: its needs and productions
    /// with the ids they were given, the waiting list, the needs tied so far.
    pub(crate) fn restored(
        needs: Vec<Need>,
        prods: Vec<Production>,
        waiting: Vec<NeedId>,
        ties: u32,
    ) -> Self {
        let celled = needs.iter().map(|n| n.celled).max().unwrap_or(0);
        let last = needs
            .iter()
            .map(|n| n.id)
            .chain(prods.iter().map(|p| p.id))
            .max();
        Self {
            next_id: last.map_or(0, |id| id + 1),
            made: (needs.len() + prods.len()) as u32,
            needs,
            prods,
            waiting,
            ties,
            celled,
        }
    }

    pub fn clear(&mut self) {
        self.needs.clear();
        self.prods.clear();
        self.next_id = 0;
        self.waiting.clear();
        self.ties = 0;
        self.celled = 0;
    }

    /// AI-CON-2.5b (T102): the need `id` is given `cell`: it joins the end of
    /// the cell's need list.
    pub fn give_cell(&mut self, id: NeedId, cell: CellPos) {
        self.celled += 1;
        let place = self.celled;
        if let Some(n) = self.need_mut(id) {
            n.cell = Some(cell);
            n.celled = place;
        }
    }

    /// AI-CON-2.5b (T102): the first need in `cell`'s need list (the order
    /// the needs were given the cell; needs given it before snapshot format
    /// 82 first, by id) that has no production yet and whose good `takes`.
    pub fn first_need_in(&self, cell: CellPos, takes: impl Fn(u32) -> bool) -> Option<NeedId> {
        self.needs
            .iter()
            .filter(|n| n.cell == Some(cell) && n.production.is_none() && takes(n.good))
            .min_by_key(|n| (n.celled, n.id))
            .map(|n| n.id)
    }

    /// AI-CON-8.3 b (T51): the next place in a consumer's need list.
    fn next_tie(&mut self) -> u32 {
        self.ties += 1;
        self.ties
    }

    /// AI-CON-2.5: the need of `good` tied to the new building `consumer`:
    /// the first existing need of that good without a consumer, else a new
    /// one; it joins the end of the building's need list (T51).
    pub fn tie_need(&mut self, good: u32, consumer: Point, now: f32) -> NeedId {
        let tie = self.next_tie();
        match self
            .needs
            .iter_mut()
            .find(|n| n.good == good && n.consumer.is_none())
        {
            Some(need) => {
                need.consumer = Some(consumer);
                need.tied = tie;
                need.id
            }
            None => {
                let id = self.add_need(good, Some(consumer), now);
                if let Some(need) = self.need_mut(id) {
                    need.tied = tie;
                }
                id
            }
        }
    }

    /// AI-CON-3.4: the need goes on the end of the waiting list.
    pub fn put_waiting(&mut self, id: NeedId) {
        if let Some(n) = self.need_mut(id) {
            n.waiting = true;
            if !self.waiting.contains(&id) {
                self.waiting.push(id);
            }
        }
    }

    /// AI-CON-3.4, 3.5: the need comes off the waiting list.
    pub fn unwait(&mut self, id: NeedId) {
        if let Some(n) = self.need_mut(id) {
            n.waiting = false;
        }
        self.waiting.retain(|&w| w != id);
    }

    fn fresh(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// SIM-START-4.4 b: the production record of a building, an object made
    /// when the building is added; a production only planned is not one.
    pub fn building_record_made(&mut self) {
        self.made = self.made.wrapping_add(1);
    }

    /// SIM-START-4.2: how many records were made so far.
    pub fn made(&self) -> u32 {
        self.made
    }

    pub fn add_need(&mut self, good: u32, consumer: Option<Point>, now: f32) -> NeedId {
        // SIM-START-4.4 g (corrected 2026-10-27 b, T14): every need is a
        // record of its own, an object with a number, also one of a good
        // its consumer already needs: the Hard castle's three start plank
        // needs are three records (was: one record per good of a consumer,
        // which the Weak profile's single plank need could not tell apart).
        self.made = self.made.wrapping_add(1);
        let id = self.fresh();
        self.needs.push(Need {
            id,
            good,
            consumer,
            cell: None,
            square: None,
            production: None,
            waiting: false,
            food: None,
            created: now,
            tied: 0,
            again: false,
            celled: 0,
        });
        // T51: a need made for a consumer joins the end of its list.
        if consumer.is_some() {
            let tie = self.next_tie();
            if let Some(need) = self.needs.last_mut() {
                need.tied = tie;
            }
        }
        id
    }

    pub fn add_production(&mut self, good: u32, cell: CellPos, need: Option<NeedId>) -> ProdId {
        let id = self.fresh();
        self.prods.push(Production {
            id,
            good,
            cell,
            building: None,
            need,
        });
        id
    }

    pub fn need(&self, id: NeedId) -> Option<&Need> {
        self.needs.iter().find(|n| n.id == id)
    }

    pub fn need_mut(&mut self, id: NeedId) -> Option<&mut Need> {
        self.needs.iter_mut().find(|n| n.id == id)
    }

    pub fn production(&self, id: ProdId) -> Option<&Production> {
        self.prods.iter().find(|p| p.id == id)
    }

    /// Delete a production and detach it from its need.
    pub fn remove_production(&mut self, id: ProdId) {
        self.prods.retain(|p| p.id != id);
        for n in &mut self.needs {
            if n.production == Some(id) {
                n.production = None;
            }
        }
    }

    pub fn remove_need(&mut self, id: NeedId) {
        self.needs.retain(|n| n.id != id);
        self.waiting.retain(|&w| w != id);
        for p in &mut self.prods {
            if p.need == Some(id) {
                p.need = None;
            }
        }
        for n in &mut self.needs {
            if n.food == Some(id) {
                n.food = None;
            }
        }
    }

    /// Productions of `good` in `cell`.
    pub fn producers_in(&self, cell: CellPos, good: u32) -> usize {
        self.prods
            .iter()
            .filter(|p| p.cell == cell && p.good == good)
            .count()
    }

    /// AI-CON-7.1 (T66): productions of `good` in `cell` whose site is
    /// placed. The original makes a production record with the placed site
    /// (SIM-START-4.4 b), so one only ordered is none.
    pub fn placed_producers_in(&self, cell: CellPos, good: u32) -> usize {
        self.prods
            .iter()
            .filter(|p| p.cell == cell && p.good == good && p.building.is_some())
            .count()
    }

    /// AI-CON-3.3 (T71): the needs that have `cell` as their cell.
    pub fn needs_in(&self, cell: CellPos) -> usize {
        self.needs.iter().filter(|n| n.cell == Some(cell)).count()
    }

    /// AI-CON-3.3 (T71): the needs of one of `goods` in `cell`.
    pub fn needs_of_in(&self, cell: CellPos, goods: &[u32]) -> usize {
        self.needs
            .iter()
            .filter(|n| n.cell == Some(cell) && goods.contains(&n.good))
            .count()
    }

    /// AI-CON-3.3 (T70): a free production: one without a need, or whose
    /// need has no consumer building.
    pub fn is_free(&self, p: &Production) -> bool {
        p.need
            .and_then(|id| self.need(id))
            .is_none_or(|n| n.consumer.is_none())
    }

    /// Productions of any good in `cell`.
    pub fn productions_in(&self, cell: CellPos) -> usize {
        self.prods.iter().filter(|p| p.cell == cell).count()
    }

    /// Productions of `good` in all cells.
    pub fn producers(&self, good: u32) -> usize {
        self.prods.iter().filter(|p| p.good == good).count()
    }

    /// Productions with a placed building.
    pub fn placed_producers(&self, good: u32) -> usize {
        self.prods
            .iter()
            .filter(|p| p.good == good && p.building.is_some())
            .count()
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_struct!(Plan {
    needs,
    prods,
    next_id,
    made,
    waiting,
    ties,
    celled @ 83 = 0
});
