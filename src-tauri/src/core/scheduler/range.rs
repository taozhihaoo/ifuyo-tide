//! Range set math for the scheduler (M7(下) §105/§106).
//!
//! Half-open `[start, end)` ranges over u64 — the SAME model as
//! `core::resource::ByteRange` (M4上 §9). This is a small, total,
//! exhaustively-tested utility: the policy uses `missing_from` to turn
//! "Demand ∩ Availability" into the exact missing window (§10/§106),
//! and the fuzz/property tests assert the invariants (no overlaps,
//! no empty ranges, sorted output).

/// A normalized set of disjoint, non-empty, sorted half-open ranges.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RangeSet {
    ranges: Vec<(u64, u64)>,
}

impl RangeSet {
    pub fn empty() -> RangeSet {
        RangeSet { ranges: Vec::new() }
    }

    /// Builds from unsorted/overlapping/empty input, normalizing.
    pub fn from_ranges(mut input: Vec<(u64, u64)>) -> RangeSet {
        input.retain(|(s, e)| e > s);
        input.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut ranges: Vec<(u64, u64)> = Vec::with_capacity(input.len());
        for (start, end) in input {
            match ranges.last_mut() {
                // Overlapping or ADJACENT ranges merge (§105 "adjacent").
                Some((_, last_end)) if start <= *last_end => {
                    *last_end = (*last_end).max(end);
                }
                _ => ranges.push((start, end)),
            }
        }
        RangeSet { ranges }
    }

    pub fn ranges(&self) -> &[(u64, u64)] {
        &self.ranges
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn total_len(&self) -> u64 {
        self.ranges.iter().map(|(s, e)| e - s).sum()
    }

    /// §106: what parts of `demand` are NOT covered by `self`?
    /// `demand` is a single half-open range.
    pub fn missing_from(&self, demand_start: u64, demand_end: u64) -> RangeSet {
        let mut missing = Vec::new();
        let mut cursor = demand_start;
        for (s, e) in &self.ranges {
            if *e <= cursor {
                continue; // entirely before the demand window
            }
            if *s >= demand_end {
                break; // entirely after
            }
            if *s > cursor {
                let hole_end = (*s).min(demand_end);
                if hole_end > cursor {
                    missing.push((cursor, hole_end));
                }
            }
            cursor = cursor.max(*e);
            if cursor >= demand_end {
                break;
            }
        }
        if cursor < demand_end {
            missing.push((cursor, demand_end));
        }
        RangeSet { ranges: missing }
    }

    /// Intersection with another set (set semantics over the union).
    pub fn intersect(&self, other: &RangeSet) -> RangeSet {
        let mut out = Vec::new();
        for (as_, ae) in &self.ranges {
            for (bs, be) in &other.ranges {
                let s = (*as_).max(*bs);
                let e = (*ae).min(*be);
                if e > s {
                    out.push((s, e));
                }
            }
        }
        RangeSet::from_ranges(out)
    }

    /// `self ∖ other`.
    pub fn subtract(&self, other: &RangeSet) -> RangeSet {
        let mut missing_of_self = Vec::new();
        for (s, e) in &self.ranges {
            let hole = other.missing_from(*s, *e);
            missing_of_self.extend(hole.ranges);
        }
        RangeSet::from_ranges(missing_of_self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §105: merge / subtract / intersection / difference over the
    /// exact shapes the spec lists.
    #[test]
    fn range_operations() {
        // [0,100) + [100,200) adjacent → merged.
        let merged = RangeSet::from_ranges(vec![(0, 100), (100, 200)]);
        assert_eq!(merged.ranges(), &[(0, 200)]);

        // Overlapping + nested normalize.
        let nested = RangeSet::from_ranges(vec![(50, 150), (0, 200), (20, 30)]);
        assert_eq!(nested.ranges(), &[(0, 200)]);

        // Disjoint stay disjoint, sorted.
        let disjoint = RangeSet::from_ranges(vec![(200, 300), (0, 100)]);
        assert_eq!(disjoint.ranges(), &[(0, 100), (200, 300)]);

        // Empty + invalid (end <= start) dropped.
        let invalid = RangeSet::from_ranges(vec![(50, 50), (60, 40), (0, 10)]);
        assert_eq!(invalid.ranges(), &[(0, 10)]);
    }

    /// §106 EXACT example: availability [0,100)+[200,300), demand
    /// [50,250) → missing [100,200) — precisely.
    #[test]
    fn availability_hole_exact() {
        let availability = RangeSet::from_ranges(vec![(0, 100), (200, 300)]);
        let missing = availability.missing_from(50, 250);
        assert_eq!(missing.ranges(), &[(100, 200)]);
        assert_eq!(missing.total_len(), 100);
    }

    #[test]
    fn missing_handles_edges() {
        let avail = RangeSet::from_ranges(vec![(10, 20)]);
        // Demand fully covered.
        assert!(avail.missing_from(10, 20).is_empty());
        // Demand fully outside → the whole demand is missing.
        assert_eq!(avail.missing_from(20, 30).ranges(), &[(20, 30)]);
        assert_eq!(avail.missing_from(0, 10).ranges(), &[(0, 10)]);
        // Empty availability → whole demand missing.
        assert_eq!(RangeSet::empty().missing_from(5, 9).ranges(), &[(5, 9)]);
        // Zero-length demand → nothing.
        assert!(avail.missing_from(7, 7).is_empty());
        // Large offsets: no overflow.
        let big = RangeSet::from_ranges(vec![(u64::MAX - 10, u64::MAX)]);
        assert_eq!(
            big.missing_from(u64::MAX - 20, u64::MAX).ranges(),
            &[(u64::MAX - 20, u64::MAX - 10)]
        );
    }

    #[test]
    fn intersect_and_subtract() {
        let a = RangeSet::from_ranges(vec![(0, 100), (150, 200)]);
        let b = RangeSet::from_ranges(vec![(50, 175)]);
        assert_eq!(a.intersect(&b).ranges(), &[(50, 100), (150, 175)]);
        assert_eq!(a.subtract(&b).ranges(), &[(0, 50), (175, 200)]);
    }
}
