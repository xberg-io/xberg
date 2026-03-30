//! Post-processing pipeline orchestration.
//!
//! This module orchestrates the post-processing pipeline, executing validators,
//! quality processing, chunking, and custom hooks in the correct order.

mod cache;
mod execution;
mod features;
mod format;
mod initialization;

#[cfg(test)]
mod tests;

pub use cache::clear_processor_cache;
pub use format::apply_output_format;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::types::ExtractionResult;
use crate::types::internal::InternalDocument;

use execution::{execute_processors, execute_validators};
use features::{execute_chunking, execute_language_detection, execute_token_reduction};
use initialization::{get_processors_from_cache, initialize_features, initialize_processor_cache};

/// Run the post-processing pipeline on an `InternalDocument`.
///
/// Derives `ExtractionResult` from `InternalDocument` via the derivation pipeline,
/// then executes post-processing in the following order:
/// 1. Post-Processors - Execute by stage (Early, Middle, Late) to modify/enhance the result
/// 2. Quality Processing - Text cleaning and quality scoring
/// 3. Chunking - Text splitting if enabled
/// 4. Validators - Run validation hooks on the processed result (can fail fast)
///
/// # Arguments
///
/// * `doc` - The internal document produced by the extractor
/// * `config` - Extraction configuration
///
/// # Returns
///
/// The processed extraction result.
///
/// # Errors
///
/// - Validator errors bubble up immediately
/// - Post-processor errors are caught and recorded in metadata
/// - System errors (IO, RuntimeError equivalents) always bubble up
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(doc, config),
    fields(
        pipeline.stage = "post_processing",
        content.element_count = doc.elements.len(),
    )
))]
pub async fn run_pipeline(doc: InternalDocument, config: &ExtractionConfig) -> Result<ExtractionResult> {
    // Pre-render markdown for the chunker's heading context resolution when:
    // - Markdown chunking is configured
    // - Output format is not already Markdown (which would produce formatted_content anyway)
    // Plain-text rendering strips heading markers, so the markdown chunker needs
    // a separate markdown rendering to build the heading hierarchy for chunk metadata.
    #[cfg(feature = "chunking")]
    let chunker_heading_source = {
        let needs_markdown = config.chunking.as_ref().is_some_and(|c| {
            c.chunker_type == crate::core::config::ChunkerType::Markdown
                || c.resolve_preset().chunker_type == crate::core::config::ChunkerType::Markdown
        }) && config.output_format == crate::core::config::OutputFormat::Plain;
        if needs_markdown {
            Some(crate::rendering::render_markdown(&doc))
        } else {
            None
        }
    };

    // 1. Derive ExtractionResult from InternalDocument
    let include_structure = config.include_document_structure;
    let mut result = crate::extraction::derive::derive_extraction_result(doc, include_structure, config.output_format);

    // Temporarily store pre-rendered markdown for chunker heading context.
    // Tracked separately so we can remove it after chunking — apply_output_format
    // must not swap this into result.content when output_format is Plain.
    #[cfg(feature = "chunking")]
    let chunker_only_markdown = result.formatted_content.is_none();
    #[cfg(feature = "chunking")]
    if chunker_only_markdown && let Some(md) = chunker_heading_source {
        result.formatted_content = Some(md);
    }

    // 2. Run post-processing pipeline
    let pp_config = config.postprocessor.as_ref();
    let postprocessing_enabled = pp_config.is_none_or(|c| c.enabled);

    if postprocessing_enabled {
        initialize_features();
        initialize_processor_cache()?;

        let (early_processors, middle_processors, late_processors) = get_processors_from_cache()?;

        execute_processors(
            &mut result,
            config,
            &pp_config,
            early_processors,
            middle_processors,
            late_processors,
        )
        .await?;
    }

    execute_chunking(&mut result, config)?;

    // Clear temporary markdown if it was only stored for chunker heading context.
    // This prevents apply_output_format from swapping it into result.content.
    #[cfg(feature = "chunking")]
    if chunker_only_markdown {
        result.formatted_content = None;
    }

    execute_language_detection(&mut result, config)?;
    execute_token_reduction(&mut result, config)?;
    execute_validators(&result, config).await?;

    apply_element_transform(&mut result, config);
    normalize_nfc(&mut result);

    // Apply output format conversion as the final step
    apply_output_format(&mut result, config.output_format);

    Ok(result)
}

/// Run the post-processing pipeline synchronously (WASM-compatible version).
///
/// This is a synchronous implementation for WASM and non-async contexts.
/// It performs a subset of the full async pipeline, excluding async post-processors
/// and validators.
///
/// # Arguments
///
/// * `doc` - The internal document produced by the extractor
/// * `config` - Extraction configuration
///
/// # Returns
///
/// The processed extraction result.
///
/// # Notes
///
/// This function is only available when the `tokio-runtime` feature is disabled.
/// It handles:
/// - Quality processing (if enabled)
/// - Chunking (if enabled)
/// - Language detection (if enabled)
///
/// It does NOT handle:
/// - Async post-processors
/// - Async validators
#[cfg(not(feature = "tokio-runtime"))]
pub fn run_pipeline_sync(doc: InternalDocument, config: &ExtractionConfig) -> Result<ExtractionResult> {
    // Pre-render markdown for chunker heading context (same logic as async path).
    #[cfg(feature = "chunking")]
    let chunker_heading_source = {
        let needs_markdown = config.chunking.as_ref().is_some_and(|c| {
            c.chunker_type == crate::core::config::ChunkerType::Markdown
                || c.resolve_preset().chunker_type == crate::core::config::ChunkerType::Markdown
        }) && config.output_format == crate::core::config::OutputFormat::Plain;
        if needs_markdown {
            Some(crate::rendering::render_markdown(&doc))
        } else {
            None
        }
    };

    // 1. Derive ExtractionResult from InternalDocument
    let include_structure = config.include_document_structure;
    let mut result = crate::extraction::derive::derive_extraction_result(doc, include_structure, config.output_format);

    #[cfg(feature = "chunking")]
    let chunker_only_markdown = result.formatted_content.is_none();
    #[cfg(feature = "chunking")]
    if chunker_only_markdown {
        if let Some(md) = chunker_heading_source {
            result.formatted_content = Some(md);
        }
    }

    // 2. Run synchronous post-processing
    execute_chunking(&mut result, config)?;

    #[cfg(feature = "chunking")]
    if chunker_only_markdown {
        result.formatted_content = None;
    }

    execute_language_detection(&mut result, config)?;
    execute_token_reduction(&mut result, config)?;

    apply_element_transform(&mut result, config);
    normalize_nfc(&mut result);

    // Apply output format conversion as the final step
    apply_output_format(&mut result, config.output_format);

    Ok(result)
}

/// Transform to element-based output if requested by the config.
fn apply_element_transform(result: &mut ExtractionResult, config: &ExtractionConfig) {
    if config.result_format == crate::types::OutputFormat::ElementBased {
        result.elements = Some(crate::extraction::transform::transform_extraction_result_to_elements(
            result,
        ));
    }
}

/// Apply NFC unicode normalization to all text content.
///
/// Ensures consistent representation of composed characters (e.g., é vs e+combining accent)
/// across all extraction backends (PDF, OCR, DOCX, HTML, etc.).
fn normalize_nfc(result: &mut ExtractionResult) {
    #[cfg(feature = "quality")]
    {
        use unicode_normalization::UnicodeNormalization;
        result.content = result.content.nfc().collect();
        if let Some(pages) = result.pages.as_mut() {
            for page in pages.iter_mut() {
                page.content = page.content.nfc().collect();
            }
        }
    }
    // Suppress unused variable warning when quality feature is disabled
    let _ = result;
}
