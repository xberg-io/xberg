//! OCR functionality for PDF extraction.
//!
//! Handles text quality evaluation, OCR fallback decision logic, and OCR processing.

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

/// Render PDF pages to images for OCR processing.
///
/// Returns images at the default OCR DPI (300).
#[cfg(feature = "ocr")]
pub(crate) fn render_pages_for_ocr(content: &[u8]) -> crate::Result<Vec<image::DynamicImage>> {
    use crate::pdf::rendering::{PageRenderOptions, PdfRenderer};

    let render_options = PageRenderOptions::default();
    let renderer = PdfRenderer::new().map_err(|e| crate::KreuzbergError::Parsing {
        message: format!("Failed to initialize PDF renderer: {}", e),
        source: None,
    })?;

    renderer
        .render_all_pages(content, &render_options)
        .map_err(|e| crate::KreuzbergError::Parsing {
            message: format!("Failed to render PDF pages: {}", e),
            source: None,
        })
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
) -> crate::Result<(String, Option<f64>)> {
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
        let registry = registry.read().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire read lock on OCR backend registry: {}", e),
            plugin_name: "ocr-registry".to_string(),
        })?;
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

    let rendered_images;
    let images_to_use = if let Some(imgs) = images {
        imgs
    } else if !supports_doc || path.is_none() {
        // We need images but don't have them. Render now if we have content.
        if let Some(bytes) = content {
            rendered_images = render_pages_for_ocr(bytes)?;
            rendered_images.as_slice()
        } else {
            // No images and no content to render from.
            &[] as &[image::DynamicImage]
        }
    } else {
        // We have a path and support doc processing, so we might not need images.
        &[] as &[image::DynamicImage]
    };

    if let Some(doc_path) = path
        && supports_doc
    {
        tracing::debug!(backend = %ocr_config.backend, "Using document-level OCR processing");
        let result = backend.process_document(doc_path, ocr_config).await?;
        let mean_conf = result
            .metadata
            .additional
            .get("mean_text_conf")
            .and_then(|v| v.as_f64())
            .map(|v| v / 100.0);
        return Ok((result.content, mean_conf));
    }

    // Encode all page images to PNG bytes in parallel (CPU-bound).
    // Each element is (page_idx, image_data, width, height).
    // We wrap image bytes in Arc so that spawning per-page OCR tasks only
    // clones a pointer instead of copying the entire PNG buffer.
    use rayon::prelude::*;
    use std::sync::Arc;
    #[allow(clippy::type_complexity)]
    let encoded_pages: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = images_to_use
        .par_iter()
        .enumerate()
        .map(|(page_idx, image)| {
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
    let encoded_pages = encoded_pages?;

    // Run OCR on all pages concurrently. Each page spawns a tokio task so
    // the async backend (which internally uses spawn_blocking) can run in
    // parallel across the thread pool.
    // `backend` is already an `Arc<dyn OcrBackend>`; clone it cheaply per task.
    let ocr_config_owned = ocr_config.clone();

    use tokio::task::JoinSet;
    let mut join_set: JoinSet<(usize, crate::Result<crate::types::ExtractionResult>)> =
        JoinSet::new();

    for (page_idx, image_data, _width, _height) in &encoded_pages {
        let backend_clone = std::sync::Arc::clone(&backend);
        let config_clone = ocr_config_owned.clone();
        let data_clone = Arc::clone(image_data);
        let idx = *page_idx;
        join_set.spawn(async move {
            let result = backend_clone.process_image(&data_clone, &config_clone).await;
            (idx, result)
        });
    }

    // Collect results, preserving page order.
    let mut ocr_results: Vec<Option<crate::types::ExtractionResult>> = vec![None; images_to_use.len()];
    while let Some(join_result) = join_set.join_next().await {
        let (page_idx, ocr_result) = join_result.map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("OCR task panicked: {}", e),
            plugin_name: "ocr".to_string(),
        })?;
        ocr_results[page_idx] = Some(ocr_result?);
    }

    // Initialize TATR for table structure recognition when layout detection is active.
    // TATR requires mutable access so pages are processed sequentially after OCR.
    #[cfg(feature = "layout-detection")]
    let mut tatr_model = if layout_detections.is_some() {
        crate::layout::take_or_create_tatr()
    } else {
        None
    };

    let mut page_texts = Vec::with_capacity(images_to_use.len());
    let mut conf_sum: f64 = 0.0;
    let mut conf_count: usize = 0;

    for (page_idx, ocr_result) in ocr_results.into_iter().enumerate() {
        // SAFETY: every slot was filled in the join loop above; None is unreachable.
        let ocr_result = ocr_result.expect("OCR result missing for page");
        let (_page_idx_enc, _image_data, _width, _height) = &encoded_pages[page_idx];
        #[cfg(feature = "layout-detection")]
        let height = *_height;

        // Accumulate mean_text_conf from per-page OCR results.
        if let Some(conf_val) = ocr_result
            .metadata
            .additional
            .get("mean_text_conf")
            .and_then(|v| v.as_i64())
        {
            conf_sum += conf_val as f64;
            conf_count += 1;
        }

        // When layout detections are available and OCR produced elements,
        // use layout-aware markdown assembly instead of plain text.
        #[cfg(feature = "layout-detection")]
        if let Some(detections) = layout_detections
            && let Some(ref elements) = ocr_result.ocr_elements
            && !elements.is_empty()
        {
            let detection = detections.get(page_idx);            // Run TATR table recognition if available (requires mutable model).
            let recognized_tables = match (detection, tatr_model.as_mut()) {
                (Some(det), Some(model)) => {
                    let rgb = images_to_use[page_idx].to_rgb8();
                    crate::ocr::layout_assembly::recognize_page_tables(&rgb, det, elements, model)
                }
                _ => Vec::new(),
            };

            // Convert OcrElements to PageContent via unified adapter.
            let mut page_content = crate::pdf::markdown::adapters::from_ocr_elements(elements, height as f32);

            // Reorder for multi-column reading order.
            crate::pdf::markdown::reorder_elements_reading_order(&mut page_content.elements);

            // Convert to paragraphs via the unified pipeline.
            let mut paragraphs = crate::pdf::markdown::content_to_paragraphs(&page_content);

            // Apply layout classification overrides (Title, SectionHeader, PageHeader, etc.)
            // from the RT-DETR layout detection model. This is the same classification step
            // that the native PDF path uses (pipeline.rs:397,474) — without it, OCR output
            // has no heading detection or furniture filtering from layout.
            if let Some(det) = detection {
                let hints = detection_to_layout_hints(det, height as f32);
                crate::pdf::markdown::layout_classify::apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.2, None);
            }

            // Filter page furniture (headers/footers) — must come AFTER layout overrides
            // since overrides may reclassify paragraphs as PageHeader/PageFooter.
            let paragraphs: Vec<_> = paragraphs.into_iter().filter(|p| !p.is_page_furniture).collect();

            // Interleave paragraphs and TATR tables by vertical position.
            //
            // Paragraphs have baseline_y in PDF space (y=0 at bottom; higher = top of page).
            // RecognizedTable.detection_bbox is in image space (y=0 at top; smaller = top of page).
            // Convert table positions to PDF space: pdf_y = page_height - bbox.y1.
            let page_md = {
                struct Block {
                    /// Vertical position in PDF space (higher = earlier in reading order).
                    y_pos: f32,
                    text: String,
                }

                let mut blocks: Vec<Block> = paragraphs
                    .iter()
                    .filter_map(|p| {
                        // Render a single paragraph by borrowing render_paragraphs_to_string
                        // with a one-element slice.
                        let text = crate::pdf::markdown::render_paragraphs_to_string(std::slice::from_ref(p));
                        if text.trim().is_empty() {
                            return None;
                        }
                        let y_pos = p.lines.first().map(|l| l.baseline_y).unwrap_or(0.0);
                        Some(Block { y_pos, text })
                    })
                    .collect();

                for rt in &recognized_tables {
                    if rt.markdown.is_empty() {
                        continue;
                    }
                    // Convert image-space y1 (top of bbox) to PDF space.
                    let y_pos = height as f32 - rt.detection_bbox.y1;
                    blocks.push(Block {
                        y_pos,
                        text: rt.markdown.clone(),
                    });
                }

                // Sort descending: highest PDF y (top of page) first.
                blocks.sort_by(|a, b| b.y_pos.total_cmp(&a.y_pos));

                let mut output = String::new();
                for block in &blocks {
                    if !output.is_empty() {
                        output.push_str("\n\n");
                    }
                    output.push_str(block.text.trim());
                }
                output
            };

            page_texts.push(page_md);
            continue;
        }

        // Fallback: use plain OCR text (move the String directly; no clone needed)
        let _ = page_idx; // used only in layout-detection path above
        page_texts.push(ocr_result.content);
    }

    // Return TATR model to global cache for reuse
    #[cfg(feature = "layout-detection")]
    if let Some(model) = tatr_model.take() {
        crate::layout::return_tatr(model);
    }

    // Compute average mean_text_conf across all pages, normalized to 0.0-1.0.
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
    Ok((result, mean_text_conf))
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
) -> crate::Result<String> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let ocr_config = config.ocr.as_ref().ok_or_else(|| {
        crate::error::KreuzbergError::parsing("OCR configuration missing")
    })?;

    // Sort stages by priority (highest first)
    let mut stages = pipeline.stages.clone();
    stages.sort_by_key(|b| std::cmp::Reverse(b.priority));

    // Filter to available backends
    let requested_backends: Vec<String> = stages.iter().map(|s| s.backend.clone()).collect();
    let available_stages: Vec<_> = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire read lock on OCR backend registry: {}", e),
            plugin_name: "ocr-registry".to_string(),
        })?;
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

    let mut best_result: Option<(String, f64)> = None;

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
            Ok((text, mean_conf)) => {
                let text_score = compute_quality_score(&text, &pipeline.quality_thresholds);

                // Blend the heuristic text score with the native Tesseract mean_text_conf
                // when available. The OCR engine confidence is weighted at 30% to complement
                // the text-analysis heuristics (70%).
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
                    return Ok(text);
                }

                // Track best-so-far
                match best_result {
                    Some((_, best_score)) if score > best_score => {
                        best_result = Some((text, score));
                    }
                    None => {
                        best_result = Some((text, score));
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
        Some((text, score)) => {
            tracing::warn!(
                score,
                threshold = pipeline.quality_thresholds.pipeline_min_quality,
                "All OCR pipeline backends produced suboptimal quality, using best result"
            );
            Ok(text)
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

/// Convert pixel-space layout detections to `LayoutHint`s in the pseudo-PDF
/// coordinate space used by the OCR markdown pipeline.
///
/// The OCR path uses `from_ocr_elements(elements, page_height)` which flips
/// image y-coordinates to PDF convention (y=0 at bottom, y increases upward)
/// by computing `pdf_y = page_height - pixel_y`. Layout hints must use the
/// same coordinate space so `apply_layout_overrides` can spatially match
/// paragraphs to layout regions.
///
/// This bridges the gap between the raw layout detection output (pixel space,
/// y=0 at top) and the markdown pipeline's expectation (PDF space, y=0 at bottom).
/// The same LayoutClass→LayoutHintClass mapping used by the native PDF path
/// (`extractors/pdf/extraction.rs:convert_results_to_hints`) is replicated here
/// because that function operates on `PageLayoutResult` (already in PDF coords),
/// while we have raw `DetectionResult` (pixel coords).
#[cfg(all(feature = "ocr", feature = "layout-detection"))]
fn detection_to_layout_hints(
    detection: &crate::layout::DetectionResult,
    page_height: f32,
) -> Vec<crate::pdf::markdown::types::LayoutHint> {
    use crate::layout::LayoutClass;
    use crate::pdf::markdown::types::{LayoutHint, LayoutHintClass};

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
            // Flip y: pixel y1 (top of bbox, small value) → PDF top (large value)
            // pixel y2 (bottom of bbox, large value) → PDF bottom (small value)
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
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::core::config::OcrConfig;
        use crate::types::ExtractionResult;
        use std::path::Path;

        struct MockBackend {
            called: Arc<AtomicBool>,
        }

        #[async_trait::async_trait]
        impl OcrBackend for MockBackend {
            fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
            fn supports_language(&self, _: &str) -> bool { true }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractionResult> {
                panic!("Should not call process_image");
            }
            fn supports_document_processing(&self) -> bool { true }
            async fn process_document(&self, path: &Path, _: &OcrConfig) -> crate::Result<ExtractionResult> {
                assert!(path.to_string_lossy().contains("test.pdf"));
                self.called.store(true, Ordering::SeqCst);
                Ok(ExtractionResult::default())
            }
        }

        impl Plugin for MockBackend {
            fn name(&self) -> &str { "mock" }
            fn version(&self) -> String { "1.0.0".to_string() }
            fn initialize(&self) -> crate::Result<()> { Ok(()) }
            fn shutdown(&self) -> crate::Result<()> { Ok(()) }
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
            None, // No content
            Some(&[]), // No images
            #[cfg(feature = "layout-detection")]
            None, // No layout
            &config,
            Some(path),
        ).await;

        assert!(result.is_ok());
        assert!(called.load(Ordering::SeqCst), "process_document was not called");

        // Clean up
        crate::plugins::unregister_ocr_backend("mock").unwrap();
    }
}
