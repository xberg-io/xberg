use super::{HocrWord, RuleWork, VerticalRule};
use crate::pdf::hierarchy::SegmentData;
use std::cell::OnceCell;
use std::collections::BTreeMap;

type WordKey<'a> = (&'a str, u32, u32, u32, u32);
#[derive(Clone, Copy)]
struct WordBounds {
    left: f64,
    right: f64,
    bottom: f64,
    top: f64,
}
type Bands<'a> = BTreeMap<WordKey<'a>, WordBounds>;

fn key(word: &HocrWord) -> WordKey<'_> {
    (&word.text, word.left, word.top, word.width, word.height)
}

pub(super) struct ExactRows<'a> {
    segments: &'a [SegmentData],
    words: &'a [HocrWord],
    sources: &'a [Option<usize>],
    bands: OnceCell<Option<Bands<'a>>>,
}

impl<'a> ExactRows<'a> {
    pub(super) fn new(segments: &'a [SegmentData], words: &'a [HocrWord], sources: &'a [Option<usize>]) -> Self {
        Self {
            segments,
            words,
            sources,
            bands: OnceCell::new(),
        }
    }

    pub(super) fn frame(&self, page_height: f32) -> Option<RuleFrame<'_>> {
        Some(RuleFrame {
            page_height,
            bands: Some(self.bands.get_or_init(|| self.bands()).as_ref()?),
        })
    }

    fn bands(&self) -> Option<Bands<'a>> {
        if self.words.len() != self.sources.len() {
            return None;
        }
        let mut work = RuleWork::default();
        let mut projected = BTreeMap::new();
        for (word, &source) in self.words.iter().zip(self.sources) {
            work.charge(word.text.len().saturating_add(1))?;
            let band = source
                .and_then(|index| self.segments.get(index))
                .and_then(segment_bounds);
            projected
                .entry(key(word))
                .and_modify(|state| *state = None)
                .or_insert(band);
        }
        Some(
            projected
                .into_iter()
                .filter_map(|(key, band)| Some((key, band?)))
                .collect(),
        )
    }
}

fn segment_bounds(segment: &SegmentData) -> Option<WordBounds> {
    let left = f64::from(segment.x);
    let bottom = f64::from(segment.y);
    let right = left + f64::from(segment.width);
    let top = bottom + f64::from(segment.height);
    (segment.is_unrotated()
        && [left, bottom, right, top].into_iter().all(f64::is_finite)
        && right > left
        && top > bottom)
        .then_some(WordBounds {
            left,
            right,
            bottom,
            top,
        })
}

#[derive(Clone, Copy)]
pub(super) struct RuleFrame<'a> {
    pub(super) page_height: f32,
    bands: Option<&'a Bands<'a>>,
}

impl From<f32> for RuleFrame<'_> {
    fn from(page_height: f32) -> Self {
        Self {
            page_height,
            bands: None,
        }
    }
}

impl RuleFrame<'_> {
    pub(super) fn covers_pair(self, rule: &VerticalRule, left: &HocrWord, right: &HocrWord) -> bool {
        let Some(bands) = self.bands else {
            let top = f64::from(self.page_height) - f64::from(left.top);
            let bottom = top - f64::from(left.height);
            return f64::from(rule.bottom) <= bottom && f64::from(rule.top) >= top;
        };
        let (Some(left), Some(right)) = (bands.get(&key(left)), bands.get(&key(right))) else {
            return false;
        };
        left.bottom == right.bottom
            && left.top == right.top
            && f64::from(rule.bottom) <= left.bottom
            && f64::from(rule.top) >= left.top
            && rule.covers_gap(left.right, right.left)
    }
}
