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
pub async fn run_pipeline(mut doc: InternalDocument, config: &ExtractionConfig) -> Result<ExtractionResult> {
    // 1. Process extracted images with OCR if configured
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
    if config.ocr.is_some() && !doc.images.is_empty() {
        let images_to_process = std::mem::take(&mut doc.images);
        match crate::extraction::image_ocr::process_images_with_ocr(
            images_to_process,
            config,
            &mut doc.processing_warnings,
        )
        .await
        {
            Ok(processed) => {
                doc.images = processed;
            }
            Err(e) => {
                doc.processing_warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("image_ocr"),
                    message: std::borrow::Cow::Owned(format!("Image OCR failed: {e}")),
                });
            }
        }
    }

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

    // Pre-render styled HTML before `doc` is consumed by `derive_extraction_result`.
    // When `html` is active and the caller has configured `html_output`, we
    // render the document here and inject the result after derivation.
    #[cfg(feature = "html")]
    let styled_html_prerender: Option<String> = {
        use crate::plugins::Renderer as _;
        if config.output_format == crate::core::config::OutputFormat::Html {
            config.html_output.as_ref().and_then(|html_cfg| {
                match crate::rendering::StyledHtmlRenderer::new(html_cfg.clone()) {
                    Ok(renderer) => match renderer.render(&doc) {
                        Ok(html) => Some(html),
                        Err(e) => {
                            tracing::warn!("StyledHtmlRenderer render failed, falling back to default HTML: {e}");
                            None
                        }
                    },
                    Err(e) => {
                        tracing::warn!("StyledHtmlRenderer construction failed, falling back to default HTML: {e}");
                        None
                    }
                }
            })
        } else {
            None
        }
    };

    // 2. Derive ExtractionResult from InternalDocument
    let include_structure = config.include_document_structure;
    let mut result =
        crate::extraction::derive::derive_extraction_result(doc, include_structure, config.output_format.clone());

    // Inject pre-rendered styled HTML (overrides the default render_html output).
    #[cfg(feature = "html")]
    if let Some(html) = styled_html_prerender {
        result.formatted_content = Some(html);
    }

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

    // Run LLM-based structured extraction BEFORE output formatting
    // so extraction sees plain text, not markdown/HTML
    #[cfg(all(feature = "liter-llm", not(target_os = "windows")))]
    if let Some(ref structured_config) = config.structured_extraction {
        match crate::llm::structured::extract_structured(&result.content, structured_config).await {
            Ok((output, usage)) => {
                result.structured_output = Some(output);
                crate::llm::usage::push_llm_usage(&mut result, usage);
            }
            Err(e) => {
                tracing::warn!("Structured extraction failed: {e}");
                result.processing_warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("structured_extraction"),
                    message: std::borrow::Cow::Owned(format!("Structured extraction failed: {e}")),
                });
            }
        }
    }

    #[cfg(any(not(feature = "liter-llm"), target_os = "windows"))]
    if config.structured_extraction.is_some() {
        result.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("structured_extraction"),
            message: std::borrow::Cow::Borrowed("Structured extraction requires the 'liter-llm' feature"),
        });
    }

    // Apply output format conversion as the final step
    result = apply_output_format(result, config.output_format.clone());

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

    // Pre-render styled HTML before `doc` is consumed (mirrors async path).
    #[cfg(feature = "html")]
    let styled_html_prerender: Option<String> = {
        use crate::plugins::Renderer as _;
        if config.output_format == crate::core::config::OutputFormat::Html {
            config.html_output.as_ref().and_then(|html_cfg| {
                match crate::rendering::StyledHtmlRenderer::new(html_cfg.clone()) {
                    Ok(renderer) => match renderer.render(&doc) {
                        Ok(html) => Some(html),
                        Err(e) => {
                            tracing::warn!("StyledHtmlRenderer render failed, falling back to default HTML: {e}");
                            None
                        }
                    },
                    Err(e) => {
                        tracing::warn!("StyledHtmlRenderer construction failed, falling back to default HTML: {e}");
                        None
                    }
                }
            })
        } else {
            None
        }
    };

    // 1. Derive ExtractionResult from InternalDocument
    let include_structure = config.include_document_structure;
    let mut result =
        crate::extraction::derive::derive_extraction_result(doc, include_structure, config.output_format.clone());

    // Inject pre-rendered styled HTML.
    #[cfg(feature = "html")]
    if let Some(html) = styled_html_prerender {
        result.formatted_content = Some(html);
    }

    #[cfg(feature = "chunking")]
    let chunker_only_markdown = result.formatted_content.is_none();
    #[cfg(feature = "chunking")]
    if chunker_only_markdown && let Some(md) = chunker_heading_source {
        result.formatted_content = Some(md);
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
    result = apply_output_format(result, config.output_format.clone());

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
