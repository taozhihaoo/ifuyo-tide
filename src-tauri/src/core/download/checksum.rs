//! Streaming checksum (M1 §30): SHA-256 computed while writing.

use crate::error::TideError;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Sha256,
}

impl ChecksumAlgorithm {
    pub fn parse(raw: &str) -> Result<ChecksumAlgorithm, TideError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "sha256" | "sha-256" => Ok(ChecksumAlgorithm::Sha256),
            other => Err(TideError::InvalidInput(format!(
                "unsupported checksum algorithm: {other}"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ChecksumAlgorithm::Sha256 => "sha256",
        }
    }
}

/// Incremental hasher handed to the file sink.
#[derive(Debug)]
pub struct ChecksumWriter {
    hasher: Sha256,
}

impl ChecksumWriter {
    pub fn new() -> ChecksumWriter {
        ChecksumWriter {
            hasher: Sha256::new(),
        }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }

    /// Lowercase hex digest.
    pub fn finalize_hex(self) -> String {
        let digest = self.hasher.finalize();
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }
}

impl Default for ChecksumWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Compares an expected hex digest against the computed one,
/// case-insensitively.
pub fn matches(expected_hex: &str, computed_hex: &str) -> bool {
    expected_hex.trim().eq_ignore_ascii_case(computed_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_known_input() {
        // sha256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let mut w = ChecksumWriter::new();
        w.update(b"abc");
        assert_eq!(
            w.finalize_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn chunked_updates_equal_one_shot() {
        let payload: Vec<u8> = (0..=255u8).cycle().take(100_000).collect();
        let mut incremental = ChecksumWriter::new();
        for chunk in payload.chunks(1024) {
            incremental.update(chunk);
        }
        let mut one_shot = ChecksumWriter::new();
        one_shot.update(&payload);
        assert_eq!(incremental.finalize_hex(), one_shot.finalize_hex());
    }

    #[test]
    fn comparison_is_case_insensitive_and_trimmed() {
        assert!(matches("ABCDEF", "abcdef"));
        assert!(matches(" abcdef ", "abcdef"));
        assert!(!matches("abcd", "abcde"));
    }

    #[test]
    fn algorithm_parse() {
        assert_eq!(
            ChecksumAlgorithm::parse("sha256").unwrap(),
            ChecksumAlgorithm::Sha256
        );
        assert_eq!(
            ChecksumAlgorithm::parse("SHA-256").unwrap(),
            ChecksumAlgorithm::Sha256
        );
        assert!(ChecksumAlgorithm::parse("md5").is_err());
        assert!(ChecksumAlgorithm::parse("").is_err());
    }
}
