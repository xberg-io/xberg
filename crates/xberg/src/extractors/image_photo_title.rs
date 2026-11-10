use std::collections::HashSet;
use std::io::Cursor;
use std::sync::Arc;

use image::{ColorType, GrayImage, ImageDecoder, RgbImage};

use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::ocr::tesseract_backend::TesseractBackend;
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::plugins::OcrBackend;
use crate::types::formats::{ImagePreprocessingConfig, TesseractConfig};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{BoundingBox, ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};
use crate::{Result, XbergError};

#[path = "image_photo_title/duplicate.rs"]
mod duplicate;

const MAX_SOURCE_PIXELS: u32 = 2_500_000;
const MAX_SOURCE_AXIS: u32 = 4096;
const MIN_TITLE_HEIGHT: u32 = 30;
const MAX_TITLE_PIXELS_PER_THOUSAND: u32 = 35;
const BRIGHT_CUTOFFS: [u8; 2] = [235, 245];
const NEUTRAL_CHANNEL_SPREAD: u8 = 8;
const DARK_CUTOFF: u8 = 40;
const RING_RADIUS: u32 = 4;
const MASK_PADDING: u32 = 16;
const MIN_WORD_CONFIDENCE: f64 = 0.94;
const PRIMARY_PROTECTION_CONFIDENCE: f64 = 0.85;
const MASK_DPI: i32 = 150;
const MAX_TITLE_WORDS: usize = 4;
const MIN_TITLE_WORDS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Rect {
    fn size(self) -> (u32, u32) {
        (self.right - self.left, self.bottom - self.top)
    }

    fn contains(self, x: u32, y: u32) -> bool {
        self.left <= x && x < self.right && self.top <= y && y < self.bottom
    }

    fn intersects(self, other: Self) -> bool {
        self.left < other.right && other.left < self.right && self.top < other.bottom && other.top < self.bottom
    }

    fn from_word(word: &OcrElement) -> Option<Self> {
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = word.geometry
        else {
            return None;
        };
        (width > 0 && height > 0).then_some(Self {
            left,
            top,
            right: left.checked_add(width)?,
            bottom: top.checked_add(height)?,
        })
    }

    fn translate(self, frame: Self) -> Option<Self> {
        let (width, height) = frame.size();
        if self.right > width || self.bottom > height {
            return None;
        }
        Some(Self {
            left: self.left + frame.left,
            top: self.top + frame.top,
            right: self.right + frame.left,
            bottom: self.bottom + frame.top,
        })
    }
}

pub(super) fn eligible(config: &ExtractionConfig, ocr: &OcrConfig, mime_type: &str) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    mime_type.eq_ignore_ascii_case("image/png")
        && supports_output(config)
        && config.images.is_none()
        && !config.force_ocr
        && config.force_ocr_pages.is_none()
        && !config.include_document_structure
        && config.redaction.is_none()
        && config.structured_extraction.is_none()
        && is_default_english_profile(ocr)
}

fn supports_output(config: &ExtractionConfig) -> bool {
    match config.output_format {
        OutputFormat::Plain => true,
        OutputFormat::Markdown => {
            #[cfg(feature = "redaction")]
            if crate::text::redaction::external::external_redaction_is_scoped() {
                return false;
            }
            config.escape_markdown && config.chunking.is_none()
        }
        _ => false,
    }
}

fn is_default_english_profile(ocr: &OcrConfig) -> bool {
    ocr.language == ["eng"]
        && ocr.tesseract_config.is_none()
        && ocr.backend_options.is_none()
        && ocr.element_config.is_none()
        && ocr.output_format.is_none()
        && ocr.quality_thresholds.as_ref().is_none_or(|thresholds| {
            let default = crate::core::config::OcrQualityThresholds {
                prune_symbol_noise_lines: thresholds.prune_symbol_noise_lines,
                ..Default::default()
            };
            serde_json::to_value(thresholds).ok() == serde_json::to_value(&default).ok()
        })
        && ocr.tessdata_path.is_none()
        && ocr.tessdata_bytes.is_none()
        && ocr.pipeline.is_none()
        && ocr.auto_rotate != Some(true)
        && !ocr.numeric_repair
        && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
}

fn processed_frame(document: &ExtractedDocument, psm: &str) -> Option<(u32, u32)> {
    let metadata = &document.metadata.additional;
    if metadata.get("psm")?.as_str()? != psm {
        return None;
    }
    let dimension = |key| u32::try_from(metadata.get(key)?.as_u64()?).ok();
    Some((
        dimension(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)?,
        dimension(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)?,
    ))
}

fn check_cancellation(cancel: Option<&CancellationToken>) -> Result<()> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        return Err(XbergError::Cancelled);
    }
    Ok(())
}

fn decode_rgb(bytes: &[u8], frame: (u32, u32), budget: usize) -> Option<RgbImage> {
    let (width, height) = frame;
    let pixels = width.checked_mul(height)?;
    if pixels == 0 || pixels > MAX_SOURCE_PIXELS || width > MAX_SOURCE_AXIS || height > MAX_SOURCE_AXIS {
        return None;
    }
    let output = (pixels as usize).checked_mul(3)?;
    let decoder_budget = budget.checked_sub(output)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    limits.max_alloc = Some(decoder_budget.try_into().ok()?);
    // The IHDR color type excludes transparent and palette images before decoding.
    if bytes.get(24..26)? != [8, 2] {
        return None;
    }
    let decoder = image::codecs::png::PngDecoder::with_limits(Cursor::new(bytes), limits).ok()?;
    if decoder.dimensions() != frame || decoder.color_type() != ColorType::Rgb8 || decoder.is_apng().ok()? {
        return None;
    }
    let mut rgb = vec![0; output];
    decoder.read_image(&mut rgb).ok()?;
    RgbImage::from_raw(width, height, rgb)
}

fn near_white(pixel: [u8; 3], cutoff: u8) -> bool {
    let [red, green, blue] = pixel;
    let low = red.min(green).min(blue);
    let high = red.max(green).max(blue);
    low >= cutoff && high - low <= NEUTRAL_CHANNEL_SPREAD
}

fn title_support(rgb: &RgbImage, cutoff: u8) -> Option<Rect> {
    let (width, height) = rgb.dimensions();
    let max_pixels = width * height * MAX_TITLE_PIXELS_PER_THOUSAND / 1000;
    let mut count = 0;
    let mut bounds = Rect {
        left: width,
        top: height,
        right: 0,
        bottom: 0,
    };
    for (x, y, pixel) in rgb.enumerate_pixels() {
        if near_white(pixel.0, cutoff) {
            count += 1;
            if count > max_pixels {
                return None;
            }
            bounds.left = bounds.left.min(x);
            bounds.top = bounds.top.min(y);
            bounds.right = bounds.right.max(x + 1);
            bounds.bottom = bounds.bottom.max(y + 1);
        }
    }
    if count < (width * height / 1000).max(1) {
        return None;
    }
    let (span, depth) = bounds.size();
    (depth >= MIN_TITLE_HEIGHT && depth <= height / 4 && span >= width / 10 && span <= width * 9 / 10).then_some(bounds)
}

fn dark_ring(rgb: &RgbImage, frame: Rect, mask: &GrayImage) -> bool {
    let (width, height) = frame.size();
    let mut ring = vec![false; (width * height) as usize];
    for (x, y, _) in mask.enumerate_pixels().filter(|(_, _, p)| p.0[0] == 0) {
        let left = x.saturating_sub(RING_RADIUS);
        let right = (x + RING_RADIUS + 1).min(width);
        for row in y.saturating_sub(RING_RADIUS)..(y + RING_RADIUS + 1).min(height) {
            ring[(row * width + left) as usize..(row * width + right) as usize].fill(true);
        }
    }
    let mut exterior = 0;
    let mut dark = 0;
    for ((x, y, pixel), in_ring) in mask.enumerate_pixels().zip(ring) {
        if in_ring && pixel.0[0] != 0 {
            exterior += 1;
            dark += u32::from(
                rgb.get_pixel(frame.left + x, frame.top + y)
                    .0
                    .iter()
                    .all(|&v| v <= DARK_CUTOFF),
            );
        }
    }
    exterior > 0 && dark * 4 >= exterior * 3
}

struct Masks {
    frame: Rect,
    masks: [GrayImage; 2],
    available_ocr_budget: usize,
}

fn make_masks(bytes: &[u8], frame: (u32, u32), budget: usize) -> Option<Masks> {
    // The caller still borrows the source while the worker owns its encoded copy.
    let budget = budget.checked_sub(bytes.len().checked_mul(2)?)?;
    let rgb = decode_rgb(bytes, frame, budget)?;
    let [low, high] = BRIGHT_CUTOFFS;
    let bounds = title_support(&rgb, low)?;
    if title_support(&rgb, high)? != bounds {
        return None;
    }
    let crop = Rect {
        left: bounds.left.checked_sub(MASK_PADDING)?,
        top: bounds.top.checked_sub(MASK_PADDING)?,
        right: bounds.right + MASK_PADDING,
        bottom: bounds.bottom + MASK_PADDING,
    };
    if crop.right > frame.0 || crop.bottom > frame.1 {
        return None;
    }
    let (width, height) = crop.size();
    let crop_bytes = (width * height) as usize;
    let available_ocr_budget = budget.checked_sub(2 * crop_bytes)?;
    // The decoder is gone before allocating the pair of masks and the ring.
    if available_ocr_budget < rgb.as_raw().len().checked_add(crop_bytes)? {
        return None;
    }
    let make = |cutoff| {
        let mask = GrayImage::from_fn(width, height, |x, y| {
            let white = near_white(rgb.get_pixel(crop.left + x, crop.top + y).0, cutoff);
            image::Luma([if white { 0 } else { 255 }])
        });
        dark_ring(&rgb, crop, &mask).then_some(mask)
    };
    let masks = [make(low)?, make(high)?];
    if masks[0] == masks[1] {
        return None;
    }
    Some(Masks {
        frame: crop,
        masks,
        available_ocr_budget,
    })
}

#[derive(PartialEq, Eq)]
struct MaskWord {
    text: String,
    bounds: Rect,
}

fn confident_word(element: &OcrElement) -> Option<MaskWord> {
    let text = &element.text;
    if !(3..=24).contains(&text.len())
        || !text.bytes().all(|byte| byte.is_ascii_uppercase())
        || !(MIN_WORD_CONFIDENCE..=1.0).contains(&element.confidence.recognition)
    {
        return None;
    }
    Some(MaskWord {
        text: text.clone(),
        bounds: Rect::from_word(element)?,
    })
}

fn certify_words(document: &ExtractedDocument, mask: &GrayImage) -> Option<Vec<MaskWord>> {
    if processed_frame(document, "7")? != mask.dimensions() || !document.tables.is_empty() {
        return None;
    }
    let preprocessing = document.metadata.image_preprocessing.as_ref()?;
    if !preprocessing.skipped_resize || preprocessing.new_dimensions.is_some() || preprocessing.resize_error.is_some() {
        return None;
    }
    let elements = document.ocr_elements.as_deref()?;
    let mut lines = elements.iter().filter(|element| element.level == OcrElementLevel::Line);
    let line = lines.next()?;
    if lines.next().is_some() {
        return None;
    }
    let words = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .take(MAX_TITLE_WORDS + 1)
        .map(confident_word)
        .collect::<Option<Vec<_>>>()?;
    if !(MIN_TITLE_WORDS..=MAX_TITLE_WORDS).contains(&words.len()) {
        return None;
    }
    let line_text = words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if line.text.trim() != line_text || !is_complete_mask_line(mask, &words) {
        return None;
    }
    Some(words)
}

fn is_complete_mask_line(mask: &GrayImage, words: &[MaskWord]) -> bool {
    if words
        .iter()
        .any(|word| word.bounds.right > mask.width() || word.bounds.bottom > mask.height())
    {
        return false;
    }
    if words.windows(2).any(|pair| {
        let [first, second] = [pair[0].bounds, pair[1].bounds];
        let max_height = first.size().1.max(second.size().1);
        let overlap = first
            .bottom
            .min(second.bottom)
            .saturating_sub(first.top.max(second.top));
        first.right > second.left || second.left - first.right > 2 * max_height || overlap * 2 < max_height
    }) {
        return false;
    }
    mask.enumerate_pixels()
        .all(|(x, y, pixel)| pixel.0[0] != 0 || words.iter().any(|word| word.bounds.contains(x, y)))
}

fn word_envelope(words: &[MaskWord]) -> Option<Rect> {
    let (first, rest) = words.split_first()?;
    Some(rest.iter().fold(first.bounds, |mut bounds, word| {
        bounds.right = word.bounds.right;
        bounds.top = bounds.top.min(word.bounds.top);
        bounds.bottom = bounds.bottom.max(word.bounds.bottom);
        bounds
    }))
}

fn overlaps_primary(words: &[MaskWord], crop: Rect, original: &[OcrElement]) -> Option<bool> {
    let mut original_words = original
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .peekable();
    original_words.peek()?;
    let source_bounds = words
        .iter()
        .map(|word| word.bounds.translate(crop))
        .collect::<Option<Vec<_>>>()?;
    let envelope = word_envelope(words)?.translate(crop)?;
    let joined_title: String = words.iter().map(|word| word.text.as_str()).collect();
    for element in original_words {
        let bounds = Rect::from_word(element)?;
        if !envelope.intersects(bounds) {
            continue;
        }
        let literal = element.text.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let fused_title = literal
            .bytes()
            .filter(u8::is_ascii_alphanumeric)
            .map(|byte| byte.to_ascii_uppercase())
            .eq(joined_title.bytes());
        let protected = element.confidence.recognition >= PRIMARY_PROTECTION_CONFIDENCE
            || literal.bytes().any(|byte| byte.is_ascii_digit())
            || fused_title;
        let repeats_word = words
            .iter()
            .zip(&source_bounds)
            .any(|(word, source)| source.intersects(bounds) && word.text.eq_ignore_ascii_case(literal));
        if protected || repeats_word {
            return Some(true);
        }
    }
    Some(false)
}

async fn read_mask(
    backend: &TesseractBackend,
    primary_config: &OcrConfig,
    mask: &GrayImage,
    budget: usize,
) -> Result<Option<Vec<MaskWord>>> {
    // Binary PGM preserves every mask byte without an encoder scratch buffer.
    let header = format!("P5\n{} {}\n255\n", mask.width(), mask.height());
    let encoded_size = header.len() + mask.as_raw().len();
    if encoded_size >= budget {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(encoded_size);
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(mask.as_raw());
    let Some(budget) = budget.checked_sub(bytes.capacity()) else {
        return Ok(None);
    };
    let mut config = primary_config.clone();
    config.backend_options = Some(serde_json::json!({ "source_dpi": MASK_DPI, "use_cache": false }));
    config.security_limits.get_or_insert_default().max_content_size = budget;
    config.tesseract_config = Some(TesseractConfig {
        psm: Some(7),
        enable_table_detection: false,
        use_cache: false,
        preprocessing: Some(ImagePreprocessingConfig {
            target_dpi: MASK_DPI,
            deskew: false,
            binarization_method: "none".into(),
            ..Default::default()
        }),
        ..Default::default()
    });
    super::enable_image_ocr_elements(&mut config, true);
    let evidence = backend.process_image_owned(Arc::new(bytes), &config).await?;
    Ok(certify_words(&evidence, mask))
}

pub(super) struct Title {
    text: String,
    bounds: Rect,
    duplicate_glyph: Option<char>,
}

pub(super) async fn recover_if_eligible(
    source: Option<(&TesseractBackend, Arc<Vec<u8>>)>,
    config: &OcrConfig,
    primary: &ExtractedDocument,
    cancel: Option<&CancellationToken>,
) -> Result<Option<Title>> {
    let Some((backend, bytes)) = source else {
        return Ok(None);
    };
    match recover(backend, bytes, config, primary, cancel).await {
        Ok(title) => Ok(title),
        Err(error @ XbergError::Cancelled) => Err(error),
        Err(error) => {
            crate::core::extractor::cacheability::invalidate_current();
            tracing::warn!(%error, "standalone photo title recovery failed");
            Ok(None)
        }
    }
}

async fn recover(
    backend: &TesseractBackend,
    bytes: Arc<Vec<u8>>,
    config: &OcrConfig,
    primary: &ExtractedDocument,
    cancel: Option<&CancellationToken>,
) -> Result<Option<Title>> {
    if !primary.tables.is_empty()
        || !primary.formulas.is_empty()
        || primary.document.is_some()
        || primary.metadata.image_preprocessing.is_some()
    {
        return Ok(None);
    }
    let Some(frame) = processed_frame(primary, "11") else {
        return Ok(None);
    };
    let Some(original) = primary.ocr_elements.as_deref() else {
        return Ok(None);
    };
    check_cancellation(cancel)?;
    let budget = config.security_limits.clone().unwrap_or_default().max_content_size;
    #[cfg(feature = "tokio-runtime")]
    let masks = {
        let cancel = cancel.cloned();
        tokio::task::spawn_blocking(move || {
            check_cancellation(cancel.as_ref())?;
            Ok::<_, XbergError>(make_masks(&bytes, frame, budget))
        })
        .await
        .map_err(|error| XbergError::Other(format!("standalone title mask worker failed: {error}")))??
    };
    #[cfg(not(feature = "tokio-runtime"))]
    let masks = make_masks(&bytes, frame, budget);
    check_cancellation(cancel)?;
    let Some(Masks {
        frame,
        masks: [low, high],
        available_ocr_budget,
    }) = masks
    else {
        return Ok(None);
    };
    let words = read_mask(backend, config, &low, available_ocr_budget).await;
    check_cancellation(cancel)?;
    let Some(words) = words? else { return Ok(None) };
    if overlaps_primary(&words, frame, original) != Some(false) {
        return Ok(None);
    }
    let evidence = read_mask(backend, config, &high, available_ocr_budget).await;
    check_cancellation(cancel)?;
    if evidence?.as_deref() != Some(words.as_slice()) {
        return Ok(None);
    }
    let Some(bounds) = word_envelope(&words).and_then(|bounds| bounds.translate(frame)) else {
        return Ok(None);
    };
    let duplicate_glyph = duplicate::duplicate_title_glyph(&words, [&low, &high], frame, original);
    let text = words.into_iter().map(|word| word.text).collect::<Vec<_>>().join(" ");
    Ok(Some(Title {
        text,
        bounds,
        duplicate_glyph,
    }))
}

fn preserves_markdown_page(document: &InternalDocument) -> bool {
    document.pre_rendered_content.is_none()
        && document.annotations.as_ref().is_none_or(Vec::is_empty)
        && document.images.is_empty()
        && document.elements.iter().all(|element| {
            element.page.is_none()
                && element.annotations.is_empty()
                && matches!(
                    element.kind,
                    ElementKind::Paragraph | ElementKind::Heading { .. } | ElementKind::Image { image_index: 0 }
                )
        })
}

fn text_lines(text: &str) -> impl Iterator<Item = (usize, &str)> + Clone {
    text.split_inclusive('\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line.trim()))
        })
        .filter(|(_, line)| !line.is_empty())
}

fn matching_line<'a, Position: Copy>(
    mut lines: impl Iterator<Item = (Position, &'a str)> + Clone,
    recognized: &[&str],
    anchor: usize,
) -> Option<Position> {
    if lines.clone().map(|(_, line)| line).eq(recognized.iter().copied()) {
        return lines.nth(anchor).map(|(position, _)| position);
    }

    // If the renderer regrouped or removed lines, a repeated label cannot identify an occurrence.
    let target = recognized[anchor];
    if recognized.iter().filter(|&&line| line == target).count() != 1 {
        return None;
    }
    let mut matches = lines.filter_map(|(position, line)| (line == target).then_some(position));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn title_insertion(
    page: &str,
    elements: &[InternalElement],
    ocr_elements: &[OcrElement],
    bounds: Rect,
) -> Option<(usize, (usize, usize))> {
    let page_lines = text_lines(page);
    let element_lines = elements
        .iter()
        .enumerate()
        .flat_map(|(index, element)| text_lines(&element.text).map(move |(offset, line)| ((index, offset), line)));
    let emitted: HashSet<_> = page_lines
        .clone()
        .map(|(_, line)| line)
        .chain(element_lines.clone().map(|(_, line)| line))
        .collect();
    let lines = ocr_elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Line && !element.text.trim().is_empty());
    let anchor = lines.clone().position(|element| {
        Rect::from_word(element).is_some_and(|line| line.top >= bounds.bottom) && emitted.contains(element.text.trim())
    })?;
    let recognized: Vec<_> = lines.map(|element| element.text.trim()).collect();
    let page_offset = matching_line(page_lines, &recognized, anchor)?;
    let (index, offset) = matching_line(element_lines, &recognized, anchor)?;
    let element = &elements[index];
    if offset != 0
        && (element.kind != ElementKind::Paragraph || !element.annotations.is_empty() || element.attributes.is_some())
    {
        return None;
    }
    Some((page_offset, (index, offset)))
}

/// Places a recovered title; when symbol-noise pruning emptied the page (`pruned` carries the
/// page confidence), the title becomes the whole page.
pub(super) fn place_recovered(
    document: &mut InternalDocument,
    title: Title,
    output_format: &OutputFormat,
    pruned: Option<Option<crate::types::page::PageOcrConfidence>>,
) {
    match pruned {
        Some(confidence) if document.prebuilt_pages.is_none() => place_alone(document, title, confidence),
        _ => place(document, title, output_format),
    }
}

fn place_alone(
    document: &mut InternalDocument,
    title: Title,
    ocr_confidence: Option<crate::types::page::PageOcrConfidence>,
) {
    let mut element = InternalElement::text(ElementKind::Paragraph, title.text.clone(), 0);
    element.bbox = Some(BoundingBox {
        x0: title.bounds.left.into(),
        y0: title.bounds.top.into(),
        x1: title.bounds.right.into(),
        y1: title.bounds.bottom.into(),
    });
    document.push_element(element);
    document.prebuilt_pages = Some(vec![crate::types::PageContent {
        page_number: 1,
        content: title.text,
        tables: vec![],
        image_indices: vec![],
        image_preprocessing: None,
        hierarchy: None,
        is_blank: None,
        layout_regions: None,
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence,
        native_content: None,
    }]);
}

pub(super) fn place(document: &mut InternalDocument, title: Title, output_format: &OutputFormat) {
    // Tagged elements would make Markdown rerender the page instead of keeping its raw OCR body.
    if *output_format == OutputFormat::Markdown && !preserves_markdown_page(document) {
        return;
    }
    let Some([page]) = document.prebuilt_pages.as_deref_mut() else {
        return;
    };
    if page.page_number != 1
        || !page.tables.is_empty()
        || page.content.trim().is_empty()
        || page.content.contains(&title.text)
        || document
            .elements
            .iter()
            .any(|element| element.text.contains(&title.text))
    {
        return;
    }
    let insertion = document
        .prebuilt_ocr_elements
        .as_deref()
        .and_then(|ocr_elements| title_insertion(&page.content, &document.elements, ocr_elements, title.bounds));
    let mut element = InternalElement::text(ElementKind::Paragraph, title.text, 0);
    element.bbox = Some(BoundingBox {
        x0: title.bounds.left.into(),
        y0: title.bounds.top.into(),
        x1: title.bounds.right.into(),
        y1: title.bounds.bottom.into(),
    });
    if let Some(mut insertion) = insertion {
        if let Some(removal) = title
            .duplicate_glyph
            .and_then(|glyph| duplicate::duplicate_removal(&page.content, &document.elements, glyph))
        {
            duplicate::remove_duplicate(&mut page.content, &mut document.elements, removal, &mut insertion);
        }
        let (page_offset, (index, offset)) = insertion;
        let paragraph = format!("{}\n\n", element.text);
        page.content.insert_str(page_offset, &paragraph);
        if offset == 0 {
            document.elements.insert(index, element);
        } else {
            document.elements[index].text.insert_str(offset, &paragraph);
        }
        return;
    }
    if !page.content.ends_with('\n') {
        page.content.push('\n');
    }
    if !page.content.ends_with("\n\n") {
        page.content.push('\n');
    }
    page.content.push_str(&element.text);
    document.push_element(element);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OcrConfidence;

    fn words() -> [MaskWord; 2] {
        [
            MaskWord {
                text: "SECTION".to_string(),
                bounds: Rect {
                    left: 0,
                    top: 0,
                    right: 90,
                    bottom: 25,
                },
            },
            MaskWord {
                text: "TITLE".to_string(),
                bounds: Rect {
                    left: 110,
                    top: 0,
                    right: 170,
                    bottom: 25,
                },
            },
        ]
    }

    fn primary(text: &str, left: u32, width: u32, confidence: f64) -> OcrElement {
        OcrElement::new(
            text,
            OcrBoundingGeometry::Rectangle {
                left,
                top: 200,
                width,
                height: 25,
            },
            OcrConfidence::from_tesseract(confidence),
        )
        .with_level(OcrElementLevel::Word)
    }

    const CROP: Rect = Rect {
        left: 100,
        top: 200,
        right: 280,
        bottom: 250,
    };

    fn line(text: &str, top: u32) -> OcrElement {
        OcrElement::new(
            text,
            OcrBoundingGeometry::Rectangle {
                left: 100,
                top,
                width: 90,
                height: 20,
            },
            OcrConfidence::from_tesseract(98.0),
        )
        .with_level(OcrElementLevel::Line)
    }

    #[test]
    fn places_title_before_a_repeated_line_inside_a_paragraph() {
        let page = "Same\n\nSame";
        let elements = [InternalElement::text(ElementKind::Paragraph, "Same\nSame", 0)];
        let recognized = [line("Same", 150), line("Same", 300)];
        assert_eq!(title_insertion(page, &elements, &recognized, CROP), Some((6, (0, 5))));
    }

    #[test]
    fn skips_a_recognized_line_absent_from_both_outputs() {
        let page = "Above\n\nBelow";
        let elements = [InternalElement::text(ElementKind::Paragraph, "Above\nBelow", 0)];
        let recognized = [line("Above", 150), line("Filtered", 300), line("Below", 400)];
        assert_eq!(title_insertion(page, &elements, &recognized, CROP), Some((7, (0, 6))));
    }

    #[test]
    fn protects_different_confident_primary_words_in_the_title_envelope() {
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("BOUNDARY", 115, 70, 85.0)]),
            Some(true)
        );
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("BOUNDARY", 275, 35, 99.0)]),
            Some(false)
        );
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("Ke", 115, 70, 75.36)]),
            Some(false)
        );
    }

    #[test]
    fn protects_fused_title_even_at_low_confidence_when_the_same_source_overlaps() {
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("Section–Title", 115, 145, 42.0)]),
            Some(true)
        );
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("Section–Title", 275, 90, 42.0)]),
            Some(false)
        );
    }

    fn mask_source(interior: u8) -> Vec<u8> {
        let mut rgb = RgbImage::from_pixel(500, 300, image::Rgb([8, 8, 8]));
        for x in (130..161).chain(180..211) {
            for y in 100..160 {
                rgb.put_pixel(x, y, image::Rgb([255; 3]));
            }
        }
        rgb.put_pixel(140, 110, image::Rgb([interior; 3]));
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn refuses_two_identical_masks_but_accepts_distinct_support() {
        let frame = (500, 300);
        let budget = 8 * 1024 * 1024;
        assert!(make_masks(&mask_source(240), frame, budget).is_some());
        assert!(make_masks(&mask_source(255), frame, budget).is_none());
    }

    #[test]
    fn protects_amounts_between_certified_title_words() {
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("42", 195, 10, 25.0)]),
            Some(true)
        );
        assert_eq!(
            overlaps_primary(&words(), CROP, &[primary("42", 275, 10, 25.0)]),
            Some(false)
        );
    }
}
