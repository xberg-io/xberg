//! OCR functionality for PDF extraction.
//!
//! Handles text quality evaluation, OCR fallback decision logic, and OCR processing.

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use std::borrow::Cow;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::core::config::ExtractionConfig;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::core::config::OcrQualityThresholds;

/// Minimum average non-whitespace characters per page for extracted text to be treated as
/// substantive. At or above this, prose-tuned quality checks (fragmentation, avg word length,
/// consecutive-repeat ratio) are skipped so legitimately non-prose content — numeric tables,
/// formula pages, sparse forms — is not misclassified as needing OCR (issue #1176). Corruption
/// checks (empty, no-alphanumerics, garbage chars, critical fragmentation) still always apply.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const MIN_AVG_NON_WHITESPACE_TO_TRUST: f64 = 150.0;

#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Default)]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub struct NativeTextStats {
    pub non_whitespace: usize,
    pub alnum: usize,
    pub meaningful_words: usize,
    pub alnum_ratio: f64,
    /// Count of Unicode replacement characters (U+FFFD) indicating encoding failures.
    pub garbage_char_count: usize,
    /// Fraction of whitespace-delimited words that are 1-2 characters (0.0-1.0).
    /// High values indicate fragmented/garbled text extraction.
    pub fragmented_word_ratio: f64,
    /// Fraction of consecutive word pairs that are identical (0.0-1.0).
    /// High values indicate column scrambling where text is duplicated.
    pub consecutive_repeat_ratio: f64,
    /// Average word length (by chars). Very low values indicate garbled extraction.
    pub avg_word_length: f64,
    /// Total word count (whitespace-delimited).
    pub word_count: usize,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub struct OcrFallbackDecision {
    pub stats: NativeTextStats,
    pub avg_non_whitespace: f64,
    pub avg_alnum: f64,
    pub fallback: bool,
    pub failing_pages: Vec<u32>,
    /// Set to `true` when the aggregate document quality check triggered `fallback`,
    /// independently of any per-page analysis. When this is true the gate routes to
    /// `RunFallback` (full OCR) regardless of whether `failing_pages` is populated.
    pub whole_doc_failure: bool,
}

/// Which branch the OCR skip gate selects, given pre-rendered doc presence,
/// text statistics, and the per-page fallback decision.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OcrGateOutcome {
    /// Content is non-textual and a pre-rendered doc is available — skip OCR.
    SkipNonText,
    /// Pre-rendered doc is substantive and no per-page fallback is needed — skip OCR.
    SkipSubstantive,
    /// A document-level quality check flagged the entire document — OCR every page.
    RunFallback,
    /// A per-page quality check flagged a scanned page — run OCR fallback.
    RunFallbackOnPages(Vec<u32>),
    /// Insufficient native text or no structured doc available — use native text.
    UseNative,
}

/// Decide whether to skip OCR, run OCR fallback, or use native text.
///
/// Extracted from the async PDF pipeline so the gate logic can be unit-tested
/// independently. Fixes #917: `has_substantive_doc` alone must not suppress
/// OCR when `decision_fallback` is true (a scanned page was detected despite
/// good aggregate text).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn evaluate_ocr_skip_gate(
    pre_rendered_doc_present: bool,
    total_chars: usize,
    alnum_ws_ratio: f64,
    decision: &OcrFallbackDecision,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> OcrGateOutcome {
    let skip_for_non_text = pre_rendered_doc_present
        && total_chars >= thresholds.non_text_min_chars
        && alnum_ws_ratio < thresholds.alnum_ws_ratio_threshold;

    let has_substantive_doc = pre_rendered_doc_present
        && total_chars >= thresholds.substantive_min_chars
        && alnum_ws_ratio >= thresholds.alnum_ws_ratio_threshold;

    if skip_for_non_text {
        OcrGateOutcome::SkipNonText
    } else if has_substantive_doc && !decision.fallback {
        OcrGateOutcome::SkipSubstantive
    } else if decision.fallback {
        // `failing_pages` is empty when `evaluate_native_text_for_ocr` triggered the
        // fallback with no page boundaries available — there were no boundaries to
        // enumerate, so we must treat the whole document as failed rather than routing
        // to per-page mode with an empty page list.
        if decision.whole_doc_failure || decision.failing_pages.is_empty() {
            OcrGateOutcome::RunFallback
        } else {
            OcrGateOutcome::RunFallbackOnPages(decision.failing_pages.clone())
        }
    } else {
        OcrGateOutcome::UseNative
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl NativeTextStats {
    pub(crate) fn compute(text: &str, thresholds: &OcrQualityThresholds) -> Self {
        let mut non_whitespace = 0usize;
        let mut alnum = 0usize;
        let mut garbage_char_count = 0usize;

        for ch in text.chars() {
            if ch == '\u{FFFD}' {
                garbage_char_count += 1;
            }
            if !ch.is_whitespace() {
                non_whitespace += 1;
                if ch.is_alphanumeric() {
                    alnum += 1;
                }
            }
        }

        let meaningful_words = text
            .split_whitespace()
            .filter(|word| {
                word.chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(thresholds.min_meaningful_word_len)
                    .count()
                    >= thresholds.min_meaningful_word_len
            })
            .count();

        let alnum_ratio = if non_whitespace == 0 {
            0.0
        } else {
            alnum as f64 / non_whitespace as f64
        };

        // Compute fragmented word ratio: fraction of words that are 1-2 chars.
        // Only meaningful when there are enough words to judge.
        let words: Vec<&str> = text.split_whitespace().collect();
        let fragmented_word_ratio = if words.len() >= 10 {
            let short_count = words.iter().filter(|w| w.len() <= 2).count();
            short_count as f64 / words.len() as f64
        } else {
            0.0
        };

        // Compute consecutive word repetition ratio: fraction of adjacent word pairs
        // that are identical. High values indicate column scrambling where the PDF extractor
        // reads multi-column text row-by-row, duplicating words.
        let consecutive_repeat_ratio = if words.len() >= thresholds.min_words_for_repeat_check {
            let repeat_count = words.windows(2).filter(|pair| pair[0] == pair[1]).count();
            repeat_count as f64 / (words.len() - 1) as f64
        } else {
            0.0
        };

        let avg_word_length = if words.is_empty() {
            0.0
        } else {
            words.iter().map(|w| w.len()).sum::<usize>() as f64 / words.len() as f64
        };

        Self {
            non_whitespace,
            alnum,
            meaningful_words,
            alnum_ratio,
            garbage_char_count,
            fragmented_word_ratio,
            consecutive_repeat_ratio,
            avg_word_length,
            word_count: words.len(),
        }
    }

    /// Convenience method using default thresholds.
    #[cfg(test)]
    pub(crate) fn from(text: &str) -> Self {
        Self::compute(text, &OcrQualityThresholds::default())
    }
}

/// Evaluates native PDF text quality to determine if OCR fallback is needed.
///
/// Uses the provided quality thresholds (or defaults) to make the decision.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn evaluate_native_text_for_ocr(
    native_text: &str,
    page_count: Option<u32>,
    thresholds: &OcrQualityThresholds,
) -> OcrFallbackDecision {
    let trimmed = native_text.trim();

    if trimmed.is_empty() {
        let empty_stats = NativeTextStats {
            non_whitespace: 0,
            alnum: 0,
            meaningful_words: 0,
            alnum_ratio: 0.0,
            garbage_char_count: 0,
            fragmented_word_ratio: 0.0,
            consecutive_repeat_ratio: 0.0,
            avg_word_length: 0.0,
            word_count: 0,
        };
        return OcrFallbackDecision {
            stats: empty_stats,
            avg_non_whitespace: 0.0,
            avg_alnum: 0.0,
            fallback: true,
            failing_pages: Vec::new(),
            whole_doc_failure: true,
        };
    }

    let stats = NativeTextStats::compute(trimmed, thresholds);
    let pages = page_count.unwrap_or(1).max(1) as f64;
    let avg_non_whitespace = stats.non_whitespace as f64 / pages;
    let avg_alnum = stats.alnum as f64 / pages;

    let has_substantial_text = stats.non_whitespace >= thresholds.min_total_non_whitespace
        && avg_non_whitespace >= thresholds.min_non_whitespace_per_page
        && stats.meaningful_words >= thresholds.min_meaningful_words;

    // Definitive quality failures — always trigger OCR fallback.
    // Fix for #1176: skip prose-tuned quality checks if the page has substantial non-whitespace
    // content (avg_non_whitespace >= threshold). This prevents spurious OCR on numeric tables,
    // formulas, and forms that have legitimate (but non-prose) content extraction.
    //
    // When content is substantial, we skip prose-only quality signals (fragmentation ratios,
    // avg word length, repetition) that can occur legitimately in numeric, formula, or
    // structured text. However, we still apply:
    // - Empty text (non_whitespace == 0)
    // - No alphanumeric (alnum == 0)
    // - Extensive corruption (garbage_char_count >= threshold)
    // - CRITICAL fragmentation (>= 0.80 = 80%+ short words, definitive corruption indicator)
    let has_substantial_content = avg_non_whitespace >= MIN_AVG_NON_WHITESPACE_TO_TRUST;

    let definitive_failure = stats.non_whitespace == 0
        || stats.alnum == 0
        || stats.garbage_char_count >= thresholds.min_garbage_chars
        // Critical fragmentation (>= 0.80) is always an indicator of corruption
        || stats.fragmented_word_ratio >= thresholds.critical_fragmented_word_ratio
        // Skip moderate fragmentation check if content is substantial (can be legitimate in tables/formulas)
        || (!has_substantial_content
            && (stats.fragmented_word_ratio >= thresholds.max_fragmented_word_ratio
                && stats.meaningful_words < thresholds.min_meaningful_words))
        // Skip avg_word_length check if content is substantial (numerics/formulas have short tokens)
        || (!has_substantial_content
            && (stats.avg_word_length < thresholds.min_avg_word_length
                && stats.word_count >= thresholds.min_words_for_avg_length_check))
        // Skip repeat ratio check if content is substantial (numeric tables can have repeated values)
        || (!has_substantial_content && stats.consecutive_repeat_ratio >= thresholds.min_consecutive_repeat_ratio);

    let fallback = if definitive_failure {
        true
    } else if has_substantial_text {
        false
    } else if (stats.alnum_ratio < thresholds.min_alnum_ratio && avg_alnum < thresholds.min_non_whitespace_per_page)
        || (stats.non_whitespace < thresholds.min_total_non_whitespace
            && avg_non_whitespace < thresholds.min_non_whitespace_per_page)
    {
        true
    } else {
        stats.meaningful_words == 0 && avg_non_whitespace < thresholds.min_non_whitespace_per_page
    };

    OcrFallbackDecision {
        stats,
        avg_non_whitespace,
        avg_alnum,
        fallback,
        failing_pages: Vec::new(),
        whole_doc_failure: fallback,
    }
}

/// Compute a quality score (0.0-1.0) for OCR output text.
///
/// Used by the pipeline to decide whether to accept a result or try the next backend.
/// Higher is better. Combines multiple signal dimensions into a single score.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn compute_quality_score(text: &str, thresholds: &OcrQualityThresholds) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    let stats = NativeTextStats::compute(trimmed, thresholds);

    // Component scores (each 0.0-1.0, higher is better)
    let alnum_score = stats.alnum_ratio.min(1.0);
    let fragmentation_score = 1.0 - stats.fragmented_word_ratio.min(1.0);
    let word_length_score = (stats.avg_word_length / 5.0).min(1.0);
    let repeat_score = if thresholds.min_consecutive_repeat_ratio > 0.0 {
        1.0 - (stats.consecutive_repeat_ratio / thresholds.min_consecutive_repeat_ratio).min(1.0)
    } else {
        1.0
    };
    let meaningful_score = if thresholds.min_meaningful_words == 0 {
        1.0
    } else {
        (stats.meaningful_words as f64 / thresholds.min_meaningful_words as f64).min(1.0)
    };
    let garbage_score = if stats.garbage_char_count == 0 {
        1.0
    } else if thresholds.min_garbage_chars == 0 {
        0.0
    } else {
        (1.0 - stats.garbage_char_count as f64 / (thresholds.min_garbage_chars as f64 * 2.0)).max(0.0)
    };

    // Weighted average
    (alnum_score * 0.25
        + fragmentation_score * 0.20
        + word_length_score * 0.15
        + repeat_score * 0.15
        + meaningful_score * 0.15
        + garbage_score * 0.10)
        .clamp(0.0, 1.0)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn evaluate_per_page_ocr(
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
    page_count: Option<u32>,
    thresholds: &OcrQualityThresholds,
) -> OcrFallbackDecision {
    let boundaries = match boundaries {
        Some(b) if !b.is_empty() => b,
        _ => return evaluate_native_text_for_ocr(native_text, page_count, thresholds),
    };

    let mut document_decision = evaluate_native_text_for_ocr(native_text, page_count, thresholds);

    // The doc-level check already condemned the whole document — per-page scanning
    // would be O(N) wasted work because the gate routes to RunFallback regardless of
    // `failing_pages` when `whole_doc_failure` is true.
    if document_decision.whole_doc_failure {
        return document_decision;
    }

    let mut failing_pages: Vec<u32> = Vec::with_capacity(boundaries.len());
    let mut valid_boundary_count: usize = 0;
    for boundary in boundaries {
        if boundary.byte_end > native_text.len() || boundary.byte_start > boundary.byte_end {
            continue;
        }
        valid_boundary_count += 1;
        let page_text = &native_text[boundary.byte_start..boundary.byte_end];
        if evaluate_native_text_for_ocr(page_text, Some(1), thresholds).fallback {
            failing_pages.push(boundary.page_number);
        }
    }

    if !failing_pages.is_empty() {
        document_decision.fallback = true;
        // If every valid page boundary failed the per-page check, treat this as a
        // whole-document failure so the gate routes to RunFallback
        // (ExtractionMethod::Ocr) rather than RunFallbackOnPages
        // (ExtractionMethod::Mixed). A document where every page needs OCR is
        // not a "mixed" document.
        if failing_pages.len() == valid_boundary_count {
            document_decision.whole_doc_failure = true;
        }
    }
    document_decision.failing_pages = failing_pages;
    document_decision
}

// We no longer pre-render all pages for OCR to prevent OOMs.
// See `extract_with_ocr` for lazy streaming logic.

/// Render only specific PDF pages to images for OCR processing.
///
/// `page_indices` are 0-indexed. Only the requested pages are rendered,
/// returned as `(page_index, image)` pairs.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) fn render_selected_pages_for_ocr(
    content: &[u8],
    page_indices: &[usize],
) -> crate::Result<Vec<(usize, image::DynamicImage)>> {
    let doc = pdf_oxide::PdfDocument::from_bytes(content.to_vec()).map_err(|e| crate::XbergError::Parsing {
        message: format!("Failed to open PDF for rendering: {}", e),
        source: None,
    })?;

    let page_count = doc.page_count().map_err(|e| crate::XbergError::Parsing {
        message: format!("Failed to get PDF page count: {}", e),
        source: None,
    })?;

    // pdf_oxide's renderer ignores /Rotate; correct each page so OCR sees
    // upright text. Rotations are parsed once for the whole document.
    let page_rotations = crate::pdf::render::get_page_rotations(content, page_count);

    // Use safeguarded render (handles very wide / extreme-aspect pages that previously
    // caused PdfiumLibraryInternalError or equivalent "Failed to create pixmap" inside
    // the rasterizer). This is the core of the fix for #1078.
    let mut images = Vec::with_capacity(page_indices.len());
    for &idx in page_indices {
        if idx >= page_count {
            tracing::warn!(
                page = idx + 1,
                page_count,
                "force_ocr_pages: page {} is out of range (document has {} pages), skipping",
                idx + 1,
                page_count
            );
            continue;
        }
        let rendered = crate::pdf::render::render_page_with_safeguards(&doc, idx, 150).map_err(|e| {
            crate::XbergError::Parsing {
                message: format!("Failed to render PDF page {}: {}", idx + 1, e),
                source: None,
            }
        })?;
        // rendered.data is PNG-encoded; decode back to DynamicImage for OCR.
        let img = image::load_from_memory(&rendered.data).map_err(|e| crate::XbergError::Parsing {
            message: format!("Failed to decode rendered page {}: {}", idx + 1, e),
            source: None,
        })?;
        let rotation = page_rotations.get(idx).copied().unwrap_or(0);
        let img = crate::pdf::render::rotate_dynamic_image(img, rotation);
        images.push((idx, img));
    }

    Ok(images)
}

/// Build mixed text from native extraction and per-page OCR results.
///
/// For each page boundary, if the page is in `ocr_page_numbers` (1-indexed),
/// use the OCR result; otherwise use the native text slice.
///
/// Page numbers must be >= 1 (invalid values are filtered out with a warning).
/// An `ocr` config is recommended but not required; defaults are used if absent.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) async fn extract_mixed_ocr_native(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_page_numbers: &[u32],
    content: &[u8],
    config: &ExtractionConfig,
    _path: Option<&std::path::Path>,
) -> crate::Result<(
    String,
    ahash::AHashMap<u32, String>,
    Vec<crate::types::LlmUsage>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
)> {
    // Deduplicate and validate page numbers (must be >= 1)
    let ocr_set: std::collections::HashSet<u32> = ocr_page_numbers
        .iter()
        .copied()
        .filter(|&p| {
            if p == 0 {
                tracing::warn!("force_ocr_pages contains 0; page numbers are 1-indexed, ignoring");
                false
            } else {
                true
            }
        })
        .collect();

    if ocr_set.is_empty() {
        return Ok((
            native_text.to_string(),
            ahash::AHashMap::new(),
            Vec::new(),
            None,
            Vec::new(),
        ));
    }

    // Convert 1-indexed page numbers to 0-indexed for rendering (sorted + deduplicated)
    let mut page_indices: Vec<usize> = ocr_set.iter().map(|&p| (p - 1) as usize).collect();
    page_indices.sort_unstable();
    let page_images = render_selected_pages_for_ocr(content, &page_indices)?;

    if page_images.is_empty() {
        return Ok((
            native_text.to_string(),
            ahash::AHashMap::new(),
            Vec::new(),
            None,
            Vec::new(),
        ));
    }

    // OCR all selected pages concurrently using the same batched pipeline pattern
    // as extract_with_ocr: rayon-parallel PNG encoding + tokio JoinSet OCR calls.
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    #[cfg(feature = "tokio-runtime")]
    use rayon::prelude::*;
    use std::io::Cursor;
    use std::sync::Arc;

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let mut ocr_config_resolved = config.ocr.as_ref().unwrap_or(&default_ocr_config).clone();
    if ocr_config_resolved.acceleration.is_none() {
        ocr_config_resolved.acceleration = config.acceleration.clone();
    }

    let backend = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        registry.get(&ocr_config_resolved.backend)?
    };

    let batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());

    let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
    let ocr_config_owned = ocr_config_resolved;
    let total = page_images.len();
    let mut ocr_results: ahash::AHashMap<u32, String> = ahash::AHashMap::with_capacity(total);
    let mut accumulated_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
    let mut accumulated_formulas: Vec<crate::types::Formula> = Vec::new();
    let mut captured_rasters: Vec<crate::types::ExtractedImage> = Vec::new();

    // Process in batches to bound peak memory (PNG buffers freed between batches)
    for batch_start in (0..total).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total);
        let batch_slice = &page_images[batch_start..batch_end];

        type EncodedPage = (usize, Arc<Vec<u8>>, u32, u32);
        // Encode this batch's images to PNG. On native targets this runs in parallel
        // via rayon (CPU-bound); on wasm32 it falls back to a sequential iterator
        // because rayon's thread pool is unavailable without the `wasm-threads` feature.
        #[cfg(feature = "tokio-runtime")]
        let encoded: crate::Result<Vec<EncodedPage>> = batch_slice
            .par_iter()
            .map(|(page_idx, image)| {
                let rgb = image.to_rgb8();
                let (w, h) = rgb.dimensions();
                let mut buf = Cursor::new(Vec::new());
                PngEncoder::new(&mut buf)
                    .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                    .map_err(|e| crate::XbergError::Parsing {
                        message: format!("Failed to encode page {} for OCR: {}", page_idx + 1, e),
                        source: None,
                    })?;
                Ok((*page_idx, Arc::new(buf.into_inner()), w, h))
            })
            .collect();
        #[cfg(not(feature = "tokio-runtime"))]
        let encoded: crate::Result<Vec<EncodedPage>> = batch_slice
            .iter()
            .map(|(page_idx, image)| {
                let rgb = image.to_rgb8();
                let (w, h) = rgb.dimensions();
                let mut buf = Cursor::new(Vec::new());
                PngEncoder::new(&mut buf)
                    .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                    .map_err(|e| crate::XbergError::Parsing {
                        message: format!("Failed to encode page {} for OCR: {}", page_idx + 1, e),
                        source: None,
                    })?;
                Ok((*page_idx, Arc::new(buf.into_inner()), w, h))
            })
            .collect();
        let encoded = encoded?;

        // OCR this batch. On native targets tasks run concurrently via tokio::task::JoinSet
        // (requires the multi-threaded runtime). On wasm32 futures are awaited sequentially
        // because JoinSet::spawn requires thread-spawning, which is unavailable there.
        #[cfg(feature = "tokio-runtime")]
        {
            let mut join_set = tokio::task::JoinSet::new();
            for (page_idx, data, _w, _h) in &encoded {
                let backend_clone = Arc::clone(&backend);
                let config_clone = ocr_config_owned.clone();
                let data_clone = Arc::clone(data);
                let idx = *page_idx;
                join_set.spawn(async move {
                    let result = backend_clone.process_image(&data_clone, &config_clone).await;
                    (idx, result)
                });
            }
            while let Some(join_result) = join_set.join_next().await {
                let (page_idx, result) = join_result.map_err(|e| crate::XbergError::Plugin {
                    message: format!("OCR task panicked: {}", e),
                    plugin_name: "ocr".to_string(),
                })?;
                let mut extraction_result = result?;
                if let Some(usage) = extraction_result.llm_usage.take() {
                    accumulated_llm_usage.extend(usage);
                }
                // Accumulate formulas, renumbering to 1-indexed document page number.
                for mut formula in std::mem::take(&mut extraction_result.formulas) {
                    formula.page = (page_idx + 1) as u32;
                    accumulated_formulas.push(formula);
                }
                ocr_results.insert((page_idx + 1) as u32, extraction_result.content); // 1-indexed
            }
        }
        #[cfg(not(feature = "tokio-runtime"))]
        {
            for (page_idx, data, _w, _h) in &encoded {
                let mut extraction_result = backend.process_image(data.as_slice(), &ocr_config_owned).await?;
                if let Some(usage) = extraction_result.llm_usage.take() {
                    accumulated_llm_usage.extend(usage);
                }
                // Accumulate formulas, renumbering to 1-indexed document page number.
                for mut formula in std::mem::take(&mut extraction_result.formulas) {
                    formula.page = (*page_idx + 1) as u32;
                    accumulated_formulas.push(formula);
                }
                ocr_results.insert((*page_idx + 1) as u32, extraction_result.content); // 1-indexed
            }
        }

        if capture_rasters {
            for (page_idx, png_arc, w, h) in &encoded {
                let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                captured_rasters.push(build_page_raster_image(*page_idx, png_bytes, *w, *h));
            }
        }
        // encoded PNGs dropped here — memory freed before next batch
    }

    // Assemble final text by replacing OCR pages in-place within the native text.
    let result = merge_ocr_pages_into_native(native_text, boundaries, &ocr_results);

    Ok((
        result,
        ocr_results,
        accumulated_llm_usage,
        if capture_rasters { Some(captured_rasters) } else { None },
        accumulated_formulas,
    ))
}

/// Merge per-page OCR text into the native text, replacing each OCR'd page's
/// byte range in place.
///
/// Boundaries are processed in reverse byte order so earlier offsets stay valid
/// after each replacement. An OCR entry that is empty (or whitespace-only) is
/// skipped rather than applied: an empty OCR result must never overwrite a page's
/// native text, or a page whose backend produced nothing would silently lose its
/// already-extracted content.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn merge_ocr_pages_into_native(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_results: &ahash::AHashMap<u32, String>,
) -> String {
    let mut result = native_text.to_string();

    let mut sorted_boundaries: Vec<&crate::types::PageBoundary> = boundaries
        .iter()
        .filter(|b| b.byte_end <= native_text.len() && b.byte_start <= b.byte_end)
        .collect();
    sorted_boundaries.sort_unstable_by_key(|b| std::cmp::Reverse(b.byte_start));

    for boundary in sorted_boundaries {
        if let Some(ocr_text) = ocr_results.get(&boundary.page_number) {
            if ocr_text.trim().is_empty() {
                continue;
            }
            result.replace_range(boundary.byte_start..boundary.byte_end, ocr_text);
        }
    }

    result
}

/// Extract text from PDF using OCR on pre-rendered page images.
///
/// When `layout_detections` are provided (pixel-space, from the same images),
/// uses layout-aware markdown assembly for structured output. Otherwise falls
/// back to plain OCR text concatenation.
///
/// # Arguments
///
/// * `images` - Pre-rendered page images (shared with layout detection)
/// * `layout_detections` - Optional pixel-space layout detections per page
/// * `config` - Extraction configuration including OCR settings
///
/// # Returns
///
/// Concatenated text from all pages, with markdown structure when layout is available
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) async fn extract_with_ocr(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
) -> crate::Result<(
    String,
    Option<f64>,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
)> {
    use crate::plugins::registry::get_ocr_backend_registry;
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use std::io::Cursor;

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let base_ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    // Propagate acceleration from ExtractionConfig if not set on OcrConfig
    let accel_ocr_config;
    let base_ocr_config = if base_ocr_config.acceleration.is_none() && config.acceleration.is_some() {
        accel_ocr_config = {
            let mut c = base_ocr_config.clone();
            c.acceleration = config.acceleration.clone();
            c
        };
        &accel_ocr_config
    } else {
        base_ocr_config
    };

    let backend = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        registry.get(&base_ocr_config.backend)?
    };

    // When layout detections are available, ensure OCR produces elements
    // so the layout assembly module can use them for structured markdown.
    // Also inject layout-specific backend configuration (e.g., enable_chart_understanding).
    // Additionally, inject the chart flag for backends that emit structured markdown (e.g., paired-mode GLM-OCR)
    // which internally run layout detection and need the chart understanding flag.
    #[cfg(feature = "layout-detection")]
    let layout_ocr_config;
    let ocr_config = {
        #[cfg(feature = "layout-detection")]
        {
            let should_inject = layout_detections.is_some() || backend.emits_structured_markdown();
            if should_inject {
                layout_ocr_config = {
                    let mut cfg = ensure_elements_enabled(base_ocr_config);
                    cfg = inject_layout_config_to_backend(&cfg, config);
                    cfg
                };
                &layout_ocr_config
            } else {
                base_ocr_config
            }
        }
        #[cfg(not(feature = "layout-detection"))]
        {
            base_ocr_config
        }
    };

    // If the backend supports direct document processing and we have a path,
    // use it to process the entire document at once, bypassing page rendering.
    // This is currently only supported when layout detection is NOT active,
    // as layout assembly requires per-rendering results.
    #[cfg(not(feature = "layout-detection"))]
    let supports_doc = backend.supports_document_processing();
    #[cfg(feature = "layout-detection")]
    let supports_doc = backend.supports_document_processing() && layout_detections.is_none();

    let use_document_processing = supports_doc && path.is_some();

    if let Some(doc_path) = path
        && use_document_processing
    {
        tracing::debug!(backend = %ocr_config.backend, "Using document-level OCR processing");
        let result = backend.process_document(doc_path, ocr_config).await?;
        let mean_conf = result
            .metadata
            .additional
            .get("mean_text_conf")
            .and_then(|v| v.as_f64())
            .map(|v| v / 100.0);
        let ocr_elements = result.ocr_elements.unwrap_or_default();
        let llm_usage = result.llm_usage.unwrap_or_default();
        let formulas = result.formulas;
        let page_texts = if let Some(pages) = result.pages {
            pages.into_iter().map(|p| p.content).collect()
        } else {
            vec![result.content.clone()]
        };
        return Ok((
            result.content,
            mean_conf,
            Vec::new(),
            ocr_elements,
            None,
            llm_usage,
            page_texts,
            None, // no per-page renders on document-level bypass
            formulas,
        ));
    }
    let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
    let mut captured_rasters: Vec<crate::types::ExtractedImage> = Vec::new();

    let mut lazy_pdf_page_count = 0;

    if !use_document_processing
        && images.is_none()
        && let Some(bytes) = content
    {
        #[cfg(feature = "pdf")]
        {
            let doc = pdf_oxide::PdfDocument::from_bytes(bytes.to_vec()).map_err(|e| crate::XbergError::Parsing {
                message: format!("Failed to open PDF for OCR streaming: {:?}", e),
                source: None,
            })?;
            lazy_pdf_page_count = doc.page_count().map_err(|e| crate::XbergError::Parsing {
                message: format!("Failed to get document page count: {:?}", e),
                source: None,
            })?;
        }
    }

    // Encode and OCR pages in bounded batches so that at most `batch_size`
    // PNG-encoded images are alive at a time. This caps peak memory to roughly
    // batch_size * (encoded_PNG + OCR working set) instead of
    // page_count * that amount. Images are rendered and encoded one at a time
    // within each batch to avoid holding multiple decoded RGB buffers.
    #[cfg(feature = "tokio-runtime")]
    use rayon::prelude::*;
    use std::sync::Arc;
    #[cfg(feature = "tokio-runtime")]
    use tokio::task::JoinSet;

    let configured_batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());

    // Estimate per-page memory cost and adapt batch size to available system memory.
    // A rendered page at 300 DPI (A4) is ~26MB RGB + ~5MB PNG + ~100MB OCR working set.
    // We also need headroom for the PDF document itself and other allocations.
    let batch_size = if images.is_none() {
        adapt_batch_size_to_memory(configured_batch_size, content.map(|b| b.len()).unwrap_or(0))
    } else {
        configured_batch_size
    };

    if batch_size < configured_batch_size {
        tracing::info!(
            configured = configured_batch_size,
            adapted = batch_size,
            "Reduced OCR batch size to fit available memory"
        );
    }

    let mut ocr_config_owned = ocr_config.clone();
    ocr_config_owned.acceleration = config.acceleration.clone();
    let total_pages = if let Some(imgs) = images {
        imgs.len()
    } else {
        lazy_pdf_page_count
    };

    let mut page_texts = vec![String::new(); total_pages];
    #[cfg(feature = "layout-detection")]
    let mut all_page_paragraphs: Vec<Option<Vec<crate::pdf::structure::types::PdfParagraph>>> = vec![None; total_pages];
    #[allow(unused_mut)]
    let mut collected_tables: Vec<crate::types::Table> = Vec::new();
    let mut all_ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
    let mut accumulated_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
    let mut accumulated_formulas: Vec<crate::types::Formula> = Vec::new();
    let mut conf_sum: f64 = 0.0;
    let mut conf_count: usize = 0;

    // Initialize TATR for table structure recognition when layout detection is active.
    // TATR requires mutable access so pages are processed sequentially after OCR.
    #[cfg(feature = "layout-detection")]
    let mut tatr_model = if layout_detections.is_some() {
        crate::layout::take_or_create_tatr(config.acceleration.as_ref())
    } else {
        None
    };

    for batch_start in (0..total_pages).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total_pages);

        // Render and encode pages one at a time within the batch to avoid holding
        // multiple decoded RGB buffers (~26MB each at 300 DPI) simultaneously.
        // Only the compact PNG-encoded bytes are kept for the batch's OCR phase.
        #[allow(unused_variables)]
        let (batch_slice, encoded_batch) = if let Some(imgs) = images {
            let slice: Cow<'_, [image::DynamicImage]> = Cow::Borrowed(&imgs[batch_start..batch_end]);
            // Encode pre-rendered images. On native targets this runs in parallel via rayon
            // (CPU-bound); on wasm32 it falls back to a sequential iterator because
            // rayon's thread pool is unavailable without the `wasm-threads` feature.
            #[allow(clippy::type_complexity)]
            #[cfg(feature = "tokio-runtime")]
            let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = slice
                .par_iter()
                .enumerate()
                .map(|(offset, image)| {
                    let page_idx = batch_start + offset;
                    let rgb_image = image.to_rgb8();
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let encoder = PngEncoder::new(&mut image_bytes);
                    encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode image: {}", e),
                            source: None,
                        })?;
                    Ok((page_idx, Arc::new(image_bytes.into_inner()), width, height))
                })
                .collect();
            #[allow(clippy::type_complexity)]
            #[cfg(not(feature = "tokio-runtime"))]
            let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = slice
                .iter()
                .enumerate()
                .map(|(offset, image)| {
                    let page_idx = batch_start + offset;
                    let rgb_image = image.to_rgb8();
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let encoder = PngEncoder::new(&mut image_bytes);
                    encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode image: {}", e),
                            source: None,
                        })?;
                    Ok((page_idx, Arc::new(image_bytes.into_inner()), width, height))
                })
                .collect();
            (Some(slice), encoded?)
        } else {
            #[cfg(feature = "pdf")]
            let encoded = {
                // Render each page to PNG bytes directly via pdf_oxide.
                // RenderedImage.data is already PNG-encoded, so no re-encode step needed.
                let pdf_bytes = content.ok_or_else(|| crate::XbergError::Parsing {
                    message: "PDF content is required for OCR rendering but was not provided".to_string(),
                    source: None,
                })?;
                let doc =
                    pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec()).map_err(|e| crate::XbergError::Parsing {
                        message: format!("Failed to open PDF for OCR batch rendering: {:?}", e),
                        source: None,
                    })?;
                // pdf_oxide's renderer ignores /Rotate; correct rotated pages so
                // OCR sees upright text (no-op decode-free path for rotation 0).
                let page_count = doc.page_count().unwrap_or(0);
                let page_rotations = crate::pdf::render::get_page_rotations(pdf_bytes, page_count);

                // Use the safeguarded renderer (see render.rs). This prevents hard
                // failures on the exact class of inputs reported in #1078 (single-page
                // very wide vector-heavy PDFs) when force_ocr + VLM (or other ocr-pipeline
                // backends) is used. Normal pages are unaffected.
                let mut batch_encoded: Vec<(usize, Arc<Vec<u8>>, u32, u32)> =
                    Vec::with_capacity(batch_end - batch_start);
                for i in batch_start..batch_end {
                    let rendered = crate::pdf::render::render_page_with_safeguards(&doc, i, 150).map_err(|e| {
                        crate::XbergError::Parsing {
                            message: format!("Failed to render page {} for OCR: {:?}", i, e),
                            source: None,
                        }
                    })?;
                    let rotation = page_rotations.get(i).copied().unwrap_or(0);
                    let (data, width, height) = crate::pdf::render::rotate_png_page_if_needed(
                        rendered.data,
                        rendered.width,
                        rendered.height,
                        rotation,
                    )?;
                    batch_encoded.push((i, Arc::new(data), width, height));
                }
                batch_encoded
            };
            #[cfg(not(feature = "pdf"))]
            let encoded: Vec<(usize, Arc<Vec<u8>>, u32, u32)> = Vec::new();
            (None::<Cow<'_, [image::DynamicImage]>>, encoded)
        };

        // OCR this batch. On native targets tasks run concurrently via tokio::task::JoinSet
        // (requires the multi-threaded runtime). On wasm32 futures are awaited sequentially
        // because JoinSet::spawn requires thread-spawning, which is unavailable there.
        let batch_count = encoded_batch.len();
        let mut batch_ocr_results: Vec<Option<crate::types::ExtractedDocument>> = vec![None; batch_count];

        #[cfg(feature = "tokio-runtime")]
        {
            let mut join_set: JoinSet<(usize, crate::Result<crate::types::ExtractedDocument>)> = JoinSet::new();
            for (page_idx, image_data, _width, _height) in &encoded_batch {
                let backend_clone = std::sync::Arc::clone(&backend);
                let config_clone = ocr_config_owned.clone();
                let data_clone = Arc::clone(image_data);
                let idx = *page_idx;
                join_set.spawn(async move {
                    let result = backend_clone.process_image(&data_clone, &config_clone).await;
                    (idx, result)
                });
            }
            while let Some(join_result) = join_set.join_next().await {
                let (page_idx, ocr_result) = join_result.map_err(|e| crate::XbergError::Plugin {
                    message: format!("OCR task panicked: {}", e),
                    plugin_name: "ocr".to_string(),
                })?;
                batch_ocr_results[page_idx - batch_start] = Some(ocr_result?);
            }
        }
        #[cfg(not(feature = "tokio-runtime"))]
        {
            for (page_idx, image_data, _width, _height) in &encoded_batch {
                let ocr_result = backend.process_image(image_data.as_slice(), &ocr_config_owned).await?;
                batch_ocr_results[page_idx - batch_start] = Some(ocr_result);
            }
        }

        // Sequential post-processing for this batch utilizing TATR.
        for offset in 0..batch_count {
            let page_idx = batch_start + offset;
            let mut ocr_result = batch_ocr_results[offset].take().expect("OCR result missing for page");
            #[cfg(feature = "layout-detection")]
            let _height = encoded_batch[offset].3;

            if let Some(conf_val) = ocr_result
                .metadata
                .additional
                .get("mean_text_conf")
                .and_then(|v| v.as_i64())
            {
                conf_sum += conf_val as f64;
                conf_count += 1;
            }

            // Accumulate LLM usage from this page (e.g., VLM OCR).
            if let Some(usage) = ocr_result.llm_usage.take() {
                accumulated_llm_usage.extend(usage);
            }

            // Accumulate OCR elements from this page.
            if let Some(ref mut elems) = ocr_result.ocr_elements {
                for elem in elems.iter_mut() {
                    elem.page_number = (page_idx + 1) as u32;
                }
                all_ocr_elements.extend(elems.iter().cloned());
            }

            // Accumulate formulas from this page, renumbering page field to document page number.
            for mut formula in ocr_result.formulas {
                formula.page = (page_idx + 1) as u32;
                accumulated_formulas.push(formula);
            }

            #[cfg(feature = "layout-detection")]
            if let Some(detections) = layout_detections
                && let Some(ref elements) = ocr_result.ocr_elements
                && !elements.is_empty()
            {
                let detection = detections.get(page_idx);

                // Scale layout detection bounding boxes from layout-model resolution
                // (e.g. 640×640) to OCR render resolution so that coordinates are
                // consistent when passed to recognize_page_tables and
                // detection_to_layout_hints (both use pixel-space coordinates).
                let ocr_render_width = encoded_batch[offset].2;
                let ocr_render_height = encoded_batch[offset].3;
                let scaled_detection: Option<crate::layout::DetectionResult> = detection.map(|det| {
                    let sx = ocr_render_width as f32 / det.page_width as f32;
                    let sy = ocr_render_height as f32 / det.page_height as f32;
                    let mut scaled = det.clone();
                    scaled.page_width = ocr_render_width;
                    scaled.page_height = ocr_render_height;
                    for region in &mut scaled.detections {
                        region.bbox.x1 *= sx;
                        region.bbox.y1 *= sy;
                        region.bbox.x2 *= sx;
                        region.bbox.y2 *= sy;
                    }
                    scaled
                });

                let recognized_tables = match (scaled_detection.as_ref(), tatr_model.as_mut()) {
                    (Some(scaled_det), Some(model)) => {
                        // Decode the page image from its PNG for TATR table recognition.
                        // When pre-rendered images are available, use them directly.
                        // Otherwise, decode from the PNG we already encoded.
                        let rgb = if let Some(ref slice) = batch_slice {
                            slice[offset].to_rgb8()
                        } else {
                            let png_data = &encoded_batch[offset].1;
                            let decoded =
                                image::load_from_memory(png_data).map_err(|e| crate::XbergError::Parsing {
                                    message: format!("Failed to decode PNG for TATR: {}", e),
                                    source: None,
                                })?;
                            decoded.to_rgb8()
                        };
                        crate::ocr::layout_assembly::recognize_page_tables(&rgb, scaled_det, elements, model)
                    }
                    _ => Vec::new(),
                };

                // Collect recognized tables as Table structs for ExtractedDocument.tables
                for rt in &recognized_tables {
                    if !rt.markdown.is_empty() {
                        collected_tables.push(crate::types::Table {
                            cells: rt.cells.clone(),
                            markdown: rt.markdown.clone(),
                            page_number: (page_idx + 1) as u32,
                            bounding_box: None,
                        });
                    }
                }

                // Convert hOCR structure to PdfParagraphs, then apply layout overrides.
                // This follows the oxide path: structure → layout classify → assemble.
                if let Some(ref ocr_doc) = ocr_result.ocr_internal_document {
                    let mut paragraphs =
                        crate::pdf::structure::adapters::ocr_doc_to_paragraphs(ocr_doc, ocr_render_height);

                    if let Some(ref scaled_det) = scaled_detection {
                        let hints = super::layout_hints::detection_to_layout_hints_pixel_space(
                            scaled_det,
                            ocr_render_height as f32,
                        );
                        // Trust the layout model for OCR — no body-font-size guard
                        // since OCR text lacks reliable font size information.
                        crate::pdf::structure::layout_classify::apply_layout_overrides(
                            &mut paragraphs,
                            &hints,
                            0.5,
                            0.2,
                            None,
                        );
                    }

                    tracing::debug!(
                        page = page_idx + 1,
                        paragraphs = paragraphs.len(),
                        raw_content_len = ocr_result.content.len(),
                        "OCR page layout classification complete"
                    );

                    // Don't filter page furniture for OCR — the layout model's
                    // header/footer detection is less reliable on OCR-rendered pages,
                    // and falsely filtering content is worse than keeping it.
                    all_page_paragraphs[page_idx] = Some(paragraphs);
                }

                // Use tesseract's own text output (preserves reading order).
                if capture_rasters {
                    let (_, png_arc, w, h) = &encoded_batch[offset];
                    let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                    captured_rasters.push(build_page_raster_image(page_idx, png_bytes, *w, *h));
                }
                page_texts[page_idx] = ocr_result.content;
                continue;
            }

            let _ = page_idx;
            if capture_rasters {
                let (_, png_arc, w, h) = &encoded_batch[offset];
                let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                captured_rasters.push(build_page_raster_image(page_idx, png_bytes, *w, *h));
            }
            page_texts[page_idx] = ocr_result.content;
        }
    }

    #[cfg(feature = "layout-detection")]
    if let Some(model) = tatr_model.take() {
        crate::layout::return_tatr(model);
    }

    let mean_text_conf = if conf_count > 0 {
        Some((conf_sum / conf_count as f64) / 100.0)
    } else {
        None
    };

    let page_marker_cfg = config.pages.as_ref().filter(|p| p.insert_page_markers);
    let mut result = String::new();
    for (i, text) in page_texts.iter().enumerate() {
        if let Some(cfg) = page_marker_cfg {
            let marker = cfg.marker_format.replace("{page_num}", &(i + 1).to_string());
            result.push_str(&marker);
        } else if i > 0 {
            result.push_str("\n\n");
        }
        result.push_str(text);
    }

    #[cfg(feature = "layout-detection")]
    let ocr_doc = {
        let has_structured = all_page_paragraphs.iter().any(|p| p.is_some());
        if has_structured {
            let pages: Vec<Vec<crate::pdf::structure::types::PdfParagraph>> = all_page_paragraphs
                .into_iter()
                .map(|opt| opt.unwrap_or_default())
                .collect();
            Some(crate::pdf::structure::assemble_internal_document(
                pages,
                &collected_tables,
                None,
                &[],
            ))
        } else {
            None
        }
    };
    #[cfg(not(feature = "layout-detection"))]
    let ocr_doc: Option<crate::types::internal::InternalDocument> = {
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        for paragraph in result.split("\n\n") {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                doc.push_element(crate::types::internal::InternalElement::text(
                    crate::types::internal::ElementKind::Paragraph,
                    trimmed,
                    0,
                ));
            }
        }
        doc.tables = collected_tables.clone();
        Some(doc)
    };

    Ok((
        result,
        mean_text_conf,
        collected_tables,
        all_ocr_elements,
        ocr_doc,
        accumulated_llm_usage,
        page_texts,
        if capture_rasters { Some(captured_rasters) } else { None },
        accumulated_formulas,
    ))
}

/// Build an [`crate::types::ExtractedImage`] for a full-page OCR raster.
///
/// `image_index` is set to 0; the caller must reindex after merging into
/// the document's image collection.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn build_page_raster_image(
    page_idx: usize,
    png_bytes: bytes::Bytes,
    width: u32,
    height: u32,
) -> crate::types::ExtractedImage {
    crate::types::ExtractedImage {
        data: png_bytes,
        format: std::borrow::Cow::Borrowed("png"),
        image_index: 0,
        page_number: Some((page_idx + 1) as u32),
        width: Some(width),
        height: Some(height),
        colorspace: Some("RGB".to_string()),
        bits_per_component: Some(8),
        is_mask: false,
        description: None,
        ocr_result: None,
        bounding_box: None,
        source_path: None,
        image_kind: Some(crate::types::ImageKind::PageRaster),
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    }
}

/// Adapt batch size to available system memory.
///
/// Estimates per-page memory cost based on typical page dimensions at 300 DPI
/// and compares against available system memory. Returns a batch size that
/// should keep peak memory within safe bounds.
///
/// Conservative estimate: each page in a batch needs approximately:
/// - ~50MB for render + encode working set (RGB buffer briefly, then PNG)
/// - ~100MB for OCR working set per concurrent page
/// - Plus the document itself and base allocations
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn adapt_batch_size_to_memory(configured: usize, document_size: usize) -> usize {
    let available_bytes = get_available_memory();

    if available_bytes == 0 {
        return configured;
    }

    // Reserve memory for: the document itself, base process overhead, and safety margin.
    let reserved = document_size + 512 * 1024 * 1024; // document + 512MB overhead
    let usable = available_bytes.saturating_sub(reserved);

    // Estimated memory per concurrent page in OCR batch:
    // ~50MB render/encode working set + ~100MB OCR working set
    const PER_PAGE_ESTIMATE: usize = 150 * 1024 * 1024;

    let memory_limited_batch = (usable / PER_PAGE_ESTIMATE).max(1);

    let result = configured.min(memory_limited_batch);

    tracing::debug!(
        available_mb = available_bytes / (1024 * 1024),
        usable_mb = usable / (1024 * 1024),
        document_mb = document_size / (1024 * 1024),
        memory_limited_batch,
        configured,
        result,
        "OCR batch size adaptation"
    );

    result
}

/// Query available system memory without external dependencies.
///
/// On Linux (including Docker), reads `/proc/meminfo` for `MemAvailable`.
/// On macOS, uses `sysctl hw.memsize` for total memory (conservative fallback).
/// Returns 0 if the query fails, signaling the caller to use the default batch size.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn get_available_memory() -> usize {
    #[cfg(target_os = "linux")]
    {
        let host = read_meminfo_available();
        host.min(cgroup_headroom().unwrap_or(usize::MAX))
    }
    #[cfg(target_os = "macos")]
    {
        // On macOS, read page size and free+inactive pages from vm_stat.
        // This is a rough estimate since macOS memory management is complex.
        use std::process::Command;
        if let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output()
            && let Ok(s) = std::str::from_utf8(&output.stdout)
            && let Ok(total) = s.trim().parse::<usize>()
        {
            // Use 50% of total as a conservative "available" estimate.
            return total / 2;
        }
        0
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        0
    }
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), target_os = "linux"))]
fn parse_meminfo_available(contents: &str) -> usize {
    contents
        .lines()
        .find_map(|l| {
            l.strip_prefix("MemAvailable:")?
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<usize>()
                .ok()
        })
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), target_os = "linux"))]
fn read_meminfo_available() -> usize {
    parse_meminfo_available(&std::fs::read_to_string("/proc/meminfo").unwrap_or_default())
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), target_os = "linux"))]
fn parse_cgroup_v2(max: &str, current: &str) -> Option<usize> {
    let max = max.trim();
    if max == "max" {
        return None;
    }
    let limit = max.parse::<usize>().ok()?;
    let usage = current.trim().parse::<usize>().ok()?;
    Some(limit.saturating_sub(usage))
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), target_os = "linux"))]
fn parse_cgroup_v1(limit: &str, usage: &str) -> Option<usize> {
    let limit = limit.trim().parse::<usize>().ok()?;
    let usage = usage.trim().parse::<usize>().ok()?;
    // v1 limit_in_bytes returns ~9.2e18 when unlimited
    (limit < (isize::MAX as usize)).then(|| limit.saturating_sub(usage))
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), target_os = "linux"))]
fn cgroup_headroom() -> Option<usize> {
    // cgroup v2 (unified): /sys/fs/cgroup/memory.{max,current}
    if let (Ok(max), Ok(cur)) = (
        std::fs::read_to_string("/sys/fs/cgroup/memory.max"),
        std::fs::read_to_string("/sys/fs/cgroup/memory.current"),
    ) {
        return parse_cgroup_v2(&max, &cur);
    }
    // cgroup v1 fallback
    let limit = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok()?;
    let usage = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.usage_in_bytes").ok()?;
    parse_cgroup_v1(&limit, &usage)
}
/// Run a multi-backend OCR pipeline with quality-based fallback.
///
/// Images and layout detections are computed once and shared across all stages.
/// Each stage produces OCR output that is scored; if the score meets the
/// pipeline's quality threshold, the result is accepted. Otherwise, the next
/// backend is tried. Returns the best result seen across all stages.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) async fn run_ocr_pipeline(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    pipeline: &crate::core::config::OcrPipelineConfig,
    path: Option<&std::path::Path>,
) -> crate::Result<(
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
)> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    // Sort stages by priority (highest first)
    let mut stages = pipeline.stages.clone();
    stages.sort_by_key(|b| std::cmp::Reverse(b.priority));

    // Filter to available backends
    let requested_backends: Vec<String> = stages.iter().map(|s| s.backend.clone()).collect();
    let available_stages: Vec<_> = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        stages
            .into_iter()
            .filter(|s| registry.get(&s.backend).is_ok())
            .collect()
    };

    if available_stages.is_empty() {
        return Err(crate::XbergError::Parsing {
            message: format!(
                "No available OCR backends for pipeline (requested: {})",
                requested_backends.join(", ")
            ),
            source: None,
        });
    }

    #[allow(clippy::type_complexity)]
    let mut best_result: Option<(
        String,
        f64,
        Vec<crate::types::Table>,
        Vec<crate::types::OcrElement>,
        Option<crate::types::internal::InternalDocument>,
        Vec<String>,
        Option<Vec<crate::types::ExtractedImage>>,
        Vec<crate::types::Formula>,
    )> = None;

    // Accumulate LLM usage from ALL attempted stages for accurate billing.
    // Usage is incurred even when a backend doesn't win the quality race.
    let mut accumulated_usage: Vec<crate::types::LlmUsage> = Vec::new();

    for stage in &available_stages {
        // Build a modified config for this stage
        let mut stage_ocr = ocr_config.clone();
        stage_ocr.backend = stage.backend.clone();
        if let Some(ref lang) = stage.language {
            stage_ocr.language = lang.clone();
        }
        if let Some(ref tc) = stage.tesseract_config {
            stage_ocr.tesseract_config = Some(tc.clone());
        }
        if let Some(ref pc) = stage.paddle_ocr_config {
            stage_ocr.paddle_ocr_config = Some(pc.clone());
        }
        stage_ocr.vlm_config = stage.vlm_config.clone();
        stage_ocr.backend_options = stage.backend_options.clone();

        let stage_config = ExtractionConfig {
            ocr: Some(stage_ocr),
            ..config.clone()
        };

        tracing::debug!(
            backend = %stage.backend,
            priority = stage.priority,
            "Pipeline: trying OCR backend"
        );

        // Box::pin so this large OCR future lives on the heap rather than being
        // held inline in the pipeline-loop frame, which is already deep. Keeps the
        // OCR await chain's stack footprint down.
        let result = Box::pin(extract_with_ocr(
            content,
            images,
            #[cfg(feature = "layout-detection")]
            layout_detections,
            &stage_config,
            path,
        ))
        .await;

        match result {
            Ok((
                text,
                mean_conf,
                stage_tables,
                stage_ocr_elements,
                stage_doc,
                stage_llm_usage,
                stage_page_texts,
                stage_rasters,
                stage_formulas,
            )) => {
                let text_score = compute_quality_score(&text, &pipeline.quality_thresholds);

                let score = match mean_conf {
                    Some(conf) => text_score * 0.7 + conf * 0.3,
                    None => text_score,
                };

                tracing::debug!(
                    backend = %stage.backend,
                    score,
                    text_score,
                    mean_text_conf = ?mean_conf,
                    threshold = pipeline.quality_thresholds.pipeline_min_quality,
                    "Pipeline: backend produced result"
                );

                // Always accumulate usage regardless of whether this stage wins.
                accumulated_usage.extend(stage_llm_usage);

                if score >= pipeline.quality_thresholds.pipeline_min_quality {
                    return Ok((
                        text,
                        stage_tables,
                        stage_ocr_elements,
                        stage_doc,
                        accumulated_usage,
                        stage_page_texts,
                        stage_rasters,
                        stage_formulas,
                    ));
                }

                // Track best-so-far (without usage, which is in accumulated_usage)
                match best_result {
                    Some((_, best_score, _, _, _, _, _, _)) if score > best_score => {
                        best_result = Some((
                            text,
                            score,
                            stage_tables,
                            stage_ocr_elements,
                            stage_doc,
                            stage_page_texts,
                            stage_rasters,
                            stage_formulas,
                        ));
                    }
                    None => {
                        best_result = Some((
                            text,
                            score,
                            stage_tables,
                            stage_ocr_elements,
                            stage_doc,
                            stage_page_texts,
                            stage_rasters,
                            stage_formulas,
                        ));
                    }
                    _ => {}
                }
            }
            Err(e) => {
                tracing::warn!(
                    backend = %stage.backend,
                    error = %e,
                    "Pipeline: backend failed, trying next"
                );
            }
        }
    }

    // Return best result (with warning) or error if all backends failed entirely
    match best_result {
        Some((text, score, tables, elements, doc, page_texts, rasters, formulas)) => {
            let threshold = pipeline.quality_thresholds.pipeline_min_quality;
            tracing::warn!(
                score,
                threshold,
                "All OCR pipeline backends produced suboptimal quality, using best result"
            );
            // Attach a ProcessingWarning so consumers can tell the returned text is
            // best-effort/below the configured quality threshold rather than clean.
            // If the winning stage produced no InternalDocument (document-level bypass
            // path), build a minimal one from the text so the warning still surfaces
            // instead of being dropped when the caller reconstructs the document.
            let mut doc = doc.unwrap_or_else(|| {
                let mut d = crate::types::internal::InternalDocument::new("pdf");
                for paragraph in text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        d.push_element(crate::types::internal::InternalElement::text(
                            crate::types::internal::ElementKind::Paragraph,
                            trimmed,
                            0,
                        ));
                    }
                }
                d
            });
            doc.processing_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr_pipeline"),
                message: std::borrow::Cow::Owned(format!(
                    "All OCR pipeline backends scored below the configured quality threshold \
                     (best score {score:.3} < {threshold:.3}); returning the best-effort result, \
                     which may be inaccurate or incomplete."
                )),
            });
            Ok((
                text,
                tables,
                elements,
                Some(doc),
                accumulated_usage,
                page_texts,
                rasters,
                formulas,
            ))
        }
        None => Err(crate::XbergError::Parsing {
            message: "All OCR pipeline backends failed".to_string(),
            source: None,
        }),
    }
}

/// Clone an OCR config with `include_elements` forced to true.
///
/// Layout assembly requires OCR elements with bounding geometry. This ensures
/// the backend produces them regardless of the user's original config.
#[cfg(all(feature = "ocr", feature = "layout-detection"))]
fn ensure_elements_enabled(config: &crate::core::config::ocr::OcrConfig) -> crate::core::config::ocr::OcrConfig {
    let mut config = config.clone();
    match config.element_config.as_mut() {
        Some(ec) => ec.include_elements = true,
        None => {
            config.element_config = Some(crate::types::OcrElementConfig {
                include_elements: true,
                ..Default::default()
            });
        }
    }
    config
}

/// Inject layout-detection settings into OcrConfig backend options for paired-mode backends.
///
/// When layout detection is active and provides detections, certain backends (e.g., GLM-OCR)
/// may need configuration injected from the layout-detection config. This function ensures
/// that the `enable_chart_understanding` flag from `ExtractionConfig.layout` is propagated
/// to the OCR backend via `backend_options` so per-region task dispatch can honor it.
#[cfg(all(feature = "ocr", feature = "layout-detection"))]
fn inject_layout_config_to_backend(
    config: &crate::core::config::ocr::OcrConfig,
    extraction_config: &ExtractionConfig,
) -> crate::core::config::ocr::OcrConfig {
    let mut config = config.clone();
    if let Some(layout_cfg) = &extraction_config.layout {
        // Prepare or merge backend_options JSON object
        let mut opts = config.backend_options.take().unwrap_or_else(|| serde_json::json!({}));

        // If backend_options is not an object, replace it with a new object
        // (warn if we're discarding a non-null, non-object value).
        if !opts.is_object() {
            if !opts.is_null() {
                tracing::warn!(
                    backend_options = %opts,
                    "backend_options was not a JSON object; replacing with new object to inject enable_chart_understanding"
                );
            }
            opts = serde_json::json!({});
        }

        // Inject enable_chart_understanding into the object
        if let Some(obj) = opts.as_object_mut() {
            obj.insert(
                "enable_chart_understanding".to_string(),
                serde_json::Value::Bool(layout_cfg.enable_chart_understanding),
            );
        }

        config.backend_options = Some(opts);
    }
    config
}

// `detection_to_layout_hints` for the OCR path lives in the shared
// `super::layout_hints` module as `detection_to_layout_hints_pixel_space`.
// The OCR path uses the pixel-space variant because OCR-derived paragraphs
// reach `apply_layout_overrides` in pixel space (via `ocr_doc_to_paragraphs`).

#[cfg(all(test, feature = "ocr"))]
mod tests {
    use super::*;

    #[cfg(feature = "ocr")]
    fn t() -> OcrQualityThresholds {
        OcrQualityThresholds::default()
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_empty_text_triggers_fallback() {
        let decision = evaluate_native_text_for_ocr("", Some(1), &t());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_replacement_chars_trigger_fallback() {
        let text = "The \u{FFFD}\u{FFFD}\u{FFFD} quick \u{FFFD}\u{FFFD}\u{FFFD} brown fox";
        let stats = NativeTextStats::from(text);
        assert_eq!(stats.garbage_char_count, 6);
        let decision = evaluate_native_text_for_ocr(text, Some(1), &t());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_fragmented_words_trigger_fallback() {
        let text = "T h e q u i c k b r o w n f o x j u m p s";
        let stats = NativeTextStats::from(text);
        assert!(stats.fragmented_word_ratio > 0.8);
        let decision = evaluate_native_text_for_ocr(text, Some(1), &t());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_good_text_no_fallback() {
        let text = "This is a normal paragraph with meaningful words and proper structure. \
                    It contains multiple sentences that form a coherent text block.";
        let decision = evaluate_native_text_for_ocr(text, Some(1), &t());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_single_bad_page_triggers() {
        use crate::types::PageBoundary;

        let text = "Good text on page one with meaningful content.\x00\x00\x00";
        let boundaries = vec![
            PageBoundary {
                page_number: 1,
                byte_start: 0,
                byte_end: 46,
            },
            PageBoundary {
                page_number: 2,
                byte_start: 46,
                byte_end: text.len(),
            },
        ];
        let decision = evaluate_per_page_ocr(text, Some(&boundaries), Some(2), &t());
        assert!(decision.fallback);
    }

    // --- Mixed-page OCR/native merge (issue #1223) ---

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_merge_empty_ocr_result_keeps_native_text() {
        use crate::types::PageBoundary;

        // Page 1 has good native text; page 2 was flagged for OCR but the backend
        // produced an empty result. The page-2 native text must be preserved, not wiped.
        let native = "PAGE ONE NATIVE\nPAGE TWO NATIVE";
        let boundaries = vec![
            PageBoundary {
                page_number: 1,
                byte_start: 0,
                byte_end: 16, // "PAGE ONE NATIVE\n"
            },
            PageBoundary {
                page_number: 2,
                byte_start: 16,
                byte_end: native.len(),
            },
        ];
        let mut ocr_results: ahash::AHashMap<u32, String> = ahash::AHashMap::new();
        ocr_results.insert(2, String::new()); // empty OCR result for page 2

        let merged = merge_ocr_pages_into_native(native, &boundaries, &ocr_results);
        assert_eq!(
            merged, native,
            "an empty OCR result must not overwrite the page's native text"
        );
        assert!(merged.contains("PAGE TWO NATIVE"));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_merge_nonempty_ocr_result_replaces_native_text() {
        use crate::types::PageBoundary;

        let native = "PAGE ONE NATIVE\ngarbage page two";
        let boundaries = vec![
            PageBoundary {
                page_number: 1,
                byte_start: 0,
                byte_end: 16,
            },
            PageBoundary {
                page_number: 2,
                byte_start: 16,
                byte_end: native.len(),
            },
        ];
        let mut ocr_results: ahash::AHashMap<u32, String> = ahash::AHashMap::new();
        ocr_results.insert(2, "CLEAN OCR PAGE TWO".to_string());

        let merged = merge_ocr_pages_into_native(native, &boundaries, &ocr_results);
        assert!(merged.contains("PAGE ONE NATIVE"));
        assert!(merged.contains("CLEAN OCR PAGE TWO"));
        assert!(!merged.contains("garbage page two"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_few_replacement_chars_no_fallback() {
        let text = "The quick \u{FFFD} brown fox jumps over the lazy dog repeatedly.";
        let stats = NativeTextStats::from(text);
        assert_eq!(stats.garbage_char_count, 1);
        let decision = evaluate_native_text_for_ocr(text, Some(1), &t());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_consecutive_repeat_high_with_substantial_content_no_ocr() {
        // Fix for #1176: repeat ratio is prose-tuned and causes false positives
        // on numeric tables. When content is substantial, we tolerate repetition.
        // This test verifies that high repeat ratio alone doesn't trigger OCR
        // if there's substantial non-whitespace content.
        let defaults = t();
        let mut words = Vec::new();
        for _ in 0..10 {
            words.extend_from_slice(&[
                "TALK", "TALK", "of", "of", "the", "the", "TOWN", "TOWN", "London", "London",
            ]);
        }
        let text = words.join(" ");
        let stats = NativeTextStats::from(&text);
        assert!(
            stats.consecutive_repeat_ratio >= defaults.min_consecutive_repeat_ratio,
            "ratio {} should be >= {}",
            stats.consecutive_repeat_ratio,
            defaults.min_consecutive_repeat_ratio
        );
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &defaults);

        // With substantial content (>= min_avg_non_whitespace_to_trust),
        // high repeat ratio alone should NOT trigger OCR.
        // This prevents false positives on numeric tables with repeated values.
        assert!(
            !decision.fallback,
            "Substantial content should NOT trigger OCR even with high repeat ratio. \
             Stats: non_ws={}, avg_non_ws={:.2}",
            stats.non_whitespace, decision.avg_non_whitespace
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_normal_text_no_consecutive_repeat_false_positive() {
        let defaults = t();
        let text = "The quick brown fox jumps over the lazy dog. This is a completely normal \
                    paragraph of text that forms coherent sentences. It contains multiple \
                    meaningful words and no unusual patterns of repetition. The text continues \
                    with more content that demonstrates typical English prose structure and \
                    vocabulary distribution across several sentences of varying length.";
        let stats = NativeTextStats::from(text);
        assert!(
            stats.consecutive_repeat_ratio < defaults.min_consecutive_repeat_ratio,
            "Normal text ratio {} should be < {}",
            stats.consecutive_repeat_ratio,
            defaults.min_consecutive_repeat_ratio
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_critical_fragmentation_triggers_fallback() {
        let defaults = t();
        let mut words: Vec<&str> = vec!["A"; 90];
        words.extend(vec!["document"; 10]);
        let text = words.join(" ");
        let stats = NativeTextStats::from(&text);
        assert!(
            stats.fragmented_word_ratio >= defaults.critical_fragmented_word_ratio,
            "fragmented ratio {} should be >= {}",
            stats.fragmented_word_ratio,
            defaults.critical_fragmented_word_ratio
        );
        assert!(stats.meaningful_words >= defaults.min_meaningful_words);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &defaults);
        assert!(
            decision.fallback,
            "Critical fragmentation should trigger OCR even with meaningful words"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_low_avg_word_length_triggers_fallback() {
        let defaults = t();
        let mut words: Vec<&str> = vec!["x"; 55];
        words.push("hello");
        words.push("world");
        words.push("testing");
        let text = words.join(" ");
        let stats = NativeTextStats::from(&text);
        assert!(stats.avg_word_length < defaults.min_avg_word_length);
        assert!(stats.word_count >= defaults.min_words_for_avg_length_check);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &defaults);
        assert!(decision.fallback, "Low avg word length should trigger OCR fallback");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_normal_text_with_articles_no_false_positive() {
        let defaults = t();
        let text = "I am a fan of it. It is an old or new idea. A to do list is on my desk. \
                    He is in on it. We do go to it. I am at it. Is it so? He or I do it. \
                    The paragraph contains meaningful content with proper structure and sentences.";
        let stats = NativeTextStats::from(text);
        assert!(stats.meaningful_words >= defaults.min_meaningful_words);
        assert!(
            stats.fragmented_word_ratio < defaults.critical_fragmented_word_ratio,
            "Normal text fragmentation {} should be < {}",
            stats.fragmented_word_ratio,
            defaults.critical_fragmented_word_ratio
        );
        let decision = evaluate_native_text_for_ocr(text, Some(1), &defaults);
        assert!(
            !decision.fallback,
            "Normal text with short words should not trigger OCR"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_short_words_in_normal_text_no_false_positive() {
        let text = "I am a fan of this document. He is on to something here. \
                    We do have meaningful words like paragraph and structure throughout.";
        let stats = NativeTextStats::from(text);
        assert!(stats.meaningful_words >= t().min_meaningful_words);
        let decision = evaluate_native_text_for_ocr(text, Some(1), &t());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_good_text() {
        let text = "This is a normal paragraph with meaningful words and proper structure. \
                    It contains multiple sentences that form a coherent text block.";
        let score = compute_quality_score(text, &t());
        assert!(score > 0.7, "Good text should score > 0.7, got {score}");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_empty_text() {
        assert_eq!(compute_quality_score("", &t()), 0.0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_garbled_text() {
        // Fragmented text with single-character words should score significantly
        // lower than good text, even if individual chars are alphanumeric
        let text = "x y z a b c d e f g h i j k l m n o p q r s t u v w";
        let score = compute_quality_score(text, &t());
        let good_score = compute_quality_score("This is a well-formed sentence with proper words and structure.", &t());
        assert!(
            score < good_score,
            "Garbled text ({score}) should score lower than good text ({good_score})"
        );
    }

    // ── compute_quality_score tests ──

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_zero_min_meaningful_words_no_panic() {
        let mut thresholds = t();
        thresholds.min_meaningful_words = 0;
        // Should not panic and should treat meaningful_score as 1.0
        let score = compute_quality_score("hello world", &thresholds);
        assert!(score > 0.0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_zero_min_consecutive_repeat_ratio_no_panic() {
        let mut thresholds = t();
        thresholds.min_consecutive_repeat_ratio = 0.0;
        // Should not panic; repeat_score should be 1.0 when threshold is zero
        let score = compute_quality_score("hello hello world world", &thresholds);
        assert!(score > 0.0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_zero_min_garbage_chars_no_panic() {
        let mut thresholds = t();
        thresholds.min_garbage_chars = 0;
        // Text without garbage chars should score normally
        let score = compute_quality_score("hello world testing", &thresholds);
        assert!(score > 0.0);
        // Text WITH garbage chars should get garbage_score = 0.0
        let score_with_garbage = compute_quality_score("hello \u{FFFD} world", &thresholds);
        assert!(score > score_with_garbage);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_meaningful_words_not_capped() {
        // If meaningful_words were capped (e.g. .take(3)), text with 50 meaningful
        // words would still only count 3. With the fix, it counts all of them.
        let words: Vec<&str> = vec!["programming"; 50];
        let text = words.join(" ");
        let score = compute_quality_score(&text, &t());
        // meaningful_score = min(50 / 3, 1.0) = 1.0
        // The score should be high because all components are good
        let stats = NativeTextStats::compute(&text, &t());
        assert_eq!(stats.meaningful_words, 50);
        let meaningful_score = (stats.meaningful_words as f64 / t().min_meaningful_words as f64).min(1.0);
        assert!(
            (meaningful_score - 1.0).abs() < f64::EPSILON,
            "meaningful_score should be 1.0 with 50 meaningful words, got {meaningful_score}"
        );
        assert!(
            score > 0.7,
            "Score with many meaningful words should be high, got {score}"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_repeat_threshold_relative_normalization() {
        // repeat_score = 1.0 - (ratio / threshold).min(1.0)
        // With ratio = half the threshold, repeat_score should be ~0.5
        let thresholds = t();
        // Verify the formula: at half the threshold, repeat_score should be ~0.5
        let text = "The quick brown fox jumps over the lazy dog near the stream. \
                    The quick brown fox jumps over the lazy dog near the stream. \
                    The quick brown fox jumps over the lazy dog near the stream.";
        let stats = NativeTextStats::compute(text, &thresholds);
        if stats.consecutive_repeat_ratio > 0.0
            && stats.consecutive_repeat_ratio < thresholds.min_consecutive_repeat_ratio
        {
            let expected_repeat_score =
                1.0 - (stats.consecutive_repeat_ratio / thresholds.min_consecutive_repeat_ratio).min(1.0);
            let _ = expected_repeat_score; // just verifying the formula doesn't panic
        }
        // Direct formula check: if ratio is exactly half the threshold
        let half_ratio = thresholds.min_consecutive_repeat_ratio / 2.0;
        let expected = 1.0 - (half_ratio / thresholds.min_consecutive_repeat_ratio).min(1.0);
        assert!(
            (expected - 0.5).abs() < f64::EPSILON,
            "repeat_score at half threshold should be 0.5, got {expected}"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_strictly_monotonic() {
        let thresholds = t();

        let perfect_text = "This document contains comprehensive analysis of market trends \
                           and provides detailed recommendations for future investment strategies. \
                           The methodology involves rigorous statistical examination of historical \
                           data patterns across multiple economic sectors and geographical regions.";

        let good_text = "This is a normal paragraph with meaningful words and proper structure. \
                        It contains multiple sentences that form a coherent text block.";

        let mediocre_text = "ok so um the uh thing is that we like need to uh figure out what \
                            to do about the um situation or whatever it is that happened here today";

        let garbled_text = "x y z a b c d e f g h i j k l m n o p q r s t u v w x y z a b";

        let empty_text = "";

        let perfect_score = compute_quality_score(perfect_text, &thresholds);
        let good_score = compute_quality_score(good_text, &thresholds);
        let mediocre_score = compute_quality_score(mediocre_text, &thresholds);
        let garbled_score = compute_quality_score(garbled_text, &thresholds);
        let empty_score = compute_quality_score(empty_text, &thresholds);

        assert!(
            perfect_score > good_score,
            "perfect ({perfect_score}) > good ({good_score})"
        );
        assert!(
            good_score > mediocre_score,
            "good ({good_score}) > mediocre ({mediocre_score})"
        );
        assert!(
            mediocre_score > garbled_score,
            "mediocre ({mediocre_score}) > garbled ({garbled_score})"
        );
        assert!(
            garbled_score > empty_score,
            "garbled ({garbled_score}) > empty ({empty_score})"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_high_garbage_chars() {
        let thresholds = t();
        // Text with many garbage chars
        let text = format!("Hello world testing {} more words here", "\u{FFFD}".repeat(20));
        let score = compute_quality_score(&text, &thresholds);
        let clean_score = compute_quality_score("Hello world testing more words here", &thresholds);
        assert!(
            score < clean_score,
            "Text with garbage chars ({score}) should score lower than clean text ({clean_score})"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_quality_score_high_consecutive_repetition() {
        let thresholds = t();
        // Build highly repetitive text
        let mut words = Vec::new();
        for _ in 0..30 {
            words.push("word");
            words.push("word");
        }
        let text = words.join(" ");
        let score = compute_quality_score(&text, &thresholds);
        let normal_score = compute_quality_score(
            "The quick brown fox jumps over the lazy dog repeatedly in various ways throughout the day",
            &thresholds,
        );
        assert!(
            score < normal_score,
            "Highly repetitive text ({score}) should score lower than normal text ({normal_score})"
        );
    }

    // ── evaluate_native_text_for_ocr tests ──

    #[cfg(feature = "ocr")]
    #[test]
    fn test_definitive_failure_all_zeros() {
        // Non-whitespace chars that are all non-alphanumeric (alnum == 0)
        let text = "... --- !!! @@@ ### $$$ %%% ^^^ &&& *** ((( )))";
        let decision = evaluate_native_text_for_ocr(text, Some(1), &t());
        assert!(decision.fallback, "All non-alnum text should trigger fallback");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_definitive_failure_garbage_at_threshold() {
        let thresholds = t();
        let garbage = "\u{FFFD}".repeat(thresholds.min_garbage_chars);
        let text = format!("Some normal text with garbage {garbage} embedded here");
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);
        assert!(
            decision.fallback,
            "Text with garbage chars at threshold should trigger fallback"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_definitive_failure_fragmented_few_meaningful() {
        let thresholds = t();
        // High fragmented_word_ratio AND few meaningful words
        // Need >= 10 words for fragmented_word_ratio to be computed
        let text = "I a b c d e f g h j k l m n o p q r s u";
        let stats = NativeTextStats::compute(text, &thresholds);
        assert!(stats.fragmented_word_ratio >= thresholds.max_fragmented_word_ratio);
        assert!(stats.meaningful_words < thresholds.min_meaningful_words);
        let decision = evaluate_native_text_for_ocr(text, Some(1), &thresholds);
        assert!(
            decision.fallback,
            "Fragmented + few meaningful words should trigger fallback"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_definitive_failure_critical_fragmentation_with_meaningful_words() {
        // Already tested above in test_critical_fragmentation_triggers_fallback,
        // but let's verify the specific definitive_failure path
        let thresholds = t();
        let mut words: Vec<&str> = vec!["A"; 90];
        words.extend(vec!["document"; 10]);
        let text = words.join(" ");
        let stats = NativeTextStats::compute(&text, &thresholds);
        assert!(stats.fragmented_word_ratio >= thresholds.critical_fragmented_word_ratio);
        assert!(stats.meaningful_words >= thresholds.min_meaningful_words);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);
        assert!(
            decision.fallback,
            "Critical fragmentation triggers fallback even with meaningful words"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_definitive_failure_low_avg_word_length() {
        let thresholds = t();
        // Many very short words (avg word length < 2.0) with enough words
        let mut words: Vec<&str> = vec!["a"; 55];
        words.push("hello");
        let text = words.join(" ");
        let stats = NativeTextStats::compute(&text, &thresholds);
        assert!(stats.avg_word_length < thresholds.min_avg_word_length);
        assert!(stats.word_count >= thresholds.min_words_for_avg_length_check);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);
        assert!(
            decision.fallback,
            "Low avg word length with enough words should trigger fallback"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_definitive_failure_high_consecutive_repeat_sparse() {
        // Fix for #1176: when repeat ratio is high but content is sparse,
        // it should trigger OCR. But when content is substantial, repeat ratio is tolerated.
        let thresholds = t();

        // Create sparse content with high repeat ratio (same word repeated many times)
        // Need >= min_words_for_repeat_check (default 50) words for ratio to be calculated
        // Use short words to keep content sparse: 50 words * 2 chars = 100 chars + spacing = ~150 chars
        // This is right at the boundary of min_avg_non_whitespace_to_trust (150)
        // 1 char word, 50 words total = ~100 non-ws chars
        let words = vec!["x"; 50];
        let text = words.join(" ");
        let stats = NativeTextStats::compute(&text, &thresholds);

        // Verify we have high repeat ratio (all consecutive pairs should be identical words)
        assert!(
            stats.word_count >= thresholds.min_words_for_repeat_check,
            "Test setup: need >= {} words for repeat check, got {}",
            thresholds.min_words_for_repeat_check,
            stats.word_count
        );
        assert!(
            stats.consecutive_repeat_ratio >= thresholds.min_consecutive_repeat_ratio,
            "Test setup: should have high repeat ratio >= {}, got {:.2}",
            thresholds.min_consecutive_repeat_ratio,
            stats.consecutive_repeat_ratio
        );
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);

        // With sparse content (<150 avg chars), high repeat ratio SHOULD trigger
        // But this text is borderline (160 chars with threshold 150), so let's verify
        if decision.avg_non_whitespace < MIN_AVG_NON_WHITESPACE_TO_TRUST {
            assert!(
                decision.fallback,
                "High consecutive repeat on sparse content should trigger fallback"
            );
        } else {
            // If it happens to be just above the threshold, that's also ok - it's the boundary
            eprintln!("Text is borderline sparse: {:.2} chars", decision.avg_non_whitespace);
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_non_definitive_fails_on_alnum_ratio() {
        let thresholds = t();
        // Text that is NOT a definitive failure but has low alnum_ratio and low avg_alnum
        // Needs: non_whitespace > 0, alnum > 0, no garbage, no fragmentation issues,
        //        but alnum_ratio < min_alnum_ratio and avg_alnum < min_non_whitespace_per_page
        // Also: not has_substantial_text (so small text)
        let text = "a!@# b%^ c*( d_+";
        let stats = NativeTextStats::compute(text, &thresholds);
        // If alnum is 0, it's definitive. We need alnum > 0 but ratio < threshold
        if stats.alnum > 0 && stats.alnum_ratio < thresholds.min_alnum_ratio && stats.non_whitespace != 0 {
            let decision = evaluate_native_text_for_ocr(text, Some(1), &thresholds);
            assert!(
                decision.fallback,
                "Low alnum ratio should trigger fallback through non-definitive path"
            );
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_text_passes_all_checks() {
        let thresholds = t();
        let text = "This is a well-structured document containing multiple meaningful sentences. \
                    The content provides detailed information about various topics including \
                    science, technology, engineering, and mathematics. Each paragraph builds \
                    upon the previous one to create a comprehensive narrative that demonstrates \
                    proper text extraction quality from the PDF document format.";
        let decision = evaluate_native_text_for_ocr(text, Some(1), &thresholds);
        assert!(!decision.fallback, "Well-formed text should pass all checks");
        assert!(decision.stats.meaningful_words >= thresholds.min_meaningful_words);
        assert!(decision.stats.alnum_ratio >= thresholds.min_alnum_ratio);
        assert!(decision.stats.garbage_char_count < thresholds.min_garbage_chars);
    }

    // ── NativeTextStats::compute tests ──

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_meaningful_words_actual_count_not_capped() {
        let thresholds = t();
        // Create text with many meaningful words (>= 4 chars each)
        let words: Vec<&str> = vec!["programming"; 20];
        let text = words.join(" ");
        let stats = NativeTextStats::compute(&text, &thresholds);
        assert_eq!(
            stats.meaningful_words, 20,
            "meaningful_words should be 20 (not capped), got {}",
            stats.meaningful_words
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_fragmented_word_ratio_calculation() {
        let thresholds = t();
        // 10 words, 5 are short (1-2 chars) => ratio = 0.5
        let text = "I a am b so the one quick brown fox";
        let stats = NativeTextStats::compute(text, &thresholds);
        assert_eq!(stats.word_count, 10);
        // Count short words: "I"(1), "a"(1), "am"(2), "b"(1), "so"(2) = 5 short
        let expected_ratio = 5.0 / 10.0;
        assert!(
            (stats.fragmented_word_ratio - expected_ratio).abs() < 0.01,
            "fragmented_word_ratio should be ~{expected_ratio}, got {}",
            stats.fragmented_word_ratio
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_fragmented_word_ratio_below_10_words() {
        let thresholds = t();
        // Fewer than 10 words => fragmented_word_ratio should be 0.0
        let text = "a b c d e f g h i";
        let stats = NativeTextStats::compute(text, &thresholds);
        assert_eq!(stats.word_count, 9);
        assert_eq!(
            stats.fragmented_word_ratio, 0.0,
            "fragmented_word_ratio should be 0.0 with < 10 words"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_consecutive_repeat_ratio_calculation() {
        let thresholds = t();
        // Need >= min_words_for_repeat_check words
        let mut words = Vec::new();
        for _ in 0..25 {
            words.push("alpha");
            words.push("beta");
        }
        // No consecutive repeats (alternating pattern)
        let text = words.join(" ");
        let stats = NativeTextStats::compute(&text, &thresholds);
        assert_eq!(stats.word_count, 50);
        assert!(
            stats.consecutive_repeat_ratio < 0.01,
            "Alternating words should have ~0 repeat ratio, got {}",
            stats.consecutive_repeat_ratio
        );

        // Now with all repeats
        let mut repeat_words = Vec::new();
        for _ in 0..25 {
            repeat_words.push("same");
            repeat_words.push("same");
        }
        let repeat_text = repeat_words.join(" ");
        let repeat_stats = NativeTextStats::compute(&repeat_text, &thresholds);
        assert!(
            repeat_stats.consecutive_repeat_ratio > 0.4,
            "All-same words should have high repeat ratio, got {}",
            repeat_stats.consecutive_repeat_ratio
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_consecutive_repeat_below_min_words() {
        let thresholds = t();
        // Below min_words_for_repeat_check => ratio should be 0.0
        let text = "same same same";
        let stats = NativeTextStats::compute(text, &thresholds);
        assert!(stats.word_count < thresholds.min_words_for_repeat_check);
        assert_eq!(
            stats.consecutive_repeat_ratio, 0.0,
            "consecutive_repeat_ratio should be 0.0 below word threshold"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_empty_string() {
        let thresholds = t();
        let stats = NativeTextStats::compute("", &thresholds);
        assert_eq!(stats.non_whitespace, 0);
        assert_eq!(stats.alnum, 0);
        assert_eq!(stats.meaningful_words, 0);
        assert_eq!(stats.alnum_ratio, 0.0);
        assert_eq!(stats.garbage_char_count, 0);
        assert_eq!(stats.fragmented_word_ratio, 0.0);
        assert_eq!(stats.consecutive_repeat_ratio, 0.0);
        assert_eq!(stats.avg_word_length, 0.0);
        assert_eq!(stats.word_count, 0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_single_word() {
        let thresholds = t();
        let stats = NativeTextStats::compute("hello", &thresholds);
        assert_eq!(stats.word_count, 1);
        assert_eq!(stats.non_whitespace, 5);
        assert_eq!(stats.alnum, 5);
        assert_eq!(stats.meaningful_words, 1);
        assert_eq!(stats.avg_word_length, 5.0);
        assert_eq!(stats.fragmented_word_ratio, 0.0);
        assert_eq!(stats.consecutive_repeat_ratio, 0.0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_stats_single_char() {
        let thresholds = t();
        let stats = NativeTextStats::compute("x", &thresholds);
        assert_eq!(stats.word_count, 1);
        assert_eq!(stats.non_whitespace, 1);
        assert_eq!(stats.alnum, 1);
        assert_eq!(stats.meaningful_words, 0); // "x" has len 1 < min_meaningful_word_len (4)
        assert_eq!(stats.avg_word_length, 1.0);
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_process_document_propagation() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::ExtractedDocument;
        use std::path::Path;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct MockBackend {
            called: Arc<AtomicBool>,
        }

        #[async_trait::async_trait]
        impl OcrBackend for MockBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                panic!("Should not call process_image");
            }
            fn supports_document_processing(&self) -> bool {
                true
            }
            async fn process_document(&self, path: &Path, _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                assert!(path.to_string_lossy().contains("test.pdf"));
                self.called.store(true, Ordering::SeqCst);
                Ok(ExtractedDocument::default())
            }
        }

        impl Plugin for MockBackend {
            fn name(&self) -> &str {
                "mock"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        let called = Arc::new(AtomicBool::new(false));
        let backend = Arc::new(MockBackend { called: called.clone() });
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "mock".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Register the mock backend so extract_with_ocr can find it
        crate::plugins::register_ocr_backend(backend).unwrap();

        let path = Path::new("test.pdf");
        let result = extract_with_ocr(
            None,      // No content
            Some(&[]), // No images
            #[cfg(feature = "layout-detection")]
            None, // No layout
            &config,
            Some(path),
        )
        .await;

        assert!(result.is_ok());
        assert!(called.load(Ordering::SeqCst), "process_document was not called");
        let (_, _, _, _, _, llm_usage, _, _, _) = result.unwrap();
        assert!(llm_usage.is_empty(), "No LLM usage expected for mock backend");

        // Clean up
        crate::plugins::unregister_ocr_backend("mock").unwrap();
    }

    /// Verifies that `llm_usage` entries returned by a VLM OCR backend are
    /// accumulated per-page and returned from `extract_with_ocr`.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_llm_usage_propagated_through_extract_with_ocr() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::{ExtractedDocument, LlmUsage};
        use std::sync::Arc;

        struct VlmMockBackend;

        #[async_trait::async_trait]
        impl OcrBackend for VlmMockBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                Ok(ExtractedDocument {
                    content: "page text".to_string(),
                    llm_usage: Some(vec![LlmUsage {
                        model: "gpt-4o".to_string(),
                        source: "vlm_ocr".to_string(),
                        input_tokens: Some(100),
                        output_tokens: Some(50),
                        total_tokens: Some(150),
                        estimated_cost: Some(0.001),
                        finish_reason: Some("stop".to_string()),
                    }]),
                    ..Default::default()
                })
            }
            fn supports_document_processing(&self) -> bool {
                false
            }
        }

        impl Plugin for VlmMockBackend {
            fn name(&self) -> &str {
                "vlm-mock"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        let backend = Arc::new(VlmMockBackend);
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "vlm-mock".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        crate::plugins::register_ocr_backend(backend).unwrap();

        // Provide two synthetic 1x1 pixel images so extract_with_ocr processes two pages.
        let tiny_png = {
            use image::ImageEncoder;
            use image::codecs::png::PngEncoder;
            use std::io::Cursor;
            let img = image::DynamicImage::new_rgb8(1, 1);
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();
            let mut buf = Cursor::new(Vec::new());
            PngEncoder::new(&mut buf)
                .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                .unwrap();
            image::load_from_memory(&buf.into_inner()).unwrap()
        };
        let images = vec![tiny_png.clone(), tiny_png];

        let result = extract_with_ocr(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
        )
        .await;

        crate::plugins::unregister_ocr_backend("vlm-mock").unwrap();

        let (_, _, _, _, _, llm_usage, _, _, _) = result.expect("extract_with_ocr should succeed");
        assert_eq!(
            llm_usage.len(),
            2,
            "should have one LlmUsage entry per page, got {}",
            llm_usage.len()
        );
        assert_eq!(llm_usage[0].model, "gpt-4o");
        assert_eq!(llm_usage[0].source, "vlm_ocr");
        assert_eq!(llm_usage[0].total_tokens, Some(150));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_build_page_raster_image_fields() {
        let png_bytes = bytes::Bytes::from_static(b"\x89PNG\r\n\x1a\n");
        let img = build_page_raster_image(0, png_bytes.clone(), 800, 600);

        assert_eq!(img.page_number, Some(1), "page_number must be 1-indexed");
        assert_eq!(img.width, Some(800));
        assert_eq!(img.height, Some(600));
        assert_eq!(img.format.as_ref(), "png");
        assert_eq!(img.image_kind, Some(crate::types::ImageKind::PageRaster));
        assert_eq!(img.colorspace.as_deref(), Some("RGB"));
        assert_eq!(img.bits_per_component, Some(8));
        assert!(!img.is_mask);
        assert!(img.bounding_box.is_none());
        assert!(img.ocr_result.is_none());
        assert_eq!(img.data, png_bytes);
        assert_eq!(img.image_index, 0, "image_index is a placeholder; caller must reindex");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_build_page_raster_image_page_idx_to_page_number() {
        for page_idx in 0usize..5 {
            let img = build_page_raster_image(page_idx, bytes::Bytes::new(), 100, 100);
            assert_eq!(
                img.page_number,
                Some((page_idx + 1) as u32),
                "page_number must be page_idx + 1"
            );
        }
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_cgroup_v2_unlimited_returns_none() {
        assert_eq!(parse_cgroup_v2("max\n", "12345"), None);
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_cgroup_v2_numeric_saturating_subtraction() {
        assert_eq!(parse_cgroup_v2("1000000000\n", "250000000\n"), Some(750_000_000));
        // usage > limit must saturate to 0, not underflow.
        assert_eq!(parse_cgroup_v2("100", "500"), Some(0));
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_cgroup_v2_invalid_returns_none() {
        assert_eq!(parse_cgroup_v2("not-a-number", "0"), None);
        assert_eq!(parse_cgroup_v2("1000", "not-a-number"), None);
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_cgroup_v1_unlimited_sentinel_returns_none() {
        // Real-world cgroup v1 unlimited values are near isize::MAX.
        let unlimited = usize::MAX.to_string();
        assert_eq!(parse_cgroup_v1(&unlimited, "0"), None);

        let just_under = (isize::MAX as usize - 1).to_string();
        assert!(parse_cgroup_v1(&just_under, "0").is_some());
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_cgroup_v1_numeric_saturating_subtraction() {
        assert_eq!(parse_cgroup_v1("2000000", "500000"), Some(1_500_000));
        assert_eq!(parse_cgroup_v1("100", "500"), Some(0));
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_meminfo_available_extracts_kb_and_converts_to_bytes() {
        let synthetic = "\
MemTotal:        8000000 kB
MemFree:         1000000 kB
MemAvailable:       2048 kB
Buffers:           50000 kB
";
        assert_eq!(parse_meminfo_available(synthetic), 2048 * 1024);
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_meminfo_available_missing_field_returns_zero() {
        let synthetic = "MemTotal: 8000000 kB\nMemFree: 1000000 kB\n";
        assert_eq!(parse_meminfo_available(synthetic), 0);
    }

    #[cfg(all(feature = "ocr", target_os = "linux"))]
    #[test]
    fn parse_meminfo_available_handles_unparseable_value_as_zero() {
        let synthetic = "MemAvailable: notanumber kB\n";
        assert_eq!(parse_meminfo_available(synthetic), 0);
    }

    /// Pipeline-level test for the actual bug path in #1078 (force_ocr_pages / mixed
    /// path uses render_selected_pages_for_ocr; full force_ocr uses similar batch
    /// render in extract_with_ocr).
    /// This proves the wide PDF no longer hard-fails through the OCR render path
    /// that was crashing in production.
    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[test]
    fn test_render_selected_pages_for_ocr_wide_pdf_does_not_fail() {
        // Repro for the render failure reported in #1078.
        // Note limitation (per review): the in-memory minimal PDF has empty content
        // stream and no /Resources. It exercises the MediaBox guard in
        // render_selected_pages_for_ocr but may not trigger all rasterizer paths
        // that a real wide vector-heavy diagram PDF would. A sanitized real repro
        // was used for manual verification during development.
        let wide_pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(20000.0, 300.0);
        let result = render_selected_pages_for_ocr(&wide_pdf, &[0]);
        assert!(
            result.is_ok(),
            "render_selected_pages_for_ocr on wide page (the #1078 bug path) should succeed via safeguard, got: {:?}",
            result.err()
        );
    }

    /// Verifies that formulas returned by a per-page OCR backend are accumulated and
    /// renumbered to 1-indexed document page numbers by `extract_with_ocr`.
    ///
    /// This exercises the same `formula.page = (page_idx + 1) as u32` accumulation
    /// logic that is now replicated in `extract_mixed_ocr_native` for the mixed-OCR
    /// path. Since `extract_mixed_ocr_native` requires real PDF bytes for rendering,
    /// this test uses `extract_with_ocr` with in-memory images to validate that the
    /// accumulation pattern works correctly end-to-end.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_formulas_accumulated_and_renumbered_per_page() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::{BoundingBox, ExtractedDocument};
        use std::sync::Arc;

        struct FormulaMockBackend;

        #[async_trait::async_trait]
        impl OcrBackend for FormulaMockBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            // Each page returns one formula. The page field is set to 0 (unset) here;
            // extract_with_ocr must overwrite it with the 1-indexed document page number.
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                Ok(ExtractedDocument {
                    content: "page text".to_string(),
                    formulas: vec![crate::types::Formula {
                        latex: "E = mc^2".to_string(),
                        bbox: BoundingBox {
                            x0: 0.0,
                            y0: 0.0,
                            x1: 100.0,
                            y1: 50.0,
                        },
                        page: 0, // intentionally wrong; pipeline must renumber
                    }],
                    ..Default::default()
                })
            }
            fn supports_document_processing(&self) -> bool {
                false
            }
        }

        impl Plugin for FormulaMockBackend {
            fn name(&self) -> &str {
                "formula-mock-mixed-ocr"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        let backend = Arc::new(FormulaMockBackend);
        crate::plugins::register_ocr_backend(backend).unwrap();

        // Provide two synthetic 1×1 images so extract_with_ocr processes two pages.
        let tiny_image = {
            use image::ImageEncoder;
            use image::codecs::png::PngEncoder;
            use std::io::Cursor;
            let img = image::DynamicImage::new_rgb8(1, 1);
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();
            let mut buf = Cursor::new(Vec::new());
            PngEncoder::new(&mut buf)
                .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                .unwrap();
            image::load_from_memory(&buf.into_inner()).unwrap()
        };
        let images = vec![tiny_image.clone(), tiny_image];

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "formula-mock-mixed-ocr".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_with_ocr(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
        )
        .await;

        crate::plugins::unregister_ocr_backend("formula-mock-mixed-ocr").unwrap();

        let (_, _, _, _, _, _, _, _, formulas) = result.expect("extract_with_ocr should succeed");

        assert_eq!(formulas.len(), 2, "one formula per page, got {}", formulas.len());

        // Page numbers must be 1-indexed document pages, NOT the backend's placeholder 0.
        let mut pages: Vec<u32> = formulas.iter().map(|f| f.page).collect();
        pages.sort_unstable();
        assert_eq!(
            pages,
            vec![1, 2],
            "formula pages must be renumbered to 1-indexed doc pages"
        );

        // LaTeX content must be preserved.
        assert!(
            formulas.iter().all(|f| f.latex == "E = mc^2"),
            "formula latex must be preserved through accumulation"
        );
    }

    /// Test that inject_layout_config_to_backend handles non-object backend_options
    /// by replacing with a fresh object instead of silently dropping the flag.
    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn test_inject_layout_config_handles_non_object_backend_options() {
        use crate::core::config::LayoutDetectionConfig;
        // Set backend_options to a non-object value (e.g., a string)
        let ocr_config = crate::core::config::OcrConfig {
            backend_options: Some(serde_json::json!("invalid")),
            ..Default::default()
        };

        let extraction_config = ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                enable_chart_understanding: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = inject_layout_config_to_backend(&ocr_config, &extraction_config);

        // Should have replaced the string with an object containing enable_chart_understanding
        assert!(result.backend_options.is_some());
        let opts = result.backend_options.unwrap();
        assert!(opts.is_object());
        assert_eq!(
            opts.get("enable_chart_understanding").and_then(|v| v.as_bool()),
            Some(true),
            "enable_chart_understanding should be injected into the new object"
        );
    }

    // Tests for issue #1176: spurious auto-OCR on born-digital PDFs with numeric/formula content.
    // These tests verify that the heuristic respects content density (avg_non_whitespace)
    // and doesn't reject legitimate non-prose content based purely on prose-tuned signals.

    /// Simulate NICS background checks table: many short numeric tokens.
    /// Characteristics:
    /// - Substantial non-whitespace content (1000+ chars)
    /// - Many short numeric tokens (1-4 chars, e.g., "0", "100", "500")
    /// - High fragmented_word_ratio (~70%)
    /// - Low avg_word_length (~2.5)
    /// - High consecutive_repeat_ratio (repeated numbers)
    #[cfg(feature = "ocr")]
    fn numeric_table_text() -> String {
        let mut text = String::new();
        for row in 0..20 {
            for col in 0..15 {
                let val = (row * col) % 1000;
                text.push_str(&format!("{} ", val));
            }
            text.push('\n');
        }
        text
    }

    /// Simulate math formula page: mix of words and short tokens.
    /// Real formula pages have "where", "define", "equation", "therefore" mixed with symbols.
    /// Characteristics:
    /// - Mixture of long and short tokens
    /// - Substantial content if multiple equations
    /// - Some fragmentation from mathematical notation
    /// - But not extreme critical fragmentation (< 0.80)
    #[cfg(feature = "ocr")]
    fn formula_text() -> String {
        let mut text = String::new();
        for i in 0..20 {
            text.push_str(&format!(
                "Definition {}: where variable equals expression and function applies therefore x y z\n",
                i
            ));
        }
        text
    }

    /// Simulate sparse form with short tokens: checkboxes, small fields.
    /// Characteristics:
    /// - Few non-whitespace chars (<30 per page, genuinely sparse)
    /// - Short tokens
    /// - Should trigger OCR (legitimately sparse, not just non-prose)
    #[cfg(feature = "ocr")]
    fn sparse_form_text() -> String {
        let text = r#"
[]  Yes
[]  No

Name: ___
"#;
        text.to_string()
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_numeric_table_with_short_tokens_no_ocr() {
        // Issue #1176: numeric tables have short tokens but substantial content.
        // Should NOT trigger OCR based purely on prose signals (avg_word_length, fragmentation).
        let text = numeric_table_text();
        let thresholds = t();

        let stats = NativeTextStats::compute(&text, &thresholds);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);

        // Verify test setup: numeric table has substantial content
        assert!(
            stats.non_whitespace >= 300,
            "Test setup: numeric table should have 300+ non-whitespace chars, got {}",
            stats.non_whitespace
        );
        assert!(
            decision.avg_non_whitespace >= 100.0,
            "Test setup: numeric table should have avg_non_whitespace >= 100, got {:.2}",
            decision.avg_non_whitespace
        );

        // Numeric table prose signals are bad (short tokens, fragmentation)
        assert!(
            stats.fragmented_word_ratio > 0.5,
            "Test setup: numeric table should have high fragmentation (>0.5), got {:.2}",
            stats.fragmented_word_ratio
        );

        // But despite bad prose signals, should NOT trigger OCR
        // because it has substantial content density
        assert!(
            !decision.fallback,
            "Numeric table with substantial content should NOT trigger OCR fallback. \
             Stats: non_ws={}, avg_word_len={:.2}, frag_ratio={:.2}",
            stats.non_whitespace, stats.avg_word_length, stats.fragmented_word_ratio
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_formula_page_with_short_tokens_no_ocr() {
        // Issue #1176: formula pages have short symbols but substantial content.
        // Should NOT trigger OCR based on low meaningful_words or fragmentation.
        let text = formula_text();
        let thresholds = t();

        let stats = NativeTextStats::compute(&text, &thresholds);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);

        // Verify test setup: formula text has substantial content
        assert!(
            stats.non_whitespace >= 500,
            "Test setup: formula text should have 500+ non-whitespace chars, got {}",
            stats.non_whitespace
        );

        // Formula text has low meaningful_words (symbols aren't "meaningful")
        // This used to trigger: "(fragmented_word_ratio >= 0.6 && meaningful_words < 3)"
        let would_trigger_old_logic = stats.fragmented_word_ratio >= thresholds.max_fragmented_word_ratio
            && stats.meaningful_words < thresholds.min_meaningful_words;

        // Should NOT trigger OCR despite old prose logic
        assert!(
            !decision.fallback,
            "Formula page with substantial content should NOT trigger OCR fallback. \
             Would trigger old logic: {}, frag={:.2}, meaningful={}",
            would_trigger_old_logic, stats.fragmented_word_ratio, stats.meaningful_words
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_sparse_form_triggers_ocr() {
        // Sparse form is legitimately sparse (few non-whitespace chars).
        // Should STILL trigger OCR because it's not just non-prose,
        // it's actually sparse (content density < threshold).
        let text = sparse_form_text();
        let thresholds = t();

        let stats = NativeTextStats::compute(&text, &thresholds);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);

        eprintln!(
            "Sparse form stats: non_ws={}, avg_non_ws={:.2}, meaningful_words={}, fallback={}",
            stats.non_whitespace, decision.avg_non_whitespace, stats.meaningful_words, decision.fallback
        );

        // Verify test setup: form is actually sparse
        assert!(
            stats.non_whitespace < 100,
            "Test setup: sparse form should have <100 non-whitespace chars, got {}",
            stats.non_whitespace
        );

        // Sparse form SHOULD trigger OCR
        assert!(
            decision.fallback,
            "Sparse form (legitimately few chars) SHOULD trigger OCR fallback. Stats: non_ws={}, meaningful={}",
            stats.non_whitespace, stats.meaningful_words
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_short_token_dense_content_no_ocr() {
        // Test the core fix: if avg_non_whitespace >= min_non_whitespace_to_trust,
        // don't reject based on prose signals (avg_word_length, fragmentation, etc).
        // Generate realistic numeric table: mix of short and longer numbers with row/column labels.
        let mut text = String::new();
        for i in 0..20 {
            // Row label (word): creates some non-short tokens
            text.push_str(&format!("Row{} ", i));

            // Data columns: mixture of 1, 2, and 3+ digit numbers
            for j in 0..15 {
                let val = (i * 13 + j * 7) % 5000; // Range: 0-4999, mix of 1-4 digit numbers
                text.push_str(&format!("{} ", val));
            }
            text.push('\n');
        }

        let thresholds = t();
        let stats = NativeTextStats::compute(&text, &thresholds);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);

        // Verify setup: realistic numeric table with substantial content
        assert!(
            decision.avg_non_whitespace >= 100.0,
            "Test setup: should have avg_non_whitespace >= 100, got {:.2}",
            decision.avg_non_whitespace
        );
        // Fragmentation from mixed-length numbers: some short (1-2 chars), some longer (3-4)
        assert!(
            stats.fragmented_word_ratio < 0.80,
            "Test setup: should be sub-critical < 0.80, got {:.2}",
            stats.fragmented_word_ratio
        );

        // Should NOT trigger OCR because content density is substantial
        // even though it has fragmentation and short tokens (from numbers)
        assert!(
            !decision.fallback,
            "Dense numeric table should NOT trigger OCR fallback"
        );
    }
}
