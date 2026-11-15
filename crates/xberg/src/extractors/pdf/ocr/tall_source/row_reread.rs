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
    columns: Range<u32>,
    inserted_letters: usize,
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
        let first = owned_box(boxes.next()?, text.next()?, strip, band, source.width)?;
        if !boxes.peek().is_some_and(|element| {
            matches!(element.geometry, OcrBoundingGeometry::Rectangle { top, .. } if top < band.end - strip.start)
        }) {
            continue;
        }
        let other_text = text.next()?;
        let second = owned_box(boxes.next()?, other_text, strip, band, source.width)?;
        if selected.is_some() || !other_text.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return None;
        }
        let common_rows = first.rows.start.max(second.rows.start)..first.rows.end.min(second.rows.end);
        let inserted_letters = join_ink(source, band, &common_rows, first.columns.end, second.columns.start)?;
        selected = Some(SplitRow {
            index,
            band,
            columns: first.columns.start..second.columns.end,
            inserted_letters,
        });
    }
    (boxes.next().is_none() && text.next().is_none()).then_some(selected)?
}

struct LineBox {
    columns: Range<u32>,
    rows: Range<u32>,
}

fn owned_box(
    element: &crate::types::OcrElement,
    text: &str,
    strip: &Range<u32>,
    band: &Range<u32>,
    source_width: u32,
) -> Option<LineBox> {
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
    .then_some(LineBox {
        columns: left..right,
        rows: top..bottom,
    })
}

fn join_ink(
    source: &Source,
    band: &Range<u32>,
    common_rows: &Range<u32>,
    first_end: u32,
    second_start: u32,
) -> Option<usize> {
    const MAX_GLYPH_GAP: u32 = 2;
    let left_anchor = first_end.checked_sub(1)?;
    if first_end > second_start || second_start - first_end > band.end - band.start {
        return None;
    }
    let ink = |x: u32, y: u32| source.samples[y as usize * source.width as usize + x as usize] < MIN_LIGHT_SAMPLE;
    let mut runs: Vec<Range<u32>> = Vec::new();
    for x in (left_anchor..=second_start).filter(|&x| band.clone().any(|y| ink(x, y))) {
        if let Some(last) = runs.last_mut() {
            if last.end == x {
                last.end += 1;
                continue;
            }
            if x - last.end > MAX_GLYPH_GAP {
                return None;
            }
        }
        runs.push(x..x + 1);
    }
    let [first, internal @ .., last] = runs.as_slice() else {
        return None;
    };
    if internal.is_empty() || first.start != left_anchor || last.end != second_start + 1 {
        return None;
    }
    // Each inserted letter needs its own previously uncovered run. The runs and both
    // fragment edges must carry ink on a shared vertical interval to exclude staggered text. ~keep
    let required_rows = (band.end - band.start).div_ceil(3);
    let mut consecutive = 0;
    for y in common_rows.clone() {
        if ink(left_anchor, y) && ink(second_start, y) && internal.iter().all(|run| run.clone().any(|x| ink(x, y))) {
            consecutive += 1;
            if consecutive >= required_rows {
                return Some(internal.len());
            }
        } else {
            consecutive = 0;
        }
    }
    None
}

pub(super) async fn reread(
    source: &Source,
    strip: &Range<u32>,
    ink: &[Range<u32>],
    result: &ExtractedDocument,
    (backend, config): (&Arc<dyn OcrBackend>, &OcrConfig),
    cancelled: impl Fn() -> bool,
) -> crate::Result<Option<[ExtractedDocument; 3]>> {
    let Some(SplitRow {
        index,
        band,
        columns,
        inserted_letters,
    }) = split_row(source, strip, ink, result)
    else {
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
    let mut verification = config.clone();
    let Some(tesseract) = verification.tesseract_config.as_mut() else {
        return Ok(None);
    };
    let Some(primary_psm) = tesseract.psm else {
        return Ok(None);
    };
    // A second layout constrains the same row without repeating the strip's segmentation. ~keep
    const SINGLE_LINE: i32 = 7;
    const RAW_LINE: i32 = 13;
    let verification_psm = if primary_psm == SINGLE_LINE {
        RAW_LINE
    } else {
        SINGLE_LINE
    };
    tesseract.psm = Some(verification_psm);
    let mut parts: Vec<ExtractedDocument> = Vec::with_capacity(EXTRA_CALLS);
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
        let read_config = if index == 1 { &verification } else { config };
        let read = backend.process_image_owned(Arc::new(png), read_config).await;
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
            && read
                .ocr_elements
                .as_ref()
                .and_then(|elements| {
                    let line = elements.iter().find(|element| element.level == OcrElementLevel::Line)?;
                    owned_box(line, read.content.trim(), &rows, band, source.width).map(|line| line.columns)
                })
                .as_ref()
                != Some(&columns)
        {
            return Ok(None);
        }
        if index < 2 {
            let expected_psm = if index == 1 { verification_psm } else { primary_psm };
            if !matches!(&read.metadata.format, Some(crate::types::FormatMetadata::Ocr(metadata)) if metadata.psm == expected_psm)
                || parts
                    .first()
                    .is_some_and(|first| first.content.trim() != read.content.trim())
            {
                return Ok(None);
            }
        }
        parts.push(read);
    }
    let Ok([line, _, before, after]) = <[_; EXTRA_CALLS]>::try_from(parts) else {
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
        || !joined_line(first, second, line.content.trim(), inserted_letters)
    {
        return Ok(None);
    }
    Ok(Some([before, line, after]))
}

fn joined_line(first: &str, second: &str, combined: &str, inserted_letters: usize) -> bool {
    let Some((prefix, tail)) = first.rsplit_once(char::is_whitespace) else {
        return false;
    };
    let Some((new_prefix, new_tail)) = combined.rsplit_once(char::is_whitespace) else {
        return false;
    };
    prefix == new_prefix
        && new_tail
            .strip_prefix(tail)
            .and_then(|rest| rest.strip_suffix(second))
            .is_some_and(|middle| {
                middle.len() == inserted_letters && middle.bytes().all(|byte| byte.is_ascii_alphabetic())
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LayoutBackend(Arc<std::sync::Mutex<Vec<i32>>>);

    impl crate::plugins::Plugin for LayoutBackend {
        fn name(&self) -> &str {
            "split-row-layout-control"
        }
        fn version(&self) -> String {
            "1".into()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl OcrBackend for LayoutBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(&self, png: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            let height = image::load_from_memory(png).unwrap().height();
            let psm = config.tesseract_config.as_ref().unwrap().psm.unwrap();
            self.0.lock().unwrap().push(psm);
            let line = match height {
                22 => ("keep the railing", 1, 20, 8),
                38 if psm == 7 => ("keep the raiding", 1, 20, 16),
                38 => ("keep the railing", 1, 20, 16),
                16 => ("before", 1, 20, 4),
                36 => ("after", 1, 20, 11),
                _ => panic!("unexpected crop height {height}"),
            };
            let mut result = report(height, &[line]);
            result.metadata.format = Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
                language: "eng".into(),
                psm,
                output_format: "text".into(),
                table_count: 0,
                table_rows: None,
                table_cols: None,
            }));
            Ok(result)
        }
    }

    fn report(height: u32, lines: &[(&str, u32, u32, u32)]) -> ExtractedDocument {
        use crate::ocr_metadata_keys::{
            OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
        };
        let mut result = ExtractedDocument {
            content: lines.iter().map(|line| line.0).collect::<Vec<_>>().join("\n\n"),
            ocr_elements: Some(
                lines
                    .iter()
                    .map(|&(text, left, right, top)| {
                        crate::types::OcrElement::new(
                            text,
                            OcrBoundingGeometry::Rectangle {
                                left,
                                top,
                                width: right - left,
                                height: 6,
                            },
                            crate::types::OcrConfidence {
                                detection: None,
                                recognition: 0.99,
                            },
                        )
                    })
                    .collect(),
            ),
            ..Default::default()
        };
        result
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), 32.into());
        result
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), height.into());
        result
    }

    #[tokio::test]
    async fn split_row_adoption_refuses_same_count_spelling_disagreement_from_single_line_layout() {
        let mut source = blank_source();
        source.height = 90;
        source.samples.resize(32 * 90, 255);
        for y in 32..38 {
            for x in [10, 12, 13, 15, 17] {
                source.samples[y * 32 + x] = 0;
            }
        }
        let result = report(
            90,
            &[
                ("before", 1, 20, 4),
                ("keep the ra", 1, 11, 32),
                ("ing", 17, 20, 32),
                ("after", 1, 20, 65),
            ],
        );
        let modes = Arc::new(std::sync::Mutex::new(Vec::new()));
        let backend: Arc<dyn OcrBackend> = Arc::new(LayoutBackend(Arc::clone(&modes)));
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(11),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            reread(
                &source,
                &(0..90),
                &[4..10, 32..38, 65..71],
                &result,
                (&backend, &config),
                || false
            )
            .await
            .unwrap()
            .is_none()
        );
        assert_eq!(*modes.lock().unwrap(), [11, 7]);
    }

    #[test]
    fn agreed_reread_cannot_rewrite_an_unchanged_prefix_or_drop_the_fragment() {
        assert!(joined_line("leave this ra", "in", "leave this raisin", 2));
        assert!(!joined_line("leave this ra", "in", "leave that raisin", 2));
        assert!(!joined_line("leave this ra", "in", "leave this raise", 2));
        assert!(!joined_line("leave this ra", "in", "leave this rain", 2));
    }

    #[test]
    fn agreeing_reads_cannot_insert_two_letters_with_only_one_ink_run() {
        let mut source = blank_source();
        for y in 3..9 {
            for x in [10, 12, 13, 14, 16] {
                source.samples[y * 32 + x] = 0;
            }
        }
        let letters = join_ink(&source, &(2..10), &(2..10), 11, 16).unwrap();
        assert_eq!(letters, 1);
        assert!(!joined_line("keep the ra", "ing", "keep the railing", letters));
    }

    #[test]
    fn staggered_ink_in_one_band_does_not_link_fragments() {
        let mut source = blank_source();
        for y in 2..5 {
            for x in [9, 10, 12, 13] {
                source.samples[y * 32 + x] = 0;
            }
        }
        for y in 7..10 {
            for x in [15, 16, 18, 19] {
                source.samples[y * 32 + x] = 0;
            }
        }
        assert_eq!(join_ink(&source, &(2..10), &(2..10), 11, 18), None);
    }

    fn blank_source() -> Source {
        Source {
            width: 32,
            height: 12,
            dpi: 120.0,
            samples: vec![255; 32 * 12],
            retained_bytes: 32 * 12,
            certify_rows: true,
            reread_split_row: true,
        }
    }

    #[test]
    fn same_band_fragment_requires_ink_between_both_recognized_edges() {
        let mut source = blank_source();
        for y in 3..9 {
            for x in [9, 10, 13, 14, 17, 18, 19] {
                source.samples[y * 32 + x] = 0;
            }
        }
        assert_eq!(join_ink(&source, &(2..10), &(2..10), 11, 17), Some(1));
        for y in 3..9 {
            source.samples[y * 32 + 13..y * 32 + 15].fill(255);
        }
        assert_eq!(join_ink(&source, &(2..10), &(2..10), 11, 17), None);
        assert_eq!(join_ink(&source, &(2..10), &(2..10), 11, 22), None);
    }
}
