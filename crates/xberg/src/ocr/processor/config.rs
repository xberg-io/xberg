//! Configuration hashing and Tesseract variable management.
//!
//! This module handles configuration hashing for caching and
//! setting Tesseract variables.

use crate::ocr::error::OcrError;
use crate::ocr::types::TesseractConfig;
use xberg_tesseract::TesseractAPI;

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
    hash_bytes(&mut hasher, config.language.as_bytes());
    hasher.update(&config.psm.to_le_bytes());
    hasher.update(&config.oem.to_le_bytes());
    hasher.update(&config.min_confidence.to_bits().to_le_bytes());
    hash_bytes(&mut hasher, config.output_format.as_bytes());
    match config.preprocessing.as_ref() {
        Some(preprocessing) => {
            hasher.update(&[1]);
            hasher.update(&preprocessing.target_dpi.to_le_bytes());
            hasher.update(&[
                preprocessing.auto_rotate as u8,
                preprocessing.deskew as u8,
                preprocessing.denoise as u8,
                preprocessing.contrast_enhance as u8,
                preprocessing.invert_colors as u8,
            ]);
            hash_bytes(&mut hasher, preprocessing.binarization_method.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(&[config.enable_table_detection as u8]);
    hasher.update(&config.table_min_confidence.to_bits().to_le_bytes());
    hasher.update(&config.table_column_threshold.to_le_bytes());
    hasher.update(&config.table_row_threshold_ratio.to_bits().to_le_bytes());
    hasher.update(&[config.classify_use_pre_adapted_templates as u8]);
    hasher.update(&[config.language_model_ngram_on as u8]);
    hasher.update(&[config.tessedit_dont_blkrej_good_wds as u8]);
    hasher.update(&[config.tessedit_dont_rowrej_good_wds as u8]);
    hasher.update(&[config.tessedit_enable_dict_correction as u8]);
    hash_bytes(&mut hasher, config.tessedit_char_whitelist.as_bytes());
    hash_bytes(&mut hasher, config.tessedit_char_blacklist.as_bytes());
    hasher.update(&[config.tessedit_use_primary_params_model as u8]);
    hasher.update(&[config.textord_space_size_is_variable as u8]);
    hasher.update(&[config.thresholding_method as u8]);
    hasher.update(&[config.auto_rotate as u8]);
    match config.tessdata_path.as_ref() {
        Some(path) => {
            hasher.update(&[1]);
            hash_bytes(&mut hasher, path.as_os_str().as_encoded_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }

    let hash = hasher.finalize();
    hex::encode(&hash.as_bytes()[..16])
}

fn hash_bytes(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
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

    for (name, value) in character_variables(config) {
        api.set_variable(name, value)
            .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set {name}: {e}")))?;
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

fn character_variables(config: &TesseractConfig) -> [(&'static str, &str); 2] {
    [
        ("tessedit_char_whitelist", &config.tessedit_char_whitelist),
        ("tessedit_char_blacklist", &config.tessedit_char_blacklist),
    ]
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

    #[test]
    fn test_hash_config_blacklist() {
        let config1 = create_test_config();
        let mut config2 = create_test_config();
        config2.tessedit_char_blacklist = "abc".to_string();

        assert_ne!(hash_config(&config1), hash_config(&config2));
    }

    #[test]
    fn test_hash_config_frames_whitelist_and_blacklist() {
        let mut config1 = create_test_config();
        config1.tessedit_char_whitelist = "ab".to_string();
        config1.tessedit_char_blacklist = "c".to_string();
        let mut config2 = create_test_config();
        config2.tessedit_char_whitelist = "a".to_string();
        config2.tessedit_char_blacklist = "bc".to_string();

        assert_ne!(hash_config(&config1), hash_config(&config2));
    }

    #[test]
    fn test_character_variables_include_empty_resets() {
        let mut configured = create_test_config();
        configured.tessedit_char_whitelist = "0123456789".to_string();
        configured.tessedit_char_blacklist = "abc".to_string();
        let empty = create_test_config();

        assert_eq!(character_variables(&configured)[0].1, "0123456789");
        assert_eq!(character_variables(&configured)[1].1, "abc");
        assert_eq!(character_variables(&empty)[0].1, "");
        assert_eq!(character_variables(&empty)[1].1, "");
    }
}
