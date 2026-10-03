use crate::error::TideError;

/// A half-open byte range `[start, end)`.
///
/// Tide uses half-open intervals everywhere — inclusive ends are never
/// mixed in. An empty range (`start == end`) is valid and has length 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    start: u64,
    /// Exclusive.
    end: u64,
}

impl ByteRange {
    pub fn new(start: u64, end: u64) -> Result<ByteRange, TideError> {
        if end < start {
            return Err(TideError::InvalidInput(format!(
                "invalid byte range: end {end} is before start {start}"
            )));
        }
        Ok(ByteRange { start, end })
    }

    pub const fn start(&self) -> u64 {
        self.start
    }

    /// Exclusive end.
    pub const fn end(&self) -> u64 {
        self.end
    }

    pub const fn length(&self) -> u64 {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Whether the byte at `position` falls inside this range.
    pub const fn contains(&self, position: u64) -> bool {
        position >= self.start && position < self.end
    }

    /// Whether this range shares at least one byte with `other`.
    ///
    /// Adjacent ranges (`[0, 100)` and `[100, 200)`) do not overlap.
    pub const fn overlaps(&self, other: &ByteRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_properties() {
        let r = ByteRange::new(100, 200).unwrap();
        assert_eq!(r.start(), 100);
        assert_eq!(r.end(), 200);
        assert_eq!(r.length(), 100);
        assert!(!r.is_empty());
    }

    #[test]
    fn empty_range_is_allowed() {
        let r = ByteRange::new(50, 50).unwrap();
        assert_eq!(r.length(), 0);
        assert!(r.is_empty());
        assert!(!r.contains(50));
    }

    #[test]
    fn end_before_start_is_rejected() {
        assert!(ByteRange::new(200, 100).is_err());
    }

    #[test]
    fn contains_is_half_open() {
        let r = ByteRange::new(100, 200).unwrap();
        assert!(!r.contains(99));
        assert!(r.contains(100));
        assert!(r.contains(199));
        assert!(!r.contains(200));
    }

    #[test]
    fn adjacent_ranges_do_not_overlap() {
        let a = ByteRange::new(0, 100).unwrap();
        let b = ByteRange::new(100, 200).unwrap();
        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn overlapping_ranges_are_detected() {
        let a = ByteRange::new(0, 100).unwrap();
        let b = ByteRange::new(50, 150).unwrap();
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn contained_range_overlaps() {
        let a = ByteRange::new(0, 1000).unwrap();
        let b = ByteRange::new(10, 20).unwrap();
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn disjoint_ranges_do_not_overlap() {
        let a = ByteRange::new(0, 10).unwrap();
        let b = ByteRange::new(20, 30).unwrap();
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn empty_range_never_overlaps() {
        let empty = ByteRange::new(50, 50).unwrap();
        let other = ByteRange::new(0, 100).unwrap();
        assert!(!empty.overlaps(&other));
        assert!(!other.overlaps(&empty));
    }

    #[test]
    fn large_offsets_are_supported() {
        // 10 GB file positions must not overflow.
        let big: u64 = 10 * 1024 * 1024 * 1024;
        let r = ByteRange::new(big, big + 1024).unwrap();
        assert_eq!(r.length(), 1024);
        assert!(r.contains(big + 512));
    }
}
