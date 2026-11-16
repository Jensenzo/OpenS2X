//! Commands in, results out: every change to the world is a [`Command`]
//! applied at the start of a step (`docs/api.md`).

use crate::grid::Point;
use crate::{Cheat, PlayerId};

/// Every way a player (human or AI) can change the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Does nothing.
    Noop { player: PlayerId },
    /// ECO-PLACE-2.2 (`place-flag`); on a road it splits the road
    /// (TRN-ROAD-4).
    PlaceFlag { player: PlayerId, at: Point },
    /// A road between two of the player's flags along the cheapest route
    /// (TRN-ROAD-2).
    BuildRoad {
        player: PlayerId,
        from: Point,
        to: Point,
    },
    /// The computer player's road builder (AI-CON-8.1 b, 8.3 c) from its
    /// flag `from` to its flag `to`: the planning stub takes a number
    /// (SIM-START-4.4 c), the path found evens the ground along it
    /// (TRN-ROAD-3.3a), and the road is made only when its steps are at
    /// most 2 x the distance of its ends.
    PlanRoad {
        player: PlayerId,
        from: Point,
        to: Point,
    },
    /// `place-building`: a construction site of building type `building`
    /// (its script id) on `at`, with its flag on the south-east neighbour
    /// (ECO-PLACE-3, 5.1).
    PlaceBuilding {
        player: PlayerId,
        building: u32,
        at: Point,
    },
    /// The computer player's construction site: as `PlaceBuilding`, and
    /// the `records` its building-added hook makes (the cell, production
    /// and need records) take their object numbers right after the
    /// building's own, before its flag's (SIM-START-4.4 b, T13).
    PlanBuilding {
        player: PlayerId,
        building: u32,
        at: Point,
        records: u32,
    },
    /// TRN-ROAD-7 (waterway mode): a waterway between two of
    /// the player's flags, the first on a Coast field.
    BuildWaterway {
        player: PlayerId,
        from: Point,
        to: Point,
    },
    /// TRN-ROAD-8.1 (`start-road`): start building a road,
    /// or a waterway, from the player's flag on `at`.
    StartRoute {
        player: PlayerId,
        at: Point,
        waterway: bool,
    },
    /// TRN-ROAD-8.2 (`extend-road`): extend the route being
    /// built to `to`, or cut it back to `to` if it is one of its fields.
    ExtendRoute { player: PlayerId, to: Point },
    /// TRN-ROAD-8.3 (`cancel-road`): drop the route being built.
    CancelRoute { player: PlayerId },
    /// ECO-PLACE-5.1 (`remove-building`): remove the building standing on
    /// `at`. Its flag stays.
    RemoveBuilding { player: PlayerId, at: Point },
    /// The computer player's demolish (AI-CON-9.3): the building on `at`
    /// is removed with remains (MIL-CON-4.1, 4.4): part of its material on
    /// its flag, then its ruin. Its flag stays.
    TearDown { player: PlayerId, at: Point },
    /// MIL-UPG-1.3 (`upgrade-building`): upgrade the finished military
    /// building on `at` to the
    /// next type of its chain. Nothing is paid at the order.
    UpgradeBuilding { player: PlayerId, at: Point },
    /// TRN-ROAD-6.1 (`remove-road`): destroy the road with an inner field
    /// `at`. Its end
    /// flags stay.
    RemoveRoad { player: PlayerId, at: Point },
    /// TRN-ROAD-6.2 (`remove-flag`): destroy the flag on `at` with its
    /// roads, the building
    /// behind it and the wares lying on it.
    RemoveFlag { player: PlayerId, at: Point },
    /// SHIP-5.1 (`prepare-expedition`): the harbour on `at`
    /// prepares an expedition.
    ExpeditionStart { player: PlayerId, at: Point },
    /// SHIP-6.5 (`cancel-expedition`): cancel the expedition of
    /// the ship (its index in [`World::ships`]).
    ExpeditionStop { player: PlayerId, ship: u32 },
    /// SHIP-7 (`found-harbour`): found a harbour.
    ExpeditionBuildHarbor { player: PlayerId, ship: u32 },
    /// SHIP-6.2 (`sail-expedition`): sail
    /// to the landing option in compass `direction` (0 to 5).
    ExpeditionGo {
        player: PlayerId,
        ship: u32,
        direction: u8,
    },
    /// Debug: a new ware of `good` at the flag `at`, sent to the flag
    /// `destination`. Which building a ware goes to is not specified yet
    /// (TRN-CARRY-5.6).
    DropWare {
        player: PlayerId,
        good: u32,
        at: Point,
        destination: Point,
    },
    /// ECO-PROD-1.2 (`toggle-production`): toggle production of a finished
    /// building.
    ProductionToggle { player: PlayerId, at: Point },
    /// ECO-PROD-1.3 (`switch-production`): toggle the product switch.
    ProductionSwitch { player: PlayerId, at: Point },
    /// ECO-PROD-7.3 `set-tool-weight`: the weight of tool `slot`
    /// (0 to 11) in thousandths (500 is 0.5); at most 1 gives 0.001.
    ToolWeight {
        player: PlayerId,
        slot: u8,
        per_mille: u32,
    },
    /// ECO-PROD-7.3 `set-only-tool`.
    ToolExclusive { player: PlayerId, slot: u8 },
    /// ECO-PROD-7.3 `clear-only-tool`.
    ToolClearExclusive { player: PlayerId },
    /// ECO-PROD-7.3 `reset-tools`: default weights, no exclusive slot,
    /// accumulators kept.
    ToolReset { player: PlayerId },
    /// ECO-DIST-1.3 `cycle-ware-flags`: the click on a good in the warehouse
    /// window.
    WarehouseCycle {
        player: PlayerId,
        at: Point,
        good: u32,
    },
    /// ECO-DIST-1.3 `toggle-stop-flag` / `toggle-out-flag`.
    WarehouseToggle {
        player: PlayerId,
        at: Point,
        good: u32,
        stop: bool,
    },
    /// ECO-DIST-1.3 `set-stop-flag` (`stop`) or `set-out-flag` (not `stop`).
    WarehouseSet {
        player: PlayerId,
        at: Point,
        good: u32,
        stop: bool,
        on: bool,
    },
    /// ECO-DIST-1.3 `set-all-ware-flags`: `mode` 0 clears, 1 stop, 2 out,
    /// for every ware
    /// (or, with `settlers`, every settler) good.
    WarehouseAll {
        player: PlayerId,
        at: Point,
        mode: u8,
        settlers: bool,
    },
    /// ECO-DIST-2.2 `set-distribution-weight`: the weight of entry `entry`
    /// of table `table`, in
    /// thousandths (at most 1 gives 0.001).
    DistributionWeight {
        player: PlayerId,
        table: u8,
        entry: u8,
        thousandths: u32,
    },
    /// ECO-DIST-2.2 `reset-distribution` of one table.
    DistributionReset { player: PlayerId, table: u8 },
    /// ECO-DIST-4.2 `move-priority`: the goods of rank `from` get rank `to`.
    PriorityMove {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    /// ECO-DIST-4.2 `reset-priorities`.
    PriorityReset { player: PlayerId },
    /// MIL-SOL-7.1 `set-close-occupation`, `set-medium-occupation`,
    /// `set-far-occupation`: the allocation factor of one distance class (0
    /// near, 1 medium, 2 far), in thousandths (1000 is 1.0).
    MilitaryAllocation {
        player: PlayerId,
        class: u8,
        thousandths: u32,
    },
    /// MIL-SOL-7.1 `set-attack-strength`: "Attack Strength" factor in
    /// thousandths.
    MilitaryAttackStrength { player: PlayerId, thousandths: u32 },
    /// MIL-SOL-7.1 `set-interceptors`: "Intercepting Defenders" factor in
    /// thousandths.
    MilitaryDefenders { player: PlayerId, thousandths: u32 },
    /// MIL-SOL-7.1 `set-soldier-preference`: defense type, true for "prefer
    /// strong soldiers".
    MilitaryDefenseType { player: PlayerId, strong: bool },
    /// MIL-SOL-7.1 `reset-military-settings`: "Reset dialog settings".
    MilitaryReset { player: PlayerId },
    /// MIL-PROM-3.2, MIL-SOL-7.2 (`toggle-coins`): toggle the coin supply of
    /// a building.
    ToggleCoinSupply { player: PlayerId, at: Point },
    /// MIL-SOL-4.6, 7.2 (`toggle-soldiers-out`): toggle moving soldiers out
    /// of a building.
    ToggleMoveOut { player: PlayerId, at: Point },
    /// MIL-SOL-6.1 (`set-soldier-reserve`): soldiers of `rank` the
    /// headquarters at `at` keeps outside its stock.
    SetCastleSoldiers {
        player: PlayerId,
        at: Point,
        rank: u8,
        amount: u32,
    },
    /// MIL-ATK-1.5 (`attack-building`): attack the enemy military building
    /// on `target` with
    /// `count` soldiers; `strong_first` is the dialog's "Use strong units
    /// first".
    Attack {
        player: PlayerId,
        target: Point,
        count: u32,
        strong_first: bool,
    },
    /// CAM-1.3 `menu.addResource`: `amount` of `good` into the player's
    /// headquarters.
    AddGoods {
        player: PlayerId,
        good: u32,
        amount: u32,
    },
    /// CAM-1.3 `menu.clearResources`: the headquarters' stock emptied.
    ClearGoods { player: PlayerId },
    /// CAM-1.3 `menu.lockBuilding`: the type cannot be placed by the player.
    LockBuilding { player: PlayerId, building: u32 },
    /// CAM-1.3 `menu.createBuilding` (`settler` none) and
    /// `createBuildingWithSettler`: the building finished on `at`.
    CreateBuilding {
        player: PlayerId,
        building: u32,
        at: Point,
        settler: Option<u32>,
    },
    /// CAM-1.3 `menu.createStreet`: a road between two fields; an end
    /// without a flag of the player gets one first (Q-187).
    CreateStreet {
        player: PlayerId,
        from: Point,
        to: Point,
    },
    /// CAM-1.3 `menu.createShip`.
    CreateShip { player: PlayerId, at: Point },
    /// UI-GAME-4.5 c: the name typed in the window of the player's ship
    /// `ship` (its index in the ship list).
    NameShip {
        player: PlayerId,
        ship: u32,
        name: String,
    },
    /// CAM-1.4 `triggers.removeTrigger`.
    RemoveTrigger { name: String },
    /// CAM-3.7: a cheat typed by the player.
    Cheat { player: PlayerId, cheat: Cheat },
    /// ECO-SPC-1.1 `send-geologist`: a geologist to the flag on `at`.
    OrderGeologist { player: PlayerId, at: Point },
    /// ECO-SPC-1.1 `send-scout`: a scout to the flag on `at`.
    OrderScout { player: PlayerId, at: Point },
    /// DEBUG ONLY, not a game rule: a shortcut past requests and orders
    /// (ECO-REQ). Puts `wares` (good, count) straight onto the construction
    /// site at `at` and, with
    /// `workers`, a leveller (while levelling is unfinished) and a builder
    /// there. Nothing is taken from any stock.
    DebugSupplySite {
        player: PlayerId,
        at: Point,
        wares: Vec<(u32, u32)>,
        workers: bool,
    },
    /// Hook W20 (a project's, not the original's): the finished building on
    /// `at` works recipe `index` of its type's set (`Rules::recipe_sets`,
    /// `docs/extending.md` "Recipes a player chooses"); the open orders of
    /// goods the new recipe does not take are withdrawn.
    ChooseRecipe {
        player: PlayerId,
        at: Point,
        index: u32,
    },
    /// Hook W15 (a project's, not the original's): deepen the player's
    /// exhausted mine on `at` (`Rules::deepenings`, `docs/extending.md`
    /// "Deepening a mine"): its material is ordered and a builder works it
    /// as an upgrade's construction; at its end part of the ore comes back.
    DeepenMine { player: PlayerId, at: Point },
    /// Hook W19 (a project's, not the original's): pave the player's road
    /// through the inner field `at` (`Rules::paving`, `docs/extending.md`
    /// "Paved roads"): its material is sent from the warehouses to the
    /// road's first flag; once it has all arrived the road is paved.
    PaveRoad { player: PlayerId, at: Point },
}

/// Why a command was not carried out. Each names the rule it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    OutsideMap,
    /// ECO-PLACE-2.1, 3.1, 3.6: the field is not the player's.
    NotOwned,
    /// ECO-PLACE-2.1: Border, Water, ElevationBlocked, DoodadBlocked, Blocked,
    /// DepositBlocked or MovementBlocked.
    NotGeneralSite,
    /// ECO-PLACE-2.2: a flag on the field or a neighbour.
    FlagTooClose,
    /// ECO-PLACE-3.2: the flag field (south-east) is unusable.
    FlagFieldUnusable,
    /// ECO-PLACE-3.3: the field or a neighbour is Blocked (another building).
    TooCloseToBuilding,
    /// ECO-PLACE-3.4, 3.10: a height difference to the mean above 2400.
    TooSteep,
    /// ECO-PLACE-3.5: a mine needs a general ForMining field without a road.
    NotMiningGround,
    /// ECO-PLACE-3.6: a footprint field is unusable.
    FootprintUnusable,
    /// ECO-PLACE-3.8: fewer than 3 suitable water fields at distance 3.
    NoWaterForHarbor,
    /// ECO-PLACE-3.9: another building, or the map edge, within 2 fields.
    NoRoomForLargeBuilding,
    /// ECO-PLACE-3.10: a field within 2 is outside the map.
    TooCloseToEdge,
    UnknownBuildingType,
    /// TRN-ROAD: a road starts and ends at one of the player's flags.
    NoFlagAtStart,
    NoFlagAtTarget,
    SameFlag,
    /// TRN-ROAD-1.1: the target is not a general site.
    TargetNotGeneralSite,
    /// TRN-ROAD-2.3: no route.
    NoRoute,
    /// ECO-PLACE-5.1: no building of the player on that field.
    NoBuilding,
    /// A ware needs one of the player's flags at both ends.
    NotAFlag,
    /// No road of the player has an inner field there.
    NoRoad,
    /// TRN-ROAD-7.1: a waterway starts at a flag on a Coast field.
    NotCoast,
    /// TRN-ROAD-7.4: a waterway is at most 6 steps long.
    WaterwayTooLong,
    /// AI-CON-8.1 b: the computer player's road builder takes a road of at
    /// most 2 x the distance of its ends steps.
    PlannedRoadTooLong,
    /// TRN-ROAD-8.1: the player already builds a road.
    RouteExists,
    /// The field has no finished harbour of the player.
    NotAHarbour,
    /// No finished ship of the player has that number.
    NoSuchShip,
    /// The ship is not on an expedition, or is not ready for the command.
    NotOnExpedition,
    /// Not a compass direction (0 to 5).
    NoSuchDirection,
    /// No harbour can be founded here (SHIP-7.1).
    NoHarbourSpot,
    UnknownGood,
    /// TRN-CARRY-4: the flag cannot take another ware.
    FlagFull,
    /// The building is finished, not a construction site.
    NotAConstructionSite,
    /// More of a good than the site's costs (ECO-HQ-4.1) still need.
    WareNotNeeded,
    /// ECO-PROD-7: a tool slot is 0 to 11.
    NoSuchToolSlot,
    /// ECO-DIST: no such mode, distribution table or entry.
    NoSuchSetting,
    /// MIL-SOL-7: the building is not a military building of its kind.
    NotMilitary,
    /// MIL-ATK-1.3: only an enemy's building can be attacked.
    NotAnEnemy,
    /// MIL-UPG-1.2: unfinished, already upgrading, or no next type in the
    /// chain ("Upgrade Running", "Upgrade not possible").
    NotUpgradable,
    /// MIL-UPG-1.2: the bigger footprint does not fit ("Upgrade no space").
    NoSpaceToUpgrade,
    /// CAM-1.3: the mission script has locked the type for the player.
    BuildingLocked,
    /// CAM-1.4: no trigger has that name.
    NoSuchTrigger,
    /// Hook W20: the building's type has no recipe of that index.
    NotARecipe,
    /// Hook W15: not a mine of a type with a deepening, not exhausted, at
    /// its most deepenings, or one under way.
    NotDeepenable,
    /// Hook W19: no paving rule, a waterway, or paved or being paved.
    NotPavable,
    /// ECO-PLACE-6.2: a military type within 4 fields of a military
    /// building or site ("Too close to other military buildings").
    TooCloseToMilitary,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::OutsideMap => "outside the map",
            Self::NotOwned => "not your land (ECO-PLACE-2.1)",
            Self::NotGeneralSite => "field is blocked, water, too steep or border (ECO-PLACE-2.1)",
            Self::FlagTooClose => "a flag is on or next to this field (ECO-PLACE-2.2)",
            Self::FlagFieldUnusable => "no flag possible south-east of it (ECO-PLACE-3.2)",
            Self::TooCloseToBuilding => "next to another building (ECO-PLACE-3.3)",
            Self::TooSteep => "ground too steep (ECO-PLACE-3.4, 3.10)",
            Self::NotMiningGround => "not mining ground (ECO-PLACE-3.5)",
            Self::FootprintUnusable => "building ground unusable (ECO-PLACE-3.6)",
            Self::NoWaterForHarbor => "not enough open water for a harbour (ECO-PLACE-3.8)",
            Self::NoRoomForLargeBuilding => {
                "another building or the map edge within 2 fields (ECO-PLACE-3.9)"
            }
            Self::TooCloseToEdge => "too close to the map edge (ECO-PLACE-3.10)",
            Self::UnknownBuildingType => "unknown building type",
            Self::NoFlagAtStart => "the road must start at one of your flags",
            Self::NoFlagAtTarget => "the road must end at one of your flags",
            Self::SameFlag => "start and end are the same flag",
            Self::TargetNotGeneralSite => "the road's end is not usable ground (TRN-ROAD-1.1)",
            Self::NoRoute => "no route (TRN-ROAD-2.3)",
            Self::NoBuilding => "none of your buildings stands there",
            Self::NoRoad => "there is no road of yours there",
            Self::NotCoast => "a waterway starts at a flag on the coast",
            Self::WaterwayTooLong => "a waterway is at most 6 steps long",
            Self::PlannedRoadTooLong => "longer than 2 x the distance of its ends (AI-CON-8.1 b)",
            Self::NotARecipe => "the building's type has no such recipe (a project's hook W20)",
            Self::NotDeepenable => "the mine cannot be deepened now (a project's hook W15)",
            Self::NotPavable => "the road cannot be paved (a project's hook W19)",
            Self::RouteExists => "a road is already being built",
            Self::NotAHarbour => "there is no finished harbour of yours there",
            Self::NoSuchShip => "you have no such ship",
            Self::NotOnExpedition => "the ship is not on an expedition ready for that",
            Self::NoSuchDirection => "a compass direction is 0 to 5",
            Self::NoHarbourSpot => "no harbour can be founded from here",
            Self::NotAFlag => "a ware needs one of your flags where it starts and ends",
            Self::NoSuchToolSlot => "a tool slot is 0 to 11 (ECO-PROD-7)",
            Self::NoSuchSetting => "no such mode, table or entry (ECO-DIST)",
            Self::NotMilitary => "not a military building (MIL-SOL-2.1)",
            Self::NotAnEnemy => "not an enemy's building (MIL-ATK-1.3)",
            Self::NotUpgradable => "the building cannot be upgraded (MIL-UPG-1.2)",
            Self::NoSpaceToUpgrade => "no space for the bigger building (MIL-UPG-1.2)",
            Self::UnknownGood => "unknown good",
            Self::FlagFull => "the flag holds no more wares (TRN-CARRY-4)",
            Self::NotAConstructionSite => "not a construction site",
            Self::WareNotNeeded => "the site does not need that (ECO-HQ-4.1)",
            Self::BuildingLocked => "locked in this mission (CAM-1.3)",
            Self::NoSuchTrigger => "no trigger of that name (CAM-1.4)",
            Self::TooCloseToMilitary => "too close to other military buildings (ECO-PLACE-6.2)",
        })
    }
}

/// The outcome of one command applied in a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub command: Command,
    pub outcome: Result<(), Rejection>,
}
