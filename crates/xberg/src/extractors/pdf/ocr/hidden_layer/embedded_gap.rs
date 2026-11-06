use super::{
    ImageEncoding, MAX_CONTENT_BYTES, MAX_OPERATORS, Object, Operator, PdfDocument, decode_probe_stream, image_source,
    page_frame_and_contents, simple_font_resources, stream_filters, visit_complete_content_stream,
};
use crate::extraction::image_ocr::pdf_gap::Placement;
use crate::extractors::security::SecurityLimits;
use crate::types::ExtractedImage;

const MAX_SOURCE_PIXELS: u64 = 1024 * 1024;
const POINTS_PER_INCH: f64 = 72.0;

pub(crate) fn embedded_gap_placement(
    doc: &PdfDocument,
    image: &ExtractedImage,
    limits: &SecurityLimits,
) -> Option<Placement> {
    if doc.page_count().ok()? != 1 || image.page_number != Some(1) || image.image_index != 0 || image.is_mask {
        return None;
    }
    if doc.get_page(0).ok()?.as_dict()?.contains_key("CropBox") {
        return None;
    }
    let (page, content) = page_frame_and_contents(doc, 0)?;
    let Object::Stream { dict, data } = content else {
        return None;
    };
    if dict.contains_key("DecodeParms") {
        return None;
    }
    let byte_limit = limits.max_content_size;
    let bytes = decode_probe_stream(
        &data,
        stream_filters(dict.get("Filter"))?,
        MAX_CONTENT_BYTES.min(byte_limit / 4),
        byte_limit,
    )?;
    let fonts = simple_font_resources(doc, 0)?;
    let (name, bbox) = last_opaque_panel(&bytes, |font| fonts.contains_key(font))?;
    if doc.get_page_content_data(0).ok()? != bytes {
        return None;
    }
    let [left, bottom, right, top] = bbox;
    if !is_partial_page_panel(page, bbox) {
        return None;
    }
    let actual = image.bounding_box.as_ref()?;
    if [actual.x0, actual.y0, actual.x1, actual.y1] != bbox {
        return None;
    }
    let source = image_source(doc, 0, &name)?;
    let dimensions = (source.width, source.height);
    let dpi = f64::from(source.width) * POINTS_PER_INCH / (right - left);
    if source.channels != 3
        || !matches!(source.encoding, ImageEncoding::Stream { predictor: None, .. })
        || image.width.zip(image.height) != Some(dimensions)
        || f64::from(source.height) * POINTS_PER_INCH / (top - bottom) != dpi
        || !(0.0 < dpi && dpi <= POINTS_PER_INCH)
        || !same_rgb_pixels(&source, image, limits)
    {
        return None;
    }
    Placement::new(image, dimensions, dpi, limits)
}

fn is_partial_page_panel(page: super::Rect, [left, bottom, right, top]: [f64; 4]) -> bool {
    let page_x = f64::from(page.x);
    let page_y = f64::from(page.y);
    let page_w = f64::from(page.width);
    let page_h = f64::from(page.height);
    left >= page_x
        && bottom >= page_y
        && right <= page_x + page_w
        && top <= page_y + page_h
        && (right - left) * (top - bottom) < page_w * page_h * crate::core::pipeline::FULL_PAGE_IMAGE_AREA_RATIO
}

fn same_rgb_pixels(source: &super::ImageSource, image: &ExtractedImage, limits: &SecurityLimits) -> bool {
    let verify = || {
        if image.format != "png" || source.decoded_size()? as u64 / 3 > MAX_SOURCE_PIXELS {
            return None;
        }
        let samples = source.decode_samples(limits.max_content_size)?;
        let mut remaining = limits.clone();
        remaining.max_content_size = remaining.max_content_size.checked_sub(samples.capacity())?;
        let image::DynamicImage::ImageRgb8(raster) =
            crate::extraction::image_decode::decode_standard_image_with_security_limits(&image.data, &remaining)
                .ok()?
        else {
            return None;
        };

        Some(raster.dimensions() == (source.width, source.height) && raster.as_raw() == &samples)
    };
    verify() == Some(true)
}

enum Paint {
    Before,
    Text { font: bool, shown: bool },
    Saved,
    Transform([f64; 4]),
    Panel(String, [f64; 4]),
    Closed(String, [f64; 4]),
}

fn last_opaque_panel(bytes: &[u8], known_font: impl Fn(&str) -> bool) -> Option<(String, [f64; 4])> {
    let mut state = Paint::Before;
    let mut saw_text = false;
    let complete = visit_complete_content_stream(bytes, MAX_OPERATORS, |op| {
        state = match (std::mem::replace(&mut state, Paint::Before), op) {
            (Paint::Before, Operator::BeginText) => Paint::Text {
                font: false,
                shown: false,
            },
            (
                Paint::Text {
                    font: false,
                    shown: false,
                },
                Operator::Tf { font, size },
            ) if known_font(&font) && size.is_finite() && size > 0.0 => Paint::Text {
                font: true,
                shown: false,
            },
            (
                state @ Paint::Text {
                    font: true,
                    shown: false,
                },
                Operator::Tm { a, b, c, d, e, f },
            ) if super::positive_axis_matrix(a, b, c, d, e, f) => state,
            (
                Paint::Text {
                    font: true,
                    shown: false,
                },
                Operator::Tj { text },
            ) if text.iter().any(u8::is_ascii_alphanumeric) && text.iter().all(u8::is_ascii) => Paint::Text {
                font: true,
                shown: true,
            },
            (Paint::Text { shown: true, .. }, Operator::EndText) => {
                saw_text = true;
                Paint::Before
            }
            (Paint::Before, Operator::SaveState) if saw_text => Paint::Saved,
            (Paint::Saved, Operator::Cm { a, b, c, d, e, f }) if super::positive_axis_matrix(a, b, c, d, e, f) => {
                Paint::Transform([
                    f64::from(e),
                    f64::from(f),
                    f64::from(e) + f64::from(a),
                    f64::from(f) + f64::from(d),
                ])
            }
            (Paint::Transform(bounds), Operator::Do { name }) => Paint::Panel(name, bounds),
            (Paint::Panel(name, bounds), Operator::RestoreState) => Paint::Closed(name, bounds),
            _ => return false,
        };
        true
    })
    .ok()?;
    match state {
        Paint::Closed(name, bounds) if complete => Some((name, bounds)),
        _ => None,
    }
}
