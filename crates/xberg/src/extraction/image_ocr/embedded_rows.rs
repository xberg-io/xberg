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
    if columns < MIN_NUMERIC_COLUMNS || !body.iter().all(|row| ledger_row(row, header, columns)) {
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

fn ledger_row(row: &[Word<'_>], header: &[Word<'_>], columns: usize) -> bool {
    let Some(label_words) = row.len().checked_sub(columns) else {
        return false;
    };
    let (label, values) = row.split_at(label_words);
    label
        .first()
        .is_some_and(|word| word.left < header[0].right && word.text.chars().any(char::is_alphabetic))
        && values.iter().zip(&header[1..]).all(|(value, heading)| {
            value.left < heading.right
                && heading.left < value.right
                && value.text.bytes().any(|byte| byte.is_ascii_digit())
                && value
                    .text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || b",.()+-%$".contains(&byte))
        })
}
