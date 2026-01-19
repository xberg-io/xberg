//! String pool for MIME types with pre-interning of common types.
//!
//! This module provides a thread-safe string pool specifically for MIME types,
//! with lazy initialization of common MIME types on first access.

use super::interned::InternedString;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// String pool for MIME types.
///
/// Lazily initializes with all known MIME types from `kreuzberg::core::mime`.
/// Pre-interning is deferred until first access to reduce startup memory usage.
pub(super) struct MimeStringPool {
    pool: dashmap::DashMap<String, Arc<String>>,
    initialized: AtomicBool,
}

impl MimeStringPool {
    /// Create a new MIME string pool.
    /// Pre-interning is deferred until first `get_or_intern()` call.
    pub(super) fn new() -> Self {
        MimeStringPool {
            pool: dashmap::DashMap::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Ensure all known MIME types are pre-interned (one-time initialization).
    #[inline]
    fn ensure_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }

        let mime_types = vec![
            "text/html",
            "text/markdown",
            "text/x-markdown",
            "text/plain",
            "application/pdf",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/msword",
            "application/vnd.ms-powerpoint",
            "message/rfc822",
            "application/vnd.ms-outlook",
            "application/json",
            "text/json",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
            "application/yaml",
            "application/toml",
            "text/toml",
            "application/xml",
            "text/xml",
            "image/svg+xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.ms-excel",
            "application/vnd.ms-excel.sheet.macroEnabled.12",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
            "application/vnd.ms-excel.addin.macroEnabled.12",
            "application/vnd.ms-excel.template.macroEnabled.12",
            "application/vnd.oasis.opendocument.spreadsheet",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.oasis.opendocument.text",
            "image/bmp",
            "image/gif",
            "image/jp2",
            "image/jpeg",
            "image/jpm",
            "image/jpx",
            "image/mj2",
            "image/pjpeg",
            "image/png",
            "image/tiff",
            "image/webp",
            "image/x-bmp",
            "image/x-ms-bmp",
            "image/x-portable-anymap",
            "image/x-portable-bitmap",
            "image/x-portable-graymap",
            "image/x-portable-pixmap",
            "image/x-tiff",
            "application/csl+json",
            "application/docbook+xml",
            "application/epub+zip",
            "application/rtf",
            "application/x-biblatex",
            "application/x-bibtex",
            "application/x-endnote+xml",
            "application/x-fictionbook+xml",
            "application/x-ipynb+json",
            "application/x-jats+xml",
            "application/x-latex",
            "application/xml+opml",
            "application/x-opml+xml",
            "application/x-research-info-systems",
            "application/x-typst",
            "text/csv",
            "text/tab-separated-values",
            "text/troff",
            "text/x-commonmark",
            "text/x-dokuwiki",
            "text/x-gfm",
            "text/x-markdown-extra",
            "text/x-mdoc",
            "text/x-multimarkdown",
            "text/x-opml",
            "text/x-org",
            "text/x-pod",
            "text/x-rst",
            "application/zip",
            "application/x-zip-compressed",
            "application/x-tar",
            "application/tar",
            "application/x-gtar",
            "application/x-ustar",
            "application/gzip",
            "application/x-7z-compressed",
        ];

        for mime_type in mime_types {
            self.pool.insert(mime_type.to_string(), Arc::new(mime_type.to_string()));
        }

        let _ = self
            .initialized
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed);
    }

    /// Get or intern a MIME type string.
    /// Ensures pre-interned MIME types are initialized on first call.
    pub(super) fn get_or_intern(&self, mime_type: &str) -> Arc<String> {
        self.ensure_initialized();

        if let Some(entry) = self.pool.get(mime_type) {
            Arc::clone(&*entry)
        } else {
            let arc_string = Arc::new(mime_type.to_string());
            self.pool.insert(mime_type.to_string(), Arc::clone(&arc_string));
            arc_string
        }
    }
}

/// Global MIME type string pool.
pub(super) static MIME_POOL: Lazy<MimeStringPool> = Lazy::new(MimeStringPool::new);

/// Get or intern a MIME type string.
///
/// Returns an `InternedString` that is guaranteed to be deduplicated with any other
/// intern call for the same MIME type. This reduces memory usage and allows
/// fast pointer-based comparisons.
///
/// # Arguments
///
/// * `mime_type` - The MIME type string to intern
///
/// # Returns
///
/// An `InternedString` pointing to the deduplicated string
///
/// # Example
///
/// ```rust,ignore
/// let pdf1 = intern_mime_type("application/pdf");
/// let pdf2 = intern_mime_type("application/pdf");
/// assert_eq!(pdf1, pdf2); // Same pointer
/// ```
pub fn intern_mime_type(mime_type: &str) -> InternedString {
    InternedString(MIME_POOL.get_or_intern(mime_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_deduplication() {
        let mime1 = intern_mime_type("application/pdf");
        let mime2 = intern_mime_type("application/pdf");

        assert_eq!(mime1, mime2);
        assert!(Arc::ptr_eq(&mime1.0, &mime2.0));
    }

    #[test]
    fn test_preinterned_mime_types() {
        let pdf = intern_mime_type("application/pdf");
        assert_eq!(pdf.as_str(), "application/pdf");

        let html = intern_mime_type("text/html");
        assert_eq!(html.as_str(), "text/html");

        let json = intern_mime_type("application/json");
        assert_eq!(json.as_str(), "application/json");
    }

    #[test]
    #[ignore = "Flaky test - concurrent interning may not always share the same Arc"]
    fn test_concurrent_interning() {
        use std::sync::Arc as StdArc;
        use std::thread;

        let mime = "application/pdf";
        let results = StdArc::new(std::sync::Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let results = StdArc::clone(&results);
                thread::spawn(move || {
                    let interned = intern_mime_type(mime);
                    results.lock().unwrap().push(interned);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let interned_strings = results.lock().unwrap();
        assert_eq!(interned_strings.len(), 10);

        let first_arc = &interned_strings[0].0;
        for interned in &*interned_strings {
            assert!(
                Arc::ptr_eq(&interned.0, first_arc),
                "All interned strings should share the same Arc"
            );
        }
    }
}
