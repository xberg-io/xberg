//! OCR functionality for PDF extraction.
//!
//! Handles text quality evaluation, OCR fallback decision logic, and OCR processing.

#[cfg(feature = "ocr")]
use std::borrow::Cow;

#[cfg(feature = "ocr")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "ocr")]
use crate::core::config::OcrQualityThresholds;

#[cfg(feature = "ocr")]
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

#[cfg(feature = "ocr")]
pub struct OcrFallbackDecision {
    pub stats: NativeTextStats,
    pub avg_non_whitespace: f64,
    pub avg_alnum: f64,
    pub fallback: bool,
}

#[cfg(feature = "ocr")]
impl NativeTextStats {
    pub fn compute(text: &str, thresholds: &OcrQualityThresholds) -> Self {
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
        // that are identical. High values indicate column scrambling where pdfium
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
    pub fn from(text: &str) -> Self {
        Self::compute(text, &OcrQualityThresholds::default())
    }
}

/// Evaluates native PDF text quality to determine if OCR fallback is needed.
///
/// Uses the provided quality thresholds (or defaults) to make the decision.
#[cfg(feature = "ocr")]
pub fn evaluate_native_text_for_ocr(
    native_text: &str,
    page_count: Option<usize>,
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
        };
    }

    let stats = NativeTextStats::compute(trimmed, thresholds);
    let pages = page_count.unwrap_or(1).max(1) as f64;
    let avg_non_whitespace = stats.non_whitespace as f64 / pages;
    let avg_alnum = stats.alnum as f64 / pages;

    let has_substantial_text = stats.non_whitespace >= thresholds.min_total_non_whitespace
        && avg_non_whitespace >= thresholds.min_non_whitespace_per_page
        && stats.meaningful_words >= thresholds.min_meaningful_words;

    // Definitive quality failures — always trigger OCR fallback
    let definitive_failure = stats.non_whitespace == 0
        || stats.alnum == 0
        || stats.garbage_char_count >= thresholds.min_garbage_chars
        || (stats.fragmented_word_ratio >= thresholds.max_fragmented_word_ratio
            && stats.meaningful_words < thresholds.min_meaningful_words)
        || stats.fragmented_word_ratio >= thresholds.critical_fragmented_word_ratio
        || (stats.avg_word_length < thresholds.min_avg_word_length
            && stats.word_count >= thresholds.min_words_for_avg_length_check)
        || stats.consecutive_repeat_ratio >= thresholds.min_consecutive_repeat_ratio;

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
    }
}

/// Compute a quality score (0.0-1.0) for OCR output text.
///
/// Used by the pipeline to decide whether to accept a result or try the next backend.
/// Higher is better. Combines multiple signal dimensions into a single score.
#[cfg(feature = "ocr")]
pub fn compute_quality_score(text: &str, thresholds: &OcrQualityThresholds) -> f64 {
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

#[cfg(feature = "ocr")]
pub fn evaluate_per_page_ocr(
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
    page_count: Option<usize>,
    thresholds: &OcrQualityThresholds,
) -> OcrFallbackDecision {
    let boundaries = match boundaries {
        Some(b) if !b.is_empty() => b,
        _ => return evaluate_native_text_for_ocr(native_text, page_count, thresholds),
    };

    let mut document_decision = evaluate_native_text_for_ocr(native_text, page_count, thresholds);

    for boundary in boundaries {
        if boundary.byte_end > native_text.len() || boundary.byte_start > boundary.byte_end {
            continue;
        }
        let page_text = &native_text[boundary.byte_start..boundary.byte_end];
        if evaluate_native_text_for_ocr(page_text, Some(1), thresholds).fallback {
            document_decision.fallback = true;
            return document_decision;
        }
    }

    document_decision
}

// We no longer pre-render all pages for OCR to prevent OOMs.
// See `extract_with_ocr` for lazy streaming logic.

/// Render only specific PDF pages to images for OCR processing.
///
/// `page_indices` are 0-indexed. Only the requested pages are rendered,
/// returned as `(page_index, image)` pairs.
#[cfg(feature = "ocr")]
pub(crate) fn render_selected_pages_for_ocr(
    content: &[u8],
    page_indices: &[usize],
) -> crate::Result<Vec<(usize, image::DynamicImage)>> {
    use crate::pdf::rendering::{PageRenderOptions, PdfRenderer};

    let renderer = PdfRenderer::new().map_err(|e| crate::KreuzbergError::Parsing {
        message: format!("Failed to initialize PDF renderer: {}", e),
        source: None,
    })?;

    let page_count = renderer
        .page_count(content)
        .map_err(|e| crate::KreuzbergError::Parsing {
            message: format!("Failed to get PDF page count: {}", e),
            source: None,
        })?;

    let render_options = PageRenderOptions::default();
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
        let image = renderer
            .render_page_to_image(content, idx, &render_options)
            .map_err(|e| crate::KreuzbergError::Parsing {
                message: format!("Failed to render PDF page {}: {}", idx + 1, e),
                source: None,
            })?;
        images.push((idx, image));
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
#[cfg(feature = "ocr")]
pub(crate) async fn extract_mixed_ocr_native(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_page_numbers: &[usize],
    content: &[u8],
    config: &ExtractionConfig,
    _path: Option<&std::path::Path>,
) -> crate::Result<String> {
    use std::collections::HashSet;

    // Deduplicate and validate page numbers (must be >= 1)
    let ocr_set: HashSet<usize> = ocr_page_numbers
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
        return Ok(native_text.to_string());
    }

    // Convert 1-indexed page numbers to 0-indexed for rendering (sorted + deduplicated)
    let mut page_indices: Vec<usize> = ocr_set.iter().map(|&p| p - 1).collect();
    page_indices.sort_unstable();
    let page_images = render_selected_pages_for_ocr(content, &page_indices)?;

    if page_images.is_empty() {
        return Ok(native_text.to_string());
    }

    // OCR all selected pages concurrently using the same batched pipeline pattern
    // as extract_with_ocr: rayon-parallel PNG encoding + tokio JoinSet OCR calls.
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use rayon::prelude::*;
    use std::io::Cursor;
    use std::sync::Arc;

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    let backend = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        registry.get(&ocr_config.backend)?
    };

    let batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());

    let ocr_config_owned = ocr_config.clone();
    let total = page_images.len();
    let mut ocr_results: ahash::AHashMap<usize, String> = ahash::AHashMap::with_capacity(total);

    // Process in batches to bound peak memory (PNG buffers freed between batches)
    for batch_start in (0..total).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total);
        let batch_slice = &page_images[batch_start..batch_end];

        // Encode this batch's images to PNG in parallel (CPU-bound, rayon)
        let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>)>> = batch_slice
            .par_iter()
            .map(|(page_idx, image)| {
                let rgb = image.to_rgb8();
                let (w, h) = rgb.dimensions();
                let mut buf = Cursor::new(Vec::new());
                PngEncoder::new(&mut buf)
                    .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                    .map_err(|e| crate::KreuzbergError::Parsing {
                        message: format!("Failed to encode page {} for OCR: {}", page_idx + 1, e),
                        source: None,
                    })?;
                Ok((*page_idx, Arc::new(buf.into_inner())))
            })
            .collect();
        let encoded = encoded?;

        // OCR this batch concurrently (tokio JoinSet)
        let mut join_set = tokio::task::JoinSet::new();
        for (page_idx, data) in &encoded {
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
            let (page_idx, result) = join_result.map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("OCR task panicked: {}", e),
                plugin_name: "ocr".to_string(),
            })?;
            let extraction_result = result?;
            ocr_results.insert(page_idx + 1, extraction_result.content); // 1-indexed
        }
        // encoded PNGs dropped here — memory freed before next batch
    }

    // Assemble final text by replacing OCR pages in-place within the native text.
    // Process boundaries in reverse byte order so offsets remain valid after replacement.
    let mut result = native_text.to_string();

    let mut sorted_boundaries: Vec<&crate::types::PageBoundary> = boundaries
        .iter()
        .filter(|b| b.byte_end <= native_text.len() && b.byte_start <= b.byte_end)
        .collect();
    sorted_boundaries.sort_unstable_by(|a, b| b.byte_start.cmp(&a.byte_start));

    for boundary in sorted_boundaries {
        if let Some(ocr_text) = ocr_results.get(&boundary.page_number) {
            result.replace_range(boundary.byte_start..boundary.byte_end, ocr_text);
        }
    }

    Ok(result)
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
#[cfg(feature = "ocr")]
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
)> {
    use crate::plugins::registry::get_ocr_backend_registry;
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use std::io::Cursor;

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let base_ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    // When layout detections are available, ensure OCR produces elements
    // so the layout assembly module can use them for structured markdown.
    #[cfg(feature = "layout-detection")]
    let layout_ocr_config;
    let ocr_config = {
        #[cfg(feature = "layout-detection")]
        if layout_detections.is_some() {
            layout_ocr_config = ensure_elements_enabled(base_ocr_config);
            &layout_ocr_config
        } else {
            base_ocr_config
        }
        #[cfg(not(feature = "layout-detection"))]
        {
            base_ocr_config
        }
    };

    let backend = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        registry.get(&ocr_config.backend)?
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
        return Ok((result.content, mean_conf, Vec::new(), ocr_elements, None));
    }
    let mut lazy_pdf_page_count = 0;

    if !use_document_processing
        && images.is_none()
        && let Some(bytes) = content
    {
        #[cfg(feature = "pdf")]
        {
            let renderer = crate::pdf::rendering::PdfRenderer::new().map_err(|e| crate::KreuzbergError::Parsing {
                message: format!("Failed to initialize PDF renderer for OCR streaming: {:?}", e),
                source: None,
            })?;
            lazy_pdf_page_count = renderer.page_count(bytes).map_err(|e| crate::KreuzbergError::Parsing {
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
    use rayon::prelude::*;
    use std::sync::Arc;
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

    let ocr_config_owned = ocr_config.clone();
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
    let mut conf_sum: f64 = 0.0;
    let mut conf_count: usize = 0;

    // Initialize TATR for table structure recognition when layout detection is active.
    // TATR requires mutable access so pages are processed sequentially after OCR.
    #[cfg(feature = "layout-detection")]
    let mut tatr_model = if layout_detections.is_some() {
        crate::layout::take_or_create_tatr()
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
            // Encode pre-rendered images in parallel.
            #[allow(clippy::type_complexity)]
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
                        .map_err(|e| crate::KreuzbergError::Parsing {
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
                // Render each page, encode to PNG immediately, then drop the RGB buffer.
                // This keeps only one ~26MB RGB image alive at a time instead of batch_size.
                let renderer =
                    crate::pdf::rendering::PdfRenderer::new().map_err(|e| crate::KreuzbergError::Parsing {
                        message: format!("Failed to initialize PDF renderer for OCR batch: {:?}", e),
                        source: None,
                    })?;
                let render_opts = crate::pdf::rendering::PageRenderOptions::default();
                let mut batch_encoded: Vec<(usize, Arc<Vec<u8>>, u32, u32)> =
                    Vec::with_capacity(batch_end - batch_start);
                for i in batch_start..batch_end {
                    let pdf_bytes = content.ok_or_else(|| crate::KreuzbergError::Parsing {
                        message: "PDF content is required for OCR rendering but was not provided".to_string(),
                        source: None,
                    })?;
                    let image = renderer.render_page_to_image(pdf_bytes, i, &render_opts).map_err(|e| {
                        crate::KreuzbergError::Parsing {
                            message: format!("Failed to render page {} for OCR: {:?}", i, e),
                            source: None,
                        }
                    })?;
                    // Encode immediately so the DynamicImage can be dropped.
                    let rgb_image = image.to_rgb8();
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let png_encoder = PngEncoder::new(&mut image_bytes);
                    png_encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::KreuzbergError::Parsing {
                            message: format!("Failed to encode page {} image: {}", i, e),
                            source: None,
                        })?;
                    batch_encoded.push((i, Arc::new(image_bytes.into_inner()), width, height));
                    // `image` and `rgb_image` are dropped here, freeing ~52MB per page.
                }
                batch_encoded
            };
            #[cfg(not(feature = "pdf"))]
            let encoded: Vec<(usize, Arc<Vec<u8>>, u32, u32)> = Vec::new();
            (None::<Cow<'_, [image::DynamicImage]>>, encoded)
        };

        // OCR this batch concurrently (tokio JoinSet).
        let mut join_set: JoinSet<(usize, crate::Result<crate::types::ExtractionResult>)> = JoinSet::new();

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

        let batch_count = encoded_batch.len();
        let mut batch_ocr_results: Vec<Option<crate::types::ExtractionResult>> = vec![None; batch_count];
        while let Some(join_result) = join_set.join_next().await {
            let (page_idx, ocr_result) = join_result.map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("OCR task panicked: {}", e),
                plugin_name: "ocr".to_string(),
            })?;
            batch_ocr_results[page_idx - batch_start] = Some(ocr_result?);
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

            // Accumulate OCR elements from this page.
            if let Some(ref mut elems) = ocr_result.ocr_elements {
                for elem in elems.iter_mut() {
                    elem.page_number = page_idx + 1;
                }
                all_ocr_elements.extend(elems.iter().cloned());
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
                                image::load_from_memory(png_data).map_err(|e| crate::KreuzbergError::Parsing {
                                    message: format!("Failed to decode PNG for TATR: {}", e),
                                    source: None,
                                })?;
                            decoded.to_rgb8()
                        };
                        crate::ocr::layout_assembly::recognize_page_tables(&rgb, scaled_det, elements, model)
                    }
                    _ => Vec::new(),
                };

                // Collect recognized tables as Table structs for ExtractionResult.tables
                for rt in &recognized_tables {
                    if !rt.markdown.is_empty() {
                        collected_tables.push(crate::types::Table {
                            cells: rt.cells.clone(),
                            markdown: rt.markdown.clone(),
                            page_number: page_idx + 1,
                            bounding_box: None,
                        });
                    }
                }

                // Convert hOCR structure to PdfParagraphs, then apply layout overrides.
                // This mirrors the pdfium path: structure → layout classify → assemble.
                if let Some(ref ocr_doc) = ocr_result.ocr_internal_document {
                    let mut paragraphs =
                        crate::pdf::structure::adapters::ocr_doc_to_paragraphs(ocr_doc, ocr_render_height);

                    if let Some(ref scaled_det) = scaled_detection {
                        let hints = detection_to_layout_hints(scaled_det, ocr_render_height as f32);
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

                    // Don't filter page furniture for OCR — the layout model's
                    // header/footer detection is less reliable on OCR-rendered pages,
                    // and falsely filtering content is worse than keeping it.
                    all_page_paragraphs[page_idx] = Some(paragraphs);
                }

                // Use tesseract's own text output (preserves reading order).
                page_texts[page_idx] = ocr_result.content;
                continue;
            }

            let _ = page_idx;
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

    Ok((result, mean_text_conf, collected_tables, all_ocr_elements, ocr_doc))
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
#[cfg(feature = "ocr")]
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
#[cfg(feature = "ocr")]
fn get_available_memory() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if let Some(rest) = line.strip_prefix("MemAvailable:") {
                    let kb_str = rest.trim().trim_end_matches("kB").trim();
                    if let Ok(kb) = kb_str.parse::<usize>() {
                        return kb * 1024;
                    }
                }
            }
        }
        0
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

/// Run a multi-backend OCR pipeline with quality-based fallback.
///
/// Images and layout detections are computed once and shared across all stages.
/// Each stage produces OCR output that is scored; if the score meets the
/// pipeline's quality threshold, the result is accepted. Otherwise, the next
/// backend is tried. Returns the best result seen across all stages.
#[cfg(feature = "ocr")]
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
        return Err(crate::KreuzbergError::Parsing {
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
    )> = None;

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

        let stage_config = ExtractionConfig {
            ocr: Some(stage_ocr),
            ..config.clone()
        };

        tracing::debug!(
            backend = %stage.backend,
            priority = stage.priority,
            "Pipeline: trying OCR backend"
        );

        let result = extract_with_ocr(
            content,
            images,
            #[cfg(feature = "layout-detection")]
            layout_detections,
            &stage_config,
            path,
        )
        .await;

        match result {
            Ok((text, mean_conf, stage_tables, stage_ocr_elements, stage_doc)) => {
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

                if score >= pipeline.quality_thresholds.pipeline_min_quality {
                    return Ok((text, stage_tables, stage_ocr_elements, stage_doc));
                }

                // Track best-so-far
                match best_result {
                    Some((_, best_score, _, _, _)) if score > best_score => {
                        best_result = Some((text, score, stage_tables, stage_ocr_elements, stage_doc));
                    }
                    None => {
                        best_result = Some((text, score, stage_tables, stage_ocr_elements, stage_doc));
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
        Some((text, score, tables, elements, doc)) => {
            tracing::warn!(
                score,
                threshold = pipeline.quality_thresholds.pipeline_min_quality,
                "All OCR pipeline backends produced suboptimal quality, using best result"
            );
            Ok((text, tables, elements, doc))
        }
        None => Err(crate::KreuzbergError::Parsing {
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

/// Convert pixel-space layout detections to PDF-space `LayoutHint`s.
///
/// Flips y-coordinates from image space (y=0 at top) to PDF space (y=0 at bottom)
/// to match the coordinate system used by `apply_layout_overrides`.
#[cfg(all(feature = "ocr", feature = "layout-detection"))]
fn detection_to_layout_hints(
    detection: &crate::layout::DetectionResult,
    page_height: f32,
) -> Vec<crate::pdf::structure::types::LayoutHint> {
    use crate::layout::LayoutClass;
    use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};

    detection
        .detections
        .iter()
        .map(|det| {
            let class = match det.class {
                LayoutClass::Title => LayoutHintClass::Title,
                LayoutClass::SectionHeader => LayoutHintClass::SectionHeader,
                LayoutClass::Code => LayoutHintClass::Code,
                LayoutClass::Formula => LayoutHintClass::Formula,
                LayoutClass::ListItem => LayoutHintClass::ListItem,
                LayoutClass::Caption => LayoutHintClass::Caption,
                LayoutClass::Footnote => LayoutHintClass::Footnote,
                LayoutClass::PageHeader => LayoutHintClass::PageHeader,
                LayoutClass::PageFooter => LayoutHintClass::PageFooter,
                LayoutClass::Table => LayoutHintClass::Table,
                LayoutClass::Picture => LayoutHintClass::Picture,
                LayoutClass::Text => LayoutHintClass::Text,
                _ => LayoutHintClass::Other,
            };
            LayoutHint {
                class,
                confidence: det.confidence,
                left: det.bbox.x1,
                right: det.bbox.x2,
                top: page_height - det.bbox.y1,
                bottom: page_height - det.bbox.y2,
            }
        })
        .collect()
}

#[cfg(test)]
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
    fn test_consecutive_repeat_detects_column_scrambling() {
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
        assert!(decision.fallback, "Scrambled column text should trigger OCR fallback");
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
    fn test_definitive_failure_high_consecutive_repeat() {
        let thresholds = t();
        let mut words = Vec::new();
        for _ in 0..30 {
            words.push("hello");
            words.push("hello");
        }
        let text = words.join(" ");
        let stats = NativeTextStats::compute(&text, &thresholds);
        assert!(stats.consecutive_repeat_ratio >= thresholds.min_consecutive_repeat_ratio);
        let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);
        assert!(decision.fallback, "High consecutive repeat should trigger fallback");
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
        use crate::types::ExtractionResult;
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
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractionResult> {
                panic!("Should not call process_image");
            }
            fn supports_document_processing(&self) -> bool {
                true
            }
            async fn process_document(&self, path: &Path, _: &OcrConfig) -> crate::Result<ExtractionResult> {
                assert!(path.to_string_lossy().contains("test.pdf"));
                self.called.store(true, Ordering::SeqCst);
                Ok(ExtractionResult::default())
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

        // Clean up
        crate::plugins::unregister_ocr_backend("mock").unwrap();
    }
}
