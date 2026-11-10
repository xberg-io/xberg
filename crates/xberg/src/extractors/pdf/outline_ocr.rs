use crate::core::config::{ExtractionConfig, OcrStrategy};
use crate::types::PageBoundary;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::elements::{PathContent, PathOperation};
use xberg_native_pdf::geometry::Rect;

const MAX_NATIVE_NONSPACE: usize = 200;
const MAX_CANDIDATE_PAGES: usize = 32;
// An outlined 12 pt serif row costs about 1600 operators and 55 KiB; the budget holds a full page of rows. ~keep
const MAX_PAGE_BYTES: usize = 6 * 1024 * 1024;
const MAX_PAGE_OPERATORS: usize = 160_000;
const MIN_CONTOURS_PER_ROW: usize = 16;
const MIN_PROSE_ROWS: usize = 3;
const MIN_CONTOURS_PER_NATIVE_CHARACTER: usize = 2;
const MIN_ROW_WIDTH_IN_HEIGHTS: f32 = 12.0;
const BODY_CONTOUR_WIDTH_PT: std::ops::RangeInclusive<f32> = 1.0..=18.0;
const BODY_CONTOUR_HEIGHT_PT: std::ops::RangeInclusive<f32> = 4.0..=18.0;

pub(super) fn candidates(
    doc: &PdfDocument,
    config: &ExtractionConfig,
    text: &str,
    boundaries: &[PageBoundary],
) -> Vec<u32> {
    if config.effective_disable_ocr()
        || config.ocr_strategy != OcrStrategy::Auto
        || config.force_ocr
        || config.force_ocr_pages.is_some()
        || !super::near_empty_ocr_fallback_applies(config, text)
    {
        return Vec::new();
    }
    let mut raster = super::ocr::VisibleRasterProof::new(config);
    boundaries
        .iter()
        .take_while(|_| config.cancel_token.as_ref().is_none_or(|token| !token.is_cancelled()))
        .filter_map(|boundary| {
            let page_text = text.get(boundary.byte_start..boundary.byte_end)?;
            let nonspace = page_text
                .chars()
                .filter(|c| !c.is_whitespace())
                .take(MAX_NATIVE_NONSPACE + 1)
                .count();
            (1..=MAX_NATIVE_NONSPACE)
                .contains(&nonspace)
                .then_some((boundary.page_number, nonspace))
        })
        .take(MAX_CANDIDATE_PAGES)
        .filter_map(|(page, nonspace)| {
            let index = page.checked_sub(1)? as usize;
            let outlined = outline_rows(doc, index, nonspace).is_some_and(|rows| rows >= MIN_PROSE_ROWS);
            (outlined || raster.as_mut().is_some_and(|proof| proof.has_prose(doc, index))).then_some(page)
        })
        .collect()
}

fn outline_rows(doc: &PdfDocument, index: usize, native_nonspace: usize) -> Option<usize> {
    if doc.get_page_rotation(index).ok()? != 0 {
        return None;
    }
    let (left, bottom, right, top) = doc.get_page_media_box(index).ok()?;
    let page_bounds = Rect::new(left, bottom, right - left, top - bottom);
    let paths = doc
        .extract_bounded_page_local_paths(index, MAX_PAGE_BYTES, MAX_PAGE_OPERATORS)
        .ok()?;
    let contours = filled_contours(&paths);
    let min_contours = (MIN_CONTOURS_PER_ROW * MIN_PROSE_ROWS).max(native_nonspace * MIN_CONTOURS_PER_NATIVE_CHARACTER);
    if contours.len() < min_contours {
        return None;
    }

    let native_boxes: Vec<_> = doc
        .extract_chars(index)
        .ok()?
        .into_iter()
        .filter(|glyph| !glyph.char.is_whitespace())
        .map(|glyph| glyph.bbox)
        .take(MAX_NATIVE_NONSPACE + 1)
        .collect();
    if !(1..=MAX_NATIVE_NONSPACE).contains(&native_boxes.len()) {
        return None;
    }

    let rows = aligned_rows(contours);
    Some(
        rows.into_iter()
            .filter(|(row, count)| {
                *count >= MIN_CONTOURS_PER_ROW
                    && row.width >= MIN_ROW_WIDTH_IN_HEIGHTS * row.height
                    && page_bounds.contains_rect(row)
                    && native_boxes.iter().all(|bounds| !row.intersects(bounds))
            })
            .count(),
    )
}

fn filled_contours(paths: &[PathContent]) -> Vec<Rect> {
    let mut contours = Vec::new();
    for path in paths.iter().filter(|path| path.layer.is_none() && path.has_fill()) {
        let starts = path
            .operations
            .iter()
            .enumerate()
            .filter_map(|(index, op)| matches!(op, PathOperation::MoveTo(..)).then_some(index));
        for start in starts {
            let contour = &path.operations[start..];
            if let Some(bounds) = contour_bounds(contour) {
                contours.push(bounds);
            }
        }
    }
    contours
}

fn contour_bounds(operations: &[PathOperation]) -> Option<Rect> {
    let mut points = Vec::new();
    let mut curved = false;
    for op in operations {
        match *op {
            PathOperation::MoveTo(x, y) if points.is_empty() => points.push((x, y)),
            PathOperation::LineTo(x, y) => points.push((x, y)),
            PathOperation::CurveTo(ax, ay, bx, by, x, y) => {
                curved = true;
                points.extend([(ax, ay), (bx, by), (x, y)]);
            }
            PathOperation::ClosePath if curved => return glyph_sized_bounds(&points),
            _ => return None,
        }
    }
    None
}

fn glyph_sized_bounds(points: &[(f32, f32)]) -> Option<Rect> {
    let (mut left, mut bottom, mut right, mut top) =
        (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for &(x, y) in points {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        left = left.min(x);
        bottom = bottom.min(y);
        right = right.max(x);
        top = top.max(y);
    }
    let (width, height) = (right - left, top - bottom);
    // Body-sized contour evidence, excluding rules, accents and large outlined artwork. ~keep
    (BODY_CONTOUR_WIDTH_PT.contains(&width) && BODY_CONTOUR_HEIGHT_PT.contains(&height))
        .then(|| Rect::new(left, bottom, width, height))
}

fn aligned_rows(mut contours: Vec<Rect>) -> Vec<(Rect, usize)> {
    contours.sort_unstable_by(|left, right| left.y.total_cmp(&right.y));
    let mut rows: Vec<(Rect, usize)> = Vec::new();
    for contour in contours {
        if let Some((bounds, count)) = rows.last_mut()
            && contour.y < bounds.y + bounds.height
        {
            *bounds = bounds.union(&contour);
            *count += 1;
        } else {
            rows.push((contour, 1));
        }
    }
    rows
}
