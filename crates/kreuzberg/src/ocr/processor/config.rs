//! Configuration hashing and Tesseract variable management.
//!
//! This module handles configuration hashing for caching and
//! setting Tesseract variables.

use crate::ocr::error::OcrError;
use crate::ocr::types::TesseractConfig;
use kreuzberg_tesseract::TesseractAPI;

/// Compute a deterministic hash of the OCR configuration.
///
/// This hash is used as part of the cache key to ensure different
/// configurations produce different cached results.
///
/// # Arguments
///
/// * `config` - Configuration to hash
///
/// # Returns
///
/// Hexadecimal string representation of the configuration hash
pub(super) fn hash_config(config: &TesseractConfig) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(config.language.as_bytes());
    hasher.update(&config.psm.to_le_bytes());
    hasher.update(config.output_format.as_bytes());
    hasher.update(&[config.enable_table_detection as u8]);
    hasher.update(&config.table_min_confidence.to_bits().to_le_bytes());
    hasher.update(&config.table_column_threshold.to_le_bytes());
    hasher.update(&config.table_row_threshold_ratio.to_bits().to_le_bytes());
    hasher.update(&[config.classify_use_pre_adapted_templates as u8]);
    hasher.update(&[config.language_model_ngram_on as u8]);
    hasher.update(&[config.tessedit_dont_blkrej_good_wds as u8]);
    hasher.update(&[config.tessedit_dont_rowrej_good_wds as u8]);
    hasher.update(&[config.tessedit_enable_dict_correction as u8]);
    hasher.update(config.tessedit_char_whitelist.as_bytes());
    hasher.update(&[config.tessedit_use_primary_params_model as u8]);
    hasher.update(&[config.textord_space_size_is_variable as u8]);
    hasher.update(&[config.thresholding_method as u8]);

    let hash = hasher.finalize();
    hex::encode(&hash.as_bytes()[..16])
}

/// Apply Tesseract configuration variables to the API.
///
/// Sets all the advanced Tesseract variables from the configuration.
///
/// # Arguments
///
/// * `api` - Tesseract API instance
/// * `config` - Configuration with variables to apply
///
/// # Returns
///
/// `Ok(())` if all variables were set successfully, otherwise an error
pub(super) fn apply_tesseract_variables(api: &TesseractAPI, config: &TesseractConfig) -> Result<(), OcrError> {
    api.set_variable(
        "classify_use_pre_adapted_templates",
        &config.classify_use_pre_adapted_templates.to_string(),
    )
    .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set classify_use_pre_adapted_templates: {}", e)))?;

    api.set_variable("language_model_ngram_on", &config.language_model_ngram_on.to_string())
        .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set language_model_ngram_on: {}", e)))?;

    api.set_variable(
        "tessedit_dont_blkrej_good_wds",
        &config.tessedit_dont_blkrej_good_wds.to_string(),
    )
    .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_dont_blkrej_good_wds: {}", e)))?;

    api.set_variable(
        "tessedit_dont_rowrej_good_wds",
        &config.tessedit_dont_rowrej_good_wds.to_string(),
    )
    .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_dont_rowrej_good_wds: {}", e)))?;

    api.set_variable(
        "tessedit_enable_dict_correction",
        &config.tessedit_enable_dict_correction.to_string(),
    )
    .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_enable_dict_correction: {}", e)))?;

    if !config.tessedit_char_whitelist.is_empty() {
        api.set_variable("tessedit_char_whitelist", &config.tessedit_char_whitelist)
            .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_char_whitelist: {}", e)))?;
    }

    api.set_variable(
        "tessedit_use_primary_params_model",
        &config.tessedit_use_primary_params_model.to_string(),
    )
    .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_use_primary_params_model: {}", e)))?;

    api.set_variable(
        "textord_space_size_is_variable",
        &config.textord_space_size_is_variable.to_string(),
    )
    .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set textord_space_size_is_variable: {}", e)))?;

    api.set_variable("thresholding_method", &config.thresholding_method.to_string())
        .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set thresholding_method: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> TesseractConfig {
        TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        }
    }

    #[test]
    fn test_hash_config_deterministic() {
        let config = create_test_config();

        let hash1 = hash_config(&config);
        let hash2 = hash_config(&config);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_hash_config_different_languages() {
        let mut config1 = create_test_config();
        config1.language = "eng".to_string();

        let mut config2 = create_test_config();
        config2.language = "fra".to_string();

        let hash1 = hash_config(&config1);
        let hash2 = hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_psm() {
        let mut config1 = create_test_config();
        config1.psm = 3;

        let mut config2 = create_test_config();
        config2.psm = 6;

        let hash1 = hash_config(&config1);
        let hash2 = hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_output_format() {
        let mut config1 = create_test_config();
        config1.output_format = "text".to_string();

        let mut config2 = create_test_config();
        config2.output_format = "markdown".to_string();

        let hash1 = hash_config(&config1);
        let hash2 = hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_table_detection_flag() {
        let mut config1 = create_test_config();
        config1.enable_table_detection = false;

        let mut config2 = create_test_config();
        config2.enable_table_detection = true;

        let hash1 = hash_config(&config1);
        let hash2 = hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_whitelist() {
        let mut config1 = create_test_config();
        config1.tessedit_char_whitelist = "".to_string();

        let mut config2 = create_test_config();
        config2.tessedit_char_whitelist = "0123456789".to_string();

        let hash1 = hash_config(&config1);
        let hash2 = hash_config(&config2);

        assert_ne!(hash1, hash2);
    }
}
