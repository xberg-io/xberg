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
    /// Create a new language registry with all supported backends.
    ///
    /// # Returns
    ///
    /// A new `LanguageRegistry` with EasyOCR, PaddleOCR, and Tesseract languages pre-populated.
    pub fn new() -> Self {
        let mut registry = Self {
            backends: HashMap::new(),
        };

        registry.backends.insert(
            "easyocr".to_string(),
            vec![
                "abq".to_string(),
                "ady".to_string(),
                "af".to_string(),
                "ang".to_string(),
                "ar".to_string(),
                "as".to_string(),
                "ava".to_string(),
                "az".to_string(),
                "be".to_string(),
                "bg".to_string(),
                "bh".to_string(),
                "bho".to_string(),
                "bn".to_string(),
                "bs".to_string(),
                "ch_sim".to_string(),
                "ch_tra".to_string(),
                "che".to_string(),
                "cs".to_string(),
                "cy".to_string(),
                "da".to_string(),
                "dar".to_string(),
                "de".to_string(),
                "en".to_string(),
                "es".to_string(),
                "et".to_string(),
                "fa".to_string(),
                "fr".to_string(),
                "ga".to_string(),
                "gom".to_string(),
                "hi".to_string(),
                "hr".to_string(),
                "hu".to_string(),
                "id".to_string(),
                "inh".to_string(),
                "is".to_string(),
                "it".to_string(),
                "ja".to_string(),
                "kbd".to_string(),
                "kn".to_string(),
                "ko".to_string(),
                "ku".to_string(),
                "la".to_string(),
                "lbe".to_string(),
                "lez".to_string(),
                "lt".to_string(),
                "lv".to_string(),
                "mah".to_string(),
                "mai".to_string(),
                "mi".to_string(),
                "mn".to_string(),
                "mr".to_string(),
                "ms".to_string(),
                "mt".to_string(),
                "ne".to_string(),
                "new".to_string(),
                "nl".to_string(),
                "no".to_string(),
                "oc".to_string(),
                "pi".to_string(),
                "pl".to_string(),
                "pt".to_string(),
                "ro".to_string(),
                "ru".to_string(),
                "rs_cyrillic".to_string(),
                "rs_latin".to_string(),
                "sck".to_string(),
                "sk".to_string(),
                "sl".to_string(),
                "sq".to_string(),
                "sv".to_string(),
                "sw".to_string(),
                "ta".to_string(),
                "tab".to_string(),
                "te".to_string(),
                "th".to_string(),
                "tjk".to_string(),
                "tl".to_string(),
                "tr".to_string(),
                "ug".to_string(),
                "uk".to_string(),
                "ur".to_string(),
                "uz".to_string(),
                "vi".to_string(),
            ],
        );

        registry.backends.insert(
            "paddleocr".to_string(),
            vec![
                "ch".to_string(),
                "en".to_string(),
                "french".to_string(),
                "german".to_string(),
                "korean".to_string(),
                "japan".to_string(),
                "chinese_cht".to_string(),
                "ta".to_string(),
                "te".to_string(),
                "ka".to_string(),
                "latin".to_string(),
                "arabic".to_string(),
                "cyrillic".to_string(),
                "devanagari".to_string(),
            ],
        );

        registry.backends.insert(
            "tesseract".to_string(),
            vec![
                "afr".to_string(),
                "amh".to_string(),
                "ara".to_string(),
                "asm".to_string(),
                "aze".to_string(),
                "aze_cyrl".to_string(),
                "bel".to_string(),
                "ben".to_string(),
                "bod".to_string(),
                "bos".to_string(),
                "bre".to_string(),
                "bul".to_string(),
                "cat".to_string(),
                "ceb".to_string(),
                "ces".to_string(),
                "chi_sim".to_string(),
                "chi_tra".to_string(),
                "chr".to_string(),
                "cos".to_string(),
                "cym".to_string(),
                "dan".to_string(),
                "deu".to_string(),
                "div".to_string(),
                "dzo".to_string(),
                "ell".to_string(),
                "eng".to_string(),
                "enm".to_string(),
                "epo".to_string(),
                "equ".to_string(),
                "est".to_string(),
                "eus".to_string(),
                "fao".to_string(),
                "fas".to_string(),
                "fil".to_string(),
                "fin".to_string(),
                "fra".to_string(),
                "frk".to_string(),
                "frm".to_string(),
                "fry".to_string(),
                "gla".to_string(),
                "gle".to_string(),
                "glg".to_string(),
                "grc".to_string(),
                "guj".to_string(),
                "hat".to_string(),
                "heb".to_string(),
                "hin".to_string(),
                "hrv".to_string(),
                "hun".to_string(),
                "hye".to_string(),
                "iku".to_string(),
                "ind".to_string(),
                "isl".to_string(),
                "ita".to_string(),
                "ita_old".to_string(),
                "jav".to_string(),
                "jpn".to_string(),
                "kan".to_string(),
                "kat".to_string(),
                "kat_old".to_string(),
                "kaz".to_string(),
                "khm".to_string(),
                "kir".to_string(),
                "kmr".to_string(),
                "kor".to_string(),
                "lao".to_string(),
                "lat".to_string(),
                "lav".to_string(),
                "lit".to_string(),
                "ltz".to_string(),
                "mal".to_string(),
                "mar".to_string(),
                "mkd".to_string(),
                "mlt".to_string(),
                "mon".to_string(),
                "mri".to_string(),
                "msa".to_string(),
                "mya".to_string(),
                "nep".to_string(),
                "nld".to_string(),
                "nor".to_string(),
                "oci".to_string(),
                "ori".to_string(),
                "osd".to_string(),
                "pan".to_string(),
                "pol".to_string(),
                "por".to_string(),
                "pus".to_string(),
                "que".to_string(),
                "ron".to_string(),
                "rus".to_string(),
                "san".to_string(),
                "sin".to_string(),
                "slk".to_string(),
                "slv".to_string(),
                "snd".to_string(),
                "spa".to_string(),
                "spa_old".to_string(),
                "sqi".to_string(),
                "srp".to_string(),
                "srp_latn".to_string(),
                "sun".to_string(),
                "swa".to_string(),
                "swe".to_string(),
                "syr".to_string(),
                "tam".to_string(),
                "tat".to_string(),
                "tel".to_string(),
                "tgk".to_string(),
                "tha".to_string(),
                "tir".to_string(),
                "ton".to_string(),
                "tur".to_string(),
                "uig".to_string(),
                "ukr".to_string(),
                "urd".to_string(),
                "uzb".to_string(),
                "uzb_cyrl".to_string(),
                "vie".to_string(),
                "yid".to_string(),
                "yor".to_string(),
            ],
        );

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
        self.backends.get(backend).map(|v| v.as_slice())
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
            .get(backend)
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
        self.backends.get(backend).map(|langs| langs.len()).unwrap_or(0)
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
