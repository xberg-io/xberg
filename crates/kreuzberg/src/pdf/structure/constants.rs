//! Threshold constants for PDF-to-Markdown spatial analysis.

// ── Glyph-fragmentation repair (issue #962) ────────────────────────────────
//
// Word-exported PDFs position each glyph via its own BT…ET block with a
// sinusoidal y-jitter (~3–5 pt amplitude, 6-glyph period). pdf_oxide's
// ColumnAware reading order groups spans by y-level, scrambling reading
// order for these documents. The constants below parameterise the
// detection and reconstruction heuristic in `pdf::oxide::text`.
//
// TODO: remove this heuristic when kreuzberg upgrades to pdf_oxide ≥ 0.3.51.
// Upstream fix shipped in v0.3.51 (2026-05-19, closing issue #518): the Tm
// continuation check now uses glyph height as the tolerance floor (≥ 0.5 pt)
// so per-glyph sinusoidal jitter merges natively into a single span and the
// reading-order scramble no longer occurs. kreuzberg currently pins v0.3.50.
// After bumping, verify with test_3_5pt_jitter_coalesced and the other tests
// in tests/pdf_glyph_spacing_issue_962.rs before deleting this block.

/// Maximum y-gap (pt) between two spans that can still be considered "same
/// line" under the glyph-fragmentation detection heuristic.
///
/// Word's sinusoidal jitter (6-glyph period, ~3 pt amplitude) produces
/// consecutive-pair y-gaps of ≤ ~3.03 pt. 5 pt adds headroom for atypical
/// Word configurations while remaining well below normal body-text leading
/// (~12–14 pt). Using an absolute ceiling instead of a font-size fraction
/// avoids the false-positive zone where `font_size * 0.25` (the old
/// fallback) overlaps with normal tight leading for larger fonts.
pub(crate) const MAX_GLYPH_JITTER_PT: f32 = 5.0;

/// Minimum qualifying x-disorder events before classifying a span list as
/// glyph-fragmented.
///
/// A 32-char Word jitter word (period 6, 3 distinct y-levels) produces
/// exactly 4 disorder events. Requiring ≥ 3 is sufficient to detect all
/// jitter amplitudes ≥ 3 pt while being robust against false positives:
/// the short-span guard (≤ 3 chars) and the 5 pt same-line ceiling
/// together make it essentially impossible for normal multi-column text
/// to accumulate 3 consecutive qualifying resets.
pub(crate) const MIN_DISORDER_COUNT: usize = 3;

/// y-proximity threshold (pt) for grouping spans into visual lines during
/// reconstruction. Must be ≥ MAX_GLYPH_JITTER_PT so every span pair
/// accepted by the detection gate is merged into the same group.
pub(crate) const COALESCE_THRESHOLD: f32 = 5.0;

// ── Structural heading analysis ────────────────────────────────────────────

/// Maximum word count for a paragraph to qualify as a heading.
pub(super) const MAX_HEADING_WORD_COUNT: usize = 20;
/// Maximum distance multiplier relative to average inter-cluster gap for heading assignment.
pub(super) const MAX_HEADING_DISTANCE_MULTIPLIER: f32 = 2.0;
/// Minimum ratio of heading font size to body font size (heading must be this much larger).
/// 1.15 captures LaTeX \subsection (12pt vs 10pt body = 1.2 ratio).
pub(super) const MIN_HEADING_FONT_RATIO: f32 = 1.15;
/// Minimum absolute font-size difference (in points) between heading and body.
/// 1.5pt captures academic sub-headings (11.5pt vs 10pt body).
pub(super) const MIN_HEADING_FONT_GAP: f32 = 1.5;
/// Maximum word count for a bold paragraph to be promoted to a section heading.
pub(super) const MAX_BOLD_HEADING_WORD_COUNT: usize = 12;
/// Fraction of the maximum right edge that a line must reach to be considered "full"
/// (used for dehyphenation to avoid false joins on short/indented lines).
pub(super) const FULL_LINE_FRACTION: f32 = 0.85;
