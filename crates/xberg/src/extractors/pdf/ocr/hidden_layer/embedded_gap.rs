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
    drop(bytes);
    drop(data);
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
        let (samples, retained_source_bytes) = panel_samples(source, image.data.len(), limits.max_content_size)?;
        let mut remaining = limits.clone();
        remaining.max_content_size = remaining
            .max_content_size
            .checked_sub(retained_source_bytes)?
            .checked_sub(samples.capacity())?;
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

fn panel_samples(source: &super::ImageSource, png_bytes: usize, byte_limit: usize) -> Option<(Vec<u8>, usize)> {
    let ImageEncoding::Stream { predictor, .. } = &source.encoding else {
        return None;
    };
    let Some(params) = predictor else {
        return Some((source.decode_samples(byte_limit)?, 0));
    };

    let compressed_bytes = source.data.len();
    let tagged_bytes = source.inflated_size(Some(params))?;
    let decoded_bytes = source.decoded_size()?;
    if compressed_bytes
        .checked_add(png_bytes)?
        .checked_add(tagged_bytes)?
        .checked_add(1)?
        .checked_add(decoded_bytes)?
        > byte_limit
    {
        return None;
    }
    let tagged = super::decode_exact_flate(&source.data, tagged_bytes)?;
    let samples = super::decode_predictor(&tagged, params).ok()?;
    (samples.len() == decoded_bytes).then_some((samples, compressed_bytes))
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

#[cfg(test)]
mod tests {
    use super::super::{ImageSource, StreamFilters};
    use super::*;
    use std::io::Write;
    use xberg_native_pdf::decoders::DecodeParams;

    #[test]
    fn predictor_requires_complete_tagged_rows_and_live_decode_budget() {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&[2, 11, 13, 17]).unwrap();
        let mut compressed = encoder.finish().unwrap();
        let png_bytes = 8;
        let limit = compressed.len() + png_bytes + 4 + 1 + 3;
        let mut source = ImageSource {
            width: 1,
            height: 1,
            channels: 3,
            data: bytes::Bytes::copy_from_slice(&compressed),
            encoding: ImageEncoding::Stream {
                filters: StreamFilters::Flate,
                predictor: Some(DecodeParams {
                    predictor: 12,
                    columns: 1,
                    colors: 3,
                    bits_per_component: 8,
                }),
            },
        };
        assert_eq!(
            panel_samples(&source, png_bytes, limit),
            Some((vec![11, 13, 17], compressed.len()))
        );
        assert!(panel_samples(&source, png_bytes, limit - 1).is_none());

        compressed.push(0);
        source.data = bytes::Bytes::from(compressed);
        assert!(panel_samples(&source, png_bytes, limit + 1).is_none());
    }
}
