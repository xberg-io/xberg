use super::{
    BandCoverage, ImageEncoding, MAX_CONTENT_BYTES, MAX_OPERATORS, StreamFilters, band_coverage, bands,
    decode_probe_stream, gray_preview, image_source, native_boxes, page_frame_and_contents, positive_axis_matrix,
    preview_byte_limit, preview_page_limit, preview_size, simple_font_resources, stock_backend, stream_filters,
    text_like_band,
};
use crate::core::config::ExtractionConfig;
use crate::ocr::processor::light_page_foreground_mode;
use image::GrayImage;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::content::{Operator, parser::visit_complete_content_stream};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

const MAX_SOURCE_PIXELS: usize = 2 * 1024 * 1024;
const MAX_DOCUMENT_PIXELS: usize = 8 * 1024 * 1024;
const CONTENT_PASSES: usize = 3;
const MIN_PROSE_ROWS: usize = 3;
const MIN_INK_RUNS: usize = 16;
const MIN_ROW_WIDTH_IN_HEIGHTS: usize = 12;

pub(in crate::extractors::pdf) struct VisibleRasterProof {
    byte_limit: usize,
    work_left: usize,
    pixels_left: usize,
}

impl VisibleRasterProof {
    pub(in crate::extractors::pdf) fn new(config: &ExtractionConfig) -> Option<Self> {
        stock_backend(config)?;
        Some(Self {
            // Reserve half the caller limit for Native decoder stream copies.
            byte_limit: preview_byte_limit(config) / 2,
            work_left: preview_page_limit(config),
            pixels_left: MAX_DOCUMENT_PIXELS,
        })
    }

    pub(in crate::extractors::pdf) fn has_prose(&mut self, doc: &PdfDocument, index: usize) -> bool {
        self.inspect(doc, index).unwrap_or(false)
    }

    fn inspect(&mut self, doc: &PdfDocument, index: usize) -> Option<bool> {
        self.work_left = self.work_left.checked_sub(1)?;
        let (frame, content) = page_frame_and_contents(doc, index)?;
        let (name, paint, audited) = audited_stub_image(doc, index, &content, self.byte_limit, &mut self.work_left)?;
        if paint != frame {
            return None;
        }
        let source = image_source(doc, index, &name)?;
        if source.channels != 1
            || !matches!(
                source.encoding,
                ImageEncoding::Stream {
                    filters: StreamFilters::Flate,
                    predictor: None
                }
            )
        {
            return None;
        }
        let pixels = source.decoded_size()?;
        if pixels > MAX_SOURCE_PIXELS {
            return None;
        }

        let size = preview_size(frame, &source, self.byte_limit)?;
        self.pixels_left = self.pixels_left.checked_sub(pixels)?;
        if doc.get_page_content_data(index).ok()? != audited {
            return None;
        }
        drop(audited);
        let handles = doc.page_image_handles(index).ok()?;
        let [image] = handles.as_slice() else { return None };
        if image.is_inline
            || image.rotation_degrees != 0.0
            || image.bbox != frame
            || (image.width, image.height) != (source.width, source.height)
        {
            return None;
        }
        let boxes = native_boxes(doc, index, frame)?;
        let samples = source.decode_samples(self.byte_limit)?;
        let preview = gray_preview(&samples, &source, size)?;
        Some(disjoint_prose_bands(&preview, frame, &boxes))
    }
}

fn audited_stub_image(
    doc: &PdfDocument,
    index: usize,
    content: &Object,
    byte_limit: usize,
    work_left: &mut usize,
) -> Option<(String, Rect, Vec<u8>)> {
    let Object::Stream { dict, data } = content else {
        return None;
    };
    let filters = stream_filters(dict.get("Filter"))?;
    if dict.contains_key("DecodeParms") || matches!(filters, StreamFilters::Ascii85Flate) || *work_left < CONTENT_PASSES
    {
        return None;
    }
    let bytes = decode_probe_stream(data, filters, MAX_CONTENT_BYTES.min(byte_limit / 4), byte_limit)?;
    let fonts = simple_font_resources(doc, index)?;
    let (name, bounds) = visible_stub_image(&bytes, |font| fonts.contains_key(font), work_left)?;
    Some((name, bounds, bytes))
}

#[derive(Clone, Copy)]
enum Paint {
    Start,
    Saved,
    Transformed,
    Image,
    BetweenLines,
    Text,
    Font,
    Positioned,
    Shown,
}

fn visible_stub_image(
    bytes: &[u8],
    font_is_known: impl Fn(&str) -> bool,
    work_left: &mut usize,
) -> Option<(String, Rect)> {
    let mut state = Paint::Start;
    let mut image = None;
    let mut bounds = None;
    let mut saw_line = false;
    let mut visited = 0;
    // Reserve the image-only and native glyph passes over these audited operators.
    let limit = (*work_left / CONTENT_PASSES).min(MAX_OPERATORS);
    let complete = visit_complete_content_stream(bytes, limit, |operator| {
        visited += 1;
        state = match (state, operator) {
            (Paint::Start, Operator::SaveState) => Paint::Saved,
            (Paint::Saved, Operator::Cm { a, b, c, d, e, f }) if positive_axis_matrix(a, b, c, d, e, f) => {
                bounds = Some(Rect::new(e, f, a, d));
                Paint::Transformed
            }
            (Paint::Transformed, Operator::Do { name }) => {
                image = Some(name);
                Paint::Image
            }
            (Paint::Image, Operator::RestoreState) => Paint::BetweenLines,
            (Paint::BetweenLines, Operator::BeginText) => Paint::Text,
            (Paint::Text, Operator::Tf { font, size }) if size.is_finite() && size > 0.0 && font_is_known(&font) => {
                Paint::Font
            }
            (Paint::Font, Operator::Tm { a, b, c, d, e, f }) if positive_axis_matrix(a, b, c, d, e, f) => {
                Paint::Positioned
            }
            (Paint::Positioned, Operator::Tj { text }) if !text.is_empty() => Paint::Shown,
            (Paint::Shown, Operator::EndText) => {
                saw_line = true;
                Paint::BetweenLines
            }
            _ => return false,
        };
        true
    });
    *work_left = work_left.saturating_sub(visited * CONTENT_PASSES);
    if complete.ok()? && saw_line && matches!(state, Paint::BetweenLines) {
        image.zip(bounds)
    } else {
        None
    }
}

fn disjoint_prose_bands(preview: &GrayImage, frame: Rect, boxes: &[Rect]) -> bool {
    let (width, height) = (preview.width() as usize, preview.height() as usize);
    let mut histogram = [0; 256];
    for &sample in preview.as_raw() {
        histogram[usize::from(sample)] += 1;
    }
    let Some(threshold) = light_page_foreground_mode(&histogram) else {
        return false;
    };
    let mask = preview
        .as_raw()
        .iter()
        .map(|&sample| sample <= threshold)
        .collect::<Vec<_>>();
    let rows = bands(&mask, width);
    let Some(coverage) = band_coverage(&rows, boxes, frame, width, height) else {
        return false;
    };
    if coverage
        .iter()
        .any(|coverage| !matches!(coverage, BandCoverage::Uncovered))
    {
        return false;
    }

    let mut body_rows = rows.iter().filter(|band| {
        band.runs >= MIN_INK_RUNS
            && band.columns.len() >= MIN_ROW_WIDTH_IN_HEIGHTS * band.rows.len()
            && text_like_band(&mask, width, &band.rows)
    });
    let Some(first) = body_rows.next() else { return false };
    body_rows
        .filter(|band| {
            let shorter = band.rows.len().min(first.rows.len());
            let taller = band.rows.len().max(first.rows.len());
            taller <= 2 * shorter && band.columns.start.abs_diff(first.columns.start) <= taller
        })
        .take(MIN_PROSE_ROWS - 1)
        .count()
        == MIN_PROSE_ROWS - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROSE: &[u8] = include_bytes!("../../../../../tests/fixtures/ocr/raster_prose_stubs_2313.pdf");
    const CAPTION: &[u8] = include_bytes!("../../../../../tests/fixtures/ocr/raster_caption_stubs_2313.pdf");

    fn proof(pixels_left: usize, byte_limit: usize) -> VisibleRasterProof {
        VisibleRasterProof {
            byte_limit,
            work_left: usize::MAX,
            pixels_left,
        }
    }

    #[test]
    fn visible_raster_source_proof_requires_prose_and_obeys_budgets() {
        let prose = PdfDocument::from_bytes(PROSE.to_vec()).unwrap();
        let caption = PdfDocument::from_bytes(CAPTION.to_vec()).unwrap();
        let pixels = 1224 * 1584;
        let mut exact = proof(pixels, usize::MAX);
        assert!(exact.has_prose(&prose, 0));
        assert_eq!(exact.pixels_left, 0);
        assert!(!exact.has_prose(&prose, 0));
        assert!(!proof(pixels - 1, usize::MAX).has_prose(&prose, 0));
        assert!(!proof(pixels, 1024).has_prose(&prose, 0));
        let mut low_work = proof(pixels, usize::MAX);
        low_work.work_left = 12;
        assert!(!low_work.has_prose(&prose, 0));
        assert_eq!(low_work.pixels_left, pixels);
        assert!(!proof(pixels, usize::MAX).has_prose(&caption, 0));
    }

    #[test]
    fn visible_stubs_overlapping_raster_ink_do_not_supply_a_disjoint_prose_proof() {
        let mut pdf = lopdf::Document::load_mem(PROSE).unwrap();
        let page = pdf.get_pages()[&1];
        let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
        let content = String::from_utf8(pdf.get_page_content(page)).unwrap();
        let overlapping = content.replace("72 740 Tm", "72 640 Tm");
        assert_ne!(content, overlapping);
        pdf.get_object_mut(stream)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .set_plain_content(overlapping.into_bytes());
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let overlap = PdfDocument::from_bytes(bytes).unwrap();
        assert!(!proof(MAX_SOURCE_PIXELS, usize::MAX).has_prose(&overlap, 0));
    }

    #[test]
    fn visible_stub_audit_refuses_unproven_paint_order_or_modes() {
        let parse = |bytes: &[u8]| {
            let mut work = usize::MAX;
            visible_stub_image(bytes, |font| font == "F1", &mut work).map(|(name, _)| name)
        };
        let literal = "BT /F1 12 Tf 1 0 0 1 1 1 Tm (Visible) Tj ET";
        let image = "q 612 0 0 792 0 0 cm /Panel Do Q";
        assert_eq!(parse(format!("{image} {literal}").as_bytes()).as_deref(), Some("Panel"));
        for input in [
            format!("{literal} {image}"),
            format!("{image} {image} {literal}"),
            format!("{image} {}", literal.replace("(Visible)", "3 Tr (Visible)")),
            format!("{image} {}", literal.replace("/F1", "/Custom")),
        ] {
            assert_eq!(parse(input.as_bytes()), None, "{input}");
        }
    }
}
