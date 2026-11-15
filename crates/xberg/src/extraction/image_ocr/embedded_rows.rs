use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};

const MAX_PLAIN_TEXT_BYTES: usize = 4096;
const MAX_PLAIN_WORDS: usize = 128;
const MIN_BODY_ROWS: usize = 3;
const MIN_NUMERIC_COLUMNS: usize = 3;

struct Word<'a> {
    text: &'a str,
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl<'a> Word<'a> {
    fn has_alphanumeric_text(&self) -> bool {
        self.text.chars().any(char::is_alphanumeric)
    }

    fn from_element(element: &'a OcrElement) -> Option<Self> {
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = element.geometry
        else {
            return None;
        };
        (width > 0
            && height > 0
            && element.rotation.is_none()
            && element.page_number == 1
            && element.backend_metadata.get("backend")?.as_str()? == "tesseract-iterator")
            .then_some(Self {
                text: &element.text,
                left,
                right: left.checked_add(width)?,
                top,
                bottom: top.checked_add(height)?,
            })
    }
}

pub(super) fn assemble_plain(document: &ExtractedDocument) -> Option<String> {
    if !document.tables.is_empty() || document.content.len() > MAX_PLAIN_TEXT_BYTES {
        return None;
    }
    assemble_words(document, MAX_PLAIN_WORDS)
}

pub(super) fn assemble_html(document: &ExtractedDocument) -> Option<String> {
    assemble_plain(document).or_else(|| owns_stock_table_lines(document).then(|| document.content.clone()))
}

fn owns_stock_table_lines(document: &ExtractedDocument) -> bool {
    let [table] = document.tables.as_slice() else {
        return false;
    };
    let Some(bounds) = &table.bounding_box else {
        return false;
    };
    let Some(elements) = &document.ocr_elements else {
        return false;
    };
    let Some((header, body)) = table.cells.split_first() else {
        return false;
    };
    if !document.processing_warnings.is_empty()
        || document.content.len() > MAX_PLAIN_TEXT_BYTES
        || document.content.split_whitespace().count() > MAX_PLAIN_WORDS
        || table.cells.len() > MAX_PLAIN_WORDS
        || body.len() < MIN_BODY_ROWS
        || header.len() <= MIN_NUMERIC_COLUMNS
    {
        return false;
    }

    let mut lines = elements.iter().filter(|element| element.level == OcrElementLevel::Line);
    let mut stock = document.content.lines().filter(|line| !line.trim().is_empty());
    let mut previous = None;
    // These are lines already emitted by the recognizer. Do not reorder or join fragments. ~keep
    for (index, cells) in table.cells.iter().enumerate() {
        let Some((line, text)) = lines.next().and_then(Word::from_element).zip(stock.next()) else {
            return false;
        };
        if cells.len() != header.len()
            || !line.text.split_whitespace().eq(text.split_whitespace())
            || !line_inside_table(&line, bounds, previous.as_ref())
            || !matches_stock_table_line(text, cells, index == 0)
        {
            return false;
        }
        previous = Some(line);
    }
    lines.next().is_none() && stock.next().is_none()
}

fn line_inside_table(
    line: &Word<'_>,
    bounds: &crate::types::extraction::BoundingBox,
    previous: Option<&Word<'_>>,
) -> bool {
    bounds.x0 <= f64::from(line.left)
        && f64::from(line.right) <= bounds.x1
        && bounds.y0 <= f64::from(line.top)
        && f64::from(line.bottom) <= bounds.y1
        && previous.is_none_or(|prior| prior.top < line.top && prior.bottom < line.bottom)
}

fn matches_stock_table_line(text: &str, cells: &[String], header: bool) -> bool {
    let mut words = text.split_whitespace();
    if header {
        return words.eq(cells.iter().flat_map(|cell| cell.split_whitespace()));
    }
    let (label, values) = cells.split_first().expect("validated table");
    if !label.chars().any(char::is_alphabetic)
        || !label.split_whitespace().all(|word| words.next() == Some(word))
        || !values
            .iter()
            .rev()
            .all(|value| numeric_literal(value) && words.next_back() == Some(value))
    {
        return false;
    }
    words.next().is_none_or(|leader| {
        leader.bytes().filter(|&byte| byte == b'.').count() >= 3
            && leader.bytes().all(|byte| matches!(byte, b'.' | b'+' | b'-'))
    }) && words.next().is_none()
}

pub(super) fn assemble(document: &ExtractedDocument) -> Option<String> {
    if !document.processing_warnings.is_empty() {
        return None;
    }
    if let [table] = document.tables.as_slice()
        && !table.cells.is_empty()
        && document.content.trim() == crate::rendering::common::render_table_markdown(&table.cells).trim()
    {
        return Some(crate::rendering::common::render_table_plain(&table.cells));
    }
    if !document.tables.is_empty() {
        return None;
    }
    assemble_words(document, usize::MAX)
}

fn assemble_words(document: &ExtractedDocument, max_words: usize) -> Option<String> {
    if !document.processing_warnings.is_empty() {
        return None;
    }
    let words = document
        .ocr_elements
        .as_ref()?
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .take(max_words.saturating_add(1))
        .map(Word::from_element)
        .collect::<Option<Vec<_>>>()?;
    if words.len() > max_words {
        return None;
    }
    // Require every literal occurrence, including repeated values and leader marks. ~keep
    let mut expected: Vec<_> = document.content.split_whitespace().collect();
    let mut actual: Vec<_> = words.iter().map(|word| word.text).collect();
    expected.sort_unstable();
    actual.sort_unstable();
    if actual != expected {
        return None;
    }

    validate_and_render_bands(words)
}

fn validate_and_render_bands(mut words: Vec<Word<'_>>) -> Option<String> {
    words.sort_unstable_by_key(|word| (word.top, word.left));
    let mut bands: Vec<Vec<Word<'_>>> = Vec::new();
    let mut common_bottom = None;
    let mut bottom = 0;
    for word in words {
        if word.top >= bottom {
            bands.push(Vec::new());
            common_bottom = None;
            bottom = word.bottom;
        }
        // A vertical bridge between two rows cannot certify either row. ~keep
        if word.has_alphanumeric_text() {
            if common_bottom.is_some_and(|end| word.top >= end) {
                return None;
            }
            common_bottom = Some(common_bottom.map_or(word.bottom, |end| end.min(word.bottom)));
        }
        bottom = bottom.max(word.bottom);
        bands.last_mut()?.push(word);
    }
    let (header, body) = bands.split_first_mut()?;
    if body.len() < MIN_BODY_ROWS {
        return None;
    }
    for band in body.iter_mut().chain(std::iter::once(&mut *header)) {
        if !punctuation_overlaps_text(band) {
            return None;
        }
        band.sort_unstable_by_key(|word| word.left);
        if band.windows(2).any(|pair| pair[0].right > pair[1].left) {
            return None;
        }
    }
    let columns = header.len().checked_sub(1)?;
    if columns < MIN_NUMERIC_COLUMNS
        || !header
            .iter()
            .all(|word| word.text.chars().any(char::is_alphabetic) && !word.text.chars().any(char::is_numeric))
        || !has_disjoint_column_extents(header, body)
    {
        return None;
    }
    Some(
        bands
            .iter()
            .map(|band| band.iter().map(|word| word.text).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn punctuation_overlaps_text(band: &[Word<'_>]) -> bool {
    let mut text_words = band.iter().filter(|word| word.has_alphanumeric_text());
    let Some(first) = text_words.next() else {
        return false;
    };
    // The alphanumeric boxes already share a vertical intersection. Their combined span has no gaps. ~keep
    let (top, bottom) = text_words.fold((first.top, first.bottom), |(top, bottom), word| {
        (top.min(word.top), bottom.max(word.bottom))
    });
    band.iter().all(|word| word.top < bottom && top < word.bottom)
}

fn has_disjoint_column_extents(header: &[Word<'_>], body: &[Vec<Word<'_>>]) -> bool {
    let mut extents: Vec<_> = header.iter().map(|word| word.left..word.right).collect();
    for row in body {
        if !ledger_row(row, header) {
            return false;
        }
        let (label, values) = row.split_at(row.len() - (header.len() - 1));
        let row_extents = std::iter::once(label[0].left..label[label.len() - 1].right)
            .chain(values.iter().map(|word| word.left..word.right));
        for (extent, bounds) in extents.iter_mut().zip(row_extents) {
            extent.start = extent.start.min(bounds.start);
            extent.end = extent.end.max(bounds.end);
        }
    }
    extents.windows(2).all(|pair| pair[0].end <= pair[1].start)
}

fn ledger_row(row: &[Word<'_>], header: &[Word<'_>]) -> bool {
    let (label_heading, headings) = header.split_first().expect("validated header");
    let Some(label_words) = row.len().checked_sub(headings.len()) else {
        return false;
    };
    let (label, values) = row.split_at(label_words);
    label
        .first()
        .is_some_and(|word| overlaps(word, label_heading) && word.text.chars().any(char::is_alphabetic))
        && label.last().is_some_and(|word| word.right <= headings[0].left)
        && label.iter().all(|word| !word.text.chars().any(char::is_numeric))
        && values.iter().enumerate().all(|(index, value)| {
            overlaps(value, &headings[index])
                && !overlaps(value, &header[index])
                && headings.get(index + 1).is_none_or(|next| !overlaps(value, next))
                && numeric_literal(value.text)
        })
}

fn numeric_literal(text: &str) -> bool {
    text.bytes().any(|byte| byte.is_ascii_digit())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b",.()+-%$".contains(&byte))
}

fn overlaps(first: &Word<'_>, second: &Word<'_>) -> bool {
    first.left < second.right && second.left < first.right
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, left: u32, top: u32, width: u32) -> OcrElement {
        OcrElement {
            text: text.into(),
            geometry: OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height: 6,
            },
            level: OcrElementLevel::Word,
            page_number: 1,
            backend_metadata: [("backend".into(), "tesseract-iterator".into())].into(),
            ..Default::default()
        }
    }

    fn ledger() -> Vec<OcrElement> {
        let mut words = vec![
            word("Item", 0, 0, 12),
            word("Count", 40, 0, 15),
            word("Budget", 70, 0, 15),
            word("Change", 100, 0, 15),
        ];
        for (label, top) in [("Snow", 10), ("Field", 20), ("Water", 30)] {
            words.extend([
                word(label, 0, top, 12),
                word("...", 20, top, 12),
                word("12", 44, top, 9),
                word("300", 70, top, 12),
                word("(4)", 100, top, 12),
            ]);
        }
        words
    }

    fn assemble_words(mut words: Vec<OcrElement>) -> Option<String> {
        words.sort_by_key(|word| Word::from_element(word).unwrap().left);
        let content = words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assemble(&ExtractedDocument {
            content,
            ocr_elements: Some(words),
            ..Default::default()
        })
    }

    #[test]
    fn assembles_column_ordered_words_into_exact_rows() {
        assert_eq!(
            assemble_words(ledger()).as_deref(),
            Some("Item Count Budget Change\nSnow ... 12 300 (4)\nField ... 12 300 (4)\nWater ... 12 300 (4)")
        );
    }

    #[test]
    fn accepts_leader_above_text_when_it_overlaps_the_row() {
        let mut words = ledger();
        words[5] = word("...", 20, 8, 12);
        words[6] = word("12", 44, 14, 9);
        assert_eq!(
            assemble_words(words).as_deref(),
            Some("Item Count Budget Change\nSnow ... 12 300 (4)\nField ... 12 300 (4)\nWater ... 12 300 (4)")
        );
    }

    #[test]
    fn rejects_punctuation_connected_only_through_another_punctuation_box() {
        let mut words = ledger();
        words[5] = word("...", 20, 14, 8);
        let mut detached = word("..", 30, 17, 8);
        detached.geometry = OcrBoundingGeometry::Rectangle {
            left: 30,
            top: 17,
            width: 8,
            height: 2,
        };
        words.push(detached);
        assert_eq!(assemble_words(words), None);
    }

    #[test]
    fn rejects_columns_that_overlap_across_body_rows() {
        let mut words = ledger();
        words[6] = word("12", 44, 10, 24);
        words[12] = word("300", 60, 20, 22);
        assert_eq!(assemble_words(words), None);
    }

    #[test]
    fn rejects_missing_header_evidence_and_ambiguous_geometry() {
        for (description, index, replacements) in [
            ("numeric header", 1, vec![word("2026", 40, 0, 15)]),
            ("label crosses column", 5, vec![word("removal", 20, 10, 22)]),
            ("extra numeric label", 5, vec![word("99", 20, 10, 12)]),
            (
                "value covers two headings",
                6,
                vec![word("1234", 44, 10, 28), word("300", 74, 10, 10)],
            ),
            ("value in gutter", 6, vec![word("12", 56, 10, 9)]),
        ] {
            let mut words = ledger();
            words.splice(index..index + replacements.len(), replacements);
            assert_eq!(assemble_words(words), None, "{description}");
        }
    }
}
