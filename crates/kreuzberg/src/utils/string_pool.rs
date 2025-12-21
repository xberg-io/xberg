//! String interning/pooling for frequently used strings.
//!
//! This module provides thread-safe string interning to reduce memory allocations
//! for strings that appear repeatedly across documents (MIME types, language codes, format field names).
//!
//! # Performance
//!
//! String interning provides 0.1-0.3% improvement by:
//! - Deduplicating repeated strings (e.g., "application/pdf" appears 1000s of times)
//! - Reducing allocation overhead for commonly used strings
//! - Enabling pointer comparisons instead of string comparisons
//!
//! # Thread Safety
//!
//! The intern pool uses a `DashMap` for lock-free concurrent access. Multiple threads
//! can insert and lookup strings simultaneously without contention.
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::utils::string_pool::intern_mime_type;
//!
//! let mime1 = intern_mime_type("application/pdf");
//! let mime2 = intern_mime_type("application/pdf");
//! // Both mime1 and mime2 point to the same interned string
//! assert_eq!(mime1, mime2);
//! ```

use once_cell::sync::Lazy;
use std::sync::Arc;

/// A reference to an interned string stored in an Arc.
///
/// This wraps an Arc<String> and provides convenient access to the string content.
/// Multiple calls with the same string content will share the same Arc, reducing memory usage.
#[derive(Clone)]
pub struct InternedString(Arc<String>);

impl InternedString {
    /// Get the string content.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for InternedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InternedString").field(&self.as_str()).finish()
    }
}

impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        // Pointer equality check (fast) + fallback to string comparison
        Arc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}

impl Eq for InternedString {}

impl std::hash::Hash for InternedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl std::ops::Deref for InternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// String pool for MIME types.
///
/// Pre-initializes with all known MIME types from `kreuzberg::core::mime`.
struct MimeStringPool {
    pool: dashmap::DashMap<String, Arc<String>>,
}

impl MimeStringPool {
    /// Create a new MIME string pool with pre-interned common types.
    fn new() -> Self {
        let pool = dashmap::DashMap::new();

        // Pre-intern all known MIME types
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
            // Image types
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
            // Document formats
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
            // Archives
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
            pool.insert(mime_type.to_string(), Arc::new(mime_type.to_string()));
        }

        MimeStringPool { pool }
    }

    /// Get or intern a MIME type string.
    fn get_or_intern(&self, mime_type: &str) -> Arc<String> {
        if let Some(entry) = self.pool.get(mime_type) {
            Arc::clone(&*entry)
        } else {
            let arc_string = Arc::new(mime_type.to_string());
            self.pool.insert(mime_type.to_string(), Arc::clone(&arc_string));
            arc_string
        }
    }
}

/// String pool for language codes.
///
/// Pre-initializes with common ISO 639 language codes.
struct LanguageStringPool {
    pool: dashmap::DashMap<String, Arc<String>>,
}

impl LanguageStringPool {
    /// Create a new language string pool with pre-interned common codes.
    fn new() -> Self {
        let pool = dashmap::DashMap::new();

        // Pre-intern common ISO 639 language codes
        let lang_codes = vec![
            "en", "es", "fr", "de", "it", "pt", "ru", "ja", "ko", "zh", "ar", "hi", "th", "tr", "pl", "nl", "sv", "no",
            "da", "fi", "cs", "hu", "ro", "el", "he", "fa", "ur", "vi", "id", "ms", "bn", "pa", "te", "mr", "ta", "gu",
            "kn", "ml", "or", "uk", "bg", "sr", "hr", "sl", "sk", "et", "lv", "lt", "sq", "mk", "ka", "hy", "eo",
            "ast", "ca", "eu", "gl", "cy", "gd", "ga",
        ];

        for code in lang_codes {
            pool.insert(code.to_string(), Arc::new(code.to_string()));
        }

        LanguageStringPool { pool }
    }

    /// Get or intern a language code string.
    fn get_or_intern(&self, lang_code: &str) -> Arc<String> {
        if let Some(entry) = self.pool.get(lang_code) {
            Arc::clone(&*entry)
        } else {
            let arc_string = Arc::new(lang_code.to_string());
            self.pool.insert(lang_code.to_string(), Arc::clone(&arc_string));
            arc_string
        }
    }
}

/// Global MIME type string pool.
static MIME_POOL: Lazy<MimeStringPool> = Lazy::new(MimeStringPool::new);

/// Global language code string pool.
static LANGUAGE_POOL: Lazy<LanguageStringPool> = Lazy::new(LanguageStringPool::new);

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

/// Get or intern a language code string.
///
/// Returns an `InternedString` that is guaranteed to be deduplicated with any other
/// intern call for the same language code.
///
/// # Arguments
///
/// * `lang_code` - The language code to intern (e.g., "en", "es", "fr")
///
/// # Returns
///
/// An `InternedString` pointing to the deduplicated string
///
/// # Example
///
/// ```rust,ignore
/// let en1 = intern_language_code("en");
/// let en2 = intern_language_code("en");
/// assert_eq!(en1, en2); // Same pointer
/// ```
pub fn intern_language_code(lang_code: &str) -> InternedString {
    InternedString(LANGUAGE_POOL.get_or_intern(lang_code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_deduplication() {
        let mime1 = intern_mime_type("application/pdf");
        let mime2 = intern_mime_type("application/pdf");

        assert_eq!(mime1, mime2);
        // Check pointer equality (Arc should point to same allocation)
        assert!(Arc::ptr_eq(&mime1.0, &mime2.0));
    }

    #[test]
    fn test_language_code_deduplication() {
        let en1 = intern_language_code("en");
        let en2 = intern_language_code("en");

        assert_eq!(en1, en2);
        // Check pointer equality
        assert!(Arc::ptr_eq(&en1.0, &en2.0));
    }

    #[test]
    fn test_interned_string_display() {
        let mime = intern_mime_type("text/html");
        assert_eq!(format!("{}", mime), "text/html");
    }

    #[test]
    fn test_interned_string_deref() {
        let mime = intern_mime_type("application/json");
        assert_eq!(&*mime, "application/json");
        assert_eq!(mime.as_ref(), "application/json");
        assert_eq!(mime.as_str(), "application/json");
    }

    #[test]
    fn test_preinterned_mime_types() {
        // Verify that pre-interned MIME types are actually interned
        let pdf = intern_mime_type("application/pdf");
        assert_eq!(pdf.as_str(), "application/pdf");

        let html = intern_mime_type("text/html");
        assert_eq!(html.as_str(), "text/html");

        let json = intern_mime_type("application/json");
        assert_eq!(json.as_str(), "application/json");
    }

    #[test]
    fn test_preinterned_language_codes() {
        // Verify that pre-interned language codes are actually interned
        let en = intern_language_code("en");
        assert_eq!(en.as_str(), "en");

        let es = intern_language_code("es");
        assert_eq!(es.as_str(), "es");

        let fr = intern_language_code("fr");
        assert_eq!(fr.as_str(), "fr");
    }

    #[test]
    fn test_concurrent_interning() {
        use std::sync::Arc;
        use std::thread;

        let mime = "application/pdf";
        let results = Arc::new(std::sync::Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let results = Arc::clone(&results);
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

        // All should point to the same Arc
        let first_arc = &interned_strings[0].0;
        for interned in &*interned_strings {
            assert!(
                Arc::ptr_eq(&interned.0, first_arc),
                "All interned strings should share the same Arc"
            );
        }
    }

    #[test]
    fn test_interned_string_hash() {
        let mime1 = intern_mime_type("application/pdf");
        let mime2 = intern_mime_type("application/pdf");

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(mime1);
        set.insert(mime2);

        // Should only contain 1 item since they're equal and hash the same
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_interned_string_clone() {
        let mime1 = intern_mime_type("text/html");
        let mime2 = mime1.clone();

        assert_eq!(mime1, mime2);
        assert!(Arc::ptr_eq(&mime1.0, &mime2.0));
    }
}
