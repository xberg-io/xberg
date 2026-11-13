//! Positional, sequence-aware page-number detection (GH#1411).
//!
//! A page number is not a *shape*, it is a *position plus a cross-page progression*.
//! The predicate this module replaces tested shape alone ("1-4 digits", "made of
//! `i`/`v`/`x` letters") and everything it matched was deleted, so table cells, list
//! markers, footnote references and stray capitals were silently dropped — the bare
//! string `I` alone matched 244 times across the 17-document regression corpus.
//!
//! The model here separates the two signals:
//!
//! * [`classify_page_number_text`] answers "could this string ever be a page number?"
//!   and attaches a deliberately modest shape-only confidence. A `Some` result never
//!   means "is a page number".
//! * [`PageNumberSequence`] accumulates candidates across every page and only raises
//!   confidence when the same margin position carries a near-monotonic run of values.
//!   That progression is what actually distinguishes page furniture from a table cell.
//!
//! Callers must gate deletion on [`PageNumberSequence::DELETION_THRESHOLD`]. Keeping a
//! stray page number costs one short line; deleting a table cell corrupts a document.
//!
//! ## Why the OCR path is not aligned with this (GH#1411 requirement 7)
//!
//! It has nothing to align. Page furniture is a native-PDF concept in this codebase:
//! `is_page_furniture` exists only on [`PdfParagraph`](super::types::PdfParagraph) and is
//! read only under `pdf/structure/`, and no OCR code path marks or deletes it. The
//! `page_number` fields under `crate::ocr` are page *indices* on OCR elements and tables,
//! not a furniture heuristic — nothing in the OCR pipeline drops a paragraph for looking
//! like a page number.
//!
//! So OCR output retains page numbers unconditionally, which is the conservative side of
//! the policy this module implements and cannot produce the data loss GH#1411 reported.
//! Should furniture removal ever be added to the OCR path, it must reuse
//! [`classify_page_number_text`] and [`PageNumberSequence`] rather than reintroduce a
//! shape-only predicate — that predicate is exactly what this module replaced.

use std::collections::BTreeMap;

// ── Band geometry ────────────────────────────────────────────────────────────────

/// Fraction of page height, measured from the top edge, that counts as the top margin.
///
/// Running heads sit inside the top ~1 inch of a US Letter / A4 page; 0.12 of an 11 in
/// page is 1.32 in, which covers a header plus its trailing whitespace without reaching
/// the first body line of a normally-margined document.
const TOP_BAND_MAX_Y_RATIO: f32 = 0.12;

/// Height fraction above which a paragraph counts as sitting in the bottom margin.
///
/// Mirrors [`TOP_BAND_MAX_Y_RATIO`] from the bottom edge; footers and folios live here.
const BOTTOM_BAND_MIN_Y_RATIO: f32 = 0.88;

/// Where on the page a paragraph sits. Page numbers live in the margins, never mid-body,
/// so the band is the first and cheapest filter against table and list false positives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarginBand {
    Top,
    Body,
    Bottom,
}

/// Classify a paragraph's vertical position into a band.
///
/// `y_ratio` is 0.0 at the top of the page and 1.0 at the bottom. A non-finite ratio
/// falls through to [`MarginBand::Body`], which is the safe answer because body-band
/// candidates can never reach a deletable confidence.
pub(crate) fn margin_band(y_ratio: f32) -> MarginBand {
    if y_ratio <= TOP_BAND_MAX_Y_RATIO {
        MarginBand::Top
    } else if y_ratio >= BOTTOM_BAND_MIN_Y_RATIO {
        MarginBand::Bottom
    } else {
        MarginBand::Body
    }
}

// ── Shape classification ─────────────────────────────────────────────────────────

/// Longest string still worth testing as a page number. `Page 1234 of 5678` is 17
/// characters, so 24 leaves headroom for padding and unusual separators while rejecting
/// prose outright.
const MAX_CANDIDATE_CHARS: usize = 24;

/// Maximum digits in an arabic page number. Four covers every realistic pagination
/// (up to 9999) without matching longer numeric runs such as identifiers or amounts.
const MAX_PAGE_DIGITS: usize = 4;

/// The literal keyword introducing the `Page N` and `Page N of M` conventions.
const PAGE_KEYWORD: &str = "page";

/// The separator in the `Page N of M` convention, matched case-insensitively.
const OF_SEPARATOR: &str = " of ";

/// Minimum characters in a dash-flanked folio (`-1-` is the shortest legal form).
const MIN_DASHED_CHARS: usize = 3;

/// Shape confidence for `Page N of M` — the only convention that is essentially
/// unambiguous, since no table cell or citation carries a total-page count.
const PAGE_N_OF_M_CONFIDENCE: f32 = 0.95;

/// Shape confidence for `Page N`. Nearly as safe as `Page N of M`; the keyword makes an
/// accidental match from body content very unlikely.
const PAGE_N_CONFIDENCE: f32 = 0.85;

/// Shape confidence for a dash-flanked folio (`- 5 -`). The flanking dashes are a
/// typesetting convention used almost exclusively for page numbers.
const DASHED_N_CONFIDENCE: f32 = 0.85;

/// Shape confidence for `N / M`. Strong, but the same shape appears in fractions, dates
/// and ratio-bearing table cells, so it sits below the keyword forms.
const N_SLASH_M_CONFIDENCE: f32 = 0.70;

/// Shape confidence for section-prefixed pagination (`3-12`). Also matches numeric
/// ranges and hyphenated identifiers, so it needs sequence evidence to survive.
const SECTION_PREFIXED_CONFIDENCE: f32 = 0.45;

/// Shape confidence for `[N]`. Deliberately low: bracketed integers are overwhelmingly
/// citation or footnote references in the corpora this runs on.
const BRACKETED_N_CONFIDENCE: f32 = 0.35;

/// Shape confidence for bare digits — the single largest source of the old predicate's
/// false positives (every numeric table cell). Shape alone must be near worthless here.
const BARE_DIGITS_CONFIDENCE: f32 = 0.30;

/// Shape confidence for a multi-character lowercase roman numeral, the standard
/// front-matter convention (`ii`, `xiv`).
const LOWERCASE_ROMAN_CONFIDENCE: f32 = 0.45;

/// Shape confidence for a multi-character uppercase roman numeral. Uppercase romans are
/// also used for section and volume labels, so they score below the lowercase form.
const UPPERCASE_ROMAN_CONFIDENCE: f32 = 0.35;

/// Shape confidence for a single roman letter (`I`, `i`, `V`, `X`).
///
/// This is the corpus's worst offender: `I` alone produced 244 false matches as a
/// pronoun, an initial, a list marker and a column label. Only overwhelming sequence
/// evidence should ever let one of these be deleted.
const SINGLE_LETTER_ROMAN_CONFIDENCE: f32 = 0.15;

/// The pagination conventions this module recognises. Each corresponds to one shape rule
/// in [`classify_page_number_text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageNumberConvention {
    BareDigits,
    PageN,
    PageNofM,
    NSlashM,
    DashedN,
    BracketedN,
    SectionPrefixed,
    Roman,
}

/// A string that *could* be a page number, together with how much its shape alone is
/// worth. The confidence is never sufficient to delete on its own — see
/// [`PageNumberSequence::confidence_at`].
#[derive(Debug, Clone)]
pub(crate) struct PageNumberCandidate {
    /// Parsed ordinal value where one could be recovered. `None` disables this candidate
    /// as sequence evidence but still records its position.
    pub value: Option<u32>,
    pub convention: PageNumberConvention,
    /// Shape-only confidence in `0.0..=1.0`. NEVER sufficient to delete alone.
    pub shape_confidence: f32,
}

/// Shape-only match. Returns `None` when the text cannot be a page number at all.
///
/// A `Some` result means "could be", never "is". Rules are tried most-specific first so
/// that `- 5 -` is read as a dashed folio rather than as section-prefixed `5`.
pub(crate) fn classify_page_number_text(text: &str) -> Option<PageNumberCandidate> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_CANDIDATE_CHARS {
        return None;
    }
    classify_page_n_of_m(trimmed)
        .or_else(|| classify_page_n(trimmed))
        .or_else(|| classify_dashed_n(trimmed))
        .or_else(|| classify_n_slash_m(trimmed))
        .or_else(|| classify_bracketed_n(trimmed))
        .or_else(|| classify_section_prefixed(trimmed))
        .or_else(|| classify_bare_digits(trimmed))
        .or_else(|| classify_roman(trimmed))
}

fn candidate(value: Option<u32>, convention: PageNumberConvention, shape_confidence: f32) -> PageNumberCandidate {
    PageNumberCandidate {
        value,
        convention,
        shape_confidence,
    }
}

/// Strip a leading case-insensitive `Page` keyword plus its separating space.
fn strip_page_keyword(text: &str) -> Option<&str> {
    let (head, rest) = text.split_at_checked(PAGE_KEYWORD.len())?;
    if !head.eq_ignore_ascii_case(PAGE_KEYWORD) {
        return None;
    }
    let separated = rest.strip_prefix(' ').or_else(|| rest.strip_prefix('\u{a0}'))?;
    Some(separated.trim())
}

fn classify_page_n_of_m(text: &str) -> Option<PageNumberCandidate> {
    let rest = strip_page_keyword(text)?;
    let lowered = rest.to_ascii_lowercase();
    let separator = lowered.find(OF_SEPARATOR)?;
    let current = parse_ordinal(rest.get(..separator)?.trim())?;
    let total = parse_ordinal(rest.get(separator + OF_SEPARATOR.len()..)?.trim())?;
    if total < current {
        return None;
    }
    Some(candidate(
        Some(current),
        PageNumberConvention::PageNofM,
        PAGE_N_OF_M_CONFIDENCE,
    ))
}

fn classify_page_n(text: &str) -> Option<PageNumberCandidate> {
    let value = parse_ordinal(strip_page_keyword(text)?)?;
    Some(candidate(Some(value), PageNumberConvention::PageN, PAGE_N_CONFIDENCE))
}

/// ASCII hyphen, en dash and em dash are all used to flank folios; the old predicate
/// only handled the first two and only with mandatory surrounding spaces.
fn is_dash(character: char) -> bool {
    matches!(character, '-' | '\u{2013}' | '\u{2014}')
}

fn classify_dashed_n(text: &str) -> Option<PageNumberCandidate> {
    if text.chars().count() < MIN_DASHED_CHARS {
        return None;
    }
    let first = text.chars().next()?;
    let last = text.chars().next_back()?;
    if !is_dash(first) || !is_dash(last) {
        return None;
    }
    let inner = text.get(first.len_utf8()..text.len() - last.len_utf8())?.trim();
    let value = parse_ordinal(inner)?;
    Some(candidate(
        Some(value),
        PageNumberConvention::DashedN,
        DASHED_N_CONFIDENCE,
    ))
}

fn classify_n_slash_m(text: &str) -> Option<PageNumberCandidate> {
    let (current, total) = text.split_once('/')?;
    let current = parse_ordinal(current.trim())?;
    let total = parse_ordinal(total.trim())?;
    if total < current {
        return None;
    }
    Some(candidate(
        Some(current),
        PageNumberConvention::NSlashM,
        N_SLASH_M_CONFIDENCE,
    ))
}

fn classify_bracketed_n(text: &str) -> Option<PageNumberCandidate> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let value = parse_ordinal(inner.trim())?;
    Some(candidate(
        Some(value),
        PageNumberConvention::BracketedN,
        BRACKETED_N_CONFIDENCE,
    ))
}

/// Section-prefixed pagination such as `3-12`: chapter 3, page 12. The ordinal that
/// progresses across pages is the second component.
fn classify_section_prefixed(text: &str) -> Option<PageNumberCandidate> {
    let (section, page) = text.split_once(['-', '\u{2013}'])?;
    if !is_digit_run(section.trim()) || !is_digit_run(page.trim()) {
        return None;
    }
    let value = page.trim().parse().ok()?;
    Some(candidate(
        Some(value),
        PageNumberConvention::SectionPrefixed,
        SECTION_PREFIXED_CONFIDENCE,
    ))
}

fn classify_bare_digits(text: &str) -> Option<PageNumberCandidate> {
    if !is_digit_run(text) {
        return None;
    }
    let value = text.parse().ok()?;
    Some(candidate(
        Some(value),
        PageNumberConvention::BareDigits,
        BARE_DIGITS_CONFIDENCE,
    ))
}

fn classify_roman(text: &str) -> Option<PageNumberCandidate> {
    let value = parse_roman_numeral(text)?;
    Some(candidate(
        Some(value),
        PageNumberConvention::Roman,
        roman_shape_confidence(text),
    ))
}

fn roman_shape_confidence(text: &str) -> f32 {
    if text.chars().count() == 1 {
        SINGLE_LETTER_ROMAN_CONFIDENCE
    } else if text.chars().all(|character| character.is_ascii_lowercase()) {
        LOWERCASE_ROMAN_CONFIDENCE
    } else {
        UPPERCASE_ROMAN_CONFIDENCE
    }
}

fn is_digit_run(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_PAGE_DIGITS && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Parse either an arabic or a roman ordinal — both appear after `Page`, inside brackets
/// and between dashes, and front matter mixes the two within one document.
fn parse_ordinal(text: &str) -> Option<u32> {
    if is_digit_run(text) {
        return text.parse().ok();
    }
    parse_roman_numeral(text)
}

// ── Roman numerals ───────────────────────────────────────────────────────────────

/// Largest value representable in standard (non-overlined) roman notation. `MMMM` and
/// anything longer is malformed, which is why the parser rejects it.
const MAX_ROMAN_VALUE: u32 = 3999;

/// Longest well-formed roman numeral, `MMMDCCCLXXXVIII` (3888) at 15 characters. Longer
/// input cannot be valid and is rejected before any parsing work.
const MAX_ROMAN_CHARS: usize = 15;

/// Greedy canonical-form table, largest value first, including the four subtractive
/// pairs. Rendering through this table and comparing is what enforces well-formedness.
const ROMAN_CANONICAL_TABLE: [(u32, &str); 13] = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

fn roman_letter_value(letter: char) -> Option<u32> {
    match letter {
        'I' => Some(1),
        'V' => Some(5),
        'X' => Some(10),
        'L' => Some(50),
        'C' => Some(100),
        'D' => Some(500),
        'M' => Some(1000),
        _ => None,
    }
}

/// Render `value` in canonical roman notation.
fn to_canonical_roman(value: u32) -> String {
    let mut remainder = value;
    let mut rendered = String::new();
    for (unit, symbol) in ROMAN_CANONICAL_TABLE {
        while remainder >= unit {
            rendered.push_str(symbol);
            remainder -= unit;
        }
    }
    rendered
}

/// Well-formed roman numeral parse over the full alphabet (`i v x l c d m`, either case).
/// `None` if malformed.
///
/// Well-formedness is checked by canonical round-trip rather than by "built only from
/// numeral letters", which is what let the old predicate accept `IIII`, `VX` and `IC`
/// and, worse, treat any short run of `i`/`v`/`x` as pagination. Case must be uniform:
/// real documents never mix, and `Iv` is far more likely to be a truncated word.
fn parse_roman_numeral(text: &str) -> Option<u32> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_ROMAN_CHARS {
        return None;
    }
    let all_lowercase = trimmed.chars().all(|character| character.is_ascii_lowercase());
    let all_uppercase = trimmed.chars().all(|character| character.is_ascii_uppercase());
    if !all_lowercase && !all_uppercase {
        return None;
    }
    let uppercased = trimmed.to_ascii_uppercase();
    let letters = uppercased
        .chars()
        .map(roman_letter_value)
        .collect::<Option<Vec<u32>>>()?;
    let total = accumulate_roman(&letters);
    let value = u32::try_from(total).ok()?;
    if value == 0 || value > MAX_ROMAN_VALUE {
        return None;
    }
    (to_canonical_roman(value) == uppercased).then_some(value)
}

/// Standard subtractive accumulation: a letter smaller than its successor is subtracted.
/// Uses a signed accumulator so malformed input such as `IM` cannot underflow.
fn accumulate_roman(letters: &[u32]) -> i64 {
    let mut total: i64 = 0;
    for (index, &current) in letters.iter().enumerate() {
        let next = letters.get(index + 1).copied().unwrap_or(0);
        if current < next {
            total -= i64::from(current);
        } else {
            total += i64::from(current);
        }
    }
    total
}

// ── Cross-page sequence evidence ─────────────────────────────────────────────────

/// Horizontal tolerance, as a fraction of page width, for treating two candidates as
/// occupying "the same" margin position.
///
/// Folios drift by a digit's width as the number grows (`9` to `10`) and by typesetting
/// jitter, so an exact match is useless. 0.08 of a 8.5 in page is 0.68 in — wide enough
/// to absorb that drift, narrow enough that the left (~0.1) and right (~0.9) folios of a
/// recto/verso book stay in separate cohorts and each builds its own +2 progression.
const X_RATIO_TOLERANCE: f32 = 0.08;

/// Minimum candidates at one position before any sequence evidence exists. A single
/// observation cannot show progression, so two is the floor.
const MIN_SEQUENCE_PAGES: usize = 2;

/// Cohort size at which sequence evidence is considered complete. Four consecutive
/// correctly-progressing folios is already an overwhelming signal; more adds nothing.
const SEQUENCE_SATURATION_PAGES: f32 = 4.0;

/// Four alternating recto or verso folios can span six pages. Distant uses of
/// a margin slot, such as chapter-opening folios, do not confirm a running slot.
const MAX_SEQUENCE_PAGE_DISTANCE: usize = 6;

/// Allowed mismatch between the page step and the value step of two consecutive
/// candidates.
///
/// A tolerance of 1 accepts a page whose folio was suppressed or failed extraction (the
/// value jumps by 2 across a 1-page step) without accepting arbitrary numbers that
/// happen to increase.
const SEQUENCE_STEP_TOLERANCE: i64 = 1;

/// Weight of the shape signal in the final confidence. Small by design: shape is what
/// over-matched in the first place.
const SHAPE_WEIGHT: f32 = 0.30;

/// Weight of the cross-page sequence signal. Dominant, because a monotonic run at a
/// fixed margin position is the only evidence that actually identifies pagination.
const SEQUENCE_WEIGHT: f32 = 0.70;

/// Hard ceiling on the confidence of any body-band candidate.
///
/// A numbered column in a table is a perfect monotonic sequence at a stable x position;
/// only the band separates it from a folio. Capping below
/// [`PageNumberSequence::DELETION_THRESHOLD`] makes that case structurally undeletable.
const BODY_BAND_CONFIDENCE_CAP: f32 = 0.30;

#[derive(Debug, Clone)]
struct Observation {
    page_index: usize,
    band: MarginBand,
    x_ratio: f32,
    value: Option<u32>,
    convention: PageNumberConvention,
    shape_confidence: f32,
}

/// Accumulates candidates across pages and confirms only those forming a near-monotonic
/// sequence at a stable margin position.
///
/// Observe every page before asking for a confidence: the whole point is that the answer
/// for one page depends on what the other pages carry at the same position.
#[derive(Debug, Default)]
pub(crate) struct PageNumberSequence {
    observations: Vec<Observation>,
}

impl PageNumberSequence {
    /// Confidence at or above which a caller may delete. Deliberately high — keeping a
    /// stray page number is far cheaper than deleting a table cell.
    ///
    /// Calibrated so that bare digits need a full four-page progression (0.79) while an
    /// isolated candidate of any shape tops out at 0.285.
    pub(crate) const DELETION_THRESHOLD: f32 = 0.75;

    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record a candidate seen on `page_index` at `x_ratio` (0.0 left, 1.0 right) in the
    /// given band.
    pub(crate) fn observe(
        &mut self,
        page_index: usize,
        band: MarginBand,
        x_ratio: f32,
        candidate: &PageNumberCandidate,
    ) {
        self.observations.push(Observation {
            page_index,
            band,
            x_ratio,
            value: candidate.value,
            convention: candidate.convention,
            shape_confidence: candidate.shape_confidence,
        });
    }

    /// Final confidence that the candidate recorded at this exact position is a real page
    /// number. Call only after every page has been observed. Returns 0.0 when nothing was
    /// recorded there.
    pub(crate) fn confidence_at(&self, page_index: usize, band: MarginBand, x_ratio: f32) -> f32 {
        let cohort = self.positional_cohort(page_index, band, x_ratio);
        if !cohort.iter().any(|observation| observation.page_index == page_index) {
            return 0.0;
        }
        let shape = mean_shape_confidence(&cohort);
        let sequence = sequence_score(&cohort);
        let confidence = (shape * SHAPE_WEIGHT + sequence * SEQUENCE_WEIGHT).clamp(0.0, 1.0);
        match band {
            MarginBand::Body => confidence.min(BODY_BAND_CONFIDENCE_CAP),
            MarginBand::Top | MarginBand::Bottom => confidence,
        }
    }

    /// Nearby pages that carry a candidate at this band and x position, one per page
    /// (the horizontally nearest), ordered by page index.
    fn positional_cohort(&self, page_index: usize, band: MarginBand, x_ratio: f32) -> Vec<&Observation> {
        let mut nearest_per_page: BTreeMap<usize, &Observation> = BTreeMap::new();
        for observation in &self.observations {
            if observation.page_index.abs_diff(page_index) > MAX_SEQUENCE_PAGE_DISTANCE
                || observation.band != band
                || (observation.x_ratio - x_ratio).abs() > X_RATIO_TOLERANCE
            {
                continue;
            }
            nearest_per_page
                .entry(observation.page_index)
                .and_modify(|best| {
                    if (observation.x_ratio - x_ratio).abs() < (best.x_ratio - x_ratio).abs() {
                        *best = observation;
                    }
                })
                .or_insert(observation);
        }
        nearest_per_page.into_values().collect()
    }
}

/// Average shape confidence over the whole cohort rather than the queried candidate
/// alone. Inside a confirmed run, one weak member (`i` at the head of `i, ii, iii, iv`)
/// is carried by its neighbours; outside one, averaging changes nothing.
fn mean_shape_confidence(cohort: &[&Observation]) -> f32 {
    if cohort.is_empty() {
        return 0.0;
    }
    let total: f32 = cohort.iter().map(|observation| observation.shape_confidence).sum();
    total / cohort.len() as f32
}

/// Fraction of consecutive cohort pairs that progress correctly, scaled by how much of
/// the saturation window the cohort fills. Zero when there is nothing to compare.
fn sequence_score(cohort: &[&Observation]) -> f32 {
    if cohort.len() < MIN_SEQUENCE_PAGES {
        return 0.0;
    }
    let pairs = cohort.len() - 1;
    let progressive = cohort
        .windows(2)
        .filter(|pair| is_progressive(pair[0], pair[1]))
        .count();
    let ratio = progressive as f32 / pairs as f32;
    let coverage = (cohort.len() as f32 / SEQUENCE_SATURATION_PAGES).min(1.0);
    ratio * coverage
}

/// Whether two cohort members advance like pagination: same convention, strictly
/// increasing, by roughly the number of pages between them. A repeated value (a label
/// such as `7` on every page) is not progressive, which is exactly how a running header
/// is rejected.
///
/// The convention must match because a document paginates one way at a time. A cohort
/// that reads `[1]` on one page and `Page 2 of 9` on the next is not one numbering, it is
/// two unrelated things that happen to share a margin position — most often a citation
/// reference sitting low on the page next to genuine folios. Requiring agreement makes
/// that pair contribute no evidence, which is the conservative direction R5 demands.
fn is_progressive(previous: &Observation, next: &Observation) -> bool {
    if previous.convention != next.convention {
        return false;
    }
    let (Some(earlier), Some(later)) = (previous.value, next.value) else {
        return false;
    };
    if later <= earlier {
        return false;
    }
    let page_step = i64::try_from(next.page_index.saturating_sub(previous.page_index)).unwrap_or(i64::MAX);
    let value_step = i64::from(later - earlier);
    (value_step - page_step).abs() <= SEQUENCE_STEP_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::{
        MarginBand, PageNumberConvention, PageNumberSequence, classify_page_number_text, margin_band,
        parse_roman_numeral,
    };

    /// Tolerance for comparing f32 confidences against hand-computed expectations.
    const CONFIDENCE_EPSILON: f32 = 1e-4;

    /// Representative bottom-margin x position used throughout the sequence tests.
    const CENTRE_X: f32 = 0.5;

    fn assert_close(actual: f32, expected: f32, label: impl std::fmt::Display) {
        assert!(
            (actual - expected).abs() < CONFIDENCE_EPSILON,
            "{label}: expected {expected}, got {actual}"
        );
    }

    fn observe_texts(sequence: &mut PageNumberSequence, band: MarginBand, x_ratio: f32, texts: &[&str]) {
        for (page_index, text) in texts.iter().enumerate() {
            let candidate = classify_page_number_text(text).unwrap_or_else(|| panic!("{text} should classify"));
            sequence.observe(page_index, band, x_ratio, &candidate);
        }
    }

    #[test]
    fn should_split_page_height_into_top_body_and_bottom_bands() {
        assert_eq!(margin_band(0.0), MarginBand::Top);
        assert_eq!(margin_band(0.12), MarginBand::Top);
        assert_eq!(margin_band(0.13), MarginBand::Body);
        assert_eq!(margin_band(0.5), MarginBand::Body);
        assert_eq!(margin_band(0.87), MarginBand::Body);
        assert_eq!(margin_band(0.88), MarginBand::Bottom);
        assert_eq!(margin_band(1.0), MarginBand::Bottom);
    }

    #[test]
    fn should_treat_non_finite_y_ratio_as_body() {
        assert_eq!(margin_band(f32::NAN), MarginBand::Body);
    }

    #[test]
    fn should_classify_every_supported_convention() {
        for (text, convention, value) in [
            ("12", PageNumberConvention::BareDigits, 12_u32),
            ("Page 7", PageNumberConvention::PageN, 7),
            ("page 7", PageNumberConvention::PageN, 7),
            ("Page 3 of 40", PageNumberConvention::PageNofM, 3),
            ("PAGE 3 OF 40", PageNumberConvention::PageNofM, 3),
            ("5 / 12", PageNumberConvention::NSlashM, 5),
            ("5/12", PageNumberConvention::NSlashM, 5),
            ("[9]", PageNumberConvention::BracketedN, 9),
            ("3-12", PageNumberConvention::SectionPrefixed, 12),
            ("xiv", PageNumberConvention::Roman, 14),
        ] {
            let candidate = classify_page_number_text(text).unwrap_or_else(|| panic!("{text} should classify"));
            assert_eq!(candidate.convention, convention, "convention for {text}");
            assert_eq!(candidate.value, Some(value), "value for {text}");
        }
    }

    #[test]
    fn should_accept_all_three_dash_characters_around_a_folio() {
        for text in ["- 5 -", "\u{2013} 5 \u{2013}", "\u{2014} 5 \u{2014}", "-5-"] {
            let candidate = classify_page_number_text(text).unwrap_or_else(|| panic!("{text} should classify"));
            assert_eq!(
                candidate.convention,
                PageNumberConvention::DashedN,
                "convention for {text}"
            );
            assert_eq!(candidate.value, Some(5), "value for {text}");
        }
    }

    #[test]
    fn should_accept_roman_ordinals_inside_the_keyword_and_dash_conventions() {
        let keyword = classify_page_number_text("Page iv").expect("Page iv should classify");
        assert_eq!(keyword.convention, PageNumberConvention::PageN);
        assert_eq!(keyword.value, Some(4));

        let dashed = classify_page_number_text("- xii -").expect("- xii - should classify");
        assert_eq!(dashed.convention, PageNumberConvention::DashedN);
        assert_eq!(dashed.value, Some(12));
    }

    #[test]
    fn should_reject_text_that_cannot_be_a_page_number() {
        for text in [
            "",
            "   ",
            "Page layout options",
            "Introduction",
            "12345",
            "12.5",
            "Table 3",
            "Q4",
            "the quick brown fox jumped over",
        ] {
            assert!(
                classify_page_number_text(text).is_none(),
                "{text} must not classify as a page number"
            );
        }
    }

    #[test]
    fn should_reject_page_n_of_m_when_total_is_below_current() {
        assert!(classify_page_number_text("Page 40 of 3").is_none());
        assert!(classify_page_number_text("40 / 3").is_none());
    }

    #[test]
    fn should_parse_well_formed_roman_numerals_in_either_case() {
        for (text, expected) in [
            ("I", 1_u32),
            ("IV", 4),
            ("IX", 9),
            ("XLII", 42),
            ("MCMXCIV", 1994),
            ("MMMCMXCIX", 3999),
            ("iv", 4),
            ("xii", 12),
            ("cd", 400),
            ("lxx", 70),
        ] {
            assert_eq!(parse_roman_numeral(text), Some(expected), "roman parse of {text}");
        }
    }

    #[test]
    fn should_reject_malformed_roman_numerals() {
        for text in ["IIII", "VX", "IC", "MMMM", "VV", "XXXX", "IL", "Iv", "", "abc", "MIM"] {
            assert_eq!(parse_roman_numeral(text), None, "{text} must be rejected");
        }
    }

    #[test]
    fn should_keep_the_bare_letter_i_far_below_the_deletion_threshold() {
        let candidate = classify_page_number_text("I").expect("I is shape-compatible");
        assert_eq!(candidate.convention, PageNumberConvention::Roman);
        assert_close(candidate.shape_confidence, 0.15, "shape confidence of I");

        let mut sequence = PageNumberSequence::new();
        sequence.observe(0, MarginBand::Bottom, CENTRE_X, &candidate);
        let confidence = sequence.confidence_at(0, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.045, "confidence of an isolated I");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_keep_a_repeated_capital_i_below_the_deletion_threshold() {
        let candidate = classify_page_number_text("I").expect("I is shape-compatible");
        let mut sequence = PageNumberSequence::new();
        for page_index in 0..10 {
            sequence.observe(page_index, MarginBand::Bottom, CENTRE_X, &candidate);
        }
        let confidence = sequence.confidence_at(4, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.045, "confidence of a repeated I");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_keep_a_body_band_number_below_the_deletion_threshold() {
        let candidate = classify_page_number_text("22").expect("22 is shape-compatible");
        let mut sequence = PageNumberSequence::new();
        sequence.observe(0, MarginBand::Body, CENTRE_X, &candidate);
        let confidence = sequence.confidence_at(0, MarginBand::Body, CENTRE_X);
        assert_close(confidence, 0.09, "confidence of a body-band 22");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_cap_a_perfect_body_band_sequence_below_the_deletion_threshold() {
        let texts: Vec<String> = (1..=10).map(|value| value.to_string()).collect();
        let borrowed: Vec<&str> = texts.iter().map(String::as_str).collect();
        let mut sequence = PageNumberSequence::new();
        observe_texts(&mut sequence, MarginBand::Body, CENTRE_X, &borrowed);
        let confidence = sequence.confidence_at(5, MarginBand::Body, CENTRE_X);
        assert_close(confidence, 0.30, "capped confidence of a numbered table column");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_confirm_a_bottom_band_arabic_sequence_across_ten_pages() {
        let texts: Vec<String> = (1..=10).map(|value| value.to_string()).collect();
        let borrowed: Vec<&str> = texts.iter().map(String::as_str).collect();
        let mut sequence = PageNumberSequence::new();
        observe_texts(&mut sequence, MarginBand::Bottom, CENTRE_X, &borrowed);
        for page_index in 0..10 {
            let confidence = sequence.confidence_at(page_index, MarginBand::Bottom, CENTRE_X);
            assert_close(confidence, 0.79, format!("confidence on page {page_index}"));
            assert!(confidence >= PageNumberSequence::DELETION_THRESHOLD);
        }
    }

    #[test]
    fn should_reject_a_run_whose_pagination_convention_keeps_changing() {
        // Values ascend 1..4 at one stable bottom-margin position, so the arithmetic
        // alone would confirm the run. Every page uses a different convention, which
        // means these are four unrelated tokens, not one numbering.
        let mut sequence = PageNumberSequence::new();
        observe_texts(
            &mut sequence,
            MarginBand::Bottom,
            CENTRE_X,
            &["[1]", "Page 2", "3 / 9", "- 4 -"],
        );
        let confidence = sequence.confidence_at(2, MarginBand::Bottom, CENTRE_X);
        let mean_shape = (0.35 + 0.85 + 0.70 + 0.85) / 4.0;
        assert_close(
            confidence,
            mean_shape * 0.30,
            "mixed-convention run earns no sequence credit",
        );
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_still_confirm_a_run_that_keeps_one_convention() {
        // Same positions and same ascending values as the mixed-convention run above;
        // only the convention agreement differs, which isolates the new rule. ~keep
        let mut sequence = PageNumberSequence::new();
        observe_texts(
            &mut sequence,
            MarginBand::Bottom,
            CENTRE_X,
            &["- 1 -", "- 2 -", "- 3 -", "- 4 -"],
        );
        let confidence = sequence.confidence_at(2, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.955, "consistent dashed folios");
        assert!(confidence >= PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_reject_a_constant_value_repeated_at_a_stable_position() {
        let repeated = ["7"; 10];
        let mut sequence = PageNumberSequence::new();
        observe_texts(&mut sequence, MarginBand::Bottom, CENTRE_X, &repeated);
        let confidence = sequence.confidence_at(3, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.09, "confidence of a repeated label");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_confirm_lowercase_roman_front_matter_across_four_pages() {
        let mut sequence = PageNumberSequence::new();
        observe_texts(&mut sequence, MarginBand::Bottom, CENTRE_X, &["i", "ii", "iii", "iv"]);
        for page_index in 0..4 {
            let confidence = sequence.confidence_at(page_index, MarginBand::Bottom, CENTRE_X);
            assert_close(
                confidence,
                0.8125,
                format!("front-matter confidence on page {page_index}"),
            );
            assert!(confidence >= PageNumberSequence::DELETION_THRESHOLD);
        }
    }

    #[test]
    fn should_tolerate_a_missing_folio_within_a_run() {
        let mut sequence = PageNumberSequence::new();
        for (page_index, text) in [(0_usize, "1"), (1, "2"), (3, "4"), (4, "5")] {
            let candidate = classify_page_number_text(text).expect("digits classify");
            sequence.observe(page_index, MarginBand::Bottom, CENTRE_X, &candidate);
        }
        let confidence = sequence.confidence_at(4, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.79, "confidence with one page missing");
        assert!(confidence >= PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_preserve_sparse_chapter_opening_folios_but_confirm_running_folios() {
        let mut sequence = PageNumberSequence::new();
        for page in 0..40 {
            let text = (page + 1).to_string();
            let candidate = classify_page_number_text(&text).expect("digits classify");
            let (band, x) = if page % 8 == 0 {
                (MarginBand::Bottom, CENTRE_X)
            } else if page % 2 == 0 {
                (MarginBand::Top, 0.1)
            } else {
                (MarginBand::Top, 0.9)
            };
            sequence.observe(page, band, x, &candidate);
        }

        for page in (0..40).step_by(8) {
            assert!(
                sequence.confidence_at(page, MarginBand::Bottom, CENTRE_X) < PageNumberSequence::DELETION_THRESHOLD
            );
        }
        assert!(sequence.confidence_at(17, MarginBand::Top, 0.9) >= PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_confirm_alternating_recto_verso_folios_as_two_positions() {
        const LEFT_X: f32 = 0.1;
        const RIGHT_X: f32 = 0.9;
        let mut sequence = PageNumberSequence::new();
        for page_index in 0..10_usize {
            let text = (page_index + 1).to_string();
            let candidate = classify_page_number_text(&text).expect("digits classify");
            let x_ratio = if page_index % 2 == 0 { LEFT_X } else { RIGHT_X };
            sequence.observe(page_index, MarginBand::Bottom, x_ratio, &candidate);
        }
        for (page_index, x_ratio) in [(0_usize, LEFT_X), (1, RIGHT_X)] {
            let confidence = sequence.confidence_at(page_index, MarginBand::Bottom, x_ratio);
            assert_close(
                confidence,
                0.79,
                format!("alternating folio confidence on page {page_index}"),
            );
            assert!(confidence >= PageNumberSequence::DELETION_THRESHOLD);
        }
    }

    #[test]
    fn should_reject_a_sequence_that_drifts_horizontally_out_of_tolerance() {
        // A per-page drift of 0.09 exceeds X_RATIO_TOLERANCE, so no two candidates ever
        // share a position and the run yields no sequence evidence at all.
        const DRIFT_PER_PAGE: f32 = 0.09;
        let mut sequence = PageNumberSequence::new();
        for page_index in 0..10_usize {
            let text = (page_index + 1).to_string();
            let candidate = classify_page_number_text(&text).expect("digits classify");
            let x_ratio = 0.1 + page_index as f32 * DRIFT_PER_PAGE;
            sequence.observe(page_index, MarginBand::Bottom, x_ratio, &candidate);
        }
        let confidence = sequence.confidence_at(0, MarginBand::Bottom, 0.1);
        assert_close(confidence, 0.09, "confidence for a positionally unstable run");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_return_zero_when_nothing_was_recorded_at_the_position() {
        let candidate = classify_page_number_text("4").expect("digits classify");
        let mut sequence = PageNumberSequence::new();
        sequence.observe(0, MarginBand::Bottom, CENTRE_X, &candidate);
        assert_close(
            sequence.confidence_at(1, MarginBand::Bottom, CENTRE_X),
            0.0,
            "unobserved page",
        );
        assert_close(
            sequence.confidence_at(0, MarginBand::Top, CENTRE_X),
            0.0,
            "unobserved band",
        );
        assert_close(
            sequence.confidence_at(0, MarginBand::Bottom, 0.9),
            0.0,
            "unobserved x position",
        );
    }

    #[test]
    fn should_keep_a_single_explicit_page_of_total_below_the_deletion_threshold() {
        let candidate = classify_page_number_text("Page 3 of 40").expect("Page 3 of 40 classifies");
        let mut sequence = PageNumberSequence::new();
        sequence.observe(0, MarginBand::Bottom, CENTRE_X, &candidate);
        let confidence = sequence.confidence_at(0, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.285, "strongest possible isolated shape");
        assert!(confidence < PageNumberSequence::DELETION_THRESHOLD);
    }

    #[test]
    fn should_confirm_a_three_page_page_of_total_run() {
        let mut sequence = PageNumberSequence::new();
        let texts = ["Page 1 of 3", "Page 2 of 3", "Page 3 of 3"];
        observe_texts(&mut sequence, MarginBand::Bottom, CENTRE_X, &texts);
        let confidence = sequence.confidence_at(1, MarginBand::Bottom, CENTRE_X);
        assert_close(confidence, 0.81, "three-page explicit run");
        assert!(confidence >= PageNumberSequence::DELETION_THRESHOLD);
    }
}
