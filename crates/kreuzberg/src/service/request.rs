//! Request and response types for the extraction service.

use crate::core::config::{ExtractionConfig, FileExtractionConfig};
use bytes::Bytes;
use std::path::PathBuf;

/// The source of a document to extract.
#[derive(Debug, Clone)]
pub enum ExtractionSource {
    /// Extract from a filesystem path with an optional MIME type hint.
    File { path: PathBuf, mime_hint: Option<String> },
    /// Extract from in-memory bytes with a known MIME type.
    Bytes { data: Bytes, mime_type: String },
}

/// A request to extract content from a single document.
#[derive(Debug, Clone)]
pub struct ExtractionRequest {
    /// Where to read the document from.
    pub source: ExtractionSource,
    /// Base extraction configuration.
    pub config: ExtractionConfig,
    /// Optional per-file overrides (merged on top of `config`).
    pub file_overrides: Option<FileExtractionConfig>,
}

impl ExtractionRequest {
    /// Create a file-based extraction request.
    pub(crate) fn file(path: impl Into<PathBuf>, config: ExtractionConfig) -> Self {
        Self {
            source: ExtractionSource::File {
                path: path.into(),
                mime_hint: None,
            },
            config,
            file_overrides: None,
        }
    }

    /// Create a file-based extraction request with a MIME type hint.
    pub(crate) fn file_with_mime(
        path: impl Into<PathBuf>,
        mime_hint: impl Into<String>,
        config: ExtractionConfig,
    ) -> Self {
        Self {
            source: ExtractionSource::File {
                path: path.into(),
                mime_hint: Some(mime_hint.into()),
            },
            config,
            file_overrides: None,
        }
    }

    /// Create a bytes-based extraction request.
    pub(crate) fn bytes(data: impl Into<Bytes>, mime_type: impl Into<String>, config: ExtractionConfig) -> Self {
        Self {
            source: ExtractionSource::Bytes {
                data: data.into(),
                mime_type: mime_type.into(),
            },
            config,
            file_overrides: None,
        }
    }

    /// Set per-file overrides on this request.
    #[cfg(test)]
    pub(crate) fn with_overrides(mut self, overrides: FileExtractionConfig) -> Self {
        self.file_overrides = Some(overrides);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_creates_file_source() {
        let req = ExtractionRequest::file("/tmp/doc.pdf", ExtractionConfig::default());
        match &req.source {
            ExtractionSource::File { path, mime_hint } => {
                assert_eq!(path, &PathBuf::from("/tmp/doc.pdf"));
                assert!(mime_hint.is_none());
            }
            _ => panic!("expected File source"),
        }
        assert!(req.file_overrides.is_none());
    }

    #[test]
    fn bytes_creates_bytes_source() {
        let req = ExtractionRequest::bytes(b"hello".as_slice(), "text/plain", ExtractionConfig::default());
        match &req.source {
            ExtractionSource::Bytes { data, mime_type } => {
                assert_eq!(data.as_ref(), b"hello");
                assert_eq!(mime_type, "text/plain");
            }
            _ => panic!("expected Bytes source"),
        }
    }

    #[test]
    fn file_with_mime_sets_hint() {
        let req = ExtractionRequest::file_with_mime("/tmp/doc.pdf", "application/pdf", ExtractionConfig::default());
        match &req.source {
            ExtractionSource::File { mime_hint, .. } => {
                assert_eq!(mime_hint.as_deref(), Some("application/pdf"));
            }
            _ => panic!("expected File source"),
        }
    }

    #[test]
    fn with_overrides_sets_file_overrides() {
        let overrides = FileExtractionConfig::default();
        let req = ExtractionRequest::file("/tmp/doc.pdf", ExtractionConfig::default()).with_overrides(overrides);
        assert!(req.file_overrides.is_some());
    }
}
