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

use execution::{execute_processors, execute_validators};
use features::{execute_chunking, execute_language_detection};
use initialization::{get_processors_from_cache, initialize_features, initialize_processor_cache};

/// Run the post-processing pipeline on an extraction result.
///
/// Executes post-processing in the following order:
/// 1. Post-Processors - Execute by stage (Early, Middle, Late) to modify/enhance the result
/// 2. Quality Processing - Text cleaning and quality scoring
/// 3. Chunking - Text splitting if enabled
/// 4. Validators - Run validation hooks on the processed result (can fail fast)
///
/// # Arguments
///
/// * `result` - The extraction result to process
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
    skip(result, config),
    fields(
        pipeline.stage = "post_processing",
        content.length = result.content.len(),
    )
))]
pub async fn run_pipeline(mut result: ExtractionResult, config: &ExtractionConfig) -> Result<ExtractionResult> {
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
    execute_language_detection(&mut result, config)?;
    execute_validators(&result, config).await?;

    // Transform to element-based output if requested
    if config.result_format == crate::types::OutputFormat::ElementBased {
        result.elements = Some(crate::extraction::transform::transform_extraction_result_to_elements(
            &result,
        ));
    }

    // Transform to structured document tree if requested (only if not already populated by extractor)
    if config.include_document_structure && result.document.is_none() {
        result.document = Some(crate::extraction::transform::transform_to_document_structure(&result));
    }

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
/// * `result` - The extraction result to process
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
pub fn run_pipeline_sync(mut result: ExtractionResult, config: &ExtractionConfig) -> Result<ExtractionResult> {
    execute_chunking(&mut result, config)?;
    execute_language_detection(&mut result, config)?;

    // Transform to element-based output if requested
    if config.result_format == crate::types::OutputFormat::ElementBased {
        result.elements = Some(crate::extraction::transform::transform_extraction_result_to_elements(
            &result,
        ));
    }

    // Transform to structured document tree if requested (only if not already populated by extractor)
    if config.include_document_structure && result.document.is_none() {
        result.document = Some(crate::extraction::transform::transform_to_document_structure(&result));
    }

    // Apply output format conversion as the final step
    apply_output_format(&mut result, config.output_format);

    Ok(result)
}
