use crate::core::config::{ExtractionConfig, OcrStrategy, OutputFormat, VlmFallbackPolicy};
use crate::ocr::processor::{light_page_foreground_mode, lower_mode_mask_has_text_structure};
use crate::plugins::OcrBackend;
use image::{GrayImage, ImageBuffer, Luma, Rgb, imageops};
use std::ops::Range;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::content::Operator;
use xberg_native_pdf::content::parser::visit_complete_content_stream;
use xberg_native_pdf::decoders::{Ascii85Decoder, DecodeParams, FlateDecoder, StreamDecoder, decode_predictor};
use xberg_native_pdf::document::{PageRotation, ReadingOrder};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

const MAX_CONTENT_BYTES: usize = 64 * 1024;
const MAX_OPERATORS: usize = 4096;
const MAX_PREVIEW_PIXELS: usize = 1024 * 1024;
const MIN_TRAILING_INK_RUNS: usize = 4;
const MIN_TRAILING_WIDTH_IN_LINE_HEIGHTS: usize = 3;

// This lane replaces literal text only. Caller-selected OCR, layout and projection policies
// retain their original routes. ~keep
pub(super) fn stock_backend(config: &ExtractionConfig) -> Option<Arc<dyn OcrBackend>> {
    let ocr = config.ocr.as_ref()?;
    if !preserves_requested_projection(config)
        || ocr.backend != "tesseract"
        || ocr.tesseract_config.is_some()
        || ocr.pipeline.is_some()
        || ocr.backend_options.is_some()
        || ocr.output_format.is_some()
        || ocr.element_config.is_some()
        || ocr.quality_thresholds.is_some()
        || ocr.paddle_ocr_config.is_some()
        || ocr.paddle_ocr_settings.is_some()
        || ocr.auto_rotate
        || ocr.numeric_repair
        || !matches!(ocr.vlm_fallback, VlmFallbackPolicy::Disabled)
    {
        return None;
    }
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let registry = registry.read();
    let backend = registry.get(&ocr.backend).ok()?;
    registry.is_builtin_tesseract(&backend).then_some(backend)
}

fn preserves_requested_projection(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    config.output_format == OutputFormat::Plain
        && config.ocr_strategy == OcrStrategy::Auto
        && !config.include_document_structure
        && config.content_filter.is_none()
        && config.images.as_ref().is_none_or(|images| !images.include_page_rasters)
        && config.ocr_near_empty_fallback != Some(false)
        && config.pdf_options.as_ref().is_none_or(|pdf| {
            pdf.top_margin_fraction.is_none()
                && pdf.bottom_margin_fraction.is_none()
                && !pdf.reading_order
                && pdf.backend == crate::core::config::PdfBackend::Native
                && pdf.hierarchy.is_none()
                && !pdf.ocr_inline_images
        })
}

pub(super) fn preview_byte_limit(config: &ExtractionConfig) -> usize {
    let default_limits = crate::extractors::security::SecurityLimits::default();
    let limit = config
        .security_limits
        .as_ref()
        .unwrap_or(&default_limits)
        .max_content_size;
    config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.security_limits.as_ref())
        .map_or(limit, |ocr_limits| limit.min(ocr_limits.max_content_size))
}

pub(super) fn preview_page_limit(config: &ExtractionConfig) -> usize {
    let default_limits = crate::extractors::security::SecurityLimits::default();
    let limit = config
        .security_limits
        .as_ref()
        .unwrap_or(&default_limits)
        .max_iterations;
    config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.security_limits.as_ref())
        .map_or(limit, |ocr_limits| limit.min(ocr_limits.max_iterations))
}

pub(super) fn needs_ocr(
    doc: &PdfDocument,
    page: usize,
    byte_limit: usize,
    is_cancelled: impl Fn() -> bool,
) -> Option<bool> {
    if is_cancelled() {
        return None;
    }
    let (frame, content) = page_frame_and_contents(doc, page)?;
    let name = audited_image_name(doc, page, &content, byte_limit)?;
    let source = image_source(doc, page, &name)?;
    let handles = doc.page_image_handles(page).ok()?;
    let [handle] = handles.as_slice() else {
        return None;
    };
    if handle.is_inline
        || handle.width != source.width
        || handle.height != source.height
        || handle.rotation_degrees != 0.0
        || !covers_frame(handle.bbox, frame)
        || is_cancelled()
    {
        return None;
    }
    let size = preview_size(handle.bbox, &source, byte_limit)?;
    let boxes = native_boxes(doc, page, frame)?;
    if is_cancelled() {
        return None;
    }
    let samples = source.decode_samples(byte_limit)?;
    let preview = gray_preview(&samples, &source, size)?;
    Some(!is_cancelled() && has_uncovered_text(&preview, handle.bbox, &boxes))
}

pub(super) fn page_frame_and_contents(doc: &PdfDocument, page: usize) -> Option<(Rect, Object)> {
    if doc.is_encrypted()
        || !matches!(
            doc.get_page_rotation_status(page).ok()?,
            PageRotation::Absent | PageRotation::Valid(0)
        )
    {
        return None;
    }
    let page_object = doc.get_page(page).ok()?;
    let dictionary = page_object.as_dict()?;
    if ["Annots", "Group", "UserUnit"]
        .iter()
        .any(|key| dictionary.contains_key(*key))
    {
        return None;
    }
    let media = resolved_page_box(doc, dictionary.get("MediaBox")?)?;
    if let Some(crop) = dictionary.get("CropBox")
        && resolved_page_box(doc, crop)? != media
    {
        return None;
    }
    let contents = doc.resolve_object(dictionary.get("Contents")?).ok()?;
    Some((media, contents))
}

fn audited_image_name(doc: &PdfDocument, page: usize, content: &Object, byte_limit: usize) -> Option<String> {
    let Object::Stream { dict, data } = content else {
        return None;
    };
    if dict.contains_key("DecodeParms") {
        return None;
    }
    let filters = stream_filters(dict.get("Filter"))?;
    let output_limit = MAX_CONTENT_BYTES.min(byte_limit / 4);
    let bytes = decode_probe_stream(data, filters, output_limit, byte_limit)?;
    let fonts = simple_font_resources(doc, page)?;
    let source = hidden_text_image_name(&bytes, |font| fonts.contains_key(font))?;
    // Subsequent Native inspection uses the same immutable single stream. Require
    // its ordinary decoder to agree with the audited bytes before trusting geometry.
    (doc.get_page_content_data(page).ok()? == bytes).then_some(source)
}

/// Certify one literal native line painted above a raster by a simple page stream.
pub(super) fn has_visible_native_line(doc: &PdfDocument, layer: &str, byte_limit: usize) -> bool {
    const MAX_AUDITED_LINE_BYTES: usize = 128;
    if layer.len() > MAX_AUDITED_LINE_BYTES {
        return false;
    }
    let verify = || {
        let (frame, content) = page_frame_and_contents(doc, 0)?;
        let Object::Stream { dict, data } = content else {
            return None;
        };
        if dict.contains_key("DecodeParms") {
            return None;
        }
        let filters = stream_filters(dict.get("Filter"))?;
        let bytes = decode_probe_stream(&data, filters, MAX_CONTENT_BYTES.min(byte_limit / 4), byte_limit)?;
        let fonts = simple_font_resources(doc, 0)?;
        let image = painted_literal_image(&bytes, layer, |font| fonts.contains_key(font))?;
        let resources = doc.get_page_resources(0).ok()?;
        let xobjects = doc.resolve_object(resources.as_dict()?.get("XObject")?).ok()?;
        let Object::Stream { dict, .. } = doc.resolve_object(xobjects.as_dict()?.get(&image)?).ok()? else {
            return None;
        };
        if dict.get("Subtype").and_then(Object::as_name) != Some("Image") || doc.get_page_content_data(0).ok()? != bytes
        {
            return None;
        }
        let images = doc.page_image_handles(0).ok()?;
        let [image] = images.as_slice() else { return None };
        if image.is_inline || image.rotation_degrees != 0.0 || !covers_frame(image.bbox, frame) {
            return None;
        }
        native_line_glyphs(doc, layer, frame)
    };
    let Some(glyphs) = verify() else { return false };
    match xberg_native_pdf::rendering::exact_text_show_has_contrast(doc, layer.as_bytes(), &glyphs, byte_limit) {
        Ok(visible) => visible,
        Err(error) => {
            tracing::debug!(%error, "could not verify painted native line contrast");
            false
        }
    }
}

fn native_line_glyphs(doc: &PdfDocument, layer: &str, frame: Rect) -> Option<Vec<Rect>> {
    let chars = doc.extract_chars(0).ok()?;
    if !chars
        .iter()
        .map(|character| character.char)
        .filter(|c| !c.is_whitespace())
        .eq(layer.chars().filter(|c| !c.is_whitespace()))
    {
        return None;
    }
    chars
        .iter()
        .filter(|character| !character.char.is_whitespace())
        .map(|character| {
            let bounds = character.bbox;
            (character.rotation_degrees == 0.0
                && finite_box(bounds)
                && bounds.left() >= frame.left()
                && bounds.right() <= frame.right()
                && bounds.top() >= frame.top()
                && bounds.bottom() <= frame.bottom())
            .then_some(bounds)
        })
        .collect()
}

fn painted_literal_image(content: &[u8], layer: &str, font_is_known: impl Fn(&str) -> bool) -> Option<String> {
    let mut depth = 0usize;
    let mut inside_text = false;
    let mut known_font = false;
    let mut painted = false;
    let mut image = None;
    let complete = visit_complete_content_stream(content, MAX_OPERATORS, |op| {
        match op {
            Operator::SaveState if !inside_text && !painted => depth += 1,
            Operator::RestoreState if !inside_text && depth > 0 => depth -= 1,
            Operator::Cm { a, b, c, d, e, f } if !inside_text && !painted && positive_axis_matrix(a, b, c, d, e, f) => {
            }
            Operator::Do { name } if !inside_text && !painted && image.is_none() => image = Some(name),
            Operator::SetFillRgb { r, g, b }
                if !inside_text && [r, g, b].into_iter().all(|v| (0.0..=1.0).contains(&v)) => {}
            Operator::BeginText if !inside_text && depth == 0 => inside_text = true,
            Operator::EndText if inside_text => inside_text = false,
            Operator::Tf { font, size } if inside_text && size.is_finite() && size > 0.0 && font_is_known(&font) => {
                known_font = true;
            }
            Operator::Tr { render: 0 } | Operator::TStar if inside_text => {}
            Operator::TL { leading } if inside_text && leading.is_finite() => {}
            Operator::Tm { a, b, c, d, e, f } if inside_text && positive_axis_matrix(a, b, c, d, e, f) => {}
            Operator::Tj { text }
                if inside_text
                    && known_font
                    && !painted
                    && image.is_some()
                    && text.iter().all(|byte| byte.is_ascii_alphanumeric() || *byte == b' ')
                    && std::str::from_utf8(&text).is_ok_and(|text| text.trim() == layer) =>
            {
                painted = true;
            }
            _ => return false,
        }
        true
    })
    .ok()?;
    (complete && depth == 0 && !inside_text && painted)
        .then_some(image)
        .flatten()
}

fn positive_axis_matrix(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> bool {
    a.is_finite() && d.is_finite() && a > 0.0 && d > 0.0 && b == 0.0 && c == 0.0 && e.is_finite() && f.is_finite()
}

fn resolved_page_box(doc: &PdfDocument, object: &Object) -> Option<Rect> {
    let resolved = doc.resolve_object(object).ok()?;
    let [x0, y0, x1, y1] = resolved.as_array()?.as_slice() else {
        return None;
    };
    let number = |value: &Object| match doc.resolve_object(value).ok()? {
        Object::Real(value) => Some(value as f32),
        Object::Integer(value) => Some(value as f32),
        _ => None,
    };
    let (x0, y0, x1, y1) = (number(x0)?, number(y0)?, number(x1)?, number(y1)?);
    let bounds = Rect::new(x0, y0, x1 - x0, y1 - y0);
    finite_box(bounds).then_some(bounds)
}

fn hidden_text_image_name(content: &[u8], font_is_known: impl Fn(&str) -> bool) -> Option<String> {
    let mut mode = 0;
    let mut known_font = false;
    let mut inside_text = false;
    let mut states = Vec::new();
    let mut image = None;
    let mut hidden = false;
    let complete = visit_complete_content_stream(content, MAX_OPERATORS, |op| {
        match op {
            Operator::SaveState if !inside_text => states.push((mode, known_font)),
            Operator::RestoreState if !inside_text => {
                let Some((previous_mode, previous_font)) = states.pop() else {
                    return false;
                };
                mode = previous_mode;
                known_font = previous_font;
            }
            Operator::Cm { a, b, c, d, e, f } if !inside_text && positive_axis_matrix(a, b, c, d, e, f) => {}
            Operator::Do { name } if !inside_text && image.is_none() => image = Some(name),
            Operator::BeginText if !inside_text => inside_text = true,
            Operator::EndText if inside_text => inside_text = false,
            Operator::Tr { render } if inside_text => mode = render,
            Operator::Tf { font, size } if inside_text && size.is_finite() && size > 0.0 && font_is_known(&font) => {
                known_font = true;
            }
            Operator::Tj { .. } | Operator::TJ { .. } | Operator::Quote { .. } | Operator::DoubleQuote { .. }
                if inside_text && mode == 3 && known_font =>
            {
                hidden = true
            }
            Operator::Td { .. }
            | Operator::TD { .. }
            | Operator::Tm { .. }
            | Operator::TStar
            | Operator::Tz { .. }
            | Operator::TL { .. }
            | Operator::Tc { .. }
            | Operator::Tw { .. }
            | Operator::Ts { .. }
                if inside_text => {}
            _ => return false,
        }
        true
    })
    .ok()?;
    if !complete || inside_text || !states.is_empty() || !hidden {
        return None;
    }
    image
}

#[derive(Clone, Copy)]
pub(super) enum StreamFilters {
    Raw,
    Flate,
    Ascii85Flate,
}

pub(super) fn stream_filters(filter: Option<&Object>) -> Option<StreamFilters> {
    match filter {
        None => Some(StreamFilters::Raw),
        Some(filter) if filter.as_name() == Some("FlateDecode") => Some(StreamFilters::Flate),
        Some(filter) if filter.as_array().is_some_and(|filters| {
            matches!(filters.as_slice(), [a, b] if a.as_name() == Some("ASCII85Decode") && b.as_name() == Some("FlateDecode"))
        }) => Some(StreamFilters::Ascii85Flate),
        _ => None,
    }
}

pub(super) fn decode_probe_stream(
    data: &[u8],
    filters: StreamFilters,
    output_limit: usize,
    byte_limit: usize,
) -> Option<Vec<u8>> {
    // Native's decoder rejects output that reaches its exclusive limit.
    let limit = output_limit
        .checked_add(1)?
        .min(usize::try_from(FlateDecoder::default().max_decompressed_bytes).ok()?);
    if output_limit == 0 || limit == 0 || data.len() > byte_limit {
        return None;
    }
    match filters {
        StreamFilters::Raw => (data.len() < limit).then(|| data.to_vec()),
        StreamFilters::Flate => FlateDecoder::with_limit(limit as u64).decode(data, limit).ok(),
        StreamFilters::Ascii85Flate => {
            // A one-byte zero group can emit four bytes. Bound the wrapper before
            // allocation because ASCII85's decoder ignores the trait's byte limit.
            if data.len().checked_mul(8)? > byte_limit {
                return None;
            }
            let unwrapped = Ascii85Decoder.decode(data, byte_limit).ok()?;
            FlateDecoder::with_limit(limit as u64).decode(&unwrapped, limit).ok()
        }
    }
}

struct ImageSource {
    width: u32,
    height: u32,
    channels: usize,
    data: bytes::Bytes,
    filters: StreamFilters,
    predictor: Option<DecodeParams>,
}

impl ImageSource {
    fn decoded_size(&self) -> Option<usize> {
        (self.width as usize)
            .checked_mul(self.height as usize)?
            .checked_mul(self.channels)
    }

    fn inflated_size(&self) -> Option<usize> {
        if let Some(params) = &self.predictor {
            params.checked_bytes_per_row().ok()?.checked_mul(self.height as usize)
        } else {
            self.decoded_size()
        }
    }

    fn decode_samples(&self, byte_limit: usize) -> Option<Vec<u8>> {
        let expected = self.inflated_size()?;
        let inflated = decode_probe_stream(&self.data, self.filters, expected, byte_limit)?;
        if inflated.len() != expected {
            return None;
        }
        let samples = if let Some(params) = &self.predictor {
            decode_predictor(&inflated, params).ok()?
        } else {
            inflated
        };
        (samples.len() == self.decoded_size()?).then_some(samples)
    }
}

fn image_source(doc: &PdfDocument, page: usize, name: &str) -> Option<ImageSource> {
    let resources = doc.get_page_resources(page).ok()?;
    let resources = resources.as_dict()?;
    let xobjects = doc.resolve_object(resources.get("XObject")?).ok()?;
    let Object::Stream { dict, data } = doc.resolve_object(xobjects.as_dict()?.get(name)?).ok()? else {
        return None;
    };
    let channels = match dict.get("ColorSpace")?.as_name()? {
        "DeviceRGB" => 3,
        "DeviceGray" => 1,
        _ => return None,
    };
    if dict.get("Subtype")?.as_name()? != "Image"
        || dict.get("BitsPerComponent")?.as_integer()? != 8
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
        || resources.contains_key("ColorSpace")
    {
        return None;
    }
    let width = u32::try_from(dict.get("Width")?.as_integer()?).ok()?;
    let height = u32::try_from(dict.get("Height")?.as_integer()?).ok()?;
    let filters = stream_filters(dict.get("Filter"))?;
    let predictor = match dict.get("DecodeParms") {
        None => None,
        Some(params) if matches!(filters, StreamFilters::Flate) => {
            Some(image_predictor(params.as_dict()?, width, channels)?)
        }
        Some(_) => return None,
    };
    Some(ImageSource {
        width,
        height,
        channels,
        data,
        filters,
        predictor,
    })
}

fn image_predictor(
    params: &std::collections::HashMap<String, Object>,
    width: u32,
    channels: usize,
) -> Option<DecodeParams> {
    let predictor = params.get("Predictor")?.as_integer()?;
    if !(10..=15).contains(&predictor)
        || params.get("Columns")?.as_integer()? != i64::from(width)
        || params.get("Colors")?.as_integer()? != channels as i64
        || params.get("BitsPerComponent")?.as_integer()? != 8
    {
        return None;
    }
    Some(DecodeParams {
        predictor,
        columns: width as usize,
        colors: channels,
        bits_per_component: 8,
    })
}

fn finite_box(bounds: Rect) -> bool {
    [bounds.x, bounds.y, bounds.width, bounds.height]
        .into_iter()
        .all(f32::is_finite)
        && bounds.width > 0.0
        && bounds.height > 0.0
}

fn covers_frame(image: Rect, frame: Rect) -> bool {
    let (x, y, w, h) = (
        f64::from(image.x),
        f64::from(image.y),
        f64::from(image.width),
        f64::from(image.height),
    );
    let (fx, fy, fw, fh) = (
        f64::from(frame.x),
        f64::from(frame.y),
        f64::from(frame.width),
        f64::from(frame.height),
    );
    finite_box(image)
        && x >= fx
        && y >= fy
        && x + w <= fx + fw
        && y + h <= fy + fh
        && w * h >= fw * fh * crate::core::pipeline::FULL_PAGE_IMAGE_AREA_RATIO
}

fn simple_font_resources(doc: &PdfDocument, page: usize) -> Option<std::collections::HashMap<String, Object>> {
    // The optional geometry read must not inflate embedded fonts or CMaps through
    // Native's ordinary text decoder. Simple unembedded Type 1 fonts have no such streams.
    let resources = doc.get_page_resources(page).ok()?;
    let Object::Dictionary(fonts) = doc.resolve_object(resources.as_dict()?.get("Font")?).ok()? else {
        return None;
    };
    let simple = !fonts.is_empty()
        && fonts.len() <= MAX_OPERATORS
        && fonts.values().all(|font| {
            doc.resolve_object(font)
                .ok()
                .as_ref()
                .and_then(Object::as_dict)
                .is_some_and(|dict| {
                    dict.get("Subtype").and_then(Object::as_name) == Some("Type1")
                        && dict.get("BaseFont").and_then(Object::as_name).is_some()
                        && !dict.contains_key("FontDescriptor")
                        && !dict.contains_key("ToUnicode")
                        && dict.get("Encoding").is_none_or(|encoding| {
                            matches!(
                                encoding.as_name(),
                                Some("WinAnsiEncoding" | "MacRomanEncoding" | "StandardEncoding")
                            )
                        })
                })
        });
    simple.then_some(fonts)
}

fn native_boxes(doc: &PdfDocument, page: usize, frame: Rect) -> Option<Vec<Rect>> {
    let spans = doc
        .extract_spans_with_reading_order(page, ReadingOrder::TopToBottom)
        .ok()?;
    if spans.len() > MAX_OPERATORS {
        return None;
    }
    let mut boxes = Vec::new();
    for span in spans.into_iter().filter(|span| !span.text.trim().is_empty()) {
        let b = span.bbox;
        if span.rotation_degrees != 0.0 || !finite_box(b) || !frame.intersects(&b) {
            return None;
        }
        boxes.push(b);
    }
    (!boxes.is_empty()).then_some(boxes)
}

fn preview_size(bounds: Rect, source: &ImageSource, limit: usize) -> Option<(u32, u32)> {
    let width = (bounds.width.ceil() as u32).min(source.width);
    let height = (bounds.height.ceil() as u32).min(source.height);
    if width == 0 || height == 0 {
        return None;
    }
    let decoded_bytes = source.decoded_size()?;
    let preview_pixels = (width as usize).checked_mul(height as usize)?;
    if preview_pixels > MAX_PREVIEW_PIXELS {
        return None;
    }
    // Triangle resize holds a vertically resampled RGBA32F buffer. Band scoring
    // uses a grayscale raster, masks including a trailing-text crop, a
    // pixel-index stack and interval buffers.
    let resize_bytes = (source.width as usize)
        .checked_mul(height as usize)?
        .checked_mul(size_of::<[f32; 4]>())?;
    let preview_bytes = preview_pixels.checked_mul(4 + 2 * size_of::<usize>() + source.channels)?;
    let intervals = (width as usize)
        .checked_add(height as usize)?
        .checked_mul(4 * size_of::<Band>())?;
    let box_bytes = MAX_OPERATORS.checked_mul(size_of::<Rect>())?;
    let preview_peak = decoded_bytes
        .checked_add(resize_bytes)?
        .checked_add(preview_bytes)?
        .checked_add(intervals)?
        .checked_add(box_bytes)?;
    let wrapped_bytes = match source.filters {
        StreamFilters::Ascii85Flate => source.data.len().checked_mul(8)?,
        _ => source.data.len(),
    };
    let filter_peak = wrapped_bytes
        .checked_add(source.inflated_size()?.checked_add(1)?.checked_mul(2)?)?
        .checked_add(decoded_bytes)?;
    (preview_peak.max(filter_peak) <= limit).then_some((width, height))
}

fn gray_preview(samples: &[u8], source: &ImageSource, (width, height): (u32, u32)) -> Option<GrayImage> {
    let filter = imageops::FilterType::Triangle;
    match source.channels {
        3 => {
            let image = ImageBuffer::<Rgb<u8>, _>::from_raw(source.width, source.height, samples)?;
            Some(imageops::grayscale(&imageops::resize(&image, width, height, filter)))
        }
        1 => {
            let image = ImageBuffer::<Luma<u8>, _>::from_raw(source.width, source.height, samples)?;
            Some(imageops::resize(&image, width, height, filter))
        }
        _ => None,
    }
}

struct Band {
    rows: Range<usize>,
    columns: Range<usize>,
    runs: usize,
}

fn has_uncovered_text(preview: &GrayImage, image: Rect, boxes: &[Rect]) -> bool {
    let (width, height) = (preview.width() as usize, preview.height() as usize);
    let mut histogram = [0; 256];
    for &gray in preview.as_raw() {
        histogram[usize::from(gray)] += 1;
    }
    let Some(threshold) = light_page_foreground_mode(&histogram) else {
        return false;
    };
    let mask = preview
        .as_raw()
        .iter()
        .map(|&gray| gray <= threshold)
        .collect::<Vec<_>>();
    let bands = bands(&mask, width);
    let Some(coverage) = band_coverage(&bands, boxes, image, width, height) else {
        return false;
    };
    let Some((min_height, max_height, min_width, min_runs)) = bands
        .iter()
        .zip(&coverage)
        .filter(|(_, state)| matches!(state, BandCoverage::Covered { anchored: true, .. }))
        .map(|(band, _)| (band.rows.len(), band.rows.len(), band.columns.len(), band.runs))
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3)))
    else {
        return false;
    };
    bands.iter().zip(coverage).any(|(band, state)| match state {
        BandCoverage::Uncovered => {
            (min_height..=max_height).contains(&band.rows.len())
                && band.columns.len() >= min_width
                && band.runs >= min_runs
                && text_like_band(&mask, width, &band.rows)
        }
        BandCoverage::Covered {
            native_right,
            anchored: true,
        } => has_trailing_text(&mask, width, band, native_right),
        BandCoverage::Covered { anchored: false, .. } => false,
    })
}

#[derive(Clone, Copy)]
enum BandCoverage {
    Uncovered,
    Covered { native_right: usize, anchored: bool },
}

fn band_coverage(
    bands: &[Band],
    boxes: &[Rect],
    image: Rect,
    width: usize,
    height: usize,
) -> Option<Vec<BandCoverage>> {
    let mut coverage = vec![BandCoverage::Uncovered; bands.len()];
    for bounds in boxes {
        let (x, y) = projected_box(*bounds, image, width, height);
        let first = bands.partition_point(|band| band.rows.end <= y.start);
        let Some(band) = bands.get(first).filter(|band| intersects(&band.rows, &y)) else {
            continue;
        };
        if bands.get(first + 1).is_some_and(|next| intersects(&next.rows, &y)) {
            return None;
        }
        let (native_right, anchored) = match coverage[first] {
            BandCoverage::Uncovered => (x.end, false),
            BandCoverage::Covered { native_right, anchored } => (native_right.max(x.end), anchored),
        };
        coverage[first] = BandCoverage::Covered {
            native_right,
            anchored: anchored || intersects(&band.columns, &x),
        };
    }
    Some(coverage)
}

fn has_trailing_text(mask: &[bool], width: usize, band: &Band, native_right: usize) -> bool {
    let line_height = band.rows.len();
    // Allow a line-height gap for ordinary differences between font boxes and raster ink.
    let start = native_right.saturating_add(line_height);
    let min_width = MIN_TRAILING_WIDTH_IN_LINE_HEIGHTS * line_height;
    if band.columns.end.saturating_sub(start) < min_width {
        return false;
    }

    let mut first_ink = None;
    let mut previous_ink = false;
    let mut runs = 0;
    for column in start..band.columns.end {
        let ink = band.rows.clone().any(|row| mask[row * width + column]);
        if ink && !previous_ink {
            first_ink.get_or_insert(column);
            runs += 1;
        }
        previous_ink = ink;
    }
    let Some(first) = first_ink.filter(|&first| band.columns.end - first >= min_width) else {
        return false;
    };
    if runs < MIN_TRAILING_INK_RUNS {
        return false;
    }

    let columns = first..band.columns.end;
    let mut tail = Vec::with_capacity(columns.len() * line_height);
    for row in mask[band.rows.start * width..band.rows.end * width].chunks_exact(width) {
        tail.extend_from_slice(&row[columns.clone()]);
    }
    let ink = tail.iter().filter(|&&pixel| pixel).count();
    lower_mode_mask_has_text_structure(&tail, columns.len(), line_height, ink)
}

fn bands(mask: &[bool], width: usize) -> Vec<Band> {
    let rows = true_runs(mask.chunks_exact(width).map(|row| row.contains(&true)));
    let mut columns = vec![false; width];
    rows.into_iter()
        .filter_map(|rows| {
            columns.fill(false);
            for row in mask[rows.start * width..rows.end * width].chunks_exact(width) {
                for (active, &ink) in columns.iter_mut().zip(row) {
                    *active |= ink;
                }
            }
            let x_runs = true_runs(columns.iter().copied());
            let left = x_runs.first()?.start;
            let right = x_runs.last()?.end;
            Some(Band {
                rows,
                columns: left..right,
                runs: x_runs.len(),
            })
        })
        .collect()
}

fn true_runs(values: impl Iterator<Item = bool>) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut start = None;
    for (index, value) in values.chain(std::iter::once(false)).enumerate() {
        if value {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            runs.push(begin..index);
        }
    }
    runs
}

fn projected_box(bounds: Rect, image: Rect, width: usize, height: usize) -> (Range<usize>, Range<usize>) {
    let x = |value| (value - image.x) / image.width * width as f32;
    let y = |value| (image.y + image.height - value) / image.height * height as f32;
    (
        x(bounds.x).floor() as usize..x(bounds.x + bounds.width).ceil() as usize,
        y(bounds.y + bounds.height).floor() as usize..y(bounds.y).ceil() as usize,
    )
}

fn intersects(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn text_like_band(mask: &[bool], width: usize, rows: &Range<usize>) -> bool {
    let mask = &mask[rows.start * width..rows.end * width];
    let ink = mask.iter().filter(|&&pixel| pixel).count();
    lower_mode_mask_has_text_structure(mask, width, rows.len(), ink)
}

pub(super) fn adds_text_preserving_native(native: &str, candidate: &str) -> bool {
    let native_words = bounded_words(native);
    let candidate = bounded_words(candidate);
    let (Some(native_words), Some(candidate)) = (native_words, candidate) else {
        return false;
    };
    let native_set: std::collections::HashSet<_> = native_words.iter().copied().collect();
    if native_words.is_empty()
        || !candidate
            .iter()
            .any(|word| !native_set.contains(word) && word.chars().any(char::is_alphanumeric))
    {
        return false;
    }
    let mut ordered = native_words.iter().peekable();
    for word in &candidate {
        if ordered.peek() == Some(&word) {
            ordered.next();
        }
    }
    ordered.peek().is_none()
        && native.lines().all(|line| {
            let line: Vec<_> = line.split_whitespace().collect();
            line.is_empty() || count_sequence(&candidate, &line) <= count_sequence(&native_words, &line)
        })
}

fn bounded_words(text: &str) -> Option<Vec<&str>> {
    let words: Vec<_> = text.split_whitespace().take(MAX_OPERATORS + 1).collect();
    (words.len() <= MAX_OPERATORS).then_some(words)
}

fn count_sequence(words: &[&str], sequence: &[&str]) -> usize {
    words
        .windows(sequence.len())
        .filter(|window| *window == sequence)
        .count()
}

#[cfg(test)]
mod tests;
