mod byte_range;

pub use byte_range::ByteRange;

use crate::error::TideError;

/// Opaque, non-enumerable identifier for a resource.
///
/// This is the type the future stream gateway will expose; it must never
/// become a predictable integer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new() -> Self {
        ResourceId(crate::core::ids::new_id())
    }

    pub fn from_string(value: String) -> Result<Self, TideError> {
        if !crate::core::ids::is_valid_id(&value) {
            return Err(TideError::InvalidInput(
                "resource_id is not a valid id".into(),
            ));
        }
        Ok(ResourceId(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal resource concept reserved for M4/M5.
///
/// M0 fixes identity and optional total size only. Availability (the byte
/// range model) is introduced by the M4 streaming work and deliberately
/// not modelled here yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub resource_id: ResourceId,
    /// Total size in bytes, when known.
    pub total_size: Option<u64>,
}

impl Resource {
    pub fn new(resource_id: ResourceId, total_size: Option<u64>) -> Resource {
        Resource {
            resource_id,
            total_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_generation_is_non_enumerable() {
        let a = ResourceId::new();
        let b = ResourceId::new();
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 36);
    }

    #[test]
    fn resource_id_from_string_validates() {
        assert!(ResourceId::from_string("not-a-uuid".into()).is_err());
        assert!(ResourceId::from_string(crate::core::ids::new_id()).is_ok());
    }

    #[test]
    fn resource_holds_optional_size() {
        let r = Resource::new(ResourceId::new(), None);
        assert_eq!(r.total_size, None);
        let r = Resource::new(ResourceId::new(), Some(1024));
        assert_eq!(r.total_size, Some(1024));
    }
}
