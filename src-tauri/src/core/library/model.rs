//! Library domain model (M6(上) §6–§10/§30/§86/§87).
//!
//! Four concepts stay separate (§3): Task = one download operation,
//! Resource = readable bytes, LibraryItem = a local entity the user
//! manages long-term, Playback = one session. Nothing here merges them.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaCategory {
    Video,
    Audio,
    Image,
    Subtitle,
    Archive,
    Document,
    Other,
}

impl MediaCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaCategory::Video => "video",
            MediaCategory::Audio => "audio",
            MediaCategory::Image => "image",
            MediaCategory::Subtitle => "subtitle",
            MediaCategory::Archive => "archive",
            MediaCategory::Document => "document",
            MediaCategory::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<MediaCategory> {
        Some(match value {
            "video" => MediaCategory::Video,
            "audio" => MediaCategory::Audio,
            "image" => MediaCategory::Image,
            "subtitle" => MediaCategory::Subtitle,
            "archive" => MediaCategory::Archive,
            "document" => MediaCategory::Document,
            "other" => MediaCategory::Other,
            _ => return None,
        })
    }
}

/// §86: one enum, not a pile of booleans — `missing` and `available`
/// can never both be true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    Available,
    Missing,
    Inaccessible,
}

impl AvailabilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AvailabilityStatus::Available => "available",
            AvailabilityStatus::Missing => "missing",
            AvailabilityStatus::Inaccessible => "inaccessible",
        }
    }

    pub fn parse(value: &str) -> Option<AvailabilityStatus> {
        Some(match value {
            "available" => AvailabilityStatus::Available,
            "missing" => AvailabilityStatus::Missing,
            "inaccessible" => AvailabilityStatus::Inaccessible,
            _ => return None,
        })
    }
}

/// §8: where an item came from. NOT the download Source concept — an
/// imported file has no Source at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginType {
    Download,
    ImportedLocal,
    Unknown,
}

impl OriginType {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginType::Download => "download",
            OriginType::ImportedLocal => "imported_local",
            OriginType::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Option<OriginType> {
        Some(match value {
            "download" => OriginType::Download,
            "imported_local" => OriginType::ImportedLocal,
            "unknown" => OriginType::Unknown,
            _ => return None,
        })
    }
}

/// §87: root state as a small enum + one runtime status string, not
/// stacked booleans.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub id: String,
    pub path: PathBuf,
    pub display_name: String,
    pub enabled: bool,
    /// idle | scanning | ok | partial | failed (§70).
    pub scan_status: String,
    pub last_scan_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: String,
    pub root_id: String,
    /// §11: canonical, verified local path. Never accepted from the
    /// frontend as a string — only produced by the scanner/import or
    /// re-validated from this DB on use.
    pub canonical_path: PathBuf,
    pub relative_path: String,
    pub display_name: String,
    pub extension: Option<String>,
    pub media_category: MediaCategory,
    /// §78: MIME is optional and never authoritative.
    pub mime_type: Option<String>,
    pub size_bytes: u64,
    /// Unix ms of the file's mtime.
    pub modified_at: i64,
    pub availability: AvailabilityStatus,
    pub origin: OriginType,
    /// Nullable by design (§31): imported files have no task.
    pub task_id: Option<String>,
    /// Reused from M1's verified checksum when the item came from a
    /// download (§20) — never recomputed by the library.
    pub checksum: Option<String>,
    pub last_verified_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// §41–§45: backend-side query — search/filter/sort/pagination all
/// happen in SQL, never "fetch all + JS sort".
#[derive(Debug, Clone, Default)]
pub struct ItemQuery {
    /// LIKE substring over display_name / relative_path.
    pub search: Option<String>,
    pub category: Option<MediaCategory>,
    pub availability: Option<AvailabilityStatus>,
    pub origin: Option<OriginType>,
    /// name | size | modified | added (whitelisted in the repository).
    pub sort: Option<String>,
    pub ascending: bool,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPage {
    pub items: Vec<LibraryItem>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}
