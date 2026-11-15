//! Plan line-local edits from two agreeing, positioned source reads.

use std::ops::Range;

#[path = "region_projection.rs"]
pub(super) mod projection;

const MAX_TRANSCRIPT_BYTES: usize = 16 * 1024;
const MAX_LINE_WORDS: usize = 64;
const MAX_PROTECTED_BOXES: usize = 1024;
const MIN_SOURCE_CONFIDENCE: f64 = 85.0;

#[derive(Clone, Copy)]
pub(super) struct RegionBox {
    pub(super) left: u32,
    pub(super) top: u32,
    pub(super) right: u32,
    pub(super) bottom: u32,
}

impl RegionBox {
    fn valid(self) -> bool {
        self.left < self.right && self.top < self.bottom
    }

    fn contains(self, inner: Self) -> bool {
        inner.valid()
            && self.left <= inner.left
            && self.top <= inner.top
            && inner.right <= self.right
            && inner.bottom <= self.bottom
    }

    fn overlap(self, other: Self) -> (u32, u32) {
        (
            self.right.min(other.right).saturating_sub(self.left.max(other.left)),
            self.bottom.min(other.bottom).saturating_sub(self.top.max(other.top)),
        )
    }

    fn intersects(self, other: Self) -> bool {
        let (width, height) = self.overlap(other);
        width > 0 && height > 0
    }

    fn covers_stock(self, stock: Self) -> bool {
        let (width, height) = self.overlap(stock);
        width >= (stock.right - stock.left).div_ceil(2) && height >= (stock.bottom - stock.top).div_ceil(2)
    }
}

pub(super) struct StockWord<'a> {
    pub(super) transcript_bytes: Option<Range<usize>>,
    pub(super) text: &'a str,
    pub(super) bounds: RegionBox,
}

pub(super) struct SourceWord<'a> {
    pub(super) text: &'a str,
    pub(super) bounds: RegionBox,
    pub(super) confidence: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct OccurrenceEdit<'a> {
    pub(super) bytes: Range<usize>,
    pub(super) expected: &'a str,
    pub(super) replacement: &'a str,
}

fn ordered_line(region: RegionBox, words: impl Iterator<Item = RegionBox>) -> bool {
    let (mut right, mut top, mut bottom) = (region.left, region.top, region.bottom);
    for word in words {
        if !region.contains(word) || word.left < right {
            return false;
        }
        right = word.right;
        top = top.max(word.top);
        bottom = bottom.min(word.bottom);
    }
    top < bottom
}

fn verified_token(transcript: &str, word: &StockWord<'_>) -> Option<Range<usize>> {
    let bytes = word.transcript_bytes.clone()?;
    let text = transcript.get(bytes.clone())?;
    if text != word.text || text.is_empty() || text.chars().any(char::is_whitespace) {
        return None;
    }
    let before = transcript.get(..bytes.start)?.chars().next_back();
    let after = transcript.get(bytes.end..)?.chars().next();
    (before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)).then_some(bytes)
}

fn read_owns_stock(region: RegionBox, stock: &[StockWord<'_>], source: &[SourceWord<'_>]) -> bool {
    source.len() == stock.len()
        && ordered_line(region, source.iter().map(|word| word.bounds))
        && source.iter().enumerate().all(|(index, word)| {
            (MIN_SOURCE_CONFIDENCE..=100.0).contains(&word.confidence)
                && !word.text.is_empty()
                && word.text.chars().all(char::is_alphabetic)
                && word.bounds.covers_stock(stock[index].bounds)
                && stock
                    .iter()
                    .enumerate()
                    .all(|(other, owner)| other == index || !word.bounds.intersects(owner.bounds))
        })
}

/// All boxes must be projected into the same source frame. The caller supplies the complete
/// stock line, both complete source reads, and every other stock or protected box that could
/// overlap the region. Stock byte ranges must come from the caller's verified occurrence-to-
/// transcript projection for this exact transcript. Missing ranges cannot be inferred from text.
/// The column bounds must exclude gutters and other columns. ~keep
fn plan_line_edits<'a>(
    transcript: &str,
    region: RegionBox,
    column: RegionBox,
    stock: &[StockWord<'a>],
    sources: [&[SourceWord<'a>]; 2],
    protected: &[RegionBox],
) -> Option<Vec<OccurrenceEdit<'a>>> {
    if transcript.len() > MAX_TRANSCRIPT_BYTES
        || !column.contains(region)
        || !(1..=MAX_LINE_WORDS).contains(&stock.len())
        || protected.len() > MAX_PROTECTED_BOXES
        || protected
            .iter()
            .any(|&bounds| !bounds.valid() || region.intersects(bounds))
        || !ordered_line(region, stock.iter().map(|word| word.bounds))
        || !sources.iter().all(|words| read_owns_stock(region, stock, words))
    {
        return None;
    }
    let [source, confirmation] = sources;
    let mut previous_end = None;
    let mut line_breaks = 0;
    let mut edits = Vec::new();
    for ((original, candidate), confirm) in stock.iter().zip(source).zip(confirmation) {
        if candidate.text != confirm.text || !candidate.bounds.intersects(confirm.bounds) {
            return None;
        }
        let bytes = verified_token(transcript, original)?;
        if let Some(end) = previous_end {
            let gap = transcript.get(end..bytes.start)?;
            if gap.is_empty() || !gap.chars().all(char::is_whitespace) {
                return None;
            }
            line_breaks += gap.bytes().filter(|&byte| byte == b'\n').count();
            if line_breaks > 2 {
                return None;
            }
        }
        previous_end = Some(bytes.end);
        if original.text != candidate.text {
            edits.push(OccurrenceEdit {
                bytes,
                expected: original.text,
                replacement: candidate.text,
            });
        }
    }
    (!edits.is_empty()).then_some(edits)
}

#[cfg(test)]
#[path = "region_reconciliation_tests.rs"]
mod tests;
