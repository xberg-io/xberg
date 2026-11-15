use super::{
    ExtractedDocument, MAX_SOURCE_PIXELS, MIN_LIGHT_SAMPLE, OcrBackend, OcrConfig, Range, Source, certified_strip_lines,
};
use crate::types::{OcrBoundingGeometry, OcrElementLevel};
use std::sync::Arc;

pub(super) const EXTRA_CALLS: usize = 4;

fn lines(result: &ExtractedDocument) -> impl Iterator<Item = &str> {
    result.content.lines().map(str::trim).filter(|line| !line.is_empty())
}

struct SplitRow<'a> {
    index: usize,
    band: &'a Range<u32>,
    columns: (u32, u32),
}

fn split_row<'a>(
    source: &Source,
    strip: &Range<u32>,
    ink: &'a [Range<u32>],
    result: &ExtractedDocument,
) -> Option<SplitRow<'a>> {
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};

    let dimensions = &result.metadata.additional;
    if dimensions.get(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)?.as_u64()? != u64::from(source.width)
        || dimensions.get(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)?.as_u64()? != u64::from(strip.end - strip.start)
    {
        return None;
    }
    let mut text = lines(result);
    let mut boxes = result
        .ocr_elements
        .as_ref()?
        .iter()
        .filter(|e| e.level == OcrElementLevel::Line)
        .peekable();
    let mut selected = None;

    // The prefix and suffix are reread independently. Every original line must first be owned by
    // its native band, with exactly one band containing two adjacent OCR fragments. ~keep
    for (index, band) in ink
        .iter()
        .filter(|band| strip.start <= band.start && band.end <= strip.end)
        .enumerate()
    {
        let (left, right) = owned_box(boxes.next()?, text.next()?, strip, band, source.width)?;
        if !boxes.peek().is_some_and(|element| {
            matches!(element.geometry, OcrBoundingGeometry::Rectangle { top, .. } if top < band.end - strip.start)
        }) {
            continue;
        }
        let other_text = text.next()?;
        let (other_left, other_right) = owned_box(boxes.next()?, other_text, strip, band, source.width)?;
        if selected.is_some()
            || other_text.split_whitespace().count() != 1
            || !connected_ink(source, band, left..right, other_left..other_right)
        {
            return None;
        }
        selected = Some(SplitRow {
            index,
            band,
            columns: (left, other_right),
        });
    }
    (boxes.next().is_none() && text.next().is_none()).then_some(selected)?
}

fn owned_box(
    element: &crate::types::OcrElement,
    text: &str,
    strip: &Range<u32>,
    band: &Range<u32>,
    source_width: u32,
) -> Option<(u32, u32)> {
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
    let top = strip.start.checked_add(top)?;
    let bottom = top.checked_add(height)?;
    (left < right
        && right <= source_width
        && top < bottom
        && top >= band.start
        && bottom <= band.end
        && element.text.split_whitespace().eq(text.split_whitespace()))
    .then_some((left, right))
}

fn connected_ink(source: &Source, band: &Range<u32>, first: Range<u32>, second: Range<u32>) -> bool {
    const MAX_GLYPH_GAP: usize = 2;
    if first.end > second.start || second.start - first.end > band.end - band.start {
        return false;
    }
    let width = source.width as usize;
    let has_ink = |x: u32| {
        band.clone()
            .any(|y| source.samples[y as usize * width + x as usize] < MIN_LIGHT_SAMPLE)
    };
    let mut gap = 0;
    for x in first.end - 1..=second.start {
        if has_ink(x) {
            gap = 0;
        } else {
            gap += 1;
            if gap > MAX_GLYPH_GAP {
                return false;
            }
        }
    }
    has_ink(first.end - 1) && has_ink(second.start)
}

pub(super) async fn reread(
    source: &Source,
    strip: &Range<u32>,
    ink: &[Range<u32>],
    result: &ExtractedDocument,
    (backend, config): (&Arc<dyn OcrBackend>, &OcrConfig),
    cancelled: impl Fn() -> bool,
) -> crate::Result<Option<[ExtractedDocument; 3]>> {
    let Some(SplitRow { index, band, columns }) = split_row(source, strip, ink, result) else {
        return Ok(None);
    };
    let previous = ink.iter().rev().find(|row| row.end <= band.start);
    let following = ink.iter().find(|row| row.start >= band.end);
    if !previous.is_some_and(|row| row.start >= strip.start && band.start - row.end >= 16)
        || !following.is_some_and(|row| row.end <= strip.end && row.start - band.end >= 16)
    {
        return Ok(None);
    }
    let narrow = band.start - 8..band.end + 8;
    let wide = band.start - 16..band.end + 16;
    let read_rows =
        u64::from(source.height) + u64::from(strip.end - strip.start) + u64::from(narrow.end - narrow.start);
    if read_rows * u64::from(source.width) > MAX_SOURCE_PIXELS as u64 {
        return Ok(None);
    }
    let mut parts = Vec::with_capacity(EXTRA_CALLS);
    for (index, rows) in [narrow, wide.clone(), strip.start..wide.start, wide.end..strip.end]
        .into_iter()
        .enumerate()
    {
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let Some(png) = source.encode_rows(rows.clone()) else {
            return Ok(None);
        };
        let read = backend.process_image_owned(Arc::new(png), config).await;
        if cancelled() || matches!(read, Err(crate::XbergError::Cancelled)) {
            return Err(crate::XbergError::Cancelled);
        }
        let Ok(read) = read else {
            return Ok(None);
        };
        if !certified_strip_lines(&read, source.width, &rows, ink) {
            return Ok(None);
        }
        if index < 2
            && read.ocr_elements.as_ref().and_then(|elements| {
                let line = elements.iter().find(|element| element.level == OcrElementLevel::Line)?;
                owned_box(line, read.content.trim(), &rows, band, source.width)
            }) != Some(columns)
        {
            return Ok(None);
        }
        parts.push(read);
    }
    let Ok([line, check, before, after]) = <[_; EXTRA_CALLS]>::try_from(parts) else {
        return Ok(None);
    };
    let original_parts: Vec<_> = result.content.trim().split("\n\n").collect();
    let Some((first, second)) = original_parts.get(index).zip(original_parts.get(index + 1)) else {
        return Ok(None);
    };
    if before
        .content
        .trim()
        .split("\n\n")
        .ne(original_parts[..index].iter().copied())
        || after
            .content
            .trim()
            .split("\n\n")
            .ne(original_parts[index + 2..].iter().copied())
        || !joined_line(first, second, line.content.trim())
        || line.content.trim() != check.content.trim()
    {
        return Ok(None);
    }
    Ok(Some([before, line, after]))
}

fn joined_line(first: &str, second: &str, combined: &str) -> bool {
    let Some((prefix, tail)) = first.rsplit_once(char::is_whitespace) else {
        return false;
    };
    let Some((new_prefix, new_tail)) = combined.rsplit_once(char::is_whitespace) else {
        return false;
    };
    prefix == new_prefix
        && new_tail.starts_with(tail)
        && new_tail.ends_with(second)
        && new_tail.len() > tail.len() + second.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agreed_reread_cannot_rewrite_an_unchanged_prefix_or_drop_the_fragment() {
        assert!(joined_line("leave this ra", "in", "leave this raisin"));
        assert!(!joined_line("leave this ra", "in", "leave that raisin"));
        assert!(!joined_line("leave this ra", "in", "leave this raise"));
        assert!(!joined_line("leave this ra", "in", "leave this rain"));
    }

    #[test]
    fn same_band_fragment_requires_ink_between_both_recognized_edges() {
        let mut source = Source {
            width: 32,
            height: 12,
            dpi: 120.0,
            samples: vec![255; 32 * 12],
            retained_bytes: 32 * 12,
            certify_rows: true,
            reread_split_row: true,
        };
        for y in 3..9 {
            for x in [9, 10, 13, 14, 17, 18, 19] {
                source.samples[y * 32 + x] = 0;
            }
        }
        assert!(connected_ink(&source, &(2..10), 8..11, 17..20));
        for y in 3..9 {
            source.samples[y * 32 + 13..y * 32 + 15].fill(255);
        }
        assert!(!connected_ink(&source, &(2..10), 8..11, 17..20));
        assert!(!connected_ink(&source, &(2..10), 8..11, 22..25));
    }
}
