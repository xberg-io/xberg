//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.

use anyhow::{Context, Result};
use kreuzberg::{
    ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, OcrConfig, batch_extract_file_sync, extract_file_sync,
};
use std::path::PathBuf;

use crate::{ContentOutputFormatArg, OutputFormat};

/// Execute single document extraction command
pub fn extract_command(
    path: PathBuf,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    let result = extract_file_sync(&path_str, mime_type.as_deref(), &config).with_context(|| {
        format!(
            "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
            path.display()
        )
    })?;

    match format {
        OutputFormat::Text => {
            println!("{}", result.content);
        }
        OutputFormat::Json => {
            // Serialize the full ExtractionResult including chunks, images, elements, etc.
            println!(
                "{}",
                serde_json::to_string_pretty(&result).context("Failed to serialize extraction result to JSON")?
            );
        }
    }

    Ok(())
}

/// Execute batch extraction command
pub fn batch_command(paths: Vec<PathBuf>, config: ExtractionConfig, format: OutputFormat) -> Result<()> {
    let path_strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

    let results = batch_extract_file_sync(path_strs, &config).with_context(|| {
        format!(
            "Failed to batch extract {} documents. Check that all files are readable and formats are supported.",
            paths.len()
        )
    })?;

    match format {
        OutputFormat::Text => {
            for (i, result) in results.iter().enumerate() {
                println!("=== Document {} ===", i + 1);
                println!("MIME Type: {}", result.mime_type);
                println!("Content:\n{}", result.content);
                println!();
            }
        }
        OutputFormat::Json => {
            // Serialize the full ExtractionResult for each document
            println!(
                "{}",
                serde_json::to_string_pretty(&results)
                    .context("Failed to serialize batch extraction results to JSON")?
            );
        }
    }

    Ok(())
}

/// Apply extraction CLI overrides to config
///
/// # Deprecation Notices
///
/// - `output_format` (via `--output-format`): Recommended for all new code
/// - `content_format` (via `--content-format`): Deprecated since 4.2.0, use `--output-format` instead
#[allow(clippy::too_many_arguments)]
pub fn apply_extraction_overrides(
    config: &mut ExtractionConfig,
    ocr: Option<bool>,
    ocr_backend: Option<&str>,
    ocr_language: Option<&str>,
    force_ocr: Option<bool>,
    no_cache: Option<bool>,
    chunk: Option<bool>,
    chunk_size: Option<usize>,
    chunk_overlap: Option<usize>,
    chunking_tokenizer: Option<&str>,
    quality: Option<bool>,
    detect_language: Option<bool>,
    output_format: Option<ContentOutputFormatArg>,
    content_format: Option<ContentOutputFormatArg>,
) {
    if let Some(ocr_flag) = ocr {
        if ocr_flag {
            let backend = match ocr_backend {
                Some("paddle-ocr") => "paddle-ocr",
                Some("easyocr") => "easyocr",
                _ => "tesseract",
            };
            let language = match ocr_language {
                Some(lang) => lang.to_string(),
                None => match backend {
                    "paddle-ocr" | "easyocr" => "en".to_string(),
                    _ => "eng".to_string(),
                },
            };
            // Preserve existing paddle_ocr_config and element_config from config file/inline JSON
            let existing_paddle_config = config.ocr.as_ref().and_then(|o| o.paddle_ocr_config.clone());
            let existing_element_config = config.ocr.as_ref().and_then(|o| o.element_config.clone());
            config.ocr = Some(OcrConfig {
                backend: backend.to_string(),
                language,
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: existing_paddle_config,
                element_config: existing_element_config,
            });
        } else {
            config.ocr = None;
        }
    }

    // Override language on existing OCR config when --ocr-language is used without --ocr
    if ocr.is_none()
        && let Some(lang) = ocr_language
        && let Some(ref mut existing_ocr) = config.ocr
    {
        existing_ocr.language = lang.to_string();
    }
    if let Some(force_ocr_flag) = force_ocr {
        config.force_ocr = force_ocr_flag;
    }
    if let Some(no_cache_flag) = no_cache {
        config.use_cache = !no_cache_flag;
    }
    // Handle --chunking-tokenizer: implicitly enables chunking with tokenizer sizing
    let chunk = if chunking_tokenizer.is_some() && chunk.is_none() {
        Some(true)
    } else {
        chunk
    };

    if let Some(chunk_flag) = chunk {
        if chunk_flag {
            let max_characters = chunk_size.unwrap_or(1000);
            let overlap = chunk_overlap.unwrap_or(200);
            let mut chunking_config = ChunkingConfig {
                max_characters,
                overlap,
                trim: true,
                chunker_type: kreuzberg::chunking::ChunkerType::Text,
                ..Default::default()
            };

            // Apply tokenizer sizing if specified
            #[cfg(feature = "chunking-tokenizers")]
            if let Some(model) = chunking_tokenizer {
                chunking_config.sizing = kreuzberg::chunking::ChunkSizing::Tokenizer {
                    model: model.to_string(),
                    cache_dir: None,
                };
            }

            config.chunking = Some(chunking_config);
        } else {
            config.chunking = None;
        }
    } else if let Some(ref mut chunking) = config.chunking {
        if let Some(max_characters) = chunk_size {
            chunking.max_characters = max_characters;
        }
        if let Some(overlap) = chunk_overlap {
            chunking.overlap = overlap;
        }

        // Apply tokenizer sizing to existing config
        #[cfg(feature = "chunking-tokenizers")]
        if let Some(model) = chunking_tokenizer {
            chunking.sizing = kreuzberg::chunking::ChunkSizing::Tokenizer {
                model: model.to_string(),
                cache_dir: None,
            };
        }
    }
    if let Some(quality_flag) = quality {
        config.enable_quality_processing = quality_flag;
    }
    if let Some(detect_language_flag) = detect_language {
        if detect_language_flag {
            config.language_detection = Some(LanguageDetectionConfig {
                enabled: true,
                min_confidence: 0.8,
                detect_multiple: false,
            });
        } else {
            config.language_detection = None;
        }
    }

    // Handle output format with deprecation warning for --content-format
    let final_output_format = output_format.or_else(|| {
        if content_format.is_some() {
            eprintln!("warning: '--content-format' is deprecated since 4.2.0, use '--output-format' instead");
        }
        content_format
    });

    if let Some(content_fmt) = final_output_format {
        config.output_format = content_fmt.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kreuzberg::ExtractionConfig;

    #[test]
    fn test_ocr_default_language_tesseract() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            Some(true),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "eng");
    }

    #[test]
    fn test_ocr_default_language_paddleocr() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            Some(true),
            Some("paddle-ocr"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "paddle-ocr");
        assert_eq!(ocr.language, "en");
    }

    #[test]
    fn test_ocr_default_language_easyocr() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            Some(true),
            Some("easyocr"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "easyocr");
        assert_eq!(ocr.language, "en");
    }

    #[test]
    fn test_ocr_language_override_tesseract() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            Some(true),
            None,
            Some("fra"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "fra");
    }

    #[test]
    fn test_ocr_language_override_paddleocr() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            Some(true),
            Some("paddle-ocr"),
            Some("ch"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "paddle-ocr");
        assert_eq!(ocr.language, "ch");
    }

    #[test]
    fn test_ocr_language_without_ocr_flag_no_existing_config() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            None,
            None,
            Some("deu"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        // No OCR config exists, so --ocr-language alone doesn't create one
        assert!(config.ocr.is_none());
    }

    #[test]
    fn test_ocr_language_without_ocr_flag_existing_config() {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
            }),
            ..Default::default()
        };
        apply_extraction_overrides(
            &mut config,
            None,
            None,
            Some("deu"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "deu");
    }

    #[test]
    fn test_ocr_disabled_ignores_language() {
        let mut config = ExtractionConfig::default();
        apply_extraction_overrides(
            &mut config,
            Some(false),
            None,
            Some("fra"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(config.ocr.is_none());
    }
}
