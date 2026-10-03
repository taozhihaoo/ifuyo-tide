use crate::core::{ids, time};
use crate::error::TideError;
use serde::{Deserialize, Serialize};

/// The kind of a user-provided source.
///
/// M0 only names the types; no protocol is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Http,
    Torrent,
    Magnet,
    LocalFile,
}

impl SourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceType::Http => "http",
            SourceType::Torrent => "torrent",
            SourceType::Magnet => "magnet",
            SourceType::LocalFile => "local_file",
        }
    }

    pub fn parse(value: &str) -> Result<SourceType, TideError> {
        match value {
            "http" => Ok(SourceType::Http),
            "torrent" => Ok(SourceType::Torrent),
            "magnet" => Ok(SourceType::Magnet),
            "local_file" => Ok(SourceType::LocalFile),
            other => Err(TideError::InvalidInput(format!(
                "unknown source type: {other}"
            ))),
        }
    }
}

/// A user-provided source record.
///
/// M0 stores identity and display metadata only. URL/magnet/torrent
/// payloads are never parsed in M0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub source_id: String,
    pub source_type: SourceType,
    pub display_name: String,
    pub created_at: i64,
}

impl Source {
    pub fn new(
        source_id: String,
        source_type: SourceType,
        display_name: String,
    ) -> Result<Source, TideError> {
        let display_name = crate::security::validate_display_name(&display_name)?;
        if !ids::is_valid_id(&source_id) {
            return Err(TideError::InvalidInput(
                "source_id is not a valid id".into(),
            ));
        }
        Ok(Source {
            source_id,
            source_type,
            display_name,
            created_at: time::now_unix_ms(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_type_roundtrip_is_lossless() {
        for t in [
            SourceType::Http,
            SourceType::Torrent,
            SourceType::Magnet,
            SourceType::LocalFile,
        ] {
            assert_eq!(SourceType::parse(t.as_str()).unwrap(), t);
        }
        assert!(SourceType::parse("ftp").is_err());
    }

    #[test]
    fn serde_uses_snake_case() {
        assert_eq!(
            serde_json::to_string(&SourceType::LocalFile).unwrap(),
            "\"local_file\""
        );
    }

    #[test]
    fn new_rejects_invalid_fields() {
        assert!(Source::new("bad".into(), SourceType::Http, "n".into()).is_err());
        assert!(Source::new(ids::new_id(), SourceType::Http, "".into()).is_err());
    }
}
