// Vendored from yake-rust 1.0.3 (MIT) — https://github.com/quesurifn/yake-rust
// Replaced hashbrown::HashMap with ahash::AHashMap.

use ahash::AHashMap;

use super::counter::Counter;

/// Stats for a single term against its neighbors.
#[derive(Default)]
pub(crate) struct PairwiseFreq<'s> {
    /// How often the term follows another: `A..T`
    follows: Counter<&'s str>,
    /// How often the term is followed by another: `T..A`
    followed_by: Counter<&'s str>,
}

#[derive(Default)]
pub(crate) struct Contexts<'s> {
    map: AHashMap<&'s str, PairwiseFreq<'s>>,
}

impl<'s> Contexts<'s> {
    /// Record a co-occurrence: `left` appears before `right`.
    #[inline]
    pub fn track(&mut self, left: &'s str, right: &'s str) {
        self.map.entry(right).or_default().follows.inc(left);
        self.map.entry(left).or_default().followed_by.inc(right);
    }

    /// The total number of cases where `term` is followed by `by`: `term...by`.
    #[inline]
    pub fn cases_term_is_followed(&self, term: &str, by: &str) -> usize {
        self.map.get(term).map_or(0, |freq| freq.followed_by.get(&by))
    }

    /// Dispersion of the term's context (left, right).
    /// `0` = fixed expression, `1` = dispersive.
    pub fn dispersion_of(&self, term: &str) -> (f64, f64) {
        match self.map.get(term) {
            None => (0.0, 0.0),
            Some(PairwiseFreq { follows, followed_by }) => (
                if follows.is_empty() {
                    0.0
                } else {
                    follows.distinct() as f64 / follows.total() as f64
                },
                if followed_by.is_empty() {
                    0.0
                } else {
                    followed_by.distinct() as f64 / followed_by.total() as f64
                },
            ),
        }
    }
}
