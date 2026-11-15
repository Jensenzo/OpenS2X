//! Expansion targets (AI-MIL-3): where the AI wants its border to move.
//! The sources set them; the military part consumes them
//! (`AI-military.md` AI-MIL-3, 4.1).

use s2_sim::grid::Point;

/// AI-MIL-3.1: kind 1 "towards a position", 2 "crowded cell", 3 "anywhere".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Position,
    Crowded,
    Anywhere,
}

/// The four sources of AI-MIL-3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The cell system, for a waiting resource need (AI-CON-3.5).
    WaitingNeed = 0,
    /// A failed attack (AI-MIL-5.5).
    Attack = 1,
    /// A crowded cell (AI-CON-7.6).
    Crowded = 2,
    /// A need found no cell (AI-CON-3.3).
    Anywhere = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub at: Point,
    /// The tick it was set (the oldest target is taken first).
    pub since: u64,
}

/// AI-MIL-3: the pending target of each source and the current target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expansion {
    pub pending: [Option<Target>; 4],
    /// The target the military building creation works on (AI-MIL-4.1) and
    /// the source slot it came from.
    pub current: Option<(Source, Target)>,
    /// AI-MIL-3.1: the oldest pending target is picked one run before it
    /// becomes current.
    pub taken: Option<(Source, Target)>,
    /// AI-MIL-3.2, 4.1: the current target is remembered by the military
    /// building run that made an order for it; the next run that is not
    /// skipped finishes it. A cancel or an upgrade (4.4) ends it before.
    pub remembered: bool,
}

impl Expansion {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether a target is pending in any source or current.
    pub fn any_pending(&self) -> bool {
        self.pending.iter().any(Option::is_some) || self.current.is_some() || self.taken.is_some()
    }

    /// Whether the source has a pending target.
    pub fn is_pending(&self, source: Source) -> bool {
        self.pending[source as usize].is_some()
    }

    /// Set the target of a source (it replaces an older one of the same
    /// source).
    pub fn set(&mut self, source: Source, kind: Kind, at: Point, now: u64) {
        self.pending[source as usize] = Some(Target {
            kind,
            at,
            since: now,
        });
    }

    pub fn clear_source(&mut self, source: Source) {
        self.pending[source as usize] = None;
    }
}
