use super::error::OcrError;
#[cfg(test)]
use super::utils::MINIMAL_SUPPORTED_TESSERACT_VERSION;
use ahash::AHashSet;
use std::sync::LazyLock;

/// Set of all Tesseract ISO 639-based language codes recognized by this library.
///
/// Built from `core::config_validation::TESSERACT_LANGUAGE_CODES`, the single feature-independent
/// source of truth, rather than a second hardcoded list — see that const's doc comment (GH#1621).
/// ~keep
pub static TESSERACT_SUPPORTED_LANGUAGE_CODES: LazyLock<AHashSet<&'static str>> = LazyLock::new(|| {
    crate::core::config_validation::TESSERACT_LANGUAGE_CODES
        .iter()
        .copied()
        .collect()
});

/// Validate a Tesseract language code (or `+`-separated list of codes).
///
/// Accepts `"all"` and `"*"` as special values that auto-detect installed languages.
///
/// # Errors
///
/// Returns [`OcrError::InvalidLanguageCode`] if any component is not in the supported set.
#[cfg_attr(alef, alef(skip))]
pub fn validate_language_code(lang_code: &str) -> Result<(), OcrError> {
    let lower = lang_code.to_ascii_lowercase();
    if lower == "all" || lower == "*" {
        return Ok(());
    }

    for code in lang_code.split('+') {
        if !TESSERACT_SUPPORTED_LANGUAGE_CODES.contains(code) {
            return Err(OcrError::InvalidLanguageCode(format!(
                "Language code '{}' is not supported by Tesseract",
                code
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn validate_tesseract_version(version: u32) -> Result<(), OcrError> {
    if version < MINIMAL_SUPPORTED_TESSERACT_VERSION {
        return Err(OcrError::UnsupportedVersion(format!(
            "Tesseract version {} is not supported. Minimum required version is {}",
            version, MINIMAL_SUPPORTED_TESSERACT_VERSION
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GH#1621: the general `OcrConfig` validator (`core::config_validation::validate_language_code`)
    /// and this Tesseract-specific validator drifted apart because they held two independently
    /// maintained allowlists. Both now read from the same `TESSERACT_LANGUAGE_CODES` array, so this
    /// test asserts that invariant holds for every entry — it fails the moment a future edit adds a
    /// code to only one of the two call sites. ~keep
    #[test]
    fn every_tesseract_supported_code_is_accepted_by_the_general_validator() {
        for code in TESSERACT_SUPPORTED_LANGUAGE_CODES.iter() {
            assert!(
                crate::core::config_validation::validate_language_code(code).is_ok(),
                "general OcrConfig validator rejects tesseract-supported code '{code}'; \
                 the two language-code lists have diverged again"
            );
        }
    }

    #[test]
    fn test_validate_language_code_all_keyword() {
        assert!(validate_language_code("all").is_ok());
        assert!(validate_language_code("*").is_ok());
        assert!(validate_language_code("ALL").is_ok());
        assert!(validate_language_code("All").is_ok());
    }

    #[test]
    fn test_validate_language_code_valid() {
        assert!(validate_language_code("eng").is_ok());
        assert!(validate_language_code("fra").is_ok());
        assert!(validate_language_code("deu").is_ok());
        assert!(validate_language_code("chi_sim").is_ok());
    }

    #[test]
    fn test_validate_language_code_multiple() {
        assert!(validate_language_code("eng+fra").is_ok());
        assert!(validate_language_code("eng+fra+deu").is_ok());
    }

    #[test]
    fn test_validate_language_code_invalid() {
        let result = validate_language_code("invalid_lang");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OcrError::InvalidLanguageCode(_)));
    }

    #[test]
    fn test_validate_language_code_mixed_valid_invalid() {
        let result = validate_language_code("eng+invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tesseract_version_valid() {
        assert!(validate_tesseract_version(5).is_ok());
        assert!(validate_tesseract_version(6).is_ok());
    }

    #[test]
    fn test_validate_tesseract_version_invalid() {
        let result = validate_tesseract_version(4);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OcrError::UnsupportedVersion(_)));
    }

    #[test]
    fn test_language_codes_exist() {
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("eng"));
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("fra"));
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("chi_sim"));
        assert!(!TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("fake"));
    }
}
