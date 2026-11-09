//! Bounded, atomic proof of fragmented hOCR table ownership against TSV source words.

use super::{ocr_table_bbox, sort_ocr_line_word_refs};
use crate::extractors::security::SecurityLimits;
use crate::ocr::hocr_parser::{
    parse_hocr_to_internal_document_with_page_offset_and_stats, parse_hocr_without_proved_table_fragments,
};
use crate::ocr::types::TesseractConfig;
use crate::table_core::{HocrWord, detect_rows, find_row_index};
use crate::types::extraction::BoundingBox;
use crate::types::internal::InternalDocument;
use crate::types::{OcrTable, OcrTableBoundingBox};
use std::collections::HashMap;

#[cfg(test)]
#[path = "fragmented_tables/tests.rs"]
mod tests;

// ~keep This proof is optional. Bound the extra retained input and the row
// searches even when the configured extraction limits are larger.
pub(super) const MAX_FRAGMENTED_HOCR_INPUT_BYTES: usize = 512 * 1024;
pub(super) const MAX_FRAGMENTED_HOCR_WORDS: usize = 1024;
const MAX_FRAGMENTED_HOCR_ALIGNMENTS: usize = 8;

pub(super) fn fragmented_hocr_budget(hocr: &str, tsv: &str, limits: &SecurityLimits) -> Option<usize> {
    let bytes = hocr.len().saturating_add(tsv.len());
    if bytes > MAX_FRAGMENTED_HOCR_INPUT_BYTES.min(limits.max_content_size) {
        return None;
    }
    limits.max_iterations.checked_sub(bytes.saturating_mul(4))
}

fn prepare_fragmented_table_rows<'a>(
    words: &'a mut Vec<HocrWord>,
    bbox: BoundingBox,
    cells: &[Vec<String>],
    row_threshold: f64,
    budget: &mut usize,
) -> Option<(Vec<Vec<&'a HocrWord>>, usize)> {
    if cells.is_empty() || words.len() > MAX_FRAGMENTED_HOCR_WORDS {
        return None;
    }
    words.retain(|word| {
        word.width > 0
            && word.height > 0
            && f64::from(word.left) >= bbox.x0
            && f64::from(word.top) >= bbox.y0
            && f64::from(word.left.saturating_add(word.width)) <= bbox.x1
            && f64::from(word.top.saturating_add(word.height)) <= bbox.y1
    });
    // Both row detection and nearest-row assignment can scan every word for
    // each row. The word cap makes the multiplication safe.
    *budget = budget.checked_sub(2 * words.len() * words.len())?;
    let row_positions = detect_rows(words, row_threshold);
    let alignments = row_positions.len().checked_sub(cells.len())? + 1;
    if alignments > MAX_FRAGMENTED_HOCR_ALIGNMENTS {
        return None;
    }
    let comparison_bytes = words
        .iter()
        .map(|word| word.text.len() + 1)
        .chain(cells.iter().flatten().map(|cell| cell.len() + 1))
        .fold(0usize, usize::saturating_add);
    let comparison_work = comparison_bytes.saturating_mul(alignments);
    if comparison_work > *budget {
        return None;
    }
    let mut rows = vec![Vec::new(); row_positions.len()];
    for word in words.iter() {
        rows[find_row_index(&row_positions, word)?].push(word);
    }
    for row in &mut rows {
        sort_ocr_line_word_refs(row);
    }
    Some((rows, comparison_work))
}

fn prove_fragmented_table_rows_budgeted<'a>(
    words: &'a mut Vec<HocrWord>,
    bbox: BoundingBox,
    cells: &[Vec<String>],
    row_threshold: f64,
    budget: &mut usize,
    allow_dot_leader: bool,
) -> Option<Vec<&'a HocrWord>> {
    let (rows, comparison_work) = prepare_fragmented_table_rows(words, bbox, cells, row_threshold, budget)?;
    *budget = budget.checked_sub(comparison_work)?;
    let matched = unique_table_row_window(&rows, cells, false).or_else(|| {
        if !allow_dot_leader {
            return None;
        }
        *budget = budget.checked_sub(comparison_work)?;
        unique_table_row_window(&rows, cells, true)
    })?;
    Some(matched.iter().flatten().copied().collect())
}

#[cfg(test)]
fn prove_fragmented_table_rows<'a>(
    words: &'a mut Vec<HocrWord>,
    bbox: BoundingBox,
    cells: &[Vec<String>],
    row_threshold: f64,
    mut budget: usize,
    allow_dot_leader: bool,
) -> Option<Vec<&'a HocrWord>> {
    prove_fragmented_table_rows_budgeted(words, bbox, cells, row_threshold, &mut budget, allow_dot_leader)
}

// ~keep A matching window can omit a caption, but ambiguous matches cannot claim any words.
fn unique_table_row_window<'a, 'word>(
    rows: &'a [Vec<&'word HocrWord>],
    cells: &[Vec<String>],
    allow_dot_leader: bool,
) -> Option<&'a [Vec<&'word HocrWord>]> {
    let mut matches = rows.windows(cells.len()).filter(|window| {
        window.iter().zip(cells).all(|(row, cells)| {
            !row.is_empty()
                && row.windows(2).all(|pair| pair[0].left != pair[1].left)
                && (row
                    .iter()
                    .flat_map(|word| word.text.split_whitespace())
                    .eq(cells.iter().flat_map(|cell| cell.split_whitespace()))
                    || allow_dot_leader && row_has_only_extra_dot_leader(row, cells))
        })
    });
    let matched = matches.next()?;
    matches.next().is_none().then_some(matched)
}

fn row_has_only_extra_dot_leader(row: &[&HocrWord], cells: &[String]) -> bool {
    let [label, values @ ..] = cells else {
        return false;
    };
    let label_words = label.split_whitespace();
    let split = label_words.clone().count();
    if split == 0
        || values.len() < 2
        || row.len() != split + 1 + values.len()
        || !row[..split].iter().map(|word| word.text.as_str()).eq(label_words)
        || !row[split + 1..]
            .iter()
            .zip(values)
            .all(|(word, value)| word.text == *value)
    {
        return false;
    }
    let [left, dots, right] = &row[split - 1..=split + 1] else {
        return false;
    };
    const MIN_DOT_LEADER_LENGTH: usize = 3;
    dots.text.len() >= MIN_DOT_LEADER_LENGTH
        && dots.text.bytes().all(|byte| byte == b'.')
        && dots.width > left.height.max(right.height)
        && dots.height.saturating_mul(2) <= left.height.min(right.height)
        && left.left.saturating_add(left.width) < dots.left
        && dots.left.saturating_add(dots.width) < right.left
        && dots.top >= left.top.max(right.top)
        && dots.top.saturating_add(dots.height)
            <= left
                .top
                .saturating_add(left.height)
                .min(right.top.saturating_add(right.height))
}

fn fragmented_table_boxes(
    tables: &[OcrTable],
    page: u32,
    limits: &SecurityLimits,
    budget: &mut usize,
) -> Option<Vec<BoundingBox>> {
    if tables.is_empty() || tables.len() > MAX_FRAGMENTED_HOCR_WORDS {
        return None;
    }
    *budget = budget.checked_sub(tables.len().saturating_mul(tables.len()))?;
    let mut cells = 0usize;
    let mut boxes: Vec<BoundingBox> = Vec::with_capacity(tables.len());
    for table in tables {
        if table.cells.is_empty()
            || table.cells.len() > MAX_FRAGMENTED_HOCR_WORDS
            || table.cells.iter().any(Vec::is_empty)
        {
            return None;
        }
        cells = cells.checked_add(table.cells.iter().map(Vec::len).sum::<usize>())?;
        let cell_bytes = table
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.len() + 1)
            .fold(0usize, usize::saturating_add);
        *budget = budget.checked_sub(cell_bytes)?;
        let bbox = ocr_table_bbox(table)?;
        if table.page_number != page
            || cells > limits.max_table_cells
            || bbox.x0 >= bbox.x1
            || bbox.y0 >= bbox.y1
            || boxes
                .iter()
                .any(|other| bbox.x0 <= other.x1 && other.x0 <= bbox.x1 && bbox.y0 <= other.y1 && other.y0 <= bbox.y1)
        {
            return None;
        }
        boxes.push(bbox);
    }
    Some(boxes)
}

fn fragmented_hocr_attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (_, value) = tag.split_once(name)?;
    let quote = value.as_bytes().first().copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    value[1..].split_once(char::from(quote)).map(|(value, _)| value)
}

fn fragmented_hocr_local_page(tag: &str) -> bool {
    let Some(title) = fragmented_hocr_attribute(tag, "title=") else {
        return false;
    };
    let pages: Vec<_> = title
        .split(';')
        .filter_map(|part| {
            let mut fields = part.split_whitespace();
            (fields.next()? == "ppageno").then(|| fields.collect::<Vec<_>>())
        })
        .collect();
    pages == [vec!["0"]]
}

type FragmentedWordKey = (String, [u32; 4]);

fn fragmented_hocr_word(
    remaining: &str,
    end: usize,
    name: &str,
    budget: &mut usize,
) -> Option<(FragmentedWordKey, usize)> {
    let source_bounds = fragmented_source_bounds(&remaining[..end])?;
    let closing = format!("</{name}>");
    let word_end = remaining[end + 1..].find(&closing)? + end + 1 + closing.len();
    let fragment = format!("<p class=\"ocr_par\"><{}</p>", &remaining[..word_end]);
    *budget = budget.checked_sub(fragment.len().saturating_mul(4))?;
    let parsed = parse_hocr_to_internal_document_with_page_offset_and_stats(&fragment, None, 1).document;
    let [word] = parsed.elements.as_slice() else {
        return None;
    };
    let bbox = word.bbox.as_ref()?;
    let bounds = fragmented_bound_coordinates(bbox)?;
    (bounds == source_bounds).then(|| ((word.text.clone(), bounds), word_end))
}

fn fragmented_source_bounds(tag: &str) -> Option<[u32; 4]> {
    let title = fragmented_hocr_attribute(tag, "title=")?;
    let mut bounds = None;
    for part in title.split(';') {
        let mut fields = part.split_whitespace();
        if fields.next() != Some("bbox") {
            continue;
        }
        if bounds.is_some() {
            return None;
        }
        bounds = Some([
            fields.next()?.parse().ok()?,
            fields.next()?.parse().ok()?,
            fields.next()?.parse().ok()?,
            fields.next()?.parse().ok()?,
        ]);
        if fields.next().is_some() {
            return None;
        }
    }
    bounds
}

fn fragmented_bound_coordinates(bbox: &BoundingBox) -> Option<[u32; 4]> {
    let coordinates = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    if coordinates.iter().any(|coordinate| {
        !coordinate.is_finite() || coordinate.fract() != 0.0 || !(0.0..=f64::from(u32::MAX)).contains(coordinate)
    }) {
        return None;
    }
    Some(coordinates.map(|coordinate| coordinate as u32))
}

struct FragmentedHocrSource<'a> {
    page_seen: bool,
    page_depth: Option<usize>,
    stack: Vec<&'a str>,
    occurrences: HashMap<FragmentedWordKey, usize>,
}

impl<'a> FragmentedHocrSource<'a> {
    fn opening_word_end(&mut self, remaining: &'a str, end: usize, budget: &mut usize) -> Option<usize> {
        let tag = &remaining[..end];
        let name = tag.split_whitespace().next()?;
        let class = fragmented_hocr_attribute(tag, "class=").unwrap_or("");
        if class.split_whitespace().any(|class| class == "ocr_page") {
            if self.page_seen || !fragmented_hocr_local_page(tag) {
                return None;
            }
            self.page_seen = true;
            self.page_depth = Some(self.stack.len() + 1);
        }
        if class.split_whitespace().any(|class| class == "ocrx_word") {
            if self.page_depth.is_none() || self.occurrences.len() >= MAX_FRAGMENTED_HOCR_WORDS {
                return None;
            }
            let (key, word_end) = fragmented_hocr_word(remaining, end, name, budget)?;
            if self.occurrences.insert(key, 1usize).is_some() {
                return None;
            }
            return Some(word_end);
        }
        if !tag.ends_with('/')
            && !name.starts_with(['!', '?'])
            && !matches!(name, "meta" | "link" | "br" | "hr" | "img" | "input")
        {
            self.stack.push(name);
        }
        Some(end + 1)
    }
}

// ~keep Parse each source word through the removal parser itself, so text normalization and
// geometry agree. Count occurrences before its set-based removal can hide duplicates.
fn fragmented_hocr_source_matches(hocr: &str, source: &[HocrWord], budget: &mut usize) -> Option<()> {
    let mut remaining = hocr;
    let mut state = FragmentedHocrSource {
        page_seen: false,
        page_depth: None,
        stack: Vec::new(),
        occurrences: HashMap::new(),
    };
    while let Some(start) = remaining.find('<') {
        remaining = &remaining[start + 1..];
        let end = remaining.find('>')?;
        let tag = &remaining[..end];
        if let Some(closing) = tag.strip_prefix('/') {
            if state.stack.pop()? != closing.trim() {
                return None;
            }
            if state.page_depth.is_some_and(|depth| state.stack.len() < depth) {
                state.page_depth = None;
            }
            remaining = &remaining[end + 1..];
        } else {
            remaining = &remaining[state.opening_word_end(remaining, end, budget)?..];
        }
    }
    if !state.stack.is_empty() || !state.page_seen || state.occurrences.len() != source.len() {
        return None;
    }
    for word in source {
        let bbox = [
            word.left,
            word.top,
            word.left.checked_add(word.width)?,
            word.top.checked_add(word.height)?,
        ];
        state.occurrences.remove(&(word.text.clone(), bbox))?;
    }
    state.occurrences.is_empty().then_some(())
}

pub(super) struct FragmentedTablePolicy<'a> {
    pub(super) config: &'a TesseractConfig,
    pub(super) allow_dot_leader: bool,
}

pub(super) fn reconcile_fragmented_hocr_table_rows(
    hocr: &str,
    document: &InternalDocument,
    tables: &[OcrTable],
    tsv: &str,
    policy: FragmentedTablePolicy<'_>,
    limits: &SecurityLimits,
) -> Option<(InternalDocument, Vec<OcrTableBoundingBox>)> {
    let FragmentedTablePolicy {
        config,
        allow_dot_leader,
    } = policy;
    let mut budget = fragmented_hocr_budget(hocr, tsv, limits)?;
    let boxes = fragmented_table_boxes(tables, config.page_number, limits, &mut budget)?;
    let source = fragmented_tsv_source(tsv)?;
    fragmented_hocr_source_matches(hocr, &source, &mut budget)?;
    let table_words = prove_all_fragmented_tables(&source, tables, boxes, config, &mut budget, allow_dot_leader)?;
    reconcile_proved_fragments(hocr, document, &table_words, config.page_number)
}

fn fragmented_tsv_source(tsv: &str) -> Option<Vec<HocrWord>> {
    let source = crate::ocr::table::tsv_parser::words_on_lines(tsv, 0.0);
    if source.is_empty()
        || source.len() > MAX_FRAGMENTED_HOCR_WORDS
        || source.iter().any(|(line, _)| line.is_none_or(|line| line[0] != 1))
    {
        return None;
    }
    Some(source.into_iter().map(|(_, word)| word).collect())
}

fn prove_all_fragmented_tables(
    source: &[HocrWord],
    tables: &[OcrTable],
    boxes: Vec<BoundingBox>,
    config: &TesseractConfig,
    budget: &mut usize,
    allow_dot_leader: bool,
) -> Option<Vec<Vec<HocrWord>>> {
    let mut table_words = Vec::with_capacity(tables.len());
    for (table, bbox) in tables.iter().zip(boxes) {
        *budget = budget.checked_sub(source.iter().map(|word| word.text.len() + 1).sum::<usize>())?;
        let mut words = source.to_vec();
        let proved = prove_fragmented_table_rows_budgeted(
            &mut words,
            bbox,
            &table.cells,
            config.table_row_threshold_ratio,
            budget,
            allow_dot_leader,
        )?;
        table_words.push(proved.into_iter().cloned().collect::<Vec<_>>());
    }
    Some(table_words)
}

fn reconcile_proved_fragments(
    hocr: &str,
    document: &InternalDocument,
    table_words: &[Vec<HocrWord>],
    page_number: u32,
) -> Option<(InternalDocument, Vec<OcrTableBoundingBox>)> {
    let proved: Vec<_> = table_words.iter().flatten().collect();
    let claimed_words = proved
        .iter()
        .map(|word| word.text.split_whitespace().count())
        .sum::<usize>();
    let mut reconciled = parse_hocr_without_proved_table_fragments(hocr, page_number, proved);
    let words = |doc: &InternalDocument| {
        doc.elements
            .iter()
            .map(|e| e.text.split_whitespace().count())
            .sum::<usize>()
    };
    let remaining = words(&reconciled);
    reconciled.ocr_text_claimed_by_tables = remaining == 0;
    if claimed_words == 0 || words(document).checked_sub(remaining) != Some(claimed_words) {
        return None;
    }
    let bounds = if table_words.len() == 2 && remaining > 0 {
        table_words
            .iter()
            .map(|words| proved_fragment_bounds(words))
            .collect::<Option<Vec<_>>>()?
    } else {
        Vec::new()
    };
    Some((reconciled, bounds))
}

pub(super) fn apply_fragmented_reconciliation(
    document: &mut InternalDocument,
    tables: &mut [OcrTable],
    reconciled: InternalDocument,
    bounds: Vec<OcrTableBoundingBox>,
) -> bool {
    if bounds.is_empty() {
        *document = reconciled;
        return true;
    }
    if bounds.len() != tables.len() {
        return false;
    }
    *document = reconciled;
    for (table, bbox) in tables.iter_mut().zip(bounds) {
        table.bounding_box = Some(bbox);
    }
    true
}

fn proved_fragment_bounds(words: &[HocrWord]) -> Option<OcrTableBoundingBox> {
    Some(OcrTableBoundingBox {
        left: words.iter().map(|word| word.left).min()?,
        top: words.iter().map(|word| word.top).min()?,
        right: words
            .iter()
            .map(|word| word.left.checked_add(word.width))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .max()?,
        bottom: words
            .iter()
            .map(|word| word.top.checked_add(word.height))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .max()?,
    })
}
