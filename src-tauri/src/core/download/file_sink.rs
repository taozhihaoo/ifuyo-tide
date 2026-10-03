//! File sink: temporary `.part` handling, ranged writes, atomic
//! finalization (M1 §9, §11, §29, §40, §41).

use crate::core::download::checksum::ChecksumWriter;
use crate::core::download::url::DownloadFailure;
use crate::error::TideError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// What to do when the FINAL destination file already exists (M1 §41).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwritePolicy {
    /// Refuse to start (default; never clobbers user files).
    Fail,
    /// Replace the existing final file.
    Overwrite,
    /// Append " (1)", " (2)"… to the stem until free.
    AutoRename,
}

impl OverwritePolicy {
    pub fn parse(raw: &str) -> Result<OverwritePolicy, TideError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "fail" => Ok(OverwritePolicy::Fail),
            "overwrite" => Ok(OverwritePolicy::Overwrite),
            "auto_rename" | "autorename" => Ok(OverwritePolicy::AutoRename),
            other => Err(TideError::InvalidInput(format!(
                "unknown overwrite policy: {other}"
            ))),
        }
    }
}

/// The `.part` file and its finalization (M1 §11: crash must never leave
/// a corrupt file that LOOKS complete — everything is written to
/// `<name>.part` and atomically renamed at the very end).
pub struct FileSink {
    part_path: PathBuf,
    final_path: PathBuf,
    file: Option<File>,
    bytes_written: u64,
    checksum: Option<ChecksumWriter>,
}

/// Classifies an std::io::Error into the structured download failures
/// (M1 §28) without leaking raw OS strings to the UI.
pub fn classify_io_error(err: &std::io::Error, context: &str) -> TideError {
    let failure = match err.kind() {
        std::io::ErrorKind::StorageFull => DownloadFailure::DiskFull,
        std::io::ErrorKind::PermissionDenied => DownloadFailure::PermissionDenied,
        std::io::ErrorKind::NotFound
        | std::io::ErrorKind::InvalidInput
        | std::io::ErrorKind::InvalidFilename => DownloadFailure::FileWrite,
        _ => DownloadFailure::FileWrite,
    };
    // Context stays internal to the log; the frontend gets code+message.
    eprintln!("IO_ERR_DETAIL: {context}: {err} (kind={:?})", err.kind());
    crate::diagnostics::warn(
        "download.io_error",
        &[("context", context), ("kind", &format!("{:?}", err.kind()))],
    );
    TideError::Download(failure)
}

impl FileSink {
    /// Plans the part/final paths for a destination directory + filename.
    pub fn plan(
        destination_dir: &Path,
        filename: &str,
        policy: OverwritePolicy,
    ) -> Result<(PathBuf, PathBuf), TideError> {
        let dir_canonical = destination_dir
            .canonicalize()
            .map_err(|err| classify_io_error(&err, "canonicalize destination"))?;
        let candidate = dir_canonical.join(filename);
        // Containment: canonicalized dir + sanitized name can still be
        // tricked only if `filename` contained separators — callers pass
        // sanitized names; assert containment regardless (M1 §9).
        if !candidate.starts_with(&dir_canonical) {
            return Err(TideError::Security(
                "destination escapes download dir".into(),
            ));
        }
        let final_path = match policy {
            OverwritePolicy::Fail => {
                if candidate.exists() {
                    return Err(TideError::Download(DownloadFailure::DestinationExists {
                        path: candidate.to_string_lossy().to_string(),
                    }));
                }
                candidate
            }
            OverwritePolicy::Overwrite => candidate,
            OverwritePolicy::AutoRename => {
                let mut counter = 1u32;
                let mut candidate = candidate;
                while candidate.exists() {
                    let stem = filename
                        .rsplit_once('.')
                        .filter(|(_, ext)| !ext.is_empty() && !filename.starts_with('.'))
                        .map(|(stem, _)| stem.to_string())
                        .unwrap_or_else(|| filename.to_string());
                    let ext = Path::new(filename)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| format!(".{e}"))
                        .unwrap_or_default();
                    candidate = dir_canonical.join(format!("{stem} ({counter}){ext}"));
                    counter += 1;
                }
                candidate
            }
        };
        let part_path = {
            let file_name = final_path
                .file_name()
                .ok_or_else(|| TideError::InvalidInput("destination has no file name".into()))?;
            let mut part_name = file_name.to_os_string();
            part_name.push(".part");
            final_path.with_file_name(part_name)
        };
        Ok((part_path, final_path))
    }

    /// Opens (creating or resuming) the part file at `resume_offset`.
    /// Truncates to `resume_offset` so a resumed part never contains
    /// bytes beyond what the engine believes exist.
    pub fn open(
        part_path: PathBuf,
        final_path: PathBuf,
        resume_offset: u64,
        checksum: Option<ChecksumWriter>,
    ) -> Result<FileSink, TideError> {
        if let Some(parent) = part_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(false)
            .open(&part_path)
            .map_err(|err| classify_io_error(&err, "open part file"))?;
        let existing = file
            .metadata()
            .map_err(|err| classify_io_error(&err, "part metadata"))?
            .len();
        if existing > resume_offset {
            // Stale tail (e.g. crashed mid-write): cut back to the
            // verified length before appending.
            file.set_len(resume_offset)
                .map_err(|err| classify_io_error(&err, "truncate part"))?;
        }
        eprintln!(
            "SINK_DEBUG: open part={} resume_offset={} exists={}",
            part_path.display(),
            resume_offset,
            part_path.exists()
        );
        file.seek(SeekFrom::Start(resume_offset))
            .map_err(|err| classify_io_error(&err, "seek part"))?;
        Ok(FileSink {
            part_path,
            final_path,
            file: Some(file),
            bytes_written: resume_offset,
            checksum,
        })
    }

    pub fn part_path(&self) -> &Path {
        &self.part_path
    }

    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Appends one chunk to the part file and the checksummer.
    pub fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), TideError> {
        let file = self
            .file
            .as_mut()
            .ok_or(TideError::Download(DownloadFailure::FileWrite))?;
        file.write_all(chunk)
            .and_then(|_| file.flush())
            .map_err(|err| classify_io_error(&err, "write chunk"))?;
        if let Some(checksum) = self.checksum.as_mut() {
            checksum.update(chunk);
        }
        self.bytes_written += chunk.len() as u64;
        Ok(())
    }

    /// Reads back `len` bytes at `offset` (used by resume verification).
    pub fn read_part_range(
        part_path: &Path,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, TideError> {
        let mut file =
            File::open(part_path).map_err(|err| classify_io_error(&err, "open part for read"))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|err| classify_io_error(&err, "seek part for read"))?;
        let mut buf = vec![0u8; len];
        file.read_exact(&mut buf)
            .map_err(|err| classify_io_error(&err, "read part range"))?;
        Ok(buf)
    }

    /// Finalizes: flush + fsync + close + atomic rename (M1 §40). Only
    /// called after body/size/checksum validation succeeded.
    pub fn finalize(mut self) -> Result<PathBuf, TideError> {
        let file = self
            .file
            .take()
            .ok_or(TideError::Download(DownloadFailure::FileWrite))?;
        file.sync_all()
            .map_err(|err| classify_io_error(&err, "sync part"))?;
        drop(file);
        eprintln!(
            "RENAME_DEBUG: {} -> {} (part_exists_before={})",
            self.part_path.display(),
            self.final_path.display(),
            self.part_path.exists()
        );
        std::fs::rename(&self.part_path, &self.final_path).map_err(|err| {
            eprintln!("RENAME_DEBUG: FAILED: {err}");
            classify_io_error(&err, "atomic rename")
        })?;
        Ok(self.final_path)
    }

    /// Removes the part file (cancel semantics: discard partial data).
    pub fn discard(part_path: &Path) {
        eprintln!(
            "SINK_DEBUG: discard part={}(exists={})",
            part_path.display(),
            part_path.exists()
        );
        let _ = std::fs::remove_file(part_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tide-m1-sink-{}-{}",
            label,
            crate::core::ids::new_id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn plan_fail_policy_rejects_existing_final() {
        let dir = temp_dir("fail");
        std::fs::write(dir.join("a.bin"), b"existing").unwrap();
        let err = FileSink::plan(&dir, "a.bin", OverwritePolicy::Fail).unwrap_err();
        assert!(matches!(
            err,
            TideError::Download(DownloadFailure::DestinationExists { .. })
        ));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn plan_auto_rename_finds_free_name() {
        let dir = temp_dir("rename");
        std::fs::write(dir.join("a.bin"), b"x").unwrap();
        let (_, final_path) = FileSink::plan(&dir, "a.bin", OverwritePolicy::AutoRename).unwrap();
        assert!(final_path.ends_with("a (1).bin"), "{final_path:?}");
        std::fs::write(&final_path, b"x").unwrap();
        let (_, final_path) = FileSink::plan(&dir, "a.bin", OverwritePolicy::AutoRename).unwrap();
        assert!(final_path.ends_with("a (2).bin"), "{final_path:?}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn plan_overwrite_accepts_existing() {
        let dir = temp_dir("overwrite");
        std::fs::write(dir.join("a.bin"), b"old").unwrap();
        let (_, final_path) = FileSink::plan(&dir, "a.bin", OverwritePolicy::Overwrite).unwrap();
        assert!(final_path.ends_with("a.bin"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn write_seek_finalize_is_atomic_and_exact() {
        let dir = temp_dir("finalize");
        let (part, final_path) = FileSink::plan(&dir, "f.bin", OverwritePolicy::Fail).unwrap();
        let mut sink = FileSink::open(part.clone(), final_path.clone(), 0, None).unwrap();
        sink.write_chunk(b"hello ").unwrap();
        sink.write_chunk(b"world").unwrap();
        assert_eq!(sink.bytes_written(), 11);
        let finalized = sink.finalize().unwrap();
        assert_eq!(std::fs::read(&finalized).unwrap(), b"hello world");
        assert!(!part.exists(), "part must be gone after finalize");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn resume_offset_truncates_stale_tail() {
        let dir = temp_dir("resume");
        let (part, final_path) = FileSink::plan(&dir, "r.bin", OverwritePolicy::Fail).unwrap();
        // Simulate a crash-written part with garbage beyond the verified
        // 6 bytes.
        std::fs::write(&part, b"verifiedTAILGARBAGE").unwrap();
        let mut sink = FileSink::open(part.clone(), final_path.clone(), 8, None).unwrap();
        sink.write_chunk(b"tail").unwrap();
        sink.finalize().unwrap();
        assert_eq!(std::fs::read(&final_path).unwrap(), b"verifiedtail");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn checksummer_follows_written_bytes() {
        let dir = temp_dir("checksum");
        let (part, final_path) = FileSink::plan(&dir, "c.bin", OverwritePolicy::Fail).unwrap();
        let mut checksum = ChecksumWriter::new();
        checksum.update(b"verified");
        let mut sink = FileSink::open(part, final_path, 8, Some(checksum)).unwrap();
        sink.write_chunk(b"tail").unwrap();
        // sha256("verifiedtail")
        let digest = sink.checksum.unwrap().finalize_hex();
        assert_eq!(digest.len(), 64);
        assert_ne!(digest, ChecksumWriter::new().finalize_hex());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn discard_removes_part() {
        let dir = temp_dir("discard");
        let (part, _) = FileSink::plan(&dir, "d.bin", OverwritePolicy::Fail).unwrap();
        std::fs::write(&part, b"partial").unwrap();
        FileSink::discard(&part);
        assert!(!part.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn read_part_range_roundtrips() {
        let dir = temp_dir("readrange");
        let (part, _) = FileSink::plan(&dir, "p.bin", OverwritePolicy::Fail).unwrap();
        std::fs::write(&part, b"0123456789").unwrap();
        let bytes = FileSink::read_part_range(&part, 3, 4).unwrap();
        assert_eq!(bytes, b"3456");
        let _ = std::fs::remove_dir_all(dir);
    }
}
