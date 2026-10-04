//! Library file classification (M6(上) §17/§18/§77/§78).
//!
//! ONE place maps extension → category/MIME. The extension is a
//! CLASSIFICATION input, never a playability claim — real playability
//! belongs to the M5 player (§18/§79).

use super::model::MediaCategory;

/// §16: never index Tide's own incomplete artifacts — `Movie.mp4.part`
/// must not become "Movie.mp4" in the library.
pub fn is_excluded_extension(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "part" | "tmp" | "crdownload" | "download" | "torrent"
    )
}

pub fn extension_of(file_name: &str) -> Option<String> {
    let raw = file_name.rsplit_once('.')?.1.to_ascii_lowercase();
    (!raw.is_empty()).then_some(raw)
}

/// §17 categories. Library UIs present Video/Audio/Image as primary
/// content; the rest stay indexable (§17).
pub fn classify(extension: Option<&str>) -> MediaCategory {
    match extension.map(str::to_ascii_lowercase).as_deref() {
        Some(ext) => match ext {
            "mp4" | "m4v" | "mkv" | "webm" | "mov" | "avi" | "wmv" | "flv" | "ts" | "m2ts"
            | "mpg" | "mpeg" => MediaCategory::Video,
            "mp3" | "m4a" | "flac" | "wav" | "ogg" | "opus" | "aac" | "wma" => MediaCategory::Audio,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "avif" => MediaCategory::Image,
            "srt" | "ass" | "ssa" | "vtt" | "sub" => MediaCategory::Subtitle,
            "zip" | "7z" | "rar" | "tar" | "gz" => MediaCategory::Archive,
            "pdf" | "txt" | "md" | "doc" | "docx" | "epub" => MediaCategory::Document,
            _ => MediaCategory::Other,
        },
        None => MediaCategory::Other,
    }
}

/// §78: MIME is optional and untrusted; unknown extensions stay
/// octet-stream and the player decides. Reuses the M4 media mapping
/// for video/audio and adds non-media labels for the index.
pub fn mime_for(extension: Option<&str>) -> Option<&'static str> {
    match extension? {
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mkv" => Some("video/x-matroska"),
        "mov" => Some("video/quicktime"),
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "flac" => Some("audio/flac"),
        "wav" => Some("audio/wav"),
        "ogg" | "opus" => Some("audio/ogg"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "pdf" => Some("application/pdf"),
        "txt" | "md" => Some("text/plain"),
        "srt" => Some("application/x-subrip"),
        "vtt" => Some("text/vtt"),
        "zip" => Some("application/zip"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_artifacts_are_excluded() {
        assert!(is_excluded_extension("part"));
        assert!(is_excluded_extension("TMP"));
        assert!(is_excluded_extension("crdownload"));
        assert!(!is_excluded_extension("mp4"));
    }

    #[test]
    fn classification_is_central() {
        assert_eq!(classify(Some("mp4")), MediaCategory::Video);
        assert_eq!(classify(Some("MKV")), MediaCategory::Video);
        assert_eq!(classify(Some("flac")), MediaCategory::Audio);
        assert_eq!(classify(Some("jpg")), MediaCategory::Image);
        assert_eq!(classify(Some("srt")), MediaCategory::Subtitle);
        assert_eq!(classify(Some("zip")), MediaCategory::Archive);
        assert_eq!(classify(Some("xyz123")), MediaCategory::Other);
        assert_eq!(classify(None), MediaCategory::Other);
    }

    #[test]
    fn extension_extraction() {
        assert_eq!(extension_of("Movie.2023.mp4").as_deref(), Some("mp4"));
        assert!(extension_of("noext").is_none());
        assert_eq!(extension_of("archive.tar.gz").as_deref(), Some("gz"));
    }
}
