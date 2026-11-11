use super::document::LineBox;
use super::native_word_coverage::{
    INK_PROOF_DPI, InkProofLimits, glyph_ink_in, is_iterator_word, native_box, ocr_word_in_page_points,
    upright_media_box,
};
use crate::types::internal::{ElementKind, InternalElement};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};
use xberg_native_pdf::{PdfDocument, geometry::Rect};

/// The fewest decodable layer words the read must locate before its agreement with the layer says
/// anything about the page.
const MIN_LOCATED_WORDS: usize = 3;
/// The longest layer token that is a word rather than a run-together string.
const MAX_TOKEN_CHARS: usize = 48;
/// The most an aspect ratio of the read raster may differ from the page's before the raster is
/// not the whole page.
const MAX_ASPECT_DRIFT: f32 = 0.01;
const RECOVERY_BACKEND: &str = "layer-token-recovery";

/// A word placed on the page, in the points of the read's frame.
#[derive(Clone, Debug)]
pub(super) struct PlacedWord {
    pub(super) text: String,
    pub(super) place: LineBox,
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

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut row: Vec<usize> = (0..=right.len()).collect();
    for (at, a) in left.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = at + 1;
        for (to, b) in right.iter().enumerate() {
            let above = row[to + 1];
            row[to + 1] = (diagonal + usize::from(a != *b)).min(above + 1).min(row[to] + 1);
            diagonal = above;
        }
    }
    row[right.len()]
}

/// Whether the read word is a misreading of the layer word: one edit for a token of three
/// characters or more, two for a token of five or more (an identifier with a letter and a digit
/// swapped, an edge character dropped or added). A longer distance is a different word, and the
/// read keeps it. Words that differ only in punctuation are the same word and stay as read.
fn is_near_miss(read: &str, layer: &str) -> bool {
    if alnum_core(read) == alnum_core(layer) {
        return false;
    }
    let length = layer.chars().count();
    match edit_distance(read, layer) {
        1 => length >= 3,
        2 => length >= 5,
        _ => false,
    }
}

fn overlaps(left: &LineBox, right: &LineBox) -> bool {
    left.left < right.right && right.left < left.right && left.top < right.bottom && right.top < left.bottom
}

/// Decide what the layer adds to a read. A layer word is acted on only where the read and the
/// layer agree about the page: the words the read locates at a layer word's own place must equal
/// the layer or be near misses of it for at least half of them. Then a decodable layer word that
/// the page shows (`shows_ink`) and whose place holds exactly one read word replaces that word
/// when it is a near miss, and one whose place holds no read word at all is added. A place that
/// holds a different word, several words, or a word only partly over it is left to the read.
pub(super) fn plan(layer: &[PlacedWord], read: &[PlacedWord], shows_ink: &mut dyn FnMut(&LineBox) -> bool) -> Plan {
    let mut corrections: Vec<(usize, usize)> = Vec::new();
    let mut additions: Vec<usize> = Vec::new();
    let (mut located, mut agreeing) = (0usize, 0usize);
    for (index, word) in layer.iter().enumerate().filter(|(_, word)| is_decodable(&word.text)) {
        let over: Vec<usize> = (0..read.len())
            .filter(|&at| word.place.owns(&read[at].place) || read[at].place.owns(&word.place))
            .collect();
        match over.as_slice() {
            [] if !read.iter().any(|other| overlaps(&other.place, &word.place)) => additions.push(index),
            [only] if word.place.owns(&read[*only].place) && read[*only].place.owns(&word.place) => {
                located += 1;
                if read[*only].text == word.text {
                    agreeing += 1;
                } else if is_near_miss(&read[*only].text, &word.text) {
                    agreeing += 1;
                    corrections.push((*only, index));
                }
            }
            _ => {}
        }
    }
    if located < MIN_LOCATED_WORDS || agreeing * 2 < located {
        return Plan::default();
    }

    let claimed = |read_at: usize| corrections.iter().filter(|(at, _)| *at == read_at).count();
    let consistent: Vec<(usize, usize)> = corrections
        .iter()
        .copied()
        .filter(|(read_at, layer_at)| {
            let text = &read[*read_at].text;
            claimed(*read_at) == 1
                && read
                    .iter()
                    .enumerate()
                    .filter(|(_, word)| word.text == *text)
                    .all(|(other, _)| {
                        corrections
                            .iter()
                            .any(|(at, owner)| *at == other && layer[*owner].text == layer[*layer_at].text)
                    })
        })
        .collect();

    let additions: Vec<usize> = additions
        .into_iter()
        .filter(|&index| {
            !layer
                .iter()
                .enumerate()
                .any(|(other, word)| other != index && overlaps(&word.place, &layer[index].place))
        })
        .collect();

    let corrections: Vec<(usize, String)> = consistent
        .into_iter()
        .filter(|(_, layer_at)| shows_ink(&layer[*layer_at].place))
        .map(|(read_at, layer_at)| (read_at, layer[layer_at].text.clone()))
        .collect();
    let additions = additions
        .into_iter()
        .filter(|&index| shows_ink(&layer[index].place))
        .collect();
    Plan { corrections, additions }
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_alphanumeric())
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

/// Recover into `read` the layer tokens of one page that a whole-page read of it lost: a layer
/// word the read misread by a near miss takes the layer's spelling, and a layer word the page
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
        })
        .collect();
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
    let plan = plan(&layer, &read_words, &mut shows_ink);
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
    apply(read, &corrections, &additions)
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

fn apply(read: &mut ExtractedDocument, corrections: &[(String, String)], additions: &[AddedWord]) -> bool {
    let mut changed = false;
    for (from, to) in corrections {
        changed |= correct(read, from, to);
    }
    let page = read
        .ocr_elements
        .as_deref()
        .and_then(|elements| elements.first())
        .map_or(1, |element| element.page_number);
    let mut ordered: Vec<&AddedWord> = additions.iter().collect();
    ordered.sort_by_key(|(_, (left, top, _, _))| (*top, *left));
    for (text, place) in ordered {
        changed |= add(read, text, *place, page);
    }
    changed
}

/// Rewrite every whole-token occurrence of `from` in every form the read holds, when the text
/// holds it exactly as often as the read has words of it.
fn correct(read: &mut ExtractedDocument, from: &str, to: &str) -> bool {
    let words = read.ocr_elements.as_deref().map_or(0, |elements| {
        elements
            .iter()
            .filter(|element| element.level == OcrElementLevel::Word && element.text.trim() == from)
            .count()
    });
    if words == 0 || count_token(&read.content, from) != words {
        return false;
    }
    replace_token(&mut read.content, from, to);
    for element in read.ocr_elements.iter_mut().flatten() {
        replace_token(&mut element.text, from, to);
    }
    for table in &mut read.tables {
        replace_token(&mut table.markdown, from, to);
        for cell in table.cells.iter_mut().flatten() {
            replace_token(cell, from, to);
        }
        for cell in table.columns.iter_mut().flatten() {
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

/// Add `text` at `place` to the read's text, words and structure, when the text places it.
fn add(read: &mut ExtractedDocument, text: &str, (left, top, width, height): (u32, u32, u32, u32), page: u32) -> bool {
    let area = PixelBox {
        left: i64::from(left),
        top: i64::from(top),
        right: i64::from(left + width),
        bottom: i64::from(top + height),
    };
    if !outside_tables(read, &area) {
        return false;
    }
    let words: Vec<(&str, PixelBox)> = read
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word)
        .filter_map(|element| Some((element.text.trim(), PixelBox::of(element)?)))
        .collect();
    let Some(insertion) = content_insertion(&read.content, &words, &area) else {
        return false;
    };
    insertion.apply(&mut read.content, text);
    let geometry = OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    };
    if let Some(elements) = read.ocr_elements.as_mut() {
        elements.push(OcrElement {
            text: text.to_owned(),
            geometry: geometry.clone(),
            confidence: OcrConfidence {
                detection: None,
                recognition: 1.0,
            },
            level: OcrElementLevel::Word,
            rotation: None,
            page_number: page,
            parent_id: None,
            backend_metadata: std::collections::HashMap::from([(
                "backend".to_owned(),
                serde_json::Value::from(RECOVERY_BACKEND),
            )]),
        });
    }
    if let Some(document) = read.ocr_internal_document.as_mut() {
        let mut element = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            text.to_owned(),
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
        document.elements.push(element);
    }
    true
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
fn content_insertion(content: &str, words: &[(&str, PixelBox)], area: &PixelBox) -> Option<Insertion> {
    let once = |text: &str| {
        let mut found = words.iter().filter(|(word, _)| *word == text);
        match (found.next(), found.next()) {
            (Some((_, place)), None) if count_token(content, text) == 1 => Some(*place),
            _ => None,
        }
    };
    let mut blocks: Vec<(std::ops::Range<usize>, Option<PixelBox>)> = Vec::new();
    let mut start = 0usize;
    for part in content.split("\n\n") {
        let range = start..start + part.len();
        start = range.end + 2;
        let place = part
            .split_whitespace()
            .filter(|word| word.chars().any(char::is_alphanumeric))
            .filter_map(once)
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
            .filter_map(|word| Some((word, once(word)?)))
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
