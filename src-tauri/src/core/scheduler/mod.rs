/// Scheduler domain boundary.
///
/// M0 establishes only the coarse [`Priority`] vocabulary so that tasks
/// and (later) playback demands can reference it. No queue, no algorithm:
/// the real demand-driven scheduler is M7 work, built on
/// `Demand → Priority → Allocation` (see the overall design document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Data the user is currently watching / waiting for.
    Playback,
    /// Explicitly user-boosted tasks.
    UserExplicit,
    /// Regular downloads.
    Normal,
    /// Background completion work.
    Background,
}

impl Priority {
    /// Coarse scheduling weight; M7 replaces this with the full
    /// demand-aware model.
    pub fn weight(self) -> u32 {
        match self {
            Priority::Playback => 100,
            Priority::UserExplicit => 80,
            Priority::Normal => 50,
            Priority::Background => 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_order_matches_semantics() {
        assert!(Priority::Playback.weight() > Priority::UserExplicit.weight());
        assert!(Priority::UserExplicit.weight() > Priority::Normal.weight());
        assert!(Priority::Normal.weight() > Priority::Background.weight());
    }
}
