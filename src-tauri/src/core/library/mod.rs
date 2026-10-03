/// Library item domain boundary.
///
/// M0 fixes only identity and location concepts. No metadata scraping,
/// no posters, no subtitle search — the library becomes real in M6.
use crate::core::resource::ResourceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryItem {
    pub library_item_id: String,
    pub resource_id: ResourceId,
    /// Filesystem location of the media file.
    pub path: String,
    pub created_at: i64,
}

impl LibraryItem {
    pub fn new(resource_id: ResourceId, path: String) -> Result<Self, crate::error::TideError> {
        if path.trim().is_empty() {
            return Err(crate::error::TideError::InvalidInput(
                "library item path must not be empty".into(),
            ));
        }
        Ok(LibraryItem {
            library_item_id: crate::core::ids::new_id(),
            resource_id,
            path,
            created_at: crate::core::time::now_unix_ms(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_path() {
        assert!(LibraryItem::new(ResourceId::new(), "".into()).is_err());
        assert!(LibraryItem::new(ResourceId::new(), "   ".into()).is_err());
    }

    #[test]
    fn generates_identity() {
        let a = LibraryItem::new(ResourceId::new(), "C:\\media\\a.mkv".into()).unwrap();
        let b = LibraryItem::new(ResourceId::new(), "C:\\media\\a.mkv".into()).unwrap();
        assert_ne!(a.library_item_id, b.library_item_id);
    }
}
