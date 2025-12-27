#![cfg(feature = "ocr")]
//! Integration tests for OCR language registry
//!
//! Tests the language registry functionality across all OCR backends.

use kreuzberg::ocr::LanguageRegistry;

#[test]
fn test_registry_provides_easyocr_languages() {
    let registry = LanguageRegistry::new();
    let languages = registry.get_supported_languages("easyocr").expect("EasyOCR not found");

    assert_eq!(languages.len(), 83);

    let expected = vec!["en", "fr", "de", "es", "it", "pt", "ja", "ko", "ch_sim", "ch_tra", "ru"];
    for lang in expected {
        assert!(
            languages.contains(&lang.to_string()),
            "Expected language '{}' not found in EasyOCR",
            lang
        );
    }
}

#[test]
fn test_registry_provides_paddleocr_languages() {
    let registry = LanguageRegistry::new();
    let languages = registry
        .get_supported_languages("paddleocr")
        .expect("PaddleOCR not found");

    assert_eq!(languages.len(), 14);

    let expected = vec!["en", "ch", "french", "german", "korean", "japan", "arabic"];
    for lang in expected {
        assert!(
            languages.contains(&lang.to_string()),
            "Expected language '{}' not found in PaddleOCR",
            lang
        );
    }
}

#[test]
fn test_registry_provides_tesseract_languages() {
    let registry = LanguageRegistry::new();
    let languages = registry
        .get_supported_languages("tesseract")
        .expect("Tesseract not found");

    assert!(languages.len() >= 100, "Tesseract should support 100+ languages");

    let expected = vec![
        "eng", "fra", "deu", "spa", "ita", "por", "jpn", "kor", "chi_sim", "chi_tra", "rus",
    ];
    for lang in expected {
        assert!(
            languages.contains(&lang.to_string()),
            "Expected language '{}' not found in Tesseract",
            lang
        );
    }
}

#[test]
fn test_language_support_checking() {
    let registry = LanguageRegistry::new();

    assert!(registry.is_language_supported("easyocr", "en"));
    assert!(registry.is_language_supported("easyocr", "fr"));
    assert!(!registry.is_language_supported("easyocr", "invalid_lang"));

    assert!(registry.is_language_supported("paddleocr", "en"));
    assert!(registry.is_language_supported("paddleocr", "ch"));
    assert!(!registry.is_language_supported("paddleocr", "en_US"));

    assert!(registry.is_language_supported("tesseract", "eng"));
    assert!(registry.is_language_supported("tesseract", "fra"));
    assert!(!registry.is_language_supported("tesseract", "en"));

    assert!(!registry.is_language_supported("invalid_backend", "en"));
}

#[test]
fn test_backend_enumeration() {
    let registry = LanguageRegistry::new();
    let backends = registry.get_backends();

    assert_eq!(backends.len(), 3);
    assert!(backends.contains(&"easyocr".to_string()));
    assert!(backends.contains(&"paddleocr".to_string()));
    assert!(backends.contains(&"tesseract".to_string()));
}

#[test]
fn test_language_count_per_backend() {
    let registry = LanguageRegistry::new();

    assert_eq!(registry.get_language_count("easyocr"), 83);
    assert_eq!(registry.get_language_count("paddleocr"), 14);
    assert!(registry.get_language_count("tesseract") >= 100);
    assert_eq!(registry.get_language_count("nonexistent"), 0);
}

#[test]
fn test_registry_singleton_behavior() {
    let global1 = LanguageRegistry::global();
    let global2 = LanguageRegistry::global();

    assert_eq!(
        global1.get_language_count("easyocr"),
        global2.get_language_count("easyocr")
    );
    assert_eq!(
        global1.get_language_count("paddleocr"),
        global2.get_language_count("paddleocr")
    );
}

#[test]
fn test_easyocr_special_languages() {
    let registry = LanguageRegistry::new();
    let languages = registry.get_supported_languages("easyocr").unwrap();

    let special_langs = vec!["ch_sim", "ch_tra", "rs_cyrillic", "rs_latin"];

    for lang in special_langs {
        assert!(
            languages.contains(&lang.to_string()),
            "EasyOCR should support special language '{}'",
            lang
        );
    }
}

#[test]
fn test_registry_clone() {
    let registry1 = LanguageRegistry::new();
    let registry2 = registry1.clone();

    assert_eq!(
        registry1.get_language_count("easyocr"),
        registry2.get_language_count("easyocr")
    );
    assert_eq!(registry1.get_backends(), registry2.get_backends());
}

#[test]
fn test_registry_default() {
    let registry_default = LanguageRegistry::default();
    let registry_new = LanguageRegistry::new();

    assert_eq!(registry_default.get_backends().len(), registry_new.get_backends().len());
}

#[test]
fn test_registry_consistency() {
    let registries: Vec<_> = (0..5).map(|_| LanguageRegistry::new()).collect();

    let expected_backends = vec!["easyocr", "paddleocr", "tesseract"];
    let expected_counts = vec![("easyocr", 83), ("paddleocr", 14), ("tesseract", 100_usize)];

    for registry in &registries {
        let backends = registry.get_backends();
        assert_eq!(backends.len(), 3);

        for expected_backend in &expected_backends {
            assert!(backends.contains(&expected_backend.to_string()));
        }

        for (backend, min_count) in &expected_counts {
            let count = registry.get_language_count(backend);
            if backend == &"tesseract" {
                assert!(count >= *min_count);
            } else {
                assert_eq!(count, *min_count);
            }
        }
    }
}

#[test]
fn test_language_case_sensitivity() {
    let registry = LanguageRegistry::new();

    assert!(registry.is_language_supported("easyocr", "en"));
    assert!(!registry.is_language_supported("easyocr", "EN"));

    assert!(registry.is_language_supported("easyocr", "en"));
    assert!(!registry.is_language_supported("EASYOCR", "en"));
}
