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
/// The most the ground may vary under a read word that no glyph and no native word accounts for.
/// One gray level is the rounding of the luminance conversion; more is a mark, and Tesseract
/// binarizes a page, so a faint mark can be read, which makes it content the native text lacks. ~keep
const FLAT_GROUND_SPREAD: u8 = 1;
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
    let InkProofLimits {
        thresholds,
        max_working_bytes,
        cancelled,
    } = limits;
    let mut layer = layer.to_owned();
    let lost = super::super::unmapped_glyphs::strip_native_fragment(&mut layer);
    if !native_layer_decodes_cleanly(&layer, thresholds) || !ocr_confirms_layer(&layer, &read.content, thresholds) {
        return false;
    }
    let Some(elements) = read.ocr_elements.as_deref() else {
        return false;
    };
    let words = || elements.iter().filter(|element| element.level == OcrElementLevel::Word);
    if words().next().is_none()
        || !words().all(is_iterator_word)
        || !content_holds_exactly_the_read_words(&read.content, words().map(|word| word.text.trim()))
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
    let read_words: Vec<(String, LineBox)> = words()
        .map(|w| w.text.clone())
        .zip(read_boxes.iter().copied())
        .collect();
    if !lost_glyphs_leave_native_text(&lines, lost, &read_words) {
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
    // The words are the unit because a lost glyph spoils the word it stands in, and a reader
    // takes the page in by words: with more holes than words, most words are holed, and the
    // intact native text no longer carries the page the read would replace. ~keep
    let solid_native_words = native_words
        .iter()
        .filter(|(text, _)| !alnum_key(text).is_empty())
        .count();
    if lost > solid_native_words {
        return false;
    }
    let Ok(Some(rasters)) = xberg_native_pdf::rendering::render_text_ink_rasters(
        pdf,
        page_index,
        INK_PROOF_DPI,
        max_working_bytes,
        cancelled,
    ) else {
        return false;
    };
    !native_word_glues_separate_cells(&native_words, &read_words)
        && agreement_share(&native_words, &read_words) >= MIN_NATIVE_READ_AGREEMENT
        && ink_is_glyph_ink(&rasters, media, &placed, &read_words, &native_words)
}

/// Whether the read's text is the iterator words and nothing else. A read that Tesseract laid out
/// as a table renders the same words in cell order inside markdown pipes and separator rows, and
/// leaves out the iterator words that are only a ruling mark, so such a read is compared as a
/// multiset of its words with the table markup set aside. The table renderer escapes a cell's `|`
/// as `\|` (`render_table_markdown`), so that escape is undone on the content side. ~keep
fn content_holds_exactly_the_read_words<'a>(content: &str, words: impl Iterator<Item = &'a str> + Clone) -> bool {
    if content.split_whitespace().eq(words.clone()) {
        return true;
    }
    let is_table_markup = |token: &str| token.chars().all(|c| matches!(c, '|' | '-' | '\\'));
    let sorted = |tokens: &mut dyn Iterator<Item = &str>| {
        let mut kept: Vec<String> = tokens
            .filter(|token| !is_table_markup(token))
            .map(str::to_owned)
            .collect();
        kept.sort_unstable();
        kept
    };
    let unescaped = content.replace("\\|", "|");
    content.contains('|') && sorted(&mut unescaped.split_whitespace()) == sorted(&mut words.clone())
}

/// Whether every lost glyph of the layer sits in a located native word and the read does not
/// recover them as text (see [`lost_glyphs_recovered_by_read`]).
fn lost_glyphs_leave_native_text(
    lines: &[xberg_native_pdf::layout::TextLine],
    lost: usize,
    read_words: &[(String, LineBox)],
) -> bool {
    use super::super::unmapped_glyphs::LOST_GLYPH_MARKER;

    let lost_words: Vec<_> = lines
        .iter()
        .flat_map(|line| &line.words)
        .filter(|word| word.text.contains(LOST_GLYPH_MARKER))
        .collect();
    let located: usize = lost_words
        .iter()
        .map(|word| word.text.matches(LOST_GLYPH_MARKER).count())
        .sum();
    located == lost
        && !lost_glyphs_recovered_by_read(&lost_words, |gap| {
            read_words
                .iter()
                .any(|(text, placed)| !alnum_key(text).is_empty() && gap.owns(placed))
        })
}

/// Every read word holds only glyph ink (or sits on blank paper between native words), glyph ink
/// shows in at least one, and every native word with letters, digits or a lost glyph shows glyph ink.
fn ink_is_glyph_ink(
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    media: Rect,
    placed: &[(LineBox, bool)],
    read_words: &[(String, LineBox)],
    native_words: &[(String, LineBox)],
) -> bool {
    debug_assert_eq!(placed.len(), read_words.len());
    let scale = INK_PROOF_DPI as f32 / 72.0;
    let shown: Option<Vec<bool>> = placed
        .iter()
        .zip(read_words)
        .filter(|(_, (text, _))| !is_ruling_bar(text))
        .map(|((word, over_native_word), _)| {
            word_ink(rasters, word, media, scale).filter(|shows_glyph| *shows_glyph || !over_native_word)
        })
        .collect();
    shown.is_some_and(|shown| shown.contains(&true))
        && native_words
            .iter()
            .filter(|(text, _)| {
                !alnum_key(text).is_empty() || text.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER)
            })
            .all(|(_, word)| glyph_ink_in(rasters, word, media, scale))
}

/// A read word that is only vertical bars is how Tesseract reads a table's ruling. It holds no
/// text, so it neither needs glyph ink nor counts against the page being glyph ink. ~keep
fn is_ruling_bar(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c == '|')
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

/// The fewest leader characters that make a run a leader rather than punctuation.
const MIN_LEADER_RUN: usize = 3;

/// Whether `text` is a run of the characters a table of contents or form sets between a label and
/// its value: dots, ellipses, middle dots, underscores and hyphens. A lone `.` or a `|` is not one.
fn is_leader_run(text: &str) -> bool {
    let weight: usize = text
        .chars()
        .map(|c| match c {
            '.' | '·' | '_' | '-' => Some(1),
            '…' => Some(MIN_LEADER_RUN),
            _ => None,
        })
        .sum::<Option<usize>>()
        .unwrap_or(0);
    weight >= MIN_LEADER_RUN
}

/// Whether a native word glues read words that stand a cell apart. The native text joins cells
/// that touch in its glyph stream (`1ThreadOvf_PMC1`) although the page sets them a column apart,
/// so the words the read separates are not words of the native text, and the share cannot credit
/// the token for them: the joined read words contain it, yet a reader of the native text never
/// sees `Ovf_PMC1`. The gap is measured along the line from the furthest right edge of the read
/// words before it. A gap under one line height is a break inside a word, and a leader run read
/// in it (`.....`) closes it; a `|` or a lone `.` does not, since a ruled table reads its rules
/// that way. A native token that holds a leader run is a label and its value joined by a leader
/// whether or not the read kept the dots (`Introduction.........12`), so it never vetoes. Words
/// set sideways stack their read words vertically and are out of scope. ~keep
fn native_word_glues_separate_cells(native: &[(String, LineBox)], read: &[(String, LineBox)]) -> bool {
    native.iter().any(|(text, place)| {
        let height = place.bottom - place.top;
        if height <= 0.0 || has_leader_run(text) {
            return false;
        }
        let mut parts: Vec<(&LineBox, bool, bool)> = read
            .iter()
            .filter(|(word, at)| place.owns(at) && !word.trim().is_empty())
            .map(|(word, at)| (at, !alnum_key(word).is_empty(), is_leader_run(word)))
            .filter(|(_, is_word, is_leader)| *is_word || *is_leader)
            .collect();
        parts.sort_by(|a, b| a.0.left.total_cmp(&b.0.left));
        let mut reach = f32::NEG_INFINITY;
        let (mut word_before, mut gap_after_word) = (false, false);
        for (part, is_word, is_leader) in parts {
            if reach.is_finite() && word_before && !is_leader && part.left - reach > height {
                gap_after_word = true;
            }
            if is_word {
                if gap_after_word {
                    return true;
                }
                word_before = true;
            }
            reach = reach.max(part.right);
        }
        false
    })
}

/// Whether the token holds a run of [`MIN_LEADER_RUN`] or more leader characters in a row.
fn has_leader_run(text: &str) -> bool {
    let mut run = 0;
    for c in text.chars() {
        run = match c {
            '.' | '·' | '_' | '-' => run + 1,
            '…' => run + MIN_LEADER_RUN,
            _ => 0,
        };
        if run >= MIN_LEADER_RUN {
            return true;
        }
    }
    false
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

/// The fewest places of a kind that can show a pattern. One or two places of a font and size say
/// nothing about a glyph code, so such kinds decide a page only together.
const MIN_KIND_PLACES: usize = 3;

/// Whether the read recovers the glyphs the native text lost. A missing Unicode mapping belongs to
/// a glyph code of one font, so every place that shows the glyph is lost the same way, and a read
/// that really recovers the glyph recovers it wherever it stands, as letters or digits. A kind
/// whose places the read sees as text at fewer than half is being misread, not recovered: the
/// unread places outnumber the read ones, which a recovered glyph does not explain. At half or
/// more the misses are ordinary read misses of a recovered glyph; the tie goes to the read
/// because keeping the native text would then discard characters the read supplied. Kinds are
/// judged one by one, never by a tally over the page: a kind of `MIN_KIND_PLACES` or more that the
/// read recovers is text the native layer lacks, and no number of unread symbols elsewhere
/// outweighs it. Kinds of fewer places cannot show a pattern one by one, and one code can split
/// into several of them when its boxes differ in size or advance by a step; they are pooled, and
/// the pool decides the page when it holds `MIN_KIND_PLACES` places or more, or when no larger
/// kind exists. A lone read letter among many unread symbols is a pool of one and decides
/// nothing. ~keep
fn lost_glyphs_recovered_by_read(
    lost_words: &[&xberg_native_pdf::layout::Word],
    is_read_as_text_at: impl Fn(&LineBox) -> bool,
) -> bool {
    let mut kinds: std::collections::HashMap<LostGlyphKind<'_>, (usize, usize)> = std::collections::HashMap::new();
    for word in lost_words {
        let markers = word
            .text
            .matches(super::super::unmapped_glyphs::LOST_GLYPH_MARKER)
            .count();
        let (places, read) = kinds.entry(LostGlyphKind::of(word, markers)).or_default();
        *places += markers;
        if is_read_as_text_at(&native_box(word.bbox)) {
            *read += markers;
        }
    }
    let recovered = |&&(places, read): &&(usize, usize)| read * 2 >= places;
    let (large, small): (Vec<_>, Vec<_>) = kinds.values().partition(|(places, _)| *places >= MIN_KIND_PLACES);
    if large.iter().any(recovered) {
        return true;
    }
    let places: usize = small.iter().map(|(places, _)| places).sum();
    let read: usize = small.iter().map(|(_, read)| read).sum();
    (large.is_empty() || places >= MIN_KIND_PLACES) && places > 0 && read * 2 >= places
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
    let ground = spread(&rasters.without_glyphs);
    let glyph_ink = glyph_ink_in(rasters, placed, media, scale);
    (ground < MIN_VISIBLE_CONTRAST && (glyph_ink || ground <= FLAT_GROUND_SPREAD)).then_some(glyph_ink)
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

/// Whether the page's final structure cuts a certified Tesseract read into blocks and tables
/// without adding anything else (see [`only_segments_certified_tesseract_text`]).
pub(super) fn only_segments_certified_tesseract_text_and_tables(doc: &InternalDocument, read: &str) -> bool {
    doc.prebuilt_ocr_elements.is_none()
        && super::document::ocr_page_only_segments_text_and_tables_with(doc, read, is_hocr_bookkeeping_with_tables)
}

fn is_hocr_bookkeeping_with_tables(key: &str, value: &serde_json::Value) -> bool {
    match key {
        "tables_detected" | "table_count" | "table_rows" | "table_cols" => value.as_u64().is_some(),
        _ => is_hocr_bookkeeping(key, value),
    }
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
#[path = "native_word_coverage_tests.rs"]
mod tests;
