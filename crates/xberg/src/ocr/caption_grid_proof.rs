//! Certify an intact OCR figure caption that a grid heuristic split into cells.

use crate::extractors::security::SecurityLimits;
#[cfg(test)]
use crate::types::formats::OcrTable;
use crate::types::formats::OcrTableBoundingBox;
use crate::types::{OcrBoundingGeometry, OcrElement, OcrElementLevel};

#[path = "caption_grid_paint.rs"]
mod paint;
pub(crate) use paint::CaptionPaintProof;

pub(crate) const MAX_ELEMENTS: usize = 1024;
const MAX_EVIDENCE_BYTES: usize = 512 * 1024;
const MIN_LINES: usize = 4;
const MAX_LINES: usize = 12;
const MAX_COLUMNS: usize = 6;
const MIN_SPLIT_ROWS: usize = 2;
const MARGIN_TOLERANCE_PX: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Rect {
    fn table(bounds: OcrTableBoundingBox) -> Self {
        Self {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        }
    }

    fn element(geometry: &OcrBoundingGeometry) -> Option<Self> {
        let &OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = geometry
        else {
            return None;
        };
        Some(Self {
            left,
            top,
            right: left.checked_add(width)?,
            bottom: top.checked_add(height)?,
        })
    }

    fn within_frame(self, (width, height): (u32, u32)) -> bool {
        self.left < self.right && self.top < self.bottom && self.right <= width && self.bottom <= height
    }

    fn contains(self, other: Self) -> bool {
        self.left <= other.left && self.top <= other.top && self.right >= other.right && self.bottom >= other.bottom
    }

    fn intersects(self, other: Self) -> bool {
        self.left < other.right && other.left < self.right && self.top < other.bottom && other.top < self.bottom
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

struct SourceElement<'a> {
    text: &'a str,
    bounds: Rect,
}

pub(crate) struct CaptionGridProof<'a> {
    lines: Vec<&'a str>,
    pub(crate) paint: CaptionPaintProof,
}

impl<'a> CaptionGridProof<'a> {
    pub(crate) fn lines(&self) -> &[&'a str] {
        &self.lines
    }
}

fn remaining_evidence_work(cells: &[Vec<String>], elements: &[OcrElement], limits: &SecurityLimits) -> Option<usize> {
    if elements.len() > MAX_ELEMENTS || !(MIN_LINES..=MAX_LINES).contains(&cells.len()) {
        return None;
    }
    let columns = cells[0].len();
    if !(2..=MAX_COLUMNS).contains(&columns) || cells.iter().any(|row| row.len() != columns) {
        return None;
    }
    let max_bytes = MAX_EVIDENCE_BYTES.min(limits.max_content_size);
    let mut text_lengths = cells
        .iter()
        .flatten()
        .map(String::len)
        .chain(elements.iter().map(|e| e.text.len()));
    let bytes = text_lengths.try_fold(0usize, |bytes, length| {
        bytes.checked_add(length).filter(|total| *total <= max_bytes)
    })?;
    // ~keep Bound text passes, per-row geometry scans, and sorting before allocating evidence.
    const MAX_GAPS: usize = MAX_LINES * (MAX_COLUMNS - 1);
    let geometry_work = elements.len() * (MAX_LINES + MAX_ELEMENTS.ilog2() as usize + 2 * MAX_GAPS + 8);
    limits
        .max_iterations
        .checked_sub(bytes * 8 + geometry_work + MAX_GAPS * MAX_GAPS)
}

fn collect_sources<'a>(
    elements: &'a [OcrElement],
    page: u32,
    region: Rect,
    frame: (u32, u32),
) -> Option<(Vec<SourceElement<'a>>, Vec<SourceElement<'a>>)> {
    let mut lines = Vec::new();
    let mut words = Vec::new();
    for element in elements {
        let collection = match element.level {
            OcrElementLevel::Line => &mut lines,
            OcrElementLevel::Word => &mut words,
            _ => continue,
        };
        let bounds = Rect::element(&element.geometry)?;
        if element.page_number != page || !bounds.within_frame(frame) {
            return None;
        }
        if bounds.intersects(region) {
            if !region.contains(bounds) {
                return None;
            }
            collection.push(SourceElement {
                text: &element.text,
                bounds,
            });
        }
    }
    Some((lines, words))
}

fn follows_wrapped_caption_shape(lines: &[SourceElement<'_>]) -> bool {
    let [first, middle @ .., last] = lines else {
        return false;
    };
    let mut label = first.text.split_whitespace();
    let figure = label.next() == Some("Figure")
        && label
            .next()
            .and_then(|value| value.strip_suffix(':'))
            .is_some_and(|index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
        && label.next().is_some();
    let first_width = first.bounds.right - first.bounds.left;
    let last_width = last.bounds.right - last.bounds.left;
    figure
        && last.text.trim_end().ends_with('.')
        && last_width <= first_width / 2
        && last.bounds.left.abs_diff(first.bounds.left) <= MARGIN_TOLERANCE_PX
        && middle.iter().all(|line| {
            line.bounds.left.abs_diff(first.bounds.left) <= MARGIN_TOLERANCE_PX
                && line.bounds.right.abs_diff(first.bounds.right) <= MARGIN_TOLERANCE_PX
        })
        && lines.windows(2).all(|pair| {
            let previous = pair[0].bounds;
            let next = pair[1].bounds;
            previous.bottom <= next.top && next.top - previous.bottom <= previous.bottom - previous.top
        })
}

fn row_owners<'a, 'text>(
    line: &SourceElement<'_>,
    words: &'a [SourceElement<'text>],
) -> Option<Vec<&'a SourceElement<'text>>> {
    let mut owned: Vec<_> = words
        .iter()
        .filter(|word| line.bounds.intersects(word.bounds))
        .collect();
    owned.sort_unstable_by_key(|word| word.bounds.left);
    if !owned.iter().all(|word| line.bounds.contains(word.bounds))
        || owned.iter().map(|word| word.bounds).reduce(Rect::union) != Some(line.bounds)
        || owned.windows(2).any(|pair| pair[0].bounds.right >= pair[1].bounds.left)
        || !owned.iter().map(|word| word.text).eq(line.text.split_whitespace())
    {
        return None;
    }
    Some(owned)
}

#[derive(Clone, Copy)]
struct Gap {
    left: u32,
    right: u32,
}

fn row_cell_cuts<'a>(
    cells: &[String],
    words: &[&'a SourceElement<'a>],
    line: Rect,
) -> Option<Vec<(Gap, &'a SourceElement<'a>, &'a SourceElement<'a>)>> {
    let nonempty = cells.iter().filter(|cell| !cell.trim().is_empty());
    if !nonempty
        .clone()
        .flat_map(|cell| cell.split_whitespace())
        .eq(words.iter().map(|word| word.text))
    {
        return None;
    }
    let mut cursor = 0;
    let mut cuts = Vec::new();
    for cell in nonempty {
        if cursor > 0 {
            let gap = Gap {
                left: words[cursor - 1].bounds.right,
                right: words[cursor].bounds.left,
            };
            if gap.right - gap.left >= line.bottom - line.top {
                return None;
            }
            cuts.push((gap, words[cursor - 1], words[cursor]));
        }
        cursor += cell.split_whitespace().count();
    }
    Some(cuts)
}

fn word_gap_proof(
    lines: &[SourceElement<'_>],
    words: &[SourceElement<'_>],
    cells: &[Vec<String>],
    region: Rect,
    remaining_work: usize,
) -> Option<CaptionPaintProof> {
    let mut previous_cuts: Vec<Gap> = Vec::new();
    let mut split_rows = 0;
    let mut owned_words = 0;
    let mut paint = CaptionPaintProof::new(
        region,
        lines.iter().map(|line| line.bounds.bottom - line.bounds.top).min()?,
        remaining_work,
    );
    for (line, row) in lines.iter().zip(cells) {
        let owned = row_owners(line, words)?;
        owned_words += owned.len();
        let cuts = row_cell_cuts(row, &owned, line.bounds)?;
        if cuts.iter().any(|(cut, _, _)| {
            previous_cuts
                .iter()
                .any(|previous| cut.left < previous.right && previous.left < cut.right)
        }) {
            return None;
        }
        split_rows += usize::from(!cuts.is_empty());
        for (gap, left, right) in cuts {
            paint.push(line.bounds, left, right, words);
            previous_cuts.push(gap);
        }
    }
    (owned_words == words.len() && split_rows >= MIN_SPLIT_ROWS).then_some(paint)
}

/// Verify line and word ownership in the same frame as the table.
/// The caller must establish that the complete evidence belongs to the accepted OCR attempt.
pub(crate) fn certify_figure_caption_rows<'a>(
    cells: &[Vec<String>],
    page: u32,
    bounds: OcrTableBoundingBox,
    elements: &'a [OcrElement],
    frame: (u32, u32),
    limits: &SecurityLimits,
) -> Option<CaptionGridProof<'a>> {
    let region = Rect::table(bounds);
    if page == 0 || !region.within_frame(frame) {
        return None;
    }
    let remaining_work = remaining_evidence_work(cells, elements, limits)?;
    let (mut lines, words) = collect_sources(elements, page, region, frame)?;
    lines.sort_unstable_by_key(|line| line.bounds.top);
    if lines.len() != cells.len()
        || lines.iter().map(|line| line.bounds).reduce(Rect::union) != Some(region)
        || !follows_wrapped_caption_shape(&lines)
    {
        return None;
    }
    let paint = word_gap_proof(&lines, &words, cells, region, remaining_work)?;
    Some(CaptionGridProof {
        lines: lines.into_iter().map(|line| line.text).collect(),
        paint,
    })
}

#[cfg(test)]
fn certify_figure_caption<'a>(
    table: &OcrTable,
    elements: &'a [OcrElement],
    frame: (u32, u32),
    limits: &SecurityLimits,
) -> Option<CaptionGridProof<'a>> {
    certify_figure_caption_rows(
        &table.cells,
        table.page_number,
        table.bounding_box?,
        elements,
        frame,
        limits,
    )
}

#[cfg(test)]
#[path = "caption_grid_proof_tests.rs"]
mod tests;
