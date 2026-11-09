use super::{
    BandCoverage, ImageEncoding, ImageSource, MAX_CONTENT_BYTES, MAX_OPERATORS, StreamFilters, band_coverage, bands,
    decode_probe_stream, gray_preview, image_source, native_boxes, page_frame_and_contents, positive_axis_matrix,
    preview_byte_limit, preview_page_limit, preview_size, simple_font_resources, stock_backend, stream_filters,
    text_like_band,
};
use crate::core::config::ExtractionConfig;
use crate::ocr::processor::light_page_foreground_mode;
use image::GrayImage;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::color::IccHeader;
use xberg_native_pdf::content::{Operator, parser::visit_complete_content_stream};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

const MAX_SOURCE_PIXELS: usize = 2 * 1024 * 1024;
const MAX_GRAY_PROFILE_BYTES: usize = 16 * 1024;
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
        let (source, icc) = match image_source(doc, index, &name) {
            Some(source) => (source, false),
            None => (icc_gray_image_source(doc, index, &name, self.byte_limit)?, true),
        };
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
        let samples = if icc {
            super::decode_exact_flate(&source.data, pixels)?
        } else {
            source.decode_samples(self.byte_limit)?
        };
        let preview = gray_preview(&samples, &source, size)?;
        Some(disjoint_prose_bands(&preview, frame, &boxes))
    }
}

fn icc_gray_image_source(doc: &PdfDocument, page: usize, name: &str, byte_limit: usize) -> Option<ImageSource> {
    let resources = doc.get_page_resources(page).ok()?;
    let resources = resources.as_dict()?;
    if resources.contains_key("ColorSpace") {
        return None;
    }
    let images = doc.resolve_object(resources.get("XObject")?).ok()?;
    let image = images.as_dict()?.get(name)?.as_reference()?;
    let Object::Stream { dict, data } = doc.load_object(image).ok()? else {
        return None;
    };
    if dict.get("Type")?.as_name()? != "XObject"
        || dict.get("Subtype")?.as_name()? != "Image"
        || dict.get("Filter")?.as_name()? != "FlateDecode"
        || dict.get("BitsPerComponent")?.as_integer()? != 8
        || dict.len() != 8
        || usize::try_from(dict.get("Length")?.as_integer()?).ok()? != data.len()
    {
        return None;
    }

    let space = doc.load_object(dict.get("ColorSpace")?.as_reference()?).ok()?;
    let [kind, profile] = space.as_array()?.as_slice() else {
        return None;
    };
    if kind.as_name()? != "ICCBased" {
        return None;
    }
    certified_gray_profile(doc, profile, byte_limit.checked_sub(data.len())?)?;
    Some(ImageSource {
        width: u32::try_from(dict.get("Width")?.as_integer()?).ok()?,
        height: u32::try_from(dict.get("Height")?.as_integer()?).ok()?,
        channels: 1,
        data,
        encoding: ImageEncoding::Stream {
            filters: StreamFilters::Flate,
            predictor: None,
        },
    })
}

fn certified_gray_profile(doc: &PdfDocument, profile: &Object, byte_limit: usize) -> Option<()> {
    let Object::Stream { dict, data } = doc.load_object(profile.as_reference()?).ok()? else {
        return None;
    };
    if dict.len() != 3
        || dict.get("N")?.as_integer()? != 1
        || dict.get("Alternate")?.as_name()? != "DeviceGray"
        || usize::try_from(dict.get("Length")?.as_integer()?).ok()? != data.len()
        || data.len() > byte_limit.min(MAX_GRAY_PROFILE_BYTES)
    {
        return None;
    }
    let header = IccHeader::parse(&data)?;
    let declared_size = icc_u32(&data[..4])?;
    // Native page rendering expands 8-bit ICC gray samples directly. Use only a bounded
    // single-channel display profile with the default range; export is color-managed. ~keep
    (declared_size == data.len()
        && header.color_space == *b"GRAY"
        && header.device_class == *b"mntr"
        && header.pcs == *b"XYZ "
        && gray_profile_tags(&data)?)
    .then_some(())
}

fn gray_profile_tags(profile: &[u8]) -> Option<bool> {
    const WHITE_POINT: u8 = 1 << 2;
    const TONE_CURVE: u8 = 1 << 4;
    let count = icc_u32(profile.get(128..132)?)?;
    if !(2..=5).contains(&count) {
        return None;
    }
    let table_end = 132 + count * 12;
    let mut previous_end = table_end;
    let mut seen = 0;
    for tag in profile.get(132..table_end)?.as_chunks::<12>().0 {
        let offset = icc_u32(&tag[4..8])?;
        let length = icc_u32(&tag[8..12])?;
        let end = offset.checked_add(length)?;
        let data = profile.get(offset..end)?;
        if offset < previous_end || !offset.is_multiple_of(4) || data.get(4..8)? != [0; 4] {
            return None;
        }
        let tag_bit = match (&tag[..4], data.get(..4)?) {
            (b"desc", b"desc" | b"mluc") => 1,
            (b"cprt", b"text" | b"mluc") => 1 << 1,
            (b"wtpt", b"XYZ ") if data.len() == 20 => WHITE_POINT,
            (b"bkpt", b"XYZ ") if data.len() == 20 => 1 << 3,
            (b"kTRC", _) if valid_gray_curve(data)? => TONE_CURVE,
            _ => return None,
        };
        if seen & tag_bit != 0 {
            return None;
        }
        seen |= tag_bit;
        previous_end = end;
    }
    let required_tags = WHITE_POINT | TONE_CURVE;
    Some(seen & required_tags == required_tags)
}

fn valid_gray_curve(data: &[u8]) -> Option<bool> {
    match data.get(..4)? {
        b"curv" => {
            let count = icc_u32(data.get(8..12)?)?;
            let entries = data.get(12..)?;
            if entries.len() != count.checked_mul(2)? {
                return None;
            }
            let entries = entries.as_chunks::<2>().0;
            if let [gamma] = entries {
                return Some(u16::from_be_bytes(*gamma) > 0);
            }
            Some(entries.windows(2).all(|pair| pair[0] <= pair[1]))
        }
        b"para" if data.len() == 16 && data[8..12] == [0; 4] => {
            Some(i32::from_be_bytes(data[12..16].try_into().ok()?) > 0)
        }
        _ => None,
    }
}

fn icc_u32(bytes: &[u8]) -> Option<usize> {
    usize::try_from(u32::from_be_bytes(bytes.try_into().ok()?)).ok()
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

    const ICC_PROSE: &[u8] = include_bytes!("../../../../../tests/fixtures/ocr/raster_prose_icc_stubs_2313.pdf");

    #[test]
    fn gray_profile_refuses_luts_duplicate_tags_and_malformed_tone_curves() {
        let pdf = lopdf::Document::load_mem(ICC_PROSE).unwrap();
        let profile = &pdf.get_object((5, 0)).unwrap().as_stream().unwrap().content;
        assert_eq!(gray_profile_tags(profile), Some(true));
        for (range, value) in [
            (132..136, *b"A2B0"),
            (144..148, *b"desc"),
            (340..344, [0, 1, 0, 0]),
            (344..348, [0; 4]),
        ] {
            let mut malformed = profile.clone();
            malformed[range].copy_from_slice(&value);
            assert_ne!(gray_profile_tags(&malformed), Some(true));
        }
    }

    #[test]
    fn icc_gray_proof_refuses_trailing_compressed_data() {
        let mut pdf = lopdf::Document::load_mem(ICC_PROSE).unwrap();
        let image = pdf.get_object_mut((4, 0)).unwrap().as_stream_mut().unwrap();
        let mut compressed = image.content.clone();
        compressed.push(0);
        image.set_content(compressed);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let document = PdfDocument::from_bytes(bytes).unwrap();
        let source = icc_gray_image_source(&document, 0, "Panel", usize::MAX).unwrap();
        assert!(super::super::decode_exact_flate(&source.data, source.decoded_size().unwrap()).is_none());
        assert!(!proof(1224 * 1584, usize::MAX).has_prose(&document, 0));
    }

    fn icc_source_after(edit: impl FnOnce(&mut lopdf::Dictionary)) -> Option<ImageSource> {
        icc_source_after_document(|pdf| edit(&mut pdf.get_object_mut((4, 0)).unwrap().as_stream_mut().unwrap().dict))
    }

    fn icc_source_after_document(edit: impl FnOnce(&mut lopdf::Document)) -> Option<ImageSource> {
        let mut pdf = lopdf::Document::load_mem(ICC_PROSE).unwrap();
        edit(&mut pdf);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let document = PdfDocument::from_bytes(bytes).unwrap();
        icc_gray_image_source(&document, 0, "Panel", usize::MAX)
    }

    #[test]
    fn icc_gray_source_declines_non_8_bit_samples() {
        assert!(icc_source_after(|_| {}).is_some());
        assert!(icc_source_after(|dict| dict.set("BitsPerComponent", 16)).is_none());
    }

    #[test]
    fn icc_gray_source_declines_extra_image_dictionary_keys() {
        assert!(icc_source_after(|dict| dict.set("Interpolate", true)).is_none());
    }

    fn icc_source_after_profile(edit: impl FnOnce(&mut lopdf::Stream)) -> Option<ImageSource> {
        icc_source_after_document(|pdf| edit(pdf.get_object_mut((5, 0)).unwrap().as_stream_mut().unwrap()))
    }

    fn icc_source_after_profile_bytes(edit: impl FnOnce(&mut Vec<u8>)) -> Option<ImageSource> {
        icc_source_after_profile(|profile| {
            let mut bytes = profile.content.clone();
            edit(&mut bytes);
            profile.set_content(bytes);
        })
    }

    #[test]
    fn icc_gray_source_declines_a_page_with_a_color_space_resource() {
        assert!(icc_source_after_document(|_| {}).is_some());
        let declined = icc_source_after_document(|pdf| {
            pdf.get_object_mut((7, 0))
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("ColorSpace", lopdf::Dictionary::new());
        });
        assert!(declined.is_none());
    }

    #[test]
    fn icc_gray_source_declines_an_image_whose_length_differs_from_its_data() {
        let with_length = |extra: i64| {
            icc_source_after(|dict| {
                let length = dict.get(b"Length").unwrap().as_i64().unwrap();
                dict.set("Length", length + extra);
            })
        };
        assert!(with_length(0).is_some());
        // +1 is absent on purpose: the parser takes the newline before endstream as data, so the Length then matches.
        for extra in [-1, 10] {
            assert!(with_length(extra).is_none(), "Length {extra:+}");
        }
    }

    #[test]
    fn icc_gray_source_declines_a_profile_whose_length_differs_from_its_data() {
        let with_length = |extra: i64| {
            icc_source_after_profile(|profile| {
                let length = i64::try_from(profile.content.len()).unwrap();
                profile.dict.set("Length", length + extra);
            })
        };
        assert!(with_length(0).is_some());
        for extra in [-1, 10] {
            assert!(with_length(extra).is_none(), "Length {extra:+}");
        }
    }

    #[test]
    fn icc_gray_source_declines_a_profile_with_a_wrong_declared_size() {
        assert!(icc_source_after_profile_bytes(|_| {}).is_some());
        for delta in [-4_i64, 4] {
            let declined = icc_source_after_profile_bytes(|bytes| {
                let declared = i64::from(u32::from_be_bytes(bytes[..4].try_into().unwrap()));
                let wrong = u32::try_from(declared + delta).unwrap();
                bytes[..4].copy_from_slice(&wrong.to_be_bytes());
            });
            assert!(declined.is_none(), "declared size {delta:+}");
        }
    }

    #[test]
    fn icc_gray_source_declines_a_profile_that_is_not_a_display_device() {
        let declined = icc_source_after_profile_bytes(|bytes| bytes[12..16].copy_from_slice(b"scnr"));
        assert!(declined.is_none());
    }

    #[test]
    fn icc_gray_source_declines_a_profile_with_another_alternate_space() {
        let declined = icc_source_after_profile(|profile| profile.dict.set("Alternate", "DeviceRGB"));
        assert!(declined.is_none());
    }

    #[test]
    fn icc_gray_source_declines_a_profile_over_the_byte_cap() {
        let pad = |bytes: &mut Vec<u8>, total: usize| {
            bytes.resize(total, 0);
            bytes[..4].copy_from_slice(&u32::try_from(total).unwrap().to_be_bytes());
        };
        assert!(icc_source_after_profile_bytes(|bytes| pad(bytes, MAX_GRAY_PROFILE_BYTES)).is_some());
        assert!(icc_source_after_profile_bytes(|bytes| pad(bytes, MAX_GRAY_PROFILE_BYTES + 4)).is_none());
    }

    #[test]
    fn icc_gray_source_declines_a_profile_without_a_white_point_or_tone_curve() {
        let without_white_point = icc_source_after_profile_bytes(|bytes| bytes[156..160].copy_from_slice(b"bkpt"));
        assert!(without_white_point.is_none());
        let without_tone_curve =
            icc_source_after_profile_bytes(|bytes| bytes[128..132].copy_from_slice(&3_u32.to_be_bytes()));
        assert!(without_tone_curve.is_none());
    }

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
