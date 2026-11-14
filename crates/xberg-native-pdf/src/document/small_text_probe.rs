use super::*;
use crate::content::{Operator, parser::visit_strict_content_stream};
use crate::extractors::TextExtractor;
use crate::fonts::FontInfo;

const PROBE_BYTES: usize = 64 * 1024;
const PROBE_OBJECT_BUDGET: usize = 2 * 1024 * 1024;
const MAX_OPERATORS: usize = 1024;
const MAX_FONTS: usize = 8;
const MAX_STREAMS: usize = 16;
const MAX_SMALL_FONT_PT: f32 = 8.0;
const MIN_SMALL_LETTERS: usize = 32;

#[derive(Clone, Copy)]
struct PaintState {
    fill: f64,
    stroke: f64,
    mode: u8,
    font_selected: bool,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            fill: 1.0,
            stroke: 1.0,
            mode: 0,
            font_selected: false,
        }
    }
}

impl PaintState {
    fn paints_text(self) -> bool {
        (matches!(self.mode, 0 | 2) && self.fill > 0.0) || (matches!(self.mode, 1 | 2) && self.stroke > 0.0)
    }
}

impl PdfDocument {
    pub(super) fn probe_small_painted_text(&self) -> bool {
        DensityProbe::new(self).qualifies().unwrap_or(false)
    }
}

struct DensityProbe<'a> {
    document: &'a PdfDocument,
    remaining: usize,
}

impl<'a> DensityProbe<'a> {
    fn new(document: &'a PdfDocument) -> Self {
        Self {
            document,
            remaining: PROBE_OBJECT_BUDGET,
        }
    }

    fn qualifies(&mut self) -> Option<bool> {
        let (page, parent) = self.single_page()?;
        let page = page.as_dict()?;
        let parent = parent.as_dict()?;
        let media = self.page_bounds(page.get("MediaBox").or_else(|| parent.get("MediaBox"))?)?;
        if let Some(crop) = page.get("CropBox").or_else(|| parent.get("CropBox"))
            && self.page_bounds(crop)? != media
        {
            return Some(false);
        }
        for unit in [page.get("UserUnit"), parent.get("UserUnit")].into_iter().flatten() {
            let unit = self.resolve(unit)?;
            if unit.as_real().or_else(|| unit.as_integer().map(|value| value as f64)) != Some(1.0) {
                return Some(false);
            }
        }
        let [left, bottom, right, top] = media;
        let data = self.bounded_probe_content_with(page.get("Contents")?, append_probe_stream)?;
        let resources = self.resolve(page.get("Resources").or_else(|| parent.get("Resources"))?)?;
        let spans = self.probe_text_spans(&data, &resources)?;
        Some(
            spans
                .iter()
                .filter(|span| {
                    let rect = &span.bbox;
                    rect.width > 0.0
                        && rect.height > 0.0
                        && rect.x >= left
                        && rect.y >= bottom
                        && rect.x + rect.width <= right
                        && rect.y + rect.height <= top
                })
                .flat_map(|span| span.text.chars())
                .filter(|character| character.is_alphabetic())
                .take(MIN_SMALL_LETTERS)
                .count()
                == MIN_SMALL_LETTERS,
        )
    }

    fn single_page(&mut self) -> Option<(Object, Object)> {
        if self.document.is_encrypted() || self.document.xref_reconstructed {
            return None;
        }
        let document = self.document;
        let root = self.resolve(document.trailer().as_dict()?.get("Root")?)?;
        let catalog = root.as_dict()?;
        if catalog.get("Type")?.as_name()? != "Catalog" {
            return None;
        }
        let parent = self.resolve(catalog.get("Pages")?)?;
        let dict = parent.as_dict()?;
        if dict.get("Type")?.as_name()? != "Pages" || dict.get("Count")?.as_integer()? != 1 {
            return None;
        }
        let kids = self.resolve(dict.get("Kids")?)?;
        let [page] = kids.as_array()?.as_slice() else {
            return None;
        };
        let page = self.resolve(page)?;
        (page.as_dict()?.get("Type")?.as_name()? == "Page").then_some((page, parent))
    }

    fn page_bounds(&mut self, media_box: &Object) -> Option<[f32; 4]> {
        let bounds = self.resolve(media_box)?;
        let mut coordinates = [0.0; 4];
        let [a, b, c, d] = bounds.as_array()?.as_slice() else {
            return None;
        };
        for (coordinate, object) in coordinates.iter_mut().zip([a, b, c, d]) {
            let value = self.resolve(object)?;
            *coordinate = value.as_real().or_else(|| value.as_integer().map(|n| n as f64))? as f32;
            if !coordinate.is_finite() {
                return None;
            }
        }
        let [x1, y1, x2, y2] = coordinates;
        (x1 != x2 && y1 != y2).then_some([x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)])
    }

    fn resolve(&mut self, object: &Object) -> Option<Object> {
        self.document.resolve_density_object(object, &mut self.remaining)
    }

    fn bounded_probe_content_with(
        &mut self,
        contents: &Object,
        mut append: impl FnMut(&mut Vec<u8>, &HashMap<String, Object>, &[u8], usize) -> Option<()>,
    ) -> Option<Vec<u8>> {
        let contents = self.resolve(contents)?;
        let streams = contents
            .as_array()
            .map_or(std::slice::from_ref(&contents), Vec::as_slice);
        if streams.len() > MAX_STREAMS {
            return None;
        }
        let mut encoded_remaining = PROBE_BYTES;
        let mut bounded_streams = Vec::with_capacity(streams.len());
        for stream in streams {
            let Object::Stream { dict, data } = self.resolve(stream)? else {
                return None;
            };
            encoded_remaining = encoded_remaining.checked_sub(data.len() + 1)?;
            bounded_streams.push((dict, data));
        }
        let mut output = Vec::new();
        for (dict, data) in bounded_streams {
            let remaining = PROBE_BYTES.checked_sub(output.len() + 1)?;
            append(&mut output, &dict, &data, remaining)?;
            output.push(b'\n');
        }
        Some(output)
    }

    fn probe_text_spans(&mut self, data: &[u8], resources: &Object) -> Option<Vec<TextSpan>> {
        let mut extractor = TextExtractor::new();
        extractor.skip_nonpainting_text = true;
        extractor.prepare_for_span_extraction();
        let mut paint = PaintState::default();
        let mut stack = Vec::new();
        let mut font_names = HashSet::new();
        let mut font_resources = None;
        let mut in_text = false;
        let mut marked_depth = 0usize;
        let mut saw_small_text = false;
        let complete = visit_strict_content_stream(data, MAX_OPERATORS, |op| {
            (|| {
                match &op {
                    Operator::ClipNonZero | Operator::ClipEvenOdd => return None,
                    Operator::SaveState => stack.push(paint),
                    Operator::RestoreState => paint = stack.pop()?,
                    Operator::BeginText if !in_text => in_text = true,
                    Operator::EndText if in_text => in_text = false,
                    Operator::BeginText | Operator::EndText => return None,
                    Operator::Tr { render: 4..=7 } => return None,
                    Operator::Tr { render } => paint.mode = *render,
                    Operator::Tf { .. } if !in_text => return None,
                    Operator::Tf { font, .. } => {
                        paint.font_selected = true;
                        if !font_names.contains(font) {
                            self.load_probe_font(
                                resources,
                                &mut font_resources,
                                &mut font_names,
                                &mut extractor,
                                font,
                            )?;
                        }
                    }
                    Operator::SetExtGState { dict_name } => return self.probe_alpha(resources, dict_name, &mut paint),
                    // The image header proves its type. A later image can still cover earlier glyphs. ~keep
                    Operator::Do { .. } if saw_small_text => return None,
                    Operator::Do { name } => return self.probe_image(resources, name),
                    Operator::BeginMarkedContent { tag } | Operator::BeginMarkedContentDict { tag, .. } => {
                        marked_depth += 1;
                        return (tag != "OC").then_some(());
                    }
                    Operator::EndMarkedContent => {
                        marked_depth = marked_depth.checked_sub(1)?;
                        return Some(());
                    }
                    _ => {}
                }
                saw_small_text |= execute_probe_operator(&mut extractor, op, paint, in_text)?;
                Some(())
            })()
            .is_some()
        })
        .ok()?;
        if !complete || in_text || marked_depth != 0 || !stack.is_empty() {
            return None;
        }
        extractor.finish_density_probe()
    }

    fn load_probe_font(
        &mut self,
        resources: &Object,
        font_resources: &mut Option<Object>,
        font_names: &mut HashSet<String>,
        extractor: &mut TextExtractor,
        font: &str,
    ) -> Option<()> {
        if font_names.len() == MAX_FONTS {
            return None;
        }
        if font_resources.is_none() {
            *font_resources = Some(self.resource_group(resources, "Font")?);
        }
        let object = font_resources.as_ref()?.as_dict()?.get(font)?;
        extractor.add_font(font.to_owned(), self.probe_font(object)?);
        font_names.insert(font.to_owned());
        Some(())
    }

    fn resource_group(&mut self, resources: &Object, group: &str) -> Option<Object> {
        self.resolve(resources.as_dict()?.get(group)?)
    }

    fn probe_image(&mut self, resources: &Object, name: &str) -> Option<()> {
        let group = self.resource_group(resources, "XObject")?;
        let reference = group.as_dict()?.get(name)?.as_reference()?;
        self.document.prove_image_stream(reference, &mut self.remaining)
    }

    fn probe_font(&mut self, object: &Object) -> Option<FontInfo> {
        let object = self.resolve(object)?;
        let Object::Dictionary(mut dict) = object else {
            return None;
        };
        if !matches!(dict.get("Subtype")?.as_name()?, "Type1" | "TrueType")
            || dict.get("Encoding")?.as_name()? != "WinAnsiEncoding"
            || dict.contains_key("ToUnicode")
        {
            return None;
        }
        if let Some(descriptor) = dict.get("FontDescriptor") {
            let descriptor = self.resolve(descriptor)?;
            let flags = descriptor.as_dict()?.get("Flags")?.as_integer()?;
            if flags & 4 != 0 {
                return None;
            }
        }
        // Density needs the explicit encoding and widths, never the embedded program.
        dict.retain(|key, _| {
            matches!(
                key.as_str(),
                "Subtype" | "BaseFont" | "Encoding" | "FirstChar" | "LastChar" | "Widths"
            )
        });
        if let Some(widths) = dict.get_mut("Widths") {
            let resolved = self.resolve(widths)?;
            let values = resolved.as_array()?;
            if values.len() > 256 || values.iter().any(|n| n.as_integer().is_none() && n.as_real().is_none()) {
                return None;
            }
            *widths = resolved;
        }
        FontInfo::from_dict(&Object::Dictionary(dict), self.document).ok()
    }

    fn probe_alpha(&mut self, resources: &Object, name: &str, state: &mut PaintState) -> Option<()> {
        let group = self.resource_group(resources, "ExtGState")?;
        let object = self.resolve(group.as_dict()?.get(name)?)?;
        let dict = object.as_dict()?;
        // Font changes, soft masks and blend modes need the renderer's interpretation.
        if dict.keys().any(|key| !matches!(key.as_str(), "Type" | "ca" | "CA")) {
            return None;
        }
        for (key, alpha) in [("ca", &mut state.fill), ("CA", &mut state.stroke)] {
            if let Some(value) = dict.get(key) {
                *alpha = value
                    .as_real()
                    .or_else(|| value.as_integer().map(|value| value as f64))?;
                if !(0.0..=1.0).contains(alpha) {
                    return None;
                }
            }
        }
        Some(())
    }
}

fn execute_probe_operator(
    extractor: &mut TextExtractor,
    op: Operator,
    paint: PaintState,
    in_text: bool,
) -> Option<bool> {
    let shows_text = matches!(
        op,
        Operator::Tj { .. } | Operator::TJ { .. } | Operator::Quote { .. } | Operator::DoubleQuote { .. }
    );
    if shows_text && (!in_text || !paint.font_selected) {
        return None;
    }
    let hide = shows_text && {
        let height = extractor.density_probe_font_height();
        !paint.paints_text() || !(height > 0.0 && height <= MAX_SMALL_FONT_PT)
    };
    if hide {
        extractor.flush_public().ok()?;
        extractor.execute_operator_public(Operator::Tr { render: 3 }).ok()?;
    }
    extractor.execute_operator_public(op).ok()?;
    if hide {
        extractor.flush_public().ok()?;
        extractor
            .execute_operator_public(Operator::Tr { render: paint.mode })
            .ok()?;
    }
    Some(shows_text && !hide)
}

fn append_probe_stream(
    output: &mut Vec<u8>,
    dict: &HashMap<String, Object>,
    data: &[u8],
    remaining: usize,
) -> Option<()> {
    let start = output.len();
    match dict.get("Filter") {
        None if data.len() <= remaining => output.extend_from_slice(data),
        Some(filter) if filter.as_name() == Some("FlateDecode") && !dict.contains_key("DecodeParms") => {
            let mut decoder = flate2::Decompress::new(true);
            let limit = remaining.checked_add(1)?;
            output.resize(start + limit, 0);
            let status = decoder
                .decompress(data, &mut output[start..], flate2::FlushDecompress::Finish)
                .ok()?;
            output.truncate(start + usize::try_from(decoder.total_out()).ok()?);
            if status != flate2::Status::StreamEnd || decoder.total_in() != data.len() as u64 {
                return None;
            }
        }
        _ => return None,
    }
    (output.len() - start <= remaining).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::tests::common::build_minimal_pdf;
    use std::io::Write;

    const SMALL_TEXT: &str = "BT /F1 7 Tf 36 450 Td (A long line of small painted letters that qualifies) Tj ET";

    fn page(content: &str, extra_resources: &str) -> PdfDocument {
        custom_page(
            content,
            &format!("/Font << /F1 5 0 R >> {extra_resources}"),
            &[HELVETICA.into()],
        )
    }

    #[test]
    fn density_probe_uses_the_rendered_vertical_axis_for_sheared_text() {
        let text = "A long line of small painted letters that qualifies";
        for (ctm, text_matrix, expected) in [
            ("", "1 0 1 1 36 450", false),
            ("", "1 1 0 1 36 450", true),
            ("2 0 0 1 0 0 cm ", "1 0 .5 1 36 450", false),
        ] {
            let content = format!("{ctm}BT /F1 7 Tf {text_matrix} Tm ({text}) Tj ET");
            assert_eq!(
                page(&content, "").has_small_painted_text_for_ocr(),
                expected,
                "CTM {ctm:?}, text matrix {text_matrix}"
            );
        }
    }

    #[test]
    fn density_probe_requires_complete_content_within_operator_budget() {
        for (suffix, expected) in [
            (String::new(), true),
            (" 99".into(), false),
            (" 42 ET".into(), false),
            (" /Span << /Type BareWord >> BDC EMC".into(), false),
            (" BT".into(), false),
            (" q".into(), false),
            (" /Span BMC".into(), false),
            (" n".repeat(MAX_OPERATORS - 5), true),
            (" n".repeat(MAX_OPERATORS - 4), false),
        ] {
            let doc = page(&format!("{SMALL_TEXT}{suffix}"), "");
            assert_eq!(
                doc.has_small_painted_text_for_ocr(),
                expected,
                "suffix length {}",
                suffix.len()
            );
        }
    }

    #[test]
    fn density_probe_shares_remaining_budget_across_repeated_resource_lookups() {
        let resources = "/ExtGState << /Opacity << /ca 1 >> >>";
        let doc = page(&format!("{SMALL_TEXT} /Opacity gs /Opacity gs"), resources);
        let one = page(&format!("{SMALL_TEXT} /Opacity gs            "), resources);
        let mut measured = DensityProbe::new(&one);
        assert_eq!(measured.qualifies(), Some(true));
        let charge_for_one = PROBE_OBJECT_BUDGET - measured.remaining;
        assert_eq!(
            DensityProbe {
                document: &one,
                remaining: charge_for_one
            }
            .qualifies(),
            Some(true)
        );
        assert_eq!(
            DensityProbe {
                document: &doc,
                remaining: charge_for_one
            }
            .qualifies(),
            None
        );
        assert!(doc.has_small_painted_text_for_ocr());
    }

    #[test]
    fn density_probe_refuses_content_clipping_with_either_fill_rule() {
        for (operator, expected) in [("", true), ("W", false), ("W*", false)] {
            let content = format!("q 0 0 600 400 re {operator} n {SMALL_TEXT} Q");
            assert_eq!(
                page(&content, "").has_small_painted_text_for_ocr(),
                expected,
                "{operator:?}"
            );
        }
    }

    #[test]
    fn density_probe_credits_only_fully_contained_spans() {
        for (x, expected) in [(36, true), (-1, false), (599, false)] {
            let content = SMALL_TEXT.replace("36 450 Td", &format!("{x} 450 Td"));
            assert_eq!(page(&content, "").has_small_painted_text_for_ocr(), expected, "x={x}");
        }
    }

    #[test]
    fn density_probe_keeps_default_when_crop_excludes_small_text() {
        for (page_box, parent_box, expected) in [
            ("", "", true),
            ("/CropBox [0 0 600 800]", "", true),
            ("/CropBox [0 0 600 400]", "", false),
            ("", "/CropBox [0 0 600 400]", false),
        ] {
            let doc = custom_page_geometry(
                SMALL_TEXT,
                "/Font << /F1 5 0 R >>",
                &[HELVETICA.into()],
                page_box,
                parent_box,
            );
            assert_eq!(
                doc.has_small_painted_text_for_ocr(),
                expected,
                "{page_box} {parent_box}"
            );
        }
    }

    #[test]
    fn density_probe_keeps_default_for_nonunit_page_scale() {
        for (unit, expected) in [("/UserUnit 1", true), ("/UserUnit 2", false), ("/UserUnit 0.5", false)] {
            let doc = custom_page_geometry(SMALL_TEXT, "/Font << /F1 5 0 R >>", &[HELVETICA.into()], unit, "");
            assert_eq!(doc.has_small_painted_text_for_ocr(), expected, "{unit}");
        }
    }

    fn custom_page(content: &str, resources: &str, extras: &[String]) -> PdfDocument {
        custom_page_geometry(content, resources, extras, "", "")
    }

    fn custom_page_geometry(
        content: &str,
        resources: &str,
        extras: &[String],
        page_extra: &str,
        parent_extra: &str,
    ) -> PdfDocument {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            format!("<< /Type /Pages /Kids [3 0 R] /Count 1 {parent_extra} >>"),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] {page_extra} /Contents 4 0 R /Resources << {resources} >> >>"
            ),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        ];
        objects.extend_from_slice(extras);
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
        }
        let xref = pdf.len();
        let size = offsets.len() + 1;
        pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!("trailer\n<< /Root 1 0 R /Size {size} >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        PdfDocument::from_bytes(pdf).unwrap()
    }

    const HELVETICA: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

    #[test]
    fn rejects_stream_when_declared_length_would_hide_an_invalid_tail() {
        let original = custom_page(SMALL_TEXT, "/Font << /F1 5 0 R >>", &[HELVETICA.into()]);
        assert!(original.has_small_painted_text_for_ocr());
        let bytes = original.source_bytes.as_slice();
        let mut content = bytes.to_vec();
        let old = format!("/Length {}", SMALL_TEXT.len());
        let new = format!("/Length {}", SMALL_TEXT.len() - 1);
        let offset = bytes
            .windows(old.len())
            .position(|window| window == old.as_bytes())
            .unwrap();
        content[offset..offset + old.len()].copy_from_slice(new.as_bytes());
        assert!(
            !PdfDocument::from_bytes(content)
                .unwrap()
                .has_small_painted_text_for_ocr()
        );
    }

    #[test]
    fn density_probe_only_resolves_used_supported_fonts_without_populating_caches() {
        let resources = "/Font << /F1 5 0 R /Missing 99 0 R >>";
        let extras = [HELVETICA.into()];
        let doc = custom_page(SMALL_TEXT, resources, &extras);
        let objects_before = doc.object_cache.lock_or_recover().len();
        let fonts_before = doc.font_cache.lock_or_recover().len();
        assert!(doc.has_small_painted_text_for_ocr());
        assert_eq!(doc.object_cache.lock_or_recover().len(), objects_before);
        assert_eq!(doc.font_cache.lock_or_recover().len(), fonts_before);
        let used_missing = SMALL_TEXT.replace("/F1", "/Missing");
        assert!(!custom_page(&used_missing, resources, &extras).has_small_painted_text_for_ocr());
        let symbolic = [
            HELVETICA.replace(">>", "/FontDescriptor 6 0 R >>"),
            "<< /Flags 4 >>".into(),
        ];
        assert!(!custom_page(SMALL_TEXT, resources, &symbolic).has_small_painted_text_for_ocr());
    }

    #[test]
    fn density_probe_refuses_small_text_covered_by_a_later_image() {
        let resources = "/Font << /F1 5 0 R >> /XObject << /Cover 6 0 R >>";
        let image = "<< /Type /XObject /Subtype /Image /Length 1 /Width 1 /Height 1 /BitsPerComponent 8 /ColorSpace /DeviceGray >>\nstream\nX\nendstream";
        let extras = [HELVETICA.into(), image.into()];
        let cover = "q 500 0 0 40 20 430 cm /Cover Do Q";
        let background = custom_page(&format!("{cover} {SMALL_TEXT}"), resources, &extras);
        let foreground = custom_page(&format!("{SMALL_TEXT} {cover}"), resources, &extras);
        assert!(background.has_small_painted_text_for_ocr());
        assert!(!foreground.has_small_painted_text_for_ocr());
    }

    #[test]
    fn density_probe_reads_large_background_image_header_without_resolving_payload() {
        let content = format!("q 10 0 0 10 0 0 cm /Logo Do Q {SMALL_TEXT}");
        let resources = "/Font << /F1 5 0 R >> /XObject << /Logo 6 0 R >>";
        let payload = "X".repeat(128 * 1024);
        let image = format!(
            "<< /Type /XObject /Subtype /Image /Length {} /Width 512 /Height 256 /BitsPerComponent 8 /ColorSpace /DeviceGray >>\nstream\n{payload}\nendstream",
            payload.len()
        );
        let doc = custom_page(&content, resources, &[HELVETICA.into(), image.clone()]);
        let mut probe = DensityProbe::new(&doc);
        assert!(probe.resolve(&Object::Reference(ObjectRef::new(6, 0))).is_none());
        assert!(doc.has_small_painted_text_for_ocr());
        let form = image.replace("/Image", "/Form");
        assert!(!custom_page(&content, resources, &[HELVETICA.into(), form]).has_small_painted_text_for_ocr());
        let truncated = image.replace("endstream", "endstrea");
        assert!(!custom_page(&content, resources, &[HELVETICA.into(), truncated]).has_small_painted_text_for_ocr());
    }

    #[test]
    fn rejects_aggregate_encoded_size_before_decoding_any_stream() {
        let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
        let stream = Object::Stream {
            dict: HashMap::new(),
            data: vec![b' '; PROBE_BYTES / 2].into(),
        };
        let contents = Object::Array(vec![stream.clone(), stream]);
        assert!(
            DensityProbe::new(&doc)
                .bounded_probe_content_with(&contents, |_, _, _, _| {
                    panic!("oversized aggregate reached the decoder")
                })
                .is_none()
        );
    }

    #[test]
    fn compressed_streams_share_one_decoded_budget() {
        let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&vec![b' '; PROBE_BYTES / 2]).unwrap();
        let stream = Object::Stream {
            dict: HashMap::from([("Filter".into(), Object::Name("FlateDecode".into()))]),
            data: encoder.finish().unwrap().into(),
        };
        assert_eq!(
            DensityProbe::new(&doc)
                .bounded_probe_content_with(&stream, append_probe_stream)
                .unwrap()
                .len(),
            PROBE_BYTES / 2 + 1
        );
        assert!(
            DensityProbe::new(&doc)
                .bounded_probe_content_with(&Object::Array(vec![stream.clone(), stream]), append_probe_stream)
                .is_none()
        );
    }

    #[test]
    fn rejects_truncated_flate_after_complete_text_prefix() {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(SMALL_TEXT.as_bytes()).unwrap();
        let mut data = encoder.finish().unwrap();
        let dict = HashMap::from([("Filter".into(), Object::Name("FlateDecode".into()))]);
        assert!(append_probe_stream(&mut Vec::new(), &dict, &data, PROBE_BYTES).is_some());
        data.pop();
        assert!(append_probe_stream(&mut Vec::new(), &dict, &data, PROBE_BYTES).is_none());
    }

    #[test]
    fn small_text_probe_leaves_actualtext_ownership_and_content_cache_untouched() {
        let content = "/Span << /MCID 5 /ActualText (Replacement text) >> BDC BT /F1 7 Tf 36 450 Td (A long line of small painted letters that qualifies) Tj ET EMC";
        let doc = page(content, "");
        let sentinel = HashSet::from([99]);
        doc.mc_actualtext_mcids.lock_or_recover().insert(0, sentinel.clone());
        assert!(doc.has_small_painted_text_for_ocr());
        assert!(doc.has_small_painted_text_for_ocr());
        assert_eq!(doc.mc_actualtext_mcids.lock_or_recover().get(&0), Some(&sentinel));
        assert!(doc.page_content_cache.lock_or_recover().get(&0).is_none());
    }
}
