use super::Placement;
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::types::{ExtractedDocument, FormatMetadata, OcrBoundingGeometry, OcrElement, OcrElementLevel};

const MIN_CONFIDENCE: f64 = 0.85;
const MAX_WORDS: usize = 128;
const MAX_TEXT_BYTES: usize = 4096;

#[derive(Clone, Copy)]
struct Bounds {
    x0: u64,
    y0: u64,
    x1: u64,
    y1: u64,
}

impl Bounds {
    fn contains(self, other: Self) -> bool {
        self.x0 <= other.x0 && self.y0 <= other.y0 && self.x1 >= other.x1 && self.y1 >= other.y1
    }

    fn x_overlaps(self, other: Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1
    }

    fn overlaps(self, other: Self) -> bool {
        self.x_overlaps(other) && self.y0 < other.y1 && other.y0 < self.y1
    }
}

fn bounds(element: &OcrElement, frame: (u32, u32), scale: (u32, u32)) -> Option<Bounds> {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = element.geometry
    else {
        return None;
    };
    let right = left.checked_add(width)?;
    let bottom = top.checked_add(height)?;
    let x = |value| u64::from(value) * u64::from(scale.0);
    let y = |value| u64::from(value) * u64::from(scale.1);
    (width > 0 && height > 0 && right <= frame.0 && bottom <= frame.1).then(|| Bounds {
        x0: x(left),
        y0: y(top),
        x1: x(right),
        y1: y(bottom),
    })
}

struct Read<'a> {
    lines: Vec<(&'a str, Bounds)>,
    words: Vec<(&'a str, Bounds)>,
}

fn read(doc: &ExtractedDocument, frame: (u32, u32), scale: (u32, u32), n: usize) -> Option<Read<'_>> {
    let metadata = &doc.metadata;
    if !doc.tables.is_empty()
        || !doc.processing_warnings.is_empty()
        || doc.content.len() > MAX_TEXT_BYTES
        || !matches!(&metadata.format, Some(FormatMetadata::Ocr(ocr)) if ocr.psm == 6 && ocr.language == "eng")
        || metadata
            .additional
            .get(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)?
            .as_u64()?
            != u64::from(frame.0)
        || metadata
            .additional
            .get(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)?
            .as_u64()?
            != u64::from(frame.1)
    {
        return None;
    }
    let elements = doc.ocr_elements.as_deref()?;
    if elements.len() > MAX_WORDS + n {
        return None;
    }
    let mut read = Read {
        lines: Vec::new(),
        words: Vec::new(),
    };
    for element in elements {
        if element.rotation.is_some()
            || element.page_number != 1
            || element.backend_metadata.get("backend")?.as_str()? != "tesseract-iterator"
        {
            return None;
        }
        let item = (element.text.as_str(), bounds(element, frame, scale)?);
        match element.level {
            OcrElementLevel::Line if (MIN_CONFIDENCE..=1.0).contains(&element.confidence.recognition) => {
                read.lines.push(item)
            }
            OcrElementLevel::Word if element.text.split_whitespace().count() == 1 => read.words.push(item),
            _ => return None,
        }
    }
    (read.lines.len() == n && owns_literal_words(&read, doc)).then_some(read)
}

fn owns_literal_words(read: &Read<'_>, doc: &ExtractedDocument) -> bool {
    if !read
        .lines
        .iter()
        .map(|(text, _)| *text)
        .eq(doc.content.lines().filter(|line| !line.is_empty()))
        || !read.lines.windows(2).all(|rows| rows[0].1.y1 < rows[1].1.y0)
        || doc
            .metadata
            .additional
            .get("word_count")
            .and_then(serde_json::Value::as_u64)
            != Some(read.words.len() as u64)
    {
        return false;
    }
    let mut claimed = 0usize;
    for &(line, region) in &read.lines {
        let mut owned = read
            .words
            .iter()
            .filter(|(_, word)| region.contains(*word))
            .collect::<Vec<_>>();
        owned.sort_unstable_by_key(|(_, word)| word.x0);
        if !owned.windows(2).all(|words| words[0].1.x1 <= words[1].1.x0)
            || !owned.iter().map(|(word, _)| *word).eq(line.split_whitespace())
        {
            return false;
        }
        claimed += owned.len();
    }
    claimed == read.words.len()
}

pub(super) fn has_primary(doc: &ExtractedDocument, frame: (u32, u32)) -> bool {
    doc.metadata.image_preprocessing.is_none() && read(doc, frame, (1, 1), 2).is_some()
}

pub(super) fn insert(
    primary: &ExtractedDocument,
    retry: &ExtractedDocument,
    placement: &Placement,
    native: &str,
) -> Option<String> {
    let resize = retry.metadata.image_preprocessing.as_ref()?;
    let dimensions = resize.new_dimensions?;
    if resize.skipped_resize
        || resize.resize_error.is_some()
        || resize.dimension_clamped
        || (resize.original_dimensions.width, resize.original_dimensions.height)
            != (placement.dimensions.0 as usize, placement.dimensions.1 as usize)
        || (resize.original_dpi.horizontal, resize.original_dpi.vertical) != (placement.dpi, placement.dpi)
        || (dimensions.width, dimensions.height) != (placement.scaled.0 as usize, placement.scaled.1 as usize)
    {
        return None;
    }
    let before = read(primary, placement.dimensions, placement.scaled, 2)?;
    let after = read(retry, placement.scaled, placement.dimensions, 3)?;
    let [(first, first_box), (last, last_box)] = before.lines.as_slice() else {
        return None;
    };
    let [
        (retry_first, retry_first_box),
        (new, new_box),
        (retry_last, retry_last_box),
    ] = after.lines.as_slice()
    else {
        return None;
    };
    if first != retry_first
        || last != retry_last
        || !first_box.overlaps(*retry_first_box)
        || !last_box.overlaps(*retry_last_box)
        || first_box.y1 >= new_box.y0
        || new_box.y1 >= last_box.y0
        || !new_box.x_overlaps(*first_box)
        || !new_box.x_overlaps(*last_box)
        || before.words.iter().any(|(_, word)| new_box.overlaps(*word))
        || !novel_prose(new, &primary.content, native)
    {
        return None;
    }
    let separator = primary.content.strip_prefix(first)?.strip_suffix(last)?;
    if separator.is_empty() || !separator.bytes().all(|byte| byte == b'\n') {
        return None;
    }
    Some(format!("{first}{separator}{new}{separator}{last}"))
}

fn novel_prose(text: &str, primary: &str, native: &str) -> bool {
    let words = text.split_ascii_whitespace().collect::<Vec<_>>();
    if !(4..=12).contains(&words.len()) || !text.bytes().all(|byte| byte.is_ascii_alphabetic() || byte == b' ') {
        return false;
    }
    [primary, native].iter().all(|existing| {
        let existing = existing
            .split(|ch: char| !ch.is_ascii_alphabetic())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        !existing
            .windows(words.len())
            .any(|window| window.iter().zip(&words).all(|(a, b)| a.eq_ignore_ascii_case(b)))
    })
}
