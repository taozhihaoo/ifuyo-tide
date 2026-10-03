//! Range parsing re-exports (M4(上) §28): M1's battle-tested parser is
//! the single source of truth — no duplicate Range implementations.

pub use crate::core::download::http_range::{
    content_range, parse_content_range_header, parse_range_header, resolve_range,
    resolve_range_header, ContentRangeParse, RangeParse, RangeResolution,
    ResolvedRange,
};
