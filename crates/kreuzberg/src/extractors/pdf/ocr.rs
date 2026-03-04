//! OCR functionality for PDF extraction.
//!
//! Handles text quality evaluation, OCR fallback decision logic, and OCR processing.

#[cfg(feature = "ocr")]
use crate::core::config::ExtractionConfig;

#[cfg(feature = "ocr")]
pub(crate) const MIN_TOTAL_NON_WHITESPACE: usize = 64;
#[cfg(feature = "ocr")]
pub(crate) const MIN_NON_WHITESPACE_PER_PAGE: f64 = 32.0;
#[cfg(feature = "ocr")]
pub(crate) const MIN_MEANINGFUL_WORD_LEN: usize = 4;
#[cfg(feature = "ocr")]
pub(crate) const MIN_MEANINGFUL_WORDS: usize = 3;
#[cfg(feature = "ocr")]
pub(crate) const MIN_ALNUM_RATIO: f64 = 0.3;

#[cfg(feature = "ocr")]
pub struct NativeTextStats {
    pub non_whitespace: usize,
    pub alnum: usize,
    pub meaningful_words: usize,
    pub alnum_ratio: f64,
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
    pub fn from(text: &str) -> Self {
        let mut non_whitespace = 0usize;
        let mut alnum = 0usize;

        for ch in text.chars() {
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
                    .take(MIN_MEANINGFUL_WORD_LEN)
                    .count()
                    >= MIN_MEANINGFUL_WORD_LEN
            })
            .take(MIN_MEANINGFUL_WORDS)
            .count();

        let alnum_ratio = if non_whitespace == 0 {
            0.0
        } else {
            alnum as f64 / non_whitespace as f64
        };

        Self {
            non_whitespace,
            alnum,
            meaningful_words,
            alnum_ratio,
        }
    }
}

/// Evaluates native PDF text quality to determine if OCR fallback is needed.
///
/// Analyzes text characteristics (whitespace, alphanumeric ratio, meaningful words)
/// to detect cases where native text extraction produced poor results (e.g., scanned
/// PDFs with garbled text).
///
/// # Arguments
///
/// * `native_text` - The text extracted from the PDF using native methods
/// * `page_count` - Optional page count for per-page average calculations
///
/// # Returns
///
/// An `OcrFallbackDecision` containing:
/// - Statistics about the text quality
/// - Per-page averages
/// - Boolean decision on whether to use OCR
#[cfg(feature = "ocr")]
pub fn evaluate_native_text_for_ocr(native_text: &str, page_count: Option<usize>) -> OcrFallbackDecision {
    let trimmed = native_text.trim();

    if trimmed.is_empty() {
        let empty_stats = NativeTextStats {
            non_whitespace: 0,
            alnum: 0,
            meaningful_words: 0,
            alnum_ratio: 0.0,
        };
        return OcrFallbackDecision {
            stats: empty_stats,
            avg_non_whitespace: 0.0,
            avg_alnum: 0.0,
            fallback: true,
        };
    }

    let stats = NativeTextStats::from(trimmed);
    let pages = page_count.unwrap_or(1).max(1) as f64;
    let avg_non_whitespace = stats.non_whitespace as f64 / pages;
    let avg_alnum = stats.alnum as f64 / pages;

    let has_substantial_text = stats.non_whitespace >= MIN_TOTAL_NON_WHITESPACE
        && avg_non_whitespace >= MIN_NON_WHITESPACE_PER_PAGE
        && stats.meaningful_words >= MIN_MEANINGFUL_WORDS;

    let fallback = if stats.non_whitespace == 0 || stats.alnum == 0 {
        true
    } else if has_substantial_text {
        false
    } else if (stats.alnum_ratio < MIN_ALNUM_RATIO && avg_alnum < MIN_NON_WHITESPACE_PER_PAGE)
        || (stats.non_whitespace < MIN_TOTAL_NON_WHITESPACE && avg_non_whitespace < MIN_NON_WHITESPACE_PER_PAGE)
    {
        true
    } else {
        stats.meaningful_words == 0 && avg_non_whitespace < MIN_NON_WHITESPACE_PER_PAGE
    };

    OcrFallbackDecision {
        stats,
        avg_non_whitespace,
        avg_alnum,
        fallback,
    }
}

#[cfg(feature = "ocr")]
pub fn evaluate_per_page_ocr(
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
    page_count: Option<usize>,
) -> OcrFallbackDecision {
    let boundaries = match boundaries {
        Some(b) if !b.is_empty() => b,
        _ => return evaluate_native_text_for_ocr(native_text, page_count),
    };

    let mut document_decision = evaluate_native_text_for_ocr(native_text, page_count);

    for boundary in boundaries {
        if boundary.byte_end > native_text.len() || boundary.byte_start > boundary.byte_end {
            continue;
        }
        let page_text = &native_text[boundary.byte_start..boundary.byte_end];
        if evaluate_native_text_for_ocr(page_text, Some(1)).fallback {
            document_decision.fallback = true;
            return document_decision;
        }
    }

    document_decision
}

/// Extract text and tables from PDF using OCR.
///
/// Renders all pages to images and processes them with OCR backend.
///
/// # Arguments
///
/// * `content` - Raw PDF bytes
/// * `config` - Extraction configuration including OCR settings
///
/// # Returns
///
/// A tuple of (concatenated text, collected tables) from all pages
#[cfg(feature = "ocr")]
pub(crate) async fn extract_with_ocr(
    content: &[u8],
    config: &ExtractionConfig,
) -> crate::Result<(String, Vec<crate::types::Table>)> {
    use crate::pdf::rendering::{PageRenderOptions, PdfRenderer};
    use crate::plugins::registry::get_ocr_backend_registry;
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use std::io::Cursor;

    let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
        message: "OCR config required for force_ocr".to_string(),
        source: None,
    })?;

    let backend = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire read lock on OCR backend registry: {}", e),
            plugin_name: "ocr-registry".to_string(),
        })?;
        registry.get(&ocr_config.backend)?
    };

    let images = {
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
            })?
    };

    let mut page_texts = Vec::with_capacity(images.len());
    let mut all_tables = Vec::new();

    for (page_idx, image) in images.iter().enumerate() {
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

        let image_data = image_bytes.into_inner();

        let ocr_result = backend.process_image(&image_data, ocr_config).await?;

        page_texts.push(ocr_result.content);

        // Collect tables from OCR result, assigning correct 1-indexed page numbers
        for mut table in ocr_result.tables {
            table.page_number = page_idx + 1;
            all_tables.push(table);
        }
    }

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
    Ok((result, all_tables))
}
