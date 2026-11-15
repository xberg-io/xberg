//! Bind stock occurrences to the byte ranges produced during transcript serialization.

use super::{MAX_LINE_WORDS, MAX_TRANSCRIPT_BYTES, OccurrenceEdit, RegionBox, SourceWord, StockWord, plan_line_edits};
use std::ops::Range;

const MAX_STOCK_OCCURRENCES: usize = 1024;

pub(in super::super) struct RawStockWord<'a> {
    pub(in super::super) text: &'a str,
    pub(in super::super) bounds: RegionBox,
}

pub(in super::super) struct ProjectionWriter<'a> {
    words: &'a [RawStockWord<'a>],
    ranges: Vec<Option<Range<usize>>>,
    transcript: String,
}

pub(in super::super) struct VerifiedProjection<'a> {
    words: &'a [RawStockWord<'a>],
    ranges: Vec<Option<Range<usize>>>,
    transcript: &'a str,
}

impl<'a> ProjectionWriter<'a> {
    pub(in super::super) fn new(words: &'a [RawStockWord<'a>]) -> Option<Self> {
        if words.len() > MAX_STOCK_OCCURRENCES {
            return None;
        }
        Some(Self {
            words,
            ranges: vec![None; words.len()],
            transcript: String::new(),
        })
    }

    /// Record the occurrence selected by the serializer at the point it emits that word.
    /// The occurrence is its index in the same positioned stock word list used for selection. ~keep
    pub(in super::super) fn append_stock(&mut self, occurrence: usize) -> Option<()> {
        let word = self.words.get(occurrence)?;
        if self.ranges.get(occurrence)?.is_some() || word.text.is_empty() || word.text.chars().any(char::is_whitespace)
        {
            return None;
        }
        let start = self.transcript.len();
        self.append_unowned(word.text)?;
        self.ranges[occurrence] = Some(start..self.transcript.len());
        Some(())
    }

    /// Emit separators, table text, or other bytes that do not own a positioned stock word. ~keep
    pub(in super::super) fn append_unowned(&mut self, text: &str) -> Option<()> {
        if self.transcript.len().checked_add(text.len())? > MAX_TRANSCRIPT_BYTES {
            return None;
        }
        self.transcript.push_str(text);
        Some(())
    }

    pub(in super::super) fn verify(self, transcript: &'a str) -> Option<VerifiedProjection<'a>> {
        (self.transcript == transcript).then_some(VerifiedProjection {
            words: self.words,
            ranges: self.ranges,
            transcript,
        })
    }
}

impl<'a> VerifiedProjection<'a> {
    pub(in super::super) fn plan(
        &self,
        region: RegionBox,
        column: RegionBox,
        occurrences: &[usize],
        sources: [&[SourceWord<'a>]; 2],
        protected: &[RegionBox],
    ) -> Option<Vec<OccurrenceEdit<'a>>> {
        let stock = self.line(occurrences)?;
        plan_line_edits(self.transcript, region, column, &stock, sources, protected)
    }

    fn line(&self, occurrences: &[usize]) -> Option<Vec<StockWord<'a>>> {
        if !(1..=MAX_LINE_WORDS).contains(&occurrences.len()) {
            return None;
        }
        let mut previous_end = 0;
        occurrences
            .iter()
            .map(|&id| {
                let word = self.words.get(id)?;
                let bytes = self.ranges.get(id)?.as_ref()?.clone();
                if bytes.start < previous_end {
                    return None;
                }
                previous_end = bytes.end;
                Some(StockWord {
                    transcript_bytes: Some(bytes),
                    text: word.text,
                    bounds: word.bounds,
                })
            })
            .collect()
    }
}
