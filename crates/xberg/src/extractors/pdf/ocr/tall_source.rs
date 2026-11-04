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

const MAX_TILE_SIDE: u32 = 4096;
const MAX_SOURCE_PIXELS: usize = 24_000_000;
const MAX_STRIPS: usize = 16;
const MIN_BLANK_ROWS: u32 = 20;
const MIN_LIGHT_SAMPLE: u8 = 210;
const MAX_LINE_ROWS: u32 = 80;
const MAX_COLUMN_GAP: usize = 48;
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
}

struct Attempt<'a> {
    document: &'a PdfDocument,
    page: usize,
    raster: (u32, u32, usize),
    byte_limit: usize,
    max_strips: usize,
    cancel: Option<&'a CancellationToken>,
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
) -> crate::Result<()> {
    if !original.tables.is_empty() || !original.formulas.is_empty() {
        return Ok(());
    }
    let defaults = crate::extractors::security::SecurityLimits::default();
    let limits = config.security_limits.as_ref().unwrap_or(&defaults);
    let byte_limit = ocr.security_limits.as_ref().map_or(limits.max_content_size, |ocr| {
        limits.max_content_size.min(ocr.max_content_size)
    });
    let attempt = Attempt {
        document,
        page,
        raster: (*width, *height, png.len()),
        byte_limit,
        max_strips: ocr
            .security_limits
            .as_ref()
            .map_or(limits.max_iterations, |ocr| {
                limits.max_iterations.min(ocr.max_iterations)
            })
            .min(MAX_STRIPS),
        cancel: config.cancel_token.as_ref(),
    };
    if let Some(mut recovered) = recover(attempt, backend, ocr).await?
        && recovered.content.split_whitespace().count() > original.content.split_whitespace().count()
    {
        let mut warnings = std::mem::take(&mut original.processing_warnings);
        warnings.append(&mut recovered.processing_warnings);
        recovered.processing_warnings = warnings;
        *original = recovered;
    }
    Ok(())
}

async fn recover(
    attempt: Attempt<'_>,
    backend: &Arc<dyn OcrBackend>,
    config: &OcrConfig,
) -> crate::Result<Option<ExtractedDocument>> {
    let cancelled = || attempt.cancel.is_some_and(CancellationToken::is_cancelled);
    if cancelled() {
        return Err(crate::XbergError::Cancelled);
    }
    let (_, page_height) = super::rendering::page_dimensions_pt(attempt.document, attempt.page);
    if page_height <= MAX_TILE_SIDE as f32 * 72.0 / 150.0 {
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
    let Some((source, strips)) = prepare(&attempt, cancelled) else {
        return if cancelled() {
            Err(crate::XbergError::Cancelled)
        } else {
            Ok(None)
        };
    };
    let mut config = ocr_config_with_page_rotation_hint(config, 0, Some(source.dpi), true, false, false).into_owned();
    config.element_config = None;
    if let Some(tesseract) = config.tesseract_config.as_mut() {
        tesseract.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            target_dpi: source.dpi.round() as i32,
            ..Default::default()
        });
    }
    if let Some(options) = config.backend_options.as_mut() {
        options["use_cache"] = serde_json::Value::Bool(false);
    }
    let mut output = TextAccumulator::default();
    let strip_count = strips.len();
    for rows in strips {
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let Some(png) = source.encode_rows(rows) else {
            return Ok(None);
        };
        let result = backend.process_image_owned(Arc::new(png), &config).await;
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let Ok(result) = result else {
            return Ok(None);
        };
        if !output.add(result) {
            return Ok(None);
        }
    }
    Ok(output.finish(attempt.page, strip_count))
}

async fn wait_for_cancel(cancel: Option<&CancellationToken>) {
    let Some(cancel) = cancel else {
        return std::future::pending().await;
    };
    while !cancel.is_cancelled() {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

fn prepare(attempt: &Attempt<'_>, cancelled: impl Fn() -> bool) -> Option<(Source, Vec<Range<u32>>)> {
    if attempt.max_strips < 2 {
        return None;
    }
    let (width, height, encoded) = attempt.raster;
    let normal_bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?
        .checked_add(encoded)?;
    let available = attempt
        .byte_limit
        .checked_sub(normal_bytes)?
        .checked_sub(FIXED_WORK_BYTES)?;
    let source = certify(attempt.document, attempt.page, width, available)?;
    let max_tile_pixels = available.checked_sub(source.retained_bytes)? / TILE_BYTES_PER_PIXEL;
    let bands = source.blank_bands(&cancelled)?;
    let plan = plan_strips(
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
    Some((source, plan.rows))
}

fn certify(doc: &PdfDocument, page: usize, rendered_width: u32, byte_limit: usize) -> Option<Source> {
    let (frame, content) = page_frame_and_contents(doc, page)?;
    let Object::Stream { dict, data } = content else {
        return None;
    };
    if dict.contains_key("DecodeParms") {
        return None;
    }
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
        || dict.get("BitsPerComponent")?.as_integer()? != 8
        || dict.get("ColorSpace")?.as_name()? != "DeviceGray"
        || dict.get("Filter")?.as_name()? != "FlateDecode"
        || [
            "Mask",
            "SMask",
            "SMaskInData",
            "Decode",
            "DecodeParms",
            "ImageMask",
            "Alternates",
            "OC",
        ]
        .iter()
        .any(|key| dict.contains_key(*key))
    {
        return None;
    }
    let width = u32::try_from(dict.get("Width")?.as_integer()?).ok()?;
    let height = u32::try_from(dict.get("Height")?.as_integer()?).ok()?;
    let pixels = (width as usize).checked_mul(height as usize)?;
    if u64::from(rendered_width) * 100 >= u64::from(width) * 98
        || width > MAX_TILE_SIDE
        || height <= MAX_TILE_SIDE
        || height > MAX_TILE_SIDE * MAX_STRIPS as u32
        || pixels > MAX_SOURCE_PIXELS
    {
        return None;
    }
    let dpi = 72.0 * f64::from(width) / f64::from(frame.width);
    let dpi_y = 72.0 * f64::from(height) / f64::from(frame.height);
    if !(150.0..=300.0).contains(&dpi) || (dpi - dpi_y).abs() > 0.01 {
        return None;
    }
    let retained_bytes = data.len().checked_add(pixels)?.checked_add(1)?;
    if retained_bytes > byte_limit {
        return None;
    }
    let samples = decode_exact_gray(&data, pixels)?;
    Some(Source {
        width,
        height,
        dpi,
        samples,
        retained_bytes,
    })
}

fn decode_exact_gray(compressed: &[u8], pixels: usize) -> Option<Vec<u8>> {
    use flate2::{Decompress, FlushDecompress, Status};

    let capacity = pixels.checked_add(1)?;
    let mut samples = Vec::new();
    samples.try_reserve_exact(capacity).ok()?;
    samples.resize(capacity, 0);
    let mut decoder = Decompress::new(true);
    let status = decoder
        .decompress(compressed, &mut samples, FlushDecompress::Finish)
        .ok()?;
    if status != Status::StreamEnd
        || decoder.total_out() != pixels as u64
        || decoder.total_in() != compressed.len() as u64
    {
        return None;
    }
    samples.truncate(pixels);
    Some(samples)
}

fn image_name(paint: &[u8], frame: Rect) -> Option<String> {
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

    fn blank_bands(&self, cancelled: impl Fn() -> bool) -> Option<Vec<Range<u32>>> {
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
                    && y - first >= MIN_BLANK_ROWS
                {
                    bands.push(first..y);
                }
            }
        }
        line.finish()?;
        if let Some(first) = start
            && self.height - first >= MIN_BLANK_ROWS
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

#[derive(Default)]
struct TextAccumulator {
    text: String,
    words: u32,
    confidence_sum: f64,
    dictionary_invalid_sum: f64,
    dictionary_words: u32,
    format: Option<crate::types::FormatMetadata>,
}

impl TextAccumulator {
    fn add(&mut self, result: ExtractedDocument) -> bool {
        let text = result.content.trim();
        if text.is_empty()
            || !result.tables.is_empty()
            || !result.formulas.is_empty()
            || !result.processing_warnings.is_empty()
            || self.text.len().saturating_add(text.len()).saturating_add(1) > MAX_OUTPUT_BYTES
        {
            return false;
        }
        let Some(words) = super::word_count_of(&result.metadata.additional) else {
            return false;
        };
        let Some(confidence) = super::mean_text_conf_of(&result.metadata.additional) else {
            return false;
        };
        let Some(total_words) = self.words.checked_add(words) else {
            return false;
        };
        if words == 0 || confidence > 100.0 {
            return false;
        }
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(text);
        self.format = self.format.take().or(result.metadata.format);
        self.words = total_words;
        self.confidence_sum += confidence * f64::from(words);
        if let Some(ratio) = result
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
            .and_then(serde_json::Value::as_f64)
            .filter(|ratio| (0.0..=1.0).contains(ratio))
        {
            self.dictionary_invalid_sum += ratio * f64::from(words);
            self.dictionary_words += words;
        }
        true
    }

    fn finish(self, page: usize, strips: usize) -> Option<ExtractedDocument> {
        if self.words == 0 {
            return None;
        }
        let mut result = ExtractedDocument {
            content: self.text,
            ..Default::default()
        };
        result.metadata.format = self.format;
        result
            .metadata
            .additional
            .insert(Cow::Borrowed("word_count"), self.words.into());
        result.metadata.additional.insert(
            Cow::Borrowed("mean_text_conf"),
            (self.confidence_sum / f64::from(self.words)).into(),
        );
        if self.dictionary_words == self.words {
            result.metadata.additional.insert(
                Cow::Borrowed(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY),
                (self.dictionary_invalid_sum / f64::from(self.words)).into(),
            );
        }
        result.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed("ocr"),
            message: Cow::Owned(format!("Page {} OCR used {strips} source-image strips.", page + 1)),
        });
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const TALL_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/repros/tall-scan-strips/strict-tall.pdf"
    ));

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
            raster: (1100, 7765, 128_000),
            byte_limit: 100 * 1024 * 1024,
            max_strips: MAX_STRIPS,
            cancel: None,
        };
        let (source, strips) = prepare(&attempt, || false).expect("admitted bounded strips");
        assert_eq!(source.height, 9000);
        assert_eq!(strips.first().unwrap().start, 0);
        assert_eq!(strips.last().unwrap().end, source.height);
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
            raster: (1100, 7765, 128_000),
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
