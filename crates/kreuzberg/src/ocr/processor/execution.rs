//! OCR execution and result processing.
//!
//! This module handles the core OCR execution logic, including image processing,
//! text extraction, and result formatting.

use super::config::{apply_tesseract_variables, hash_config};
use super::validation::{
    resolve_all_installed_languages, resolve_tessdata_path, strip_control_characters, validate_language_and_traineddata,
};
use crate::core::config::ExtractionConfig;
use crate::image::normalize_image_dpi;
use crate::ocr::cache::OcrCache;
use crate::ocr::conversion::{TsvRow, iterator_word_to_element, tsv_row_to_element};
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::parse_hocr_to_internal_document;
#[cfg(feature = "pdf")]
use crate::ocr::table::post_process_table;
use crate::ocr::table::{extract_words_from_tsv, reconstruct_table, table_to_markdown};
use crate::ocr::types::{BatchItemResult, TesseractConfig};
use crate::types::internal::{ElementKind, InternalDocument};
use crate::types::{OcrExtractionResult, OcrTable, OcrTableBoundingBox};
use kreuzberg_tesseract::{TessPageSegMode, TessPolyBlockType, TesseractAPI};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cached Tesseract API state, stored per-thread to avoid reloading tessdata (~100MB) for each image.
struct CachedApi {
    tessdata_path: String,
    language: String,
    api: TesseractAPI,
}

thread_local! {
    /// Per-thread cache of an initialized `TesseractAPI`. Each thread owns at most one
    /// cached instance. Because Tesseract is not thread-safe, the thread-local pattern
    /// guarantees exclusive access without locking.
    static CACHED_TESSERACT_API: RefCell<Option<CachedApi>> = const { RefCell::new(None) };
}

/// Returns an initialized `TesseractAPI` for the given `(tessdata_path, language)` pair.
///
/// If the thread-local cache already holds a matching API, `clear()` is called to reset
/// image state and the cached instance is returned. Otherwise a fresh instance is created,
/// initialized, and stored in the cache for subsequent calls.
///
/// The caller receives ownership of the `TesseractAPI` via the take-and-put-back pattern:
/// the value is removed from the cache, used by the caller, and returned via
/// [`return_api_to_cache`] when done.
fn get_or_init_api(tessdata_path: &str, language: &str) -> Result<TesseractAPI, OcrError> {
    // Try to take the cached API out for reuse.
    let maybe_cached = CACHED_TESSERACT_API.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(cached) = slot.as_ref()
            && cached.tessdata_path == tessdata_path
            && cached.language == language
        {
            // Cache hit: take the API out so the caller owns it.
            return slot.take();
        }
        // Cache miss or key mismatch: drop the old entry.
        *slot = None;
        None
    });

    if let Some(cached) = maybe_cached {
        // Reset image/recognition state without reloading tessdata.
        cached
            .api
            .clear()
            .map_err(|e| OcrError::ProcessingFailed(format!("Failed to clear cached Tesseract API: {}", e)))?;
        return Ok(cached.api);
    }

    // No valid cache entry — create and initialize a fresh API.
    let api = TesseractAPI::new()
        .map_err(|e| OcrError::TesseractInitializationFailed(format!("Failed to allocate Tesseract engine: {}", e)))?;
    api.init(tessdata_path, language).map_err(|e| {
        OcrError::TesseractInitializationFailed(format!("Failed to initialize language '{}': {}", language, e))
    })?;
    Ok(api)
}

/// Returns a `TesseractAPI` back to the thread-local cache after use.
///
/// If the cache already holds a different entry (e.g. due to a re-entrant call), the
/// returned API is simply dropped.
fn return_api_to_cache(api: TesseractAPI, tessdata_path: String, language: String) {
    CACHED_TESSERACT_API.with(|cell| {
        let mut slot = cell.borrow_mut();
        // Only cache if the slot is currently empty (no re-entrant replacement).
        if slot.is_none() {
            *slot = Some(CachedApi {
                tessdata_path,
                language,
                api,
            });
        }
    });
}

use crate::types::OcrElement;

/// Rotate raw RGB image data by the given degrees (0, 90, 180, 270).
///
/// Returns the rotated pixel data and the new (width, height).
/// For 90° and 270° rotations, width and height are swapped.
fn rotate_rgb_image_data(data: &[u8], width: u32, height: u32, degrees: i32) -> (Vec<u8>, u32, u32) {
    let bpp = 3usize; // bytes per pixel (RGB)
    let w = width as usize;
    let h = height as usize;

    match degrees {
        0 => (data.to_vec(), width, height),
        90 => {
            // 90° clockwise: new dimensions are (height, width)
            let new_w = h;
            let new_h = w;
            let mut out = vec![0u8; new_w * new_h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    // (x, y) -> (h - 1 - y, x) in new coordinates
                    let dst_x = h - 1 - y;
                    let dst_y = x;
                    let dst_idx = (dst_y * new_w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, new_w as u32, new_h as u32)
        }
        180 => {
            // 180°: same dimensions, pixels reversed
            let mut out = vec![0u8; w * h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    let dst_x = w - 1 - x;
                    let dst_y = h - 1 - y;
                    let dst_idx = (dst_y * w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, width, height)
        }
        270 => {
            // 270° clockwise (= 90° counter-clockwise): new dimensions are (height, width)
            let new_w = h;
            let new_h = w;
            let mut out = vec![0u8; new_w * new_h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    // (x, y) -> (y, w - 1 - x) in new coordinates
                    let dst_x = y;
                    let dst_y = w - 1 - x;
                    let dst_idx = (dst_y * new_w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, new_w as u32, new_h as u32)
        }
        _ => {
            tracing::warn!("Unsupported rotation angle: {}°, skipping rotation", degrees);
            (data.to_vec(), width, height)
        }
    }
}

/// Parse Tesseract TSV output into structured OcrElements.
///
/// TSV format columns: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
///
/// # Arguments
///
/// * `tsv_data` - Raw TSV output from Tesseract
/// * `min_confidence` - Minimum confidence threshold (0-100 scale)
///
/// # Returns
///
/// Vector of OcrElements for word-level and line-level entries
fn parse_tsv_to_elements(tsv_data: &str, min_confidence: f64) -> Vec<OcrElement> {
    let mut elements = Vec::new();

    for line in tsv_data.lines().skip(1) {
        // Skip header row
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 12 {
            continue;
        }

        // Parse fields
        let level = fields[0].parse::<i32>().unwrap_or(0);
        let page_num = fields[1].parse::<i32>().unwrap_or(1);
        let block_num = fields[2].parse::<i32>().unwrap_or(0);
        let par_num = fields[3].parse::<i32>().unwrap_or(0);
        let line_num = fields[4].parse::<i32>().unwrap_or(0);
        let word_num = fields[5].parse::<i32>().unwrap_or(0);
        let left = fields[6].parse::<u32>().unwrap_or(0);
        let top = fields[7].parse::<u32>().unwrap_or(0);
        let width = fields[8].parse::<u32>().unwrap_or(0);
        let height = fields[9].parse::<u32>().unwrap_or(0);
        let conf = fields[10].parse::<f64>().unwrap_or(-1.0);
        let text = fields[11].to_string();

        // Skip low-confidence or empty entries
        // Tesseract uses -1 for non-text levels
        if conf < 0.0 || conf < min_confidence || text.trim().is_empty() {
            continue;
        }

        // Only include word-level (5) and line-level (4) entries
        // Tesseract TSV levels: 1=page, 2=block, 3=paragraph, 4=line, 5=word
        if level != 4 && level != 5 {
            continue;
        }

        let tsv_row = TsvRow {
            level,
            page_num,
            block_num,
            par_num,
            line_num,
            word_num,
            left,
            top,
            width,
            height,
            conf,
            text,
        };

        elements.push(tsv_row_to_element(&tsv_row));
    }

    elements
}

/// CI debug logging utility.
///
/// Logs debug messages when KREUZBERG_CI_DEBUG environment variable is set.
fn log_ci_debug<F>(enabled: bool, stage: &str, details: F)
where
    F: FnOnce() -> String,
{
    if !enabled {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    tracing::debug!(stage, timestamp = format!("{timestamp:.3}"), "{}", details());
}

/// Build content with OCR tables inlined at their correct vertical positions.
///
/// Parses TSV word positions to separate table words from non-table words,
/// groups non-table words into lines and paragraphs, then interleaves
/// paragraphs and table markdown sorted by Y-position.
fn build_content_with_inline_tables(tsv_data: &str, tables: &[OcrTable], min_confidence: f64) -> String {
    let words = match extract_words_from_tsv(tsv_data, min_confidence) {
        Ok(w) => w,
        Err(_) => return String::new(),
    };

    if words.is_empty() {
        return String::new();
    }

    // Collect table bounding boxes
    let table_bboxes: Vec<_> = tables.iter().filter_map(|t| t.bounding_box.as_ref()).collect();

    // Classify words as inside a table bbox or not
    let mut non_table_words = Vec::new();
    for word in &words {
        let in_table = table_bboxes.iter().any(|bbox| {
            let word_cx = word.left + word.width / 2;
            let word_cy = word.top + word.height / 2;
            word_cx >= bbox.left && word_cx <= bbox.right && word_cy >= bbox.top && word_cy <= bbox.bottom
        });
        if !in_table {
            non_table_words.push(word);
        }
    }

    // Group non-table words into lines by Y-proximity.
    // Words on the same line have similar top values (within half the average word height).
    if non_table_words.is_empty() && tables.is_empty() {
        return String::new();
    }

    // Sort non-table words by (top, left)
    let mut sorted_words = non_table_words;
    sorted_words.sort_by(|a, b| a.top.cmp(&b.top).then(a.left.cmp(&b.left)));

    // Group into lines: words within line_threshold pixels vertically are on the same line
    let avg_height = if sorted_words.is_empty() {
        20
    } else {
        let total_h: u32 = sorted_words.iter().map(|w| w.height).sum();
        (total_h / sorted_words.len() as u32).max(1)
    };
    let line_threshold = avg_height / 2;

    struct TextLine {
        y_center: u32,
        text: String,
    }

    let mut lines: Vec<TextLine> = Vec::new();
    for word in &sorted_words {
        let word_y = word.top + word.height / 2;
        if let Some(last_line) = lines.last_mut()
            && word_y.abs_diff(last_line.y_center) <= line_threshold
        {
            last_line.text.push(' ');
            last_line.text.push_str(&word.text);
            continue;
        }
        lines.push(TextLine {
            y_center: word_y,
            text: word.text.clone(),
        });
    }

    // Group lines into paragraphs: large Y-gap between consecutive lines = paragraph break
    let paragraph_gap = avg_height * 2;

    struct Paragraph {
        y_start: u32,
        text: String,
    }

    let mut paragraphs: Vec<Paragraph> = Vec::new();
    for line in &lines {
        if let Some(last_para) = paragraphs.last_mut() {
            let last_y = last_para.y_start;
            // Check gap - we use the line's y_center vs the last line added
            if line.y_center.saturating_sub(last_y) <= paragraph_gap {
                last_para.text.push('\n');
                last_para.text.push_str(&line.text);
                last_para.y_start = line.y_center; // track last line position
                continue;
            }
        }
        paragraphs.push(Paragraph {
            y_start: line.y_center,
            text: line.text.clone(),
        });
    }

    // Build sorted elements: paragraphs + tables, ordered by Y-position
    enum ContentElement<'a> {
        Paragraph { y: u32, text: String },
        Table { y: u32, markdown: &'a str },
    }

    let mut elements: Vec<ContentElement> = Vec::new();

    // Re-derive paragraph y_start from the first line in each paragraph
    // We need the first y_center of the paragraph, which we track by rebuilding
    {
        let mut para_idx = 0;
        let mut line_idx = 0;
        for para in &paragraphs {
            let line_count = para.text.matches('\n').count() + 1;
            let first_y = if line_idx < lines.len() {
                lines[line_idx].y_center
            } else {
                para.y_start
            };
            elements.push(ContentElement::Paragraph {
                y: first_y,
                text: para.text.clone(),
            });
            line_idx += line_count;
            para_idx += 1;
        }
        let _ = para_idx; // suppress unused warning
    }

    for table in tables {
        if let Some(ref bbox) = table.bounding_box {
            elements.push(ContentElement::Table {
                y: bbox.top,
                markdown: &table.markdown,
            });
        } else {
            // Tables without bbox go at the end
            elements.push(ContentElement::Table {
                y: u32::MAX,
                markdown: &table.markdown,
            });
        }
    }

    // Sort by Y-position (ascending = top of image first)
    elements.sort_by_key(|e| match e {
        ContentElement::Paragraph { y, .. } => *y,
        ContentElement::Table { y, .. } => *y,
    });

    // Join with double newlines
    let mut output = String::new();
    for elem in &elements {
        let text = match elem {
            ContentElement::Paragraph { text, .. } => text.as_str(),
            ContentElement::Table { markdown, .. } => markdown,
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(trimmed);
    }

    output
}

/// Minimum confidence for accepting orientation detection results.
const MIN_ORIENTATION_CONFIDENCE: f32 = 0.5;

/// Check whether a center point (x, y) lies within a bounding box.
fn point_in_bbox(x: i32, y: i32, left: i32, top: i32, right: i32, bottom: i32) -> bool {
    x >= left && x <= right && y >= top && y <= bottom
}

/// Extract OcrElements via Tesseract's iterator APIs with rich metadata.
///
/// Uses ResultIterator for word-level text, bounding boxes, confidence, and font
/// attributes, plus PageIterator for block type and paragraph info. This replaces
/// TSV-based extraction with significantly richer metadata.
fn extract_elements_via_iterator(
    api: &TesseractAPI,
    page_number: usize,
    min_confidence: f64,
) -> Result<Vec<OcrElement>, OcrError> {
    // Obtain page iterator for block/paragraph spatial data.
    let page_iter = match api.get_page_iterator() {
        Ok(iter) => iter,
        Err(_) => return Ok(Vec::new()),
    };

    let blocks = match page_iter.extract_all_blocks() {
        Ok(b) => b,
        Err(_) => return Ok(Vec::new()),
    };

    let paragraphs = match page_iter.extract_all_paragraphs() {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };

    // Obtain result iterator for word-level data (text, confidence, font attrs).
    let result_iter = match api.get_iterator() {
        Ok(iter) => iter,
        Err(_) => return Ok(Vec::new()),
    };

    let words = match result_iter.extract_all_words() {
        Ok(w) => w,
        Err(_) => return Ok(Vec::new()),
    };

    // Block types that should be filtered out (non-text / noise regions).
    let skip_block_types = [
        TessPolyBlockType::PT_NOISE,
        TessPolyBlockType::PT_FLOWING_IMAGE,
        TessPolyBlockType::PT_HEADING_IMAGE,
        TessPolyBlockType::PT_PULLOUT_IMAGE,
        TessPolyBlockType::PT_HORZ_LINE,
        TessPolyBlockType::PT_VERT_LINE,
    ];

    let mut elements = Vec::new();

    for word in &words {
        // Skip low-confidence words.
        if (word.confidence as f64) < min_confidence {
            continue;
        }

        // Skip empty/whitespace-only text.
        if word.text.trim().is_empty() {
            continue;
        }

        // Compute word center for spatial containment checks.
        let cx = (word.left + word.right) / 2;
        let cy = (word.top + word.bottom) / 2;

        // Find parent block and determine its type.
        let parent_block = blocks
            .iter()
            .find(|b| point_in_bbox(cx, cy, b.left, b.top, b.right, b.bottom));

        let block_type = parent_block.map(|b| b.block_type);

        // Skip words in non-text block types.
        if let Some(bt) = block_type
            && skip_block_types.contains(&bt)
        {
            continue;
        }

        // Find parent paragraph for justification/list metadata.
        let para_info = paragraphs
            .iter()
            .find(|p| point_in_bbox(cx, cy, p.left, p.top, p.right, p.bottom));

        let element = iterator_word_to_element(word, block_type, para_info, page_number);
        elements.push(element);
    }

    Ok(elements)
}

/// Perform OCR on an image using Tesseract.
///
/// This function handles the complete OCR pipeline:
/// 1. Image loading and preprocessing
/// 2. Tesseract initialization and configuration
/// 3. Text recognition
/// 4. Output formatting (text, markdown, hOCR, or TSV)
/// 5. Optional table detection
///
/// # Arguments
///
/// * `image_bytes` - Raw image data
/// * `config` - OCR configuration
/// * `extraction_config` - Optional extraction config for output format (markdown vs djot)
///
/// # Returns
///
/// OCR extraction result containing text and optional tables
pub(super) fn perform_ocr(
    image_bytes: &[u8],
    config: &TesseractConfig,
    extraction_config: Option<&ExtractionConfig>,
) -> Result<OcrExtractionResult, OcrError> {
    let ci_debug_enabled = env::var_os("KREUZBERG_CI_DEBUG").is_some();
    log_ci_debug(ci_debug_enabled, "perform_ocr:start", || {
        format!(
            "bytes={} language={} output={} use_cache={}",
            image_bytes.len(),
            config.language,
            config.output_format,
            config.use_cache
        )
    });

    let img = crate::extraction::image::load_image_for_ocr(image_bytes)
        .map_err(|e| OcrError::ImageProcessingFailed(e.to_string()))?;

    let rgb_image = img.to_rgb8();
    let (orig_width, orig_height) = rgb_image.dimensions();

    log_ci_debug(ci_debug_enabled, "image", || {
        format!("dimensions={}x{} color_type=RGB8", orig_width, orig_height)
    });

    // Normalize image DPI: resize to target DPI and calculate the actual DPI
    // of the image that will be passed to tesseract.
    let dpi_config = config
        .preprocessing
        .as_ref()
        .map(|p| crate::types::ExtractionConfig {
            target_dpi: p.target_dpi,
            ..Default::default()
        })
        .unwrap_or_default();

    let (image_data, width, height, source_dpi) = match normalize_image_dpi(
        rgb_image.as_raw(),
        orig_width as usize,
        orig_height as usize,
        &dpi_config,
        None,
    ) {
        Ok(result) => {
            let w = result.dimensions.0 as u32;
            let h = result.dimensions.1 as u32;
            let final_dpi = result.metadata.final_dpi;

            log_ci_debug(ci_debug_enabled, "dpi_normalization", || {
                format!(
                    "original={}x{} normalized={}x{} target_dpi={} final_dpi={} resized={}",
                    orig_width,
                    orig_height,
                    w,
                    h,
                    result.metadata.target_dpi,
                    final_dpi,
                    !result.metadata.skipped_resize
                )
            });

            (result.rgb_data, w, h, final_dpi)
        }
        Err(e) => {
            // If normalization fails, fall back to the original image with default 300 DPI
            tracing::warn!("DPI normalization failed, using original image: {}", e);
            let w = orig_width;
            let h = orig_height;
            (rgb_image.into_raw(), w, h, 300)
        }
    };

    let bytes_per_pixel: u32 = 3;
    let bytes_per_line = width * bytes_per_pixel;

    let tessdata_path = resolve_tessdata_path();

    log_ci_debug(ci_debug_enabled, "tessdata", || {
        let path_preview = env::var_os("PATH").map(|paths| {
            env::split_paths(&paths)
                .take(6)
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        });
        let resolved_exists = !tessdata_path.is_empty() && std::path::Path::new(&tessdata_path).exists();

        format!(
            "env={:?} resolved={} exists={} path_preview={:?}",
            env::var("TESSDATA_PREFIX").ok(),
            if tessdata_path.is_empty() {
                "unset"
            } else {
                &tessdata_path
            },
            resolved_exists,
            path_preview
        )
    });

    log_ci_debug(ci_debug_enabled, "tesseract_version", || {
        format!("version={}", TesseractAPI::version())
    });

    // Validate language and traineddata files
    validate_language_and_traineddata(&config.language, &tessdata_path)?;

    // Obtain an initialized TesseractAPI from the per-thread cache. If the cache holds a
    // matching (tessdata_path, language) entry, `clear()` is called to reset image state
    // without reloading tessdata (~100MB). Otherwise a fresh API is created and initialized.
    //
    // The API is wrapped in a guard that returns it to the cache on drop (both on success
    // and on any error path), preventing the cached state from being wasted on failures.
    let api = get_or_init_api(&tessdata_path, &config.language)?;
    struct ApiGuard {
        api: Option<TesseractAPI>,
        tessdata_path: String,
        language: String,
    }
    impl Drop for ApiGuard {
        fn drop(&mut self) {
            if let Some(api) = self.api.take() {
                return_api_to_cache(api, self.tessdata_path.clone(), self.language.clone());
            }
        }
    }
    impl std::ops::Deref for ApiGuard {
        type Target = TesseractAPI;
        fn deref(&self) -> &TesseractAPI {
            self.api.as_ref().expect("ApiGuard consumed prematurely")
        }
    }
    let api = ApiGuard {
        api: Some(api),
        tessdata_path: tessdata_path.clone(),
        language: config.language.clone(),
    };

    log_ci_debug(ci_debug_enabled, "init", || {
        format!("language={} datapath='{}'", config.language, tessdata_path)
    });

    if ci_debug_enabled {
        match api.get_available_languages() {
            Ok(languages) => {
                log_ci_debug(ci_debug_enabled, "available_languages", move || {
                    let preview = languages.iter().take(10).cloned().collect::<Vec<_>>();
                    format!("count={} preview={:?}", languages.len(), preview)
                });
            }
            Err(err) => {
                log_ci_debug(ci_debug_enabled, "available_languages_error", move || {
                    format!("error={:?}", err)
                });
            }
        }
    }

    let psm_mode = TessPageSegMode::from_int(config.psm as i32);
    let psm_result = api.set_page_seg_mode(psm_mode);
    log_ci_debug(ci_debug_enabled, "set_psm", || match &psm_result {
        Ok(_) => format!("mode={}", config.psm),
        Err(err) => format!("error={:?}", err),
    });
    psm_result.map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set PSM mode: {}", e)))?;

    apply_tesseract_variables(&api, config)?;

    // Attempt Leptonica preprocessing for improved OCR quality.
    // This normalizes background, sharpens text, and converts to grayscale.
    // Falls back to raw RGB if Leptonica processing fails.
    //
    // IMPORTANT: `pix_guard` must remain alive until AFTER `api.recognize()` completes
    // because `TessBaseAPISetImage2` borrows the Pix pointer without taking ownership.
    // Dropping the Pix while Tesseract holds the pointer would cause a use-after-free.
    //
    // DROP ORDER NOTE: `pix_guard` is declared AFTER `api` (the `ApiGuard`), so Rust
    // drops it FIRST when the function returns (Rust drops locals in reverse declaration
    // order). This means the Pix is freed *before* the ApiGuard fires, which in turn
    // calls `return_api_to_cache`. The API therefore never enters the cache while still
    // holding a dangling Pix pointer — the Pix is already freed at that point, and
    // Tesseract's internal reference to it has been cleared by `api.clear()` (called at
    // the top of `get_or_init_api` on the next reuse). No dangling pointer issue.
    let mut pix_guard: Option<kreuzberg_tesseract::Pix> = {
        match kreuzberg_tesseract::Pix::from_raw_rgb(&image_data, width, height) {
            Ok(pix) => {
                // Pipeline: background normalization → unsharp mask → grayscale.
                // No binarization — Tesseract performs its own internal thresholding.
                let processed = pix
                    .background_normalize()
                    .and_then(|p| p.unsharp_mask(3, 0.5))
                    .and_then(|p| p.to_grayscale());
                match processed {
                    Ok(p) => Some(p),
                    Err(e) => {
                        tracing::debug!("Leptonica preprocessing failed, using raw image: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Leptonica Pix creation failed, using raw image: {}", e);
                None
            }
        }
    };

    if let Some(ref pix) = pix_guard {
        api.set_image_2(pix.as_ptr())
            .map_err(|e| OcrError::ProcessingFailed(format!("Failed to set preprocessed image: {}", e)))?;
    } else {
        api.set_image(
            &image_data,
            width as i32,
            height as i32,
            bytes_per_pixel as i32,
            bytes_per_line as i32,
        )
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to set image: {}", e)))?;
    }

    // Tell tesseract the source resolution based on our DPI normalization calculation.
    // Clamp to minimum 70 DPI — tesseract's own fallback for invalid resolution.
    // Raw images without DPI metadata may report 0, which crashes DetectOrientationScript.
    let source_dpi = source_dpi.max(70);
    api.set_source_resolution(source_dpi)
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to set source resolution: {}", e)))?;

    log_ci_debug(ci_debug_enabled, "set_image", || {
        format!(
            "width={} height={} bytes_per_pixel={} bytes_per_line={} source_dpi={}",
            width, height, bytes_per_pixel, bytes_per_line, source_dpi
        )
    });

    // Orientation detection and auto-rotation.
    // DetectOrientationScript requires PSM_OSD_ONLY (0) or PSM_AUTO_OSD (1).
    // We temporarily switch PSM, detect orientation, rotate if needed, then restore.
    let mut detected_orientation: Option<(i32, f32, String, f32)> = None;
    let auto_rotate_enabled =
        config.preprocessing.as_ref().map(|p| p.auto_rotate).unwrap_or(false) || config.auto_rotate;

    if auto_rotate_enabled {
        // Save current PSM and switch to OSD_ONLY for orientation detection
        let original_psm = psm_mode;
        api.set_page_seg_mode(TessPageSegMode::PSM_OSD_ONLY)
            .map_err(|e| OcrError::ProcessingFailed(format!("Failed to set PSM for OSD: {}", e)))?;

        match api.detect_os() {
            Ok((orient_deg, orient_conf, script_name, script_conf)) => {
                log_ci_debug(ci_debug_enabled, "orientation_detection", || {
                    format!(
                        "orientation={}° confidence={:.2} script={} script_confidence={:.2}",
                        orient_deg, orient_conf, script_name, script_conf
                    )
                });
                detected_orientation = Some((orient_deg, orient_conf, script_name, script_conf));

                // Restore original PSM
                api.set_page_seg_mode(original_psm)
                    .map_err(|e| OcrError::ProcessingFailed(format!("Failed to restore PSM: {}", e)))?;

                // If orientation is non-zero with sufficient confidence, rotate the image
                if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
                    tracing::info!(
                        "Auto-rotating image by {} degrees (confidence: {:.2})",
                        orient_deg,
                        orient_conf
                    );

                    let (rotated_data, new_width, new_height) =
                        rotate_rgb_image_data(&image_data, width, height, orient_deg);
                    let new_bytes_per_line = new_width * bytes_per_pixel;

                    // Re-apply Leptonica preprocessing to the rotated image so the same
                    // background normalisation / unsharp mask / grayscale pipeline is used.
                    // If preprocessing succeeds we set the image via set_image_2; otherwise
                    // we fall back to the raw rotated RGB bytes via set_image.
                    //
                    // Build a new preprocessed Pix from the rotated image data.
                    // If preprocessing succeeds the result is assigned to `pix_guard` below,
                    // replacing the un-rotated Pix so Tesseract keeps a valid image pointer.
                    let rotated_pix = kreuzberg_tesseract::Pix::from_raw_rgb(&rotated_data, new_width, new_height)
                        .ok()
                        .and_then(|pix| {
                            pix.background_normalize()
                                .and_then(|p| p.unsharp_mask(3, 0.5))
                                .and_then(|p| p.to_grayscale())
                                .ok()
                        });

                    if let Some(ref pix) = rotated_pix {
                        api.set_image_2(pix.as_ptr()).map_err(|e| {
                            OcrError::ProcessingFailed(format!("Failed to set rotated preprocessed image: {}", e))
                        })?;
                    } else {
                        // Leptonica preprocessing failed for the rotated image — use raw bytes.
                        api.set_image(
                            &rotated_data,
                            new_width as i32,
                            new_height as i32,
                            bytes_per_pixel as i32,
                            new_bytes_per_line as i32,
                        )
                        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to set rotated image: {}", e)))?;
                    }

                    // Replace the outer `pix_guard` with the rotated Pix so that it
                    // stays alive until after `api.recognize()` completes. The previous
                    // un-rotated Pix is dropped here when the old value is overwritten.
                    pix_guard = rotated_pix;

                    api.set_source_resolution(source_dpi).map_err(|e| {
                        OcrError::ProcessingFailed(format!("Failed to set source resolution after rotation: {}", e))
                    })?;

                    log_ci_debug(ci_debug_enabled, "auto_rotate", || {
                        format!("rotated={}° new_dimensions={}x{}", orient_deg, new_width, new_height)
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Orientation detection failed, proceeding without rotation: {}", e);
                // Restore original PSM
                let _ = api.set_page_seg_mode(original_psm);
            }
        }
    }

    api.recognize()
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to recognize text: {}", e)))?;

    // Capture mean text confidence (0-100) immediately after recognition.
    let mean_text_conf = api.mean_text_conf().unwrap_or(-1);

    log_ci_debug(ci_debug_enabled, "recognize", || {
        format!("completed mean_text_conf={}", mean_text_conf)
    });

    // Collect word-level confidence statistics from Tesseract for quality heuristics.
    let word_confidence_stats = match api.all_word_confidences() {
        Ok(confidences) if !confidences.is_empty() => {
            let word_count = confidences.len();
            let low_conf_word_count = confidences.iter().filter(|&&c| c < 50).count();

            // Compute median
            let mut sorted = confidences.clone();
            sorted.sort_unstable();
            let median_word_conf = if word_count % 2 == 0 {
                (sorted[word_count / 2 - 1] + sorted[word_count / 2]) / 2
            } else {
                sorted[word_count / 2]
            };

            // Compute 10th percentile (index = floor(0.1 * (n - 1)))
            let p10_idx = ((word_count as f64 - 1.0) * 0.1).floor() as usize;
            let p10_word_conf = sorted[p10_idx.min(word_count - 1)];

            Some((median_word_conf, p10_word_conf, word_count, low_conf_word_count))
        }
        Ok(_) => {
            // Empty confidences array — fall back to mean_text_conf
            match api.mean_text_conf() {
                Ok(mean_conf) => Some((mean_conf, mean_conf, 0usize, 0usize)),
                Err(_) => None,
            }
        }
        Err(_) => {
            // all_word_confidences failed — fall back to mean_text_conf
            match api.mean_text_conf() {
                Ok(mean_conf) => Some((mean_conf, mean_conf, 0usize, 0usize)),
                Err(_) => None,
            }
        }
    };

    let tsv_data_for_tables = if config.enable_table_detection || config.output_format == "tsv" {
        Some(
            api.get_tsv_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract TSV: {}", e)))?,
        )
    } else {
        None
    };

    let mut hocr_document: Option<InternalDocument> = None;

    let (raw_content, mime_type) = match config.output_format.as_str() {
        "text" => {
            let text = api
                .get_utf8_text()
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract text: {}", e)))?;
            (text, "text/plain".to_string())
        }
        "markdown" => {
            let hocr = api
                .get_hocr_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;

            // Parse hOCR into structured InternalDocument and flatten to text.
            // The InternalDocument is preserved for downstream layout classification.
            let internal_doc = parse_hocr_to_internal_document(&hocr);
            let content = internal_doc
                .elements
                .iter()
                .filter_map(|e| match e.kind {
                    // Skip PageBreak: each page is OCR'd independently, so any
                    // PageBreak elements within a single-page hOCR doc are
                    // artefacts and should not produce thematic breaks.
                    ElementKind::PageBreak => None,
                    _ if !e.text.is_empty() => Some(e.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            hocr_document = Some(internal_doc);

            // Set mime_type based on actual output format
            let mime_type = extraction_config
                .map(|c| match c.output_format {
                    crate::core::config::OutputFormat::Djot => "text/djot",
                    _ => "text/markdown",
                })
                .unwrap_or("text/markdown");

            (content, mime_type.to_string())
        }
        "hocr" => {
            let hocr = api
                .get_hocr_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;
            (hocr, "text/html".to_string())
        }
        "tsv" => {
            let tsv = tsv_data_for_tables
                .as_ref()
                .ok_or_else(|| OcrError::ProcessingFailed("TSV data not available".to_string()))?
                .clone();
            (tsv, "text/plain".to_string())
        }
        _ => {
            return Err(OcrError::InvalidConfiguration(format!(
                "Unsupported output format: {}",
                config.output_format
            )));
        }
    };

    let mut metadata = HashMap::new();
    metadata.insert(
        "language".to_string(),
        serde_json::Value::String(config.language.clone()),
    );
    metadata.insert("psm".to_string(), serde_json::Value::String(config.psm.to_string()));
    metadata.insert(
        "output_format".to_string(),
        serde_json::Value::String(config.output_format.clone()),
    );
    metadata.insert("table_count".to_string(), serde_json::Value::String("0".to_string()));
    metadata.insert(
        "tables_detected".to_string(),
        serde_json::Value::String("0".to_string()),
    );
    if config.output_format == "markdown" {
        metadata.insert(
            "source_format".to_string(),
            serde_json::Value::String("hocr".to_string()),
        );
    }

    // Store mean text confidence (0-100) for quality scoring.
    if mean_text_conf >= 0 {
        metadata.insert(
            "mean_text_conf".to_string(),
            serde_json::Value::Number(serde_json::Number::from(mean_text_conf)),
        );
    }

    // Store word confidence statistics for quality heuristics.
    if let Some((median_conf, p10_conf, word_count, low_conf_count)) = word_confidence_stats {
        metadata.insert(
            "median_word_conf".to_string(),
            serde_json::Value::Number(serde_json::Number::from(median_conf)),
        );
        metadata.insert(
            "p10_word_conf".to_string(),
            serde_json::Value::Number(serde_json::Number::from(p10_conf)),
        );
        metadata.insert(
            "word_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(word_count)),
        );
        metadata.insert(
            "low_conf_word_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(low_conf_count)),
        );
    }

    // Store orientation detection results in metadata.
    if let Some((orient_deg, orient_conf, ref script_name, script_conf)) = detected_orientation {
        metadata.insert(
            "orientation_degrees".to_string(),
            serde_json::Value::Number(serde_json::Number::from(orient_deg)),
        );
        metadata.insert(
            "orientation_confidence".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(orient_conf as f64).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        metadata.insert(
            "script_name".to_string(),
            serde_json::Value::String(script_name.clone()),
        );
        metadata.insert(
            "script_confidence".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(script_conf as f64).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
            metadata.insert("auto_rotated".to_string(), serde_json::Value::Bool(true));
        }
    }

    let mut tables = Vec::new();
    let mut ocr_elements = None;

    if config.enable_table_detection {
        let tsv_data = tsv_data_for_tables.as_ref().unwrap();

        let words = extract_words_from_tsv(tsv_data, config.table_min_confidence)?;

        if words.len() >= 6 {
            let table = reconstruct_table(&words, config.table_column_threshold, config.table_row_threshold_ratio);
            if !table.is_empty() && !table[0].is_empty() {
                // Apply full post-processing validation to reject false positives.
                // post_process_table is only available with the pdf feature; without it, use table as-is.
                #[cfg(feature = "pdf")]
                let cleaned = post_process_table(table, false, false);
                #[cfg(not(feature = "pdf"))]
                let cleaned = Some(table);
                if let Some(cleaned) = cleaned {
                    metadata.insert("table_count".to_string(), serde_json::Value::String("1".to_string()));
                    metadata.insert(
                        "tables_detected".to_string(),
                        serde_json::Value::String("1".to_string()),
                    );
                    metadata.insert(
                        "table_rows".to_string(),
                        serde_json::Value::String(cleaned.len().to_string()),
                    );
                    metadata.insert(
                        "table_cols".to_string(),
                        serde_json::Value::String(cleaned[0].len().to_string()),
                    );

                    let markdown_table = table_to_markdown(&cleaned);

                    // Compute bounding box from the TSV words used for table reconstruction
                    let bbox = if !words.is_empty() {
                        let left = words.iter().map(|w| w.left).min().unwrap_or(0);
                        let top = words.iter().map(|w| w.top).min().unwrap_or(0);
                        let right = words.iter().map(|w| w.left + w.width).max().unwrap_or(0);
                        let bottom = words.iter().map(|w| w.top + w.height).max().unwrap_or(0);
                        Some(OcrTableBoundingBox {
                            left,
                            top,
                            right,
                            bottom,
                        })
                    } else {
                        None
                    };

                    tables.push(OcrTable {
                        cells: cleaned,
                        markdown: markdown_table,
                        page_number: 1,
                        bounding_box: bbox,
                    });
                }
            }
        }
    }

    // Extract structured OcrElements via Tesseract iterators (rich metadata:
    // font attributes, block types, paragraph info).
    let iterator_elements = extract_elements_via_iterator(&api, 1, config.min_confidence);
    match iterator_elements {
        Ok(elements) if !elements.is_empty() => {
            ocr_elements = Some(elements);
        }
        _ => {
            // Fallback: parse TSV if iterator extraction fails
            if let Some(ref tsv_data) = tsv_data_for_tables {
                let elements = parse_tsv_to_elements(tsv_data, config.min_confidence);
                if !elements.is_empty() {
                    ocr_elements = Some(elements);
                }
            }
        }
    }

    let mut content = strip_control_characters(&raw_content).into_owned();

    // When tables were detected and output is markdown, rebuild content with tables inlined
    // at their correct vertical positions. This mirrors the PDF path's assemble_markdown_with_tables.
    let is_markdown_output = extraction_config
        .map(|c| c.output_format == crate::core::config::OutputFormat::Markdown)
        .unwrap_or(config.output_format == "markdown");

    if !tables.is_empty()
        && is_markdown_output
        && let Some(ref tsv_data) = tsv_data_for_tables
    {
        let rebuilt = build_content_with_inline_tables(tsv_data, &tables, config.table_min_confidence);
        if !rebuilt.is_empty() {
            content = rebuilt;
            // Signal that content is already formatted as markdown so
            // apply_output_format() in the pipeline skips re-conversion.
            metadata.insert(
                "pre_formatted".to_string(),
                serde_json::Value::String("markdown".to_string()),
            );
        }
    }

    // SAFETY: Drop the Pix before returning the API to the cache. Although the current
    // declaration order already guarantees this (pix_guard is declared after api, so Rust
    // drops it first), the explicit drop makes the intent clear and guards against future
    // refactoring that reorders locals. Freeing the Pix here ensures Tesseract's internal
    // image reference is severed before the API re-enters the cache; the next caller's
    // get_or_init_api invokes clear() to fully reset state.
    drop(pix_guard);

    // The ApiGuard's Drop impl returns the API to the thread-local cache automatically
    // on all exit paths (success and error). Drop explicitly here for clarity.
    drop(api);

    Ok(OcrExtractionResult {
        content,
        mime_type,
        metadata,
        tables,
        ocr_elements,
        internal_document: hocr_document,
    })
}

/// Process an image file and return OCR results.
///
/// # Arguments
///
/// * `file_path` - Path to image file
/// * `config` - OCR configuration
/// * `cache` - Cache instance
/// * `output_format` - Optional output format (Plain, Markdown, Djot) for proper mime_type handling
///
/// # Returns
///
/// OCR extraction result
pub(super) fn process_image_file_with_cache(
    file_path: &str,
    config: &TesseractConfig,
    cache: &OcrCache,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    let image_bytes = std::fs::read(file_path)
        .map_err(|e| OcrError::IOError(format!("Failed to read file '{}': {}", file_path, e)))?;
    process_image_with_cache(&image_bytes, config, cache, output_format)
}

/// Check if a language value is the "all" wildcard (case-insensitive).
fn is_all_languages(lang: &str) -> bool {
    let lower = lang.to_ascii_lowercase();
    lower == "all" || lower == "*"
}

/// Resolve the "all"/"*" wildcard in a config's language field.
///
/// If the language is a wildcard, scans the tessdata directory for installed
/// languages and returns a new config with the resolved language string.
/// Otherwise returns `None`, indicating the original config should be used as-is.
fn resolve_config_language(config: &TesseractConfig) -> Result<Option<TesseractConfig>, OcrError> {
    if is_all_languages(&config.language) {
        let tessdata_path = resolve_tessdata_path();
        let resolved = resolve_all_installed_languages(&tessdata_path)?;
        let mut resolved_config = config.clone();
        resolved_config.language = resolved;
        Ok(Some(resolved_config))
    } else {
        Ok(None)
    }
}

/// Process an image and return OCR results, using cache if enabled.
///
/// Resolves the `"all"` / `"*"` language wildcard, then delegates to
/// [`process_image_resolved`] for caching and OCR execution.
///
/// # Arguments
///
/// * `image_bytes` - Raw image data
/// * `config` - OCR configuration
/// * `cache` - Cache instance
/// * `output_format` - Optional output format (Plain, Markdown, Djot) for proper mime_type handling
///
/// # Returns
///
/// OCR extraction result
pub(super) fn process_image_with_cache(
    image_bytes: &[u8],
    config: &TesseractConfig,
    cache: &OcrCache,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    config.validate().map_err(OcrError::InvalidConfiguration)?;

    // Resolve "all" / "*" before hashing so cache keys reflect actual languages.
    // If not a wildcard, resolved is None and we use the original config (no clone).
    let resolved = resolve_config_language(config)?;
    let config = resolved.as_ref().unwrap_or(config);

    process_image_resolved(image_bytes, config, cache, output_format)
}

/// Inner implementation operating on an already-resolved config.
///
/// Handles cache lookup, OCR execution, and cache storage. Callers are
/// responsible for validating and resolving wildcards in the config before
/// calling this function.
fn process_image_resolved(
    image_bytes: &[u8],
    config: &TesseractConfig,
    cache: &OcrCache,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    let image_hash = crate::cache::blake3_hash_bytes(image_bytes);

    let config_str = hash_config(config);

    if config.use_cache
        && let Some(cached_result) = cache.get_cached_result(&image_hash, "tesseract", &config_str)?
    {
        #[cfg(feature = "otel")]
        tracing::Span::current().record("cache.hit", true);
        return Ok(cached_result);
    }

    #[cfg(feature = "otel")]
    tracing::Span::current().record("cache.hit", false);

    // Create minimal ExtractionConfig with just the output format if provided
    let extraction_config = output_format.map(|fmt| ExtractionConfig {
        output_format: fmt,
        ..Default::default()
    });

    let result = perform_ocr(image_bytes, config, extraction_config.as_ref())?;

    if config.use_cache {
        let _ = cache.set_cached_result(&image_hash, "tesseract", &config_str, &result);
    }

    Ok(result)
}

/// Process multiple image files in parallel using Rayon.
///
/// Validates and resolves the language wildcard once, then processes all files
/// in parallel using [`process_image_resolved`] directly (skipping redundant
/// per-image resolution).
///
/// Results are returned in the same order as the input file paths.
pub(super) fn process_image_files_batch(
    file_paths: Vec<String>,
    config: &TesseractConfig,
    cache: &OcrCache,
) -> Vec<BatchItemResult> {
    #[cfg(not(target_arch = "wasm32"))]
    use rayon::prelude::*;

    // Validate once for the entire batch.
    if let Err(e) = config.validate().map_err(OcrError::InvalidConfiguration) {
        return file_paths
            .into_iter()
            .map(|path| BatchItemResult {
                file_path: path,
                success: false,
                result: None,
                error: Some(e.to_string()),
            })
            .collect();
    }

    // Resolve "all" / "*" once for the entire batch.
    let resolved = match resolve_config_language(config) {
        Ok(r) => r,
        Err(e) => {
            return file_paths
                .into_iter()
                .map(|path| BatchItemResult {
                    file_path: path,
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                })
                .collect();
        }
    };
    let config = resolved.as_ref().unwrap_or(config);

    #[cfg(not(target_arch = "wasm32"))]
    {
        file_paths
            .par_iter()
            .map(|path| {
                let image_bytes = match std::fs::read(path) {
                    Ok(b) => b,
                    Err(e) => {
                        return BatchItemResult {
                            file_path: path.clone(),
                            success: false,
                            result: None,
                            error: Some(
                                OcrError::IOError(format!("Failed to read file '{}': {}", path, e)).to_string(),
                            ),
                        };
                    }
                };
                match process_image_resolved(&image_bytes, config, cache, None) {
                    Ok(result) => BatchItemResult {
                        file_path: path.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        file_path: path.clone(),
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        file_paths
            .iter()
            .map(|path| {
                let image_bytes = match std::fs::read(path) {
                    Ok(b) => b,
                    Err(e) => {
                        return BatchItemResult {
                            file_path: path.clone(),
                            success: false,
                            result: None,
                            error: Some(
                                OcrError::IOError(format!("Failed to read file '{}': {}", path, e)).to_string(),
                            ),
                        };
                    }
                };
                match process_image_resolved(&image_bytes, config, cache, None) {
                    Ok(result) => BatchItemResult {
                        file_path: path.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        file_path: path.clone(),
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_all_languages() {
        assert!(is_all_languages("all"));
        assert!(is_all_languages("ALL"));
        assert!(is_all_languages("All"));
        assert!(is_all_languages("*"));
        assert!(!is_all_languages("eng"));
        assert!(!is_all_languages("eng+fra"));
        assert!(!is_all_languages(""));
    }

    #[test]
    fn test_resolve_config_language_passthrough() {
        let config = TesseractConfig {
            language: "eng".to_string(),
            ..TesseractConfig::default()
        };
        let resolved = resolve_config_language(&config).unwrap();
        assert!(resolved.is_none(), "non-wildcard should return None (no clone)");
    }

    #[test]
    fn test_compute_image_hash_deterministic() {
        let image_bytes = vec![1, 2, 3, 4, 5];

        let hash1 = crate::cache::blake3_hash_bytes(&image_bytes);
        let hash2 = crate::cache::blake3_hash_bytes(&image_bytes);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_compute_image_hash_different_data() {
        let image_bytes1 = vec![1, 2, 3, 4, 5];
        let image_bytes2 = vec![5, 4, 3, 2, 1];

        let hash1 = crate::cache::blake3_hash_bytes(&image_bytes1);
        let hash2 = crate::cache::blake3_hash_bytes(&image_bytes2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_log_ci_debug_disabled() {
        log_ci_debug(false, "test_stage", || "test message".to_string());
    }

    #[test]
    fn test_log_ci_debug_enabled() {
        log_ci_debug(true, "test_stage", || "test message".to_string());
    }

    #[test]
    fn test_process_image_file_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        };

        let result = process_image_file_with_cache("/nonexistent/file.png", &config, &cache, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read file"));
    }

    #[test]
    fn test_process_image_invalid_image_data() {
        let temp_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        };

        let invalid_data = vec![0, 1, 2, 3, 4];
        let result = process_image_with_cache(&invalid_data, &config, &cache, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_rotate_rgb_image_data_identity() {
        // 2x3 RGB image (6 pixels, 18 bytes)
        let data: Vec<u8> = (0..18).collect();
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 3, 0);
        assert_eq!(out, data);
        assert_eq!(w, 2);
        assert_eq!(h, 3);
    }

    #[test]
    fn test_rotate_rgb_image_data_180() {
        // 2x2 RGB image: pixels [A, B, C, D] reversed to [D, C, B, A]
        let data = vec![
            1, 2, 3, // pixel (0,0)
            4, 5, 6, // pixel (1,0)
            7, 8, 9, // pixel (0,1)
            10, 11, 12, // pixel (1,1)
        ];
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 2, 180);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        // 180° rotation reverses pixel order
        assert_eq!(out, vec![10, 11, 12, 7, 8, 9, 4, 5, 6, 1, 2, 3]);
    }

    #[test]
    fn test_rotate_rgb_image_data_90_swaps_dimensions() {
        // 2x3 image -> 3x2 image after 90° rotation
        let data: Vec<u8> = (0..18).collect();
        let (_, w, h) = rotate_rgb_image_data(&data, 2, 3, 90);
        assert_eq!(w, 3); // height becomes width
        assert_eq!(h, 2); // width becomes height
    }

    #[test]
    fn test_rotate_rgb_image_data_270_swaps_dimensions() {
        let data: Vec<u8> = (0..18).collect();
        let (_, w, h) = rotate_rgb_image_data(&data, 2, 3, 270);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
    }

    #[test]
    fn test_rotate_rgb_image_data_90_then_270_is_identity() {
        let data: Vec<u8> = (0..18).collect();
        let (rotated_90, w1, h1) = rotate_rgb_image_data(&data, 2, 3, 90);
        let (back, w2, h2) = rotate_rgb_image_data(&rotated_90, w1, h1, 270);
        assert_eq!(w2, 2);
        assert_eq!(h2, 3);
        assert_eq!(back, data);
    }

    #[test]
    fn test_rotate_rgb_image_data_unsupported_angle() {
        let data: Vec<u8> = (0..12).collect();
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 2, 45);
        assert_eq!(out, data); // unchanged
        assert_eq!(w, 2);
        assert_eq!(h, 2);
    }
}
