use super::document::{
    LineBox, native_layer_decodes_cleanly, ocr_confirms_layer, ocr_page_has_only_plain_payload_with,
    processed_ocr_layout_dimensions, valid_ocr_layout_dimension,
};
use crate::core::config::OcrQualityThresholds;
use crate::ocr_metadata_keys::{
    OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
    OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY,
};
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};
use xberg_native_pdf::{PdfDocument, geometry::Rect};

/// Compare the source's glyph-derived words against the actual Tesseract result iterator before
/// the backend's dictionary filtering. Neither reflowed lines nor approximate block subdivisions
/// establish word ownership.
pub(super) fn covers_complete_native_page(
    pdf: &PdfDocument,
    page_index: usize,
    layer: &str,
    read: &ExtractedDocument,
) -> bool {
    if layer.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER) {
        return false;
    }
    let Some(elements) = read.ocr_elements.as_deref() else {
        return false;
    };
    let words = || elements.iter().filter(|element| element.level == OcrElementLevel::Word);
    if !layer.split_whitespace().eq(words().map(|word| word.text.trim())) {
        return false;
    }
    let Some(first) = words().next() else {
        return false;
    };
    if !is_iterator_word(first) {
        return false;
    }
    let Some(raster) = processed_ocr_layout_dimensions(&read.metadata) else {
        return false;
    };
    let Ok(info) = pdf.get_page_info(page_index) else {
        return false;
    };
    let media = info.media_box;
    if info.rotation != 0 || media.x != 0.0 || media.y != 0.0 || info.crop_box.is_some_and(|crop| crop != media) {
        return false;
    }
    let Ok(lines) = pdf.extract_text_lines(page_index) else {
        return false;
    };
    let native = || lines.iter().flat_map(|line| &line.words);
    layer.split_whitespace().eq(native().map(|word| word.text.as_str()))
        && native().zip(words()).all(|(source, ocr)| {
            is_iterator_word(ocr)
                && ocr_word_in_page_points(ocr, raster, media)
                    .is_some_and(|placed| native_box(source.bbox).owns(&placed))
        })
}

/// What bounds an ink proof: the quality thresholds that judge the native layer, the memory the
/// two renders may use, and a cancellation check consulted between them.
pub(super) struct InkProofLimits<'a> {
    pub(super) thresholds: &'a OcrQualityThresholds,
    pub(super) max_working_bytes: usize,
    pub(super) cancelled: &'a dyn Fn() -> bool,
}

/// Luminance difference, on a 0-255 scale, that makes a pixel visibly differ from its surround.
const MIN_VISIBLE_CONTRAST: u8 = 51;
/// Resolution of the two rasters that tell glyph ink from every other mark. Word boxes are tested
/// in points, so the resolution only has to resolve a glyph; it is not an OCR resolution.
const INK_PROOF_DPI: u32 = 144;

/// Whether the native text paints all the ink that a Tesseract read of the page sits on, for a
/// read whose words do not literally equal the native words. Every iterator word must lie in a
/// native text line, show glyph ink, and hold no mark that is not a glyph, so the read can only
/// be a second reading of ink the native text already holds. A word that differs from the native
/// word at its place is still a reading of that ink; a word over a raster, an outline, a rule or
/// a fill is not provably one.
pub(super) fn native_text_paints_all_read_ink(
    pdf: &PdfDocument,
    page_index: usize,
    layer: &str,
    read: &ExtractedDocument,
    limits: InkProofLimits<'_>,
) -> bool {
    let InkProofLimits {
        thresholds,
        max_working_bytes,
        cancelled,
    } = limits;
    if layer.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER)
        || !native_layer_decodes_cleanly(layer, thresholds)
        || !ocr_confirms_layer(layer, &read.content, thresholds)
    {
        return false;
    }
    let Some(elements) = read.ocr_elements.as_deref() else {
        return false;
    };
    let words = || elements.iter().filter(|element| element.level == OcrElementLevel::Word);
    if words().next().is_none()
        || !words().all(is_iterator_word)
        || !read.content.split_whitespace().eq(words().map(|word| word.text.trim()))
    {
        return false;
    }
    let Some(raster) = processed_ocr_layout_dimensions(&read.metadata) else {
        return false;
    };
    let Ok(info) = pdf.get_page_info(page_index) else {
        return false;
    };
    let media = info.media_box;
    if info.rotation != 0 || media.x != 0.0 || media.y != 0.0 || info.crop_box.is_some_and(|crop| crop != media) {
        return false;
    }
    let Ok(lines) = pdf.extract_text_lines(page_index) else {
        return false;
    };
    let line_boxes: Vec<LineBox> = lines.iter().map(|line| native_box(line.bbox)).collect();
    let placed: Option<Vec<LineBox>> = words()
        .map(|word| {
            ocr_word_in_page_points(word, raster, media)
                .filter(|placed| line_boxes.iter().any(|line| line.owns(placed)))
        })
        .collect();
    let Some(placed) = placed else {
        return false;
    };
    let Ok(Some(rasters)) = xberg_native_pdf::rendering::render_text_ink_rasters(
        pdf,
        page_index,
        INK_PROOF_DPI,
        max_working_bytes,
        cancelled,
    ) else {
        return false;
    };
    let scale = INK_PROOF_DPI as f32 / 72.0;
    placed.iter().all(|word| glyph_ink_only(&rasters, word, media, scale))
}

fn glyph_ink_only(
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    placed: &LineBox,
    media: Rect,
    scale: f32,
) -> bool {
    let (width, height) = (rasters.width as usize, rasters.height as usize);
    let columns = (placed.left * scale).floor().max(0.0) as usize..((placed.right * scale).ceil() as usize).min(width);
    let rows = ((media.height - placed.bottom) * scale).floor().max(0.0) as usize
        ..(((media.height - placed.top) * scale).ceil() as usize).min(height);
    if columns.is_empty() || rows.is_empty() {
        return false;
    }
    let spread = |raster: &[u8]| {
        let (mut low, mut high) = (u8::MAX, u8::MIN);
        for row in rows.clone() {
            for &value in &raster[row * width + columns.start..row * width + columns.end] {
                low = low.min(value);
                high = high.max(value);
            }
        }
        high.saturating_sub(low)
    };
    let drawn_over_glyph_free = rows.clone().any(|row| {
        let at = row * width;
        rasters.drawn[at + columns.start..at + columns.end]
            .iter()
            .zip(&rasters.without_glyphs[at + columns.start..at + columns.end])
            .any(|(drawn, bare)| drawn.abs_diff(*bare) >= MIN_VISIBLE_CONTRAST)
    });
    spread(&rasters.without_glyphs) < MIN_VISIBLE_CONTRAST && drawn_over_glyph_free
}

fn is_iterator_word(word: &OcrElement) -> bool {
    word.backend_metadata.get("backend").and_then(serde_json::Value::as_str) == Some("tesseract-iterator")
}

fn native_box(rect: Rect) -> LineBox {
    LineBox {
        left: rect.x,
        top: rect.y,
        right: rect.x + rect.width,
        bottom: rect.y + rect.height,
    }
}

fn ocr_word_in_page_points(word: &OcrElement, raster: (u32, u32), media: Rect) -> Option<LineBox> {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = word.geometry
    else {
        return None;
    };
    let (raster_width, raster_height) = raster;
    let right = left.checked_add(width)?;
    let bottom = top.checked_add(height)?;
    if width == 0 || height == 0 || right > raster_width || bottom > raster_height {
        return None;
    }
    let scale_x = media.width / raster_width as f32;
    let scale_y = media.height / raster_height as f32;
    Some(LineBox {
        left: left as f32 * scale_x,
        right: right as f32 * scale_x,
        top: (raster_height - bottom) as f32 * scale_y,
        bottom: (raster_height - top) as f32 * scale_y,
    })
}

/// These entries describe the detached hOCR run, not extracted document metadata. The mixed merge
/// publishes only the segmentation modes of retained OCR pages from this page-local map.
pub(super) fn has_only_certified_tesseract_text(doc: &InternalDocument) -> bool {
    doc.prebuilt_ocr_elements.is_none() && ocr_page_has_only_plain_payload_with(doc, is_hocr_bookkeeping)
}

fn is_hocr_bookkeeping(key: &str, value: &serde_json::Value) -> bool {
    match key {
        "source_format" => value.as_str() == Some("hocr"),
        "language" => value.as_str().is_some_and(|language| !language.is_empty()),
        "psm" => value
            .as_str()
            .and_then(|mode| mode.parse::<u8>().ok())
            .is_some_and(|mode| mode <= 13),
        "mean_text_conf" | "p10_word_conf" | "median_word_conf" => {
            value.as_u64().is_some_and(|confidence| confidence <= 100)
        }
        "word_count" | "low_conf_word_count" => value.as_u64().is_some(),
        "tables_detected" | "table_count" => value.as_u64() == Some(0),
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY | OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY => {
            valid_ocr_layout_dimension(value).is_some()
        }
        OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY => {
            value.as_f64().is_some_and(|ratio| (0.0..=1.0).contains(&ratio))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::ElementKind;

    #[test]
    fn should_ignore_only_known_hocr_run_bookkeeping_in_a_plain_page() {
        let mut doc = super::super::document::flat_ocr_page_document("real source words");
        doc.metadata
            .additional
            .insert("source_format".into(), serde_json::json!("hocr"));
        doc.metadata
            .additional
            .insert("table_count".into(), serde_json::json!(0));
        assert!(has_only_certified_tesseract_text(&doc));
        assert!(!super::super::document::ocr_page_has_only_plain_payload(&doc));

        let mut unknown = doc.clone();
        unknown
            .metadata
            .additional
            .insert("unknown".into(), serde_json::json!(0));
        assert!(!has_only_certified_tesseract_text(&unknown));

        let mut invalid = doc.clone();
        invalid
            .metadata
            .additional
            .insert("table_count".into(), serde_json::json!(1));
        assert!(!has_only_certified_tesseract_text(&invalid));

        doc.elements[0].kind = ElementKind::Heading { level: 1 };
        assert!(!has_only_certified_tesseract_text(&doc));
    }

    mod ink_ownership {
        use super::*;
        use crate::types::Metadata;

        const PAGE: (f32, f32) = (612.0, 792.0);
        const SCALE: f32 = 2.0;
        const WORKING_BYTES: usize = 64 << 20;
        const PROSE: [&str; 6] = [
            "The committee met on Tuesday to review the plans for the library.",
            "Members asked about the schedule, budget, and planned parking.",
            "The architect said work would start after the winter season ends.",
            "A vote is planned for the next regular meeting of the council.",
            "Residents may send written comments before the end of the month.",
            "The treasurer reported that the reserve fund remained steady this year.",
        ];
        const LAYER: &str = concat!(
            "mov eax, ebx\n",
            "call puts\n",
            "The committee met on Tuesday to review the plans for the library.\n",
            "Members asked about the schedule, budget, and planned parking.\n",
            "The architect said work would start after the winter season ends.\n",
            "A vote is planned for the next regular meeting of the council.\n",
            "Residents may send written comments before the end of the month.\n",
            "The treasurer reported that the reserve fund remained steady this year.",
        );

        fn pdf_with(extra_content: &str) -> PdfDocument {
            pdf_between("", extra_content)
        }

        fn pdf_between(under_text: &str, over_text: &str) -> PdfDocument {
            use lopdf::{Document, Object, Stream, dictionary};

            let prose: String = PROSE
                .iter()
                .enumerate()
                .map(|(at, line)| format!("BT /F1 12 Tf 72 {} Td ({line}) Tj ET\n", 640 - 20 * at))
                .collect();
            let content = format!(
                "{under_text}\nBT /F1 12 Tf 72 700 Td (mov eax, ebx) Tj ET\n\
                 BT /F1 12 Tf 72 680 Td (call puts) Tj ET\n{prose}\n{over_text}"
            );
            let mut document = Document::with_version("1.5");
            let pages_id = document.new_object_id();
            let font_id = document.add_object(dictionary! {
                "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier",
            });
            let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
            let page_id = document.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            document.objects.insert(
                pages_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
                }),
            );
            let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
            document.trailer.set("Root", catalog_id);
            let mut bytes = Vec::new();
            document.save_to(&mut bytes).unwrap();
            PdfDocument::from_bytes(bytes).unwrap()
        }

        fn iterator_word(text: &str, placed: Rect) -> OcrElement {
            let raster_height = PAGE.1 * SCALE;
            OcrElement {
                text: text.to_owned(),
                level: OcrElementLevel::Word,
                geometry: OcrBoundingGeometry::Rectangle {
                    left: (placed.x * SCALE) as u32,
                    top: (raster_height - (placed.y + placed.height) * SCALE) as u32,
                    width: (placed.width * SCALE) as u32,
                    height: (placed.height * SCALE) as u32,
                },
                backend_metadata: [("backend".to_owned(), serde_json::json!("tesseract-iterator"))].into(),
                ..Default::default()
            }
        }

        fn read_of(words: Vec<OcrElement>) -> ExtractedDocument {
            let mut metadata = Metadata::default();
            metadata.additional.insert(
                OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
                serde_json::json!((PAGE.0 * SCALE) as u32),
            );
            metadata.additional.insert(
                OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
                serde_json::json!((PAGE.1 * SCALE) as u32),
            );
            ExtractedDocument {
                content: words
                    .iter()
                    .map(|word| word.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                ocr_elements: Some(words),
                metadata,
                ..Default::default()
            }
        }

        fn ink_owned(pdf: &PdfDocument, layer: &str, read: &ExtractedDocument) -> bool {
            native_text_paints_all_read_ink(
                pdf,
                0,
                layer,
                read,
                InkProofLimits {
                    thresholds: &OcrQualityThresholds::default(),
                    max_working_bytes: WORKING_BYTES,
                    cancelled: &|| false,
                },
            )
        }

        fn native_word_boxes(pdf: &PdfDocument) -> Vec<(String, Rect)> {
            pdf.extract_text_lines(0)
                .unwrap()
                .iter()
                .flat_map(|line| line.words.iter().map(|word| (word.text.clone(), word.bbox)))
                .collect()
        }

        fn owns(pdf: &PdfDocument, read_texts: Option<&[&str]>) -> bool {
            let boxes = native_word_boxes(pdf);
            assert_eq!(boxes.len(), LAYER.split_whitespace().count(), "{boxes:?}");
            let words = boxes
                .iter()
                .enumerate()
                .map(|(at, (text, placed))| {
                    iterator_word(
                        read_texts
                            .and_then(|texts| texts.get(at).copied())
                            .unwrap_or(text.as_str()),
                        *placed,
                    )
                })
                .collect();
            ink_owned(pdf, LAYER, &read_of(words))
        }

        #[test]
        fn should_own_a_read_whose_words_differ_from_the_native_words_it_sits_on() {
            let pdf = pdf_with("");
            assert!(owns(&pdf, None));
            assert!(owns(&pdf, Some(&["rnov", "eax.", "ebx", "ca11", "puts"])));
        }

        #[test]
        fn should_not_own_a_word_over_a_fill_that_ends_inside_it() {
            let pdf = pdf_between("0.6 g 72 696 10 14 re f 0 g", "");
            assert!(!owns(&pdf, None));
        }

        #[test]
        fn should_own_text_on_a_flat_shaded_background() {
            let pdf = pdf_between("0.6 g 60 670 200 45 re f 0 g", "");
            assert!(owns(&pdf, None));
        }

        #[test]
        fn should_not_own_a_word_painted_over_by_an_opaque_rectangle() {
            let pdf = pdf_with("1 g 72 696 43 14 re f");
            assert!(!owns(&pdf, None));
        }

        #[test]
        fn should_not_own_a_word_struck_by_a_vector_stroke() {
            let pdf = pdf_with("0 G 1 w 70 702 m 130 702 l S");
            assert!(!owns(&pdf, None));
        }

        #[test]
        fn should_not_own_a_stroked_box_outside_every_native_line() {
            let pdf = pdf_with("0 G 1 w 150 676 m 190 676 l 190 690 l 150 690 l h S");
            let mut boxes = native_word_boxes(&pdf);
            let outlined = Rect {
                x: 150.0,
                y: 676.0,
                width: 40.0,
                height: 14.0,
            };
            boxes.push(("outlined".into(), outlined));
            let words = boxes
                .iter()
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            assert!(!ink_owned(
                &pdf,
                "mov eax, ebx
call puts",
                &read_of(words)
            ));
        }

        #[test]
        fn should_not_own_a_word_on_blank_paper_inside_a_native_line() {
            let pdf = pdf_with("");
            let boxes = native_word_boxes(&pdf);
            let (first, second) = (boxes[0].1, boxes[1].1);
            let gap = Rect {
                x: first.x + first.width + 1.0,
                y: first.y,
                width: second.x - first.x - first.width - 2.0,
                height: first.height,
            };
            assert!(gap.width > 1.0, "{first:?} {second:?}");
            let mut words: Vec<_> = boxes
                .iter()
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            words.push(iterator_word("ghost", gap));
            assert!(!ink_owned(
                &pdf,
                "mov eax, ebx
call puts",
                &read_of(words)
            ));
        }

        #[test]
        fn should_not_own_a_word_box_shifted_mostly_off_its_line() {
            let pdf = pdf_with("");
            let mut boxes = native_word_boxes(&pdf);
            let last = boxes.len() - 1;
            boxes[last].1.y -= 0.6 * boxes[last].1.height;
            let words = boxes
                .iter()
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            assert!(!ink_owned(&pdf, LAYER, &read_of(words)));
        }

        #[test]
        fn should_not_own_a_page_whose_native_text_has_a_single_undecodable_character() {
            let pdf = pdf_with("");
            let words = native_word_boxes(&pdf)
                .iter()
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            assert!(!ink_owned(
                &pdf,
                &LAYER.replace("budget", "bud\u{E000}et"),
                &read_of(words)
            ));
        }

        fn read_matching_ink(pdf: &PdfDocument) -> ExtractedDocument {
            let words = native_word_boxes(pdf)
                .iter()
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            read_of(words)
        }

        /// `layer` with each word for which `wrong(index)` holds replaced by one the page never shows.
        fn layer_with_words_wrong(wrong: impl Fn(usize) -> bool) -> String {
            LAYER
                .split(' ')
                .enumerate()
                .map(|(at, word)| if wrong(at) { "zzzz" } else { word })
                .collect::<Vec<_>>()
                .join(" ")
        }

        #[test]
        fn should_prefer_a_plausibly_wrong_native_layer_that_the_read_still_half_confirms() {
            let pdf = pdf_with("");
            let read = read_matching_ink(&pdf);
            assert!(ink_owned(&pdf, &layer_with_words_wrong(|at| at % 5 == 0), &read));
            assert!(ink_owned(&pdf, &layer_with_words_wrong(|at| at % 3 == 0), &read));
        }

        #[test]
        fn should_not_prefer_a_native_layer_the_read_does_not_confirm() {
            let pdf = pdf_with("");
            let read = read_matching_ink(&pdf);
            assert!(!ink_owned(&pdf, &layer_with_words_wrong(|_| true), &read));
        }

        #[test]
        fn should_not_render_when_cancelled() {
            let pdf = pdf_with("");
            let words = native_word_boxes(&pdf)
                .iter()
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            assert!(!native_text_paints_all_read_ink(
                &pdf,
                0,
                LAYER,
                &read_of(words),
                InkProofLimits {
                    thresholds: &OcrQualityThresholds::default(),
                    max_working_bytes: WORKING_BYTES,
                    cancelled: &|| true,
                },
            ));
        }
    }
}
