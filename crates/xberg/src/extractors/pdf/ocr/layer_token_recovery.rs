use super::document::LineBox;
use super::native_word_coverage::{
    INK_PROOF_DPI, InkProofLimits, glyph_ink_in, is_iterator_word, native_box, ocr_word_in_page_points,
    upright_media_box,
};
use crate::types::internal::{ElementKind, InternalElement};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};
use std::collections::HashMap;
use xberg_native_pdf::{PdfDocument, geometry::Rect};

/// The fewest decodable layer words the read must locate before its agreement with the layer says
/// anything about the page.
const MIN_LOCATED_WORDS: usize = 3;
/// The longest layer token that is a word rather than a run-together string.
const MAX_TOKEN_CHARS: usize = 48;
/// The most the aspect ratio of the read raster may differ from the page's before the raster is
/// not the whole page.
const MAX_ASPECT_DRIFT: f32 = 0.01;
/// A read word is corrected only below this recognition confidence: the default
/// `min_ocr_mean_confidence` (75 on Tesseract's scale), under which the page itself would not be
/// accepted. A word the engine is sure of stays as read, whatever the layer says.
const MAX_CORRECTED_CONFIDENCE: f64 = 0.75;
/// Layer and read words beyond these counts are not compared: the comparison indexes the read, but
/// a page this dense is not one whose layer is checked.
const MAX_LAYER_WORDS: usize = 4096;
const MAX_READ_WORDS: usize = 8192;
/// The most words recovery adds to a page, and the most it corrects; a page that wants more is not
/// one a few words are lost on.
const MAX_ADDITIONS: usize = 64;
const MAX_CORRECTIONS: usize = 256;
/// Words between two polls of the cancellation check.
const CANCEL_POLL_INTERVAL: usize = 64;
const RECOVERY_BACKEND: &str = "layer-token-recovery";

/// Characters an OCR engine takes for one another, as unordered pairs. Every pair holds a digit
/// or the letter pair `l`/`I`; a pair that differs only in case is left out, since it would let a
/// lowercase layer rewrite a correct `US` or `Polish`.
const CONFUSABLE_PAIRS: [(char, char); 14] = [
    ('0', 'O'),
    ('0', 'o'),
    ('1', 'l'),
    ('1', 'I'),
    ('l', 'I'),
    ('5', 'S'),
    ('5', 's'),
    ('8', 'B'),
    ('2', 'Z'),
    ('2', 'z'),
    ('9', 'O'),
    ('9', 'g'),
    ('9', 'q'),
    ('6', 'b'),
];
/// Multi-character shapes read as one character, collapsed before two tokens are compared.
const CONFUSABLE_RUNS: [(&str, &str); 2] = [("rn", "m"), ("vv", "w")];

/// A word placed on the page, in the points of the read's frame.
#[derive(Clone, Debug)]
pub(super) struct PlacedWord {
    pub(super) text: String,
    pub(super) place: LineBox,
    /// Recognition confidence of a read word, 0 to 1; unused for a layer word.
    pub(super) confidence: f64,
}

/// What recovery changes in a read.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Plan {
    /// Read words the layer corrects, by index into the read words.
    pub(super) corrections: Vec<(usize, String)>,
    /// Layer words the read lacks, by index into the layer words.
    pub(super) additions: Vec<usize>,
}

fn is_decodable(token: &str) -> bool {
    token.chars().count() <= MAX_TOKEN_CHARS
        && token.chars().any(char::is_alphanumeric)
        && token.chars().all(|character| {
            !character.is_control()
                && character != super::super::unmapped_glyphs::LOST_GLYPH_MARKER
                && !matches!(character, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{10FFFF}')
        })
}

fn alnum_core(token: &str) -> String {
    token.chars().filter(|character| character.is_alphanumeric()).collect()
}

fn collapse_runs(token: &str) -> String {
    CONFUSABLE_RUNS
        .iter()
        .fold(token.to_owned(), |text, (run, single)| text.replace(run, single))
}

fn confusable(left: char, right: char) -> bool {
    left == right
        || CONFUSABLE_PAIRS
            .iter()
            .any(|&(a, b)| (a == left && b == right) || (a == right && b == left))
}

/// Whether the read word is an OCR misreading of the layer word. Only these count: the same
/// characters with confusable ones swapped (`0`/`O`, `1`/`l`, `rn`/`m`, ...), and an edge
/// character the read dropped on an identifier that mixes letters and digits (the layer is then the longer token; a layer that lost a glyph never overwrites the read). A layer
/// token with anything beyond ASCII (a ligature or other presentation form, an accent), without a
/// letter (a number whose digits a shifted mapping could change), or that differs by any other
/// insertion, deletion or substitution is a different word and the read keeps its own.
fn is_misreading(read: &str, layer: &str) -> bool {
    if !read.is_ascii() || !layer.is_ascii() || alnum_core(read) == alnum_core(layer) {
        return false;
    }
    if !layer.chars().any(|character| character.is_ascii_alphabetic()) {
        return false;
    }
    let (collapsed_read, collapsed_layer) = (collapse_runs(read), collapse_runs(layer));
    if collapsed_read.chars().count() == collapsed_layer.chars().count()
        && collapsed_read
            .chars()
            .zip(collapsed_layer.chars())
            .all(|(a, b)| confusable(a, b))
    {
        return true;
    }
    is_identifier(layer) && layer.len() == read.len() + 1 && (layer[1..] == *read || layer[..layer.len() - 1] == *read)
}

/// A token of letters and digits, four or more characters, with both kinds: an identifier or a
/// code, which carries no dictionary word the read could have guessed.
fn is_identifier(token: &str) -> bool {
    token.chars().count() >= 4
        && token.chars().any(|character| character.is_ascii_digit())
        && token.chars().any(|character| character.is_ascii_alphabetic())
        && token.chars().all(|character| character.is_ascii_alphanumeric())
}

fn overlaps(left: &LineBox, right: &LineBox) -> bool {
    left.left < right.right && right.left < left.right && left.top < right.bottom && right.top < left.bottom
}

/// Words ordered by their top edge, so the words near a place are found without scanning all of them.
struct PlaceIndex {
    order: Vec<usize>,
    tops: Vec<f32>,
    max_height: f32,
}

impl PlaceIndex {
    fn new(words: &[PlacedWord]) -> Self {
        let mut order: Vec<usize> = (0..words.len()).collect();
        order.sort_by(|&a, &b| words[a].place.top.total_cmp(&words[b].place.top));
        let tops = order.iter().map(|&at| words[at].place.top).collect();
        let max_height = words
            .iter()
            .map(|word| word.place.bottom - word.place.top)
            .fold(0.0, f32::max);
        Self {
            order,
            tops,
            max_height,
        }
    }

    /// Indices of the words whose box may reach into the band `top..bottom`.
    fn near(&self, top: f32, bottom: f32) -> impl Iterator<Item = usize> + '_ {
        let start = self.tops.partition_point(|value| *value < top - self.max_height);
        self.order[start..]
            .iter()
            .zip(&self.tops[start..])
            .take_while(move |(_, value)| **value < bottom)
            .map(|(at, _)| *at)
    }
}

/// Decide what the layer adds to a read. A layer word is acted on only where the read and the
/// layer agree about the page: the words the read locates at a layer word's own place must equal
/// the layer or be misreadings of it for at least half of them. Then a decodable layer word that
/// the page shows (`shows_ink`) and whose place holds exactly one read word replaces that word
/// when the word is a low-confidence misreading of it, and one whose place holds no read word at
/// all, with no copy of its text beside it, is added. A place that holds a different word,
/// several words, or a word only partly over it is left to the read. `cancelled` is polled
/// while the words are compared; a cancelled plan is empty.
pub(super) fn plan(
    layer: &[PlacedWord],
    read: &[PlacedWord],
    shows_ink: &mut dyn FnMut(&LineBox) -> bool,
    cancelled: &dyn Fn() -> bool,
) -> Plan {
    if layer.len() > MAX_LAYER_WORDS || read.len() > MAX_READ_WORDS {
        return Plan::default();
    }
    let read_index = PlaceIndex::new(read);
    let layer_index = PlaceIndex::new(layer);
    let mut corrections: Vec<(usize, usize)> = Vec::new();
    let mut additions: Vec<usize> = Vec::new();
    let (mut located, mut agreeing) = (0usize, 0usize);
    for (index, word) in layer.iter().enumerate().filter(|(_, word)| is_decodable(&word.text)) {
        if index % CANCEL_POLL_INTERVAL == 0 && cancelled() {
            return Plan::default();
        }
        let near: Vec<usize> = read_index.near(word.place.top, word.place.bottom).collect();
        let over: Vec<usize> = near
            .iter()
            .copied()
            .filter(|&at| word.place.owns(&read[at].place) || read[at].place.owns(&word.place))
            .collect();
        match over.as_slice() {
            [] if !near.iter().any(|&at| overlaps(&read[at].place, &word.place)) => additions.push(index),
            [only] if word.place.owns(&read[*only].place) && read[*only].place.owns(&word.place) => {
                located += 1;
                if read[*only].text == word.text {
                    agreeing += 1;
                } else if is_misreading(&read[*only].text, &word.text) {
                    agreeing += 1;
                    if read[*only].confidence < MAX_CORRECTED_CONFIDENCE {
                        corrections.push((*only, index));
                    }
                }
            }
            _ => {}
        }
    }
    if located < MIN_LOCATED_WORDS || agreeing * 2 < located {
        return Plan::default();
    }

    let mut claims: HashMap<usize, Vec<usize>> = HashMap::new();
    for (read_at, layer_at) in &corrections {
        claims.entry(*read_at).or_default().push(*layer_at);
    }
    let mut by_text: HashMap<&str, Vec<usize>> = HashMap::new();
    for (at, word) in read.iter().enumerate() {
        by_text.entry(word.text.as_str()).or_default().push(at);
    }
    let consistent: Vec<(usize, usize)> = corrections
        .iter()
        .copied()
        .filter(|(read_at, layer_at)| {
            claims[read_at].len() == 1
                && by_text[read[*read_at].text.as_str()].iter().all(|other| {
                    claims
                        .get(other)
                        .is_some_and(|owners| owners.len() == 1 && layer[owners[0]].text == layer[*layer_at].text)
                })
        })
        .collect();

    let additions: Vec<usize> = additions
        .into_iter()
        .filter(|&index| {
            let place = &layer[index].place;
            !layer_index
                .near(place.top, place.bottom)
                .any(|other| other != index && overlaps(&layer[other].place, place))
                && !has_copy_nearby(&layer[index], read, &read_index)
        })
        .collect();
    let mut corrections = Vec::new();
    for (read_at, layer_at) in consistent {
        if cancelled() {
            return Plan::default();
        }
        if shows_ink(&layer[layer_at].place) {
            corrections.push((read_at, layer[layer_at].text.clone()));
        }
    }
    let mut shown = Vec::new();
    for index in additions {
        if cancelled() {
            return Plan::default();
        }
        if shows_ink(&layer[index].place) {
            shown.push(index);
        }
    }
    if corrections.len() > MAX_CORRECTIONS {
        corrections.clear();
    }
    if shown.len() > MAX_ADDITIONS {
        shown.clear();
    }
    Plan {
        corrections,
        additions: shown,
    }
}

/// Whether a read word near the layer word already holds its text, whole or run into a longer
/// word: the read placed that word a little off, and adding the layer word would write it twice.
fn has_copy_nearby(word: &PlacedWord, read: &[PlacedWord], index: &PlaceIndex) -> bool {
    const WINDOW_HEIGHTS: f32 = 2.0;
    let core = alnum_core(&word.text);
    let height = word.place.bottom - word.place.top;
    let reach = height * WINDOW_HEIGHTS;
    index.near(word.place.top - reach, word.place.bottom + reach).any(|at| {
        let other = &read[at];
        let near_horizontally = other.place.right > word.place.left - reach * 3.0
            && other.place.left < word.place.right + reach * 3.0
            && other.place.bottom > word.place.top - reach
            && other.place.top < word.place.bottom + reach;
        let other_core = alnum_core(&other.text);
        near_horizontally && (other_core == core || (core.chars().count() >= 3 && other_core.contains(&core)))
    })
}

/// Whether `character` ends a token: the line, a space or a table cell edge. Markup characters,
/// URL punctuation and the like do not, so a token inside them is never the whole read word.
fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| character.is_whitespace() || character == '|')
}

/// Replace every whole-token occurrence of `from`; returns the number replaced.
fn replace_token(text: &mut String, from: &str, to: &str) -> usize {
    let mut rebuilt = String::with_capacity(text.len());
    let (mut tail, mut count) = (0usize, 0usize);
    for (at, _) in text.match_indices(from) {
        let end = at + from.len();
        if at < tail || !is_boundary(text[..at].chars().next_back()) || !is_boundary(text[end..].chars().next()) {
            continue;
        }
        rebuilt.push_str(&text[tail..at]);
        rebuilt.push_str(to);
        tail = end;
        count += 1;
    }
    if count > 0 {
        rebuilt.push_str(&text[tail..]);
        *text = rebuilt;
    }
    count
}

fn count_token(text: &str, from: &str) -> usize {
    text.match_indices(from)
        .filter(|(at, _)| {
            is_boundary(text[..*at].chars().next_back()) && is_boundary(text[at + from.len()..].chars().next())
        })
        .count()
}

/// A layer word to add, with its raster box as left, top, width and height.
type AddedWord = (String, (u32, u32, u32, u32));

struct Frame {
    raster: (u32, u32),
    media: Rect,
}

impl Frame {
    fn to_raster(&self, place: &LineBox) -> Option<(u32, u32, u32, u32)> {
        let scale_x = self.media.width / self.raster.0 as f32;
        let scale_y = self.media.height / self.raster.1 as f32;
        let clamp = |value: f32, limit: u32| value.round().clamp(0.0, limit as f32) as u32;
        let left = clamp(place.left / scale_x, self.raster.0);
        let right = clamp(place.right / scale_x, self.raster.0);
        let top = clamp(self.raster.1 as f32 - place.bottom / scale_y, self.raster.1);
        let bottom = clamp(self.raster.1 as f32 - place.top / scale_y, self.raster.1);
        (right > left && bottom > top).then_some((left, top, right - left, bottom - top))
    }
}

/// Recover into `read` the layer tokens of one page that a whole-page read of it lost, when the read
/// as it stands already confirms the layer: the page's later decision to put the layer's lines
/// first rests on the same test, so words copied from the layer must not be what passes it. A layer
/// word the read misread with low confidence takes the layer's spelling, and a layer word the page
/// shows where the read has no word is added. Returns whether the read changed.
pub(super) fn recover_page(
    pdf: &PdfDocument,
    page_index: usize,
    read: &mut ExtractedDocument,
    limits: InkProofLimits<'_>,
) -> bool {
    let Some(raster) = super::document::processed_ocr_layout_dimensions(&read.metadata) else {
        return false;
    };
    let Some(media) = upright_media_box(pdf, page_index) else {
        return false;
    };
    let aspect = |width: f32, height: f32| width / height;
    if (aspect(raster.0 as f32, raster.1 as f32) / aspect(media.width, media.height) - 1.0).abs() > MAX_ASPECT_DRIFT {
        return false;
    }
    let Ok(lines) = pdf.extract_text_lines(page_index) else {
        return false;
    };
    let layer: Vec<PlacedWord> = lines
        .iter()
        .flat_map(|line| &line.words)
        .map(|word| PlacedWord {
            text: word.text.clone(),
            place: native_box(word.bbox),
            confidence: 1.0,
        })
        .collect();
    let decodable_layer = layer
        .iter()
        .filter(|word| is_decodable(&word.text))
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if !super::document::ocr_confirms_layer(&decodable_layer, &read.content, limits.thresholds) {
        return false;
    }
    let Some(elements) = read.ocr_elements.as_deref() else {
        return false;
    };
    let word_indices: Vec<usize> = elements
        .iter()
        .enumerate()
        .filter(|(_, element)| element.level == OcrElementLevel::Word)
        .map(|(at, _)| at)
        .collect();
    if word_indices.is_empty() || !word_indices.iter().all(|&at| is_iterator_word(&elements[at])) {
        return false;
    }
    let Some(read_words) = word_indices
        .iter()
        .map(|&at| {
            ocr_word_in_page_points(&elements[at], raster, media).map(|place| PlacedWord {
                text: elements[at].text.trim().to_owned(),
                place,
                confidence: elements[at].confidence.recognition,
            })
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };

    let mut rasters = None;
    let mut rendered = false;
    let scale = INK_PROOF_DPI as f32 / 72.0;
    let mut shows_ink = |place: &LineBox| {
        if !rendered {
            rendered = true;
            rasters = xberg_native_pdf::rendering::render_text_ink_rasters(
                pdf,
                page_index,
                INK_PROOF_DPI,
                limits.max_working_bytes,
                limits.cancelled,
            )
            .ok()
            .flatten();
        }
        rasters
            .as_ref()
            .is_some_and(|rasters| glyph_ink_in(rasters, place, media, scale))
    };
    let plan = plan(&layer, &read_words, &mut shows_ink, limits.cancelled);
    if plan.corrections.is_empty() && plan.additions.is_empty() {
        return false;
    }

    let frame = Frame { raster, media };
    let corrections: Vec<(String, String)> = plan
        .corrections
        .iter()
        .map(|(at, text)| (read_words[*at].text.clone(), text.clone()))
        .collect();
    let additions: Vec<AddedWord> = plan
        .additions
        .iter()
        .filter_map(|&at| Some((layer[at].text.clone(), frame.to_raster(&layer[at].place)?)))
        .collect();
    apply(read, &corrections, &additions, limits.cancelled)
}

/// Whether every table of the read carries a box that `area` does not touch.
fn outside_tables(read: &ExtractedDocument, area: &PixelBox) -> bool {
    read.tables.iter().all(|table| {
        table.bounding_box.is_some_and(|bbox| {
            area.right as f64 <= bbox.x0
                || area.left as f64 >= bbox.x1
                || area.bottom as f64 <= bbox.y0
                || area.top as f64 >= bbox.y1
        })
    })
}

fn apply(
    read: &mut ExtractedDocument,
    corrections: &[(String, String)],
    additions: &[AddedWord],
    cancelled: &dyn Fn() -> bool,
) -> bool {
    let mut changed = false;
    for (from, to) in corrections {
        if cancelled() {
            return changed;
        }
        changed |= correct(read, from, to);
    }
    let page = read
        .ocr_elements
        .as_deref()
        .and_then(|elements| elements.first())
        .map_or(1, |element| element.page_number);
    let mut ordered: Vec<&AddedWord> = additions.iter().collect();
    ordered.sort_by_key(|(_, (left, top, _, _))| (*top, *left));
    let mut text = PageText::of(read);
    for (token, place) in ordered {
        if cancelled() {
            break;
        }
        changed |= add(read, &mut text, token, *place, page);
    }
    changed
}

/// How many whole tokens of `from` the read's every surface holds, or `None` when the surfaces
/// disagree about it, which means the token is not the plain word the OCR words say it is.
fn surfaces_agree(read: &ExtractedDocument, from: &str) -> Option<usize> {
    let elements = read.ocr_elements.as_deref().unwrap_or_default();
    let words = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word && element.text.trim() == from)
        .count();
    if words == 0 || count_token(&read.content, from) != words {
        return None;
    }
    for level in [OcrElementLevel::Line, OcrElementLevel::Block, OcrElementLevel::Page] {
        let held: usize = elements
            .iter()
            .filter(|element| element.level == level)
            .map(|element| count_token(&element.text, from))
            .sum();
        if elements.iter().any(|element| element.level == level) && held != words {
            return None;
        }
    }
    let mut in_tables = 0usize;
    for table in &read.tables {
        let cells: usize = table.cells.iter().flatten().map(|cell| count_token(cell, from)).sum();
        if count_token(&table.markdown, from) != cells {
            return None;
        }
        in_tables += cells;
    }
    if let Some(document) = read.ocr_internal_document.as_ref() {
        let outside: usize = document
            .elements
            .iter()
            .map(|element| count_token(&element.text, from))
            .sum();
        if outside + in_tables != words {
            return None;
        }
        for table in &document.tables {
            let cells: usize = table.cells.iter().flatten().map(|cell| count_token(cell, from)).sum();
            if count_token(&table.markdown, from) != cells {
                return None;
            }
        }
    }
    Some(words)
}

/// Rewrite every whole-token occurrence of `from` in every form the read holds, when each form
/// holds it exactly as often as the read has words of it.
fn correct(read: &mut ExtractedDocument, from: &str, to: &str) -> bool {
    if surfaces_agree(read, from).is_none() {
        return false;
    }
    replace_token(&mut read.content, from, to);
    for element in read.ocr_elements.iter_mut().flatten() {
        replace_token(&mut element.text, from, to);
    }
    for table in &mut read.tables {
        replace_token(&mut table.markdown, from, to);
        for cell in table
            .cells
            .iter_mut()
            .flatten()
            .chain(table.columns.iter_mut().flatten())
        {
            replace_token(cell, from, to);
        }
    }
    if let Some(document) = read.ocr_internal_document.as_mut() {
        for element in &mut document.elements {
            replace_token(&mut element.text, from, to);
        }
        for element in document.prebuilt_ocr_elements.iter_mut().flatten() {
            replace_token(&mut element.text, from, to);
        }
        for table in &mut document.tables {
            replace_token(&mut table.markdown, from, to);
            for cell in table.cells.iter_mut().flatten() {
                replace_token(cell, from, to);
            }
        }
    }
    true
}

/// The read's words and tokens, counted once per page and kept current as words are added.
struct PageText {
    boxes: HashMap<String, Vec<PixelBox>>,
    tokens: HashMap<String, usize>,
}

impl PageText {
    fn of(read: &ExtractedDocument) -> Self {
        let mut boxes: HashMap<String, Vec<PixelBox>> = HashMap::new();
        for element in read.ocr_elements.iter().flatten() {
            if element.level == OcrElementLevel::Word
                && let Some(place) = PixelBox::of(element)
            {
                boxes.entry(element.text.trim().to_owned()).or_default().push(place);
            }
        }
        let mut tokens: HashMap<String, usize> = HashMap::new();
        for token in read.content.split_whitespace() {
            *tokens.entry(token.to_owned()).or_default() += 1;
        }
        Self { boxes, tokens }
    }

    /// The place of a word the read holds once, as one token of its text.
    fn once(&self, text: &str) -> Option<PixelBox> {
        match (self.boxes.get(text).map(Vec::as_slice), self.tokens.get(text)) {
            (Some([place]), Some(1)) => Some(*place),
            _ => None,
        }
    }

    fn insert(&mut self, text: &str, place: PixelBox) {
        self.boxes.entry(text.to_owned()).or_default().push(place);
        *self.tokens.entry(text.to_owned()).or_default() += 1;
    }
}

/// Add `text` at `place` to the read's text, words and structure, when the text places it.
fn add(
    read: &mut ExtractedDocument,
    text: &mut PageText,
    token: &str,
    (left, top, width, height): (u32, u32, u32, u32),
    page: u32,
) -> bool {
    let area = PixelBox {
        left: i64::from(left),
        top: i64::from(top),
        right: i64::from(left + width),
        bottom: i64::from(top + height),
    };
    if !outside_tables(read, &area) {
        return false;
    }
    let Some(insertion) = content_insertion(&read.content, text, &area) else {
        return false;
    };
    insertion.apply(&mut read.content, token);
    text.insert(token, area);
    let geometry = OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    };
    if let Some(elements) = read.ocr_elements.as_mut() {
        elements.push(OcrElement {
            text: token.to_owned(),
            geometry: geometry.clone(),
            confidence: OcrConfidence {
                detection: None,
                recognition: 1.0,
            },
            level: OcrElementLevel::Word,
            rotation: None,
            page_number: page,
            parent_id: None,
            backend_metadata: HashMap::from([("backend".to_owned(), serde_json::Value::from(RECOVERY_BACKEND))]),
        });
    }
    if let Some(document) = read.ocr_internal_document.as_mut() {
        let mut element = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            token.to_owned(),
            0,
        )
        .with_page(page)
        .with_index(u32::try_from(document.elements.len()).unwrap_or(u32::MAX));
        element.bbox = Some(crate::types::BoundingBox {
            x0: area.left as f64,
            y0: area.top as f64,
            x1: area.right as f64,
            y1: area.bottom as f64,
        });
        element.ocr_geometry = Some(geometry);
        let position = reading_position(&document.elements, &area);
        document.elements.insert(position, element);
    }
    true
}

/// Where in the document's elements a block at `area` reads: right after the last element, in the
/// document's own order, that ends above it, or first when none does. Boxes are in the same raster pixels.
fn reading_position(elements: &[InternalElement], area: &PixelBox) -> usize {
    elements
        .iter()
        .enumerate()
        .filter_map(|(at, element)| Some((at, element.bbox?)))
        .filter(|(_, bbox)| bbox.y1 <= area.top as f64)
        .map(|(at, _)| at)
        .max()
        .map_or(0, |at| at + 1)
}

/// A box in the raster pixels of the read, `top` above `bottom`.
#[derive(Clone, Copy, Debug)]
struct PixelBox {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl PixelBox {
    fn of(element: &OcrElement) -> Option<Self> {
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = element.geometry
        else {
            return None;
        };
        Some(Self {
            left: i64::from(left),
            top: i64::from(top),
            right: i64::from(left) + i64::from(width),
            bottom: i64::from(top) + i64::from(height),
        })
    }

    fn height(&self) -> i64 {
        self.bottom - self.top
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// Whether the two boxes share at least half of the shorter height.
    fn shares_line_with(&self, other: &Self) -> bool {
        let shared = self.bottom.min(other.bottom) - self.top.max(other.top);
        shared * 2 >= self.height().min(other.height()) && shared > 0
    }
}

/// The widest gap, in heights of the added word, between the added word and a read word that
/// still makes them neighbours on one line.
const MAX_LINE_GAP_HEIGHTS: i64 = 3;
/// Characters of Markdown emphasis, links and code; a word carrying one is not a neighbour to
/// insert beside, since the insertion could land inside the span.
const MARKUP_CHARACTERS: [char; 8] = ['*', '_', '[', ']', '(', ')', '`', '<'];

enum Insertion {
    /// Put the token next to the token at `offset`, after it or before it.
    Beside { offset: usize, after: bool },
    /// Put the token as its own block at `offset`, with the blank line on the given side.
    Block { offset: usize, trailing_break: bool },
}

impl Insertion {
    fn apply(&self, content: &mut String, token: &str) {
        match *self {
            Self::Beside { offset, after: true } => content.insert_str(offset, &format!(" {token}")),
            Self::Beside { offset, after: false } => content.insert_str(offset, &format!("{token} ")),
            Self::Block {
                offset,
                trailing_break: true,
            } => content.insert_str(offset, &format!("{token}\n\n")),
            Self::Block {
                offset,
                trailing_break: false,
            } => content.insert_str(offset, &format!("\n\n{token}")),
        }
    }
}

/// Where the read's text reads `area` in its own order, or `None` when that order is not
/// established. A block of the text has a place when one of its tokens is a word the read holds
/// once; the blocks that have a place must run down the page, or the page has columns whose order
/// the vertical position cannot place a word in. The word lands beside a word of its own line when
/// there is one within a few word heights, and otherwise as a block of its own between the blocks
/// above and below it, with no unplaced block between those two.
fn content_insertion(content: &str, text: &PageText, area: &PixelBox) -> Option<Insertion> {
    let mut blocks: Vec<(std::ops::Range<usize>, Option<PixelBox>)> = Vec::new();
    let mut start = 0usize;
    for part in content.split("\n\n") {
        let range = start..start + part.len();
        start = range.end + 2;
        let place = part
            .split_whitespace()
            .filter(|word| word.chars().any(char::is_alphanumeric))
            .filter_map(|word| text.once(word))
            .reduce(|left, right| left.union(&right));
        blocks.push((range, place));
    }
    let placed: Vec<(usize, PixelBox)> = blocks
        .iter()
        .enumerate()
        .filter_map(|(at, (_, place))| Some((at, (*place)?)))
        .collect();
    if placed.is_empty() || placed.windows(2).any(|pair| pair[1].1.top < pair[0].1.top) {
        return None;
    }

    if let Some((at, _)) = placed.iter().find(|(_, band)| band.shares_line_with(area)) {
        let reach = area.height() * MAX_LINE_GAP_HEIGHTS;
        let range = &blocks[*at].0;
        let (word, place, _) = content[range.clone()]
            .split_whitespace()
            .filter(|word| !word.contains(MARKUP_CHARACTERS))
            .filter_map(|word| Some((word, text.once(word)?)))
            .filter(|(_, place)| place.shares_line_with(area))
            .map(|(word, place)| {
                let gap = if place.right <= area.left {
                    area.left - place.right
                } else {
                    place.left - area.right
                };
                (word, place, gap)
            })
            .filter(|(_, _, gap)| (0..=reach).contains(gap))
            .min_by_key(|(_, _, gap)| *gap)?;
        let in_text = content[range.clone()].find(word)? + range.start;
        let line_start = content[..in_text].rfind('\n').map_or(0, |at| at + 1);
        if content[line_start..].starts_with('|') {
            return None;
        }
        let after = place.right <= area.left;
        let offset = if after { in_text + word.len() } else { in_text };
        return Some(Insertion::Beside { offset, after });
    }

    let above = placed.iter().rev().find(|(_, band)| band.top < area.top);
    let below = placed.iter().find(|(_, band)| band.top > area.top);
    match (above, below) {
        (Some((at, _)), next) if next.is_none_or(|(later, _)| *later == at + 1) => Some(Insertion::Block {
            offset: blocks[*at].0.end,
            trailing_break: false,
        }),
        (None, Some((at, _))) if *at == 0 => Some(Insertion::Block {
            offset: 0,
            trailing_break: true,
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "layer_token_recovery_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "layer_token_recovery_pdf_tests.rs"]
mod pdf_tests;
