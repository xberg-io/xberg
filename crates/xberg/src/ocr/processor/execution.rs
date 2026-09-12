//! OCR execution and result processing.
//!
//! This module handles the core OCR execution logic, including image processing,
//! text extraction, and result formatting.

use super::api_pool::TesseractApiPool;
use super::config::{apply_tesseract_variables, hash_config};
use super::validation::{
    resolve_all_installed_languages, resolve_tessdata_path, strip_control_characters, validate_language_and_traineddata,
};
use crate::core::config::ExtractionConfig;
/// GH#1630/GH#1621: the explicit-hint-then-embedded-PNG-density-then-`None` precedence is
/// defined once and shared with `extractors::image::normalize_image_bytes_for_ocr`.
use crate::extraction::image::resolve_known_source_dpi;
use crate::extractors::security::SecurityLimits;
use crate::image::normalize_image_dpi_owned;
use crate::ocr::cache::OcrCache;
use crate::ocr::conversion::{TsvRow, iterator_word_to_element, tsv_row_to_element};
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::{
    DictionaryLineFilter, RetainedWordConfidenceStats, parse_hocr_to_internal_document_with_page_offset_and_stats,
};
use crate::ocr::preprocessing::preprocess_pix;
#[cfg(test)]
use crate::ocr::preprocessing::should_invert_for_polarity;
#[cfg(feature = "pdf")]
use crate::ocr::table::post_process_table;
use crate::ocr::table::{extract_words_from_tsv, reconstruct_table, table_to_markdown};
#[cfg(test)]
use crate::ocr::types::BatchItemResult;
use crate::ocr::types::TesseractConfig;
use crate::types::internal::{ElementKind, InternalDocument};
use crate::types::{OcrExtractionResult, OcrTable, OcrTableBoundingBox};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use xberg_tesseract::{TessPageSegMode, TessPolyBlockType, TesseractAPI};

/// Process-global document-orientation classifier (ONNX PP-LCNet), shared by
/// every tesseract-backend auto-rotate call. Session initialization is lazy and
/// internally synchronized; `detect` is thread-safe.
#[cfg(auto_rotate)]
fn doc_orientation_detector() -> &'static crate::doc_orientation::DocOrientationDetector {
    static DETECTOR: std::sync::LazyLock<crate::doc_orientation::DocOrientationDetector> =
        std::sync::LazyLock::new(|| {
            crate::doc_orientation::DocOrientationDetector::with_acceleration(
                crate::doc_orientation::resolve_cache_dir(),
                None,
            )
        });
    &DETECTOR
}

use crate::table_core::{MIN_TABLE_CANDIDATE_WORDS, cluster_words_into_table_regions};
use crate::types::OcrElement;

#[cfg(auto_rotate)]
/// Rotate raw RGB image data by the given degrees (0, 90, 180, 270).
///
/// Returns the rotated pixel data and the new (width, height).
/// For 90° and 270° rotations, width and height are swapped.
fn rotate_rgb_image_data(data: &[u8], width: u32, height: u32, degrees: i32) -> (Vec<u8>, u32, u32) {
    let bpp = 3usize;
    let w = width as usize;
    let h = height as usize;

    match degrees {
        0 => (data.to_vec(), width, height),
        90 => {
            let new_w = h;
            let new_h = w;
            let mut out = vec![0u8; new_w * new_h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    let dst_x = h - 1 - y;
                    let dst_y = x;
                    let dst_idx = (dst_y * new_w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, new_w as u32, new_h as u32)
        }
        180 => {
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
            let new_w = h;
            let new_h = w;
            let mut out = vec![0u8; new_w * new_h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
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
/// * `page_number` - The true, 1-indexed page number of the source document this TSV came
///   from. Tesseract's own `page_num` TSV column is always `1` for a single-image call (see
///   `TesseractConfig::page_number`'s doc comment), so it is used only to build each
///   element's opaque `parent_id`, never as the returned elements' page number.
///
/// # Returns
///
/// Vector of OcrElements for word-level and line-level entries
fn parse_tsv_to_elements(tsv_data: &str, min_confidence: f64, page_number: u32) -> Vec<OcrElement> {
    let mut elements = Vec::new();

    for line in tsv_data.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 12 {
            continue;
        }

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

        if conf < 0.0 || conf < min_confidence || text.trim().is_empty() {
            continue;
        }

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

        let mut element = tsv_row_to_element(&tsv_row);
        element.page_number = page_number;
        elements.push(element);
    }

    elements
}

/// CI debug logging utility.
///
/// Logs debug messages when XBERG_CI_DEBUG environment variable is set.
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

/// Word-count shortfall a markdown table rebuild is allowed relative to the content it would
/// replace before the rebuild is rejected as content loss (GH#1599). Zero: table syntax (`|`,
/// `---`) itself adds whitespace-separated tokens, so a rebuild that faithfully reformats
/// existing prose into a table is never word-count-negative -- only a rebuild that actually
/// dropped page content comes in under the original count.
const TABLE_REBUILD_MIN_WORD_RETENTION: usize = 0;

/// Whether a markdown table rebuild should replace `original_content`.
///
/// `build_content_with_inline_tables` reconstructs page content from a crude y-position
/// clustering heuristic that is far less robust than the hOCR-derived content it replaces.
/// Non-emptiness alone (the previous guard) cannot distinguish a legitimate rebuild from one
/// that silently dropped most of the page -- see GH#1599, where OCR markdown output lost a
/// large amount of content that plain output retained. Comparing absolute word counts catches
/// that: a rebuild is adopted only when it does not lose material.
fn should_adopt_table_rebuild(original_content: &str, rebuilt_content: &str) -> bool {
    let original_word_count = original_content.split_whitespace().count();
    let rebuilt_word_count = rebuilt_content.split_whitespace().count();
    rebuilt_word_count + TABLE_REBUILD_MIN_WORD_RETENTION >= original_word_count
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

    let table_bboxes: Vec<_> = tables.iter().filter_map(|t| t.bounding_box.as_ref()).collect();

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

    if non_table_words.is_empty() && tables.is_empty() {
        return String::new();
    }

    let mut sorted_words = non_table_words;
    sorted_words.sort_by(|a, b| a.top.cmp(&b.top).then(a.left.cmp(&b.left)));

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

    let paragraph_gap = avg_height * 2;

    struct Paragraph {
        y_start: u32,
        text: String,
    }

    let mut paragraphs: Vec<Paragraph> = Vec::new();
    for line in &lines {
        if let Some(last_para) = paragraphs.last_mut() {
            let last_y = last_para.y_start;
            if line.y_center.saturating_sub(last_y) <= paragraph_gap {
                last_para.text.push('\n');
                last_para.text.push_str(&line.text);
                last_para.y_start = line.y_center;
                continue;
            }
        }
        paragraphs.push(Paragraph {
            y_start: line.y_center,
            text: line.text.clone(),
        });
    }

    enum ContentElement<'a> {
        Paragraph { y: u32, text: String },
        Table { y: u32, markdown: &'a str },
    }

    let mut elements: Vec<ContentElement> = Vec::new();

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
        let _ = para_idx;
    }

    for table in tables {
        if let Some(ref bbox) = table.bounding_box {
            elements.push(ContentElement::Table {
                y: bbox.top,
                markdown: &table.markdown,
            });
        } else {
            elements.push(ContentElement::Table {
                y: u32::MAX,
                markdown: &table.markdown,
            });
        }
    }

    elements.sort_by_key(|e| match e {
        ContentElement::Paragraph { y, .. } => *y,
        ContentElement::Table { y, .. } => *y,
    });

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

/// Flatten hOCR-derived paragraph elements to unformatted text, joined by
/// blank lines (#185).
///
/// This used to also promote large-font, short, single-line paragraphs to
/// `## ` markdown headings using the average `x_fsize` captured per
/// paragraph by `hocr_parser::parse_hocr_to_internal_document`. That baked
/// markdown syntax directly into `OcrExtractionResult::content`, which is
/// consumed unrendered by callers that expect `Plain` output (e.g. the
/// `flat_ocr_page_document` fallback in `extractors/pdf/ocr.rs`), leaking
/// `## ` into supposedly-plain text. It also duplicated the document-global
/// structure pipeline, which classifies headings from the same `x_fsize`
/// attribute across the whole document and is strictly better informed than
/// a single page's local heuristic. That pipeline (driven by
/// `hocr_document`/`internal_document`, not this string) is now the sole
/// owner of heading detection for routes that have it; this function only
/// flattens text. The standalone-image path, which has no structure
/// pipeline behind it, keeps its own font-size-based heading promotion in
/// `extractors/image.rs`, expressed as real `Heading` elements rather than
/// pre-escaped markdown text.
fn flatten_hocr_elements_to_text(elements: &[crate::types::internal::InternalElement]) -> String {
    elements
        .iter()
        .filter(|e| !matches!(e.kind, ElementKind::PageBreak) && !e.text.is_empty())
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Drop hOCR paragraph elements whose text was already claimed by a detected table (#1571).
///
/// `hocr_document` is parsed straight from the raw hOCR before table detection runs, so
/// nothing ever removes a table's words from it once `tables` is computed: every consumer
/// built from `internal_document` (the PDF mixed/OCR-only routes and the standalone image
/// route) receives the table's text twice, once as ordinary paragraphs and once as the
/// `OcrTable`. `build_content_with_inline_tables` already solves this for the rendered
/// `content` string using a word-centre-in-bbox test; apply the same test here at the
/// paragraph level, using the paragraph's own bbox centre, so `internal_document` agrees
/// with `content` regardless of `output_format` (the string rebuild above is skipped for
/// Plain/Djot output, but the duplication it was masking is not). ~keep
fn filter_elements_covered_by_tables(
    elements: Vec<crate::types::internal::InternalElement>,
    tables: &[OcrTable],
) -> Vec<crate::types::internal::InternalElement> {
    let table_bboxes: Vec<_> = tables.iter().filter_map(|t| t.bounding_box.as_ref()).collect();
    if table_bboxes.is_empty() {
        return elements;
    }

    elements
        .into_iter()
        .filter(|element| {
            let Some(bbox) = element.bbox.as_ref() else {
                return true;
            };
            let center_x = (bbox.x0 + bbox.x1) / 2.0;
            let center_y = (bbox.y0 + bbox.y1) / 2.0;
            !table_bboxes.iter().any(|table_bbox| {
                center_x >= table_bbox.left as f64
                    && center_x <= table_bbox.right as f64
                    && center_y >= table_bbox.top as f64
                    && center_y <= table_bbox.bottom as f64
            })
        })
        .collect()
}

/// Minimum confidence for accepting orientation detection results.
///
/// Keep in sync with `doc_orientation::MIN_CONFIDENCE` (module is feature-gated,
/// this const is not): the PP-LCNet classifier reports correct 90°/270° rotations
/// on real documents at ~0.45, so a 0.5 cutoff rejects valid detections while
/// 0° false positives above 0.35 are rare.
const MIN_ORIENTATION_CONFIDENCE: f32 = 0.35;

#[cfg(auto_rotate)]
const _: () = assert!(MIN_ORIENTATION_CONFIDENCE == crate::doc_orientation::MIN_CONFIDENCE);

/// Source resolution assumed when OCR receives the original image unchanged and the caller did
/// not tell us what resolution it really is.
///
/// Only a fallback. `TesseractConfig::source_dpi`, when the caller knows the true value, takes
/// precedence — see [`prepare_ocr_image`].
const RAW_IMAGE_SOURCE_DPI: i32 = 72;
/// Source resolution retained when explicit DPI normalization fails.
const PREPROCESSING_FALLBACK_SOURCE_DPI: i32 = 300;
/// Pixel stride used by phase-sensitive classifier regression fixtures.
#[cfg(test)]
const DEFAULT_PREPROCESSING_SAMPLE_STRIDE: usize = 4;
/// Minimum normalized mean luminance for automatic document-page preprocessing.
const CLEAN_PAGE_MEAN_LUMINANCE_THRESHOLD: f64 = 0.90;
/// Normalized luminance at which a sampled pixel counts as near-white.
const CLEAN_PAGE_LIGHT_PIXEL_THRESHOLD: f64 = 0.90;
/// Minimum summed RGB channel value equivalent to the light-pixel threshold.
const CLEAN_PAGE_LIGHT_CHANNEL_SUM_THRESHOLD: u16 =
    (CLEAN_PAGE_LIGHT_PIXEL_THRESHOLD * RGB_CHANNEL_MAX * RGB_CHANNEL_COUNT as f64).ceil() as u16;
/// Minimum near-white sample fraction for automatic document-page preprocessing.
const CLEAN_PAGE_LIGHT_PIXEL_FRACTION_THRESHOLD: f64 = 0.80;
/// Minimum separation between background and foreground luminance modes. ~keep
const CLEAN_PAGE_MIN_FOREGROUND_CONTRAST: f64 = 0.20;
/// Minimum image fraction required before a lower foreground mode counts as significant. ~keep
const CLEAN_PAGE_DARK_PIXEL_FRACTION_THRESHOLD: f64 = 0.005;
/// Minimum population required for the lower foreground mode on small rasters. ~keep
const CLEAN_PAGE_DARK_PIXEL_COUNT_THRESHOLD: usize = 8;
const CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_NUMERATOR: usize = 3;
const CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_DENOMINATOR: usize = 4;
const CLEAN_PAGE_TEXT_COMPONENT_MIN_NARROW_ASPECT_RATIO: usize = 3;
/// Maximum value of an 8-bit RGB channel.
const RGB_CHANNEL_MAX: f64 = u8::MAX as f64;
/// Number of channels in an RGB pixel.
const RGB_CHANNEL_COUNT: usize = 3;

struct PreparedOcrImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
    source_dpi: i32,
    apply_pix_preprocessing: bool,
    preprocessing: Option<crate::types::ImagePreprocessingConfig>,
    image_preprocessing: Option<crate::types::ImagePreprocessingMetadata>,
}

/// Prepare the raster Tesseract will recognize, and report the resolution it should be told.
///
/// `known_source_dpi` is the true resolution of `rgb_data` when the caller knows it (the PDF OCR
/// route derives it from the render, and [`resolve_known_source_dpi`] also reads it from PNG
/// density metadata), and `None` when it genuinely does not (raw images handed in by a user).
/// Both branches below honour it: the unpreprocessed branch reports it verbatim instead of the
/// [`RAW_IMAGE_SOURCE_DPI`] assumption, and the preprocessed branch feeds it to DPI normalization
/// so the resize scales from the real resolution.
fn prepare_ocr_image(
    rgb_data: Vec<u8>,
    width: u32,
    height: u32,
    preprocessing: Option<&crate::types::ImagePreprocessingConfig>,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    ci_debug_enabled: bool,
    known_source_dpi: Option<f64>,
) -> PreparedOcrImage {
    let Some(preprocessing) = preprocessing else {
        if should_apply_default_preprocessing(&rgb_data, width, height) {
            return prepare_preprocessed_ocr_image(
                rgb_data,
                width,
                height,
                &crate::types::ImagePreprocessingConfig::default(),
                images_config,
                ci_debug_enabled,
                known_source_dpi,
            );
        }
        return PreparedOcrImage {
            data: rgb_data,
            width,
            height,
            // The image is passed through untouched, so its resolution is whatever the caller
            // says it is; 72 remains the assumption only when nobody knows.
            source_dpi: known_source_dpi.map_or(RAW_IMAGE_SOURCE_DPI, |dpi| dpi.round() as i32),
            apply_pix_preprocessing: false,
            preprocessing: None,
            image_preprocessing: None,
        };
    };

    prepare_preprocessed_ocr_image(
        rgb_data,
        width,
        height,
        preprocessing,
        images_config,
        ci_debug_enabled,
        known_source_dpi,
    )
}

/// Classify bright, page-like RGB images that benefit from the default OCR preprocessing path.
fn should_apply_default_preprocessing(rgb_data: &[u8], width: u32, height: u32) -> bool {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return false;
    };
    let Some(expected_len) = pixel_count.checked_mul(RGB_CHANNEL_COUNT) else {
        return false;
    };
    if pixel_count == 0 || rgb_data.len() != expected_len {
        return false;
    }

    let mut channel_sum = 0u128;
    let mut light_channel_sum = 0u128;
    let mut foreground_channel_sum = 0u128;
    let mut light_pixels = 0usize;
    let mut foreground_pixels = 0usize;
    let mut foreground_histogram = [0usize; 256];

    for pixel in rgb_data.chunks_exact(RGB_CHANNEL_COUNT) {
        let pixel_channel_sum = u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
        channel_sum += u128::from(pixel_channel_sum);
        if pixel_channel_sum >= CLEAN_PAGE_LIGHT_CHANNEL_SUM_THRESHOLD {
            light_channel_sum += u128::from(pixel_channel_sum);
            light_pixels += 1;
        } else {
            foreground_channel_sum += u128::from(pixel_channel_sum);
            foreground_pixels += 1;
            foreground_histogram[((pixel_channel_sum + 1) / RGB_CHANNEL_COUNT as u16) as usize] += 1;
        }
    }
    let pixel_count_f64 = pixel_count as f64;
    let max_pixel_channel_sum = RGB_CHANNEL_MAX * RGB_CHANNEL_COUNT as f64;
    if channel_sum as f64 / (pixel_count_f64 * max_pixel_channel_sum) < CLEAN_PAGE_MEAN_LUMINANCE_THRESHOLD
        || light_pixels as f64 / pixel_count_f64 < CLEAN_PAGE_LIGHT_PIXEL_FRACTION_THRESHOLD
    {
        return false;
    }
    if foreground_pixels == 0 {
        return true;
    }

    let background_luminance = light_channel_sum as f64 / (light_pixels as f64 * max_pixel_channel_sum);
    let foreground_luminance = foreground_channel_sum as f64 / (foreground_pixels as f64 * max_pixel_channel_sum);
    if background_luminance - foreground_luminance >= CLEAN_PAGE_MIN_FOREGROUND_CONTRAST {
        return true;
    }

    let Some((lower_mode_max, lower_mode_pixels)) = foreground_lower_mode(&foreground_histogram) else {
        return false;
    };
    lower_mode_pixels >= CLEAN_PAGE_DARK_PIXEL_COUNT_THRESHOLD
        && lower_mode_pixels as f64 / pixel_count_f64 >= CLEAN_PAGE_DARK_PIXEL_FRACTION_THRESHOLD
        && lower_mode_has_text_structure(
            rgb_data,
            width as usize,
            height as usize,
            lower_mode_max,
            lower_mode_pixels,
        )
}

fn foreground_lower_mode(histogram: &[usize; 256]) -> Option<(u8, usize)> {
    let total_count: usize = histogram.iter().sum();
    let total_weight: u128 = histogram
        .iter()
        .enumerate()
        .map(|(value, count)| value as u128 * *count as u128)
        .sum();
    let mut lower_count = 0usize;
    let mut lower_weight = 0u128;
    let mut best: Option<(f64, u8, usize, f64, f64)> = None;

    for (threshold, &count) in histogram.iter().enumerate().take(u8::MAX as usize) {
        lower_count += count;
        lower_weight += threshold as u128 * count as u128;
        let upper_count = total_count - lower_count;
        if lower_count == 0 || upper_count == 0 {
            continue;
        }
        let lower_mean = lower_weight as f64 / lower_count as f64;
        let upper_mean = (total_weight - lower_weight) as f64 / upper_count as f64;
        let separation = upper_mean - lower_mean;
        let between_class_variance = lower_count as f64 * upper_count as f64 * separation * separation;
        if best
            .as_ref()
            .is_none_or(|(variance, ..)| between_class_variance > *variance)
        {
            best = Some((
                between_class_variance,
                threshold as u8,
                lower_count,
                lower_mean,
                upper_mean,
            ));
        }
    }

    let (_, threshold, lower_count, lower_mean, upper_mean) = best?;
    ((upper_mean - lower_mean) / RGB_CHANNEL_MAX >= CLEAN_PAGE_MIN_FOREGROUND_CONTRAST)
        .then_some((threshold, lower_count))
}

fn lower_mode_has_text_structure(
    rgb_data: &[u8],
    width: usize,
    height: usize,
    lower_mode_max: u8,
    lower_mode_pixels: usize,
) -> bool {
    let Some(pixel_count) = width.checked_mul(height) else {
        return false;
    };
    let lower_mode_mask: Vec<bool> = rgb_data
        .chunks_exact(RGB_CHANNEL_COUNT)
        .map(|pixel| {
            let channel_sum = u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
            (channel_sum + 1) / RGB_CHANNEL_COUNT as u16 <= u16::from(lower_mode_max)
        })
        .collect();
    if lower_mode_mask.len() != pixel_count {
        return false;
    }

    let mut visited = vec![false; pixel_count];
    let mut component_stack = Vec::new();
    let mut structured_pixels = 0usize;
    for start in 0..pixel_count {
        if !lower_mode_mask[start] || visited[start] {
            continue;
        }
        let component_pixels = structured_component_size(
            &lower_mode_mask,
            &mut visited,
            &mut component_stack,
            start,
            width,
            height,
        );
        let Some(total) = structured_pixels.checked_add(component_pixels) else {
            return false;
        };
        structured_pixels = total;
    }
    structured_pixels >= lower_mode_pixels.div_ceil(2)
}

fn structured_component_size(
    mask: &[bool],
    visited: &mut [bool],
    stack: &mut Vec<usize>,
    start: usize,
    width: usize,
    height: usize,
) -> usize {
    stack.clear();
    stack.push(start);
    visited[start] = true;
    let (mut min_x, mut max_x) = (start % width, start % width);
    let (mut min_y, mut max_y) = (start / width, start / width);
    let mut size = 0usize;

    while let Some(index) = stack.pop() {
        let Some(next_size) = size.checked_add(1) else {
            return 0;
        };
        size = next_size;
        let x = index % width;
        let y = index / width;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        let left = x.checked_sub(1).and_then(|_| index.checked_sub(1));
        let right = x
            .checked_add(1)
            .filter(|next_x| *next_x < width)
            .and_then(|_| index.checked_add(1));
        let above = y.checked_sub(1).and_then(|_| index.checked_sub(width));
        let below = y
            .checked_add(1)
            .filter(|next_y| *next_y < height)
            .and_then(|_| index.checked_add(width));
        let neighbors = [left, right, above, below];
        for neighbor in neighbors.into_iter().flatten() {
            if mask[neighbor] && !visited[neighbor] {
                visited[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }

    if component_is_text_like(size, min_x, max_x, min_y, max_y) {
        size
    } else {
        0
    }
}

fn component_is_text_like(size: usize, min_x: usize, max_x: usize, min_y: usize, max_y: usize) -> bool {
    if min_x >= max_x || min_y >= max_y {
        return false;
    }
    let component_width = max_x - min_x + 1;
    let component_height = max_y - min_y + 1;
    let Some(area) = component_width.checked_mul(component_height) else {
        return false;
    };
    let Some(scaled_size) = size.checked_mul(CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_DENOMINATOR) else {
        return false;
    };
    let Some(maximum_fill) = area.checked_mul(CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_NUMERATOR) else {
        return false;
    };
    let short_side = component_width.min(component_height);
    let long_side = component_width.max(component_height);
    let is_narrow = short_side
        .checked_mul(CLEAN_PAGE_TEXT_COMPONENT_MIN_NARROW_ASPECT_RATIO)
        .is_some_and(|minimum_long_side| long_side >= minimum_long_side);
    scaled_size <= maximum_fill || is_narrow
}

fn prepare_preprocessed_ocr_image(
    rgb_data: Vec<u8>,
    width: u32,
    height: u32,
    preprocessing: &crate::types::ImagePreprocessingConfig,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    ci_debug_enabled: bool,
    known_source_dpi: Option<f64>,
) -> PreparedOcrImage {
    // `target_dpi` always comes from the (Tesseract-specific) `preprocessing` config, which
    // takes precedence when explicitly set. The dimension/auto-adjust limits have no home in
    // `ImagePreprocessingConfig`, so they come from the real `ImageExtractionConfig` when the
    // caller has one, instead of being silently defaulted (issue #209).
    let dpi_config = match images_config {
        Some(images_config) => crate::types::ImageDpiConfig {
            target_dpi: preprocessing.target_dpi,
            ..crate::types::ImageDpiConfig::from(images_config)
        },
        None => crate::types::ImageDpiConfig {
            target_dpi: preprocessing.target_dpi,
            ..Default::default()
        },
    };
    // `known_source_dpi` is the whole point of this parameter: passing `None` here makes
    // `normalize_image_dpi_owned` assume 72 DPI, which on a page rendered at 150 inflates the
    // scale factor to `target_dpi / 72` and drives a Letter page into the `max_image_dimension`
    // clamp — 6x the pixels of the correct resize, carrying no more information, and reported to
    // Tesseract as roughly half the raster's real `scan_res`.
    match normalize_image_dpi_owned(rgb_data, width as usize, height as usize, &dpi_config, known_source_dpi) {
        Ok(result) => {
            let normalized_width = result.dimensions.0 as u32;
            let normalized_height = result.dimensions.1 as u32;
            let source_dpi = result.metadata.final_dpi;
            log_ci_debug(ci_debug_enabled, "dpi_normalization", || {
                format!(
                    "original={}x{} normalized={}x{} target_dpi={} final_dpi={} resized={}",
                    width,
                    height,
                    normalized_width,
                    normalized_height,
                    result.metadata.target_dpi,
                    source_dpi,
                    !result.metadata.skipped_resize
                )
            });
            PreparedOcrImage {
                data: result.rgb_data,
                width: normalized_width,
                height: normalized_height,
                source_dpi,
                apply_pix_preprocessing: true,
                preprocessing: Some(preprocessing.clone()),
                image_preprocessing: Some(result.metadata),
            }
        }
        Err((error, data)) => {
            tracing::warn!("DPI normalization failed, using original image: {}", error);
            PreparedOcrImage {
                data,
                width,
                height,
                // The image comes back unresized here, so its resolution is still the source
                // one. The 300 fallback is a guess for when there is nothing better.
                source_dpi: known_source_dpi.map_or(PREPROCESSING_FALLBACK_SOURCE_DPI, |dpi| dpi.round() as i32),
                apply_pix_preprocessing: true,
                preprocessing: Some(preprocessing.clone()),
                image_preprocessing: None,
            }
        }
    }
}

/// Check whether a center point (x, y) lies within a bounding box.
fn point_in_bbox(x: i32, y: i32, left: i32, top: i32, right: i32, bottom: i32) -> bool {
    x >= left && x <= right && y >= top && y <= bottom
}

/// Result of [`extract_elements_via_iterator`]: the extracted elements plus
/// counters distinguishing "nothing to extract" from "extraction degraded"
/// (#192, #180).
struct IteratorExtractionResult {
    elements: Vec<OcrElement>,
    retained_text_confidence_stats: Option<RetainedWordConfidenceStats>,
    /// Words the Tesseract result iterator itself failed to extract
    /// (null pointer / invalid parameter / invalid UTF-8 per word), per
    /// `ResultIterator::extract_all_words` (#192).
    skipped_words: usize,
    /// Words whose parent block is a non-flowing-text type (noise, an
    /// embedded image region, or a ruling line) that are now retained in
    /// `elements` instead of being silently dropped (#180).
    non_text_block_word_count: usize,
    /// Fraction of this page's dictionary-checkable words that Tesseract's own DAWG
    /// dictionary (`TesseractAPI::is_valid_word`) rejects as not-a-word. See
    /// [`dictionary_invalid_word_ratio`] for what counts as checkable and why `None`
    /// (rather than `0.0`) means "too few checkable words to be meaningful".
    dict_invalid_word_ratio: Option<f64>,
}

/// Minimum letters a word must have before Tesseract's dictionary lookup is meaningful.
/// Digits, single letters, and punctuation are never in the DAWG dictionary regardless of
/// legitimacy, so scoring them would bias the ratio toward "invalid" on output OCR read
/// perfectly correctly.
const MIN_WORD_LEN_FOR_DICT_CHECK: usize = 3;

/// Minimum number of dictionary-checkable words on a page before
/// [`dictionary_invalid_word_ratio`] reports a ratio at all, mirroring the same
/// conservatism as `OcrQualityThresholds::min_words_for_ocr_output_check`: a short page
/// (a signature block, an exhibit title) does not carry enough checkable words for the
/// ratio to mean anything.
const MIN_DICT_CANDIDATES_FOR_RATIO: usize = 5;

/// Fraction of a page's alphabetic, dictionary-checkable words that Tesseract's own
/// dictionary rejects as not-a-word.
///
/// This is the dictionary-validity supplement to the recognition-noise gate
/// (`is_ocr_recognition_noise` in `extractors::pdf::ocr`): a page of scanned line art
/// noise ("LAAALDLI", "AEA") is read by Tesseract with confident-looking output, so
/// per-word OCR confidence and the fragmented-word-ratio heuristic do not always catch
/// it — but few of its "words" are real dictionary words. `TesseractAPI::is_valid_word`
/// is Tesseract's own DAWG dictionary lookup, and is only usable while the `TesseractAPI`
/// handle that performed this page's OCR is still open (the dictionary lives inside that
/// engine instance) — callers must call this before the API is dropped.
///
/// Returns `None`, never `0.0`, when there are too few dictionary-checkable words to make
/// a ratio meaningful: a `0.0` here would read as "every word is valid" to a caller that
/// cannot tell "no evidence" from "good evidence", and this signal is a veto input, so
/// that conflation would suppress real content.
fn dictionary_invalid_word_ratio(api: &TesseractAPI, words: &[xberg_tesseract::WordData]) -> Option<f64> {
    invalid_word_ratio_from(words, |text| match api.is_valid_word(text) {
        Ok(0) => Some(false),
        Ok(_) => Some(true),
        // Lookup failed (e.g. interior NUL byte); don't let it bias the ratio either way.
        Err(_) => None,
    })
}

/// Pure core of [`dictionary_invalid_word_ratio`], taking dictionary lookup as an injected
/// function (`Some(true)` valid, `Some(false)` invalid, `None` lookup failed / skip) so it
/// is unit-testable without a live `TesseractAPI` handle.
fn invalid_word_ratio_from(
    words: &[xberg_tesseract::WordData],
    is_valid: impl Fn(&str) -> Option<bool>,
) -> Option<f64> {
    let mut candidates = 0usize;
    let mut invalid = 0usize;
    for word in words {
        let text = word.text.trim();
        if text.chars().count() < MIN_WORD_LEN_FOR_DICT_CHECK || !text.chars().all(|c| c.is_alphabetic()) {
            continue;
        }
        match is_valid(text) {
            Some(true) => candidates += 1,
            Some(false) => {
                candidates += 1;
                invalid += 1;
            }
            None => {}
        }
    }
    if candidates < MIN_DICT_CANDIDATES_FOR_RATIO {
        return None;
    }
    Some(invalid as f64 / candidates as f64)
}

/// Block types that historically caused a word to be dropped entirely from
/// `ocr_elements`. Words in these blocks are now retained (tagged with their
/// `block_type` via `iterator_word_to_element`) so text baked into figures,
/// chart axis labels, pull-out captions, and ruling-line regions is not lost
/// from OCR output (#180). Kept as a named set purely to identify and count
/// them for the `non_text_block_word_count` signal.
const NON_TEXT_BLOCK_TYPES: [TessPolyBlockType; 6] = [
    TessPolyBlockType::PT_NOISE,
    TessPolyBlockType::PT_FLOWING_IMAGE,
    TessPolyBlockType::PT_HEADING_IMAGE,
    TessPolyBlockType::PT_PULLOUT_IMAGE,
    TessPolyBlockType::PT_HORZ_LINE,
    TessPolyBlockType::PT_VERT_LINE,
];

/// Whether `auto_rotate` was requested by the caller but orientation detection
/// could not run because the `auto-rotate` build feature is not compiled in.
///
/// Extracted as a pure function, separate from the surrounding OCR pipeline and
/// its FFI calls, so the request-vs-availability logic is unit-testable without
/// a live Tesseract API instance (#309).
fn is_auto_rotate_requested_but_unavailable(auto_rotate_enabled: bool) -> bool {
    #[cfg(auto_rotate)]
    {
        let _ = auto_rotate_enabled;
        false
    }
    #[cfg(not(auto_rotate))]
    {
        auto_rotate_enabled
    }
}

/// Inserts the `word_iterator_skipped_count` metadata key only when at least one
/// word was actually skipped by the Tesseract result iterator, so callers can use
/// the key's absence (rather than a `0` value) as the "clean extraction" signal.
///
/// Extracted as its own function, separate from the surrounding OCR pipeline, so
/// the presence/absence and exact-value behaviour is unit-testable without a live
/// Tesseract API instance (#192).
fn insert_word_iterator_skipped_count_metadata(
    metadata: &mut HashMap<String, serde_json::Value>,
    skipped_words: usize,
) {
    if skipped_words > 0 {
        metadata.insert(
            "word_iterator_skipped_count".to_string(),
            serde_json::Value::Number(skipped_words.into()),
        );
    }
}

fn insert_retained_word_confidence_metadata(
    metadata: &mut HashMap<String, serde_json::Value>,
    stats: &RetainedWordConfidenceStats,
) {
    for key in [
        "word_count",
        "low_conf_word_count",
        "mean_text_conf",
        "median_word_conf",
        "p10_word_conf",
    ] {
        metadata.remove(key);
    }
    metadata.insert(
        "word_count".to_string(),
        serde_json::Value::Number(stats.word_count().into()),
    );
    metadata.insert(
        "low_conf_word_count".to_string(),
        serde_json::Value::Number(stats.low_confidence_word_count().into()),
    );
    if let Some(mean) = stats.mean() {
        metadata.insert("mean_text_conf".to_string(), serde_json::Value::Number(mean.into()));
    }
    if let Some(median) = stats.median() {
        metadata.insert("median_word_conf".to_string(), serde_json::Value::Number(median.into()));
    }
    if let Some(p10) = stats.p10() {
        metadata.insert("p10_word_conf".to_string(), serde_json::Value::Number(p10.into()));
    }
}

fn retained_text_word_confidence_stats(
    content: &str,
    words: &[xberg_tesseract::WordData],
) -> RetainedWordConfidenceStats {
    let retained_tokens = content.split_whitespace().collect::<Vec<_>>();
    let mut retained_index = 0usize;
    let mut stats = RetainedWordConfidenceStats::default();
    for word in words {
        let cleaned = strip_control_characters(&word.text);
        let word_tokens = cleaned.split_whitespace().collect::<Vec<_>>();
        if word_tokens.is_empty() {
            continue;
        }
        let Some(next_retained_index) = find_token_sequence_end(&retained_tokens, retained_index, &word_tokens) else {
            break;
        };
        stats.record(f64::from(word.confidence));
        retained_index = next_retained_index;
    }
    stats
}

fn find_token_sequence_end(haystack: &[&str], start: usize, needle: &[&str]) -> Option<usize> {
    if needle.is_empty() {
        return Some(start);
    }

    let mut prefix_lengths = vec![0usize; needle.len()];
    for index in 1..needle.len() {
        let mut matched = prefix_lengths[index - 1];
        while matched > 0 && needle[index] != needle[matched] {
            matched = prefix_lengths[matched - 1];
        }
        if needle[index] == needle[matched] {
            matched += 1;
        }
        prefix_lengths[index] = matched;
    }

    let mut matched = 0usize;
    for (offset, token) in haystack[start..].iter().enumerate() {
        while matched > 0 && *token != needle[matched] {
            matched = prefix_lengths[matched - 1];
        }
        if *token == needle[matched] {
            matched += 1;
            if matched == needle.len() {
                return Some(start + offset + 1);
            }
        }
    }
    None
}

/// Extract OcrElements via Tesseract's iterator APIs with rich metadata.
///
/// Uses ResultIterator for word-level text, bounding boxes, confidence, and font
/// attributes, plus PageIterator for block type and paragraph info. This replaces
/// TSV-based extraction with significantly richer metadata.
fn extract_elements_via_iterator(
    api: &TesseractAPI,
    page_number: u32,
    min_confidence: f64,
    retained_text: Option<&str>,
) -> Result<IteratorExtractionResult, OcrError> {
    let empty = || IteratorExtractionResult {
        elements: Vec::new(),
        retained_text_confidence_stats: None,
        skipped_words: 0,
        non_text_block_word_count: 0,
        dict_invalid_word_ratio: None,
    };

    let page_iter = match api.get_page_iterator() {
        Ok(iter) => iter,
        Err(e) => {
            tracing::warn!(error = %e, "Tesseract page iterator unavailable; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    let blocks = match page_iter.extract_all_blocks() {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to extract Tesseract page blocks; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    let paragraph_extraction = match page_iter.extract_all_paragraphs() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to extract Tesseract page paragraphs; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    if paragraph_extraction.skipped_no_para_info > 0 || paragraph_extraction.skipped_no_bbox > 0 {
        tracing::warn!(
            skipped_no_para_info = paragraph_extraction.skipped_no_para_info,
            skipped_no_bbox = paragraph_extraction.skipped_no_bbox,
            extracted_paragraphs = paragraph_extraction.paragraphs.len(),
            "Tesseract declined to describe some paragraphs; their words lose the is_crown, \
             is_list_item and justification metadata"
        );
    }

    let paragraphs = paragraph_extraction.paragraphs;

    let result_iter = match api.get_iterator() {
        Ok(iter) => iter,
        Err(e) => {
            tracing::warn!(error = %e, "Tesseract result iterator unavailable; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    let word_extraction = match result_iter.extract_all_words() {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!(error = %e, "Tesseract result iterator failed; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    if word_extraction.skipped > 0 {
        tracing::warn!(
            skipped_words = word_extraction.skipped,
            extracted_words = word_extraction.words.len(),
            "Tesseract result iterator failed to extract some words (null pointer, invalid \
             parameter, or invalid UTF-8); they were dropped from OCR output"
        );
    }

    let dict_invalid_word_ratio = dictionary_invalid_word_ratio(api, &word_extraction.words);
    let retained_text_confidence_stats =
        retained_text.map(|content| retained_text_word_confidence_stats(content, &word_extraction.words));

    let mut elements = Vec::new();
    let mut non_text_block_word_count = 0usize;

    for word in &word_extraction.words {
        if (word.confidence as f64) < min_confidence {
            continue;
        }

        if word.text.trim().is_empty() {
            continue;
        }

        let cx = (word.left + word.right) / 2;
        let cy = (word.top + word.bottom) / 2;

        let parent_block = blocks
            .iter()
            .find(|b| point_in_bbox(cx, cy, b.left, b.top, b.right, b.bottom));

        let block_type = parent_block.map(|b| b.block_type);

        if let Some(bt) = block_type
            && NON_TEXT_BLOCK_TYPES.contains(&bt)
        {
            non_text_block_word_count += 1;
        }

        let para_info = paragraphs
            .iter()
            .find(|p| point_in_bbox(cx, cy, p.left, p.top, p.right, p.bottom));

        let element = iterator_word_to_element(word, block_type, para_info, page_number);
        elements.push(element);
    }

    Ok(IteratorExtractionResult {
        elements,
        retained_text_confidence_stats,
        skipped_words: word_extraction.skipped,
        non_text_block_word_count,
        dict_invalid_word_ratio,
    })
}

/// Resolve the `SecurityLimits` to apply when decoding an image for OCR.
///
/// `None` means no `ExtractionConfig` reached this call (internal/test call sites), not
/// that limits should be waived — this falls back to the same default a configured caller
/// gets when they never set `security_limits` explicitly (GH#1554: `load_image_for_ocr`
/// previously hardcoded this default unconditionally, ignoring a caller's own configured,
/// possibly higher, limit). ~keep
fn security_limits_for_ocr(extraction_config: Option<&ExtractionConfig>) -> SecurityLimits {
    extraction_config
        .and_then(|config| config.security_limits.clone())
        .unwrap_or_default()
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
    api_pool: &Arc<TesseractApiPool>,
    extraction_config: Option<&ExtractionConfig>,
) -> Result<OcrExtractionResult, OcrError> {
    let ci_debug_enabled = env::var_os("XBERG_CI_DEBUG").is_some();
    log_ci_debug(ci_debug_enabled, "perform_ocr:start", || {
        format!(
            "bytes={} language={} output={} use_cache={}",
            image_bytes.len(),
            config.language,
            config.output_format,
            config.use_cache
        )
    });

    let security_limits = security_limits_for_ocr(extraction_config);
    let rgb_image = {
        let img = crate::extraction::image::load_image_for_ocr(image_bytes, &security_limits)
            .map_err(|e| OcrError::ImageProcessingFailed(e.to_string()))?;
        img.into_rgb8()
    };
    let (orig_width, orig_height) = rgb_image.dimensions();
    let rgb_data = rgb_image.into_raw();

    log_ci_debug(ci_debug_enabled, "image", || {
        format!("dimensions={}x{} color_type=RGB8", orig_width, orig_height)
    });

    let images_config = extraction_config.and_then(|extraction_config| extraction_config.images.as_ref());
    let known_source_dpi = resolve_known_source_dpi(config.source_dpi, image_bytes);
    let prepared_image = prepare_ocr_image(
        rgb_data,
        orig_width,
        orig_height,
        config.preprocessing.as_ref(),
        images_config,
        ci_debug_enabled,
        known_source_dpi,
    );
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut image_data = prepared_image.data;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut width = prepared_image.width;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut height = prepared_image.height;
    let source_dpi = prepared_image.source_dpi;
    let preprocessing = prepared_image.preprocessing;
    let image_preprocessing = prepared_image.image_preprocessing;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut ocr_image_width = width;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut ocr_image_height = height;

    let bytes_per_pixel: u32 = 3;

    let languages: Vec<String> = config.language.split('+').map(|lang| lang.trim().to_string()).collect();
    let tessdata_path = resolve_tessdata_path(&languages, config.tessdata_path.as_deref())?;

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

    validate_language_and_traineddata(&config.language, &tessdata_path)?;

    let api = api_pool.checkout(&tessdata_path, &config.language)?;

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
    let source_dpi = source_dpi.max(70);

    // Only (degrees, confidence): the ONNX PP-LCNet orientation classifier used
    // here has no script-detection capability, unlike Tesseract's own
    // `DetectOrientationScript()`/OSD, which this crate does not call (#184).
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut detected_orientation: Option<(i32, f32)> = None;
    let auto_rotate_enabled =
        config.preprocessing.as_ref().map(|p| p.auto_rotate).unwrap_or(false) || config.auto_rotate;

    // Recorded into `metadata` below (once `metadata` exists) under the
    // `auto_rotate_unavailable` key, mirroring `pre_formatted` and
    // `word_iterator_skipped_count`: `OcrExtractionResult` has no dedicated
    // warnings field, so `TesseractBackend` reads this key back out and turns
    // it into a `ProcessingWarning` the caller actually sees (#309). Without
    // it, a user's explicit `auto_rotate = true` silently does nothing on a
    // build without the `auto-rotate` feature.
    let auto_rotate_unavailable = is_auto_rotate_requested_but_unavailable(auto_rotate_enabled);

    #[cfg(not(auto_rotate))]
    if auto_rotate_enabled {
        tracing::warn!(
            "auto_rotate requested but the `auto-rotate` feature is not compiled in; skipping orientation detection"
        );
    }

    #[cfg(auto_rotate)]
    if auto_rotate_enabled {
        let expected_rgb_len = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(bytes_per_pixel as usize));
        let orientation_result = if expected_rgb_len != Some(image_data.len()) {
            Err(crate::error::XbergError::Ocr {
                message: "auto_rotate: image buffer does not match dimensions".to_string(),
                source: None,
            })
        } else {
            let image = image::RgbImage::from_raw(width, height, std::mem::take(&mut image_data))
                .expect("validated RGB buffer length must match dimensions");
            let result = doc_orientation_detector().detect(&image);
            image_data = image.into_raw();
            result
        };

        match orientation_result {
            Err(e) => {
                tracing::warn!("Orientation detection failed, proceeding without rotation: {}", e);
            }
            Ok(orientation) => {
                let orient_deg = orientation.degrees as i32;
                let orient_conf = orientation.confidence;
                log_ci_debug(ci_debug_enabled, "orientation_detection", || {
                    format!("orientation={}° confidence={:.2}", orient_deg, orient_conf)
                });
                detected_orientation = Some((orient_deg, orient_conf));

                if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
                    tracing::info!(
                        "Auto-rotating image by {} degrees (confidence: {:.2})",
                        orient_deg,
                        orient_conf
                    );

                    let correction_deg = (360 - orient_deg).rem_euclid(360);
                    let (rotated_data, new_width, new_height) =
                        rotate_rgb_image_data(&image_data, width, height, correction_deg);
                    image_data = rotated_data;
                    width = new_width;
                    height = new_height;
                    ocr_image_width = new_width;
                    ocr_image_height = new_height;

                    log_ci_debug(ci_debug_enabled, "auto_rotate", || {
                        format!("rotated={}° new_dimensions={}x{}", orient_deg, new_width, new_height)
                    });
                } else {
                    tracing::debug!(
                        degrees = orient_deg,
                        confidence = orient_conf,
                        threshold = MIN_ORIENTATION_CONFIDENCE,
                        "auto_rotate: keeping original orientation"
                    );
                }
            }
        }
    }

    let processed_pix: Option<xberg_tesseract::Pix> =
        if let (true, Some(preprocessing)) = (prepared_image.apply_pix_preprocessing, preprocessing.as_ref()) {
            match xberg_tesseract::Pix::from_raw_rgb(&image_data, width, height) {
                Ok(pix) => match preprocess_pix(pix, preprocessing) {
                    Ok(processed) => Some(processed),
                    Err(error) => {
                        tracing::debug!(%error, "Leptonica preprocessing failed; using raw image");
                        None
                    }
                },
                Err(error) => {
                    tracing::debug!(%error, "Leptonica Pix creation failed; using raw image");
                    None
                }
            }
        } else {
            None
        };

    if let Some(ref pix) = processed_pix {
        api.set_image_2(pix.as_ptr())
            .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set preprocessed image: {error}")))?;
    } else {
        let bytes_per_line = width * bytes_per_pixel;
        api.set_image(
            &image_data,
            width as i32,
            height as i32,
            bytes_per_pixel as i32,
            bytes_per_line as i32,
        )
        .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set image: {error}")))?;
    }
    drop(processed_pix);

    api.set_source_resolution(source_dpi)
        .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set source resolution: {error}")))?;

    log_ci_debug(ci_debug_enabled, "set_image", || {
        format!(
            "width={} height={} bytes_per_pixel={} source_dpi={}",
            width, height, bytes_per_pixel, source_dpi
        )
    });

    drop(image_data);

    api.recognize()
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to recognize text: {}", e)))?;

    let mean_text_conf = api.mean_text_conf().unwrap_or(-1);

    log_ci_debug(ci_debug_enabled, "recognize", || {
        format!("completed mean_text_conf={}", mean_text_conf)
    });

    let word_confidence_stats = if matches!(config.output_format.as_str(), "markdown" | "text") {
        None
    } else {
        match api.all_word_confidences() {
            Ok(confidences) if !confidences.is_empty() => {
                let word_count = confidences.len();
                let low_conf_word_count = confidences.iter().filter(|&&c| c < 50).count();

                let mut sorted = confidences.clone();
                sorted.sort_unstable();
                let median_word_conf = if word_count % 2 == 0 {
                    (sorted[word_count / 2 - 1] + sorted[word_count / 2]) / 2
                } else {
                    sorted[word_count / 2]
                };

                let p10_idx = ((word_count as f64 - 1.0) * 0.1).floor() as usize;
                let p10_word_conf = sorted[p10_idx.min(word_count - 1)];

                Some((median_word_conf, p10_word_conf, word_count, low_conf_word_count))
            }
            Ok(_) => match api.mean_text_conf() {
                Ok(mean_conf) => Some((mean_conf, mean_conf, 0usize, 0usize)),
                Err(_) => None,
            },
            Err(_) => match api.mean_text_conf() {
                Ok(mean_conf) => Some((mean_conf, mean_conf, 0usize, 0usize)),
                Err(_) => None,
            },
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
    let mut dictionary_filtered_line_count = 0usize;
    let mut retained_hocr_confidence_stats = None;

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

            // Per-line dictionary-invalid noise filter (#783). Built here, immediately
            // before parsing, using `hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO` -- see
            // that constant's doc comment for why this is not yet a configurable
            // `OcrQualityThresholds` field.
            let is_valid_word = |text: &str| match api.is_valid_word(text) {
                Ok(0) => Some(false),
                Ok(_) => Some(true),
                Err(_) => None,
            };
            let dictionary_filter = DictionaryLineFilter {
                is_valid_word: &is_valid_word,
                max_invalid_ratio: crate::ocr::hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO,
            };

            let parse_result = parse_hocr_to_internal_document_with_page_offset_and_stats(
                &hocr,
                Some(&dictionary_filter),
                config.page_number,
            );
            dictionary_filtered_line_count = parse_result.dictionary_filtered_line_count;
            let content = flatten_hocr_elements_to_text(&parse_result.document.elements);
            retained_hocr_confidence_stats = Some(parse_result.retained_word_confidence_stats);
            hocr_document = Some(parse_result.document);

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
    if let Some(image_preprocessing) = image_preprocessing {
        let value = serde_json::to_value(image_preprocessing).map_err(|error| {
            OcrError::ProcessingFailed(format!("Failed to serialize image preprocessing metadata: {error}"))
        })?;
        metadata.insert(
            crate::ocr_metadata_keys::OCR_IMAGE_PREPROCESSING_METADATA_KEY.to_string(),
            value,
        );
    }
    metadata.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.to_string(),
        serde_json::Value::Number(ocr_image_width.into()),
    );
    metadata.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.to_string(),
        serde_json::Value::Number(ocr_image_height.into()),
    );
    metadata.insert(
        "language".to_string(),
        serde_json::Value::String(config.language.clone()),
    );
    metadata.insert("psm".to_string(), serde_json::Value::String(config.psm.to_string()));
    metadata.insert("table_count".to_string(), serde_json::Value::Number(0.into()));
    metadata.insert("tables_detected".to_string(), serde_json::Value::Number(0.into()));
    if config.output_format == "markdown" {
        metadata.insert(
            "source_format".to_string(),
            serde_json::Value::String("hocr".to_string()),
        );
    }
    if auto_rotate_unavailable {
        metadata.insert("auto_rotate_unavailable".to_string(), serde_json::Value::Bool(true));
    }
    if dictionary_filtered_line_count > 0 {
        metadata.insert(
            "dictionary_filtered_line_count".to_string(),
            serde_json::Value::Number(dictionary_filtered_line_count.into()),
        );
    }

    if let Some(stats) = retained_hocr_confidence_stats.as_ref() {
        insert_retained_word_confidence_metadata(&mut metadata, stats);
    } else if config.output_format != "text" {
        if mean_text_conf >= 0 {
            metadata.insert(
                "mean_text_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(mean_text_conf)),
            );
        }

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
    }

    // No `script_name`/`script_confidence` metadata: the ONNX PP-LCNet
    // orientation classifier used here has no script-detection capability,
    // unlike Tesseract's own `DetectOrientationScript()`/OSD (which this crate
    // does not call). Emitting a placeholder script name would misrepresent
    // real detection as having happened (#184). ~keep
    if let Some((orient_deg, orient_conf)) = detected_orientation {
        metadata.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY.to_string(),
            serde_json::Value::Number(serde_json::Number::from(orient_deg)),
        );
        metadata.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_CONFIDENCE_METADATA_KEY.to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(orient_conf as f64).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
            metadata.insert(
                crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY.to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }

    let mut tables = Vec::new();
    let mut ocr_elements = None;

    if config.enable_table_detection {
        let tsv_data = tsv_data_for_tables.as_ref().unwrap();

        let words = extract_words_from_tsv(tsv_data, config.table_min_confidence)?;
        let regions = cluster_words_into_table_regions(&words);

        for (region_index, region_words) in regions.into_iter().enumerate() {
            if region_words.len() < MIN_TABLE_CANDIDATE_WORDS {
                tracing::debug!(
                    target: "xberg::ocr::tables",
                    region_index,
                    word_count = region_words.len(),
                    min_required = MIN_TABLE_CANDIDATE_WORDS,
                    "OCR table region skipped: below MIN_TABLE_CANDIDATE_WORDS"
                );
                continue;
            }

            let region_left = region_words.iter().map(|w| w.left).min().unwrap_or(0);
            let region_top = region_words.iter().map(|w| w.top).min().unwrap_or(0);
            let region_right = region_words.iter().map(|w| w.left + w.width).max().unwrap_or(0);
            let region_bottom = region_words.iter().map(|w| w.top + w.height).max().unwrap_or(0);
            let word_preview: String = region_words
                .iter()
                .take(12)
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(200)
                .collect();

            let table = reconstruct_table(
                &region_words,
                config.table_column_threshold,
                config.table_row_threshold_ratio,
            );

            tracing::debug!(
                target: "xberg::ocr::tables",
                region_index,
                word_count = region_words.len(),
                left = region_left,
                top = region_top,
                right = region_right,
                bottom = region_bottom,
                word_preview = %word_preview,
                raw_rows = table.len(),
                raw_cols = table.first().map_or(0, Vec::len),
                "OCR table region reconstructed"
            );

            if table.is_empty() || table[0].is_empty() {
                continue;
            }

            #[cfg(feature = "pdf")]
            let cleaned = post_process_table(table, false, false);
            #[cfg(not(feature = "pdf"))]
            let cleaned = Some(table);
            let Some(cleaned) = cleaned else {
                continue;
            };

            let markdown_table = table_to_markdown(&cleaned);

            let left = region_words.iter().map(|w| w.left).min().unwrap_or(0);
            let top = region_words.iter().map(|w| w.top).min().unwrap_or(0);
            let right = region_words.iter().map(|w| w.left + w.width).max().unwrap_or(0);
            let bottom = region_words.iter().map(|w| w.top + w.height).max().unwrap_or(0);

            tables.push(OcrTable {
                cells: cleaned,
                markdown: markdown_table,
                page_number: config.page_number,
                bounding_box: Some(OcrTableBoundingBox {
                    left,
                    top,
                    right,
                    bottom,
                }),
            });
        }

        // Real count, not hardcoded to at-most-one (#177): a page can contain
        // several independent tables separated by prose.
        metadata.insert(
            "table_count".to_string(),
            serde_json::Value::Number(tables.len().into()),
        );
        metadata.insert(
            "tables_detected".to_string(),
            serde_json::Value::Number(tables.len().into()),
        );
        if let Some(first) = tables.first() {
            metadata.insert(
                "table_rows".to_string(),
                serde_json::Value::Number(first.cells.len().into()),
            );
            metadata.insert(
                "table_cols".to_string(),
                serde_json::Value::Number(first.cells.first().map_or(0, Vec::len).into()),
            );
        }
    }

    if let Some(document) = hocr_document.as_mut() {
        document.elements = filter_elements_covered_by_tables(std::mem::take(&mut document.elements), &tables);
    }

    let mut content = strip_control_characters(&raw_content).into_owned();
    let retained_text = (config.output_format == "text").then_some(content.as_str());
    let iterator_extraction =
        extract_elements_via_iterator(&api, config.page_number, config.min_confidence, retained_text);
    if let Ok(extraction) = &iterator_extraction
        && let Some(stats) = extraction.retained_text_confidence_stats.as_ref()
    {
        insert_retained_word_confidence_metadata(&mut metadata, stats);
    }
    match iterator_extraction {
        Ok(extraction) if !extraction.elements.is_empty() => {
            insert_word_iterator_skipped_count_metadata(&mut metadata, extraction.skipped_words);
            if extraction.non_text_block_word_count > 0 {
                metadata.insert(
                    "non_text_block_word_count".to_string(),
                    serde_json::Value::Number(extraction.non_text_block_word_count.into()),
                );
            }
            if let Some(ratio) = extraction.dict_invalid_word_ratio {
                metadata.insert(
                    crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY.to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(ratio).unwrap_or(serde_json::Number::from(0)),
                    ),
                );
            }
            ocr_elements = Some(extraction.elements);
        }
        _ => {
            if let Some(ref tsv_data) = tsv_data_for_tables {
                let elements = parse_tsv_to_elements(tsv_data, config.min_confidence, config.page_number);
                if !elements.is_empty() {
                    ocr_elements = Some(elements);
                }
            }
        }
    }

    let is_markdown_output = extraction_config
        .map(|c| c.output_format == crate::core::config::OutputFormat::Markdown)
        .unwrap_or(config.output_format == "markdown");

    if !tables.is_empty()
        && is_markdown_output
        && let Some(ref tsv_data) = tsv_data_for_tables
    {
        let rebuilt = build_content_with_inline_tables(tsv_data, &tables, config.table_min_confidence);
        if !rebuilt.is_empty() {
            if should_adopt_table_rebuild(&content, &rebuilt) {
                content = rebuilt;
                metadata.insert(
                    "pre_formatted".to_string(),
                    serde_json::Value::String("markdown".to_string()),
                );
            } else {
                tracing::warn!(
                    target: "xberg::ocr::tables",
                    original_word_count = content.split_whitespace().count(),
                    rebuilt_word_count = rebuilt.split_whitespace().count(),
                    "OCR markdown table rebuild dropped content relative to the original; keeping original content"
                );
            }
        }
    }

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
    api_pool: &Arc<TesseractApiPool>,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    let image_bytes = std::fs::read(file_path)
        .map_err(|e| OcrError::IOError(format!("Failed to read file '{}': {}", file_path, e)))?;
    process_image_with_cache(&image_bytes, config, cache, api_pool, output_format)
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
        let bootstrap_langs = vec!["eng".to_string()];
        let tessdata_path = resolve_tessdata_path(&bootstrap_langs, config.tessdata_path.as_deref())?;
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
    api_pool: &Arc<TesseractApiPool>,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    config.validate().map_err(OcrError::InvalidConfiguration)?;

    let resolved = resolve_config_language(config)?;
    let config = resolved.as_ref().unwrap_or(config);

    process_image_resolved(image_bytes, config, cache, api_pool, output_format)
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
    api_pool: &Arc<TesseractApiPool>,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    let image_hash = crate::cache::blake3_hash_bytes(image_bytes);

    let config_str = hash_config(config);

    // `output_format` is part of the cache identity: it selects the renderer and
    // therefore the `content` and `mime_type` of the result. Omitting it served a
    // document cached as one format for a request for another (#205).
    if config.use_cache
        && let Some(cached_result) =
            cache.get_cached_result(&image_hash, "tesseract", &config_str, output_format.as_ref())?
    {
        #[cfg(feature = "otel")]
        tracing::Span::current().record("cache.hit", true);
        return Ok(cached_result);
    }

    #[cfg(feature = "otel")]
    tracing::Span::current().record("cache.hit", false);

    let extraction_config = output_format.as_ref().map(|fmt| ExtractionConfig {
        output_format: fmt.clone(),
        ..Default::default()
    });

    let result = perform_ocr(image_bytes, config, api_pool, extraction_config.as_ref())?;

    if config.use_cache
        && let Err(e) = cache.set_cached_result(&image_hash, "tesseract", &config_str, output_format.as_ref(), &result)
    {
        tracing::warn!(error = %e, "Failed to cache the OCR result; the next identical request will re-run OCR");
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
#[cfg(test)]
pub(super) fn process_image_files_batch(
    file_paths: Vec<String>,
    config: &TesseractConfig,
    cache: &OcrCache,
    api_pool: &Arc<TesseractApiPool>,
) -> Vec<BatchItemResult> {
    #[cfg(not(target_arch = "wasm32"))]
    use rayon::prelude::*;

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
                match process_image_resolved(&image_bytes, config, cache, api_pool, None) {
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
                match process_image_resolved(&image_bytes, config, cache, api_pool, None) {
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
    use crate::ocr::hocr_parser::{
        HOCR_FONT_SIZE_ATTRIBUTE, parse_hocr_to_internal_document_with_page_offset_and_stats,
    };
    use tempfile::tempdir;

    fn confidence_word(text: &str, confidence: f32) -> xberg_tesseract::WordData {
        xberg_tesseract::WordData {
            text: text.to_string(),
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
            confidence,
            font_attrs: None,
            language: None,
        }
    }

    #[test]
    fn should_reject_table_rebuild_that_drops_words_relative_to_original() {
        let original = "Site inspections this quarter covered fourteen locations across \
            the northern basin and several access roads remain washed out following spring runoff";
        let rebuilt = "| Site | inspections |";

        assert!(
            !should_adopt_table_rebuild(original, rebuilt),
            "a rebuild with far fewer words than the original must not replace it"
        );
    }

    #[test]
    fn should_adopt_table_rebuild_that_retains_or_exceeds_original_word_count() {
        let original = "Name Age\nAlice 30\nBob 40";
        let rebuilt = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 40 |";

        assert!(
            should_adopt_table_rebuild(original, rebuilt),
            "a rebuild that faithfully reformats the same content into a table must still be adopted"
        );
    }

    #[test]
    fn should_publish_only_retained_hocr_word_confidence_metadata() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="x_wconf 20">CLEAR</span>
                    <span class="ocrx_word" title="x_wconf 90">WORDS</span>
                </span>
                <span class="ocr_line">
                    <span class="ocrx_word" title="x_wconf 0">OWATS</span>
                    <span class="ocrx_word" title="x_wconf 0">DNDEVET</span>
                </span>
            </p>
        </div>"#;
        let is_valid_word = |word: &str| Some(matches!(word, "CLEAR" | "WORDS"));
        let filter = DictionaryLineFilter {
            is_valid_word: &is_valid_word,
            max_invalid_ratio: crate::ocr::hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO,
        };
        let result = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, Some(&filter), 1);
        let mut metadata = HashMap::new();

        insert_retained_word_confidence_metadata(&mut metadata, &result.retained_word_confidence_stats);

        assert_eq!(flatten_hocr_elements_to_text(&result.document.elements), "CLEAR WORDS");
        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(2)));
        assert_eq!(metadata.get("mean_text_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("median_word_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("p10_word_conf"), Some(&serde_json::json!(20)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(1)));
        assert_eq!(metadata.len(), 5);
    }

    /// GH#1554 regression: `perform_ocr` must use the caller's `ExtractionConfig.security_limits`
    /// rather than always decoding under `SecurityLimits::default()`, which silently refused
    /// ordinary high-DPI scans a caller had explicitly configured a higher limit to permit.
    #[test]
    fn should_use_configured_security_limits_when_extraction_config_present() {
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: 200 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = security_limits_for_ocr(Some(&config));

        assert_eq!(resolved.max_content_size, 200 * 1024 * 1024);
    }

    /// `None` (no `ExtractionConfig` reached the call) must fall back to
    /// `SecurityLimits::default()`, not to an unbounded/disabled check.
    #[test]
    fn should_fall_back_to_default_security_limits_when_extraction_config_absent() {
        let resolved = security_limits_for_ocr(None);

        assert_eq!(resolved.max_content_size, SecurityLimits::default().max_content_size);
    }

    #[test]
    fn should_omit_hocr_confidence_quantiles_when_every_word_is_rejected() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="x_wconf 10">OWATS</span>
                    <span class="ocrx_word" title="x_wconf 90">DNDEVET</span>
                </span>
            </p>
        </div>"#;
        let always_invalid = |_: &str| Some(false);
        let filter = DictionaryLineFilter {
            is_valid_word: &always_invalid,
            max_invalid_ratio: crate::ocr::hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO,
        };
        let result = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, Some(&filter), 1);
        let mut metadata = HashMap::new();

        insert_retained_word_confidence_metadata(&mut metadata, &result.retained_word_confidence_stats);

        assert!(flatten_hocr_elements_to_text(&result.document.elements).is_empty());
        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(0)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(0)));
        assert!(!metadata.contains_key("mean_text_conf"));
        assert!(!metadata.contains_key("median_word_conf"));
        assert!(!metadata.contains_key("p10_word_conf"));
        assert_eq!(metadata.len(), 2);
    }

    #[test]
    fn should_publish_zero_text_confidence_words_when_every_iterator_word_is_filtered() {
        let words = [confidence_word("\u{001f}", 95.0)];
        let stats = retained_text_word_confidence_stats("", &words);
        let mut metadata = HashMap::from([
            ("word_count".to_string(), serde_json::json!(1)),
            ("low_conf_word_count".to_string(), serde_json::json!(0)),
            ("mean_text_conf".to_string(), serde_json::json!(95)),
            ("median_word_conf".to_string(), serde_json::json!(95)),
            ("p10_word_conf".to_string(), serde_json::json!(95)),
        ]);

        insert_retained_word_confidence_metadata(&mut metadata, &stats);

        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(0)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(0)));
        assert!(!metadata.contains_key("mean_text_conf"));
        assert!(!metadata.contains_key("median_word_conf"));
        assert!(!metadata.contains_key("p10_word_conf"));
    }

    #[test]
    fn should_publish_only_partially_retained_text_word_confidences() {
        let words = [
            confidence_word("RETAINED", 20.0),
            confidence_word("\u{001f}", 95.0),
            confidence_word("WORDS", 90.0),
        ];
        let stats = retained_text_word_confidence_stats("RETAINED WORDS", &words);
        let mut metadata = HashMap::from([
            ("word_count".to_string(), serde_json::json!(3)),
            ("low_conf_word_count".to_string(), serde_json::json!(1)),
            ("mean_text_conf".to_string(), serde_json::json!(68)),
            ("median_word_conf".to_string(), serde_json::json!(90)),
            ("p10_word_conf".to_string(), serde_json::json!(20)),
        ]);

        insert_retained_word_confidence_metadata(&mut metadata, &stats);

        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(2)));
        assert_eq!(metadata.get("mean_text_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("median_word_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("p10_word_conf"), Some(&serde_json::json!(20)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn should_align_words_after_a_retained_iterator_gap() {
        let words = [confidence_word("FIRST", 10.0), confidence_word("LAST", 90.0)];

        let stats = retained_text_word_confidence_stats("FIRST OMITTED LAST", &words);

        assert_eq!(stats.word_count(), 2);
        assert_eq!(stats.mean(), Some(50));
        assert_eq!(stats.median(), Some(50));
        assert_eq!(stats.p10(), Some(10));
        assert_eq!(stats.low_confidence_word_count(), 1);
    }

    #[test]
    fn should_align_repeated_words_without_losing_later_words() {
        let words = [confidence_word("A", 20.0), confidence_word("B", 80.0)];

        let stats = retained_text_word_confidence_stats("A A B", &words);

        assert_eq!(stats.word_count(), 2);
        assert_eq!(stats.mean(), Some(50));
        assert_eq!(stats.median(), Some(50));
        assert_eq!(stats.p10(), Some(20));
        assert_eq!(stats.low_confidence_word_count(), 1);
    }

    #[test]
    fn should_align_punctuation_and_unicode_exactly() {
        let words = [
            confidence_word("Hello,", 60.0),
            confidence_word("Gr\u{00fc}\u{00df}e", 70.0),
            confidence_word("\u{4e16}\u{754c}!", 80.0),
        ];

        let stats = retained_text_word_confidence_stats("Hello, Gr\u{00fc}\u{00df}e \u{4e16}\u{754c}!", &words);

        assert_eq!(stats.word_count(), 3);
        assert_eq!(stats.mean(), Some(70));
        assert_eq!(stats.median(), Some(70));
        assert_eq!(stats.p10(), Some(60));
        assert_eq!(stats.low_confidence_word_count(), 0);
    }

    #[test]
    fn should_align_multi_token_words_across_whitespace_variants() {
        let words = [confidence_word("ALPHA\tBETA", 75.0), confidence_word("GAMMA", 85.0)];

        let stats = retained_text_word_confidence_stats("ALPHA  \n BETA\r\nGAMMA", &words);

        assert_eq!(stats.word_count(), 2);
        assert_eq!(stats.mean(), Some(80));
        assert_eq!(stats.median(), Some(80));
        assert_eq!(stats.p10(), Some(75));
        assert_eq!(stats.low_confidence_word_count(), 0);
    }

    /// Exact count: a known number of skipped words must produce exactly the
    /// matching `word_iterator_skipped_count` value, not just a truthy presence
    /// (#192 — catches off-by-one or double-counting regressions).
    #[test]
    fn should_insert_exact_skipped_count_when_words_were_skipped() {
        let mut metadata = HashMap::new();

        insert_word_iterator_skipped_count_metadata(&mut metadata, 3);

        assert_eq!(
            metadata.get("word_iterator_skipped_count"),
            Some(&serde_json::Value::Number(3.into()))
        );
    }

    /// When nothing was skipped, the metadata key must be entirely ABSENT, not
    /// present with a value of `0` — callers rely on key absence as the
    /// clean-extraction signal (#192).
    #[test]
    fn should_omit_skipped_count_key_when_nothing_was_skipped() {
        let mut metadata = HashMap::new();

        insert_word_iterator_skipped_count_metadata(&mut metadata, 0);

        assert!(
            !metadata.contains_key("word_iterator_skipped_count"),
            "key must be absent, not present with value 0"
        );
        assert!(metadata.is_empty());
    }

    /// When `auto_rotate` is not requested, the flag must be `false`
    /// regardless of which build compiled this test (#309).
    #[test]
    fn should_report_auto_rotate_available_when_not_requested() {
        assert!(!is_auto_rotate_requested_but_unavailable(false));
    }

    /// When `auto_rotate` is requested, the flag reflects whether *this* build
    /// has the `auto-rotate` feature compiled in — `true` (unavailable) on a
    /// build without it, `false` (available) on a build with it (#309).
    #[test]
    fn should_report_auto_rotate_unavailable_only_without_the_feature() {
        assert_eq!(is_auto_rotate_requested_but_unavailable(true), cfg!(not(auto_rotate)));
    }

    fn dict_word(text: &str) -> xberg_tesseract::WordData {
        xberg_tesseract::WordData {
            text: text.to_string(),
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
            confidence: 95.0,
            font_attrs: None,
            language: None,
        }
    }

    /// A page dominated by non-dictionary "words" (the LAAALDLI-style noise a scanned
    /// drawing produces) must report a HIGH invalid ratio, not `None` and not a low one.
    #[test]
    fn should_report_high_ratio_when_most_words_are_dictionary_invalid() {
        let words = ["LAAALDLI", "sky", "AEA", "ails", "Bri", "ENT", "FRONT", "ELEVATION"].map(dict_word);
        // Mirror the ordinance_2197 page-13 evidence: only FRONT/ELEVATION are real words.
        let is_valid = |text: &str| Some(matches!(text, "FRONT" | "ELEVATION"));

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(
            ratio, 0.75,
            "6 of 8 candidates are dictionary-invalid, expected exactly 0.75"
        );
    }

    /// A page of genuine prose (all real dictionary words) must report a LOW invalid
    /// ratio, not one indistinguishable from the noise case above.
    #[test]
    fn should_report_low_ratio_when_all_words_are_dictionary_valid() {
        let words = ["EXHIBIT", "PLANT", "LIST", "DATE", "November", "Nineteen"].map(dict_word);
        let is_valid = |_: &str| Some(true);

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(ratio, 0.0, "an all-valid page must score exactly 0.0, not merely 'low'");
    }

    /// Words below `MIN_WORD_LEN_FOR_DICT_CHECK` or containing non-alphabetic characters
    /// (numbers, punctuation) are never dictionary words regardless of legitimacy, so they
    /// must not count as candidates at all -- scoring them would bias real content toward
    /// "invalid".
    #[test]
    fn should_exclude_short_and_non_alphabetic_words_from_candidates() {
        let words = ["12", "a", "Bo", "3.14", "PLANT", "LIST", "DATE", "November", "Nineteen"].map(dict_word);
        let is_valid = |_: &str| Some(true);

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(
            ratio, 0.0,
            "only the 5 alphabetic 3+ char words should count as candidates"
        );
    }

    /// Too few dictionary-checkable words must yield `None`, never `0.0` -- a page this
    /// short is not evidence that "every word is valid".
    #[test]
    fn should_return_none_when_too_few_candidates() {
        let words = ["PLANT", "LIST"].map(dict_word);
        let is_valid = |_: &str| Some(true);

        assert_eq!(
            invalid_word_ratio_from(&words, is_valid),
            None,
            "fewer than MIN_DICT_CANDIDATES_FOR_RATIO checkable words must not produce a ratio"
        );
    }

    /// A failed dictionary lookup must be excluded from both the numerator and the
    /// denominator, not silently counted as invalid (which would bias the ratio) or as
    /// valid (which would hide real noise).
    #[test]
    fn should_exclude_failed_lookups_from_candidates() {
        let words = ["PLANT", "LIST", "DATE", "November", "Nineteen", "BROKEN"].map(dict_word);
        let is_valid = |text: &str| if text == "BROKEN" { None } else { Some(true) };

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(ratio, 0.0, "the failed lookup must not count as invalid");
    }

    fn word_at(left: u32, top: u32, width: u32, height: u32, text: &str) -> crate::table_core::HocrWord {
        crate::table_core::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    /// Builds a grid of `rows * cols` words starting at `(left, top)`, each
    /// cell `40x20` pixels, simulating one table's worth of OCR words.
    fn table_grid_words(left: u32, top: u32, rows: u32, cols: u32) -> Vec<crate::table_core::HocrWord> {
        let mut words = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                words.push(word_at(
                    left + col * 60,
                    top + row * 30,
                    40,
                    20,
                    &format!("r{row}c{col}"),
                ));
            }
        }
        words
    }

    #[test]
    fn cluster_words_into_table_regions_separates_two_distant_tables() {
        // Two 3x3 grids of words (9 words each) far apart vertically: a real
        // page with two independent tables separated by a paragraph of prose.
        let mut words = table_grid_words(0, 0, 3, 3);
        words.extend(table_grid_words(0, 1000, 3, 3));

        let regions = cluster_words_into_table_regions(&words);

        assert_eq!(regions.len(), 2, "two spatially distant grids should form two regions");
        assert_eq!(regions[0].len(), 9);
        assert_eq!(regions[1].len(), 9);
    }

    #[test]
    fn cluster_words_into_table_regions_keeps_one_table_as_a_single_region() {
        let words = table_grid_words(0, 0, 4, 3);

        let regions = cluster_words_into_table_regions(&words);

        assert_eq!(
            regions.len(),
            1,
            "a single table's own row gaps must not be split into regions"
        );
        assert_eq!(regions[0].len(), 12);
    }

    #[test]
    fn cluster_words_into_table_regions_empty_input_yields_no_regions() {
        assert!(cluster_words_into_table_regions(&[]).is_empty());
    }

    fn text_element_with_font_size(text: &str, font_size: Option<f64>) -> crate::types::internal::InternalElement {
        let mut elem = crate::types::internal::InternalElement::text(
            ElementKind::OcrText {
                level: crate::types::OcrElementLevel::Block,
            },
            text,
            0,
        );
        if let Some(size) = font_size {
            elem.attributes
                .get_or_insert_with(Default::default)
                .insert(HOCR_FONT_SIZE_ATTRIBUTE.to_string(), size.to_string());
        }
        elem
    }

    #[test]
    fn flatten_hocr_elements_to_text_never_bakes_heading_syntax_regardless_of_font_size() {
        // Regression test for the `Plain`-format markdown leak: a large-font,
        // short, single-line paragraph used to be promoted to a `## ` heading
        // right in this string, and `OcrExtractionResult::content` (built
        // from this string) is consumed unrendered by some callers, so the
        // markdown syntax leaked into `Plain` output. This must fail against
        // the old behavior, which produced
        // "## Chapter One\n\nBody text at normal size." instead.
        let elements = vec![
            text_element_with_font_size("Chapter One", Some(28.0)),
            text_element_with_font_size("Body text at normal size.", Some(12.0)),
        ];

        let flattened = flatten_hocr_elements_to_text(&elements);

        assert_eq!(flattened, "Chapter One\n\nBody text at normal size.");
        assert!(
            !flattened.contains('#'),
            "flattened OCR text must contain no markdown heading syntax: {flattened:?}"
        );
    }

    fn paragraph_with_bbox(text: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> crate::types::internal::InternalElement {
        let mut elem = crate::types::internal::InternalElement::text(ElementKind::Paragraph, text, 0);
        elem.bbox = Some(crate::types::extraction::BoundingBox { x0, y0, x1, y1 });
        elem
    }

    fn table_at(left: u32, top: u32, right: u32, bottom: u32) -> OcrTable {
        OcrTable {
            cells: vec![vec!["cell".to_string()]],
            markdown: "| cell |".to_string(),
            page_number: 1,
            bounding_box: Some(OcrTableBoundingBox {
                left,
                top,
                right,
                bottom,
            }),
        }
    }

    #[test]
    fn filter_elements_covered_by_tables_drops_paragraph_inside_table_bbox() {
        // A paragraph whose bbox is fully inside (so its centre is inside) a detected
        // table's bbox must be removed -- this is the #1571 duplication itself: the
        // paragraph's words are also the table's cells.
        let elements = vec![paragraph_with_bbox("Apple 50 10 00", 10.0, 10.0, 90.0, 30.0)];
        let tables = vec![table_at(0, 0, 100, 100)];

        let filtered = filter_elements_covered_by_tables(elements, &tables);

        assert!(
            filtered.is_empty(),
            "paragraph centred inside the table bbox must be dropped"
        );
    }

    #[test]
    fn filter_elements_covered_by_tables_keeps_paragraph_adjacent_to_table() {
        // Precision guard (#1571): a paragraph that merely overlaps a table's bbox edge,
        // with its centre outside the bbox, must survive -- the word-centre rule must not
        // over-delete prose that sits next to (not inside) a table.
        let elements = vec![
            paragraph_with_bbox("Vehicle Maintenance Guide", 10.0, 0.0, 90.0, 15.0),
            paragraph_with_bbox("Apple 50 10 00", 10.0, 50.0, 90.0, 70.0),
        ];
        let tables = vec![table_at(0, 40, 100, 140)];

        let filtered = filter_elements_covered_by_tables(elements, &tables);

        assert_eq!(
            filtered.len(),
            1,
            "only the paragraph centred inside the table bbox should be dropped"
        );
        assert_eq!(filtered[0].text, "Vehicle Maintenance Guide");
    }

    #[test]
    fn filter_elements_covered_by_tables_is_noop_without_tables() {
        let elements = vec![paragraph_with_bbox("Apple 50 10 00", 10.0, 10.0, 90.0, 30.0)];

        let filtered = filter_elements_covered_by_tables(elements, &[]);

        assert_eq!(filtered.len(), 1, "no tables detected means nothing should be filtered");
    }

    #[test]
    fn filter_elements_covered_by_tables_keeps_elements_without_bbox() {
        let mut elem = crate::types::internal::InternalElement::text(ElementKind::Paragraph, "no geometry", 0);
        elem.bbox = None;
        let tables = vec![table_at(0, 0, 100, 100)];

        let filtered = filter_elements_covered_by_tables(vec![elem], &tables);

        assert_eq!(
            filtered.len(),
            1,
            "an element with no bbox cannot be tested against a table and must survive"
        );
    }

    #[test]
    fn flatten_hocr_elements_to_text_leaves_content_unchanged_without_font_sizes() {
        let elements = vec![
            text_element_with_font_size("First paragraph.", None),
            text_element_with_font_size("Second paragraph.", None),
        ];

        let markdown = flatten_hocr_elements_to_text(&elements);

        assert_eq!(markdown, "First paragraph.\n\nSecond paragraph.");
    }

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

        let api_pool = TesseractApiPool::new();
        let result = process_image_file_with_cache("/nonexistent/file.png", &config, &cache, &api_pool, None);
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
        let api_pool = TesseractApiPool::new();
        let result = process_image_with_cache(&invalid_data, &config, &cache, &api_pool, None);

        assert!(result.is_err());
    }

    /// Builds a tiny valid PNG so `process_image_with_cache` runs Tesseract on real
    /// (if content-free) image bytes instead of erroring out on invalid data.
    fn tiny_test_png_bytes() -> Vec<u8> {
        use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
        use std::io::Cursor;

        let img: RgbImage = ImageBuffer::from_pixel(32, 32, Rgb([255, 255, 255]));
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();
        bytes
    }

    /// Regression test for #687 PART 2: `TesseractConfig::use_cache = false` must be an
    /// end-to-end bypass, not just a lookup skip — it must neither read a pre-existing
    /// entry nor write a new one. Uses a real (non-mocked) `TesseractAPI`, matching the
    /// pattern of `config::tests::test_apply_tesseract_variables_enables_hocr_font_info`:
    /// gracefully skips when no Tesseract/tessdata is available in this environment.
    ///
    /// This exercises a genuinely new code path (an explicit cache pre-seed followed by a
    /// `use_cache: false` call) that has no equivalent before this change, so there is
    /// nothing "unfixed" to run it against — the bypass plumbing this proves either exists
    /// or the test cannot be written.
    #[test]
    fn process_image_with_cache_does_not_read_or_write_the_cache_when_use_cache_is_false() {
        let api = match xberg_tesseract::TesseractAPI::new() {
            Ok(api) => api,
            Err(_) => return, // no Tesseract/Leptonica available in this environment
        };
        if api.init("", "eng").is_err() {
            return; // no "eng" tessdata available in this environment
        }

        let temp_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let image_bytes = tiny_test_png_bytes();
        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        };

        // Pre-seed the exact entry `process_image_resolved` would look up if caching were
        // enabled, carrying an obviously-wrong marker. If the bypass ever regresses into a
        // read, this is what would come back instead of a fresh OCR result.
        let image_hash = crate::cache::blake3_hash_bytes(&image_bytes);
        let config_str = hash_config(&config);
        let marker = OcrExtractionResult {
            content: "STALE MARKER: use_cache=false must never return this".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
            ocr_elements: None,
            internal_document: None,
        };
        cache
            .set_cached_result(&image_hash, "tesseract", &config_str, None, &marker)
            .unwrap();

        let api_pool = TesseractApiPool::new();
        let result = process_image_with_cache(&image_bytes, &config, &cache, &api_pool, None)
            .expect("OCR on a valid image must succeed");

        assert_ne!(
            result.content, marker.content,
            "use_cache=false must not read the pre-seeded cache entry"
        );

        let stats = cache.get_stats().unwrap();
        assert_eq!(
            stats.total_files, 1,
            "use_cache=false must not write a new cache entry (only the pre-seeded marker file may exist)"
        );
    }

    #[test]
    fn test_preprocess_pix_produces_grayscale_with_valid_resolution() {
        let width = 32;
        let height = 32;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let value = if (x + y) % 2 == 0 { 32 } else { 224 };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }
        let mut pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        pix.set_resolution(300, 300).unwrap();

        let processed = preprocess_pix(pix, &preprocessing_config()).unwrap();

        assert_eq!(processed.width(), width as i32);
        assert_eq!(processed.height(), height as i32);
        assert_eq!(processed.depth(), 1);
        assert_eq!(processed.get_resolution().unwrap(), (300, 300));
    }

    fn preprocessing_fixture() -> (Vec<u8>, u32, u32) {
        const WIDTH: u32 = 640;
        const HEIGHT: u32 = 480;
        const CHANNELS: usize = 3;
        let mut rgb_data = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * CHANNELS);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let background = 176 + ((x * 48) / WIDTH) as u8;
                let skewed_line = (40..=390).step_by(50).any(|line_y| y.abs_diff(line_y + x / 16) <= 2);
                let noise = (x * 37 + y * 19).is_multiple_of(97);
                let value = if noise {
                    16
                } else if skewed_line && (20..620).contains(&x) {
                    72
                } else {
                    background
                };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }

        (rgb_data, WIDTH, HEIGHT)
    }

    fn preprocessing_config() -> crate::types::ImagePreprocessingConfig {
        crate::types::ImagePreprocessingConfig {
            target_dpi: 300,
            auto_rotate: false,
            deskew: false,
            denoise: false,
            contrast_enhance: false,
            binarization_method: "otsu".to_string(),
            invert_colors: false,
        }
    }

    fn preprocess_fixture_with_config(config: &crate::types::ImagePreprocessingConfig) -> xberg_tesseract::Pix {
        let (rgb_data, width, height) = preprocessing_fixture();
        preprocess_rgb_with_config(rgb_data, width, height, config)
    }

    fn preprocess_rgb_with_config(
        rgb_data: Vec<u8>,
        width: u32,
        height: u32,
        config: &crate::types::ImagePreprocessingConfig,
    ) -> xberg_tesseract::Pix {
        let pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        preprocess_pix(pix, config).unwrap()
    }

    fn sampled_pixel_signature(pix: &xberg_tesseract::Pix) -> Vec<u32> {
        const SAMPLE_STEP: usize = 8;
        let mut signature = vec![pix.width() as u32, pix.height() as u32, pix.depth() as u32];
        for y in (0..pix.height()).step_by(SAMPLE_STEP) {
            for x in (0..pix.width()).step_by(SAMPLE_STEP) {
                let sample = pix.clip_rectangle(x, y, 1, 1).unwrap();
                let (mean, _) = sample.grayscale_stats(1, 1).unwrap();
                signature.push(mean.round() as u32);
            }
        }
        signature
    }

    fn binary_pixel_value(pix: &xberg_tesseract::Pix, x: i32, y: i32) -> u32 {
        let sample = pix.clip_rectangle(x, y, 1, 1).unwrap();
        let (mean, _) = sample.grayscale_stats(1, 1).unwrap();
        mean.round() as u32
    }

    #[test]
    fn should_change_preprocessed_pixels_when_deskew_is_enabled() {
        let disabled = preprocessing_config();
        let enabled = crate::types::ImagePreprocessingConfig {
            deskew: true,
            ..disabled.clone()
        };

        let disabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&disabled));
        let enabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&enabled));

        assert_ne!(
            enabled_signature, disabled_signature,
            "deskew must affect the OCR raster"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_denoise_is_enabled() {
        let disabled = preprocessing_config();
        let enabled = crate::types::ImagePreprocessingConfig {
            denoise: true,
            ..disabled.clone()
        };

        let disabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&disabled));
        let enabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&enabled));

        assert_ne!(
            enabled_signature, disabled_signature,
            "denoise must affect the OCR raster"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_contrast_enhance_is_enabled() {
        let disabled = crate::types::ImagePreprocessingConfig {
            binarization_method: "adaptive".to_string(),
            ..preprocessing_config()
        };
        let enabled = crate::types::ImagePreprocessingConfig {
            contrast_enhance: true,
            ..disabled.clone()
        };

        let disabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&disabled));
        let enabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&enabled));

        assert_ne!(
            enabled_signature, disabled_signature,
            "contrast enhancement must affect the OCR raster"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_binarization_method_changes() {
        let otsu = preprocessing_config();
        let adaptive = crate::types::ImagePreprocessingConfig {
            binarization_method: "adaptive".to_string(),
            ..otsu.clone()
        };

        let otsu_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&otsu));
        let adaptive_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&adaptive));

        assert_ne!(
            adaptive_signature, otsu_signature,
            "binarization_method must select a distinct preprocessing operation"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_sauvola_is_selected() {
        const WIDTH: u32 = 256;
        const HEIGHT: u32 = 128;
        let mut rgb_data = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * RGB_CHANNEL_COUNT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let background = 72 + ((x * 168) / WIDTH) as u8;
                let is_text = (20..=100).step_by(20).any(|line| y.abs_diff(line) <= 1) && (8..248).contains(&x);
                let value = if is_text {
                    background.saturating_sub(32)
                } else {
                    background
                };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }
        let otsu = preprocessing_config();
        let sauvola = crate::types::ImagePreprocessingConfig {
            binarization_method: "sauvola".to_string(),
            ..otsu.clone()
        };

        let otsu_signature =
            sampled_pixel_signature(&preprocess_rgb_with_config(rgb_data.clone(), WIDTH, HEIGHT, &otsu));
        let sauvola_signature = sampled_pixel_signature(&preprocess_rgb_with_config(rgb_data, WIDTH, HEIGHT, &sauvola));

        assert_ne!(
            sauvola_signature, otsu_signature,
            "Sauvola must select a distinct operation"
        );
    }

    #[test]
    fn test_prepare_ocr_image_without_config_preserves_shadowed_rgb() {
        let rgb_data = vec![0, 1, 2, 3, 4, 5];

        let prepared = prepare_ocr_image(rgb_data.clone(), 2, 1, None, None, false, None);

        assert_eq!(prepared.data, rgb_data);
        assert_eq!(prepared.width, 2);
        assert_eq!(prepared.height, 1);
        assert_eq!(prepared.source_dpi, RAW_IMAGE_SOURCE_DPI);
        assert!(!prepared.apply_pix_preprocessing);
    }

    #[test]
    fn test_clean_white_rgb_selects_default_preprocessing() {
        const SAMPLE_PIXEL_COUNT: usize = 16;
        let rgb_data = vec![u8::MAX; SAMPLE_PIXEL_COUNT * RGB_CHANNEL_COUNT];

        assert!(should_apply_default_preprocessing(
            &rgb_data,
            SAMPLE_PIXEL_COUNT as u32,
            1
        ));
    }

    const CONTRAST_FIXTURE_WIDTH: u32 = 512;
    const CONTRAST_FIXTURE_HEIGHT: u32 = 128;
    const CONTRAST_FIXTURE_FILL_WIDTH: u32 = 80;
    const PALE_TEXT: [u8; RGB_CHANNEL_COUNT] = [200, 220, 245];
    const DARK_TEXT: [u8; RGB_CHANNEL_COUNT] = [30, 30, 30];
    const BRIGHT_BLUE_FILL: [u8; RGB_CHANNEL_COUNT] = [210, 220, 240];

    fn glyph_mask(x: u32, y: u32, offset: u32) -> bool {
        const TOP: u32 = 48;
        const HEIGHT: u32 = 24;
        const WIDTH: u32 = 20;
        const STROKE: u32 = 2;
        for left in [4 + offset, 30 + offset, 56 + offset] {
            let local_x = x.checked_sub(left);
            let local_y = y.checked_sub(TOP);
            if let (Some(local_x), Some(local_y)) = (local_x, local_y)
                && local_x < WIDTH
                && local_y < HEIGHT
                && (!(STROKE..WIDTH - STROKE).contains(&local_x)
                    || (HEIGHT / 2..HEIGHT / 2 + STROKE).contains(&local_y))
            {
                return true;
            }
        }
        false
    }

    fn contrast_fixture(text: [u8; RGB_CHANNEL_COUNT], fill: bool, offset: u32) -> (Vec<u8>, usize) {
        let mut rgb_data =
            Vec::with_capacity(CONTRAST_FIXTURE_WIDTH as usize * CONTRAST_FIXTURE_HEIGHT as usize * RGB_CHANNEL_COUNT);
        let mut glyph_pixels = 0usize;
        for y in 0..CONTRAST_FIXTURE_HEIGHT {
            for x in 0..CONTRAST_FIXTURE_WIDTH {
                let pixel = if glyph_mask(x, y, offset) {
                    glyph_pixels += 1;
                    text
                } else if fill && x < CONTRAST_FIXTURE_FILL_WIDTH {
                    BRIGHT_BLUE_FILL
                } else {
                    [u8::MAX; RGB_CHANNEL_COUNT]
                };
                rgb_data.extend_from_slice(&pixel);
            }
        }
        (rgb_data, glyph_pixels)
    }

    fn aggregate_foreground_contrast(rgb_data: &[u8]) -> f64 {
        let mut background_sum = 0.0;
        let mut background_count = 0usize;
        let mut foreground_sum = 0.0;
        let mut foreground_count = 0usize;
        for pixel in rgb_data.chunks_exact(RGB_CHANNEL_COUNT) {
            let luminance = pixel.iter().map(|channel| f64::from(*channel)).sum::<f64>()
                / (RGB_CHANNEL_MAX * RGB_CHANNEL_COUNT as f64);
            if luminance >= CLEAN_PAGE_LIGHT_PIXEL_THRESHOLD {
                background_sum += luminance;
                background_count += 1;
            } else {
                foreground_sum += luminance;
                foreground_count += 1;
            }
        }
        background_sum / background_count as f64 - foreground_sum / foreground_count as f64
    }

    #[test]
    fn should_preserve_pale_glyphs_without_implicit_otsu() {
        let (rgb_data, glyph_pixels) = contrast_fixture(PALE_TEXT, false, 0);
        assert_eq!(
            glyph_pixels, 384,
            "fixture must contain the intended nonempty glyph mask"
        );

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
        );

        assert!(!prepared.apply_pix_preprocessing);
        assert!(prepared.preprocessing.is_none());
        assert!(prepared.image_preprocessing.is_none());
        assert_eq!(
            prepared.data, rgb_data,
            "implicit preprocessing must preserve pale glyph pixels"
        );
    }

    #[test]
    fn should_apply_implicit_otsu_to_dark_glyphs_over_bright_fill_at_every_sample_phase() {
        for offset in 0..DEFAULT_PREPROCESSING_SAMPLE_STRIDE as u32 {
            let (rgb_data, glyph_pixels) = contrast_fixture(DARK_TEXT, true, offset);
            assert_eq!(glyph_pixels, 384, "offset {offset} changed the glyph population");
            assert!(
                aggregate_foreground_contrast(&rgb_data) < CLEAN_PAGE_MIN_FOREGROUND_CONTRAST,
                "fixture must fail the legacy aggregate-contrast gate at offset {offset}"
            );

            let prepared = prepare_ocr_image(
                rgb_data,
                CONTRAST_FIXTURE_WIDTH,
                CONTRAST_FIXTURE_HEIGHT,
                None,
                None,
                false,
                Some(300.0),
            );

            assert!(
                prepared.apply_pix_preprocessing,
                "dark glyphs were missed at offset {offset}"
            );
            assert_eq!(prepared.width, CONTRAST_FIXTURE_WIDTH);
            assert_eq!(prepared.height, CONTRAST_FIXTURE_HEIGHT);
            assert_eq!(
                prepared
                    .preprocessing
                    .as_ref()
                    .map(|config| config.binarization_method.as_str()),
                Some("otsu")
            );
        }
    }

    #[test]
    fn should_reject_scattered_speckles_with_the_same_dark_pixel_count_as_glyphs() {
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        let mut speckles = 0usize;
        'rows: for y in (0..CONTRAST_FIXTURE_HEIGHT).step_by(2) {
            for x in (0..CONTRAST_FIXTURE_WIDTH).step_by(DEFAULT_PREPROCESSING_SAMPLE_STRIDE) {
                let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
                speckles += 1;
                if speckles == glyph_pixels {
                    break 'rows;
                }
            }
        }
        assert_eq!(
            speckles, glyph_pixels,
            "control must have the same dark-pixel population"
        );

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
        );
        assert!(
            !prepared.apply_pix_preprocessing,
            "isolated speckles are not text structure"
        );
        assert_eq!(prepared.data, rgb_data);
    }

    #[test]
    fn should_reject_a_contiguous_blob_with_the_same_dark_pixel_count_as_glyphs() {
        const BLOB_WIDTH: u32 = 16;
        const BLOB_HEIGHT: u32 = 24;
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        assert_eq!(BLOB_WIDTH as usize * BLOB_HEIGHT as usize, glyph_pixels);

        for y in 48..48 + BLOB_HEIGHT {
            for x in 4..4 + BLOB_WIDTH {
                let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
            }
        }

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
        );
        assert!(!prepared.apply_pix_preprocessing, "a solid blob is not text structure");
        assert_eq!(prepared.data, rgb_data);
    }

    #[test]
    fn should_preserve_three_narrow_solid_glyphs_with_the_same_dark_pixel_count() {
        const GLYPH_WIDTH: u32 = 4;
        const GLYPH_HEIGHT: u32 = 32;
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        assert_eq!(3 * GLYPH_WIDTH as usize * GLYPH_HEIGHT as usize, glyph_pixels);

        for left in [4, 12, 20] {
            for y in 44..44 + GLYPH_HEIGHT {
                for x in left..left + GLYPH_WIDTH {
                    let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                    rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
                }
            }
        }

        let prepared = prepare_ocr_image(
            rgb_data,
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
        );
        assert!(
            prepared.apply_pix_preprocessing,
            "narrow solid glyphs remain text structure"
        );
    }

    #[test]
    fn should_reject_a_small_defect_among_equal_count_scattered_speckles() {
        const DEFECT_PIXELS: usize = 4;
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        for y in 0..2 {
            for x in 0..2 {
                let pixel_index = (y * CONTRAST_FIXTURE_WIDTH as usize + x) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
            }
        }

        let mut speckles = 0usize;
        'rows: for y in (4..CONTRAST_FIXTURE_HEIGHT).step_by(2) {
            for x in (4..CONTRAST_FIXTURE_WIDTH).step_by(DEFAULT_PREPROCESSING_SAMPLE_STRIDE) {
                let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
                speckles += 1;
                if speckles + DEFECT_PIXELS == glyph_pixels {
                    break 'rows;
                }
            }
        }
        assert_eq!(speckles + DEFECT_PIXELS, glyph_pixels);

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
        );
        assert!(
            !prepared.apply_pix_preprocessing,
            "a small connected defect does not make scattered noise text-like"
        );
        assert_eq!(prepared.data, rgb_data);
    }

    #[test]
    fn should_reject_malformed_or_overflowing_rgb_dimensions() {
        assert!(!should_apply_default_preprocessing(&[u8::MAX; 3], 2, 2));
        assert!(!should_apply_default_preprocessing(&[], u32::MAX, u32::MAX));
    }

    #[test]
    fn should_select_default_preprocessing_for_black_text_on_white_page() {
        const WIDTH: u32 = 80;
        const HEIGHT: u32 = 20;
        const FOREGROUND_WIDTH: u32 = 4;
        let mut rgb_data = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * RGB_CHANNEL_COUNT);
        for _y in 0..HEIGHT {
            for x in 0..WIDTH {
                let pixel = if x < FOREGROUND_WIDTH {
                    [0; RGB_CHANNEL_COUNT]
                } else {
                    [u8::MAX; RGB_CHANNEL_COUNT]
                };
                rgb_data.extend_from_slice(&pixel);
            }
        }

        assert!(should_apply_default_preprocessing(&rgb_data, WIDTH, HEIGHT));
    }

    #[test]
    fn test_prepare_ocr_image_without_config_preprocesses_clean_white_rgb() {
        const WIDTH: u32 = 4;
        const HEIGHT: u32 = 4;
        let rgb_data = vec![u8::MAX; WIDTH as usize * HEIGHT as usize * RGB_CHANNEL_COUNT];

        let prepared = prepare_ocr_image(rgb_data, WIDTH, HEIGHT, None, None, false, None);

        assert!(prepared.apply_pix_preprocessing);
        let preprocessing = prepared
            .preprocessing
            .expect("implicit preprocessing must retain its effective configuration");
        assert!(preprocessing.deskew);
        assert_eq!(preprocessing.binarization_method, "otsu");
    }

    #[test]
    fn test_shadowed_rgb_skips_default_preprocessing() {
        const SAMPLE_PIXEL_COUNT: usize = 16;
        const SHADOWED_CHANNEL_VALUE: u8 = 128;
        let rgb_data = vec![SHADOWED_CHANNEL_VALUE; SAMPLE_PIXEL_COUNT * RGB_CHANNEL_COUNT];

        assert!(!should_apply_default_preprocessing(
            &rgb_data,
            SAMPLE_PIXEL_COUNT as u32,
            1
        ));
    }

    #[test]
    fn test_prepare_ocr_image_with_config_keeps_preprocessing_enabled() {
        let rgb_data = vec![255; 12];
        let preprocessing = crate::types::ImagePreprocessingConfig {
            target_dpi: 72,
            ..Default::default()
        };

        let prepared = prepare_ocr_image(rgb_data, 2, 2, Some(&preprocessing), None, false, None);

        assert!(prepared.apply_pix_preprocessing);
        assert_eq!(prepared.preprocessing.unwrap().target_dpi, 72);
    }

    /// #209: when a caller supplies `ImageExtractionConfig`, its `max_image_dimension`
    /// and `auto_adjust_dpi` must actually reach the DPI-normalization step instead of
    /// being silently replaced by `ImageDpiConfig::default()`.
    #[test]
    fn test_prepare_preprocessed_ocr_image_honours_image_extraction_config_limits() {
        const SOURCE_DIMENSION: u32 = 4;
        let rgb_data = vec![255; SOURCE_DIMENSION as usize * SOURCE_DIMENSION as usize * RGB_CHANNEL_COUNT];
        let preprocessing = crate::types::ImagePreprocessingConfig {
            target_dpi: 300,
            ..Default::default()
        };
        let images_config = crate::core::config::ImageExtractionConfig {
            max_image_dimension: 2,
            auto_adjust_dpi: false,
            ..Default::default()
        };

        let prepared = prepare_preprocessed_ocr_image(
            rgb_data,
            SOURCE_DIMENSION,
            SOURCE_DIMENSION,
            &preprocessing,
            Some(&images_config),
            false,
            None,
        );

        assert_eq!(prepared.width, 2, "max_image_dimension=2 must clamp the resized width");
        assert_eq!(
            prepared.height, 2,
            "max_image_dimension=2 must clamp the resized height"
        );
        let metadata = prepared
            .image_preprocessing
            .expect("successful normalization must retain its metadata");
        assert_eq!(metadata.original_dimensions.width, SOURCE_DIMENSION as usize);
        assert_eq!(metadata.original_dimensions.height, SOURCE_DIMENSION as usize);
        assert_eq!(
            metadata.new_dimensions.as_ref().map(|dimensions| dimensions.width),
            Some(2)
        );
        assert_eq!(
            metadata.new_dimensions.as_ref().map(|dimensions| dimensions.height),
            Some(2)
        );
        assert!(metadata.dimension_clamped);
    }

    /// Pixel width of a US Letter page (612pt wide) rendered at 150 DPI -- an arbitrary
    /// non-72, non-target render resolution exercising `known_source_dpi`, not tied to
    /// whatever DPI the PDF OCR route actually renders at (`effective_pdf_render_dpi`, #1577).
    const LETTER_AT_150_DPI_WIDTH_PX: u32 = 1275;
    /// Pixel height of the same page (792pt tall) at 150 DPI.
    const LETTER_AT_150_DPI_HEIGHT_PX: u32 = 1650;

    /// A clean white Letter-sized raster, the shape the PDF OCR route actually hands over: it
    /// passes `should_apply_default_preprocessing`, so both the explicit and the implicit
    /// preprocessing branch reach DPI normalization.
    fn letter_page_raster_at_150_dpi() -> Vec<u8> {
        vec![u8::MAX; LETTER_AT_150_DPI_WIDTH_PX as usize * LETTER_AT_150_DPI_HEIGHT_PX as usize * RGB_CHANNEL_COUNT]
    }

    /// The defect: a page rendered at 150 DPI was normalized as if it were 72 DPI, so the scale
    /// factor became `target_dpi / 72` instead of `target_dpi / 150`.
    ///
    /// With the real 150 handed in, `calculate_smart_dpi` sees a 8.5x11in page (612x792pt),
    /// finds 300 DPI fits inside `max_image_dimension` (11in * 300 = 3300px <= 4096), and
    /// returns the full 300. The resize is then a clean 2.0x to exactly 2550x3300 with no
    /// dimension clamp, and Tesseract is told 300 — which is what the raster now is.
    ///
    /// Fails against unfixed code: `prepare_ocr_image` has no `known_source_dpi` parameter at
    /// all there, so this does not compile. Restoring the old six-argument call (dropping the
    /// `Some(150.0)`) makes it compile and then fail on the first assertion, because the 72
    /// assumption yields `final_dpi = 179` and a 3165x4096 clamped raster:
    /// `assertion \`left == right\` failed: left: 4096, right: 3300`.
    #[test]
    fn should_honour_known_source_dpi_instead_of_assuming_72() {
        const KNOWN_RENDER_DPI: f64 = 150.0;
        const EXPECTED_WIDTH_PX: u32 = 2550;
        const EXPECTED_HEIGHT_PX: u32 = 3300;
        const EXPECTED_SOURCE_DPI: i32 = 300;

        let prepared = prepare_ocr_image(
            letter_page_raster_at_150_dpi(),
            LETTER_AT_150_DPI_WIDTH_PX,
            LETTER_AT_150_DPI_HEIGHT_PX,
            Some(&crate::types::ImagePreprocessingConfig::default()),
            None,
            false,
            Some(KNOWN_RENDER_DPI),
        );

        assert_eq!(
            prepared.height, EXPECTED_HEIGHT_PX,
            "a 150 DPI Letter page scaled to the 300 DPI target is 3300px tall, not the 4096px \
             the max_image_dimension clamp produces when the source is mistaken for 72 DPI"
        );
        assert_eq!(
            prepared.width, EXPECTED_WIDTH_PX,
            "the matching width for a clean 2.0x resize"
        );
        assert_eq!(
            prepared.source_dpi, EXPECTED_SOURCE_DPI,
            "Tesseract must be told the raster's real resolution, not the 179 the 72 assumption \
             derives"
        );
    }

    /// The contrast leg, pinning the arithmetic the defect produced so a regression is visible
    /// rather than merely different: with no known source DPI the 72 assumption still applies,
    /// the smart-DPI step derives 179 from an apparent 17.7x22.9in page, and the resize runs
    /// into the 4096px `max_image_dimension` clamp.
    ///
    /// Fails against unfixed code only by not compiling (the seventh argument does not exist);
    /// its values are identical either way, which is the point — this leg proves the fix changed
    /// nothing for callers that do not supply a DPI.
    #[test]
    fn should_keep_assuming_72_dpi_when_source_dpi_is_unknown() {
        const CLAMPED_DIMENSION_PX: u32 = 4096;
        const DERIVED_SOURCE_DPI: i32 = 179;

        let prepared = prepare_ocr_image(
            letter_page_raster_at_150_dpi(),
            LETTER_AT_150_DPI_WIDTH_PX,
            LETTER_AT_150_DPI_HEIGHT_PX,
            Some(&crate::types::ImagePreprocessingConfig::default()),
            None,
            false,
            None,
        );

        assert_eq!(
            prepared.height, CLAMPED_DIMENSION_PX,
            "without a known source DPI the 72 assumption drives the resize into the clamp"
        );
        assert_eq!(
            prepared.source_dpi, DERIVED_SOURCE_DPI,
            "and reports the DPI that assumption implies"
        );
    }

    /// A raw image the caller knows nothing about still gets the 72 fallback on the
    /// pass-through branch, so standalone image OCR is untouched by the new parameter.
    ///
    /// Fails against unfixed code by not compiling; behaviourally it is the pre-existing
    /// contract, asserted here so the new parameter cannot quietly displace it.
    #[test]
    fn should_default_to_72_dpi_for_unpreprocessed_raw_image_without_known_dpi() {
        let rgb_data = vec![0, 1, 2, 3, 4, 5];

        let prepared = prepare_ocr_image(rgb_data.clone(), 2, 1, None, None, false, None);

        assert_eq!(prepared.data, rgb_data, "the raster must pass through untouched");
        assert_eq!(prepared.source_dpi, RAW_IMAGE_SOURCE_DPI);
    }

    /// The same pass-through branch reports a known resolution verbatim rather than the 72
    /// fallback: the bytes are unchanged, so their resolution is whatever the caller measured.
    ///
    /// Fails against unfixed code: there is no way to express this at all — `prepare_ocr_image`
    /// hardcodes `RAW_IMAGE_SOURCE_DPI` on this branch, so the reported DPI is 72 and the
    /// assertion reads `assertion \`left == right\` failed: left: 72, right: 150`.
    #[test]
    fn should_report_known_source_dpi_on_the_unpreprocessed_branch() {
        const KNOWN_RENDER_DPI: f64 = 150.0;
        let rgb_data = vec![0, 1, 2, 3, 4, 5];

        let prepared = prepare_ocr_image(rgb_data, 2, 1, None, None, false, Some(KNOWN_RENDER_DPI));

        assert_eq!(prepared.source_dpi, 150);
        assert!(!prepared.apply_pix_preprocessing);
    }

    // GH#1630: standalone image OCR discarded PNG `pHYs` density and always assumed 72 DPI,
    // resampling a genuine 300 DPI scan even when the requested `target_dpi` was already 300.
    mod png_source_dpi {
        use super::*;

        /// Standard PNG CRC-32 (polynomial 0xEDB88320), needed to splice a well-formed `pHYs`
        /// chunk into a real encoded PNG.
        fn png_crc32(data: &[u8]) -> u32 {
            let mut crc: u32 = 0xFFFF_FFFF;
            for &byte in data {
                crc ^= u32::from(byte);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }

        fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut body = Vec::with_capacity(4 + data.len());
            body.extend_from_slice(chunk_type);
            body.extend_from_slice(data);
            let mut out = Vec::with_capacity(8 + body.len());
            out.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_be_bytes());
            out.extend_from_slice(&body);
            out.extend_from_slice(&png_crc32(&body).to_be_bytes());
            out
        }

        fn encode_png(width: u32, height: u32) -> Vec<u8> {
            let img = image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]));
            let mut png = Vec::new();
            {
                use image::ImageEncoder;
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgb8)
                    .expect("encode PNG");
            }
            png
        }

        /// Splice a `pHYs` chunk expressing `ppu` pixels-per-metre (both axes, unit = meter)
        /// into a real encoded PNG, immediately after IHDR (signature 8 bytes + IHDR chunk 25
        /// bytes) and always before IDAT.
        fn png_with_phys_density(width: u32, height: u32, ppu: u32) -> Vec<u8> {
            const SIGNATURE_AND_IHDR_LEN: usize = 8 + 25;
            let mut phys_data = Vec::with_capacity(9);
            phys_data.extend_from_slice(&ppu.to_be_bytes());
            phys_data.extend_from_slice(&ppu.to_be_bytes());
            phys_data.push(1); // unit = meter
            let phys_chunk = png_chunk(b"pHYs", &phys_data);

            let png = encode_png(width, height);
            let mut out = Vec::with_capacity(png.len() + phys_chunk.len());
            out.extend_from_slice(&png[..SIGNATURE_AND_IHDR_LEN]);
            out.extend_from_slice(&phys_chunk);
            out.extend_from_slice(&png[SIGNATURE_AND_IHDR_LEN..]);
            out
        }

        /// 11811 px/m is the reporter's fixture value: 11811 * 0.0254 = 299.9994 DPI, not an
        /// exact 300 — every assertion below tolerates that rather than requiring an exact
        /// integer match.
        const REPORTER_FIXTURE_PPU: u32 = 11_811;
        const EXPECTED_DPI_FROM_REPORTER_FIXTURE: f64 = 299.999_4;
        const DPI_TOLERANCE: f64 = 0.001;
        const TEST_IMAGE_SIDE: u32 = 4;

        fn rgb_fixture() -> Vec<u8> {
            vec![255u8; TEST_IMAGE_SIDE as usize * TEST_IMAGE_SIDE as usize * RGB_CHANNEL_COUNT]
        }

        fn preprocessing_targeting(target_dpi: i32) -> crate::types::ImagePreprocessingConfig {
            crate::types::ImagePreprocessingConfig {
                target_dpi,
                ..Default::default()
            }
        }

        fn images_config_without_auto_adjust() -> crate::core::config::ImageExtractionConfig {
            crate::core::config::ImageExtractionConfig {
                auto_adjust_dpi: false,
                ..Default::default()
            }
        }

        #[test]
        fn should_prefer_explicit_source_dpi_over_embedded_png_density() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);

            let resolved = resolve_known_source_dpi(Some(150.0), &png);

            assert_eq!(
                resolved,
                Some(150.0),
                "a caller-supplied source_dpi hint must win over embedded PNG metadata"
            );
        }

        #[test]
        fn should_decode_source_density_from_embedded_png_metadata() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);

            let resolved = resolve_known_source_dpi(None, &png).expect("pHYs density must be detected");

            assert!(
                (resolved - EXPECTED_DPI_FROM_REPORTER_FIXTURE).abs() < DPI_TOLERANCE,
                "expected ~{EXPECTED_DPI_FROM_REPORTER_FIXTURE} DPI, got {resolved}"
            );
        }

        #[test]
        fn should_resolve_none_when_neither_explicit_hint_nor_embedded_density_exist() {
            let png = encode_png(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);

            assert_eq!(
                resolve_known_source_dpi(None, &png),
                None,
                "a PNG without density metadata must not fabricate a source DPI"
            );
        }

        /// No resize when the requested target already matches the embedded source density: the
        /// GH#1630 reporter's own repro (`target_dpi=300` against a genuine 300 DPI scan).
        #[test]
        fn should_skip_resize_when_target_dpi_matches_known_png_density() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);
            let known_source_dpi = resolve_known_source_dpi(None, &png);
            let images_config = images_config_without_auto_adjust();
            let preprocessing = preprocessing_targeting(300);

            let prepared = prepare_ocr_image(
                rgb_fixture(),
                TEST_IMAGE_SIDE,
                TEST_IMAGE_SIDE,
                Some(&preprocessing),
                Some(&images_config),
                false,
                known_source_dpi,
            );

            assert_eq!(prepared.width, TEST_IMAGE_SIDE);
            assert_eq!(prepared.height, TEST_IMAGE_SIDE);
            let metadata = prepared
                .image_preprocessing
                .expect("preprocessing metadata is recorded");
            assert!(
                metadata.skipped_resize,
                "a 300 target against a ~300 detected source must not resize"
            );
        }

        /// A different target DPI still resizes correctly once the true source density is known,
        /// rather than scaling from the wrong 72 assumption.
        #[test]
        fn should_resize_correctly_for_a_different_target_dpi() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);
            let known_source_dpi = resolve_known_source_dpi(None, &png);
            let images_config = images_config_without_auto_adjust();
            let preprocessing = preprocessing_targeting(150);

            let prepared = prepare_ocr_image(
                rgb_fixture(),
                TEST_IMAGE_SIDE,
                TEST_IMAGE_SIDE,
                Some(&preprocessing),
                Some(&images_config),
                false,
                known_source_dpi,
            );

            assert_eq!(
                prepared.width, 2,
                "halving from a ~300 DPI source to a 150 DPI target halves the raster"
            );
            assert_eq!(prepared.height, 2);
        }

        /// Control: a PNG carrying no density metadata must keep defaulting to 72 DPI, exactly
        /// as before this fix.
        #[test]
        fn should_default_to_72_dpi_when_png_has_no_density_metadata() {
            let png = encode_png(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);
            let known_source_dpi = resolve_known_source_dpi(None, &png);
            assert_eq!(known_source_dpi, None);

            let images_config = images_config_without_auto_adjust();
            let preprocessing = preprocessing_targeting(300);
            let prepared = prepare_ocr_image(
                rgb_fixture(),
                TEST_IMAGE_SIDE,
                TEST_IMAGE_SIDE,
                Some(&preprocessing),
                Some(&images_config),
                false,
                known_source_dpi,
            );

            let metadata = prepared
                .image_preprocessing
                .expect("preprocessing metadata is recorded");
            assert_eq!(
                metadata.original_dpi,
                crate::types::ImageDpi::from((f64::from(RAW_IMAGE_SOURCE_DPI), f64::from(RAW_IMAGE_SOURCE_DPI))),
                "no embedded metadata must leave the historical 72 assumption unchanged"
            );
        }
    }

    #[test]
    fn test_should_invert_for_polarity_dark_background_with_text() {
        // Dark background (mean well below threshold) with a clear light-text
        // fraction should trigger auto-detected inversion.
        assert!(should_invert_for_polarity(40.0, 0.05, false));
    }

    #[test]
    fn test_should_invert_for_polarity_light_background_not_inverted() {
        // Typical dark-text-on-light-paper scan: high mean, no auto-invert.
        assert!(!should_invert_for_polarity(220.0, 0.9, false));
    }

    #[test]
    fn test_should_invert_for_polarity_uniformly_dark_image_not_inverted() {
        // Dark mean but essentially no light pixels: a uniformly dark, non-text
        // image (e.g. a dark photo) should NOT be auto-inverted.
        assert!(!should_invert_for_polarity(20.0, 0.0, false));
    }

    #[test]
    fn test_should_invert_for_polarity_force_true_overrides_light_background() {
        // Explicit invert_colors=true forces inversion even on a light-mean image.
        assert!(should_invert_for_polarity(220.0, 0.9, true));
    }

    #[test]
    fn test_should_invert_for_polarity_force_false_still_auto_detects() {
        assert!(should_invert_for_polarity(10.0, 0.5, false));
    }

    #[test]
    fn test_preprocess_pix_inverts_light_on_dark_image() {
        let width = 40u32;
        let height = 40u32;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_text = (10..13).contains(&y) && (5..25).contains(&x);
                let value = if is_text { 230u8 } else { 20u8 };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }

        let pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        let processed = preprocess_pix(pix, &preprocessing_config()).unwrap();

        assert_eq!(processed.depth(), 1);
        assert_eq!(binary_pixel_value(&processed, 0, 0), 0, "background must be white");
        assert_eq!(binary_pixel_value(&processed, 10, 11), 1, "text must be black");
    }

    #[test]
    fn test_preprocess_pix_does_not_invert_dark_on_light_image() {
        let width = 40u32;
        let height = 40u32;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_text = (10..13).contains(&y) && (5..25).contains(&x);
                let value = if is_text { 20u8 } else { 230u8 };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }

        let pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        let processed = preprocess_pix(pix, &preprocessing_config()).unwrap();

        assert_eq!(processed.depth(), 1);
        assert_eq!(binary_pixel_value(&processed, 0, 0), 0, "background must be white");
        assert_eq!(binary_pixel_value(&processed, 10, 11), 1, "text must be black");
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_identity() {
        let data: Vec<u8> = (0..18).collect();
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 3, 0);
        assert_eq!(out, data);
        assert_eq!(w, 2);
        assert_eq!(h, 3);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_180() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 2, 180);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(out, vec![10, 11, 12, 7, 8, 9, 4, 5, 6, 1, 2, 3]);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_90_swaps_dimensions() {
        let data: Vec<u8> = (0..18).collect();
        let (_, w, h) = rotate_rgb_image_data(&data, 2, 3, 90);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_270_swaps_dimensions() {
        let data: Vec<u8> = (0..18).collect();
        let (_, w, h) = rotate_rgb_image_data(&data, 2, 3, 270);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_90_then_270_is_identity() {
        let data: Vec<u8> = (0..18).collect();
        let (rotated_90, w1, h1) = rotate_rgb_image_data(&data, 2, 3, 90);
        let (back, w2, h2) = rotate_rgb_image_data(&rotated_90, w1, h1, 270);
        assert_eq!(w2, 2);
        assert_eq!(h2, 3);
        assert_eq!(back, data);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_unsupported_angle() {
        let data: Vec<u8> = (0..12).collect();
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 2, 45);
        assert_eq!(out, data);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
    }
}
