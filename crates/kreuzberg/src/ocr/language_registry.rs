//! OCR backend language support registry.
//!
//! This module manages supported language codes for different OCR backends.
//! It centralizes language lists that were previously hardcoded in Python bindings.
//!
//! # Supported Backends
//!
//! - **easyocr**: 83 languages with broad multilingual support
//! - **paddleocr**: 14 optimized languages for production deployments
//! - **tesseract**: 100+ languages via Tesseract OCR
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::ocr::LanguageRegistry;
//!
//! let registry = LanguageRegistry::new();
//! if let Some(languages) = registry.get_supported_languages("easyocr") {
//!     println!("EasyOCR supports {} languages", languages.len());
//! }
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

use super::backends;

/// Global language registry instance (lazy initialized)
static LANGUAGE_REGISTRY: OnceLock<LanguageRegistry> = OnceLock::new();

/// Language support registry for OCR backends.
///
/// Maintains a mapping of OCR backend names to their supported language codes.
/// This is the single source of truth for language support across all bindings.
#[derive(Debug, Clone)]
pub struct LanguageRegistry {
    backends: HashMap<String, Vec<String>>,
}

impl LanguageRegistry {
    fn canonical_backend_key(backend: &str) -> &str {
        match backend {
            "paddle-ocr" | "rapidpaddle" | "rapid-paddle" => "paddleocr",
            other => other,
        }
    }

    /// Create a new language registry with all supported backends.
    ///
    /// # Returns
    ///
    /// A new `LanguageRegistry` with EasyOCR, PaddleOCR, and Tesseract languages pre-populated.
    pub fn new() -> Self {
        let mut registry = Self {
            backends: HashMap::new(),
        };

        registry
            .backends
            .insert("easyocr".to_string(), backends::easyocr::languages());
        registry
            .backends
            .insert("paddleocr".to_string(), backends::paddleocr::languages());
        registry
            .backends
            .insert("tesseract".to_string(), backends::tesseract::languages());

        registry
    }

    /// Get the default global registry instance.
    ///
    /// The registry is created on first access and reused for all subsequent calls.
    ///
    /// # Returns
    ///
    /// A reference to the global `LanguageRegistry` instance.
    pub fn global() -> &'static Self {
        LANGUAGE_REGISTRY.get_or_init(Self::new)
    }

    /// Get supported languages for a specific OCR backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - Backend name (e.g., "easyocr", "paddleocr", "tesseract")
    ///
    /// # Returns
    ///
    /// `Some(&[String])` if the backend is registered, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::ocr::LanguageRegistry;
    ///
    /// let registry = LanguageRegistry::new();
    /// if let Some(languages) = registry.get_supported_languages("easyocr") {
    ///     assert!(languages.contains(&"en".to_string()));
    /// }
    /// ```
    pub fn get_supported_languages(&self, backend: &str) -> Option<&[String]> {
        self.backends
            .get(Self::canonical_backend_key(backend))
            .map(|v| v.as_slice())
    }

    /// Check if a language is supported by a specific backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - Backend name
    /// * `language` - Language code to check
    ///
    /// # Returns
    ///
    /// `true` if the language is supported, `false` otherwise.
    pub fn is_language_supported(&self, backend: &str, language: &str) -> bool {
        self.backends
            .get(Self::canonical_backend_key(backend))
            .map(|langs| langs.contains(&language.to_string()))
            .unwrap_or(false)
    }

    /// Get all registered backend names.
    ///
    /// # Returns
    ///
    /// A vector of backend names in the registry.
    pub fn get_backends(&self) -> Vec<String> {
        let mut backends: Vec<_> = self.backends.keys().cloned().collect();
        backends.sort();
        backends
    }

    /// Get language count for a specific backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - Backend name
    ///
    /// # Returns
    ///
    /// Number of supported languages for the backend, or 0 if backend not found.
    pub fn get_language_count(&self, backend: &str) -> usize {
        self.backends
            .get(Self::canonical_backend_key(backend))
            .map(|langs| langs.len())
            .unwrap_or(0)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = LanguageRegistry::new();
        assert!(!registry.backends.is_empty());
    }

    #[test]
    fn test_easyocr_languages() {
        let registry = LanguageRegistry::new();
        let languages = registry
            .get_supported_languages("easyocr")
            .expect("EasyOCR backend not found");

        assert_eq!(languages.len(), 83);
        assert!(languages.contains(&"en".to_string()));
        assert!(languages.contains(&"fr".to_string()));
        assert!(languages.contains(&"de".to_string()));
        assert!(languages.contains(&"ch_sim".to_string()));
        assert!(languages.contains(&"ch_tra".to_string()));
    }

    #[test]
    fn test_paddleocr_languages() {
        let registry = LanguageRegistry::new();
        let languages = registry
            .get_supported_languages("paddleocr")
            .expect("PaddleOCR backend not found");

        assert_eq!(languages.len(), 14);
        assert!(languages.contains(&"en".to_string()));
        assert!(languages.contains(&"ch".to_string()));
        assert!(languages.contains(&"french".to_string()));
        assert!(languages.contains(&"german".to_string()));
    }

    #[test]
    fn test_tesseract_languages() {
        let registry = LanguageRegistry::new();
        let languages = registry
            .get_supported_languages("tesseract")
            .expect("Tesseract backend not found");

        assert!(languages.len() >= 100);
        assert!(languages.contains(&"eng".to_string()));
        assert!(languages.contains(&"fra".to_string()));
        assert!(languages.contains(&"deu".to_string()));
    }

    #[test]
    fn test_get_unsupported_backend() {
        let registry = LanguageRegistry::new();
        assert_eq!(registry.get_supported_languages("nonexistent"), None);
    }

    #[test]
    fn test_paddle_alias_backends() {
        let registry = LanguageRegistry::new();
        assert!(registry.get_supported_languages("paddle-ocr").is_some());
        assert!(registry.get_supported_languages("rapid-paddle").is_some());
        assert!(registry.get_supported_languages("rapidpaddle").is_some());
    }

    #[test]
    fn test_is_language_supported() {
        let registry = LanguageRegistry::new();

        assert!(registry.is_language_supported("easyocr", "en"));
        assert!(registry.is_language_supported("easyocr", "fr"));
        assert!(!registry.is_language_supported("easyocr", "invalid"));

        assert!(registry.is_language_supported("paddleocr", "en"));
        assert!(!registry.is_language_supported("paddleocr", "invalid"));

        assert!(registry.is_language_supported("tesseract", "eng"));
        assert!(!registry.is_language_supported("tesseract", "invalid"));
    }

    #[test]
    fn test_get_backends() {
        let registry = LanguageRegistry::new();
        let backends = registry.get_backends();

        assert_eq!(backends.len(), 3);
        assert!(backends.contains(&"easyocr".to_string()));
        assert!(backends.contains(&"paddleocr".to_string()));
        assert!(backends.contains(&"tesseract".to_string()));
    }

    #[test]
    fn test_get_language_count() {
        let registry = LanguageRegistry::new();

        assert_eq!(registry.get_language_count("easyocr"), 83);
        assert_eq!(registry.get_language_count("paddleocr"), 14);
        assert!(registry.get_language_count("tesseract") >= 100);
        assert_eq!(registry.get_language_count("nonexistent"), 0);
    }

    #[test]
    fn test_default_implementation() {
        let registry1 = LanguageRegistry::default();
        let registry2 = LanguageRegistry::new();

        assert_eq!(
            registry1.get_language_count("easyocr"),
            registry2.get_language_count("easyocr")
        );
    }

    #[test]
    fn test_global_instance() {
        let global1 = LanguageRegistry::global();
        let global2 = LanguageRegistry::global();

        assert_eq!(
            global1.get_language_count("easyocr"),
            global2.get_language_count("easyocr")
        );
    }

    #[test]
    fn test_easyocr_specific_languages() {
        let registry = LanguageRegistry::new();
        let languages = registry.get_supported_languages("easyocr").unwrap();

        assert!(languages.contains(&"abq".to_string()));
        assert!(languages.contains(&"bho".to_string()));
        assert!(languages.contains(&"gom".to_string()));
        assert!(languages.contains(&"rs_cyrillic".to_string()));
        assert!(languages.contains(&"rs_latin".to_string()));
    }

    #[test]
    fn test_paddleocr_specific_languages() {
        let registry = LanguageRegistry::new();
        let languages = registry.get_supported_languages("paddleocr").unwrap();

        assert!(languages.contains(&"ch".to_string()));
        assert!(languages.contains(&"chinese_cht".to_string()));
        assert!(languages.contains(&"devanagari".to_string()));
        assert!(languages.contains(&"arabic".to_string()));
    }

    #[test]
    fn test_tesseract_specific_languages() {
        let registry = LanguageRegistry::new();
        let languages = registry.get_supported_languages("tesseract").unwrap();

        assert!(languages.contains(&"chi_sim".to_string()));
        assert!(languages.contains(&"chi_tra".to_string()));
        assert!(languages.contains(&"ita_old".to_string()));
        assert!(languages.contains(&"spa_old".to_string()));
    }
}
