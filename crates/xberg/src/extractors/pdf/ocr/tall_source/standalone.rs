//! Strip recovery for a tall standalone picture that OCR preprocessing clamped.
//!
//! The picture bytes are the source certificate here: they are decoded once, their blank
//! rows are measured, and every strip must read back as exactly the source lines it holds.
//! The acceptance rule is the same as the PDF route; only the certificate is different.

use super::super::pipeline::ocr_config_with_page_rotation_hint;
use super::super::tall_geometry::{StripLimits, plan_strips};
use super::policy::standalone_unsupported_reason;
use super::{
    FIXED_WORK_BYTES, MAX_SOURCE_PIXELS, MAX_STRIPS, MAX_TILE_SIDE, RecognitionQuality, Source, TILE_BYTES_PER_PIXEL,
    TextAccumulator, apply_recovery, certified_strip_lines, dictionary_ratio_is_diagnostic,
    should_replace_with_source_evidence,
};
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::plugins::OcrBackend;
use crate::types::ExtractedDocument;
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

/// The strip ranges to read and the ink rows they must reproduce.
type PlannedStrips = (Vec<Range<u32>>, Vec<Range<u32>>);

const MIN_TALL_ASPECT: u32 = 4;
// Well under `plan_strips`' 1024-row lookback, so a cut always falls near the nominal end.
const PLAN_CUT_BAND_ROWS: u32 = 512;

fn split_long_bands(bands: &[Range<u32>], chunk: u32) -> Vec<Range<u32>> {
    let mut split = Vec::with_capacity(bands.len());
    for band in bands {
        let mut start = band.start;
        while start < band.end {
            let end = start.saturating_add(chunk).min(band.end);
            split.push(start..end);
            start = end;
        }
    }
    split
}

/// Why a tall picture kept the read made from its downscaled pixels.
enum StandaloneSkip {
    Config(&'static str),
    Density(f64),
    UnsupportedSource,
    Budget,
    InsufficientReadEvidence,
    NoCertifiedRows,
    Rejected,
}

/// Replaces a tall picture's downscaled read with a read from disjoint source strips.
///
/// Returns `true` when the strip read was accepted. A skip leaves the preprocessing metadata
/// in place and records why the source strips were not used.
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) async fn try_replace_standalone(
    original: &mut ExtractedDocument,
    bytes: &[u8],
    backend: &Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
) -> crate::Result<bool> {
    let Some(bounds) = StandaloneFrame::from_read(original).filter(StandaloneFrame::was_reduced) else {
        return Ok(false);
    };
    let skip = |original: &mut ExtractedDocument, why| note_standalone_skipped(original, &bounds, why);
    if let Some(reason) = standalone_unsupported_reason(config, ocr, backend.as_ref()) {
        skip(original, StandaloneSkip::Config(reason));
        return Ok(false);
    }
    let (Some(quality), Some(dpi)) = (RecognitionQuality::from_result(original), bounds.dpi()) else {
        skip(original, StandaloneSkip::InsufficientReadEvidence);
        return Ok(false);
    };
    if !super::samples::source_density_allowed(dpi, Some(super::samples::GrayPacking::Byte)) {
        skip(original, StandaloneSkip::Density(dpi));
        return Ok(false);
    }
    let Some(samples) = decode_gray(bytes, bounds.dimensions) else {
        skip(original, StandaloneSkip::UnsupportedSource);
        return Ok(false);
    };
    let retained_bytes = samples.len();
    let Some(available) = standalone_budget(config, ocr, &bounds) else {
        skip(original, StandaloneSkip::Budget);
        return Ok(false);
    };
    let source = Source {
        width: bounds.dimensions.0,
        height: bounds.dimensions.1,
        dpi,
        samples,
        retained_bytes,
        // The decoded picture is the certificate, so every line boundary can be checked. ~keep
        certify_rows: true,
        reread_split_row: false,
    };
    let cancelled = || config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled());
    let Some((rows, ink_rows)) = plan_source_strips(&source, available, config, ocr, cancelled) else {
        skip(original, StandaloneSkip::NoCertifiedRows);
        return Ok(false);
    };
    let mut strip_config = strip_ocr_config(ocr, &quality, dpi);
    let Some((recovered, strips)) =
        read_strips(&source, &rows, &ink_rows, backend, &mut strip_config, cancelled).await?
    else {
        skip(original, StandaloneSkip::Rejected);
        return Ok(false);
    };
    let thresholds = ocr.quality_thresholds.clone().unwrap_or_default();
    if !should_replace_with_source_evidence(
        &original.content,
        quality,
        &recovered,
        &thresholds,
        dictionary_ratio_is_diagnostic(ocr),
    ) {
        skip(original, StandaloneSkip::Rejected);
        return Ok(false);
    }
    apply_recovery(original, recovered, 0, strips);
    Ok(true)
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn note_standalone_skipped(original: &mut ExtractedDocument, bounds: &StandaloneFrame, why: StandaloneSkip) {
    let reason: Cow<'static, str> = match why {
        StandaloneSkip::Config(reason) => Cow::Borrowed(reason),
        StandaloneSkip::Density(dpi) => Cow::Owned(format!(
            "the {:.0} DPI source is outside the 120 to 300 DPI strip-recovery range",
            dpi.floor()
        )),
        StandaloneSkip::UnsupportedSource => {
            Cow::Borrowed("the picture is not a single-frame PNG, JPEG, WebP or BMP that strip recovery can decode")
        }
        StandaloneSkip::Budget => Cow::Borrowed("the memory budget does not hold the decoded source and its strips"),
        StandaloneSkip::InsufficientReadEvidence => {
            Cow::Borrowed("the downscaled read did not supply enough evidence to compare the strips")
        }
        StandaloneSkip::NoCertifiedRows => {
            Cow::Borrowed("the scan has no certified blank rows between source text lines")
        }
        StandaloneSkip::Rejected => Cow::Borrowed("the strips were read but were not better than the downscaled read"),
    };
    crate::core::diagnostics::push_warning(
        &mut original.processing_warnings,
        "ocr",
        format!(
            "Standalone image OCR reduced {} × {} pixels to {} × {} pixels and its source-image strips were \
             not used: {reason}. The text may be incomplete.",
            bounds.dimensions.0, bounds.dimensions.1, bounds.processed.0, bounds.processed.1,
        ),
    );
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn plan_source_strips(
    source: &Source,
    available: usize,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    cancelled: impl Fn() -> bool,
) -> Option<PlannedStrips> {
    let max_strips = config
        .security_limits
        .as_ref()
        .map_or(usize::MAX, |limits| limits.max_iterations)
        .min(
            ocr.security_limits
                .as_ref()
                .map_or(usize::MAX, |limits| limits.max_iterations),
        )
        .min(MAX_STRIPS);
    if max_strips < 2 {
        return None;
    }
    let max_tile_pixels = available.checked_sub(source.retained_bytes)? / TILE_BYTES_PER_PIXEL;
    let bands = source.blank_bands(&cancelled, 1)?;
    // A large blank margin is blank everywhere, but `plan_strips` only offers its midpoint
    // as a cut. Splitting long bands keeps a certified cut within the plan's lookback. ~keep
    let plan_bands = split_long_bands(&bands, PLAN_CUT_BAND_ROWS);
    let ink_rows = if bands.len() <= super::MAX_CERTIFIED_SOURCE_LINES + 1
        && bands.first().is_some_and(|band| band.start == 0)
        && bands.last().is_some_and(|band| band.end == source.height)
    {
        bands.windows(2).map(|pair| pair[0].end..pair[1].start).collect()
    } else {
        Vec::new()
    };
    if ink_rows.is_empty() {
        return None;
    }
    let plan = plan_strips(
        source.width,
        source.height,
        StripLimits {
            max_dimension: MAX_TILE_SIDE,
            max_lookback: 1024,
            max_tiles: max_strips,
            max_tile_pixels: max_tile_pixels as u64,
            max_total_pixels: MAX_SOURCE_PIXELS as u64,
        },
        &plan_bands,
        &cancelled,
    )
    .ok()?;
    let rows: Vec<_> = plan
        .rows
        .into_iter()
        .filter(|rows| {
            !bands
                .iter()
                .any(|band| band.start <= rows.start && rows.end <= band.end)
        })
        .collect();
    (rows.len() >= 2).then_some((rows, ink_rows))
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn strip_ocr_config(ocr: &OcrConfig, quality: &RecognitionQuality, dpi: f64) -> OcrConfig {
    let mut base = ocr.clone();
    base.tesseract_config.get_or_insert_with(Default::default).psm = Some(quality.psm);
    let mut strip = ocr_config_with_page_rotation_hint(&base, 0, Some(dpi), true, false, false).into_owned();
    strip.element_config = Some(crate::types::OcrElementConfig {
        include_elements: true,
        min_level: crate::types::OcrElementLevel::Line,
        ..Default::default()
    });
    if let Some(tesseract) = strip.tesseract_config.as_mut() {
        tesseract.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            target_dpi: dpi.round() as i32,
            ..Default::default()
        });
    }
    if let Some(options) = strip.backend_options.as_mut() {
        options["use_cache"] = serde_json::Value::Bool(false);
    }
    strip
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
async fn read_strips(
    source: &Source,
    rows: &[Range<u32>],
    ink_rows: &[Range<u32>],
    backend: &Arc<dyn OcrBackend>,
    strip_config: &mut OcrConfig,
    cancelled: impl Fn() -> bool,
) -> crate::Result<Option<(TextAccumulator, usize)>> {
    let mut output = TextAccumulator {
        complete_source_rows: true,
        lowest_strip_confidence: 100.0,
        ..Default::default()
    };
    for rows in rows {
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let Some(png) = source.encode_rows(rows.clone()) else {
            return Ok(None);
        };
        let Ok(result) = backend.process_image_owned(Arc::new(png), strip_config).await else {
            return Ok(None);
        };
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        if !certified_strip_lines(&result, source.width, rows, ink_rows) {
            return Ok(None);
        }
        if !output.add(result) {
            return Ok(None);
        }
    }
    Ok(Some((output, rows.len())))
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn standalone_budget(config: &ExtractionConfig, ocr: &OcrConfig, bounds: &StandaloneFrame) -> Option<usize> {
    let defaults = crate::extractors::security::SecurityLimits::default();
    let limits = config.security_limits.as_ref().unwrap_or(&defaults);
    let byte_limit = ocr.security_limits.as_ref().map_or(limits.max_content_size, |ocr| {
        limits.max_content_size.min(ocr.max_content_size)
    });
    let read_bytes = (bounds.processed.0 as usize)
        .checked_mul(bounds.processed.1 as usize)?
        .checked_mul(4)?;
    byte_limit.checked_sub(read_bytes)?.checked_sub(FIXED_WORK_BYTES)
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn decode_gray(bytes: &[u8], (width, height): (u32, u32)) -> Option<Vec<u8>> {
    use image::ImageFormat;
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    if !matches!(
        reader.format()?,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Bmp
    ) {
        return None;
    }
    if reader.into_dimensions().ok()? != (width, height) {
        return None;
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    Some(reader.decode().ok()?.into_luma8().into_raw())
}

/// The downscaled read's frame, as recorded by OCR preprocessing.
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
struct StandaloneFrame {
    dimensions: (u32, u32),
    processed: (u32, u32),
    dimension_clamped: bool,
    skipped_resize: bool,
    scale_factor: f64,
    original_dpi: (f64, f64),
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
impl StandaloneFrame {
    fn from_read(original: &ExtractedDocument) -> Option<Self> {
        let metadata = original.metadata.image_preprocessing.as_ref()?;
        let dimensions = (
            u32::try_from(metadata.original_dimensions.width).ok()?,
            u32::try_from(metadata.original_dimensions.height).ok()?,
        );
        let processed = metadata.new_dimensions.as_ref().map_or(dimensions, |new| {
            (
                u32::try_from(new.width).unwrap_or(dimensions.0),
                u32::try_from(new.height).unwrap_or(dimensions.1),
            )
        });
        let read = |key| {
            original
                .metadata
                .additional
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
        };
        // The recorded read frame, not just an intermediate resize, decides the geometry. ~keep
        if read(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY).is_some_and(|width| width != processed.0)
            || read(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY).is_some_and(|height| height != processed.1)
        {
            return None;
        }
        Some(Self {
            dimensions,
            processed,
            dimension_clamped: metadata.dimension_clamped,
            skipped_resize: metadata.skipped_resize,
            scale_factor: metadata.scale_factor,
            original_dpi: (metadata.original_dpi.horizontal, metadata.original_dpi.vertical),
        })
    }

    fn was_reduced(&self) -> bool {
        (self.dimension_clamped || self.scale_factor < 1.0)
            && !self.skipped_resize
            && self.dimensions.1 >= self.dimensions.0.saturating_mul(MIN_TALL_ASPECT)
            && self.processed.1 < self.dimensions.1
    }

    #[cfg(test)]
    fn frame(dimensions: (u32, u32), processed: (u32, u32), dpi: (f64, f64)) -> Self {
        Self {
            dimensions,
            processed,
            dimension_clamped: true,
            skipped_resize: false,
            scale_factor: f64::from(processed.1) / f64::from(dimensions.1),
            original_dpi: dpi,
        }
    }

    fn dpi(&self) -> Option<f64> {
        let (horizontal, vertical) = self.original_dpi;
        // A standalone picture's density is estimated from its glyph heights, so the two
        // axes can differ by a fraction of a percent on one scan. ~keep
        (horizontal.is_finite()
            && vertical.is_finite()
            && horizontal > 0.0
            && (horizontal - vertical).abs() <= horizontal.max(vertical) * 0.01)
            .then_some(horizontal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_blank_margin_offers_cuts_inside_the_plan_lookback() {
        let bands = [0..183, 8625..12_000];
        let split = split_long_bands(&bands, PLAN_CUT_BAND_ROWS);
        assert_eq!(split[0], 0..183);
        assert!(split.iter().all(|band| band.end - band.start <= PLAN_CUT_BAND_ROWS));
        assert_eq!(split.last().unwrap().end, 12_000);
        assert!(split.windows(2).all(|pair| pair[0].end == pair[1].start));
    }

    #[test]
    fn a_standalone_density_estimate_tolerates_a_fractional_axis_difference() {
        let frame = StandaloneFrame::frame((1275, 12_000), (435, 4096), (211.034, 210.938));
        assert_eq!(frame.dpi(), Some(211.034));
        let skewed = StandaloneFrame::frame((1275, 12_000), (435, 4096), (211.0, 200.0));
        assert_eq!(skewed.dpi(), None);
    }
}
