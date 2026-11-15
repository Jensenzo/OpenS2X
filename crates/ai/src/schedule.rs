//! When each part of a computer player runs (`AI-overview.md` AI-2.2 to
//! AI-2.6).

/// AI-2.2: the phase shift per player slot.
pub const SLOT_SHIFT: u64 = 5;

/// AI-2.2: a part with period `period` runs for player slot `slot` in the
/// AI system's tick `counter` when `(counter + 5 x slot) mod period = 0`.
pub fn due(counter: u64, slot: u8, period: u64) -> bool {
    (counter + SLOT_SHIFT * u64::from(slot)).is_multiple_of(period)
}

/// A part of the computer player that runs on its own period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    // AI-2.3 player level.
    NeedsArrangement,
    NeedsConstruction,
    ResourceSystem,
    CellSystem,
    ExpansionSystem,
    MilitarySystem,
    ConstructionSystem,
    DestructionSystem,
    NeedCreation,
    Locksmith,
    Arrangements,
    ProductionControl,
    ConqueredContinents,
    // AI-2.4 construction system.
    StreetConstruction,
    StreetOptimizer,
    RouteOptimizer,
    ForesterConstruction,
    ShipConstruction,
    HunterConstruction,
    BudgetReset,
    BudgetStep,
    /// AI-2.4a: the executing order's step, the run's last step while an
    /// order is executing (AI-CON-5.3a).
    ExecutingStep,
    // AI-2.4b: the resource system's inner periods.
    FullMarkExpiry,
    CellScan,
    PitAndWoodcutter,
    DepotOrders,
    HarbourOrders,
    CrowdedCells,
    // AI-2.5 military system.
    MilitaryBuilding,
    MilitarySettings,
    AttackSystem,
    SoldierCreation,
    CatapultConstruction,
    CastleSettings,
    WeaponProduction,
    CoinProduction,
    CoinArrangement,
    MilitaryUpgrade,
    // AI-2.5 expansion system.
    Expedition,
    ExpansionTargets,
    // AI-2.6 destruction system.
    BuildingDestruction,
    CellClearing,
}

/// AI-2.3: the player-level parts in the order they run in a tick.
pub const THINK: [(Part, u64); 13] = [
    (Part::NeedsArrangement, 13),
    (Part::NeedsConstruction, 17),
    (Part::ResourceSystem, 1),
    (Part::CellSystem, 23),
    (Part::ExpansionSystem, 1),
    (Part::MilitarySystem, 1),
    (Part::ConstructionSystem, 1),
    (Part::DestructionSystem, 1),
    (Part::NeedCreation, 47),
    (Part::Locksmith, 337),
    (Part::Arrangements, 643),
    (Part::ProductionControl, 67),
    (Part::ConqueredContinents, 1323),
];

/// AI-2.4: the construction system's inner parts in order (the ship
/// construction between the forester and the hunter, Q-376). After them the
/// run takes one of two last steps (AI-2.4a): [`BUDGET_STEP`] while no order
/// is executing, else the executing order's step.
pub const CONSTRUCTION: [(Part, u64); 7] = [
    (Part::StreetConstruction, 7),
    (Part::StreetOptimizer, 67),
    (Part::RouteOptimizer, 37),
    (Part::ForesterConstruction, 3),
    (Part::ShipConstruction, 673),
    (Part::HunterConstruction, 3),
    (Part::BudgetReset, 600),
];

/// AI-2.4, 2.4a: the 50-tick step, run last and only while no order is
/// executing.
pub const BUDGET_STEP: (Part, u64) = (Part::BudgetStep, 50);

/// AI-2.4b: the resource system's inner periods, in this order within a run
/// (Q-160).
pub const RESOURCE: [(Part, u64); 6] = [
    (Part::FullMarkExpiry, 37),
    (Part::CrowdedCells, 19),
    (Part::DepotOrders, 1423),
    (Part::CellScan, 5),
    (Part::PitAndWoodcutter, 53),
    (Part::HarbourOrders, 997),
];

/// AI-2.5: the military system's parts in order.
pub const MILITARY: [(Part, u64); 10] = [
    (Part::MilitaryBuilding, 311),
    (Part::MilitarySettings, 53),
    (Part::AttackSystem, 641),
    (Part::SoldierCreation, 409),
    (Part::CatapultConstruction, 5),
    (Part::CastleSettings, 613),
    (Part::WeaponProduction, 631),
    (Part::CoinProduction, 647),
    (Part::CoinArrangement, 653),
    (Part::MilitaryUpgrade, 37),
];

/// AI-2.5: the expansion system's parts.
pub const EXPANSION: [(Part, u64); 2] = [(Part::Expedition, 2111), (Part::ExpansionTargets, 29)];

/// AI-2.6: destruction of a building every 41 ticks, cell clearing every 59,
/// a useless flag in every other tick.
pub const DESTRUCTION: [(Part, u64); 2] =
    [(Part::BuildingDestruction, 41), (Part::CellClearing, 59)];

#[cfg(test)]
mod tests {
    use super::*;

    // AI-overview T1
    // spec: AI-2.2, AI-2.3 (AI-overview T1)
    #[test]
    fn slot_two_attack_and_settings_phase() {
        // (c + 10) mod 641 = 0
        assert!(due(631, 2, 641));
        assert!(!due(641, 2, 641));
        // (c + 10) mod 53 = 0
        assert!(due(43, 2, 53));
        assert!(!due(53, 2, 53));
    }

    // AI-overview T2
    // spec: AI-2.2 (AI-overview T2)
    #[test]
    fn slot_one_runs_five_steps_after_slot_zero() {
        // street construction: slot 0 at c = 7k, slot 1 where (c + 5) mod 7 = 0.
        let slot0: Vec<u64> = (1..30).filter(|&c| due(c, 0, 7)).collect();
        let slot1: Vec<u64> = (1..30).filter(|&c| due(c, 1, 7)).collect();
        assert_eq!(slot0, [7, 14, 21, 28]);
        assert_eq!(slot1, [2, 9, 16, 23]);
    }

    // spec: AI-2.2
    #[test]
    fn period_one_runs_every_tick() {
        assert!((1..50).all(|c| (0..8).all(|s| due(c, s, 1))));
    }

    // spec: AI-2.3, AI-2.4, AI-2.4a, AI-2.4b, AI-2.5
    #[test]
    fn the_part_lists_have_the_specified_periods() {
        let periods = |l: &[(Part, u64)]| l.iter().map(|p| p.1).collect::<Vec<_>>();
        assert_eq!(
            periods(&THINK),
            [13, 17, 1, 23, 1, 1, 1, 1, 47, 337, 643, 67, 1323]
        );
        assert_eq!(periods(&RESOURCE), [37, 19, 1423, 5, 53, 997]);
        assert_eq!(periods(&CONSTRUCTION), [7, 67, 37, 3, 673, 3, 600]);
        assert_eq!(BUDGET_STEP, (Part::BudgetStep, 50));
        assert_eq!(
            periods(&MILITARY),
            [311, 53, 641, 409, 5, 613, 631, 647, 653, 37]
        );
        assert_eq!(periods(&EXPANSION), [2111, 29]);
        assert_eq!(periods(&DESTRUCTION), [41, 59]);
    }
}
