use crate::core::config::ExtractionConfig;
use crate::ocr::processor::validation::{TessdataEnv, existing_tessdata_dir_for};
use crate::ocr::processor::{ImageOcrPolicy, OcrProcessor};
use crate::ocr::types::TesseractConfig;
use crate::pdf::structure::types::{PdfLine, PdfParagraph};
use xberg_native_pdf::{PdfDocument, geometry::Rect, layout::TextChar};

const POINTS_PER_INCH: f32 = 72.0;
const MAX_NUMBER_GLYPHS: usize = 64;
const MAX_NATIVE_CHARS: usize = 32_768;
const MAX_CROP_PIXELS: u64 = 1024 * 1024;
const CROP_BORDER: u32 = 8;

mod read_proof;
pub(super) use read_proof::ReadProof;

struct Number {
    text: String,
    bbox: Rect,
    baseline: f32,
}

#[derive(Default)]
pub(super) struct Run {
    processor: once_cell::sync::OnceCell<OcrProcessor>,
}

pub(super) struct PagePolicy<'a> {
    pub(super) accepted: bool,
    pub(super) scale_only: bool,
    pub(super) deferred_structure: bool,
    pub(super) layout: bool,
    pub(super) transformed: bool,
    pub(super) raster: Option<&'a crate::pdf::render::OcrRasterDpi>,
    pub(super) structured: bool,
    pub(super) builtin: bool,
}

pub(super) fn applicable(policy: PagePolicy<'_>) -> bool {
    policy.accepted
        && policy.scale_only
        && !policy.deferred_structure
        && !policy.layout
        && !policy.transformed
        && policy.raster.is_some_and(|raster| !raster.type0_ink_loss)
        && !policy.structured
        && policy.builtin
}

fn is_khmer_digit(character: char) -> bool {
    ('\u{17e0}'..='\u{17e9}').contains(&character)
}

pub(super) struct PageSource<'a> {
    pub(super) document: &'a PdfDocument,
    pub(super) index: usize,
    pub(super) layer: &'a str,
    pub(super) raster: std::sync::Arc<Vec<u8>>,
    pub(super) dimensions: (u32, u32),
    pub(super) metadata: &'a crate::types::Metadata,
}

pub(super) async fn recover(
    source: PageSource<'_>,
    paragraphs: &mut [PdfParagraph],
    read: &mut String,
    config: &ExtractionConfig,
    run: &std::sync::Arc<Run>,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> bool {
    let count = source
        .layer
        .chars()
        .filter(|character| is_khmer_digit(*character))
        .count();
    if !eligible(config, source.index)
        || super::document::resolved_ocr_correction_degrees(source.metadata).is_some_and(|angle| angle != 0)
        || count == 0
        || count > MAX_NUMBER_GLYPHS
        || !super::document::ocr_paragraphs_plain_text(paragraphs)
            .split_whitespace()
            .eq(read.split_whitespace())
    {
        return false;
    }
    // Model presence changes the result even when configuration and PDF bytes do not. ~keep
    crate::core::extractor::cacheability::invalidate_current();
    let Some(directory) = model_directory(config) else {
        missing_model_warning(warnings, source.index);
        return false;
    };
    let (media, numbers) = match source_numbers(&source) {
        Ok(Some(prepared)) => prepared,
        Ok(None) => return false,
        Err(error) => {
            tracing::warn!(%error, "Cannot locate native numeric source crops");
            return false;
        }
    };
    let frame =
        super::document::resolved_ocr_layout_dimensions(source.metadata, source.dimensions.0, source.dimensions.1);
    if frame.0 == 0 || frame.1 == 0 {
        return false;
    }
    let scale = (media.width / frame.0 as f32, media.height / frame.1 as f32);
    let max_work = proof_limits(config).max_iterations;
    let numbers = recognize(source.raster, media, numbers, directory, config.clone(), run.clone()).await;
    let changed = insert(numbers, paragraphs, scale, max_work);
    if changed {
        *read = super::document::ocr_paragraphs_plain_text(paragraphs);
    }
    changed
}

pub(super) fn synchronize_plain_pages(
    document: &crate::types::internal::InternalDocument,
    recovered: &[bool],
    page_texts: &mut [String],
    format: &crate::OutputFormat,
) -> bool {
    if *format != crate::OutputFormat::Plain {
        return false;
    }
    let requested: std::collections::BTreeSet<u32> = recovered
        .iter()
        .enumerate()
        .filter(|(_, changed)| **changed)
        .filter_map(|(index, _)| u32::try_from(index + 1).ok())
        .collect();
    if requested.is_empty() {
        return false;
    }
    let mut rendered = crate::extraction::derive::render_plain_selected_pages(document, &requested);
    for ((index, read), changed) in page_texts.iter_mut().enumerate().zip(recovered) {
        if *changed && let Ok(page) = u32::try_from(index + 1) {
            *read = rendered.remove(&page).unwrap_or_default();
        }
    }
    true
}

fn source_numbers(source: &PageSource<'_>) -> crate::Result<Option<(Rect, Vec<Number>)>> {
    let info = source.document.get_page_info(source.index).map_err(proof_error)?;
    let media = info.media_box;
    if info.rotation != 0 || media.x != 0.0 || media.y != 0.0 || info.crop_box.is_some_and(|crop| crop != media) {
        return Ok(None);
    }
    let chars = source.document.extract_chars(source.index).map_err(proof_error)?;
    if chars.len() > MAX_NATIVE_CHARS {
        return Ok(None);
    }
    Ok(Some((media, candidates(chars))))
}

async fn recognize(
    raster: std::sync::Arc<Vec<u8>>,
    media: Rect,
    numbers: Vec<Number>,
    directory: String,
    config: ExtractionConfig,
    run: std::sync::Arc<Run>,
) -> Vec<Number> {
    match tokio::task::spawn_blocking(move || verify_source(raster, media, numbers, &directory, &config, &run)).await {
        Ok(Ok(numbers)) => numbers,
        Ok(Err(error)) => {
            tracing::warn!(%error, "Unable to verify native numeric source crops");
            Vec::new()
        }
        Err(error) => {
            tracing::warn!(%error, "Native numeric source verification task failed");
            Vec::new()
        }
    }
}

fn eligible(config: &ExtractionConfig, index: usize) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    let Ok(page) = u32::try_from(index.saturating_add(1)) else {
        return false;
    };
    super::document::native_preservation_is_automatic(config, page)
        && config.redaction.is_none()
        && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
        && matches!(
            config.output_format,
            crate::OutputFormat::Plain | crate::OutputFormat::Markdown
        )
        && config.ocr.as_ref().is_some_and(|ocr| {
            ocr.tesseract_config.is_none()
                && ocr.element_config.is_none()
                && ocr.backend_options.is_none()
                && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
        })
}

fn model_directory(config: &ExtractionConfig) -> Option<String> {
    let ocr = config.ocr.as_ref()?;
    let directory = existing_tessdata_dir_for(
        &ocr.effective_tesseract_language(),
        ocr.tessdata_path.as_deref(),
        &TessdataEnv::from_process(),
    )?;
    // An auxiliary model must accompany the selected primary model; never search or download it. ~keep
    std::path::Path::new(&directory)
        .join("khm.traineddata")
        .is_file()
        .then_some(directory)
}

pub(super) fn prepare_read(
    mut primary: crate::core::config::OcrConfig,
    config: &ExtractionConfig,
    page: usize,
    layer: Option<&str>,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> (crate::core::config::OcrConfig, bool) {
    let count = layer.map_or(0, |text| {
        text.chars().filter(|character| is_khmer_digit(*character)).count()
    });
    if !eligible(config, page) || count == 0 || count > MAX_NUMBER_GLYPHS {
        return (primary, false);
    }
    crate::core::extractor::cacheability::invalidate_current();
    if model_directory(config).is_none() {
        missing_model_warning(warnings, page);
        return (primary, false);
    }
    let options = primary.backend_options.get_or_insert_with(|| serde_json::json!({}));
    let Some(options) = options.as_object_mut() else {
        return (primary, false);
    };
    // A cached read cannot certify whether deskew changed this raster. ~keep
    options.insert("use_cache".into(), false.into());
    (primary, true)
}

fn missing_model_warning(warnings: &mut Vec<crate::types::ProcessingWarning>, index: usize) {
    let page = index.saturating_add(1);
    let message = format!(
        "The native text on PDF page {page} contains Khmer-script numbers, but no khm.traineddata accompanies \
         the selected OCR model. Those numbers cannot be verified and may be missing from the OCR text."
    );
    tracing::warn!(page, script = "Khmer", "{message}");
    crate::core::diagnostics::push_warning(warnings, "ocr", message);
}

fn proof_error(error: impl std::error::Error + Send + Sync + 'static) -> crate::XbergError {
    crate::XbergError::Ocr {
        message: "Unable to recognize a native number in its painted source crop".into(),
        source: Some(Box::new(error)),
    }
}

fn candidates(chars: Vec<TextChar>) -> Vec<Number> {
    let mut numbers: Vec<Number> = Vec::new();
    let mut adjacent = false;
    for character in chars {
        if !is_khmer_digit(character.char) || character.rotation_degrees != 0.0 {
            adjacent = false;
            continue;
        }
        if adjacent
            && let Some(last) = numbers.last_mut()
            && last.baseline == character.origin_y
            && character.bbox.x >= last.bbox.x
            && character.bbox.x <= last.bbox.x + last.bbox.width + character.advance_width
        {
            last.text.push(character.char);
            last.bbox = last.bbox.union(&character.bbox);
        } else {
            numbers.push(Number {
                text: character.char.to_string(),
                bbox: character.bbox,
                baseline: character.origin_y,
            });
        }
        adjacent = true;
    }
    numbers
}

fn verify_source(
    encoded: std::sync::Arc<Vec<u8>>,
    media: Rect,
    numbers: Vec<Number>,
    directory: &str,
    config: &ExtractionConfig,
    run: &Run,
) -> crate::Result<Vec<Number>> {
    let limits = proof_limits(config);
    if numbers.is_empty()
        || numbers.iter().map(|number| number.text.chars().count()).sum::<usize>() > MAX_NUMBER_GLYPHS
        || numbers.len() > limits.max_iterations
    {
        return Ok(Vec::new());
    }
    let raster = crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
        &encoded,
        &limits,
        u64::try_from(encoded.capacity()).unwrap_or(u64::MAX),
    )?;
    let raster = image::DynamicImage::ImageRgb8(raster);
    let processor = run
        .processor
        .get_or_try_init(|| OcrProcessor::new(None).map_err(proof_error))?;
    let retained = encoded.capacity().saturating_add(raster.as_bytes().len());
    let mut recognition = recognition_config(
        directory,
        config,
        f64::from(raster.width()) / f64::from(media.width) * f64::from(POINTS_PER_INCH),
    );
    let budget = CropBudget {
        limits: &limits,
        retained,
    };
    let mut verified = Vec::new();
    for number in numbers {
        if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
            break;
        }
        if let Some(read) = recognize_crop(&raster, media, number.bbox, processor, &mut recognition, &budget)?
            && recognized_number_matches(&read.content, &number.text)
        {
            verified.push(number);
        }
    }
    Ok(verified)
}

fn recognized_number_matches(read: &str, number: &str) -> bool {
    read.chars()
        .filter(|character| !character.is_whitespace() && *character != '\u{200b}')
        .eq(number.chars())
}

struct CropBudget<'a> {
    limits: &'a crate::extractors::security::SecurityLimits,
    retained: usize,
}

fn recognize_crop(
    raster: &image::DynamicImage,
    media: Rect,
    bbox: Rect,
    processor: &OcrProcessor,
    recognition: &mut TesseractConfig,
    budget: &CropBudget<'_>,
) -> crate::Result<Option<crate::types::OcrExtractionResult>> {
    let limits = budget.limits;
    let retained = budget.retained;
    let Some(crop) = crop_number(raster, media, bbox, limits.max_content_size.saturating_sub(retained)) else {
        return Ok(None);
    };
    let mut encoded = std::io::Cursor::new(Vec::new());
    crop.write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(proof_error)?;
    drop(crop);
    let Some(remaining) = limits
        .max_content_size
        .checked_sub(retained.saturating_add(encoded.get_ref().capacity()))
    else {
        return Ok(None);
    };
    let mut crop_limits = limits.clone();
    crop_limits.max_content_size = remaining;
    recognition.security_limits = Some(crop_limits);
    processor
        .process_image_with_policy(encoded.get_ref(), recognition, None, ImageOcrPolicy::Default)
        .map(Some)
        .map_err(proof_error)
}

fn proof_limits(config: &ExtractionConfig) -> crate::extractors::security::SecurityLimits {
    let mut limits = config.security_limits.clone().unwrap_or_default();
    if let Some(nested) = config.ocr.as_ref().and_then(|ocr| ocr.security_limits.as_ref()) {
        limits.max_content_size = limits.max_content_size.min(nested.max_content_size);
        limits.max_iterations = limits.max_iterations.min(nested.max_iterations);
    }
    limits
}

fn recognition_config(directory: &str, config: &ExtractionConfig, dpi: f64) -> TesseractConfig {
    TesseractConfig {
        language: "khm".into(),
        tessdata_path: Some(directory.into()),
        psm: 8,
        output_format: "text".into(),
        use_cache: false,
        enable_table_detection: false,
        source_dpi: Some(dpi),
        security_limits: Some(proof_limits(config)),
        ..Default::default()
    }
}

fn crop_number(raster: &image::DynamicImage, media: Rect, bbox: Rect, budget: usize) -> Option<image::DynamicImage> {
    let sx = raster.width() as f32 / media.width;
    let sy = raster.height() as f32 / media.height;
    let bounds = [
        bbox.x * sx,
        (media.height - bbox.y - bbox.height) * sy,
        bbox.width * sx,
        bbox.height * sy,
    ];
    if bounds.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return None;
    }
    let x = bounds[0].floor() as u32;
    let y = bounds[1].floor() as u32;
    let width = (bounds[0] + bounds[2]).ceil() as u32 - x;
    let height = (bounds[1] + bounds[3]).ceil() as u32 - y;
    if width == 0
        || height == 0
        || x.checked_add(width)? > raster.width()
        || y.checked_add(height)? > raster.height()
        || u64::from(width) * u64::from(height) > MAX_CROP_PIXELS
    {
        return None;
    }
    let padded_pixels = u64::from(width + 2 * CROP_BORDER) * u64::from(height + 2 * CROP_BORDER);
    if 3 * (u64::from(width) * u64::from(height) + padded_pixels) > budget as u64 {
        return None;
    }
    let crop = raster.crop_imm(x, y, width, height).into_rgb8();
    let mut padded =
        image::RgbImage::from_pixel(width + 2 * CROP_BORDER, height + 2 * CROP_BORDER, image::Rgb([255; 3]));
    image::imageops::replace(&mut padded, &crop, i64::from(CROP_BORDER), i64::from(CROP_BORDER));
    Some(image::DynamicImage::ImageRgb8(padded))
}

fn insert(numbers: Vec<Number>, paragraphs: &mut [PdfParagraph], scale: (f32, f32), max_work: usize) -> bool {
    if !scale.0.is_finite()
        || !scale.1.is_finite()
        || scale.0 <= 0.0
        || scale.1 <= 0.0
        || paragraphs
            .iter()
            .any(|paragraph| !located_text_covers_paragraph(paragraph))
    {
        return false;
    }
    let lines = paragraphs.iter().map(|paragraph| paragraph.lines.len()).sum::<usize>();
    if !bounded_unique_crops(&numbers, lines, max_work) {
        return false;
    }
    let planned = numbers
        .into_iter()
        .filter_map(|number| {
            let bbox = Rect::new(
                number.bbox.x / scale.0,
                number.bbox.y / scale.1,
                number.bbox.width / scale.0,
                number.bbox.height / scale.1,
            );
            let mut owners =
                paragraphs.iter().enumerate().flat_map(|(paragraph, value)| {
                    value.lines.iter().enumerate().filter_map(move |(line, value)| {
                        line_geometrically_owns(value, bbox).then_some((paragraph, line))
                    })
                });
            let (paragraph, line) = owners.next()?;
            if owners.next().is_some() || !line_accepts_number(&paragraphs[paragraph].lines[line], bbox) {
                return None;
            }
            Some((paragraph, line, number, bbox))
        })
        .collect::<Vec<_>>();
    let changed = !planned.is_empty();
    for (paragraph, line, number, bbox) in planned {
        let paragraph = &mut paragraphs[paragraph];
        let line = &mut paragraph.lines[line];
        let segment = numeric_segment(number, bbox, scale.1, line.dominant_font_size);
        let position = line.segments.partition_point(|existing| existing.x <= bbox.x);
        line.segments.insert(position, segment);
        paragraph.text.clear();
        paragraph.word_count = PdfParagraph::compute_word_count("", &paragraph.lines);
    }
    changed
}

fn numeric_segment(number: Number, bbox: Rect, scale: f32, font_size: f32) -> crate::pdf::hierarchy::SegmentData {
    crate::pdf::hierarchy::SegmentData {
        text: number.text,
        x: bbox.x,
        y: bbox.y,
        width: bbox.width,
        height: bbox.height,
        baseline_y: number.baseline / scale,
        font_size,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn bounded_unique_crops(numbers: &[Number], lines: usize, max_work: usize) -> bool {
    numbers.len() <= MAX_NUMBER_GLYPHS
        && numbers
            .len()
            .checked_mul(lines.saturating_add(numbers.len()))
            .is_some_and(|work| work <= max_work)
        && numbers.iter().enumerate().all(|(index, number)| {
            numbers[..index]
                .iter()
                .all(|other| !number.bbox.intersects(&other.bbox))
        })
}

fn located_text_covers_paragraph(paragraph: &PdfParagraph) -> bool {
    paragraph.text.is_empty()
        || paragraph.text.split_whitespace().eq(paragraph
            .lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .flat_map(|segment| segment.text.split_whitespace()))
}

fn line_accepts_number(line: &PdfLine, bbox: Rect) -> bool {
    if !line.dominant_font_size.is_finite() || line.dominant_font_size <= 0.0 {
        return false;
    }
    if !line.segments.windows(2).all(|pair| pair[0].x <= pair[1].x) {
        return false;
    }
    if line.segments.iter().any(|segment| {
        segment.text.chars().any(char::is_numeric)
            || (segment.text.split_whitespace().nth(1).is_some()
                && segment.x < bbox.x + bbox.width
                && bbox.x < segment.x + segment.width)
    }) {
        return false;
    }
    true
}

fn line_geometrically_owns(line: &PdfLine, bbox: Rect) -> bool {
    let Some(first) = line.segments.first() else {
        return false;
    };
    let bounds = line.segments.iter().fold(
        Rect::new(first.x, first.y, first.width, first.height),
        |bounds, segment| bounds.union(&Rect::new(segment.x, segment.y, segment.width, segment.height)),
    );
    let line_box = |rect: Rect| super::document::LineBox {
        left: rect.x,
        right: rect.x + rect.width,
        top: -rect.y - rect.height,
        bottom: -rect.y,
    };
    line_box(bounds).owns(&line_box(bbox))
}

#[cfg(test)]
mod tests;
