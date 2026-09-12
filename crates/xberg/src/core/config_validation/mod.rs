//! Configuration validation module.
//!
//! Provides centralized validation for configuration values across all bindings.
//! This eliminates duplication of validation logic in Python, TypeScript, Java, Go, and other language bindings.
//!
//! All validation functions return `Result<()>` and produce detailed error messages
//! suitable for user-facing error handling.
//!
//! # Examples
//!
//! ```ignore
//! use xberg::core::config_validation::{
//!     validate_binarization_method,
//!     validate_token_reduction_level,
//!     validate_language_code,
//! };
//!
//! // Valid values
//! assert!(validate_binarization_method("otsu").is_ok());
//! assert!(validate_token_reduction_level("moderate").is_ok());
//! assert!(validate_language_code("en").is_ok());
//!
//! // Invalid values
//! assert!(validate_binarization_method("invalid").is_err());
//! assert!(validate_token_reduction_level("extreme").is_err());
//! ```

#[cfg(feature = "api-types")]
mod dependencies;
mod sections;

#[cfg(feature = "api-types")]
pub(crate) use dependencies::{validate_cors_origin, validate_host, validate_port, validate_upload_size};
pub(crate) use sections::{
    validate_chunking_params, validate_confidence, validate_csv_delimiter, validate_dpi, validate_language_code,
    validate_ocr_backend, validate_token_reduction_level, validate_vlm_backend_config,
};
// Re-exported only for `ocr::validation`, which is itself `#[cfg(feature = "ocr")]`. Without the
// same gate the re-export is an unused import on every narrow leg, and CI builds with
// `-D warnings`. ~keep
#[cfg(feature = "ocr")]
pub(crate) use sections::TESSERACT_LANGUAGE_CODES;

// `layout_wastes_plain_output` is `pub`, not `pub(crate)`, unlike its siblings above: it backs
// a CLI-level warning (`xberg-cli`'s `ExtractionOverrides::apply`), a downstream crate that
// cannot see `pub(crate)` items. See its doc comment in `sections.rs` for the contract.
pub use sections::layout_wastes_plain_output;

#[cfg(test)]
pub(crate) use sections::validate_binarization_method;
pub(crate) use sections::{validate_image_preprocessing_config, validate_tesseract_oem, validate_tesseract_psm};

// `validate_output_format` stays `#[cfg(test)]`-only, and correctly so: both
// `ExtractionConfig::output_format` and `OcrConfig::output_format` are the strongly-typed
// `OutputFormat` enum, which rejects invalid values at deserialization time. There is no
// raw-string field left for this function to validate. ~keep
#[cfg(test)]
pub(crate) use sections::validate_output_format;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_binarization_method_valid() {
        assert!(validate_binarization_method("none").is_ok());
        assert!(validate_binarization_method("off").is_ok());
        assert!(validate_binarization_method("otsu").is_ok());
        assert!(validate_binarization_method("adaptive").is_ok());
        assert!(validate_binarization_method("sauvola").is_ok());
    }

    #[test]
    fn test_validate_binarization_method_case_insensitive() {
        assert!(validate_binarization_method("NONE").is_ok());
        assert!(validate_binarization_method("Off").is_ok());
        assert!(validate_binarization_method("OTSU").is_ok());
        assert!(validate_binarization_method("Adaptive").is_ok());
        assert!(validate_binarization_method("SAUVOLA").is_ok());
    }

    #[test]
    fn test_validate_binarization_method_invalid() {
        let result = validate_binarization_method("invalid");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid binarization method"));
        assert!(msg.contains("otsu"));
    }

    #[test]
    fn test_validate_token_reduction_level_valid() {
        assert!(validate_token_reduction_level("off").is_ok());
        assert!(validate_token_reduction_level("light").is_ok());
        assert!(validate_token_reduction_level("moderate").is_ok());
        assert!(validate_token_reduction_level("aggressive").is_ok());
        assert!(validate_token_reduction_level("maximum").is_ok());
    }

    #[test]
    fn test_validate_token_reduction_level_case_insensitive() {
        assert!(validate_token_reduction_level("OFF").is_ok());
        assert!(validate_token_reduction_level("Moderate").is_ok());
        assert!(validate_token_reduction_level("MAXIMUM").is_ok());
    }

    #[test]
    fn test_validate_token_reduction_level_invalid() {
        let result = validate_token_reduction_level("extreme");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid token reduction level"));
    }

    #[test]
    fn test_validate_ocr_backend_valid() {
        assert!(validate_ocr_backend("tesseract").is_ok());
        assert!(validate_ocr_backend("paddleocr").is_ok());
        assert!(validate_ocr_backend("sceptre").is_ok());
    }

    #[test]
    fn test_validate_ocr_backend_case_insensitive() {
        assert!(validate_ocr_backend("TESSERACT").is_ok());
        assert!(validate_ocr_backend("PADDLEOCR").is_ok());
        assert!(validate_ocr_backend("SCEPTRE").is_ok());
    }

    /// The four candle backends are registered by `OcrRegistry::register_builtin_backends`
    /// (plugins/registry/ocr.rs) under their own feature gates, but were absent from
    /// `VALID_OCR_BACKENDS` — so config validation rejected a backend the registry was
    /// perfectly capable of serving, before resolution ever ran.
    #[test]
    fn should_accept_every_name_a_builtin_candle_backend_registers_under() {
        assert!(validate_ocr_backend("candle-trocr").is_ok());
        assert!(validate_ocr_backend("candle-paddleocr-vl").is_ok());
        assert!(validate_ocr_backend("candle-glm-ocr").is_ok());
        assert!(validate_ocr_backend("candle-deepseek-ocr").is_ok());
    }

    #[test]
    fn test_validate_ocr_backend_rejects_unknown_backend() {
        let result = validate_ocr_backend("unsupported-ocr");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid OCR backend"));
    }

    #[test]
    fn test_validate_ocr_backend_invalid() {
        let result = validate_ocr_backend("invalid_backend");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid OCR backend"));
    }

    #[test]
    fn test_validate_language_code_valid_iso639_1() {
        assert!(validate_language_code("en").is_ok());
        assert!(validate_language_code("de").is_ok());
        assert!(validate_language_code("fr").is_ok());
        assert!(validate_language_code("es").is_ok());
        assert!(validate_language_code("zh").is_ok());
        assert!(validate_language_code("ja").is_ok());
        assert!(validate_language_code("ko").is_ok());
    }

    #[test]
    fn test_validate_language_code_valid_iso639_3() {
        assert!(validate_language_code("eng").is_ok());
        assert!(validate_language_code("deu").is_ok());
        assert!(validate_language_code("fra").is_ok());
        assert!(validate_language_code("spa").is_ok());
        assert!(validate_language_code("zho").is_ok());
        assert!(validate_language_code("jpn").is_ok());
        assert!(validate_language_code("jpn_vert").is_ok());
        assert!(validate_language_code("JPN_VERT").is_ok());
        assert!(validate_language_code("kor").is_ok());
        for language in ["afr", "aze", "bos", "bel", "kaz", "kir", "srp", "tgk"] {
            assert!(
                validate_language_code(language).is_ok(),
                "Sceptre language {language} should pass shared validation"
            );
        }
        for language in ["ch_sim", "rs_latin", "rs-cyrillic", "tel", "kan", "abq", "tjk"] {
            assert!(
                validate_language_code(language).is_ok(),
                "EasyOCR Gen2 language {language} should pass shared validation"
            );
        }
    }

    #[test]
    fn test_validate_language_code_case_insensitive() {
        assert!(validate_language_code("EN").is_ok());
        assert!(validate_language_code("ENG").is_ok());
        assert!(validate_language_code("De").is_ok());
        assert!(validate_language_code("DEU").is_ok());
    }

    #[test]
    fn test_validate_language_code_all_keyword() {
        assert!(validate_language_code("all").is_ok());
        assert!(validate_language_code("ALL").is_ok());
        assert!(validate_language_code("All").is_ok());
        assert!(validate_language_code("*").is_ok());
    }

    #[test]
    fn test_validate_language_code_invalid() {
        let result = validate_language_code("invalid");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid language code"));
        assert!(msg.contains("ISO 639"));
    }

    #[test]
    fn test_validate_tesseract_psm_valid() {
        for psm in 1..=13 {
            assert!(validate_tesseract_psm(psm).is_ok(), "PSM {} should be valid", psm);
        }
    }

    #[test]
    fn should_reject_tesseract_psm_zero_because_osd_only_recognises_no_text() {
        let error = validate_tesseract_psm(0).expect_err("PSM 0 is OSD-only and recognises no text");
        let message = error.to_string();
        assert!(
            message.contains('0'),
            "message should name the rejected value: {message}"
        );
        assert!(
            message.contains("orientation"),
            "message should say why PSM 0 cannot work: {message}"
        );
        assert!(message.contains('3'), "message should point at a usable PSM: {message}");
    }

    #[test]
    fn test_validate_tesseract_psm_invalid() {
        assert!(validate_tesseract_psm(-1).is_err());
        assert!(validate_tesseract_psm(14).is_err());
        assert!(validate_tesseract_psm(100).is_err());
    }

    #[test]
    fn test_validate_tesseract_oem_valid() {
        for oem in 0..=3 {
            assert!(validate_tesseract_oem(oem).is_ok(), "OEM {} should be valid", oem);
        }
    }

    #[test]
    fn test_validate_tesseract_oem_invalid() {
        assert!(validate_tesseract_oem(-1).is_err());
        assert!(validate_tesseract_oem(4).is_err());
        assert!(validate_tesseract_oem(10).is_err());
    }

    #[test]
    fn test_validate_output_format_valid() {
        assert!(validate_output_format("text").is_ok());
        assert!(validate_output_format("markdown").is_ok());
    }

    #[test]
    fn test_validate_output_format_case_insensitive() {
        assert!(validate_output_format("TEXT").is_ok());
        assert!(validate_output_format("Markdown").is_ok());
    }

    #[test]
    fn test_validate_output_format_invalid() {
        let result = validate_output_format("xml");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid output format"));
    }

    #[test]
    fn test_validate_confidence_valid() {
        assert!(validate_confidence(0.0).is_ok());
        assert!(validate_confidence(0.5).is_ok());
        assert!(validate_confidence(1.0).is_ok());
        assert!(validate_confidence(0.75).is_ok());
    }

    #[test]
    fn test_validate_confidence_invalid() {
        assert!(validate_confidence(-0.1).is_err());
        assert!(validate_confidence(1.1).is_err());
        assert!(validate_confidence(2.0).is_err());
    }

    #[test]
    fn test_validate_dpi_valid() {
        assert!(validate_dpi(72).is_ok());
        assert!(validate_dpi(96).is_ok());
        assert!(validate_dpi(300).is_ok());
        assert!(validate_dpi(600).is_ok());
        assert!(validate_dpi(1).is_ok());
    }

    #[test]
    fn test_validate_dpi_invalid() {
        assert!(validate_dpi(0).is_err());
        assert!(validate_dpi(-1).is_err());
        assert!(validate_dpi(2401).is_err());
    }

    #[test]
    fn test_validate_chunking_params_valid() {
        assert!(validate_chunking_params(1000, 200).is_ok());
        assert!(validate_chunking_params(500, 50).is_ok());
        assert!(validate_chunking_params(1, 0).is_ok());
    }

    #[test]
    fn test_validate_chunking_params_zero_chars() {
        let result = validate_chunking_params(0, 100);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_chars"));
    }

    #[test]
    fn test_validate_chunking_params_overlap_too_large() {
        let result = validate_chunking_params(100, 100);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("overlap"));

        let result = validate_chunking_params(100, 150);
        assert!(result.is_err());
    }

    #[test]
    fn test_layout_wastes_plain_output_true_when_layout_enabled_and_format_plain() {
        assert!(layout_wastes_plain_output(true, &crate::OutputFormat::Plain));
    }

    #[test]
    fn test_layout_wastes_plain_output_false_when_format_is_structured() {
        for format in [
            crate::OutputFormat::Markdown,
            crate::OutputFormat::Djot,
            crate::OutputFormat::Html,
            crate::OutputFormat::Json,
            crate::OutputFormat::DocTags,
        ] {
            assert!(
                !layout_wastes_plain_output(true, &format),
                "layout + {format:?} should not be flagged as wasted"
            );
        }
    }

    #[test]
    fn test_layout_wastes_plain_output_false_when_layout_disabled() {
        assert!(!layout_wastes_plain_output(false, &crate::OutputFormat::Plain));
    }

    #[test]
    fn test_error_messages_are_helpful() {
        let err = validate_binarization_method("bad").unwrap_err().to_string();
        assert!(err.contains("otsu"));
        assert!(err.contains("adaptive"));
        assert!(err.contains("sauvola"));

        let err = validate_token_reduction_level("bad").unwrap_err().to_string();
        assert!(err.contains("off"));
        assert!(err.contains("moderate"));

        let err = validate_language_code("bad").unwrap_err().to_string();
        assert!(err.contains("ISO 639"));
        assert!(err.contains("en"));
    }
}
