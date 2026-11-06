//! Disjoint source-image OCR strips for strictly certified, unrotated scans.

use super::hidden_layer::{decode_probe_stream, page_frame_and_contents, stream_filters};
use super::pipeline::ocr_config_with_page_rotation_hint;
use super::tall_geometry::{StripLimits, plan_strips};
use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::plugins::OcrBackend;
use crate::types::{ExtractedDocument, ProcessingWarning};
use image::ImageEncoder;
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::content::Operator;
use xberg_native_pdf::content::parser::visit_complete_content_stream;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

pub(super) mod augmentation;
mod gray_jpeg;
mod indexed;
mod ordered;
mod samples;
pub(super) use gray_jpeg::recover_after_gray_jpeg_render_failure;
use samples::{GrayPacking, decode_source_gray, flate_packing, source_density_allowed, source_frame};

const MAX_TILE_SIDE: u32 = 4096;
const MIN_STANDARD_SOURCE_DPI: f64 = 150.0;
const MAX_SOURCE_PIXELS: usize = 24_000_000;
const MAX_STRIPS: usize = 16;
const MIN_BLANK_ROWS: u32 = 20;
const MIN_LIGHT_SAMPLE: u8 = 210;
const MAX_LINE_ROWS: u32 = 80;
const MAX_COLUMN_GAP: usize = 48;
const MAX_CERTIFIED_SOURCE_LINES: usize = 256;
const MAX_OUTPUT_BYTES: usize = 128 * 1024;
// Reserve input encoding, decoded pixels and preprocessing work for one tile.
const TILE_BYTES_PER_PIXEL: usize = 24;
const FIXED_WORK_BYTES: usize = 1024 * 1024;
static STRIP_OCR: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

struct Source {
    width: u32,
    height: u32,
    dpi: f64,
    samples: Vec<u8>,
    retained_bytes: usize,
    certify_rows: bool,
}

struct StripPlan {
    source: Source,
    rows: Vec<Range<u32>>,
    ink_rows: Vec<Range<u32>>,
}

struct Attempt<'a> {
    document: &'a PdfDocument,
    page: usize,
    raster: Option<(u32, u32, usize)>,
    byte_limit: usize,
    max_strips: usize,
    cancel: Option<&'a CancellationToken>,
}

impl<'a> Attempt<'a> {
    fn new(
        document: &'a PdfDocument,
        page: usize,
        raster: Option<(u32, u32, usize)>,
        config: &'a ExtractionConfig,
        ocr: &OcrConfig,
    ) -> Self {
        let defaults = crate::extractors::security::SecurityLimits::default();
        let limits = config.security_limits.as_ref().unwrap_or(&defaults);
        let ocr_limits = ocr.security_limits.as_ref();
        let byte_limit = ocr_limits.map_or(limits.max_content_size, |ocr| {
            limits.max_content_size.min(ocr.max_content_size)
        });
        let max_strips = ocr_limits
            .map_or(limits.max_iterations, |ocr| {
                limits.max_iterations.min(ocr.max_iterations)
            })
            .min(MAX_STRIPS);
        Self {
            document,
            page,
            raster,
            byte_limit,
            max_strips,
            cancel: config.cancel_token.as_ref(),
        }
    }
}

pub(super) fn supports(config: &ExtractionConfig, ocr: &OcrConfig) -> bool {
    #[cfg(feature = "layout-detection")]
    if config.layout.is_some() {
        return false;
    }
    config.output_format == OutputFormat::Plain
        && config.result_format == crate::types::ResultFormat::Unified
        && !config.include_document_structure
        && config.images.is_none()
        && config.pdf_options.is_none()
        && config.content_filter.is_none()
        && ocr.tesseract_config.is_none()
        && ocr.element_config.is_none()
        && ocr.output_format.is_none()
        && ocr.pipeline.is_none()
        && !ocr.auto_rotate
        && ocr.language == ["eng"]
        && ocr.backend_options.as_ref().is_none_or(|options| {
            options
                .as_object()
                .is_some_and(|fields| fields.keys().all(|key| key == "use_cache"))
        })
}

pub(super) async fn try_replace(
    original: &mut ExtractedDocument,
    backend: &Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    (document, page): (&PdfDocument, usize),
    (_, png, width, height): &super::rendering::EncodedPage,
) -> crate::Result<bool> {
    if !original.tables.is_empty() || !original.formulas.is_empty() {
        return Ok(false);
    }
    let Some(quality) = RecognitionQuality::from_result(original) else {
        return Ok(false);
    };
    let attempt = Attempt::new(document, page, Some((*width, *height, png.len())), config, ocr);
    let thresholds = ocr.quality_thresholds.clone().unwrap_or_default();
    if let Some((recovered, strips)) = recover(attempt, backend, ocr).await?
        && should_replace_with_source_evidence(&original.content, quality, &recovered, &thresholds)
    {
        apply_recovery(original, recovered, page, strips);
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
fn should_replace(original_text: &str, quality: RecognitionQuality, recovered: &TextAccumulator) -> bool {
    should_replace_with_source_evidence(original_text, quality, recovered, &Default::default())
}

fn should_replace_with_source_evidence(
    original_text: &str,
    quality: RecognitionQuality,
    recovered: &TextAccumulator,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> bool {
    recovered.psm == Some(quality.psm)
        && recovered.words > quality.words
        && recovered.text.split_whitespace().count() > original_text.split_whitespace().count()
        && recovered.confidence_sum > quality.mean_confidence * f64::from(recovered.words)
        && (recovered.worst_dictionary_invalid <= quality.dictionary_invalid
            || recovered_bilevel_lines_are_clean(original_text, &quality, recovered, thresholds))
        && u64::from(recovered.low_confidence_words) * u64::from(quality.words)
            <= u64::from(quality.low_confidence_words) * u64::from(recovered.words)
}

fn recovered_bilevel_lines_are_clean(
    original_text: &str,
    quality: &RecognitionQuality,
    recovered: &TextAccumulator,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> bool {
    const MIN_STRIP_MEAN_CONFIDENCE: f64 = 85.0;
    const MAX_STRIP_DICTIONARY_INVALID: f64 = 0.05;
    recovered.complete_source_rows
        && quality.mean_confidence < thresholds.min_ocr_mean_confidence
        && super::scoring::is_ocr_recognition_noise(original_text, thresholds)
        && !super::scoring::is_ocr_recognition_noise(&recovered.text, thresholds)
        && recovered.lowest_strip_confidence >= MIN_STRIP_MEAN_CONFIDENCE.max(thresholds.min_ocr_mean_confidence)
        && recovered.worst_dictionary_invalid <= MAX_STRIP_DICTIONARY_INVALID
        && recovered.worst_dictionary_invalid <= thresholds.max_ocr_output_dict_invalid_word_ratio
}

fn apply_recovery(original: &mut ExtractedDocument, recovered: TextAccumulator, page: usize, strips: usize) {
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};

    original.content = recovered.text;
    original.ocr_elements = None;
    original.ocr_internal_document = Some(super::document::flat_ocr_page_document(&original.content));
    original.metadata.image_preprocessing = None;
    let additional = &mut original.metadata.additional;
    // Neither the old raster frame nor the old word percentiles describe the recovered text. ~keep
    for key in [
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
        OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
        "median_word_conf",
        "p10_word_conf",
        "word_iterator_skipped_count",
        "dictionary_filtered_line_count",
        "non_text_block_word_count",
    ] {
        additional.remove(key);
    }
    let words = f64::from(recovered.words);
    additional.extend([
        (Cow::Borrowed("word_count"), recovered.words.into()),
        (
            Cow::Borrowed("mean_text_conf"),
            (recovered.confidence_sum / words).into(),
        ),
        (
            Cow::Borrowed("low_conf_word_count"),
            recovered.low_confidence_words.into(),
        ),
        (
            Cow::Borrowed(DICTIONARY_INVALID),
            (recovered.dictionary_invalid_sum / words).into(),
        ),
    ]);
    original.processing_warnings.push(ProcessingWarning {
        source: Cow::Borrowed("ocr"),
        message: Cow::Owned(format!("Page {} OCR used {strips} source-image strips.", page + 1)),
    });
}

async fn recover(
    attempt: Attempt<'_>,
    backend: &Arc<dyn OcrBackend>,
    config: &OcrConfig,
) -> crate::Result<Option<(TextAccumulator, usize)>> {
    let cancelled = || attempt.cancel.is_some_and(CancellationToken::is_cancelled);
    if cancelled() {
        return Err(crate::XbergError::Cancelled);
    }
    let (_, page_height) = super::rendering::page_dimensions_pt(attempt.document, attempt.page);
    if page_height <= MAX_TILE_SIDE as f32 * 72.0 / MIN_STANDARD_SOURCE_DPI as f32 {
        return Ok(None);
    }
    // Admit one source allocation process-wide. Waiting owns only the normal raster. ~keep
    let _permit = tokio::select! {
        permit = STRIP_OCR.acquire() => permit.expect("strip semaphore never closes"),
        () = wait_for_cancel(attempt.cancel) => return Err(crate::XbergError::Cancelled),
    };
    if cancelled() {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(StripPlan { source, rows, ink_rows }) = prepare(&attempt, cancelled) else {
        return if cancelled() {
            Err(crate::XbergError::Cancelled)
        } else {
            Ok(None)
        };
    };
    let mut config = ocr_config_with_page_rotation_hint(config, 0, Some(source.dpi), true, false, false).into_owned();
    config.element_config = (!ink_rows.is_empty()).then_some(crate::types::OcrElementConfig {
        include_elements: true,
        min_level: crate::types::OcrElementLevel::Line,
        ..Default::default()
    });
    if let Some(tesseract) = config.tesseract_config.as_mut() {
        tesseract.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            target_dpi: source.dpi.round() as i32,
            ..Default::default()
        });
    }
    if let Some(options) = config.backend_options.as_mut() {
        options["use_cache"] = serde_json::Value::Bool(false);
    }
    let mut output = TextAccumulator {
        complete_source_rows: !ink_rows.is_empty(),
        lowest_strip_confidence: 100.0,
        ..Default::default()
    };
    let strip_count = rows.len();
    for rows in rows {
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let Some(png) = source.encode_rows(rows.clone()) else {
            return Ok(None);
        };
        let result = backend.process_image_owned(Arc::new(png), &config).await;
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let Ok(result) = result else {
            return Ok(None);
        };
        let certified = certified_strip_lines(&result, source.width, &rows, &ink_rows);
        if source.dpi < MIN_STANDARD_SOURCE_DPI && !certified {
            return Ok(None);
        }
        output.complete_source_rows &= certified;
        if !output.add(result) {
            return Ok(None);
        }
    }
    Ok(Some((output, strip_count)))
}

fn certified_strip_lines(result: &ExtractedDocument, width: u32, strip: &Range<u32>, ink: &[Range<u32>]) -> bool {
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
    use crate::types::{OcrBoundingGeometry, OcrElementLevel};

    let metadata = &result.metadata.additional;
    let height = strip.end - strip.start;
    if metadata
        .get(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)
        .and_then(serde_json::Value::as_u64)
        != Some(u64::from(width))
        || metadata
            .get(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)
            .and_then(serde_json::Value::as_u64)
            != Some(u64::from(height))
    {
        return false;
    }
    let Some(elements) = &result.ocr_elements else {
        return false;
    };
    let mut returned = result.content.lines().filter(|line| !line.trim().is_empty());
    let mut expected = ink
        .iter()
        .filter(|band| strip.start <= band.start && band.end <= strip.end)
        .peekable();
    if expected.peek().is_none() {
        return false;
    }
    for line in elements.iter().filter(|element| element.level == OcrElementLevel::Line) {
        let (Some(band), Some(text)) = (expected.next(), returned.next()) else {
            return false;
        };
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width: line_width,
            height: line_height,
        } = line.geometry
        else {
            return false;
        };
        let (Some(right), Some(bottom)) = (left.checked_add(line_width), top.checked_add(line_height)) else {
            return false;
        };
        if line_width == 0
            || line_height == 0
            || right > width
            || bottom > height
            || !line.text.split_whitespace().eq(text.split_whitespace())
        {
            return false;
        }
        let overlaps = |other: &Range<u32>| top + strip.start < other.end && bottom + strip.start > other.start;
        if !overlaps(band) || ink.iter().any(|other| other != band && overlaps(other)) {
            return false;
        }
    }
    expected.next().is_none() && returned.next().is_none()
}

async fn wait_for_cancel(cancel: Option<&CancellationToken>) {
    let Some(cancel) = cancel else {
        return std::future::pending().await;
    };
    while !cancel.is_cancelled() {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

fn prepare(attempt: &Attempt<'_>, cancelled: impl Fn() -> bool) -> Option<StripPlan> {
    if attempt.max_strips < 2 {
        return None;
    }
    let normal_bytes = if let Some((width, height, encoded)) = attempt.raster {
        (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?
            .checked_add(encoded)?
    } else {
        0
    };
    let available = attempt
        .byte_limit
        .checked_sub(normal_bytes)?
        .checked_sub(FIXED_WORK_BYTES)?;
    let source = certify(
        attempt.document,
        attempt.page,
        attempt.raster.map(|raster| raster.0),
        available,
    )?;
    let max_tile_pixels = available.checked_sub(source.retained_bytes)? / TILE_BYTES_PER_PIXEL;
    let mut bands = source.blank_bands(&cancelled, if source.certify_rows { 1 } else { MIN_BLANK_ROWS })?;
    let ink_rows = if source.certify_rows
        && bands.len() <= MAX_CERTIFIED_SOURCE_LINES + 1
        && bands.first().is_some_and(|band| band.start == 0)
        && bands.last().is_some_and(|band| band.end == source.height)
    {
        bands.windows(2).map(|pair| pair[0].end..pair[1].start).collect()
    } else {
        Vec::new()
    };
    if (attempt.raster.is_none() || source.dpi < MIN_STANDARD_SOURCE_DPI) && ink_rows.is_empty() {
        return None;
    }
    bands.retain(|band| band.end - band.start >= MIN_BLANK_ROWS);
    let mut plan = plan_strips(
        source.width,
        source.height,
        StripLimits {
            max_dimension: MAX_TILE_SIDE,
            max_lookback: 1024,
            max_tiles: attempt.max_strips,
            max_tile_pixels: max_tile_pixels as u64,
            max_total_pixels: MAX_SOURCE_PIXELS as u64,
        },
        &bands,
        cancelled,
    )
    .ok()?;
    // A tile boundary can leave a final strip containing only certified blank rows. ~keep
    plan.rows.retain(|rows| {
        !bands
            .iter()
            .any(|band| band.start <= rows.start && rows.end <= band.end)
    });
    (plan.rows.len() >= 2).then_some(StripPlan {
        source,
        rows: plan.rows,
        ink_rows,
    })
}

fn certify(doc: &PdfDocument, page: usize, rendered_width: Option<u32>, byte_limit: usize) -> Option<Source> {
    let (frame, content) = page_frame_and_contents(doc, page)?;
    let Object::Stream { dict, data } = content else {
        return None;
    };
    if dict.contains_key("DecodeParms") {
        return None;
    }
    let unfiltered_paint = !dict.contains_key("Filter");
    let filters = stream_filters(dict.get("Filter"))?;
    let paint = decode_probe_stream(&data, filters, 4096, byte_limit)?;
    let name = image_name(&paint, frame)?;
    let resources = doc.get_page_resources(page).ok()?;
    let resources = resources.as_dict()?;
    if resources.contains_key("ColorSpace") {
        return None;
    }
    let xobjects = doc.resolve_object(resources.get("XObject")?).ok()?;
    let Object::Stream { dict, data } = doc.resolve_object(xobjects.as_dict()?.get(&name)?).ok()? else {
        return None;
    };
    if dict.get("Subtype")?.as_name()? != "Image"
        || [
            "Mask",
            "SMask",
            "SMaskInData",
            "Decode",
            "ImageMask",
            "Alternates",
            "OC",
        ]
        .iter()
        .any(|key| dict.contains_key(*key))
    {
        return None;
    }
    let (width, height, dpi) = source_frame(&dict, frame, rendered_width)?;
    let packing = match (rendered_width, dict.get("Filter")?.as_name()?) {
        (None, "DCTDecode")
            if dict.get("ColorSpace")?.as_name()? == "DeviceGray"
                && dict.get("BitsPerComponent")?.as_integer()? == 8
                && !dict.contains_key("DecodeParms") =>
        {
            None
        }
        (Some(_), "FlateDecode") => Some(flate_packing(&dict, width, unfiltered_paint)?),
        _ => return None,
    };
    if !source_density_allowed(dpi, packing) {
        return None;
    }
    let (samples, retained_bytes) = if let Some(packing) = packing {
        decode_source_gray(&data, width as usize, height as usize, packing, byte_limit)?
    } else {
        let samples = crate::extraction::image::decode_sequential_gray_jpeg(&data, (width, height), byte_limit)?;
        let retained_bytes = samples.capacity().checked_add(data.len())?;
        (samples, retained_bytes)
    };
    Some(Source {
        width,
        height,
        dpi,
        samples,
        retained_bytes,
        certify_rows: packing.is_none_or(GrayPacking::requires_line_certificate),
    })
}

pub(super) fn image_name(paint: &[u8], frame: Rect) -> Option<String> {
    let mut index = 0;
    let mut image = None;
    let complete = visit_complete_content_stream(paint, 4, |operator| {
        match (index, operator) {
            (0, Operator::SaveState) | (3, Operator::RestoreState) => {}
            (1, Operator::Cm { a, b, c, d, e, f })
                if (a, b, c, d, e, f) == (frame.width, 0.0, 0.0, frame.height, frame.x, frame.y) => {}
            (2, Operator::Do { name }) => image = Some(name),
            _ => return false,
        }
        index += 1;
        true
    })
    .ok()?;
    if complete && index == 4 { image } else { None }
}

impl Source {
    fn encode_rows(&self, rows: Range<u32>) -> Option<Vec<u8>> {
        let width = self.width as usize;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                &self.samples[rows.start as usize * width..rows.end as usize * width],
                self.width,
                rows.end - rows.start,
                image::ExtendedColorType::L8,
            )
            .ok()?;
        Some(bytes)
    }

    fn blank_bands(&self, cancelled: impl Fn() -> bool, min_rows: u32) -> Option<Vec<Range<u32>>> {
        let mut bands = Vec::new();
        let mut start = None;
        let mut line = LineFootprint::new(self.width as usize);
        for (y, row) in self.samples.chunks_exact(self.width as usize).enumerate() {
            if cancelled() {
                return None;
            }
            let y = y as u32;
            if row[0] >= MIN_LIGHT_SAMPLE && row.iter().all(|&sample| sample == row[0]) {
                line.finish()?;
                start.get_or_insert(y);
            } else {
                line.add(row)?;
                if let Some(first) = start.take()
                    && y - first >= min_rows
                {
                    bands.push(first..y);
                }
            }
        }
        line.finish()?;
        if let Some(first) = start
            && self.height - first >= min_rows
        {
            bands.push(first..self.height);
        }
        Some(bands)
    }
}

// A consistent narrow line start and no wide internal gaps exclude common
// multi-column or non-text strips before top-to-bottom concatenation. ~keep
struct LineFootprint {
    columns: Vec<bool>,
    first_ink: Option<usize>,
    rows: u32,
}

impl LineFootprint {
    fn new(width: usize) -> Self {
        Self {
            columns: vec![false; width],
            first_ink: None,
            rows: 0,
        }
    }

    fn add(&mut self, row: &[u8]) -> Option<()> {
        self.rows += 1;
        if self.rows > MAX_LINE_ROWS {
            return None;
        }
        for (column, &sample) in self.columns.iter_mut().zip(row) {
            *column |= sample < MIN_LIGHT_SAMPLE;
        }
        Some(())
    }

    fn finish(&mut self) -> Option<()> {
        if self.rows == 0 {
            return Some(());
        }
        let left = self.columns.iter().position(|&ink| ink)?;
        let right = self.columns.iter().rposition(|&ink| ink)?;
        let first = *self.first_ink.get_or_insert(left);
        if left.abs_diff(first) >= MAX_COLUMN_GAP
            || self.columns[left..=right]
                .split(|&ink| ink)
                .any(|gap| gap.len() >= MAX_COLUMN_GAP)
        {
            return None;
        }
        self.columns.fill(false);
        self.rows = 0;
        Some(())
    }
}

const DICTIONARY_INVALID: &str = crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY;

struct RecognitionQuality {
    psm: i32,
    words: u32,
    mean_confidence: f64,
    low_confidence_words: u32,
    dictionary_invalid: f64,
}

impl RecognitionQuality {
    fn from_result(result: &ExtractedDocument) -> Option<Self> {
        let metadata = &result.metadata.additional;
        let words = super::word_count_of(metadata).filter(|&words| words > 0)?;
        let crate::types::FormatMetadata::Ocr(format) = result.metadata.format.as_ref()? else {
            return None;
        };
        Some(Self {
            psm: format.psm,
            words,
            mean_confidence: super::mean_text_conf_of(metadata).filter(|mean| (0.0..=100.0).contains(mean))?,
            low_confidence_words: metadata
                .get("low_conf_word_count")?
                .as_u64()
                .and_then(|count| u32::try_from(count).ok())
                .filter(|&count| count <= words)?,
            dictionary_invalid: metadata
                .get(DICTIONARY_INVALID)?
                .as_f64()
                .filter(|ratio| (0.0..=1.0).contains(ratio))?,
        })
    }
}

#[derive(Default)]
struct TextAccumulator {
    psm: Option<i32>,
    text: String,
    words: u32,
    low_confidence_words: u32,
    confidence_sum: f64,
    dictionary_invalid_sum: f64,
    worst_dictionary_invalid: f64,
    lowest_strip_confidence: f64,
    complete_source_rows: bool,
}

impl TextAccumulator {
    fn add(&mut self, result: ExtractedDocument) -> bool {
        let text = result.content.trim();
        if text.is_empty()
            || !result.tables.is_empty()
            || !result.formulas.is_empty()
            || !result.processing_warnings.is_empty()
            || result.metadata.error.is_some()
            || result
                .metadata
                .image_preprocessing
                .as_ref()
                .is_some_and(|metadata| metadata.resize_error.is_some())
            || [
                "word_iterator_skipped_count",
                "dictionary_filtered_line_count",
                "non_text_block_word_count",
            ]
            .iter()
            .any(|key| {
                result
                    .metadata
                    .additional
                    .get(*key)
                    .is_some_and(|count| count.as_u64() != Some(0))
            })
            || self.text.len().saturating_add(text.len()).saturating_add(1) > MAX_OUTPUT_BYTES
        {
            return false;
        }
        let Some(quality) = RecognitionQuality::from_result(&result) else {
            return false;
        };
        let Some(total_words) = self.words.checked_add(quality.words) else {
            return false;
        };
        if self.psm.is_some_and(|psm| psm != quality.psm) {
            return false;
        }
        self.psm = Some(quality.psm);
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(text);
        self.words = total_words;
        self.low_confidence_words += quality.low_confidence_words;
        self.confidence_sum += quality.mean_confidence * f64::from(quality.words);
        // Use the worst strip for acceptance. The reported ratio weights by retained word count,
        // because the backend does not expose the dictionary-specific denominator. ~keep
        self.worst_dictionary_invalid = self.worst_dictionary_invalid.max(quality.dictionary_invalid);
        self.lowest_strip_confidence = self.lowest_strip_confidence.min(quality.mean_confidence);
        self.dictionary_invalid_sum += quality.dictionary_invalid * f64::from(quality.words);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::samples::decode_exact_gray;
    use super::*;
    use std::io::Write;

    const TALL_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/repros/tall-scan-strips/strict-tall.pdf"
    ));

    fn recognized(content: &str, mean: f64, invalid: f64, words: u32) -> ExtractedDocument {
        let mut result = ExtractedDocument {
            content: content.into(),
            ..Default::default()
        };
        result.metadata.format = Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
            language: "eng".into(),
            psm: 11,
            output_format: "markdown".into(),
            table_count: 0,
            table_rows: None,
            table_cols: None,
        }));
        result.metadata.additional.extend([
            (Cow::Borrowed("mean_text_conf"), mean.into()),
            (Cow::Borrowed("word_count"), words.into()),
            (Cow::Borrowed("low_conf_word_count"), 0.into()),
            (
                Cow::Borrowed(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY),
                invalid.into(),
            ),
        ]);
        result
    }

    #[test]
    fn more_low_confidence_or_dictionary_invalid_words_do_not_replace_original() {
        let original = recognized("The clean original", 96.0, 0.0, 3);
        let low_confidence = recognized("The clean original has more words", 72.0, 0.0, 6);
        let invalid_words = recognized("The clean original xzzz yyzz", 98.0, 0.4, 5);
        for candidate in [low_confidence, invalid_words] {
            let mut recovered = TextAccumulator::default();
            assert!(recovered.add(candidate));
            assert!(!should_replace(
                &original.content,
                RecognitionQuality::from_result(&original).unwrap(),
                &recovered
            ));
        }
    }

    #[test]
    fn strip_reports_incomplete_word_iteration_and_is_rejected() {
        let mut result = recognized("The clean original has more words", 98.0, 0.0, 6);
        result
            .metadata
            .additional
            .insert(Cow::Borrowed("word_iterator_skipped_count"), 1.into());
        let mut recovered = TextAccumulator::default();
        assert!(!recovered.add(result));
        assert!(recovered.text.is_empty());
    }

    #[test]
    fn accepted_recovery_updates_text_metrics_and_keeps_page_diagnostics() {
        let mut original = recognized("The clean original", 90.0, 0.1, 3);
        original.mime_type = Cow::Borrowed("application/pdf");
        original.metadata.extraction_duration_ms = Some(21);
        original.metadata.additional.insert(Cow::Borrowed("psm"), "11".into());
        original
            .metadata
            .additional
            .insert(Cow::Borrowed("median_word_conf"), 88.into());
        original.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed("ocr"),
            message: Cow::Borrowed("original diagnostic"),
        });
        let mut recovered = TextAccumulator::default();
        assert!(recovered.add(recognized("The clean original", 94.0, 0.0, 3)));
        assert!(recovered.add(recognized("has more words", 96.0, 0.0, 3)));
        assert!(should_replace(
            &original.content,
            RecognitionQuality::from_result(&original).unwrap(),
            &recovered
        ));
        apply_recovery(&mut original, recovered, 1, 2);

        assert_eq!(original.content, "The clean original\nhas more words");
        assert_eq!(original.mime_type, "application/pdf");
        assert_eq!(original.metadata.extraction_duration_ms, Some(21));
        assert_eq!(original.metadata.additional["psm"], "11");
        assert!(matches!(
            original.metadata.format,
            Some(crate::types::FormatMetadata::Ocr(format)) if format.psm == 11
        ));
        assert_eq!(original.metadata.additional["word_count"], 6);
        assert_eq!(original.metadata.additional["mean_text_conf"], 95.0);
        assert_eq!(original.metadata.additional["low_conf_word_count"], 0);
        assert!(!original.metadata.additional.contains_key("median_word_conf"));
        assert_eq!(original.processing_warnings[0].message, "original diagnostic");
        assert_eq!(
            original.processing_warnings[1].message,
            "Page 2 OCR used 2 source-image strips."
        );
    }

    #[test]
    fn gray_source_requires_exact_stream_end_and_size() {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&[0, 1, 254, 255]).unwrap();
        let mut compressed = encoder.finish().unwrap();
        assert_eq!(decode_exact_gray(&compressed, 4), Some(vec![0, 1, 254, 255]));
        assert_eq!(decode_exact_gray(&compressed, 3), None);
        assert_eq!(decode_exact_gray(&compressed, 5), None);
        assert_eq!(decode_exact_gray(&compressed[..compressed.len() - 1], 4), None);
        compressed.push(0);
        assert_eq!(decode_exact_gray(&compressed, 4), None);
    }

    #[test]
    fn bilevel_source_ignores_padding_and_budgets_packed_and_expanded_rows() {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(&[0b1010_0101, 0b0111_1111, 0b0101_1010, 0b1000_0000])
            .unwrap();
        let compressed = encoder.finish().unwrap();
        let retained = compressed.len() + 18 + 1;
        let peak = retained + 4;
        let expected = vec![255, 0, 255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 0, 255];
        assert_eq!(
            decode_source_gray(&compressed, 9, 2, GrayPacking::Bilevel, peak),
            Some((expected, retained))
        );
        assert_eq!(
            decode_source_gray(&compressed, 9, 2, GrayPacking::Bilevel, peak - 1),
            None
        );
        assert_eq!(decode_source_gray(&compressed, 9, 1, GrayPacking::Bilevel, peak), None);
    }

    #[test]
    fn neutral_rgb_source_requires_equal_channels_and_simultaneous_buffers() {
        let compress = |samples: &[u8]| {
            let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(samples).unwrap();
            encoder.finish().unwrap()
        };
        let neutral = compress(&[10, 10, 10, 205, 205, 205]);
        let coloured = compress(&[10, 10, 10, 205, 206, 205]);
        let retained = neutral.len() + 2 + 1;
        let peak = retained + 6;
        assert_eq!(
            decode_source_gray(&neutral, 2, 1, GrayPacking::NeutralRgb, peak),
            Some((vec![10, 205], retained))
        );
        assert_eq!(
            decode_source_gray(&neutral, 2, 1, GrayPacking::NeutralRgb, peak - 1),
            None
        );
        assert_eq!(
            decode_source_gray(&coloured, 2, 1, GrayPacking::NeutralRgb, usize::MAX),
            None
        );
    }

    #[test]
    fn bilevel_line_proof_rejects_a_split_that_hides_a_missing_source_line() {
        use crate::ocr_metadata_keys::{
            OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
        };
        use crate::types::{OcrBoundingGeometry, OcrElement, OcrElementLevel};
        let line = |text: &str, top| OcrElement {
            text: text.to_owned(),
            level: OcrElementLevel::Line,
            geometry: OcrBoundingGeometry::Rectangle {
                left: 20,
                top,
                width: 100,
                height: 5,
            },
            ..Default::default()
        };
        let mut result = ExtractedDocument {
            content: "one\ntwo".to_owned(),
            ocr_elements: Some(vec![line("one", 11), line("two", 30)]),
            ..Default::default()
        };
        result
            .metadata
            .additional
            .insert(Cow::Borrowed(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY), 200.into());
        result
            .metadata
            .additional
            .insert(Cow::Borrowed(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY), 60.into());
        let ink = [110..117, 130..137];
        assert!(certified_strip_lines(&result, 200, &(100..160), &ink));
        result.ocr_elements.as_mut().unwrap()[1] = line("two", 12);
        assert!(!certified_strip_lines(&result, 200, &(100..160), &ink));
    }

    #[test]
    fn two_columns_and_staggered_line_starts_refuse_ordering() {
        let mut line = LineFootprint::new(200);
        let mut row = vec![255; 200];
        row[10..30].fill(0);
        row[150..170].fill(0);
        assert_eq!(line.add(&row), Some(()));
        assert_eq!(line.finish(), None);

        let mut line = LineFootprint::new(200);
        row[150..170].fill(255);
        assert_eq!(line.add(&row), Some(()));
        assert_eq!(line.finish(), Some(()));
        row[10..30].fill(255);
        row[150..170].fill(0);
        assert_eq!(line.add(&row), Some(()));
        assert_eq!(line.finish(), None);
    }

    #[test]
    fn source_and_normal_raster_are_charged_before_strip_buffers() {
        let document = PdfDocument::from_bytes(TALL_FIXTURE.to_vec()).expect("real source image fixture");
        let mut attempt = Attempt {
            document: &document,
            page: 0,
            raster: Some((1100, 7765, 128_000)),
            byte_limit: 100 * 1024 * 1024,
            max_strips: MAX_STRIPS,
            cancel: None,
        };
        let StripPlan {
            source, rows: strips, ..
        } = prepare(&attempt, || false).expect("admitted bounded strips");
        assert_eq!(source.height, 9000);
        assert_eq!(strips.first().unwrap().start, 0);
        assert_eq!(strips.last().unwrap().end, 8812);
        let bands = source.blank_bands(|| false, MIN_BLANK_ROWS).unwrap();
        assert!(bands.iter().any(|band| band.start <= 8812 && band.end == source.height));
        let peak = 1100 * 7765 * 4
            + 128_000
            + FIXED_WORK_BYTES
            + source.retained_bytes
            + strips.iter().map(|rows| rows.len()).max().unwrap() * source.width as usize * TILE_BYTES_PER_PIXEL;
        assert!(peak <= attempt.byte_limit);
        attempt.byte_limit = 32 * 1024 * 1024;
        assert!(prepare(&attempt, || false).is_none());
    }

    #[tokio::test]
    async fn waiting_for_the_source_slot_honors_cancellation() {
        let _held = STRIP_OCR.acquire().await.unwrap();
        let doc = PdfDocument::from_bytes(TALL_FIXTURE.to_vec()).expect("real source image fixture");
        let token = CancellationToken::new();
        crate::plugins::ensure_ocr_backends_initialized();
        let backend = crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .get("tesseract")
            .unwrap();
        let attempt = Attempt {
            document: &doc,
            page: 0,
            raster: Some((1100, 7765, 128_000)),
            byte_limit: 100 * 1024 * 1024,
            max_strips: MAX_STRIPS,
            cancel: Some(&token),
        };
        let config = OcrConfig::default();
        let cancel = async {
            tokio::task::yield_now().await;
            token.cancel();
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            tokio::join!(recover(attempt, &backend, &config), cancel)
        })
        .await
        .expect("waiting caller observes cancellation");
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }
}
