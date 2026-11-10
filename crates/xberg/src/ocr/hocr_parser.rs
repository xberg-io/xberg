//! hOCR → `InternalDocument` parser.
//!
//! Adapted from `html_to_markdown_rs::hocr` property and element parsing.
//!
//! This module parses hOCR HTML produced by Tesseract (and compatible engines)
//! into xberg's `InternalDocument` representation, preserving bounding boxes,
//! confidence scores, and page structure.
//!
//! ## hOCR hierarchy handled
//!
//! ```text
//! ocr_page  →  PageBreak between pages
//!   ocr_carea / ocrx_block
//!     ocr_par  →  InternalElement (OcrText::Block)
//!       ocr_line / ocrx_line  →  line break within paragraph
//!         ocrx_word  →  word text with bbox and confidence
//! ```

use std::collections::HashSet;

use memchr::memchr;

use crate::ocr::table::ExactTableRows;
use crate::ocr::table::tsv_parser::text_without_underscore_marks;
use crate::table_core::HocrWord;
use crate::types::extraction::BoundingBox;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElementLevel};

/// Attribute used to retain the logical hOCR block enclosing an `ocr_par`.
pub(crate) const HOCR_BLOCK_ID_ATTRIBUTE: &str = "hocr_block_id";

#[derive(Debug)]
struct HocrBlockExtent {
    end: usize,
    id: String,
}

/// Parse hOCR HTML into an `InternalDocument` with full spatial and confidence metadata.
///
/// This is the primary entry point. It replaces the older `convert_hocr_to_markdown` path
/// by producing structured [`InternalElement`]s directly, preserving OCR geometry and
/// confidence that the markdown conversion discards.
///
/// # Arguments
///
/// * `hocr_html` — raw hOCR output from Tesseract (or compatible engine).
///
/// # Output mapping
///
/// | hOCR element   | xberg element                             |
/// |---------------|-----------------------------------------------|
/// | `ocr_page`    | `PageBreak` between consecutive pages         |
/// | `ocr_par`     | `OcrText { level: Block }` with union bbox    |
/// | `ocr_line`    | newline separator within a paragraph          |
/// | `ocrx_word`   | word text, bbox, `x_wconf` → `OcrConfidence` |
///
/// Page numbers come from the `ppageno` title property (converted to 1-indexed).
#[cfg(test)]
pub(crate) fn parse_hocr_to_internal_document(hocr_html: &str) -> InternalDocument {
    parse_hocr_to_internal_document_with_dictionary_filter(hocr_html, None)
}

/// Same as [`parse_hocr_to_internal_document`], with an optional per-line
/// dictionary-invalid noise filter (#783). See [`DictionaryLineFilter`] for why this must
/// run here -- before a paragraph's `ocr_line` groups are joined into its `\n`-joined
/// `text` -- rather than against either of that text's two independent downstream
/// consumers.
///
/// Kept as a separate function (rather than adding a parameter to
/// [`parse_hocr_to_internal_document`] directly) so the ~30 existing call sites that only
/// ever want the unfiltered parse -- almost all of them tests -- do not need to change.
#[cfg(test)]
pub(crate) fn parse_hocr_to_internal_document_with_dictionary_filter(
    hocr_html: &str,
    dictionary_filter: Option<&DictionaryLineFilter<'_>>,
) -> InternalDocument {
    parse_hocr_to_internal_document_with_page_offset(hocr_html, dictionary_filter, 1)
}

/// Same as [`parse_hocr_to_internal_document_with_dictionary_filter`], except elements' page
/// numbers are computed as `ppageno + page_offset` instead of always `ppageno + 1`.
///
/// Tesseract numbers every single-image `recognize()` call's hOCR page as `ppageno 0`
/// regardless of which page of the source document that image actually is -- `perform_ocr`
/// (`ocr::processor::execution`) loads and recognizes exactly one image per call, so the hOCR
/// it gets back can never know the true page number on its own. Callers that OCR one page at a
/// time out of a larger document (the PDF OCR route) pass the real 1-indexed page number here,
/// via `TesseractConfig::page_number`, instead of letting every page collapse to `1`.
#[cfg(test)]
pub(crate) fn parse_hocr_to_internal_document_with_page_offset(
    hocr_html: &str,
    dictionary_filter: Option<&DictionaryLineFilter<'_>>,
    page_offset: u32,
) -> InternalDocument {
    parse_hocr_to_internal_document_with_page_offset_and_stats(hocr_html, dictionary_filter, page_offset).document
}

/// Parsed hOCR plus bounded counters needed by the OCR warning pipeline. ~keep
pub(crate) struct HocrParseResult {
    pub document: InternalDocument,
    pub dictionary_filtered_line_count: usize,
    pub retained_word_confidence_stats: RetainedWordConfidenceStats,
}

#[derive(Debug, Clone)]
pub(crate) struct RetainedWordConfidenceStats {
    histogram: [usize; 101],
    count: usize,
    sum: u64,
    low_confidence_count: usize,
}

impl Default for RetainedWordConfidenceStats {
    fn default() -> Self {
        Self {
            histogram: [0; 101],
            count: 0,
            sum: 0,
            low_confidence_count: 0,
        }
    }
}

impl RetainedWordConfidenceStats {
    pub(crate) fn record(&mut self, confidence: f64) {
        if !confidence.is_finite() || !(0.0..=100.0).contains(&confidence) {
            return;
        }
        let confidence = confidence.round() as usize;
        self.histogram[confidence] = self.histogram[confidence].saturating_add(1);
        self.count = self.count.saturating_add(1);
        self.sum = self.sum.saturating_add(confidence as u64);
        if confidence < 50 {
            self.low_confidence_count = self.low_confidence_count.saturating_add(1);
        }
    }

    pub(crate) fn word_count(&self) -> usize {
        self.count
    }

    pub(crate) fn mean(&self) -> Option<u64> {
        (self.count > 0).then(|| self.sum / self.count as u64)
    }

    pub(crate) fn median(&self) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        let upper = self.value_at_rank(self.count / 2)?;
        if self.count.is_multiple_of(2) {
            let lower = self.value_at_rank(self.count / 2 - 1)?;
            Some((lower + upper) / 2)
        } else {
            Some(upper)
        }
    }

    pub(crate) fn p10(&self) -> Option<usize> {
        if self.count == 0 {
            None
        } else {
            self.value_at_rank((self.count - 1) / 10)
        }
    }

    pub(crate) fn low_confidence_word_count(&self) -> usize {
        self.low_confidence_count
    }

    fn value_at_rank(&self, rank: usize) -> Option<usize> {
        let mut cumulative = 0usize;
        for (confidence, count) in self.histogram.iter().enumerate() {
            cumulative = cumulative.saturating_add(*count);
            if rank < cumulative {
                return Some(confidence);
            }
        }
        None
    }
}

/// Parse hOCR and report how many physical lines dictionary filtering removed. ~keep
pub(crate) fn parse_hocr_to_internal_document_with_page_offset_and_stats(
    hocr_html: &str,
    dictionary_filter: Option<&DictionaryLineFilter<'_>>,
    page_offset: u32,
) -> HocrParseResult {
    parse_hocr_document(
        hocr_html,
        page_offset,
        HocrLinePolicy {
            dictionary_filter,
            protected_words: None,
            table_rows: None,
            claimed_words: None,
        },
    )
}

type PositionedWord = ([u32; 4], String);

struct HocrLinePolicy<'a> {
    dictionary_filter: Option<&'a DictionaryLineFilter<'a>>,
    protected_words: Option<&'a HashSet<PositionedWord>>,
    table_rows: Option<(&'a BoundingBox, ExactTableRows)>,
    claimed_words: Option<HashSet<PositionedWord>>,
}

pub(crate) fn parse_hocr_preserving_rejected_table_words(
    hocr_html: &str,
    dictionary_filter: &DictionaryLineFilter<'_>,
    page_offset: u32,
    rejected_regions: &[Vec<HocrWord>],
) -> HocrParseResult {
    let protected_words = rejected_regions
        .iter()
        .flatten()
        .map(|word| {
            (
                [
                    word.left,
                    word.top,
                    word.left.saturating_add(word.width),
                    word.top.saturating_add(word.height),
                ],
                text_without_underscore_marks(&word.text).trim().to_owned(),
            )
        })
        .collect();
    parse_hocr_document(
        hocr_html,
        page_offset,
        HocrLinePolicy {
            dictionary_filter: Some(dictionary_filter),
            protected_words: Some(&protected_words),
            table_rows: None,
            claimed_words: None,
        },
    )
}

/// Remove exact table rows while the parser still has each physical line's word boxes. With
/// `ignore_marks`, rows are compared on their alphanumeric tokens alone (dot-leader rows, #2783).
pub(crate) fn parse_hocr_without_claimed_table_rows(
    hocr_html: &str,
    page_offset: u32,
    bbox: &BoundingBox,
    cells: &[Vec<String>],
    ignore_marks: bool,
) -> InternalDocument {
    let rows = if ignore_marks {
        ExactTableRows::ignoring_marks(cells)
    } else {
        ExactTableRows::new(cells)
    };
    parse_hocr_document(
        hocr_html,
        page_offset,
        HocrLinePolicy {
            dictionary_filter: None,
            protected_words: None,
            table_rows: Some((bbox, rows)),
            claimed_words: None,
        },
    )
    .document
}

pub(crate) fn parse_hocr_without_proved_table_fragments(
    hocr_html: &str,
    page_offset: u32,
    words: Vec<&HocrWord>,
) -> InternalDocument {
    parse_hocr_document(hocr_html, page_offset, proved_fragment_policy(words)).document
}

#[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
pub(crate) fn hocr_matches_all_proved_table_fragments(
    hocr_html: &str,
    page_offset: u32,
    words: Vec<&HocrWord>,
) -> bool {
    let count = words.len();
    let mut policy = proved_fragment_policy(words);
    if count == 0 || policy.claimed_words.as_ref().is_none_or(|keys| keys.len() != count) {
        return false;
    }
    let result = parse_hocr_document_with_policy(hocr_html, page_offset, &mut policy);
    result.document.elements.is_empty() && policy.claimed_words.is_some_and(|keys| keys.is_empty())
}

fn proved_fragment_policy(words: Vec<&HocrWord>) -> HocrLinePolicy<'static> {
    let claimed_words = words
        .into_iter()
        .map(|word| {
            (
                [word.left, word.top, word.left + word.width, word.top + word.height],
                word.text.clone(),
            )
        })
        .collect();
    HocrLinePolicy {
        dictionary_filter: None,
        protected_words: None,
        table_rows: None,
        claimed_words: Some(claimed_words),
    }
}

fn parse_hocr_document(hocr_html: &str, page_offset: u32, mut line_policy: HocrLinePolicy<'_>) -> HocrParseResult {
    parse_hocr_document_with_policy(hocr_html, page_offset, &mut line_policy)
}

fn parse_hocr_document_with_policy(
    hocr_html: &str,
    page_offset: u32,
    line_policy: &mut HocrLinePolicy<'_>,
) -> HocrParseResult {
    let mut doc = InternalDocument::new("ocr");
    doc.mime_type = "application/x-hocr".to_string();

    let mut element_index: u32 = 0;
    let mut last_page: Option<u32> = None;
    let mut dictionary_filtered_line_count = 0usize;
    let mut retained_word_confidence_stats = RetainedWordConfidenceStats::default();

    let bytes = hocr_html.as_bytes();
    let mut pos = 0;
    let mut block_extents = Vec::<HocrBlockExtent>::new();

    while pos < bytes.len() {
        let Some(tag_start) = memchr(b'<', &bytes[pos..]).map(|i| pos + i) else {
            break;
        };
        let Some(tag_end) = memchr(b'>', &bytes[tag_start..]).map(|i| tag_start + i) else {
            break;
        };
        let tag_content = &hocr_html[tag_start + 1..tag_end];
        pos = tag_end + 1;
        block_extents.retain(|extent| tag_start < extent.end);

        if tag_content.starts_with('/') || tag_content.ends_with('/') {
            continue;
        }

        if has_class(tag_content, "ocr_page") {
            let title = extract_title_attr(tag_content);
            let props = parse_title_properties(&title);
            let page_number = props.ppageno.map(|p| p + page_offset);

            if let Some(prev) = last_page
                && page_number != Some(prev)
            {
                let pb = InternalElement::text(ElementKind::PageBreak, "", 0).with_index(element_index);
                element_index += 1;
                doc.push_element(pb);
            }
            last_page = page_number;
            continue;
        }

        if has_class(tag_content, "ocr_carea") || has_class(tag_content, "ocrx_block") {
            let tag_name = tag_content
                .split_whitespace()
                .next()
                .unwrap_or("div")
                .to_ascii_lowercase();
            let end = skip_to_matching_close(hocr_html, pos, &tag_name);
            let id = extract_attribute(tag_content, "id")
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| format!("hocr-block-{tag_start}-{end}"));
            block_extents.push(HocrBlockExtent { end, id });
            continue;
        }

        if is_paragraph_tag(tag_content) {
            let par_tag_name = tag_content
                .split_whitespace()
                .next()
                .unwrap_or("p")
                .to_ascii_lowercase();
            let (paragraph, end_pos, filtered_lines) = parse_paragraph(
                hocr_html,
                pos,
                last_page.unwrap_or(page_offset),
                element_index,
                &par_tag_name,
                line_policy,
                &mut retained_word_confidence_stats,
            );
            pos = end_pos;
            dictionary_filtered_line_count = dictionary_filtered_line_count.saturating_add(filtered_lines);

            if let Some(mut elem) = paragraph {
                if let Some(block) = block_extents.last() {
                    elem.attributes
                        .get_or_insert_with(Default::default)
                        .insert(HOCR_BLOCK_ID_ATTRIBUTE.to_string(), block.id.clone());
                }
                element_index += 1;
                doc.push_element(elem);
            }
        }
    }

    tracing::debug!(
        input_bytes = hocr_html.len(),
        elements = doc.elements.len(),
        total_text_chars = doc.elements.iter().map(|e| e.text.len()).sum::<usize>(),
        "hOCR parse complete"
    );

    HocrParseResult {
        document: doc,
        dictionary_filtered_line_count,
        retained_word_confidence_stats,
    }
}

/// Parsed properties from an hOCR `title` attribute.
#[derive(Debug, Default)]
struct HocrProperties {
    /// Bounding box: (x1, y1, x2, y2).
    bbox: Option<(u32, u32, u32, u32)>,
    /// Word confidence 0–100.
    x_wconf: Option<f64>,
    /// Physical page number (0-indexed from Tesseract).
    ppageno: Option<u32>,
    /// Text rotation angle.
    textangle: Option<f64>,
    /// Baseline (slope, constant).
    baseline: Option<(f64, i32)>,
    /// Font name.
    x_font: Option<String>,
    /// Font size in points.
    x_fsize: Option<u32>,
    /// Whether the word is rendered in a bold font, from the word's `x_bold`
    /// hOCR property. Only present on `ocrx_word` titles when Tesseract's
    /// `hocr_font_info` variable is enabled (`ocr/processor/config.rs`).
    x_bold: bool,
    /// Whether the word is rendered in an italic font, from the word's
    /// `x_italic` hOCR property. Same availability as `x_bold`.
    x_italic: bool,
    /// x-height in pixels — the height of the line's lowercase letters
    /// excluding ascenders/descenders. Emitted by Tesseract on `ocr_line`/
    /// `ocrx_line` titles, not on individual words. A better heading signal
    /// than raw bbox height because it is insensitive to how many ascenders
    /// or descenders happen to appear in a given line.
    x_size: Option<f64>,
    /// Ascender height in pixels, from the line's `x_ascenders` property.
    x_ascenders: Option<f64>,
    /// Descender height in pixels, from the line's `x_descenders` property.
    x_descenders: Option<f64>,
}

/// Parse all properties from an hOCR title attribute string.
///
/// Handles the semicolon-separated `key value ...` format produced by Tesseract:
///
/// ```text
/// bbox 100 50 200 150; x_wconf 95; ppageno 0
/// ```
fn parse_title_properties(title: &str) -> HocrProperties {
    let mut props = HocrProperties::default();

    for part in title.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let mut tokens = part.split_whitespace();
        let Some(key) = tokens.next() else {
            continue;
        };

        match key {
            "bbox" => {
                let coords: Vec<u32> = tokens.filter_map(|s| s.parse().ok()).collect();
                if coords.len() == 4 {
                    props.bbox = Some((coords[0], coords[1], coords[2], coords[3]));
                }
            }
            "x_wconf" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<f64>().ok()) {
                    props.x_wconf = Some(val);
                }
            }
            "ppageno" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<u32>().ok()) {
                    props.ppageno = Some(val);
                }
            }
            "textangle" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<f64>().ok()) {
                    props.textangle = Some(val);
                }
            }
            "baseline" => {
                let slope = tokens.next().and_then(|s| s.parse::<f64>().ok());
                let constant = tokens.next().and_then(|s| s.parse::<i32>().ok());
                if let (Some(s), Some(c)) = (slope, constant) {
                    props.baseline = Some((s, c));
                }
            }
            "x_font" => {
                props.x_font = parse_quoted_value(part);
            }
            "x_fsize" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<u32>().ok()) {
                    props.x_fsize = Some(val);
                }
            }
            "x_bold" => {
                props.x_bold = true;
            }
            "x_italic" => {
                props.x_italic = true;
            }
            "x_size" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<f64>().ok()) {
                    props.x_size = Some(val);
                }
            }
            "x_ascenders" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<f64>().ok()) {
                    props.x_ascenders = Some(val);
                }
            }
            "x_descenders" => {
                if let Some(val) = tokens.next().and_then(|s| s.parse::<f64>().ok()) {
                    props.x_descenders = Some(val);
                }
            }
            _ => {}
        }
    }

    props
}

/// Extract a quoted string value from a property part like `x_font "Arial"`.
fn parse_quoted_value(part: &str) -> Option<String> {
    let start = part.find('"')?;
    let end = part[start + 1..].find('"')?;
    Some(part[start + 1..start + 1 + end].to_string())
}

/// A word extracted from hOCR with its metadata.
struct HocrWordInfo {
    text: String,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    confidence: Option<f64>,
    /// Font size in points, from the word's `x_fsize` hOCR property.
    font_size: Option<u32>,
    /// Text rotation angle in degrees, from the word's `textangle` hOCR property.
    text_angle: Option<f64>,
    /// Font family name, from the word's `x_font` hOCR property.
    font_name: Option<String>,
    /// Whether the word is bold, from the word's `x_bold` hOCR property.
    is_bold: bool,
    /// Whether the word is italic, from the word's `x_italic` hOCR property.
    is_italic: bool,
}

/// Per-`ocr_line`/`ocrx_line` metadata parsed from that tag's own `title`
/// attribute, plus the words it contains.
///
/// Kept as one entry per physical hOCR line (rather than folded immediately
/// into a paragraph-wide average) so a downstream consumer can recover a
/// line-level font size — the paragraph mean alone hides variation between,
/// say, a heading's first line and a wrapped continuation line at body size.
#[derive(Default)]
struct HocrLineInfo {
    words: Vec<HocrWordInfo>,
    /// x-height in pixels (`x_size`) — see [`HocrProperties::x_size`].
    x_size: Option<f64>,
    /// Ascender height in pixels (`x_ascenders`).
    x_ascenders: Option<f64>,
    /// Descender height in pixels (`x_descenders`).
    x_descenders: Option<f64>,
    /// Baseline (slope, constant), from the line's `baseline` property.
    baseline: Option<(f64, i32)>,
}

/// Per-line dictionary-invalid noise filter, threaded through hOCR parsing (#783).
///
/// Applied while a paragraph's `ocr_line` groups are still separate physical lines —
/// before their words are joined into the paragraph's `\n`-joined `text` — so every
/// consumer of that text sees the same, already-filtered lines. That matters because
/// there are two such consumers built from the same `InternalElement.text`:
/// `flatten_hocr_elements_to_text` (feeding the flat OCR page string) and
/// `pdf::structure::adapters::ocr_doc_to_paragraphs` / `ocr_doc_to_layout_paragraphs`
/// (feeding the rendered document's paragraphs). Filtering later, against either
/// rendering independently, risks the two silently drifting apart: a prior attempt at
/// this fix (reverted as `29738a1f29`) stripped noise lines only from the flat text
/// string, which never changed the rendered document for any page that also produced
/// structured paragraphs — exactly the elevations-page case this filter targets.
pub(crate) struct DictionaryLineFilter<'a> {
    /// Dictionary membership test, e.g. `TesseractAPI::is_valid_word`. `Some(true)` =
    /// valid, `Some(false)` = invalid, `None` = the lookup itself failed or was
    /// unavailable (never counted as evidence either way).
    pub is_valid_word: &'a dyn Fn(&str) -> Option<bool>,
    /// A line is dropped when its dictionary-checkable words' invalid fraction is
    /// STRICTLY GREATER than this. See [`DEFAULT_DICT_INVALID_LINE_RATIO`] for how the
    /// default value was derived.
    pub max_invalid_ratio: f64,
}

/// Minimum letters a word must have before a dictionary lookup on it is meaningful.
/// Mirrors `ocr::processor::execution::MIN_WORD_LEN_FOR_DICT_CHECK` — both filter the
/// same class of noise (an OCR fragment too short for the dictionary to judge either
/// way), kept as a separate constant here rather than a shared import so this module
/// does not need `ocr::processor::execution` to be `pub(crate)`.
const MIN_WORD_LEN_FOR_DICT_CHECK: usize = 3;

/// Minimum dictionary-checkable words a single hOCR line must contain before
/// [`is_dictionary_noise_line`] scores it at all.
///
/// A physical line is short (a title-block label, a heading), so a high floor would
/// silence this signal for nearly every line on a drawing page. Two independently
/// checkable words distinguish "every word on this line is nonsense" from "one unusual
/// term stands alone on this line" — the latter is exactly the shape of a real proper
/// noun or technical term (a plant genus, a part number) that must not be flagged from a
/// single data point. The blast radius of a wrong per-line call is only that one line,
/// not the whole page, which is what makes a lower floor than a page-level check safe.
pub(crate) const MIN_DICT_CANDIDATES_FOR_LINE: usize = 2;

/// Default [`DictionaryLineFilter::max_invalid_ratio`] (#783).
///
/// Not a config field: [`OcrQualityThresholds`](crate::core::config::OcrQualityThresholds)
/// is part of xberg's alef-generated multi-language binding surface, and every field on it
/// is regenerated into ~15 language bindings, so adding one requires a full `alef
/// generate` pass this fix does not perform. This constant is the internal default until
/// that threading is done deliberately, as its own change with its own binding regen.
///
/// `0.6`, derived directly from two measured examples (2026-08-22), not picked as a round
/// number:
/// - The motivating noise line, "OWATS DNDEVET OPMENT", scores 2 invalid of 3
///   dictionary-checkable candidates (0.667) even though Tesseract's DAWG lookup itself
///   falsely reports "OPMENT" as a valid word -- counting that false positive as valid
///   still leaves the line above 0.6.
/// - The plant-list guard line, "Ligustrum, Photinia, Azalea, Indian Hawthorne", scores 2
///   invalid of 5 (0.4) and must survive untouched.
///
/// 0.6 sits roughly the same distance below the first number as above the second, and
/// matches the existing `max_fragmented_word_ratio` convention in
/// `OcrQualityThresholds`. Unlike that struct's page-level
/// `max_ocr_output_dict_invalid_word_ratio` (disabled by default at `1.01` pending a
/// corpus-wide calibration), this is enabled from the start: the blast radius of a wrong
/// call here is exactly one line, never a whole page, so the acceptable cost of a false
/// positive is far lower.
pub(crate) const DEFAULT_DICT_INVALID_LINE_RATIO: f64 = 0.6;

/// Whether `line`'s dictionary-checkable words are, on balance, not real words.
///
/// Returns `false` (never noise) for a line with fewer than
/// [`MIN_DICT_CANDIDATES_FOR_LINE`] checkable words — see that constant's doc comment.
fn is_dictionary_noise_line(line: &HocrLineInfo, filter: &DictionaryLineFilter<'_>) -> bool {
    let mut candidates = 0usize;
    let mut invalid = 0usize;
    for word in &line.words {
        let text = word.text.trim();
        if text.chars().count() < MIN_WORD_LEN_FOR_DICT_CHECK || !text.chars().all(|c| c.is_alphabetic()) {
            continue;
        }
        match (filter.is_valid_word)(text) {
            Some(true) => candidates += 1,
            Some(false) => {
                candidates += 1;
                invalid += 1;
            }
            None => {}
        }
    }
    if candidates < MIN_DICT_CANDIDATES_FOR_LINE {
        return false;
    }
    (invalid as f64 / candidates as f64) > filter.max_invalid_ratio
}

fn line_matches_rejected_table_words(line: &HocrLineInfo, protected_words: &HashSet<PositionedWord>) -> bool {
    line.words.iter().all(|word| {
        word.x0 < word.x1
            && word.y0 < word.y1
            && protected_words.contains(&([word.x0, word.y0, word.x1, word.y1], word.text.clone()))
    })
}

/// Attribute key holding the paragraph's average word font size (points, as a
/// decimal string). Consumed by markdown assembly to promote large-font
/// paragraphs to headings (#185).
pub(crate) const HOCR_FONT_SIZE_ATTRIBUTE: &str = "x_fsize";

/// Attribute key holding the paragraph's average word text-rotation angle in
/// degrees (as a decimal string), when any word reported a non-zero angle.
pub(crate) const HOCR_TEXT_ANGLE_ATTRIBUTE: &str = "textangle";

/// Attribute key holding the paragraph's average line x-height in pixels (as
/// a decimal string), averaged over lines that reported an `x_size` on their
/// `ocr_line`/`ocrx_line` title. x-height is a better heading signal than raw
/// bbox height because it is insensitive to ascender/descender mix.
pub(crate) const HOCR_X_HEIGHT_ATTRIBUTE: &str = "x_size";

/// Attribute key holding the paragraph's average line ascender height in
/// pixels (as a decimal string).
pub(crate) const HOCR_X_ASCENDERS_ATTRIBUTE: &str = "x_ascenders";

/// Attribute key holding the paragraph's average line descender height in
/// pixels (as a decimal string).
pub(crate) const HOCR_X_DESCENDERS_ATTRIBUTE: &str = "x_descenders";

/// Attribute key holding the paragraph's average line baseline slope (as a
/// decimal string), averaged over lines that reported a `baseline` on their
/// `ocr_line`/`ocrx_line` title.
pub(crate) const HOCR_BASELINE_SLOPE_ATTRIBUTE: &str = "baseline_slope";

/// Attribute key holding the paragraph's average line baseline constant
/// (pixels, as a decimal string).
pub(crate) const HOCR_BASELINE_CONST_ATTRIBUTE: &str = "baseline_const";

/// Attribute key holding one average word font size (points) per physical
/// text line, comma-separated in the same order as the `\n`-separated lines
/// of the element's `text`. A line with no word reporting `x_fsize` is
/// rendered as an empty field so field position still lines up with `text`.
/// Lets a downstream consumer compute a line-level (not just paragraph-mean)
/// font size without carrying every word's bounding box.
pub(crate) const HOCR_LINE_FONT_SIZES_ATTRIBUTE: &str = "line_font_sizes";

/// Attribute key holding one line x-height (pixels, from `x_size`) per
/// physical text line, comma-separated in the same order as the
/// `\n`-separated lines of the element's `text`, with the same empty-field
/// alignment rule as [`HOCR_LINE_FONT_SIZES_ATTRIBUTE`].
pub(crate) const HOCR_LINE_X_HEIGHTS_ATTRIBUTE: &str = "line_x_heights";

/// Attribute key holding the fraction (0.0-1.0, as a decimal string) of the
/// paragraph's words that Tesseract reported as bold via `x_bold`. Only
/// populated on `ocrx_word` titles when `hocr_font_info` is enabled
/// (`ocr/processor/config.rs`). Boldness is an independent heading cue from
/// font size, restoring a signal `from_ocr_elements` consumed before it was
/// deleted as collateral of an unrelated refactor (commit `22161b0d1cc`).
pub(crate) const HOCR_BOLD_FRACTION_ATTRIBUTE: &str = "x_bold_fraction";

/// Attribute key holding the fraction (0.0-1.0, as a decimal string) of the
/// paragraph's words that Tesseract reported as italic via `x_italic`. Same
/// availability and provenance as [`HOCR_BOLD_FRACTION_ATTRIBUTE`].
pub(crate) const HOCR_ITALIC_FRACTION_ATTRIBUTE: &str = "x_italic_fraction";

/// Attribute key holding the most common font family name (from `x_font`)
/// among the paragraph's words, when at least one word reported one.
pub(crate) const HOCR_FONT_NAME_ATTRIBUTE: &str = "x_font";

/// Bold/italic fraction and dominant font name aggregated across a
/// paragraph's words.
struct WordStyleAggregate {
    bold_fraction: f64,
    italic_fraction: f64,
    dominant_font_name: Option<String>,
}

/// Aggregate the `x_bold`/`x_italic`/`x_font` hOCR word properties into
/// paragraph-level signals. `words` must be non-empty.
fn aggregate_word_style(words: &[&HocrWordInfo]) -> WordStyleAggregate {
    let word_count = words.len() as f64;
    let bold_count = words.iter().filter(|w| w.is_bold).count() as f64;
    let italic_count = words.iter().filter(|w| w.is_italic).count() as f64;

    let mut font_name_counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for word in words {
        if let Some(ref name) = word.font_name {
            *font_name_counts.entry(name.as_str()).or_insert(0) += 1;
        }
    }
    let dominant_font_name = font_name_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(name, _)| name.to_string());

    WordStyleAggregate {
        bold_fraction: bold_count / word_count,
        italic_fraction: italic_count / word_count,
        dominant_font_name,
    }
}

/// Render one optional numeric value per line as a comma-joined string,
/// preserving line position (a missing value becomes an empty field) so a
/// downstream consumer can zip the result back up against `text.split('\n')`.
fn join_per_line_values(values: &[Option<f64>]) -> String {
    values
        .iter()
        .map(|value| value.map(format_decimal).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(",")
}

/// Format a float without a trailing `.0` for whole numbers, matching the
/// existing paragraph-average attribute formatting (`f64::to_string`).
fn format_decimal(value: f64) -> String {
    value.to_string()
}

/// Parse a single `<p class="ocr_par">` (or `<span class="ocr_par">`) and all nested
/// content up to the matching closing tag.
///
/// `par_tag` is the lowercase tag name of the paragraph element (e.g. "p", "span", "div").
/// Depth tracking uses ONLY matching tag names to find the paragraph's closing tag.
/// This prevents inner elements (lines, words, formatting) from interfering with
/// the paragraph boundary detection — even if their subtrees are malformed.
///
/// Returns the constructed element (if any words were found) and the byte position
/// after the closing tag.
fn parse_paragraph(
    html: &str,
    start: usize,
    page: u32,
    element_index: u32,
    par_tag: &str,
    line_policy: &mut HocrLinePolicy<'_>,
    retained_word_confidence_stats: &mut RetainedWordConfidenceStats,
) -> (Option<InternalElement>, usize, usize) {
    let bytes = html.as_bytes();
    let mut pos = start;

    let mut lines: Vec<HocrLineInfo> = Vec::new();
    let mut current_line = HocrLineInfo::default();
    let mut in_line = false;

    let mut depth: u32 = 1;

    while pos < bytes.len() {
        let Some(tag_start) = memchr(b'<', &bytes[pos..]).map(|i| pos + i) else {
            break;
        };
        let Some(tag_end) = memchr(b'>', &bytes[tag_start..]).map(|i| tag_start + i) else {
            break;
        };
        let tag_content = &html[tag_start + 1..tag_end];
        pos = tag_end + 1;

        if let Some(stripped) = tag_content.strip_prefix('/') {
            let closing_name = stripped.trim().to_ascii_lowercase();
            if closing_name == par_tag {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if !current_line.words.is_empty() {
                        lines.push(std::mem::take(&mut current_line));
                    }
                    break;
                }
            }
            continue;
        }

        if tag_content.ends_with('/') {
            continue;
        }

        let tag_name = tag_content.split_whitespace().next().unwrap_or("").to_ascii_lowercase();

        if has_class(tag_content, "ocr_line") || has_class(tag_content, "ocrx_line") {
            if in_line && !current_line.words.is_empty() {
                lines.push(std::mem::take(&mut current_line));
            }
            in_line = true;
            let title = extract_title_attr(tag_content);
            let props = parse_title_properties(&title);
            current_line.x_size = props.x_size;
            current_line.x_ascenders = props.x_ascenders;
            current_line.x_descenders = props.x_descenders;
            current_line.baseline = props.baseline;
            if tag_name == par_tag {
                depth += 1;
            }
            continue;
        }

        if has_class(tag_content, "ocrx_word") {
            let title = extract_title_attr(tag_content);
            let props = parse_title_properties(&title);

            let word_text = extract_inner_text(html, pos);
            let decoded = decode_html_entities(&word_text);
            let normalized = text_without_underscore_marks(decoded.trim());
            let trimmed = normalized.trim();

            pos = skip_to_matching_close(html, pos, &tag_name);

            if !trimmed.is_empty() {
                let (x0, y0, x1, y1) = props.bbox.unwrap_or((0, 0, 0, 0));
                current_line.words.push(HocrWordInfo {
                    text: trimmed.to_string(),
                    x0,
                    y0,
                    x1,
                    y1,
                    confidence: props.x_wconf,
                    font_size: props.x_fsize,
                    text_angle: props.textangle,
                    font_name: props.x_font,
                    is_bold: props.x_bold,
                    is_italic: props.x_italic,
                });
            }
            continue;
        }

        if tag_name == par_tag {
            depth += 1;
        }
    }

    let mut removed_line_count = 0usize;
    if let Some(filter) = line_policy.dictionary_filter {
        let lines_before = lines.len();
        lines.retain(|line| {
            !is_dictionary_noise_line(line, filter)
                || line_policy
                    .protected_words
                    .is_some_and(|words| line_matches_rejected_table_words(line, words))
        });
        removed_line_count = lines_before - lines.len();
        if removed_line_count > 0 {
            tracing::debug!(
                page,
                removed_line_count,
                max_invalid_ratio = filter.max_invalid_ratio,
                "removed OCR line(s) whose dictionary-checkable words are mostly not real words"
            );
        }
    }

    if let Some((bbox, rows)) = line_policy.table_rows.as_mut() {
        lines.retain(|line| {
            !line_is_inside_table(line, bbox)
                || !rows.claim(line.words.iter().flat_map(|word| word.text.split_whitespace()))
        });
    }

    if let Some(words) = line_policy.claimed_words.as_mut() {
        lines.retain(|line| !claim_proved_table_fragment(line, words));
    }

    let all_words: Vec<&HocrWordInfo> = lines.iter().flat_map(|l| l.words.iter()).collect();
    if all_words.is_empty() {
        return (None, pos, removed_line_count);
    }

    let style = aggregate_word_style(&all_words);

    let text: String = lines
        .iter()
        .map(|line| line.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");

    let mut min_x0 = u32::MAX;
    let mut min_y0 = u32::MAX;
    let mut max_x1 = 0u32;
    let mut max_y1 = 0u32;
    let mut conf_sum = 0.0f64;
    let mut conf_count = 0u32;
    let mut font_size_sum = 0u32;
    let mut font_size_count = 0u32;
    let mut angle_sum = 0.0f64;
    let mut angle_count = 0u32;

    for word in &all_words {
        if word.x1 > 0 || word.y1 > 0 {
            min_x0 = min_x0.min(word.x0);
            min_y0 = min_y0.min(word.y0);
            max_x1 = max_x1.max(word.x1);
            max_y1 = max_y1.max(word.y1);
        }
        if let Some(c) = word.confidence {
            conf_sum += c;
            conf_count += 1;
            retained_word_confidence_stats.record(c);
        }
        if let Some(fs) = word.font_size {
            font_size_sum += fs;
            font_size_count += 1;
        }
        if let Some(angle) = word.text_angle {
            angle_sum += angle;
            angle_count += 1;
        }
    }

    let mut x_size_sum = 0.0f64;
    let mut x_size_count = 0u32;
    let mut x_ascenders_sum = 0.0f64;
    let mut x_ascenders_count = 0u32;
    let mut x_descenders_sum = 0.0f64;
    let mut x_descenders_count = 0u32;
    let mut baseline_slope_sum = 0.0f64;
    let mut baseline_const_sum = 0.0f64;
    let mut baseline_count = 0u32;

    for line in &lines {
        if let Some(x_size) = line.x_size {
            x_size_sum += x_size;
            x_size_count += 1;
        }
        if let Some(ascenders) = line.x_ascenders {
            x_ascenders_sum += ascenders;
            x_ascenders_count += 1;
        }
        if let Some(descenders) = line.x_descenders {
            x_descenders_sum += descenders;
            x_descenders_count += 1;
        }
        if let Some((slope, constant)) = line.baseline {
            baseline_slope_sum += slope;
            baseline_const_sum += f64::from(constant);
            baseline_count += 1;
        }
    }

    // Per-line font size / x-height, aligned to the `\n`-separated lines of
    // `text` above, so a downstream consumer can recover line-level detail
    // instead of only the paragraph mean (#667, #669).
    let line_font_sizes: Vec<Option<f64>> = lines
        .iter()
        .map(|line| {
            let sizes: Vec<f64> = line.words.iter().filter_map(|w| w.font_size).map(f64::from).collect();
            if sizes.is_empty() {
                None
            } else {
                Some(sizes.iter().sum::<f64>() / sizes.len() as f64)
            }
        })
        .collect();
    let line_x_heights: Vec<Option<f64>> = lines.iter().map(|line| line.x_size).collect();
    let any_line_font_size = line_font_sizes.iter().any(Option::is_some);
    let any_line_x_height = line_x_heights.iter().any(Option::is_some);

    let has_valid_bbox = max_x1 > 0 || max_y1 > 0;

    let bbox = if has_valid_bbox {
        Some(BoundingBox {
            x0: min_x0 as f64,
            y0: min_y0 as f64,
            x1: max_x1 as f64,
            y1: max_y1 as f64,
        })
    } else {
        None
    };

    let ocr_geometry = if has_valid_bbox {
        Some(OcrBoundingGeometry::Rectangle {
            left: min_x0,
            top: min_y0,
            width: max_x1.saturating_sub(min_x0),
            height: max_y1.saturating_sub(min_y0),
        })
    } else {
        None
    };

    let ocr_confidence = if conf_count > 0 {
        #[cfg(feature = "ocr")]
        {
            Some(OcrConfidence::from_tesseract(conf_sum / conf_count as f64))
        }
        #[cfg(not(feature = "ocr"))]
        {
            Some(OcrConfidence {
                recognition: (conf_sum / conf_count as f64) / 100.0,
                detection: None,
            })
        }
    } else {
        None
    };

    let kind = ElementKind::OcrText {
        level: OcrElementLevel::Block,
    };

    let mut elem = InternalElement::text(kind, text, 0)
        .with_page(page)
        .with_index(element_index);

    elem.bbox = bbox;
    elem.ocr_geometry = ocr_geometry;
    elem.ocr_confidence = ocr_confidence;

    if font_size_count > 0 {
        let avg_font_size = font_size_sum as f64 / font_size_count as f64;
        elem.attributes
            .get_or_insert_with(Default::default)
            .insert(HOCR_FONT_SIZE_ATTRIBUTE.to_string(), avg_font_size.to_string());
    }
    if angle_count > 0 {
        let avg_angle = angle_sum / angle_count as f64;
        if avg_angle.abs() > 0.0 {
            elem.attributes
                .get_or_insert_with(Default::default)
                .insert(HOCR_TEXT_ANGLE_ATTRIBUTE.to_string(), avg_angle.to_string());
        }
    }
    if x_size_count > 0 {
        let avg_x_size = x_size_sum / f64::from(x_size_count);
        elem.attributes
            .get_or_insert_with(Default::default)
            .insert(HOCR_X_HEIGHT_ATTRIBUTE.to_string(), avg_x_size.to_string());
    }
    if x_ascenders_count > 0 {
        let avg_ascenders = x_ascenders_sum / f64::from(x_ascenders_count);
        elem.attributes
            .get_or_insert_with(Default::default)
            .insert(HOCR_X_ASCENDERS_ATTRIBUTE.to_string(), avg_ascenders.to_string());
    }
    if x_descenders_count > 0 {
        let avg_descenders = x_descenders_sum / f64::from(x_descenders_count);
        elem.attributes
            .get_or_insert_with(Default::default)
            .insert(HOCR_X_DESCENDERS_ATTRIBUTE.to_string(), avg_descenders.to_string());
    }
    if baseline_count > 0 {
        let avg_slope = baseline_slope_sum / f64::from(baseline_count);
        let avg_const = baseline_const_sum / f64::from(baseline_count);
        let attrs = elem.attributes.get_or_insert_with(Default::default);
        attrs.insert(HOCR_BASELINE_SLOPE_ATTRIBUTE.to_string(), avg_slope.to_string());
        attrs.insert(HOCR_BASELINE_CONST_ATTRIBUTE.to_string(), avg_const.to_string());
    }
    if any_line_font_size {
        elem.attributes.get_or_insert_with(Default::default).insert(
            HOCR_LINE_FONT_SIZES_ATTRIBUTE.to_string(),
            join_per_line_values(&line_font_sizes),
        );
    }
    if any_line_x_height {
        elem.attributes.get_or_insert_with(Default::default).insert(
            HOCR_LINE_X_HEIGHTS_ATTRIBUTE.to_string(),
            join_per_line_values(&line_x_heights),
        );
    }
    {
        let attrs = elem.attributes.get_or_insert_with(Default::default);
        attrs.insert(
            HOCR_BOLD_FRACTION_ATTRIBUTE.to_string(),
            style.bold_fraction.to_string(),
        );
        attrs.insert(
            HOCR_ITALIC_FRACTION_ATTRIBUTE.to_string(),
            style.italic_fraction.to_string(),
        );
        if let Some(font_name) = style.dominant_font_name {
            attrs.insert(HOCR_FONT_NAME_ATTRIBUTE.to_string(), font_name);
        }
    }

    (Some(elem), pos, removed_line_count)
}

fn claim_proved_table_fragment(line: &HocrLineInfo, words: &mut HashSet<PositionedWord>) -> bool {
    if line.words.is_empty() {
        return false;
    }
    let fragment: HashSet<_> = line
        .words
        .iter()
        .map(|word| ([word.x0, word.y0, word.x1, word.y1], word.text.clone()))
        .collect();
    if fragment.len() != line.words.len() || !fragment.is_subset(words) {
        return false;
    }
    for word in fragment {
        words.remove(&word);
    }
    true
}

fn line_is_inside_table(line: &HocrLineInfo, bbox: &BoundingBox) -> bool {
    !line.words.is_empty()
        && line.words.iter().all(|word| {
            word.x0 < word.x1
                && word.y0 < word.y1
                && f64::from(word.x0) >= bbox.x0
                && f64::from(word.y0) >= bbox.y0
                && f64::from(word.x1) <= bbox.x1
                && f64::from(word.y1) <= bbox.y1
        })
}

/// Check if a tag's class attribute contains the given class name.
fn has_class(tag_content: &str, cls: &str) -> bool {
    if let Some(class_start) = tag_content.find("class=") {
        let rest = &tag_content[class_start + 6..];
        // `rest` is empty when `class=` is the last thing in `tag_content` (a
        // truncated or malformed tag). `.first()` (not `.unwrap_or(..)` on the
        // whole byte) is required here: defaulting a missing byte to `b'"'`
        // would make the `quote` check below pass on an empty `rest`, and the
        // following `&rest[1..]` then panics slicing byte index 1 out of a
        // 0-length string.
        if let Some(quote) = rest.as_bytes().first().copied()
            && (quote == b'"' || quote == b'\'')
        {
            let inner = &rest[1..];
            if let Some(end) = inner.find(quote as char) {
                let class_value = &inner[..end];
                return class_value.split_whitespace().any(|c| c == cls);
            }
        }
    }
    false
}

/// Check if tag content opens a paragraph element (`<p class="ocr_par">` or
/// `<span class="ocr_par">` etc.).
fn is_paragraph_tag(tag_content: &str) -> bool {
    has_class(tag_content, "ocr_par")
}

/// Extract the `title="..."` attribute value from raw tag content.
fn extract_title_attr(tag_content: &str) -> String {
    extract_attribute(tag_content, "title").unwrap_or_default()
}

/// Extract a quoted attribute value from raw tag content.
fn extract_attribute(tag_content: &str, attribute: &str) -> Option<String> {
    let marker = format!("{attribute}=");
    if let Some(attribute_start) = tag_content.find(&marker) {
        let rest = &tag_content[attribute_start + marker.len()..];
        // See the matching comment in `has_class`: `rest` can be empty when the
        // attribute marker is the last thing in `tag_content`, and defaulting a
        // missing byte to a quote char would make `&rest[1..]` panic below.
        if let Some(quote) = rest.as_bytes().first().copied()
            && (quote == b'"' || quote == b'\'')
        {
            let inner = &rest[1..];
            if let Some(end) = inner.find(quote as char) {
                return Some(inner[..end].to_string());
            }
        }
    }
    None
}

/// Extract all text content inside an element, stripping nested tags.
///
/// Walks from `pos` collecting text nodes and descending into nested tags
/// until the matching close tag for the current element is reached.
fn extract_inner_text(html: &str, start: usize) -> String {
    let bytes = html.as_bytes();
    let mut result = String::new();
    let mut pos = start;
    let mut depth: u32 = 1;

    while pos < bytes.len() && depth > 0 {
        if let Some(lt) = memchr(b'<', &bytes[pos..]).map(|i| pos + i) {
            result.push_str(&html[pos..lt]);

            if let Some(gt) = memchr(b'>', &bytes[lt..]).map(|i| lt + i) {
                let tag = &html[lt + 1..gt];
                if tag.starts_with('/') {
                    depth -= 1;
                } else if !tag.ends_with('/') {
                    depth += 1;
                }
                pos = gt + 1;
            } else {
                break;
            }
        } else {
            result.push_str(&html[pos..]);
            break;
        }
    }

    result
}

/// Skip past the matching closing tag for a tag that was just opened.
///
/// `tag_name` is the lowercase name of the opening tag (e.g. "span").
/// Returns the byte position after the closing `>`.
fn skip_to_matching_close(html: &str, start: usize, tag_name: &str) -> usize {
    let bytes = html.as_bytes();
    let mut pos = start;
    let mut depth: u32 = 1;

    while pos < bytes.len() && depth > 0 {
        let Some(lt) = memchr(b'<', &bytes[pos..]).map(|i| pos + i) else {
            break;
        };
        let Some(gt) = memchr(b'>', &bytes[lt..]).map(|i| lt + i) else {
            break;
        };
        let tag = &html[lt + 1..gt];

        if let Some(stripped) = tag.strip_prefix('/') {
            let name = stripped.split_whitespace().next().unwrap_or("");
            if name.eq_ignore_ascii_case(tag_name) {
                depth -= 1;
            }
        } else if !tag.ends_with('/') {
            let name = tag.split_whitespace().next().unwrap_or("");
            if name.eq_ignore_ascii_case(tag_name) {
                depth += 1;
            }
        }

        pos = gt + 1;
    }

    pos
}

/// Decode common HTML entities in text content.
fn decode_html_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_hocr() {
        let doc = parse_hocr_to_internal_document("");
        assert!(doc.elements.is_empty());
    }

    #[test]
    fn test_single_page_single_paragraph() {
        let hocr = r#"<div class="ocr_page" title="bbox 0 0 1000 1500; ppageno 0">
            <p class="ocr_par" title="bbox 100 100 900 200">
                <span class="ocr_line" title="bbox 100 100 900 150">
                    <span class="ocrx_word" title="bbox 100 100 200 140; x_wconf 95">Hello</span>
                    <span class="ocrx_word" title="bbox 210 100 350 140; x_wconf 90">World</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let elements = doc.elements;

        assert_eq!(elements.len(), 1);

        let elem = &elements[0];
        assert_eq!(elem.text, "Hello World");
        assert_eq!(elem.page, Some(1));

        let bbox = elem.bbox.as_ref().unwrap();
        assert_eq!(bbox.x0, 100.0);
        assert_eq!(bbox.y0, 100.0);
        assert_eq!(bbox.x1, 350.0);
        assert_eq!(bbox.y1, 140.0);

        let conf = elem.ocr_confidence.as_ref().unwrap();
        assert!((conf.recognition - 0.925).abs() < 0.01);
    }

    #[test]
    fn numeric_choices_separated_by_blank_marks_remain_distinct_words() {
        let hocr = r#"<div class="ocr_page" title="bbox 0 0 1000 1500; ppageno 0">
            <p class="ocr_par" title="bbox 100 100 900 200">
                <span class="ocr_line" title="bbox 100 100 900 150">
                    <span class="ocrx_word" title="bbox 100 100 300 140; x_wconf 95">$30__.$25</span>
                    <span class="ocrx_word" title="bbox 310 100 450 140; x_wconf 95">file_name</span>
                </span>
            </p>
        </div>"#;

        let document = parse_hocr_to_internal_document(hocr);

        assert_eq!(document.elements[0].text, "$30 .$25 file_name");
    }

    /// Regression test: Tesseract numbers every single-image `recognize()` call's hOCR page
    /// as `ppageno 0`, since each `perform_ocr` call only ever loads one image. When that
    /// image is page 2 of a larger document, the parser must report the caller-supplied true
    /// page number rather than the ppageno-derived `1` every single-image hOCR call would
    /// otherwise produce for every page of the document.
    ///
    /// Fails against unfixed code: before `parse_hocr_to_internal_document_with_page_offset`
    /// existed, this exact hOCR (`ppageno 0`, identical to what Tesseract emits for page 2 of
    /// a document, page 5, or any other page) could only be parsed by
    /// `parse_hocr_to_internal_document`/`_with_dictionary_filter`, both of which hardcode the
    /// offset to `1` -- so every page of a multi-page source would assert `elem.page ==
    /// Some(1)`, never the true page number.
    #[test]
    fn should_report_the_true_page_number_for_each_ocr_element() {
        let hocr = r#"<div class="ocr_page" title="bbox 0 0 1000 1500; ppageno 0">
            <p class="ocr_par" title="bbox 100 100 900 200">
                <span class="ocr_line" title="bbox 100 100 900 150">
                    <span class="ocrx_word" title="bbox 100 100 200 140; x_wconf 95">Hello</span>
                    <span class="ocrx_word" title="bbox 210 100 350 140; x_wconf 90">World</span>
                </span>
            </p>
        </div>"#;

        // `page_offset: 2` stands in for `TesseractConfig::page_number` when `perform_ocr` is
        // called on page 2 of a multi-page document.
        let doc = parse_hocr_to_internal_document_with_page_offset(hocr, None, 2);
        let elements = doc.elements;

        assert_eq!(elements.len(), 1);
        let elem = &elements[0];
        assert_eq!(elem.text, "Hello World");
        assert_eq!(elem.page, Some(2));
    }

    #[test]
    fn test_multi_line_paragraph() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line" title="bbox 10 10 200 30">
                    <span class="ocrx_word" title="bbox 10 10 50 30">Line</span>
                    <span class="ocrx_word" title="bbox 60 10 100 30">one</span>
                </span>
                <span class="ocr_line" title="bbox 10 40 200 60">
                    <span class="ocrx_word" title="bbox 10 40 50 60">Line</span>
                    <span class="ocrx_word" title="bbox 60 40 100 60">two</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let elements = doc.elements;
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].text, "Line one\nLine two");
    }

    #[test]
    fn test_multi_page_inserts_page_breaks() {
        let hocr = r#"
        <div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 10 10 50 30">Page1</span>
            </p>
        </div>
        <div class="ocr_page" title="ppageno 1">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 10 10 50 30">Page2</span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let elements = doc.elements;

        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0].kind, ElementKind::OcrText { .. }));
        assert!(matches!(elements[1].kind, ElementKind::PageBreak));
        assert!(matches!(elements[2].kind, ElementKind::OcrText { .. }));
        assert_eq!(elements[0].page, Some(1));
        assert_eq!(elements[2].page, Some(2));
    }

    #[test]
    fn test_html_entity_decoding() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 10 10 50 30">&amp;foo&lt;bar&gt;</span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        assert_eq!(doc.elements[0].text, "&foo<bar>");
    }

    #[test]
    fn test_words_without_bbox_still_included() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word">NoBbox</span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].text, "NoBbox");
        assert!(doc.elements[0].bbox.is_none());
    }

    #[test]
    fn test_nested_formatting_tags() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 10 10 50 30"><strong>Bold</strong></span>
                <span class="ocrx_word" title="bbox 60 10 100 30"><em>Italic</em></span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        assert_eq!(doc.elements[0].text, "Bold Italic");
    }

    #[test]
    fn test_property_parsing() {
        let props = parse_title_properties("bbox 100 50 200 150; x_wconf 95.5; ppageno 3; textangle 7.2");
        assert_eq!(props.bbox, Some((100, 50, 200, 150)));
        assert_eq!(props.x_wconf, Some(95.5));
        assert_eq!(props.ppageno, Some(3));
        assert_eq!(props.textangle, Some(7.2));
    }

    #[test]
    fn test_baseline_parsing() {
        let props = parse_title_properties("baseline 0.015 -18");
        assert_eq!(props.baseline, Some((0.015, -18)));
    }

    #[test]
    fn test_font_parsing() {
        let props = parse_title_properties("x_font \"Comic Sans MS\"; x_fsize 12");
        assert_eq!(props.x_font, Some("Comic Sans MS".to_string()));
        assert_eq!(props.x_fsize, Some(12));
    }

    #[test]
    fn test_bold_and_italic_flag_parsing() {
        let props = parse_title_properties("x_wconf 95; x_bold; x_italic");
        assert!(props.x_bold);
        assert!(props.x_italic);
    }

    #[test]
    fn test_bold_and_italic_default_to_false_when_absent() {
        let props = parse_title_properties("x_wconf 95");
        assert!(!props.x_bold);
        assert!(!props.x_italic);
    }

    #[test]
    fn test_has_class() {
        assert!(has_class(
            r#"div class="ocr_page" title="bbox 0 0 100 100""#,
            "ocr_page"
        ));
        assert!(!has_class(r#"div class="ocr_page""#, "ocr_par"));
        assert!(has_class(r#"span class="ocrx_word ocr_line""#, "ocrx_word"));
        assert!(has_class(r#"span class="ocrx_word ocr_line""#, "ocr_line"));
    }

    /// A tag whose `class=` marker is the very last thing in `tag_content` (no
    /// quote, no value, nothing after it — e.g. produced by truncated or
    /// malformed hOCR markup) used to panic: `rest` became an empty string,
    /// `rest.as_bytes().first().copied().unwrap_or(b'"')` masked the empty
    /// case by defaulting to a quote byte, and the following `&rest[1..]`
    /// then sliced byte index 1 out of a 0-length string ("byte index 1 is
    /// out of bounds of ``"). Must return `false`, not panic.
    #[test]
    fn has_class_returns_false_instead_of_panicking_when_class_marker_is_truncated() {
        assert!(!has_class("span class=", "ocr_par"));
    }

    #[test]
    fn test_extract_title_attr() {
        let title = extract_title_attr(r#"div class="ocr_page" title="bbox 0 0 100 200; ppageno 0""#);
        assert_eq!(title, "bbox 0 0 100 200; ppageno 0");
    }

    /// Same truncated-marker defect as `has_class`, exercised through
    /// `extract_attribute`/`extract_title_attr`: a tag content ending in
    /// `title=` with nothing after it must not panic.
    #[test]
    fn extract_title_attr_returns_empty_instead_of_panicking_when_title_marker_is_truncated() {
        assert_eq!(extract_title_attr("span title="), "");
    }

    #[test]
    fn test_paragraph_stores_average_font_size_attribute() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90; x_fsize 24">BIG</span>
                    <span class="ocrx_word" title="bbox 60 10 100 30; x_wconf 90; x_fsize 20">Title</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let attrs = doc.elements[0].attributes.as_ref().expect("attributes present");
        assert_eq!(attrs.get(HOCR_FONT_SIZE_ATTRIBUTE), Some(&"22".to_string()));
    }

    #[test]
    fn test_paragraph_stores_bold_italic_fraction_and_font_name_attributes() {
        // Two bold words out of four total ("HEADING" split into two spans),
        // one italic, and a single reported font family.
        let hocr = r#"<div class='ocr_page' title='ppageno 0'>
            <p class='ocr_par'>
                <span class='ocr_line'>
                    <span class='ocrx_word' title='bbox 10 10 50 30; x_wconf 90; x_font "Arial"; x_bold'>HEAD</span>
                    <span class='ocrx_word' title='bbox 60 10 100 30; x_wconf 90; x_font "Arial"; x_bold'>ING</span>
                    <span class='ocrx_word' title='bbox 110 10 150 30; x_wconf 90; x_font "Arial"; x_italic'>plain</span>
                    <span class='ocrx_word' title='bbox 160 10 200 30; x_wconf 90; x_font "Arial"'>text</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let attrs = doc.elements[0].attributes.as_ref().expect("attributes present");

        // Without the fix, `HOCR_BOLD_FRACTION_ATTRIBUTE`/`HOCR_ITALIC_FRACTION_ATTRIBUTE`
        // are never inserted (parse_paragraph has no bold/italic aggregation), so this
        // lookup returns None and the assert_eq fails against `Some("0.5")`.
        assert_eq!(attrs.get(HOCR_BOLD_FRACTION_ATTRIBUTE), Some(&"0.5".to_string()));
        assert_eq!(attrs.get(HOCR_ITALIC_FRACTION_ATTRIBUTE), Some(&"0.25".to_string()));
        assert_eq!(attrs.get(HOCR_FONT_NAME_ATTRIBUTE), Some(&"Arial".to_string()));
    }

    #[test]
    fn test_paragraph_all_words_non_bold_reports_zero_bold_fraction() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">plain</span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let attrs = doc.elements[0].attributes.as_ref().expect("attributes present");

        assert_eq!(attrs.get(HOCR_BOLD_FRACTION_ATTRIBUTE), Some(&"0".to_string()));
        assert_eq!(attrs.get(HOCR_ITALIC_FRACTION_ATTRIBUTE), Some(&"0".to_string()));
        assert_eq!(attrs.get(HOCR_FONT_NAME_ATTRIBUTE), None);
    }

    #[test]
    fn test_paragraph_without_font_size_has_no_attribute() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">NoFontSize</span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let has_font_size = doc.elements[0]
            .attributes
            .as_ref()
            .is_some_and(|attrs| attrs.contains_key(HOCR_FONT_SIZE_ATTRIBUTE));
        assert!(
            !has_font_size,
            "should not synthesize a font size when hOCR provides none"
        );
    }

    #[test]
    fn test_paragraph_stores_average_text_angle_attribute() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90; textangle 90">Rotated</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let attrs = doc.elements[0].attributes.as_ref().expect("attributes present");
        assert_eq!(attrs.get(HOCR_TEXT_ANGLE_ATTRIBUTE), Some(&"90".to_string()));
    }

    #[test]
    fn test_ocr_geometry_set() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocrx_word" title="bbox 50 60 150 100; x_wconf 88">test</span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let elem = &doc.elements[0];
        let geom = elem.ocr_geometry.as_ref().unwrap();
        match geom {
            OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } => {
                assert_eq!(left, &50);
                assert_eq!(top, &60);
                assert_eq!(width, &100);
                assert_eq!(height, &40);
            }
            _ => panic!("Expected Rectangle geometry"),
        }
    }

    #[test]
    fn test_english_pdf_real_data() {
        let hocr = include_str!("../../test_data/hocr/english_pdf_default.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(
            !doc.elements.is_empty(),
            "Should extract elements from English PDF hOCR"
        );
        let total_text: String = doc
            .elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!total_text.trim().is_empty(), "Should have non-empty text");
        let has_pages = doc.elements.iter().any(|e| e.page.is_some());
        assert!(has_pages, "Should have page numbers");
    }

    #[test]
    fn test_german_pdf_real_data() {
        let hocr = include_str!("../../test_data/hocr/german_pdf_default.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(!doc.elements.is_empty(), "Should extract elements from German PDF hOCR");
        let total_text: String = doc
            .elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!total_text.trim().is_empty(), "Should have non-empty German text");
    }

    #[test]
    fn test_invoice_image_real_data() {
        let hocr = include_str!("../../test_data/hocr/invoice_image_default.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(!doc.elements.is_empty(), "Should extract elements from invoice hOCR");
        let total_text: String = doc
            .elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            total_text.chars().any(|c| c.is_ascii_digit()),
            "Invoice should contain numbers"
        );
    }

    #[test]
    fn test_word_confidence_real_data() {
        let hocr = include_str!("../../test_data/hocr/word_confidence.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(
            doc.elements.is_empty(),
            "Non-hOCR-classed elements should not be extracted"
        );
    }

    #[test]
    fn test_utf8_encoding_real_data() {
        let hocr = include_str!("../../test_data/hocr/utf8_encoding.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(
            doc.elements.is_empty(),
            "Non-hOCR-classed UTF-8 content should not be extracted"
        );
    }

    #[test]
    fn test_v4_with_tables_and_code() {
        let hocr = include_str!("../../test_data/hocr/v4_code_formula.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(
            !doc.elements.is_empty(),
            "Should extract from v4 hOCR with code/formula"
        );
    }

    #[test]
    fn test_v4_embedded_tables() {
        let hocr = include_str!("../../test_data/hocr/v4_embedded_tables.hocr");
        let doc = parse_hocr_to_internal_document(hocr);
        assert!(
            !doc.elements.is_empty(),
            "Should extract from v4 hOCR with embedded tables"
        );
    }

    #[test]
    fn test_many_paragraphs_all_captured() {
        let paragraph_texts: Vec<&str> = vec![
            "First paragraph",
            "Second paragraph",
            "Third paragraph",
            "Fourth paragraph",
            "Fifth paragraph",
            "Sixth paragraph",
            "Seventh paragraph",
            "Eighth paragraph",
            "Ninth paragraph",
            "Tenth paragraph",
            "Eleventh paragraph",
            "Twelfth paragraph",
            "Thirteenth paragraph",
            "Fourteenth paragraph",
            "Fifteenth paragraph",
            "Sixteenth paragraph",
            "Seventeenth paragraph",
            "Eighteenth paragraph",
            "Nineteenth paragraph",
            "Twentieth paragraph",
            "Twenty-first paragraph",
            "Twenty-second paragraph",
            "Twenty-third paragraph",
            "Twenty-fourth paragraph",
            "Twenty-fifth paragraph",
            "Service category alpha",
            "Service category beta",
            "Service category gamma",
            "Service category delta",
            "All other categories",
            "Items provided by client",
            "*** Note this is the last paragraph",
        ];

        let mut hocr = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN"
    "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
 <head>
  <title></title>
  <meta http-equiv="Content-Type" content="text/html;charset=utf-8"/>
  <meta name='ocr-system' content='tesseract 5.5.1' />
 </head>
 <body>
  <div class='ocr_page' id='page_1' title='image "test.png"; bbox 0 0 2550 3300; ppageno 0; scan_res 300 300'>
"#,
        );

        let mut y = 100;
        for (i, text) in paragraph_texts.iter().enumerate() {
            let block_id = i + 1;
            let par_id = i + 1;
            let line_id = i + 1;
            let y0 = y;
            let y1 = y + 30;

            hocr.push_str(&format!(
                r#"   <div class='ocr_carea' id='block_1_{block_id}' title="bbox 100 {y0} 2400 {y1}">
    <p class='ocr_par' id='par_1_{par_id}' lang='eng' title="bbox 100 {y0} 2400 {y1}">
     <span class='ocr_line' id='line_1_{line_id}' title="bbox 100 {y0} 2400 {y1}; baseline 0 0; x_size 30; x_descenders 6; x_ascenders 8">
"#
            ));

            let mut wx = 100;
            for (wi, word) in text.split_whitespace().enumerate() {
                let word_id = i * 10 + wi + 1;
                let wx1 = wx + word.len() as u32 * 20;
                hocr.push_str(&format!(
                    "      <span class='ocrx_word' id='word_1_{word_id}' title='bbox {wx} {y0} {wx1} {y1}; x_wconf 90'>{word}</span>\n"
                ));
                wx = wx1 + 10;
            }

            hocr.push_str("     </span>\n    </p>\n   </div>\n");

            y = y1 + 10;
        }

        hocr.push_str("  </div>\n </body>\n</html>\n");

        let doc = parse_hocr_to_internal_document(&hocr);

        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(
            text_elements.len(),
            paragraph_texts.len(),
            "Expected {} paragraphs but got {}. Missing paragraphs from the end.",
            paragraph_texts.len(),
            text_elements.len()
        );

        for (i, (elem, expected)) in text_elements.iter().zip(paragraph_texts.iter()).enumerate() {
            assert_eq!(
                elem.text,
                *expected,
                "Paragraph {} mismatch: expected '{}', got '{}'",
                i + 1,
                expected,
                elem.text
            );
        }

        let last_text = &text_elements.last().unwrap().text;
        assert_eq!(
            last_text, "*** Note this is the last paragraph",
            "Last paragraph should be captured"
        );
    }

    #[test]
    fn test_paragraph_with_nested_span_in_word() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90"><span class="ocrx_font" style="font-size:12px">Hello</span></span>
        <span class="ocrx_word" title="bbox 60 10 100 30; x_wconf 90">World</span>
      </span>
    </p>
  </div>
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 80 70; x_wconf 90">Second</span>
        <span class="ocrx_word" title="bbox 90 50 180 70; x_wconf 90">paragraph</span>
      </span>
    </p>
  </div>
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 90 80 110; x_wconf 90">Third</span>
        <span class="ocrx_word" title="bbox 90 90 180 110; x_wconf 90">paragraph</span>
      </span>
    </p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(text_elements.len(), 3, "Should capture all 3 paragraphs");
        assert_eq!(text_elements[0].text, "Hello World");
        assert_eq!(text_elements[1].text, "Second paragraph");
        assert_eq!(text_elements[2].text, "Third paragraph");
    }

    #[test]
    fn test_paragraph_with_words_outside_line() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">Direct</span>
      <span class="ocrx_word" title="bbox 60 10 120 30; x_wconf 90">words</span>
    </p>
  </div>
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 80 70; x_wconf 90">Next</span>
        <span class="ocrx_word" title="bbox 90 50 160 70; x_wconf 90">paragraph</span>
      </span>
    </p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(text_elements.len(), 2, "Should capture both paragraphs");
        assert_eq!(text_elements[0].text, "Direct words");
        assert_eq!(text_elements[1].text, "Next paragraph");
    }

    #[test]
    fn test_paragraph_depth_with_extra_div_nesting() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <p class="ocr_par">
      <div class="ocr_column">
        <span class="ocr_line">
          <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">Nested</span>
        </span>
      </div>
    </p>
  </div>
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 80 70; x_wconf 90">After</span>
        <span class="ocrx_word" title="bbox 90 50 160 70; x_wconf 90">nested</span>
      </span>
    </p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(
            text_elements.len(),
            2,
            "Should capture both paragraphs even with extra div nesting"
        );
        assert_eq!(text_elements[0].text, "Nested");
        assert_eq!(text_elements[1].text, "After nested");
    }

    #[test]
    fn test_paragraph_div_swallows_carea_close() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <div class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">First</span>
      </span>
    </div>
  </div>
  <div class="ocr_carea">
    <div class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 50 70; x_wconf 90">Second</span>
      </span>
    </div>
  </div>
  <div class="ocr_carea">
    <div class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 90 50 110; x_wconf 90">Third</span>
      </span>
    </div>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(text_elements.len(), 3, "Should capture all 3 div-based paragraphs");
    }

    #[test]
    fn test_paragraph_unclosed_par_div_steals_carea_close() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <div class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">First</span>
      </span>
  </div>
  <div class="ocr_carea">
    <div class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 50 70; x_wconf 90">Second</span>
      </span>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(
            text_elements.len(),
            2,
            "Should find both paragraphs even with unclosed par divs. Got: {:?}",
            text_elements.iter().map(|e| e.text.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_depth_tracking_uses_paragraph_tag_name() {
        let hocr_separate = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90"><span>Styled</span></span>
        <span class="ocrx_word" title="bbox 60 10 120 30; x_wconf 90">text</span>
      </span>
    </p>
  </div>
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 80 70; x_wconf 90">After</span>
      </span>
    </p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr_separate);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();
        assert_eq!(text_elements.len(), 2);
        assert_eq!(text_elements[0].text, "Styled text");
        assert_eq!(text_elements[1].text, "After");

        let hocr_same_carea = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90"><span>Styled</span></span>
      </span>
    </p>
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 80 70; x_wconf 90">Should</span>
        <span class="ocrx_word" title="bbox 90 50 180 70; x_wconf 90">be</span>
        <span class="ocrx_word" title="bbox 190 50 280 70; x_wconf 90">separate</span>
      </span>
    </p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr_same_carea);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();
        assert_eq!(
            text_elements.len(),
            2,
            "Should find both paragraphs separately. Got: {:?}",
            text_elements.iter().map(|e| e.text.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(text_elements[0].text, "Styled");
        assert_eq!(text_elements[1].text, "Should be separate");
    }

    #[test]
    fn test_paragraphs_retain_enclosing_hocr_block_id() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea" id="block_1_1">
    <div class="nested"><p class="ocr_par"><span class="ocrx_word">First</span></p></div>
    <p class="ocr_par"><span class="ocrx_word">Second</span></p>
  </div>
  <div class="ocr_carea" id="block_1_2">
    <p class="ocr_par"><span class="ocrx_word">Third</span></p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let block_ids = doc
            .elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::OcrText { .. }))
            .map(|element| {
                element
                    .attributes
                    .as_ref()
                    .and_then(|attributes| attributes.get(HOCR_BLOCK_ID_ATTRIBUTE))
                    .map(String::as_str)
            })
            .collect::<Vec<_>>();

        assert_eq!(block_ids, vec![Some("block_1_1"), Some("block_1_1"), Some("block_1_2")]);
    }

    #[test]
    fn test_paragraph_with_ocr_separator_between_paragraphs() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">Before</span>
      </span>
    </p>
  </div>
  <div class="ocr_separator" title="bbox 10 40 500 42"></div>
  <div class="ocr_carea">
    <p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 50 50 70; x_wconf 90">After</span>
      </span>
    </p>
  </div>
</div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let text_elements: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .collect();

        assert_eq!(
            text_elements.len(),
            2,
            "Should capture both paragraphs around separator"
        );
    }

    #[test]
    fn test_property_parsing_recovers_x_size_ascenders_descenders() {
        // Fails against unfixed code: `HocrProperties` had no `x_size` /
        // `x_ascenders` / `x_descenders` fields at all, so these keys parsed
        // to nothing regardless of what the title string contained.
        let props =
            parse_title_properties("bbox 100 40 900 150; baseline 0.015 -18; x_size 30; x_descenders 6; x_ascenders 8");
        assert_eq!(props.x_size, Some(30.0));
        assert_eq!(props.x_ascenders, Some(8.0));
        assert_eq!(props.x_descenders, Some(6.0));
        assert_eq!(props.baseline, Some((0.015, -18)));
    }

    #[test]
    fn test_ocr_line_title_parsed_into_paragraph_x_height_and_baseline_attributes() {
        // Fails against unfixed code: the `ocr_line`/`ocrx_line` branch only
        // flipped `in_line` and never called `parse_title_properties` on that
        // tag's own title, so `baseline`/`x_size`/`x_ascenders`/`x_descenders`
        // were unreachable even though Tesseract emits them on the line tag
        // (not the word tag).
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line"
                    title="bbox 100 40 900 150; baseline 0.01 -18; x_size 30; x_ascenders 8; x_descenders 6">
                    <span class="ocrx_word" title="bbox 100 40 300 150; x_wconf 95">Heading</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let attrs = doc.elements[0].attributes.as_ref().expect("attributes present");

        assert_eq!(attrs.get(HOCR_X_HEIGHT_ATTRIBUTE), Some(&"30".to_string()));
        assert_eq!(attrs.get(HOCR_X_ASCENDERS_ATTRIBUTE), Some(&"8".to_string()));
        assert_eq!(attrs.get(HOCR_X_DESCENDERS_ATTRIBUTE), Some(&"6".to_string()));
        assert_eq!(attrs.get(HOCR_BASELINE_SLOPE_ATTRIBUTE), Some(&"0.01".to_string()));
        assert_eq!(attrs.get(HOCR_BASELINE_CONST_ATTRIBUTE), Some(&"-18".to_string()));
    }

    #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
    #[test]
    fn proved_fragments_require_exact_hocr_occurrences_in_both_directions() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
          <p class="ocr_par"><span class="ocr_line" title="bbox 10 20 30 40">
            <span class="ocrx_word" title="bbox 10 20 30 40; x_wconf 90">One</span>
          </span></p>
        </div>"#;
        let exact = HocrWord {
            text: "One".to_owned(),
            left: 10,
            top: 20,
            width: 20,
            height: 20,
            confidence: 90.0,
        };
        let unmatched = HocrWord {
            left: 31,
            ..exact.clone()
        };
        assert!(hocr_matches_all_proved_table_fragments(hocr, 1, vec![&exact]));
        assert!(!hocr_matches_all_proved_table_fragments(
            hocr,
            1,
            vec![&exact, &unmatched]
        ));
        assert!(!hocr_matches_all_proved_table_fragments(hocr, 1, vec![&exact, &exact]));
        assert!(!hocr_matches_all_proved_table_fragments(hocr, 1, vec![&unmatched]));
    }

    #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
    #[test]
    fn proved_fragments_do_not_claim_a_line_with_an_unproved_word() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
          <p class="ocr_par"><span class="ocr_line" title="bbox 10 20 70 40">
            <span class="ocrx_word" title="bbox 10 20 30 40; x_wconf 90">One</span>
            <span class="ocrx_word" title="bbox 40 20 70 40; x_wconf 90">Two</span>
          </span></p>
        </div>"#;
        let one = HocrWord {
            text: "One".to_owned(),
            left: 10,
            top: 20,
            width: 20,
            height: 20,
            confidence: 90.0,
        };
        let two = HocrWord {
            text: "Two".to_owned(),
            left: 40,
            top: 20,
            width: 30,
            height: 20,
            confidence: 90.0,
        };
        assert!(hocr_matches_all_proved_table_fragments(hocr, 1, vec![&one, &two]));
        assert!(!hocr_matches_all_proved_table_fragments(hocr, 1, vec![&one]));
    }

    #[test]
    fn claimed_row_does_not_affect_retained_line_geometry_or_style() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line" title="baseline 0 -3; x_size 17">
                    <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 92; x_fsize 16">Retained</span>
                </span>
                <span class="ocr_line" title="baseline 0 -10; x_size 35">
                    <span class="ocrx_word" title="bbox 10 40 80 70; x_wconf 83; x_fsize 32">Claimed</span>
                </span>
            </p>
        </div>"#;
        let retained_box = BoundingBox {
            x0: 10.0,
            y0: 10.0,
            x1: 50.0,
            y1: 30.0,
        };
        let table_box = BoundingBox {
            x1: 80.0,
            y1: 70.0,
            ..retained_box
        };
        let cells = vec![vec!["Claimed".to_string()]];
        let doc = parse_hocr_without_claimed_table_rows(hocr, 4, &table_box, &cells, false);

        assert_eq!(doc.elements.len(), 1);
        let line = &doc.elements[0];
        assert_eq!(line.text, "Retained");
        assert_eq!(line.bbox, Some(retained_box));
        assert_eq!(line.page, Some(4));
        assert_eq!(line.ocr_confidence.as_ref().unwrap().recognition, 0.92);
        let attrs = line.attributes.as_ref().unwrap();
        assert_eq!(attrs[HOCR_LINE_FONT_SIZES_ATTRIBUTE], "16");
        assert_eq!(attrs[HOCR_LINE_X_HEIGHTS_ATTRIBUTE], "17");
        assert_eq!(attrs[HOCR_BASELINE_CONST_ATTRIBUTE], "-3");
    }

    #[test]
    fn test_paragraph_without_line_title_has_no_x_height_attributes() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 10 50 30; x_wconf 90">Plain</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let has_x_height = doc.elements[0]
            .attributes
            .as_ref()
            .is_some_and(|attrs| attrs.contains_key(HOCR_X_HEIGHT_ATTRIBUTE));
        assert!(!has_x_height, "should not synthesize x-height when hOCR provides none");
    }

    #[test]
    fn test_multi_line_paragraph_preserves_per_line_font_size_and_x_height() {
        // Fails against unfixed code in two ways: (1) `line_font_sizes` /
        // `line_x_heights` attributes don't exist at all pre-fix, so both
        // `get` calls return `None`; (2) even measuring only the paragraph
        // mean (22 = (24+20)/2) would hide that line one is a 24pt heading
        // and line two is 20pt body text, which is exactly the per-line
        // detail #667/#669 ask to preserve.
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line" title="bbox 10 10 200 40; x_size 28">
                    <span class="ocrx_word" title="bbox 10 10 100 40; x_wconf 90; x_fsize 24">BIG</span>
                </span>
                <span class="ocr_line" title="bbox 10 50 200 70">
                    <span class="ocrx_word" title="bbox 10 50 100 70; x_wconf 90; x_fsize 20">small</span>
                </span>
            </p>
        </div>"#;

        let doc = parse_hocr_to_internal_document(hocr);
        let attrs = doc.elements[0].attributes.as_ref().expect("attributes present");

        // Paragraph mean still present for existing consumers (#185).
        assert_eq!(attrs.get(HOCR_FONT_SIZE_ATTRIBUTE), Some(&"22".to_string()));

        // Per-line detail: line one is 24pt/x_size 28, line two is 20pt with
        // no line-level x_size (second field empty, position preserved).
        assert_eq!(attrs.get(HOCR_LINE_FONT_SIZES_ATTRIBUTE), Some(&"24,20".to_string()));
        assert_eq!(attrs.get(HOCR_LINE_X_HEIGHTS_ATTRIBUTE), Some(&"28,".to_string()));
    }

    /// Coverage for the per-line dictionary-invalid noise filter (#783).
    ///
    /// Named-import trap check: the reverted prior attempt (`29738a1f29`) added its tests
    /// to a module that imported by name (`use super::{...}`), and the new function was
    /// never added to that list, so the tests silently never compiled. This module uses
    /// `use super::*;` (see the top of `mod tests`), so that specific failure mode cannot
    /// recur here.
    mod dictionary_line_filter_tests {
        use super::*;

        /// Elevations-page hOCR, reconstructed directly from the recorded GH#783 defect:
        /// a correctly-read heading line, a fully garbled title-block line, and a second
        /// correctly-read heading line, all in one `ocr_par` block (Tesseract commonly
        /// groups a title block's short lines into a single paragraph).
        const ELEVATIONS_PAGE_HOCR: &str = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 10 100 40">RIGHT</span>
                    <span class="ocrx_word" title="bbox 110 10 260 40">ELEVATION</span>
                </span>
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 50 100 80">OWATS</span>
                    <span class="ocrx_word" title="bbox 110 50 220 80">DNDEVET</span>
                    <span class="ocrx_word" title="bbox 230 50 320 80">OPMENT</span>
                </span>
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 90 100 120">LEFT</span>
                    <span class="ocrx_word" title="bbox 110 90 260 120">ELEVATION</span>
                </span>
            </p>
        </div>"#;

        /// Dictionary lookup matching the real measurement recorded against GH#783:
        /// "OWATS" and "DNDEVET" are invalid, "OPMENT" is a Tesseract DAWG false
        /// positive (reported valid), and the two "ELEVATION"/"RIGHT"/"LEFT" words are
        /// genuinely valid. Every test in this module uses this exact table so the
        /// scenario matches the measured behavior, not an idealized dictionary.
        fn measured_is_valid_word(word: &str) -> Option<bool> {
            Some(!matches!(word, "OWATS" | "DNDEVET"))
        }

        /// 0.6, matching [`DEFAULT_DICT_INVALID_LINE_RATIO`] -- duplicated here as a
        /// literal (rather than referencing the constant directly) because what this
        /// module tests is the *filtering mechanism* at a fixed, known threshold, not
        /// that the production default stays at any particular value.
        const TEST_THRESHOLD: f64 = 0.6;

        #[test]
        fn rejected_table_restores_an_exact_repeat_at_its_original_line_position() {
            let hocr = ELEVATIONS_PAGE_HOCR.replace("OPMENT", "ELEVATION");
            let filter = DictionaryLineFilter {
                is_valid_word: &measured_is_valid_word,
                max_invalid_ratio: TEST_THRESHOLD,
            };
            let rejected_word = |text: &str, left, top, width| HocrWord {
                text: text.to_owned(),
                left,
                top,
                width,
                height: 30,
                confidence: 90.0,
            };
            let mut region = vec![
                rejected_word("OWATS", 10, 50, 90),
                rejected_word("DNDEVET", 110, 50, 110),
                rejected_word("ELEVATION", 110, 10, 150),
            ];
            let mismatched = parse_hocr_preserving_rejected_table_words(&hocr, &filter, 1, &[region.clone()]);
            assert_eq!(mismatched.dictionary_filtered_line_count, 1);
            assert_eq!(mismatched.document.elements[0].text, "RIGHT ELEVATION\nLEFT ELEVATION");

            region[2] = rejected_word("ELEVATION", 230, 50, 90);
            let restored = parse_hocr_preserving_rejected_table_words(&hocr, &filter, 1, &[region]);
            assert_eq!(restored.dictionary_filtered_line_count, 0);
            assert_eq!(restored.document.elements.len(), 1);
            assert_eq!(
                restored.document.elements[0].text,
                "RIGHT ELEVATION\nOWATS DNDEVET ELEVATION\nLEFT ELEVATION"
            );
        }

        /// The exact defect from #783: with the real (imperfect) dictionary behavior,
        /// the garbage line is still removed -- 2 invalid of 3 candidates (0.667) clears
        /// the 0.6 threshold even though "OPMENT" is counted as valid -- while both
        /// correctly-read heading lines survive untouched.
        ///
        /// This is checked on `doc.elements[0].text` rather than on
        /// `ocr::processor::execution::flatten_hocr_elements_to_text`'s output (private to
        /// that module, not reachable from here) -- but that is exactly the point being
        /// proven: `flatten_hocr_elements_to_text` only ever concatenates/transforms
        /// element text that is already present, so a line filtered out here, before any
        /// `InternalElement` is constructed, cannot resurface in that flattening OR in the
        /// `PdfParagraph`s `pdf::structure::adapters` builds from these same elements. One
        /// filtered `text` field feeds both downstream renderings; there is no second
        /// place for the two to drift apart.
        #[test]
        fn removes_the_garbage_line_but_keeps_both_real_headings() {
            let filter = DictionaryLineFilter {
                is_valid_word: &measured_is_valid_word,
                max_invalid_ratio: TEST_THRESHOLD,
            };
            let doc = parse_hocr_to_internal_document_with_dictionary_filter(ELEVATIONS_PAGE_HOCR, Some(&filter));

            assert_eq!(
                doc.elements.len(),
                1,
                "the paragraph survives: two of its three lines are real text"
            );
            let text = &doc.elements[0].text;
            assert!(!text.contains("OWATS"), "the noise line must be gone: {text:?}");
            assert!(!text.contains("DNDEVET"), "the noise line must be gone: {text:?}");
            assert_eq!(
                text, "RIGHT ELEVATION\nLEFT ELEVATION",
                "exactly the two real lines remain, in order"
            );
        }

        #[test]
        fn reports_the_exact_number_of_dictionary_filtered_lines() {
            let hocr = ELEVATIONS_PAGE_HOCR.replacen(
                r#"<span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 90 100 120">LEFT</span>"#,
                r#"<span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 82 100 88">OWATS</span>
                    <span class="ocrx_word" title="bbox 110 82 220 88">DNDEVET</span>
                    <span class="ocrx_word" title="bbox 230 82 320 88">OPMENT</span>
                </span>
                <span class="ocr_line">
                    <span class="ocrx_word" title="bbox 10 90 100 120">LEFT</span>"#,
                1,
            );
            let filter = DictionaryLineFilter {
                is_valid_word: &measured_is_valid_word,
                max_invalid_ratio: TEST_THRESHOLD,
            };

            let result = parse_hocr_to_internal_document_with_page_offset_and_stats(&hocr, Some(&filter), 1);

            assert_eq!(result.dictionary_filtered_line_count, 2);
            assert_eq!(result.document.elements[0].text, "RIGHT ELEVATION\nLEFT ELEVATION");
        }

        #[test]
        fn retained_confidence_stats_exclude_dictionary_filtered_lines() {
            let hocr = r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40; x_wconf 20">CLEAR</span>
                        <span class="ocrx_word" title="bbox 110 10 220 40; x_wconf 90">WORDS</span>
                    </span>
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 50 100 80; x_wconf 0">OWATS</span>
                        <span class="ocrx_word" title="bbox 110 50 220 80; x_wconf 0">DNDEVET</span>
                    </span>
                </p>
            </div>"#;
            let is_valid = |word: &str| Some(matches!(word, "CLEAR" | "WORDS"));
            let filter = DictionaryLineFilter {
                is_valid_word: &is_valid,
                max_invalid_ratio: TEST_THRESHOLD,
            };

            let unfiltered = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, None, 1);
            let unfiltered_stats = &unfiltered.retained_word_confidence_stats;
            assert_eq!(unfiltered_stats.word_count(), 4);
            assert_eq!(unfiltered_stats.mean(), Some(27));
            assert_eq!(unfiltered_stats.median(), Some(10));
            assert_eq!(unfiltered_stats.p10(), Some(0));
            assert_eq!(unfiltered_stats.low_confidence_word_count(), 3);

            let result = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, Some(&filter), 1);
            let stats = &result.retained_word_confidence_stats;

            assert_eq!(result.document.elements[0].text, "CLEAR WORDS");
            assert_eq!(result.dictionary_filtered_line_count, 1);
            assert_eq!(stats.word_count(), 2);
            assert_eq!(stats.mean(), Some(55));
            assert_eq!(stats.median(), Some(55));
            assert_eq!(stats.p10(), Some(20));
            assert_eq!(stats.low_confidence_word_count(), 1);
        }

        #[test]
        fn rejected_all_words_produces_empty_stats() {
            let hocr = r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40; x_wconf 10">OWATS</span>
                        <span class="ocrx_word" title="bbox 110 10 220 40; x_wconf 90">DNDEVET</span>
                    </span>
                </p>
            </div>"#;
            let always_invalid = |_: &str| Some(false);
            let filter = DictionaryLineFilter {
                is_valid_word: &always_invalid,
                max_invalid_ratio: TEST_THRESHOLD,
            };

            let unfiltered = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, None, 1);
            assert_eq!(unfiltered.retained_word_confidence_stats.word_count(), 2);
            assert_eq!(unfiltered.retained_word_confidence_stats.median(), Some(50));

            let result = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, Some(&filter), 1);
            let stats = &result.retained_word_confidence_stats;

            assert!(result.document.elements.is_empty());
            assert_eq!(stats.word_count(), 0);
            assert_eq!(stats.mean(), None);
            assert_eq!(stats.median(), None);
            assert_eq!(stats.p10(), None);
            assert_eq!(stats.low_confidence_word_count(), 0);
        }

        /// A line with only ONE dictionary-checkable word must never be scored, even when
        /// that word is invalid -- a lone proper noun or a truncated title-block fragment
        /// standing alone on its own line must not be flagged from a single data point.
        #[test]
        fn a_single_candidate_line_is_never_flagged() {
            let hocr = r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40">Ligustrum</span>
                    </span>
                </p>
            </div>"#;
            let always_invalid = |_: &str| Some(false);
            let filter = DictionaryLineFilter {
                is_valid_word: &always_invalid,
                max_invalid_ratio: 0.0,
            };

            let doc = parse_hocr_to_internal_document_with_dictionary_filter(hocr, Some(&filter));

            assert_eq!(
                doc.elements.len(),
                1,
                "a single-candidate line must survive regardless of the ratio"
            );
            assert_eq!(doc.elements[0].text, "Ligustrum");
        }

        /// A mixed line at or below the threshold survives -- the plant-list guard from
        /// the original #783 report ("Ligustrum, Photinia, Azalea, Indian Hawthorne" mixes
        /// recognized words with unrecognized botanical genus names, 2 invalid of 5 =
        /// 0.4), reconstructed here as a single hOCR line.
        #[test]
        fn a_mixed_line_below_threshold_survives_verbatim() {
            let hocr = r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40">Ligustrum</span>
                        <span class="ocrx_word" title="bbox 110 10 220 40">Photinia</span>
                        <span class="ocrx_word" title="bbox 230 10 320 40">Azalea</span>
                        <span class="ocrx_word" title="bbox 330 10 420 40">Indian</span>
                        <span class="ocrx_word" title="bbox 430 10 560 40">Hawthorne</span>
                    </span>
                </p>
            </div>"#;
            let is_valid = |word: &str| Some(matches!(word, "Azalea" | "Indian" | "Hawthorne"));
            let filter = DictionaryLineFilter {
                is_valid_word: &is_valid,
                max_invalid_ratio: TEST_THRESHOLD,
            };

            let doc = parse_hocr_to_internal_document_with_dictionary_filter(hocr, Some(&filter));

            assert_eq!(doc.elements.len(), 1);
            assert_eq!(doc.elements[0].text, "Ligustrum Photinia Azalea Indian Hawthorne");
        }

        /// A ratio exactly AT the threshold must survive -- the check is strictly
        /// greater-than, matching the page-level `is_dictionary_invalid_noise` convention
        /// (`extractors::pdf::ocr`).
        #[test]
        fn a_line_exactly_at_the_threshold_is_not_removed() {
            let hocr = r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40">Photinia</span>
                        <span class="ocrx_word" title="bbox 110 10 220 40">Ligustrum</span>
                    </span>
                </p>
            </div>"#;
            let always_invalid = |_: &str| Some(false);
            let filter = DictionaryLineFilter {
                is_valid_word: &always_invalid,
                max_invalid_ratio: 1.0,
            };

            let doc = parse_hocr_to_internal_document_with_dictionary_filter(hocr, Some(&filter));

            assert_eq!(
                doc.elements.len(),
                1,
                "ratio 1.0 is not > threshold 1.0, so the line must survive"
            );
        }

        /// A paragraph whose every line is noise disappears from the document entirely --
        /// the same "no words survived" path an all-empty paragraph already takes,
        /// exercised here via dictionary filtering rather than empty text.
        #[test]
        fn a_paragraph_left_with_no_lines_produces_no_element() {
            let hocr = r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40">OWATS</span>
                        <span class="ocrx_word" title="bbox 110 10 220 40">DNDEVET</span>
                    </span>
                </p>
            </div>"#;
            let always_invalid = |_: &str| Some(false);
            let filter = DictionaryLineFilter {
                is_valid_word: &always_invalid,
                max_invalid_ratio: TEST_THRESHOLD,
            };

            let doc = parse_hocr_to_internal_document_with_dictionary_filter(hocr, Some(&filter));

            assert!(
                doc.elements.is_empty(),
                "a paragraph with no surviving lines must not appear at all"
            );
        }

        /// A line whose letter words are all dictionary-invalid but which holds a numeral keeps
        /// every word: a numeral is data, never noise (#2851). The letter-only twin of the same
        /// line is still removed, so the numeral decides the outcome.
        #[test]
        fn a_noise_line_with_a_numeral_survives_and_its_letter_only_twin_does_not() {
            let hocr = |third: &str| {
                format!(
                    r#"<div class="ocr_page" title="ppageno 0">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 10 100 40">Report</span>
                    </span>
                    <span class="ocr_line">
                        <span class="ocrx_word" title="bbox 10 50 100 80">OWATS</span>
                        <span class="ocrx_word" title="bbox 110 50 220 80">DNDEVET</span>
                        <span class="ocrx_word" title="bbox 230 50 320 80">{third}</span>
                    </span>
                </p>
            </div>"#
                )
            };
            let invalid_unless_report = |word: &str| Some(word == "Report");
            let filter = DictionaryLineFilter {
                is_valid_word: &invalid_unless_report,
                max_invalid_ratio: TEST_THRESHOLD,
            };

            let with_numeral =
                parse_hocr_to_internal_document_with_page_offset_and_stats(&hocr("1,250.75"), Some(&filter), 1);
            assert_eq!(with_numeral.dictionary_filtered_line_count, 0);
            assert_eq!(with_numeral.document.elements[0].text, "Report\nOWATS DNDEVET 1,250.75");

            let arabic_indic = parse_hocr_to_internal_document_with_page_offset_and_stats(
                &hocr("\u{0661}\u{0662}\u{0663}"),
                Some(&filter),
                1,
            );
            assert_eq!(arabic_indic.dictionary_filtered_line_count, 0);
            assert_eq!(
                arabic_indic.document.elements[0].text,
                "Report\nOWATS DNDEVET \u{0661}\u{0662}\u{0663}"
            );

            let letter_only =
                parse_hocr_to_internal_document_with_page_offset_and_stats(&hocr("OPMENT"), Some(&filter), 1);
            assert_eq!(letter_only.dictionary_filtered_line_count, 1);
            assert_eq!(letter_only.document.elements[0].text, "Report");
        }

        /// No filter at all (the plain [`parse_hocr_to_internal_document`] entry point,
        /// what every other test in this file uses) must behave exactly as before this
        /// feature existed: nothing is removed, no matter how garbled the text is.
        #[test]
        fn no_filter_leaves_every_line_untouched() {
            let doc = parse_hocr_to_internal_document(ELEVATIONS_PAGE_HOCR);
            assert_eq!(doc.elements.len(), 1);
            assert!(doc.elements[0].text.contains("OWATS DNDEVET OPMENT"));
        }
    }
}
