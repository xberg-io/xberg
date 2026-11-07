use super::document::{
    LineBox, native_layer_decodes_cleanly, ocr_confirms_layer, ocr_page_has_only_plain_payload_with,
    ocr_page_only_segments_text_with, processed_ocr_layout_dimensions, valid_ocr_layout_dimension,
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
/// The share of native words the read must show at their own place. The read judges the native
/// layer only where it is reliable: Tesseract misreads a few percent of the words of a clean
/// render (2.6% on the code-listing page and about 8% on the NIST checklist, which sets in
/// small type and symbols), so a layer that disagrees with the read at more than 15% of its words
/// is more than misreads explain and the read is the better witness. ~keep
const MIN_NATIVE_READ_AGREEMENT: f64 = 0.85;
/// Resolution of the two rasters that tell glyph ink from every other mark. Word boxes are tested
/// in points, so the resolution only has to resolve a glyph; it is not an OCR resolution.
const INK_PROOF_DPI: u32 = 144;

/// Whether the native text paints all the ink that a Tesseract read of the page sits on, for a
/// read whose words do not literally equal the native words. Every iterator word must lie in a
/// native text line, show glyph ink, and hold no mark that is not a glyph, so the read can only
/// be a second reading of ink the native text already holds. A word that differs from the native
/// word at its place is still a reading of that ink; a word over a raster, an outline, a rule or
/// a fill is not provably one.
///
/// A glyph without a Unicode value is content the native text lacks. The layer may carry such
/// markers only when each one sits in a located native word and no read word lies over it: a read
/// that sees something there may hold what the native text lost.
pub(super) fn native_text_paints_all_read_ink(
    pdf: &PdfDocument,
    page_index: usize,
    layer: &str,
    read: &ExtractedDocument,
    limits: InkProofLimits<'_>,
) -> bool {
    use super::super::unmapped_glyphs::{LOST_GLYPH_MARKER, strip_native_fragment};

    let InkProofLimits {
        thresholds,
        max_working_bytes,
        cancelled,
    } = limits;
    let mut layer = layer.to_owned();
    let lost = strip_native_fragment(&mut layer);
    if !native_layer_decodes_cleanly(&layer, thresholds) || !ocr_confirms_layer(&layer, &read.content, thresholds) {
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
    let (Some(media), Ok(lines)) = (upright_media_box(pdf, page_index), pdf.extract_text_lines(page_index)) else {
        return false;
    };
    let Some(read_boxes) = words()
        .map(|word| ocr_word_in_page_points(word, raster, media))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let lost_words: Vec<_> = lines
        .iter()
        .flat_map(|line| &line.words)
        .filter(|word| word.text.contains(LOST_GLYPH_MARKER))
        .collect();
    let located: usize = lost_words
        .iter()
        .map(|word| word.text.matches(LOST_GLYPH_MARKER).count())
        .sum();
    if located != lost
        || lost_glyphs_recovered_by_read(&lost_words, |gap| read_boxes.iter().any(|placed| gap.owns(placed)))
    {
        return false;
    }
    let native_words: Vec<(String, LineBox)> = lines
        .iter()
        .flat_map(|line| &line.words)
        .map(|word| (word.text.clone(), native_box(word.bbox)))
        .collect();
    let Some(placed) = read_on_native_lines(&read_boxes, &lines) else {
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
    let read_words: Vec<(String, LineBox)> = words()
        .map(|w| w.text.clone())
        .zip(read_boxes.iter().copied())
        .collect();
    agreement_share(&native_words, &read_words) >= MIN_NATIVE_READ_AGREEMENT
        && ink_is_glyph_ink(&rasters, media, &placed, &native_words)
}

/// Every read word holds only glyph ink (or sits on blank paper between native words), glyph ink
/// shows in at least one, and every native word with letters or digits shows glyph ink.
fn ink_is_glyph_ink(
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    media: Rect,
    placed: &[(LineBox, bool)],
    native_words: &[(String, LineBox)],
) -> bool {
    let scale = INK_PROOF_DPI as f32 / 72.0;
    let shown: Option<Vec<bool>> = placed
        .iter()
        .map(|(word, over_native_word)| {
            word_ink(rasters, word, media, scale).filter(|shows_glyph| *shows_glyph || !over_native_word)
        })
        .collect();
    shown.is_some_and(|shown| shown.contains(&true))
        && native_words
            .iter()
            .filter(|(text, _)| !alnum_key(text).is_empty())
            .all(|(_, word)| glyph_ink_in(rasters, word, media, scale))
}

fn alnum_key(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The share of native words with letters or digits that the read shows at the same place: the
/// read words over a native word, joined, hold its letters and digits. A native word that the read
/// omits counts against the share.
fn agreement_share(native: &[(String, LineBox)], read: &[(String, LineBox)]) -> f64 {
    let mut total = 0usize;
    let mut agreed = 0usize;
    for (text, place) in native {
        let key = alnum_key(text);
        if key.is_empty() {
            continue;
        }
        total += 1;
        let seen: String = read
            .iter()
            .filter(|(_, at)| place.owns(at))
            .map(|(word, _)| alnum_key(word))
            .collect();
        if seen.contains(&key) {
            agreed += 1;
        }
    }
    if total == 0 { 0.0 } else { agreed as f64 / total as f64 }
}

fn pixel_window(
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    placed: &LineBox,
    media: Rect,
    scale: f32,
) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    let (width, height) = (rasters.width as usize, rasters.height as usize);
    let columns = (placed.left * scale).floor().max(0.0) as usize..((placed.right * scale).ceil() as usize).min(width);
    let rows = ((media.height - placed.bottom) * scale).floor().max(0.0) as usize
        ..(((media.height - placed.top) * scale).ceil() as usize).min(height);
    (columns, rows)
}

/// Whether glyph paints made a visible difference somewhere in the box. A native word with none
/// is not shown on the page: it lies under a fill, is clipped, is invisible, or matches its ground.
fn glyph_ink_in(
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    placed: &LineBox,
    media: Rect,
    scale: f32,
) -> bool {
    let width = rasters.width as usize;
    let (columns, rows) = pixel_window(rasters, placed, media, scale);
    !columns.is_empty()
        && rows.clone().any(|row| {
            let at = row * width;
            rasters.drawn[at + columns.start..at + columns.end]
                .iter()
                .zip(&rasters.without_glyphs[at + columns.start..at + columns.end])
                .any(|(drawn, bare)| drawn.abs_diff(*bare) >= MIN_VISIBLE_CONTRAST)
        })
}

/// The page's media box when the page is unrotated, starts at the origin and is not cropped.
fn upright_media_box(pdf: &PdfDocument, page_index: usize) -> Option<Rect> {
    let info = pdf.get_page_info(page_index).ok()?;
    let media = info.media_box;
    (info.rotation == 0 && media.x == 0.0 && media.y == 0.0 && info.crop_box.is_none_or(|crop| crop == media))
        .then_some(media)
}

/// Each read box with whether it lies over a native word that holds its own glyphs, or `None`
/// when a box lies in no native line. A box over a lost glyph or in the gap between words is not
/// over such a word.
fn read_on_native_lines(
    read_boxes: &[LineBox],
    lines: &[xberg_native_pdf::layout::TextLine],
) -> Option<Vec<(LineBox, bool)>> {
    let line_boxes: Vec<LineBox> = lines.iter().map(|line| native_box(line.bbox)).collect();
    let word_boxes: Vec<LineBox> = lines
        .iter()
        .flat_map(|line| &line.words)
        .filter(|word| !word.text.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER))
        .map(|word| native_box(word.bbox))
        .collect();
    read_boxes
        .iter()
        .map(|placed| {
            line_boxes
                .iter()
                .any(|line| line.owns(placed))
                .then(|| (*placed, word_boxes.iter().any(|native| native.owns(placed))))
        })
        .collect()
}

/// Half a point: font sizes that differ by less print as one size.
const SIZE_STEP_PT: f32 = 0.5;
/// A tenth of a point: glyphs of one code advance by the same width, and a smaller difference is
/// rounding in the box.
const ADVANCE_STEP_PT: f32 = 0.1;

/// Places of the glyphs the native text lost that stand for one glyph code: the same font, the
/// same printed size and, for a word that is only lost glyphs, the same advance per glyph. Glyphs
/// inside a word with letters cannot be told apart by width and share the key of their font and size.
#[derive(Hash, PartialEq, Eq)]
struct LostGlyphKind<'a> {
    font: &'a str,
    size: i32,
    advance: Option<i32>,
}

impl<'a> LostGlyphKind<'a> {
    fn of(word: &'a xberg_native_pdf::layout::Word, markers: usize) -> Self {
        let alone = word
            .text
            .chars()
            .all(|c| c == super::super::unmapped_glyphs::LOST_GLYPH_MARKER);
        Self {
            font: word.dominant_font.as_str(),
            size: (word.avg_font_size / SIZE_STEP_PT).round() as i32,
            advance: alone.then(|| (word.bbox.width / markers as f32 / ADVANCE_STEP_PT).round() as i32),
        }
    }
}

/// Whether the read recovers the glyphs the native text lost. A missing Unicode mapping belongs to
/// a glyph code of one font, so every place that shows the glyph is lost the same way, and a read
/// that really recovers the glyph recovers it wherever it stands. If the read sees something at
/// fewer than half of the places of a kind, the unread places outnumber the read ones, which a
/// recovered glyph does not explain: the read is misreading the ink of a symbol. At half or more
/// the read ones match or outnumber the misses, which misreads of a recovered glyph explain; the
/// tie goes to the read because keeping the native text would then discard characters the read
/// supplied. The kinds are weighed together by the places they cover: the read replaces the native
/// text only when the glyphs it recovers are at least half of all the lost ones, because the rest
/// of the native text is intact and the read misreads some of its words. A kind of few places
/// cannot decide the page against many unrecoverable ones, and the count of lost glyphs is
/// reported with the kept page. ~keep
fn lost_glyphs_recovered_by_read(
    lost_words: &[&xberg_native_pdf::layout::Word],
    is_read_at: impl Fn(&LineBox) -> bool,
) -> bool {
    let mut kinds: std::collections::HashMap<LostGlyphKind<'_>, (usize, usize)> = std::collections::HashMap::new();
    for word in lost_words {
        let markers = word
            .text
            .matches(super::super::unmapped_glyphs::LOST_GLYPH_MARKER)
            .count();
        let (places, read) = kinds.entry(LostGlyphKind::of(word, markers)).or_default();
        *places += markers;
        if is_read_at(&native_box(word.bbox)) {
            *read += markers;
        }
    }
    let total: usize = kinds.values().map(|&(places, _)| places).sum();
    let recovered: usize = kinds
        .values()
        .filter(|&&(places, read)| read * 2 >= places)
        .map(|&(places, _)| places)
        .sum();
    total > 0 && recovered * 2 >= total
}

/// `None` when the word holds a mark that is not a glyph; otherwise whether glyph ink shows in it.
/// A read word in the gap between native words that sits on blank paper reads nothing, so it
/// cannot hide native content; over a native word, blank paper means that word's glyphs are not
/// shown, which the caller refuses. ~keep
fn word_ink(
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    placed: &LineBox,
    media: Rect,
    scale: f32,
) -> Option<bool> {
    let width = rasters.width as usize;
    let (columns, rows) = pixel_window(rasters, placed, media, scale);
    if columns.is_empty() || rows.is_empty() {
        return None;
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
    (spread(&rasters.without_glyphs) < MIN_VISIBLE_CONTRAST).then(|| glyph_ink_in(rasters, placed, media, scale))
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

/// Whether the page's final structure only cuts a certified Tesseract read into blocks.
pub(super) fn only_segments_certified_tesseract_text(doc: &InternalDocument, read: &str) -> bool {
    doc.prebuilt_ocr_elements.is_none() && ocr_page_only_segments_text_with(doc, read, is_hocr_bookkeeping)
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
            pdf_with_lost_glyphs(under_text, over_text, &[])
        }

        /// A page whose text also holds one glyph without a Unicode value at each of `lost`.
        fn pdf_with_lost_glyphs(under_text: &str, over_text: &str, lost: &[(i32, i32, i32)]) -> PdfDocument {
            use lopdf::{Document, Object, Stream, dictionary};

            let prose: String = PROSE
                .iter()
                .enumerate()
                .map(|(at, line)| format!("BT /F1 12 Tf 72 {} Td ({line}) Tj ET\n", 640 - 20 * at))
                .collect();
            let lost: String = lost
                .iter()
                .map(|(x, y, size)| format!("BT /F2 {size} Tf {x} {y} Td <0044> Tj ET\n"))
                .collect();
            let content = format!(
                "{under_text}\nBT /F1 12 Tf 72 700 Td (mov eax, ebx) Tj ET\n\
                 BT /F1 12 Tf 72 680 Td (call puts) Tj ET\n{prose}\n{lost}{over_text}"
            );
            let mut document = Document::with_version("1.5");
            let pages_id = document.new_object_id();
            let font_id = document.add_object(dictionary! {
                "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier",
            });
            let cmap = b"begincmap /CMapName /Probe def /CMapType 2 def \
                1 begincodespacerange <0000> <FFFF> endcodespacerange \
                1 beginbfchar <0041> <0041> endbfchar endcmap";
            let unicode_id = document.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
            let descriptor_id = document.add_object(dictionary! {
                "Type" => "FontDescriptor", "FontName" => "ABCDEF+Probe", "Flags" => 4,
                "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 800.into()],
                "ItalicAngle" => 0, "Ascent" => 800, "Descent" => -200, "CapHeight" => 700, "StemV" => 80,
            });
            let descendant_id = document.add_object(dictionary! {
                "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "ABCDEF+Probe",
                "CIDSystemInfo" => dictionary! {"Registry" => Object::string_literal("Adobe"),
                    "Ordering" => Object::string_literal("Identity"), "Supplement" => 0},
                "FontDescriptor" => descriptor_id, "DW" => 600, "CIDToGIDMap" => "Identity",
            });
            let lost_font_id = document.add_object(dictionary! {
                "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "ABCDEF+Probe",
                "Encoding" => "Identity-H", "DescendantFonts" => vec![descendant_id.into()],
                "ToUnicode" => unicode_id,
            });
            let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
            let page_id = document.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id, "F2" => lost_font_id } },
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
        fn should_own_a_read_word_on_blank_paper_inside_a_native_line() {
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
            assert!(ink_owned(&pdf, LAYER, &read_of(words)));
        }

        #[test]
        fn should_not_own_a_read_when_no_word_shows_glyph_ink() {
            let pdf = pdf_with("");
            let boxes = native_word_boxes(&pdf);
            let first = boxes[0].1;
            let blank = Rect {
                x: first.x,
                y: first.y + 3.0 * first.height,
                width: first.width,
                height: first.height,
            };
            assert!(!ink_owned(
                &pdf,
                "mov eax, ebx",
                &read_of(vec![iterator_word("mov", blank)])
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

        /// A read of `pdf` whose first `misread` words with letters or digits are words the page never shows.
        fn read_misreading(pdf: &PdfDocument, misread: usize) -> (ExtractedDocument, usize) {
            let mut left = misread;
            let mut solid = 0;
            let words = native_word_boxes(pdf)
                .iter()
                .map(|(text, placed)| {
                    if text.chars().any(char::is_alphanumeric) {
                        solid += 1;
                        if left > 0 {
                            left -= 1;
                            return iterator_word("zzzz", *placed);
                        }
                    }
                    iterator_word(text, *placed)
                })
                .collect();
            (read_of(words), solid)
        }

        #[test]
        fn should_own_a_read_that_misreads_up_to_the_agreement_floor() {
            let pdf = pdf_with("");
            let (_, solid) = read_misreading(&pdf, 0);
            let allowed = ((1.0 - MIN_NATIVE_READ_AGREEMENT) * solid as f64).floor() as usize;
            assert!(allowed > 0, "{solid} words leave room for a misread");
            assert!(ink_owned(&pdf, LAYER, &read_misreading(&pdf, allowed).0));
        }

        #[test]
        fn should_not_own_a_read_that_misreads_past_the_agreement_floor() {
            let pdf = pdf_with("");
            let (_, solid) = read_misreading(&pdf, 0);
            let allowed = ((1.0 - MIN_NATIVE_READ_AGREEMENT) * solid as f64).floor() as usize;
            assert!(!ink_owned(&pdf, LAYER, &read_misreading(&pdf, allowed + 1).0));
        }

        #[test]
        fn should_not_own_a_read_that_contradicts_a_third_of_the_native_words() {
            let pdf = pdf_with("");
            let (_, solid) = read_misreading(&pdf, 0);
            assert!(!ink_owned(&pdf, LAYER, &read_misreading(&pdf, solid / 3).0));
        }

        #[test]
        fn should_not_own_a_page_whose_native_word_lies_under_a_fill_the_read_omits() {
            let pdf = pdf_with("1 g 72 696 43 14 re f");
            let covered = |placed: &Rect| placed.x + placed.width <= 72.0 + 43.0;
            let words: Vec<_> = native_word_boxes(&pdf)
                .iter()
                .filter(|(_, placed)| !covered(placed))
                .map(|(text, placed)| iterator_word(text, *placed))
                .collect();
            assert!(words.len() < native_word_boxes(&pdf).len(), "a native word is covered");
            assert!(!ink_owned(&pdf, LAYER, &read_of(words)));
        }

        #[test]
        fn should_not_prefer_a_native_layer_the_read_does_not_confirm() {
            let pdf = pdf_with("");
            let read = read_matching_ink(&pdf);
            assert!(!ink_owned(&pdf, &layer_with_words_wrong(|_| true), &read));
        }

        const LOST_AT: [(i32, i32, i32); 4] = [(300, 740, 12), (300, 720, 12), (300, 700, 12), (300, 680, 12)];

        /// The layer and a read of `pdf` that sees a word over each lost glyph for which `read_if(index)` holds.
        fn lost_glyph_page_where(
            lost: &[(i32, i32, i32)],
            read_if: impl Fn(usize) -> bool,
        ) -> (PdfDocument, String, ExtractedDocument) {
            let pdf = pdf_with_lost_glyphs("", "", lost);
            let mut words = Vec::new();
            let mut seen = 0;
            for (text, placed) in native_word_boxes(&pdf) {
                if text.contains('\u{FFFD}') {
                    let at = lost
                        .iter()
                        .position(|(x, y, size)| {
                            placed.x >= *x as f32 - 1.0
                                && placed.x <= (*x + *size) as f32
                                && placed.y >= *y as f32 - *size as f32
                                && placed.y <= (*y + *size) as f32
                        })
                        .expect("a lost glyph of the page");
                    if read_if(at) {
                        words.push(iterator_word("O", placed));
                    }
                    seen += 1;
                } else {
                    words.push(iterator_word(&text, placed));
                }
            }
            assert_eq!(seen, lost.len());
            let layer = format!("{LAYER}{}", "\n\u{FFFD}".repeat(lost.len()));
            (pdf, layer, read_of(words))
        }

        fn lost_glyph_page(lost: &[(i32, i32, i32)], read_at: &[usize]) -> (PdfDocument, String, ExtractedDocument) {
            lost_glyph_page_where(lost, |at| read_at.contains(&at))
        }

        #[test]
        fn should_keep_native_text_whose_lost_glyphs_the_read_does_not_see() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[]);
            assert!(ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_keep_native_text_when_the_read_sees_few_of_its_lost_glyphs() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[0]);
            assert!(ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_take_the_read_when_it_sees_half_of_the_lost_glyphs() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[0, 2]);
            assert!(!ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_take_the_read_when_it_sees_most_of_the_lost_glyphs() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[0, 1, 3]);
            assert!(!ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_keep_native_text_when_the_read_sees_one_of_three_lost_glyphs() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..3], &[1]);
            assert!(ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_take_the_read_when_it_sees_two_of_three_lost_glyphs() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..3], &[0, 2]);
            assert!(!ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_keep_native_text_when_a_kind_of_one_glyph_is_read_among_many_unread() {
            let lost = [LOST_AT[0], LOST_AT[1], LOST_AT[2], LOST_AT[3], (420, 740, 18)];
            let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at == 4);
            assert!(ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_take_the_read_when_it_recovers_most_lost_glyphs_across_two_kinds() {
            let lost = [(420, 740, 18), LOST_AT[0], LOST_AT[1], LOST_AT[2]];
            let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at != 0);
            assert!(!ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_take_the_read_that_sees_the_only_lost_glyph() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..1], &[0]);
            assert!(!ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_keep_native_text_whose_only_lost_glyph_is_unread() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..1], &[]);
            assert!(ink_owned(&pdf, &layer, &read));
        }

        #[test]
        fn should_not_own_a_layer_with_a_lost_glyph_that_no_native_word_holds() {
            let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[]);
            assert!(!ink_owned(&pdf, &format!("{layer}\n\u{FFFD}"), &read));
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
