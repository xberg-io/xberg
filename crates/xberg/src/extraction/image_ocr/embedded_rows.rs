use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};

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

pub(super) fn assemble(document: &ExtractedDocument) -> Option<String> {
    if !document.tables.is_empty() || !document.processing_warnings.is_empty() {
        return None;
    }
    let mut words = document
        .ocr_elements
        .as_ref()?
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .map(Word::from_element)
        .collect::<Option<Vec<_>>>()?;
    // Require every literal occurrence, including repeated values and leader marks. ~keep
    let mut expected: Vec<_> = document.content.split_whitespace().collect();
    let mut actual: Vec<_> = words.iter().map(|word| word.text).collect();
    expected.sort_unstable();
    actual.sort_unstable();
    if actual != expected {
        return None;
    }

    words.sort_unstable_by_key(|word| (word.top, word.left));
    let mut bands: Vec<Vec<Word<'_>>> = Vec::new();
    let mut common_bottom = 0;
    let mut bottom = 0;
    for word in words {
        if word.top >= bottom {
            bands.push(Vec::new());
            common_bottom = word.bottom;
            bottom = word.bottom;
        }
        // A vertical bridge between two rows cannot certify either row. ~keep
        if word.text.chars().any(char::is_alphanumeric) {
            if word.top >= common_bottom {
                return None;
            }
            common_bottom = common_bottom.min(word.bottom);
        }
        bottom = bottom.max(word.bottom);
        bands.last_mut()?.push(word);
    }
    let (header, body) = bands.split_first_mut()?;
    if body.len() < MIN_BODY_ROWS {
        return None;
    }
    for band in body.iter_mut().chain(std::iter::once(&mut *header)) {
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
        || !body.iter().all(|row| ledger_row(row, header))
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
                && value.text.bytes().any(|byte| byte.is_ascii_digit())
                && value
                    .text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || b",.()+-%$".contains(&byte))
        })
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
