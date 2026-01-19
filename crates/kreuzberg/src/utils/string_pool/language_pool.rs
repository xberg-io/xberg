//! String pool for language codes with pre-interning of common ISO 639 codes.
//!
//! This module provides a thread-safe string pool specifically for language codes,
//! with lazy initialization of common language codes on first access.

use super::interned::InternedString;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// String pool for language codes.
///
/// Lazily initializes with common ISO 639 language codes.
/// Pre-interning is deferred until first access to reduce startup memory usage.
pub(super) struct LanguageStringPool {
    pool: dashmap::DashMap<String, Arc<String>>,
    initialized: AtomicBool,
}

impl LanguageStringPool {
    /// Create a new language string pool.
    /// Pre-interning is deferred until first `get_or_intern()` call.
    pub(super) fn new() -> Self {
        LanguageStringPool {
            pool: dashmap::DashMap::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Ensure all known language codes are pre-interned (one-time initialization).
    #[inline]
    fn ensure_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }

        let lang_codes = vec![
            "en", "es", "fr", "de", "it", "pt", "ru", "ja", "ko", "zh", "ar", "hi", "th", "tr", "pl", "nl", "sv", "no",
            "da", "fi", "cs", "hu", "ro", "el", "he", "fa", "ur", "vi", "id", "ms", "bn", "pa", "te", "mr", "ta", "gu",
            "kn", "ml", "or", "uk", "bg", "sr", "hr", "sl", "sk", "et", "lv", "lt", "sq", "mk", "ka", "hy", "eo",
            "ast", "ca", "eu", "gl", "cy", "gd", "ga",
        ];

        for code in lang_codes {
            self.pool.insert(code.to_string(), Arc::new(code.to_string()));
        }

        let _ = self
            .initialized
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed);
    }

    /// Get or intern a language code string.
    /// Ensures pre-interned language codes are initialized on first call.
    pub(super) fn get_or_intern(&self, lang_code: &str) -> Arc<String> {
        self.ensure_initialized();

        if let Some(entry) = self.pool.get(lang_code) {
            Arc::clone(&*entry)
        } else {
            let arc_string = Arc::new(lang_code.to_string());
            self.pool.insert(lang_code.to_string(), Arc::clone(&arc_string));
            arc_string
        }
    }
}

/// Global language code string pool.
pub(super) static LANGUAGE_POOL: Lazy<LanguageStringPool> = Lazy::new(LanguageStringPool::new);

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
    fn test_language_code_deduplication() {
        let en1 = intern_language_code("en");
        let en2 = intern_language_code("en");

        assert_eq!(en1, en2);
        assert!(Arc::ptr_eq(&en1.0, &en2.0));
    }

    #[test]
    fn test_preinterned_language_codes() {
        let en = intern_language_code("en");
        assert_eq!(en.as_str(), "en");

        let es = intern_language_code("es");
        assert_eq!(es.as_str(), "es");

        let fr = intern_language_code("fr");
        assert_eq!(fr.as_str(), "fr");
    }
}
