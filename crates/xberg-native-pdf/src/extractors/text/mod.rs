//! Text extraction from PDF content streams.
//!
//! This module executes content stream operators to extract positioned
//! text characters with their Unicode mappings, font information,
//! bounding boxes.

#![forbid(unsafe_code)]

use crate::color::cmyk_to_rgb;
use crate::config::ExtractionProfile;
use crate::content::graphics_state::{GraphicsStateStack, Matrix};
use crate::content::operators::{Operator, TextElement};
use crate::content::parse_and_execute_text_only;
use crate::content::parse_content_stream;
use crate::error::Result;
use crate::extract_log_debug;
use crate::fonts::FontInfo;
#[cfg(test)]
use crate::fonts::unicode_decode::fallback_char_to_unicode;
use crate::fonts::unicode_decode::{
    DecodePolicy, TextCharIter, decode_text_to_unicode, fallback_extraction_char_to_unicode, strip_subset_prefix,
};
use crate::geometry::Rect;
use crate::layout::{Color, FontWeight, TextChar, TextSpan};
use crate::object::{Object, ObjectRef};
use crate::pipeline::config::WordBoundaryMode;
use crate::text::{BoundaryContext, CharacterInfo, DocumentScript, WordBoundaryDetector};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag controlling whether glyph-decode sites emit `U+FFFD`
/// (REPLACEMENT CHARACTER) into `extract_text` / `extract_words` /
/// `extract_spans` output.
///
/// The historical default is to silently drop `U+FFFD` chars, which
/// is preserved here for back-compat. Setting `true` makes the
/// high-level accessors consistent with `extract_chars` (which
/// always preserves FFFD) so callers can detect unmapped-glyph
/// pages without diffing the two accessors' outputs.
///
/// `Ordering::Relaxed` is sufficient because every read is gated on
/// `Acquire`-style writes from the setter, and the flag is a single
/// boolean with no other state dependencies.
static PRESERVE_UNMAPPED_GLYPHS: AtomicBool = AtomicBool::new(false);

/// Set the global U+FFFD preservation flag. When `true`, the high-level
/// text accessors (`extract_text` / `extract_words` / `extract_spans`)
/// emit U+FFFD chars for glyphs that map to the REPLACEMENT
/// CHARACTER, matching the behaviour of `extract_chars` which has
/// always preserved them. Returns the previous flag value.
///
/// Resolves the filter divergence where the high-level accessors
/// silently drop FFFD while `extract_chars` keeps them, producing
/// empty `extract_text` output on pages whose visible glyphs all
/// map to FFFD (e.g. the MSAM10 math-symbol font).
///
/// The default is `false` to preserve historical fixture output
/// byte-identical for the no-FFFD-glyph case; downstream callers
/// that want to surface unmapped glyphs to the user opt in by
/// setting `true`.
pub fn set_preserve_unmapped_glyphs(preserve: bool) -> bool {
    PRESERVE_UNMAPPED_GLYPHS.swap(preserve, Ordering::SeqCst)
}

/// True if the high-level accessors should preserve `U+FFFD` glyphs.
#[inline]
pub(crate) fn preserve_unmapped_glyphs() -> bool {
    PRESERVE_UNMAPPED_GLYPHS.load(Ordering::Relaxed)
}

/// Source of a space decision in the unified pipeline.
///
/// This enum tracks why a space was inserted (or not), which helps with
/// debugging and understanding the text extraction behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceSource {
    /// Space triggered by TJ offset value (negative offset > threshold)
    /// Confidence: 0.95 (explicit PDF positioning signal)
    TjOffset,

    /// Space triggered by geometric gap between spans
    /// Confidence: 0.8 (heuristic based on font metrics)
    GeometricGap,

    /// Space triggered by character transition heuristic (e.g., CamelCase, number->letter)
    /// Confidence: 0.6 (pattern-based heuristic)
    CharacterHeuristic,

    /// Space already present in boundary (no insertion needed)
    /// Confidence: 1.0 (deterministic)
    AlreadyPresent,

    /// No space inserted
    /// Confidence: varies (default when no rule matches)
    NoSpace,

    /// No space: suppressed specifically by the intra-word kerning guard
    /// (a lowercase↔lowercase gap below 0.75× the space-glyph advance). Kept
    /// distinct from `NoSpace` so the per-line bimodal rescue can override
    /// ONLY this purely-geometric suppression, never the semantic ones
    /// (complex-script, CJK, ligature) that also return no-space.
    IntraWordKerning,

    /// Space triggered by WordBoundaryDetector analysis
    /// Confidence: 0.85 (combines TJ offset, geometric, and CJK signals per PDF Spec 9.4.4)
    WordBoundaryAnalysis,
}

/// Result of unified space decision process.
///
/// This struct is the single source of truth for whether a space should be inserted
/// between two text spans. It combines all available signals:
/// - TJ offset values from PDF content stream
/// - Geometric gaps between spans
/// - Character transition heuristics
/// - Existing boundary whitespace
///
/// Per PDF Spec ISO 32000-1:2008, Section 9.4.4 NOTE 6:
/// "The identification of what constitutes a word is unrelated to how the text
/// happens to be grouped into show strings... text strings should be as long as possible."
#[derive(Debug, Clone)]
pub struct SpaceDecision {
    /// Whether a space should be inserted
    pub insert_space: bool,

    /// Source/reason for this decision
    pub source: SpaceSource,

    /// Confidence score (0.0-1.0) indicating certainty
    pub confidence: f32,
}

impl SpaceDecision {
    /// Create a decision to insert a space from a specific source.
    pub fn insert(source: SpaceSource, confidence: f32) -> Self {
        Self {
            insert_space: true,
            source,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create a decision to not insert a space.
    pub fn no_space(source: SpaceSource, confidence: f32) -> Self {
        Self {
            insert_space: false,
            source,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

/// Configuration for text extraction heuristics.
///
/// PDF spec does not define explicit rules for many spacing scenarios.
/// These configurable thresholds allow tuning extraction behavior.
///
/// # PDF Spec Reference
///
/// ISO 32000-1:2008, Section 9.4.4 - Text Positioning operators (TJ, Tj)
/// The spec defines how positioning works but NOT when a position offset
/// represents a word boundary vs. tight kerning.
#[derive(Debug, Clone)]
pub struct TextExtractionConfig {
    /// Extraction profile with document-type-specific thresholds
    ///
    /// When set, this profile overrides individual threshold settings and provides
    /// pre-tuned parameters optimized for specific document types (Academic, Policy,
    /// Government, Form, ScannedOCR, etc.).
    ///
    /// **Default**: None (uses legacy individual thresholds for backward compatibility)
    pub profile: Option<ExtractionProfile>,

    /// Threshold for inserting space characters in TJ arrays.
    ///
    /// Prefer `profile` with an `ExtractionProfile`, or `word_margin_ratio` with
    /// `use_adaptive_tj_threshold` enabled, for geometry-based adaptive thresholds.
    /// This field remains the live fallback used when font metrics are unavailable,
    /// when adaptive thresholds are disabled, or when `profile` is not set.
    ///
    /// **HEURISTIC**: When a TJ array contains a negative offset (in text space units),
    /// and that offset exceeds this threshold, a space character is inserted.
    ///
    /// **Default**: -120.0 units ≈ 0.12em
    /// - Typical word space: 0.25-0.33em (250-330 units)
    /// - Typical letter kerning: <0.1em (<100 units)
    ///
    /// **Lower values** (e.g., -80): More sensitive, inserts more spaces (may add spurious spaces)
    /// **Higher values** (e.g., -200): Less sensitive, inserts fewer spaces (may miss word boundaries)
    ///
    /// Set to `f32::NEG_INFINITY` to disable space insertion entirely.
    pub space_insertion_threshold: f32,

    /// Word margin ratio for geometry-based adaptive TJ threshold.
    ///
    /// When `use_adaptive_tj_threshold` is true and font metrics are available,
    /// the TJ offset threshold is calculated as:
    /// ```text
    /// adaptive_threshold = -(average_glyph_width * word_margin_ratio)
    /// ```
    ///
    /// This approach adapts to different font sizes and families by using the
    /// actual glyph metrics instead of a static value. This matches pdfplumber's
    /// `word_margin` parameter (default 0.1).
    ///
    /// **Default**: 0.1 (10% of average glyph width)
    ///
    /// **Typical values**:
    /// - 0.05: Tighter spacing (fewer spaces inserted, better for narrow fonts)
    /// - 0.1: Standard word spacing (default, matches pdfplumber)
    /// - 0.15: Looser spacing (more spaces inserted, better for wide fonts)
    ///
    /// **Note**: If font metrics are unavailable, falls back to `space_insertion_threshold`.
    ///
    /// # PDF Spec Reference
    ///
    /// ISO 32000-1:2008, Section 9.4.4 - TJ offsets are in thousandths of em.
    /// Average glyph width is also in thousandths of em, making this ratio
    /// dimensionally correct.
    pub word_margin_ratio: f32,

    /// Enable adaptive TJ threshold based on font geometry.
    ///
    /// When true, uses font metrics to calculate the TJ offset threshold dynamically:
    /// `adaptive_threshold = -(average_glyph_width * word_margin_ratio)`
    ///
    /// This replaces the static `space_insertion_threshold` with a value that adapts
    /// to different font sizes, families, and document layouts.
    ///
    /// **Default**: true (adaptive approach enabled)
    ///
    /// Set to `false` for backward compatibility with legacy behavior, which
    /// uses only the static `space_insertion_threshold`.
    ///
    /// # Benefits
    ///
    /// - Handles font size variations (8pt vs 24pt documents)
    /// - Adapts to different character widths (serif vs sans-serif, monospace vs proportional)
    /// - Reduces spurious spaces in policy documents with tight kerning
    /// - Maintains word boundary detection in academic documents
    pub use_adaptive_tj_threshold: bool,

    /// Word boundary detection mode for TJ array processing
    ///
    /// Controls whether WordBoundaryDetector is used as:
    /// - Tiebreaker: Only when TJ and geometric signals conflict (default)
    /// - Primary: Before creating TextSpans from tj_character_array
    ///
    /// **Default**: WordBoundaryMode::Tiebreaker (backward compatible)
    pub word_boundary_mode: WordBoundaryMode,
}

impl Default for TextExtractionConfig {
    fn default() -> Self {
        Self {
            profile: None,
            // Default -120.0 (conservative; matches existing
            // ExtractionProfile::CONSERVATIVE for byte-identical
            // back-compat). Callers handling TJ-heavy PDFs that
            // produce `Loremipsumdolorsitamet`-style merged
            // paragraphs can override via
            // `TextExtractionConfig::with_space_threshold(-100.0)` or
            // via the `TJ_HEAVY` extraction profile (see
            // config/extraction_profiles.rs). The default stays at
            // -120 to preserve byte-identical fixture output for the
            // 75-PDF regression sweep.
            //
            // Per-document calibration via gap_statistics is the
            // ideal root-cause fix; it requires a calibration corpus
            // to validate the threshold against without regressing
            // other inputs. ~keep
            space_insertion_threshold: -120.0,
            word_margin_ratio: 0.1,
            use_adaptive_tj_threshold: false,
            word_boundary_mode: WordBoundaryMode::default(),
        }
    }
}

impl TextExtractionConfig {
    /// Create a new configuration with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractionConfig;
    ///
    /// let config = TextExtractionConfig::new();
    /// assert_eq!(config.space_insertion_threshold, -120.0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with custom space insertion threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Negative offset threshold for space insertion (in text space units)
    ///
    /// **Note**: This uses the static threshold. For better results, consider using
    /// `with_word_margin_ratio()` with adaptive thresholds enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractionConfig;
    ///
    /// // More aggressive space insertion
    /// let config = TextExtractionConfig::with_space_threshold(-80.0);
    ///
    /// // Disable space insertion entirely
    /// let no_spaces = TextExtractionConfig::with_space_threshold(f32::NEG_INFINITY);
    /// ```
    pub fn with_space_threshold(threshold: f32) -> Self {
        Self {
            profile: None,
            space_insertion_threshold: threshold,
            word_margin_ratio: 0.1,
            use_adaptive_tj_threshold: false,
            word_boundary_mode: WordBoundaryMode::default(),
        }
    }

    /// Create a configuration with custom word margin ratio for adaptive TJ thresholds.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Word margin ratio as fraction of average glyph width (typically 0.05-0.15)
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractionConfig;
    ///
    /// // Standard adaptive thresholds (matches pdfplumber)
    /// let config = TextExtractionConfig::with_word_margin_ratio(0.1);
    ///
    /// // More aggressive (wider thresholds, more spaces)
    /// let aggressive = TextExtractionConfig::with_word_margin_ratio(0.15);
    ///
    /// // More conservative (narrower thresholds, fewer spaces)
    /// let conservative = TextExtractionConfig::with_word_margin_ratio(0.05);
    /// ```
    pub fn with_word_margin_ratio(ratio: f32) -> Self {
        Self {
            profile: None,
            space_insertion_threshold: -120.0,
            word_margin_ratio: ratio,
            use_adaptive_tj_threshold: true,
            word_boundary_mode: WordBoundaryMode::default(),
        }
    }

    /// Set the word margin ratio on an existing configuration (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `ratio` - Word margin ratio as fraction of average glyph width
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractionConfig;
    ///
    /// let config = TextExtractionConfig::new()
    ///     .set_word_margin_ratio(0.15);
    /// ```
    pub fn set_word_margin_ratio(mut self, ratio: f32) -> Self {
        self.word_margin_ratio = ratio;
        self.use_adaptive_tj_threshold = true;
        self
    }

    /// Enable or disable adaptive TJ thresholds (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to use adaptive thresholds based on font metrics
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractionConfig;
    ///
    /// // Use static threshold only
    /// let config = TextExtractionConfig::new()
    ///     .set_adaptive_tj_threshold(false);
    /// ```
    pub fn set_adaptive_tj_threshold(mut self, enabled: bool) -> Self {
        self.use_adaptive_tj_threshold = enabled;
        self
    }

    /// Set the extraction profile and apply its threshold configuration (builder pattern).
    ///
    /// This applies the profile's thresholds to the configuration, selecting document-type-specific
    /// parameters for better text extraction quality.
    ///
    /// # Arguments
    ///
    /// * `profile` - An extraction profile (e.g., ACADEMIC, POLICY, FORM)
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractionConfig;
    /// use xberg_native_pdf::config::ExtractionProfile;
    ///
    /// // Use ACADEMIC profile for research papers
    /// let config = TextExtractionConfig::new()
    ///     .with_profile(ExtractionProfile::ACADEMIC);
    /// ```
    pub fn with_profile(mut self, profile: ExtractionProfile) -> Self {
        let tj_offset = profile.tj_offset_threshold;
        let word_margin = profile.word_margin_ratio;
        let use_adaptive = profile.use_adaptive_threshold;

        self.profile = Some(profile);
        self.space_insertion_threshold = tj_offset;
        self.word_margin_ratio = word_margin;
        self.use_adaptive_tj_threshold = use_adaptive;
        self
    }
}

/// Configuration for span merging behavior.
///
/// These thresholds control how adjacent text spans are merged together and when
/// spaces are inserted between them. All thresholds are in PDF points (1/72 inch).
///
/// # Rationale
///
/// PDF content streams don't explicitly mark word boundaries - text can be rendered
/// with arbitrary gaps. These configurable thresholds allow tuning extraction to
/// different document types:
/// - Academic papers: tight column spacing, small gaps between words
/// - Documents with tables: larger gaps to preserve structure
/// - Dense grids (author lists): very small gaps that are still word boundaries
///
/// # References
///
/// Typography standards: word spacing typically 0.25-0.33em (25-33% of font size)
#[derive(Clone, Debug, PartialEq)]
pub struct SpanMergingConfig {
    /// Minimum gap (in multiples of font size) to trigger space insertion.
    ///
    /// When the gap between two spans exceeds this threshold, a space is inserted.
    /// Expressed as a ratio of font size (em).
    ///
    /// **Default**: 0.25
    /// - Based on typography standards: typical word spacing is 0.25-0.33em
    /// - For 12pt font: 0.25em * 12pt = 3pt
    /// - For 10pt font: 0.25em * 10pt = 2.5pt
    ///
    /// **Tuning guidance**:
    /// - Lower values (0.15-0.20): More aggressive space insertion, catches dense layouts
    /// - Higher values (0.33-0.50): Conservative, only clear word boundaries
    pub space_threshold_em_ratio: f32,

    /// Conservative threshold for font transitions (in points).
    ///
    /// Below this gap, don't insert a space even if gap > 0, to avoid spurious spaces
    /// from font metric changes or very tight kerning.
    ///
    /// **Default**: 0.1
    /// - Avoids spaces from font metric alignment issues (very tight threshold)
    /// - Smaller than typical letter spacing in justified text
    /// - Catches actual overlaps/reversals while preserving character adjacency
    ///
    /// **Note**: Changed from 0.3 to 0.1 after regression testing revealed
    /// that 0.3pt was too conservative for policy documents (0.1-0.3pt word spacing),
    /// causing word fusion. Adaptive threshold analysis recommended for future improvement.
    ///
    /// **Tuning guidance**:
    /// - Lower values (0.1-0.2): More aggressive, inserts more spaces
    /// - Higher values (0.5-1.0): Conservative, only clear separations
    pub conservative_threshold_pt: f32,

    /// Column boundary threshold (in points).
    ///
    /// Gaps larger than this indicate column separation and prevent span merging.
    /// Used to preserve document structure (e.g., multi-column layouts, tables).
    ///
    /// **Default**: 5.0
    /// - Typical character width for 10-12pt font: 4-6pt
    /// - Word spacing: 2-4pt
    /// - Column gaps in academic papers: 5-15pt
    /// - Table column gaps: 10-50pt
    ///
    /// **Tuning guidance**:
    /// - Lower values (3.0-4.0): Merge more spans, risk merging across columns
    /// - Higher values (8.0-10.0): Keep columns separate, preserve structure
    pub column_boundary_threshold_pt: f32,

    /// Negative gap threshold for severe overlaps (in points).
    ///
    /// When gaps are negative (spans overlap), values more severe than this
    /// indicate genuine overlap and should prevent merging.
    ///
    /// **Default**: -0.5
    /// - Typical font metric variations: 0 to -0.3pt
    /// - Small overlaps from kerning: -0.3 to -0.5pt
    /// - Real overlap errors: worse than -0.5pt
    ///
    /// **Tuning guidance**:
    /// - Less negative (-0.2, -0.1): More conservative on overlaps
    /// - More negative (-1.0, -2.0): Allow some overlap to merge adjacent text
    pub severe_overlap_threshold_pt: f32,

    /// Enable adaptive threshold analysis (default: true).
    ///
    /// When true, the `conservative_threshold_pt` is automatically calculated
    /// based on the gap distribution within the document. This overrides the fixed
    /// threshold value and adapts to different document types.
    ///
    /// **Default**: true (adaptive enabled)
    /// Enabled by default to improve extraction quality across document types.
    /// Use `SpanMergingConfig::legacy()` for the old fixed-threshold behavior.
    ///
    /// # Performance
    ///
    /// Adaptive analysis adds minimal overhead (O(n log n) for gap analysis where n = spans).
    /// Expected overhead: <5% of total extraction time.
    pub use_adaptive_threshold: bool,

    /// Configuration for adaptive threshold analysis.
    ///
    /// Only used when `use_adaptive_threshold` is true.
    /// If None, uses `AdaptiveThresholdConfig::default()`.
    ///
    /// Allows fine-tuning the adaptive analysis for specific document types:
    /// - `AdaptiveThresholdConfig::policy_documents()` - For tight spacing
    /// - `AdaptiveThresholdConfig::academic()` - For standard spacing
    /// - `AdaptiveThresholdConfig::aggressive()` - For dense layouts
    /// - `AdaptiveThresholdConfig::conservative()` - For formal documents
    pub adaptive_config: Option<crate::extractors::gap_statistics::AdaptiveThresholdConfig>,

    /// Enable email pattern detection for spacing decisions.
    ///
    /// When true, detects email-like patterns in surrounding text
    /// (e.g., "user@domain" separated by spaces) and applies special spacing rules
    /// to preserve email addresses.
    ///
    /// Per PDF Spec ISO 32000-1:2008 Section 9.10, only extracted text patterns
    /// are used - no domain-specific semantics.
    ///
    /// **Default**: false
    pub detect_email_patterns: bool,

    /// Multiplier for email pattern threshold detection.
    ///
    /// Controls how aggressively email patterns are detected by adjusting the gap threshold.
    /// A multiplier > 1.0 makes detection more lenient (allows larger gaps to be considered email context).
    /// A multiplier < 1.0 makes detection stricter.
    ///
    /// Calculated as: `email_threshold = geometric_threshold * email_threshold_multiplier`
    ///
    /// **Default**: 2.5
    /// - At 2.5×, handles typical email address separations with spaces
    /// - Typical gap between email parts: 4-8pt (after @, before TLD)
    pub email_threshold_multiplier: f32,

    /// Enable citation marker detection for spacing decisions.
    ///
    /// When true, detects superscript citation markers (typically smaller font size)
    /// and adjusts spacing rules to preserve citation formatting.
    ///
    /// Per PDF Spec ISO 32000-1:2008 Section 9.10, font size ratios from extracted content
    /// are used for detection.
    ///
    /// **Default**: false
    pub detect_citation_markers: bool,

    /// Font size ratio for citation marker detection.
    ///
    /// Citation markers typically have font size between this ratio and 1.0 of the base text.
    /// Values below this ratio are considered citation markers.
    ///
    /// **Default**: 0.75
    /// - Typical citation markers: 70-80% of text font size
    /// - Superscript usually: 50-80% of base font
    pub citation_font_size_ratio: f32,

    /// When `false`, each `Tm` operator starts a fresh span regardless of position.
    /// Use this to preserve column boundaries for callers that need per-positioned-run spans
    /// (e.g. pdftotext `-bbox-layout` parity).
    ///
    /// # Warning
    /// Disabling this on character-by-character-positioned PDFs (common in academic typesetting)
    /// can produce very large span counts per page (100× or more).
    ///
    /// Default: `true` (existing behaviour preserved).
    ///
    /// Reference: ISO 32000-1 §9.4.2 / §9.4.4 NOTE 6.
    pub merge_tm_tj_runs: bool,
}

impl Default for SpanMergingConfig {
    fn default() -> Self {
        Self {
            space_threshold_em_ratio: 0.25,
            conservative_threshold_pt: 0.1, // Reverted from 0.3 after regression testing ~keep
            column_boundary_threshold_pt: 5.0,
            severe_overlap_threshold_pt: -0.5,
            use_adaptive_threshold: true,
            adaptive_config: None,
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }
}

impl SpanMergingConfig {
    /// Create a new configuration with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::SpanMergingConfig;
    ///
    /// let config = SpanMergingConfig::new();
    /// assert_eq!(config.space_threshold_em_ratio, 0.25);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with aggressive space insertion (for dense layouts).
    ///
    /// Uses lower thresholds to insert spaces more readily:
    /// - space_threshold_em_ratio: 0.15 (instead of 0.25)
    /// - conservative_threshold_pt: 0.1 (instead of 0.3)
    ///
    /// Good for documents with many short words close together (author lists, grids).
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::SpanMergingConfig;
    ///
    /// let config = SpanMergingConfig::aggressive();
    /// ```
    pub fn aggressive() -> Self {
        Self {
            space_threshold_em_ratio: 0.15,
            conservative_threshold_pt: 0.1,
            column_boundary_threshold_pt: 5.0,
            severe_overlap_threshold_pt: -0.5,
            use_adaptive_threshold: false,
            adaptive_config: None,
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }

    /// Create a configuration with conservative space insertion (for formal documents).
    ///
    /// Uses higher thresholds to insert spaces less readily:
    /// - space_threshold_em_ratio: 0.33 (instead of 0.25)
    /// - conservative_threshold_pt: 0.3 (instead of 0.1)
    ///
    /// Good for formal documents where spacing is reliable.
    ///
    /// **Note**: After regression testing, 0.5pt threshold was found to cause
    /// excessive word fusion in policy documents. Reduced to 0.3pt.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::SpanMergingConfig;
    ///
    /// let config = SpanMergingConfig::conservative();
    /// ```
    pub fn conservative() -> Self {
        Self {
            space_threshold_em_ratio: 0.33,
            conservative_threshold_pt: 0.3,
            // ~keep
            column_boundary_threshold_pt: 5.0,
            severe_overlap_threshold_pt: -0.5,
            use_adaptive_threshold: false,
            adaptive_config: None,
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }

    /// Create a configuration with custom thresholds.
    ///
    /// # Arguments
    ///
    /// * `space_threshold_em` - Space threshold as em ratio
    /// * `conservative_pt` - Conservative gap threshold in points
    /// * `column_boundary_pt` - Column boundary threshold in points
    /// * `overlap_pt` - Severe overlap threshold in points
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::SpanMergingConfig;
    ///
    /// let config = SpanMergingConfig::custom(0.2, 0.2, 6.0, -0.3);
    /// ```
    pub fn custom(space_threshold_em: f32, conservative_pt: f32, column_boundary_pt: f32, overlap_pt: f32) -> Self {
        Self {
            space_threshold_em_ratio: space_threshold_em,
            conservative_threshold_pt: conservative_pt,
            column_boundary_threshold_pt: column_boundary_pt,
            severe_overlap_threshold_pt: overlap_pt,
            use_adaptive_threshold: false,
            adaptive_config: None,
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }

    /// Create a configuration with adaptive threshold enabled (default settings).
    ///
    /// This enables automatic threshold calculation based on the document's gap
    /// distribution. Uses conservative base settings for reliable defaults:
    /// - space_threshold_em_ratio: 0.25
    /// - conservative_threshold_pt: 0.1 (overridden by adaptive calculation)
    /// - column_boundary_threshold_pt: 5.0
    /// - severe_overlap_threshold_pt: -0.5
    /// - adaptive_config: AdaptiveThresholdConfig::default()
    ///
    /// The adaptive threshold is computed as: median_gap * 1.5, clamped to [0.05, 1.0] points.
    ///
    /// # Benefits
    ///
    /// - Automatically adapts to different document types
    /// - Reduces word fusion in policy documents with tight spacing
    /// - Minimizes spurious spaces in other document types
    /// - Maintains backward compatibility (disabled by default)
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::SpanMergingConfig;
    ///
    /// let config = SpanMergingConfig::adaptive();
    /// assert!(config.use_adaptive_threshold);
    /// ```
    pub fn adaptive() -> Self {
        Self {
            space_threshold_em_ratio: 0.25,
            conservative_threshold_pt: 0.1,
            column_boundary_threshold_pt: 5.0,
            severe_overlap_threshold_pt: -0.5,
            use_adaptive_threshold: true,
            adaptive_config: Some(crate::extractors::gap_statistics::AdaptiveThresholdConfig::default()),
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }

    /// Create a configuration with adaptive threshold and custom settings.
    ///
    /// # Arguments
    ///
    /// * `adaptive_config` - Custom adaptive threshold configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::{SpanMergingConfig, AdaptiveThresholdConfig};
    ///
    /// let config = SpanMergingConfig::adaptive_with_config(
    ///     AdaptiveThresholdConfig::policy_documents()
    /// );
    /// assert!(config.use_adaptive_threshold);
    /// ```
    pub fn adaptive_with_config(adaptive_config: crate::extractors::gap_statistics::AdaptiveThresholdConfig) -> Self {
        Self {
            space_threshold_em_ratio: 0.25,
            conservative_threshold_pt: 0.1,
            column_boundary_threshold_pt: 5.0,
            severe_overlap_threshold_pt: -0.5,
            use_adaptive_threshold: true,
            adaptive_config: Some(adaptive_config),
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }

    /// Create a configuration using the legacy fixed-threshold approach.
    ///
    /// This provides backward compatibility with legacy behavior where
    /// adaptive threshold was disabled by default. All thresholds are fixed values.
    ///
    /// **Default values**:
    /// - space_threshold_em_ratio: 0.25 (standard word spacing)
    /// - conservative_threshold_pt: 0.1 (tight font metric threshold)
    /// - column_boundary_threshold_pt: 5.0 (standard column separation)
    /// - severe_overlap_threshold_pt: -0.5 (standard overlap tolerance)
    /// - use_adaptive_threshold: false (no automatic adjustment)
    ///
    /// # When to Use
    ///
    /// Use this when you need the fixed-threshold behavior:
    /// - Testing regression against old baselines
    /// - Documents with known quirks that required specific thresholds
    /// - Performance-critical applications where adaptive overhead is unacceptable
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::SpanMergingConfig;
    ///
    /// let config = SpanMergingConfig::legacy();
    /// assert!(!config.use_adaptive_threshold);
    /// assert_eq!(config.conservative_threshold_pt, 0.1);
    /// ```
    pub fn legacy() -> Self {
        Self {
            space_threshold_em_ratio: 0.25,
            conservative_threshold_pt: 0.1,
            column_boundary_threshold_pt: 5.0,
            severe_overlap_threshold_pt: -0.5,
            use_adaptive_threshold: false,
            adaptive_config: None,
            detect_email_patterns: false,
            email_threshold_multiplier: 2.5,
            detect_citation_markers: false,
            citation_font_size_ratio: 0.75,
            merge_tm_tj_runs: true,
        }
    }
}

/// Unified space decision function - SINGLE SOURCE OF TRUTH for space insertion.
///
/// This function consolidates all space insertion logic into one place per the
/// design principle in the comprehensive plan. It evaluates multiple signals
/// returns a definitive decision about whether to insert a space between spans.
///
/// # Rules (in priority order)
///
/// **Rule 0**: Check if boundary space already exists (from trailing/leading whitespace)
/// - If preceding text ends with space OR following text starts with space, don't insert
/// - Confidence: 1.0 (deterministic)
///
/// **Rule 1**: TJ offset triggered flag
/// - If the TJ processor set the flag due to negative offset > threshold, insert space
/// - This is explicit PDF positioning information
/// - Confidence: 0.95 (highest, explicit signal)
///
/// **Rule 2**: Dual threshold (PDFBox pattern) with document-type adjustment
/// - Calculate both space-width-based and char-width-based thresholds
/// - Adjust thresholds based on document type (Academic/Policy/Mixed)
/// - Use MINIMUM of the two for robustness
/// - If gap exceeds this threshold, insert space
/// - Confidence: 0.8 (geometric measurement)
///
/// **Rule 3**: Character heuristic (CamelCase, number->letter, etc.)
/// - Detect character transitions indicating word boundaries
/// - If heuristic fires, insert space
/// - Confidence: 0.6 (pattern-based)
///
/// **Rule 4**: Conservative threshold (document-type aware)
/// - If gap exceeds conservative threshold (very small), insert space
/// - Catches small intentional gaps that are still word boundaries
/// - Adaptive to document type (Policy uses lower threshold, Academic uses higher)
/// - Confidence: 0.5 (conservative)
///
/// **Default**: No space inserted
///
/// # Document Type Adjustment
///
/// When document_type is provided, thresholds are adjusted:
/// - **Academic** (1.4x multiplier): Higher thresholds for loose spacing
/// - **Policy** (0.6x multiplier): Lower thresholds for tight justified text
/// - **Mixed** (1.0x multiplier): Default/balanced approach
///
/// This matches research findings from LA-PDFText, pdfminer.six, PDFBox, and iText
/// that adaptive thresholds provide better results than fixed values.
///
/// # PDF Spec Reference
///
/// ISO 32000-1:2008, Section 9.4.4 NOTE 6:
/// "The identification of what constitutes a word is unrelated to how the text
/// happens to be grouped into show strings... text strings should be as long as possible."
/// Recover an honest inter-glyph gap for the space-insertion decision.
///
/// Per ISO 32000-1:2008 §9.4.4, the spacing between two glyphs is the
/// text-space displacement between their origins; a word space exists when
/// that displacement reaches the font's space advance. We measure it from
/// the bounding boxes (`raw_gap = next.x − prev.right_edge`).
///
/// When the previous span's font has no explicit `/Widths` array,
/// `FontInfo` substitutes a fixed fallback advance (~0.55 em) that
/// systematically OVER-reports proportional Latin glyphs. That inflates
/// `bbox.width`, pushing `prev.right_edge` past the real glyph end so it can
/// swallow a true word gap and drive `raw_gap` NEGATIVE — glyphs that do not
/// actually overlap appear to. Only in that overlap case do we
/// divide out the fallback inflation (0.55 em ÷ 0.45 em ≈ 1.22) to restore a
/// believable gap.
///
/// Crucially, the correction is applied ONLY when `raw_gap < 0`. When the
/// glyphs do not overlap (`raw_gap ≥ 0`) the layout is already honest
/// must not be second-guessed: inflating a non-overlapping gap manufactures
/// a phantom word space and splits single words that were positioned
/// edge-to-edge — e.g. a CamelCase brand "SalesForce" emitted as
/// "SalesF" + "orce" with `raw_gap == 0` would otherwise be torn into
/// "SalesF orce". (`bbox.width × (1 − 1/1.22)` is the algebraic form of
/// `next.x − (prev.x + width/1.22)` once `raw_gap` is substituted in.)
fn corrected_space_gap(raw_gap: f32, reliable_widths: bool, bbox_width: f32, text_empty: bool) -> f32 {
    if !reliable_widths && raw_gap < 0.0 && bbox_width > 0.0 && !text_empty {
        raw_gap + bbox_width * (1.0 - 1.0 / 1.22)
    } else {
        raw_gap
    }
}

/// detect whether a glyph's mapped text
/// represents an AGL Latin ligature (`/ff` / `/fi` / `/fl` / `/ffi` /
/// `/ffl`). When the upstream space-emission heuristic processes a
/// glyph adjacent to a ligature, the small intra-word kerning that
/// surrounds the ligature glyph can trigger spurious space
/// insertion (producing `di ff cult` for `difficult`). The detection
/// here lets the heuristic suppress space insertion at ligature
/// boundaries.
///
/// Returns true when the text *is* a bare AGL ligature glyph — a
/// single codepoint in the Latin Ligatures block (U+FB00..U+FB06) or
/// the multi-char ASCII fallback ("ff"/"fi"/"fl"/"ffi"/"ffl"). The
/// suppression at the call site targets the pdfTeX-style emission
/// pattern where the ligature is its own cluster between two
/// intra-word fragments (e.g. "di"→"ﬃ"→"cult" or "di"→"ffi"→"cult").
/// A multi-char cluster that merely starts with a ligature
/// (e.g. "ﬂuid" or "ffective") is a full word whose boundary with the
/// previous span is a legitimate space, so we return false in that
/// case.
#[inline]
pub(crate) fn starts_with_agl_ligature(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if ('\u{FB00}'..='\u{FB06}').contains(&first) && chars.next().is_none() {
        return true;
    }
    // Multi-character AGL outputs from non-PUA fallbacks — match only
    // when the cluster IS the ligature, never when it just begins
    // with one. ~keep
    matches!(text, "ff" | "fi" | "fl" | "ffi" | "ffl")
}

/// detect monospace fonts by name.
/// Monospace fonts emit one show-text op per glyph with one-em
/// advance positioning, which triggers the proportional-font space-
/// emission heuristic to fire inside ordinary tokens. Bumping the
/// threshold for these fonts closes the `function add (a , b )` repro
/// for monospace-font code listings. Used by
/// [`should_insert_space`] to switch its `word_margin_ratio` to
/// `1.2` for monospace.
///
/// Names matched case-insensitively. Covers the major monospace
/// families on macOS / Linux / Windows + the pdfTeX-emitted
/// Computer Modern Typewriter (CMTT*) and Latin Modern Mono
/// (LMMono*) families that frequently appear in academic PDFs.
pub(crate) fn is_monospace_font(font_name: &str) -> bool {
    let lower = font_name.to_lowercase();
    // "Monotype" is a type-foundry name, not a monospace indicator. A bare substring
    // match on "mono" would otherwise misclassify script/display faces sold under that
    // foundry name (e.g. Monotype Corsiva) as monospace, so it is excluded explicitly
    // rather than folded into the marker list below. Genuinely monospace faces that
    // happen to be Monotype-branded (e.g. "Monotype Consolas") still match via their
    // own marker ("consolas") below, and ordinary "*mono*"-named families (PT Mono,
    // Roboto Mono, Nimbus Mono, DejaVu Sans Mono, Fira Mono, LMMono, ...) are unaffected
    // since none of them contain "monotype". ~keep
    if lower.contains("mono") && !lower.contains("monotype") {
        return true;
    }
    const MONO_MARKERS: &[&str] = &[
        "courier",
        "consolas",
        "menlo",
        "fira code", // does NOT match "Fira Sans" (proportional) ~keep
        "source code",
        "inconsolata",
        "cmtt", // pdfTeX Computer Modern Typewriter ~keep
        "letter gothic",
        "ocr ", // OCR-A, OCR-B ~keep
        "fixedsys",
        "terminal",
    ];
    MONO_MARKERS.iter().any(|m| lower.contains(m))
}

/// True for codepoints in the main emoji / pictographic blocks.
///
/// Used only as a word-spacing hint — ISO 32000-1:2008 §9.10 leaves word
/// segmentation to the reader. Deliberately **excludes** arrows
/// (U+2190–U+21FF) and the math-operator blocks so symbolic/technical text is
/// unaffected; restricted to clearly pictographic ranges plus the VS16 emoji
/// presentation selector.
pub(crate) fn is_pictographic(c: char) -> bool {
    matches!(c as u32,
        0x1F300..=0x1FAFF   // Misc & Supplemental Symbols and Pictographs, Ext-A ~keep
        | 0x1F000..=0x1F0FF // Mahjong / Dominoes / Playing cards ~keep
        | 0x2600..=0x27BF   // Misc Symbols + Dingbats ~keep
        | 0xFE0F) // VS16 emoji presentation selector ~keep
}

/// Remove an ASCII space sitting directly between a CJK ideograph/kana and an
/// ASCII digit (either direction). In Chinese and Japanese an embedded number
/// attaches to the surrounding ideographs with no space (e.g. "公元前1000年",
/// "10,000年"); some producers — notably headless-browser print-to-PDF — emit a
/// stray space glyph at that script transition. CJK↔CJK and CJK↔letter spacing
/// is left untouched, so genuine word/term spacing is preserved.
///
/// Hangul is deliberately EXCLUDED: Korean, unlike Chinese/Japanese, is written
/// with inter-word spaces, so a space between a Korean syllable and a number is
/// a real word boundary (e.g. "14 예" = "14 cases", "7 예중") — stripping it
/// corrupts the text. Only the space-less scripts (CJK ideographs + kana) are
/// treated as number-adjacent.
pub(crate) fn strip_cjk_digit_boundary_spaces(text: &str) -> String {
    if !text.contains(' ') {
        return text.to_string();
    }
    let is_cjk = |c: char| {
        matches!(c as u32,
            0x3040..=0x30FF      // Hiragana + Katakana ~keep
            | 0x3400..=0x4DBF    // CJK Ext A ~keep
            | 0x4E00..=0x9FFF    // CJK Unified ~keep
            | 0x20000..=0x2A6DF  // CJK Ext B ~keep
            | 0xFF66..=0xFF9F    // Halfwidth Katakana ~keep
        )
    };
    let is_cjk_or_hangul = |c: char| {
        is_cjk(c)
            || matches!(c as u32,
                0xAC00..=0xD7A3   // Hangul syllables ~keep
                | 0x1100..=0x11FF // Hangul Jamo ~keep
                | 0x3130..=0x318F // Hangul Compatibility Jamo ~keep
            )
    };
    let is_hug_bracket = |c: char| matches!(c, '(' | ')' | '[' | ']' | '{' | '}');
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' && i > 0 && i + 1 < chars.len() {
            let (p, n) = (chars[i - 1], chars[i + 1]);
            if (is_cjk(p) && n.is_ascii_digit()) || (p.is_ascii_digit() && is_cjk(n)) {
                i += 1;
                continue;
            }
            if (is_cjk_or_hangul(p) && is_hug_bracket(n)) || (is_hug_bracket(p) && is_cjk_or_hangul(n)) {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Remove an ASCII space that the geometric word-break heuristic injected inside
/// a prime-notation number, e.g. `0′′.28` → `0′′ .28` or `0′′. 28`.
///
/// Arc-second / arc-minute values attach their decimal fraction to the prime
/// without a break (`0′′.28`, `1′′.47`). A prime glyph's metric advance (w0,
/// ISO 32000-1 §9.4.4) is narrow relative to its inked form, so the gap to the
/// following `.NN` reads as wider than a space and the heuristic splits the
/// token. Two artifact positions are repaired:
///   • prime → `.`   (`′ .` → `′.`)
///   • `.` → digit, when the `.` directly follows a prime (`′. 2` → `′.2`)
///
/// Feet-and-inches like `5′ 6″` are left untouched: the space there sits between
/// a prime and a *digit* (not a `.`), which is a genuine measurement boundary.
pub(crate) fn strip_prime_decimal_boundary_spaces(text: &str) -> String {
    if !text.contains(' ') {
        return text.to_string();
    }
    let is_prime = |c: char| matches!(c, '\u{2032}' | '\u{2033}' | '\u{2034}');
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' && i > 0 && i + 1 < chars.len() {
            let (p, n) = (chars[i - 1], chars[i + 1]);
            if is_prime(p) && n == '.' {
                i += 1;
                continue;
            }
            if p == '.' && n.is_ascii_digit() && i >= 2 && is_prime(chars[i - 2]) {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// True when any drawn glyph run puts ink inside the horizontal gap between
/// `left` and `right`, overlapping their vertical band.
///
/// Used by the decimal-value merge: two pure-digit runs a split-box-sized
/// gap apart merge into one decimal amount ONLY if the gap is empty. A
/// separator glyph occupying the gap — the comma of a subscript index pair
/// (`P_{1,0}`), a list delimiter — proves the runs are distinct tokens, no
/// matter where in the content stream it was drawn. The pair's own boxes
/// bound the gap exactly, so a small epsilon keeps them (and touching
/// neighbours) from counting as intruders.
fn decimal_gap_has_ink(ink_boxes: &[Rect], left: &Rect, right: &Rect) -> bool {
    const EPS: f32 = 0.01;
    let gap_start = left.x + left.width;
    let gap_end = right.x;
    if gap_end - gap_start <= 2.0 * EPS {
        return false;
    }
    let band_bottom = left.y.min(right.y);
    let band_top = (left.y + left.height).max(right.y + right.height);
    ink_boxes.iter().any(|b| {
        b.x + b.width > gap_start + EPS && b.x < gap_end - EPS && b.y < band_top && b.y + b.height > band_bottom
    })
}

/// True when a *full intervening glyph* occupies the horizontal gap between
/// `left` and `right` — e.g. a subscript drawn between a variable and the next
/// symbol (`λᵢr…`), which inflates the `λ`→`r` gap though both share a
/// baseline. Distinct from [`decimal_gap_has_ink`]: it requires an ink box to
/// cover a substantial fraction (>= 35%) of the gap width, so a mere
/// descender/ascender edge of an adjacent glyph clipping the gap band does NOT
/// count. Used by the narrow-word-gap rescue to suppress splitting a math
/// sub/superscript from its base while still recovering ordinary prose word
/// gaps (whose gaps are empty of intervening ink).
fn gap_has_intervening_glyph(ink_boxes: &[Rect], left: &Rect, right: &Rect) -> bool {
    let gap_start = left.x + left.width;
    let gap_end = right.x;
    let gap_w = gap_end - gap_start;
    if gap_w <= 0.5 {
        return false;
    }
    let band_bottom = left.y.min(right.y);
    let band_top = (left.y + left.height).max(right.y + right.height);
    ink_boxes.iter().any(|b| {
        let overlap = (b.x + b.width).min(gap_end) - b.x.max(gap_start);
        overlap > gap_w * 0.35 && b.y < band_top && b.y + b.height > band_bottom
    })
}

fn should_insert_space(
    preceding_text: &str,
    following_text: &str,
    gap_pt: f32,
    font_size: f32,
    font_name: &str,
    fonts: &std::collections::HashMap<String, std::sync::Arc<crate::fonts::FontInfo>>,
    tj_offset_triggered: bool,
    config: &SpanMergingConfig,
    prev_bbox: Option<&crate::geometry::Rect>,
    next_bbox: Option<&crate::geometry::Rect>,
    prev_font_size: f32,
    next_font_size: f32,
) -> SpaceDecision {
    if has_boundary_space(preceding_text, following_text) {
        return SpaceDecision::no_space(SpaceSource::AlreadyPresent, 1.0);
    }

    if let (Some(pc), Some(nc)) = (preceding_text.chars().next_back(), following_text.chars().next()) {
        use crate::text::complex_script_detector::{detect_complex_script, is_complex_script_mark};
        if is_complex_script_mark(pc as u32) && detect_complex_script(nc as u32).is_some() {
            return SpaceDecision::no_space(SpaceSource::NoSpace, 0.9);
        }
    }

    if gap_pt >= 0.0
        && preceding_text.chars().next_back().is_some_and(is_pictographic)
        && following_text.chars().next().is_some_and(char::is_alphabetic)
    {
        return SpaceDecision::insert(SpaceSource::GeometricGap, 0.85);
    }

    if config.detect_email_patterns && is_email_context(preceding_text, following_text) {
        let geometric_threshold = if let Some(font_info) = fonts.get(font_name) {
            let space_width_units = font_info.get_space_glyph_width();
            let space_width_pt = (space_width_units / 1000.0) * font_size;
            let word_margin_ratio = 0.5;
            space_width_pt * word_margin_ratio
        } else {
            font_size * 0.25
        };

        let email_threshold = geometric_threshold * config.email_threshold_multiplier;

        if gap_pt > email_threshold {
            tracing::trace!(
                "Email context detected: gap={:.2}pt > {:.2}pt email threshold - inserting space",
                gap_pt,
                email_threshold
            );
            return SpaceDecision::insert(SpaceSource::GeometricGap, 0.85);
        }

        tracing::trace!(
            "Email context detected: gap={:.2}pt <= {:.2}pt email threshold - suppressing space",
            gap_pt,
            email_threshold
        );
        return SpaceDecision::no_space(SpaceSource::NoSpace, 1.0);
    }

    // Line Break Handling
    // ==============================================================================
    // Per ISO 32000-1:2008 Section 5.2 (geometric positioning):
    // Line breaks are detected using bbox Y-coordinates (vertical positioning).
    // Words split across lines need special handling:
    // - Soft hyphen breaks: Previous text ends with '-' → NO space (word continuation)
    // - Hard line breaks: Normal breaks → INSERT space (new word on next line)
    //
    // Spec Reference: Section 5.2 states coordinates are in user space units.
    // Font size is used as reference for vertical gap detection threshold. ~keep

    if let (Some(prev_box), Some(next_box)) = (prev_bbox, next_bbox) {
        // Calculate vertical positioning for line break detection.
        // Use Y-coordinate difference (not bottom-to-top gap) to detect actual line breaks.
        // Two spans on the same line have nearly identical Y positions regardless of height. ~keep
        let y_diff = (prev_box.y - next_box.y).abs();

        // Line break threshold: if Y positions differ by more than 0.5× font size ~keep
        let line_break_threshold = font_size * 0.5;
        let is_line_break = y_diff > line_break_threshold;

        if is_line_break {
            let same_column = (prev_box.left() - next_box.left()).abs() < (font_size * 2.0);

            if same_column {
                tracing::trace!(
                    "Detected line break: y_diff={:.2}pt > {:.2}pt threshold, same_column=true",
                    y_diff,
                    line_break_threshold
                );

                if preceding_text.ends_with('-') {
                    tracing::trace!(
                        "Soft hyphen detected: '{}' ends with '-', suppressing space insertion",
                        preceding_text
                    );
                    return SpaceDecision::no_space(SpaceSource::NoSpace, 1.0);
                } else {
                    tracing::trace!("Hard line break detected: inserting space for word continuation");
                    return SpaceDecision::insert(SpaceSource::GeometricGap, 0.9);
                }
            }
        }
    }

    if config.detect_citation_markers
        && is_citation_context(prev_bbox, next_bbox, font_size, prev_font_size, next_font_size)
    {
        // For citations, use single-signal detection (don't require consensus)
        // Compute geometric threshold for citation context ~keep
        let citation_geometric_threshold = if let Some(font_info) = fonts.get(font_name) {
            let space_width_units = font_info.get_space_glyph_width();
            let space_width_pt = (space_width_units / 1000.0) * font_size;
            space_width_pt * 0.5
        } else {
            font_size * 0.25
        };

        if tj_offset_triggered || gap_pt > citation_geometric_threshold {
            tracing::trace!(
                "Citation context detected: using relaxed spacing rules (gap={:.2}pt, tj={})",
                gap_pt,
                tj_offset_triggered
            );
            return SpaceDecision::insert(SpaceSource::TjOffset, 0.90);
        }
    }

    // Consensus-Based Spacing Logic
    // ==============================================================================
    // Per ISO 32000-1:2008 Section 9.4.4 and 9.10:
    // "Determining word boundaries is not specified by PDF."
    // TJ offsets are typographic hints only, not definitive word boundaries.
    //
    // Solution: Require CONSENSUS between multiple PDF-spec-defined signals:
    // - TJ offset signal (explicit typography positioning)
    // - Geometric signal (bounding box analysis)
    // - Strong geometric signal alone is sufficient (gap > 2× threshold) ~keep

    // Rule 1: TJ Offset Signal (Section 9.4.3) - PDF-spec explicit signal
    // Calculate font-aware geometric threshold for consensus checking ~keep
    let geometric_threshold = if let Some(font_info) = fonts.get(font_name) {
        let space_width_units = font_info.get_space_glyph_width(); // in 1000ths of em ~keep
        let space_width_pt = (space_width_units / 1000.0) * font_size;
        // monospace fonts emit one show-text
        // op per glyph at one-em-advance positioning, so the gap
        // between glyphs in normal tokens briefly exceeds the
        // proportional-font threshold. Use a 1.2× ratio for monospace
        // so spurious spaces around punctuation in code listings
        // (`function add (a , b )` → `function add(a, b)`) don't fire. ~keep
        let mut word_margin_ratio = if is_monospace_font(font_name) {
            1.2
        } else {
            0.5 // 50% of space width (proportional default) ~keep
        };
        // when prev_font_size
        // next_font_size differ significantly, we're at a font-run
        // boundary (italic → roman, bold → regular, or a font-family
        // switch). PdfTeX-typeset titles like
        // `Astronomy & Astrophysicsmanuscript no.` exhibit this when
        // the writer doesn't emit an explicit space-glyph at the font
        // switch. Reduce the threshold by 30% at boundaries so a
        // smaller gap suffices to trigger space insertion. The full
        // fix (font-name plumbing for italic→roman within same size)
        // is tracked in — many italic transitions
        // share font_size, so this only catches the size-changing
        // subset. ~keep
        if (prev_font_size - next_font_size).abs() > 0.5 {
            word_margin_ratio *= 0.7;
        }
        let threshold = space_width_pt * word_margin_ratio;

        tracing::trace!(
            "Font-aware spacing for '{}' @ {:.1}pt: space_width={:.1}pt, threshold={:.1}pt (mono={})",
            font_name,
            font_size,
            space_width_pt,
            threshold,
            is_monospace_font(font_name),
        );

        threshold
    } else {
        tracing::trace!(
            "Font '{}' not found in font map, using default 0.25em threshold for {:.1}pt",
            font_name,
            font_size
        );
        font_size * 0.25
    };

    // suppress space insertion at AGL-
    // ligature boundaries. When the preceding or following text
    // starts with one of the Latin ligature codepoints (U+FB00..U+FB04)
    // or matches the multi-char AGL ligature names, the small kerning
    // gap that surrounds the ligature glyph is NOT a word boundary —
    // it's an intra-word position artefact from pdfTeX-style ligature
    // emission. Inflating the threshold by 1.5× at these positions
    // catches the `di ff cult` → `difficult` repro. ~keep
    let ligature_boundary = starts_with_agl_ligature(following_text)
        || preceding_text
            .chars()
            .last()
            .map(|c| ('\u{FB00}'..='\u{FB06}').contains(&c))
            .unwrap_or(false);
    let geometric_threshold = if ligature_boundary {
        geometric_threshold * 1.5
    } else {
        geometric_threshold
    };

    let geometric_suggests_space = gap_pt > geometric_threshold;

    // Intra-word kerning guard (letter-letter branch).
    //
    // On TJ-heavy producers (LaTeX, MS Word → PDF) the Primary
    // word-boundary detector hands `should_insert_space` two adjacent
    // clusters like "cha"→"nge", "diffe"→"rent", "equivalen"→"t"
    // whose gap sits just above `geometric_threshold` (= 0.5 ×
    // space-glyph width) but well below a real word gap. The
    // consensus rule below would then emit a spurious space mid-word.
    // Real word gaps in real producers reach one full space-glyph
    // width or sit next to punctuation/digits, both of which fall
    // through this guard.
    //
    // The guard fires regardless of `tj_offset_triggered` because the
    // gap can also be geometric-only (when WordBoundaryDetector splits
    // the cluster but no explicit TJ offset crossed the threshold).
    // See the sibling guard in `process_tj_array_tiebreaker` for the
    // upstream space-as-span insertion path.
    //
    // Ceiling = 1.5 × `geometric_threshold` (= 0.75 × space-glyph width,
    // ≈ 0.2 em for a typical 0.25-em space). Inter-letter kerning is a
    // property of font size — realistic microtype / Word letter-spacing
    // is a few percent of the em and sits just above the 0.5-space-width
    // threshold, never far beyond it. The previous 2.4× ceiling
    // (≈ 1.2 × a full space-glyph advance, ≈ 0.33 em for Helvetica) was
    // far wider than any real kerning and swallowed genuine *tight* word
    // gaps between lowercase words — the dominant cause of
    // "MasterofScience" / "Resultsdriven" gluing on resume-style PDFs
    // that position words via small Td offsets. 1.5× still clears
    // worst-case ~0.19-em intra-word kerning (including the ~0.15-em
    // LaTeX/microtype letter-spacing case) while letting a
    // 0.2-em-and-wider word gap through to the consensus path — the same
    // ~0.18-0.2-em word-break point PyMuPDF / poppler use. Gaps in the
    // overlap zone (wide letter-tracking in titles, ~0.28 em) are not
    // separable from real word gaps by magnitude alone and fall through.
    //
    // Only fires when the font is available so the threshold is
    // computed from the font's own space-glyph advance — the no-font
    // fallback (`font_size * 0.25`) is a wider, deliberately
    // conservative value that already separates real word gaps from
    // kerning at the consensus level. ~keep
    let kerning_guard_threshold = if fonts.contains_key(font_name) {
        Some(geometric_threshold * 1.5)
    } else {
        None
    };
    if let Some(thr) = kerning_guard_threshold
        && gap_pt < thr
    {
        let prev_last = preceding_text.chars().last();
        let next_first = following_text.chars().next();
        if let (Some(pc), Some(nc)) = (prev_last, next_first) {
            // Use is_lowercase on both sides: LaTeX/microtype intra-word kerning
            // occurs within lowercase letter runs. Real word boundaries in
            // professional PDFs frequently involve uppercase letters (headings,
            // abbreviations, proper nouns) — those fall through to the consensus
            // path, avoiding word-gluing like "APPENDIXA" or "OLIVERA.". ~keep
            if pc.is_lowercase() && nc.is_lowercase() {
                tracing::trace!(
                    "intra-word kerning guard: suppressing space between '{pc}' and '{nc}' (gap={gap_pt:.2}pt < {thr:.2}pt, threshold = 0.75× space-glyph width)"
                );
                return SpaceDecision::no_space(SpaceSource::IntraWordKerning, 0.9);
            }
        }
    }

    if tj_offset_triggered && geometric_suggests_space {
        tracing::trace!(
            "Space decision: CONSENSUS - both TJ and geometric signals triggered (gap={:.2}pt > {:.2}pt) - inserting space",
            gap_pt,
            geometric_threshold
        );
        return SpaceDecision::insert(SpaceSource::TjOffset, 1.0);
    }

    // TJ offset with relaxed geometric confirmation
    // In tight typesetting (e.g., LaTeX academic papers), word gaps are narrower than
    // the standard 50% space-width threshold. When the PDF producer explicitly encoded
    // a TJ offset, accept a lower geometric bar (25% of space width). ~keep
    if tj_offset_triggered && gap_pt > geometric_threshold * 0.5 {
        tracing::trace!(
            "Space decision: TJ + relaxed geometric (gap={:.2}pt > {:.2}pt relaxed threshold) - inserting space",
            gap_pt,
            geometric_threshold * 0.5
        );
        return SpaceDecision::insert(SpaceSource::TjOffset, 0.9);
    }

    // WordBoundaryDetector tiebreaker when TJ and geometric signals conflict
    // Per ISO 32000-1:2008 Section 9.4.4, use multiple signals to determine word boundaries ~keep
    if tj_offset_triggered != geometric_suggests_space
        && let (Some(prev_box), Some(next_box)) = (prev_bbox, next_bbox)
    {
        let (characters, context) = build_boundary_characters(
            preceding_text,
            following_text,
            prev_box,
            next_box,
            font_size,
            tj_offset_triggered,
        );

        // Use WordBoundaryDetector with geometric gap ratio matching our threshold
        // OPTIMIZATION: Detect document script profile to skip unnecessary detectors ~keep
        let script = DocumentScript::detect_from_characters(&characters);
        let detector = WordBoundaryDetector::new()
            .with_document_script(script)
            .with_geometric_gap_ratio(0.5);
        let boundaries = detector.detect_word_boundaries(&characters, &context);

        if !boundaries.is_empty() {
            tracing::trace!(
                "Space decision: WordBoundaryDetector resolved conflict (TJ={}, geo={}) - inserting space",
                tj_offset_triggered,
                geometric_suggests_space
            );
            return SpaceDecision::insert(SpaceSource::WordBoundaryAnalysis, 0.85);
        }
    }

    // Strong geometric signal alone.
    //
    // `geometric_threshold` is already `space_width_pt * 0.5`. A gap that
    // clears this threshold is >= 50 % of the font's own space-glyph
    // advance, which is what pdfium (Chrome/pypdfium2) uses as the
    // word-break heuristic in its default text-extraction path —
    // the reason xberg-native-pdf was glueing adjacent words like
    // "atBirmingham", "LIFESCIENCESRESEARCH", "STATIONFREEDOM",
    // "proteincrystals" before this change. The previous 2× multiplier
    // required gaps >= 100 % of a full space glyph, which is stricter
    // than the gaps modern tightly-kerned typesetters emit between
    // real words (often 60-80 % of a space glyph).
    //
    // Intra-word kerning and letter-spacing adjustments are well below
    // 50 % of a space glyph (typically under 5 % of font-size), so
    // lowering this threshold does not produce false word breaks
    // inside words. Pure digit-digit sequences are separately protected
    // in the value/token branch below via `digit_digit_gap_ok`.
    //
    // A corpus-wide measurement motivated this change (NASA Apollo 11
    // jaccard 0.449 → target >= 0.90 vs pypdfium2 on a 60-PDF regression
    // corpus). ~keep
    if gap_pt > geometric_threshold {
        tracing::trace!(
            "Space decision: STRONG GEOMETRIC - gap={:.2}pt > {:.2}pt threshold - inserting space",
            gap_pt,
            geometric_threshold
        );
        return SpaceDecision::insert(SpaceSource::GeometricGap, 0.95);
    }

    // Separate token detection: when two spans have a positive gap and look like
    // distinct values (not fragments of the same word), insert a space.
    //
    // This catches adjacent table cell values like "$0.00" "$0.00" that have small
    // gaps (1-2pt) which fall below the standard geometric threshold but are clearly
    // separate tokens. Word fragments within the same word have zero or near-zero
    // gaps; any meaningful positive gap between non-fragment tokens indicates a
    // word boundary.
    //
    // Heuristic: gap > 0 AND spans look like separate tokens based on boundary characters.
    // Use near-zero threshold for currency boundaries (any positive gap = separate) ~keep
    let min_token_gap = 0.01; // Essentially any positive gap triggers token check ~keep
    if gap_pt > min_token_gap {
        let prev_last = preceding_text.chars().last();
        let next_first = following_text.chars().next();

        if let (Some(pc), Some(nc)) = (prev_last, next_first) {
            // Separate value tokens: digit/currency/punctuation boundaries that
            // indicate two distinct values rather than fragments of one word.
            // Examples: "$0.00" + "$0.00", "100" + "200", "Subtotal" + "$500.00" ~keep
            let prev_is_value_end = pc.is_ascii_digit() || pc == '%' || pc == ')' || pc == ']';

            // Pure digit→digit boundaries require a larger gap than the
            // global `min_token_gap`: a long number emitted as multiple
            // spans (e.g. due to glyph-level kerning or TJ positioning
            // rounding) can have a tiny positive gap between adjacent
            // digit spans, which must NOT become "123 456". Anything less
            // than half the font-aware geometric threshold is treated as
            // intra-number kerning, not a token boundary. ~keep
            let digit_digit = nc.is_ascii_digit() && pc.is_ascii_digit();
            let digit_digit_gap_ok = !digit_digit || gap_pt > geometric_threshold * 0.5;

            let next_is_value_start = nc == '$'
                || nc == '('
                || nc == '['
                || (nc == '-' && following_text.len() > 1)
                || (nc.is_ascii_digit() && prev_is_value_end && digit_digit_gap_ok);

            let text_then_currency =
                (pc.is_ascii_alphabetic() || pc.is_ascii_digit()) && (nc == '$' || nc == '€' || nc == '£');

            if (prev_is_value_end && next_is_value_start) || text_then_currency {
                tracing::trace!(
                    "Space decision: SEPARATE VALUES - gap={:.2}pt > {:.2}pt min, prev='{}', next='{}' - inserting space",
                    gap_pt,
                    min_token_gap,
                    crate::utils::safe_suffix(preceding_text, 5),
                    crate::utils::safe_prefix(following_text, 5),
                );
                return SpaceDecision::insert(SpaceSource::GeometricGap, 0.85);
            }
        }
    }

    // Default: No space
    // Per ISO 32000-1:2008 Section 9.10, when PDF doesn't encode a clear word boundary,
    // we cannot reliably recover it. Requiring consensus prevents false positives in justified text.
    // ~keep
    tracing::trace!(
        "Space decision: Insufficient consensus (TJ={}, gap={:.2}pt <= {:.2}pt) - no space",
        tj_offset_triggered,
        gap_pt,
        geometric_threshold
    );
    SpaceDecision::no_space(SpaceSource::NoSpace, 1.0)
}

/// Check if a boundary between spans already has whitespace.
///
/// Returns true if:
/// - The preceding text ends with whitespace, OR
/// - The following text starts with whitespace
///
/// This prevents double-spacing when text already contains space characters.
fn has_boundary_space(preceding: &str, following: &str) -> bool {
    // Use ends_with/starts_with patterns instead of .chars().last() to avoid
    // O(n) iteration over the entire accumulated text ~keep
    let has_trailing_space = preceding.ends_with(|c: char| c.is_whitespace());
    let has_leading_space = following.starts_with(|c: char| c.is_whitespace());

    has_trailing_space || has_leading_space
}

/// Build CharacterInfo for word boundary analysis between two text segments.
///
/// Creates minimal character info for the last character of the preceding text
/// and the first character of the following text. This allows WordBoundaryDetector
/// to determine if a word boundary exists between two spans.
///
/// Per ISO 32000-1:2008 Section 9.4.4, word boundaries can be identified through:
/// - TJ array offsets (passed via tj_offset_triggered)
/// - Geometric gaps between glyphs (calculated from bbox positions)
/// - Space characters in the text stream
/// - CJK character transitions
fn build_boundary_characters(
    prev_text: &str,
    next_text: &str,
    prev_bbox: &Rect,
    next_bbox: &Rect,
    font_size: f32,
    tj_offset_triggered: bool,
) -> (Vec<CharacterInfo>, BoundaryContext) {
    let prev_last_char = prev_text.chars().last().unwrap_or(' ');
    let next_first_char = next_text.chars().next().unwrap_or(' ');

    // Estimate character widths from bbox and character count
    // Use byte length as fast O(1) approximation (accurate for ASCII, close for UTF-8)
    // to avoid O(n) char counting on the accumulated merge text ~keep
    let prev_char_count = prev_text.len().max(1) as f32;
    let prev_char_width = prev_bbox.width / prev_char_count;
    let prev_last_x = prev_bbox.x + prev_bbox.width - prev_char_width;

    let next_char_count = next_text.len().max(1) as f32;
    let next_char_width = next_bbox.width / next_char_count;

    let characters = vec![
        CharacterInfo {
            code: prev_last_char as u32,
            glyph_id: None,
            width: prev_char_width,
            x_position: prev_last_x,
            // Convert TJ trigger to offset value: -200 indicates word boundary ~keep
            tj_offset: if tj_offset_triggered { Some(-200) } else { None },
            font_size,
            is_ligature: false, // Not relevant for tiebreaker mode ~keep
            original_ligature: None,
            protected_from_split: false,
        },
        CharacterInfo {
            code: next_first_char as u32,
            glyph_id: None,
            width: next_char_width,
            x_position: next_bbox.x,
            tj_offset: None,
            font_size,
            is_ligature: false, // Not relevant for tiebreaker mode ~keep
            original_ligature: None,
            protected_from_split: false,
        },
    ];

    let context = BoundaryContext {
        font_size,
        horizontal_scaling: 100.0, // Default; actual value not available at span level ~keep
        word_spacing: 0.0,
        char_spacing: 0.0,
    };

    (characters, context)
}

/// Check if surrounding text forms an email-like pattern.
/// Per PDF spec, uses only extracted text pattern matching.
///
/// Patterns detected:
/// - "user@outlook" + "." + "com" (space before TLD)
/// - "user@" + "domain.com" (space after @)
fn is_email_context(preceding_text: &str, following_text: &str) -> bool {
    // Only check the last ~64 bytes for email patterns to avoid O(n) scan
    // of the entire accumulated text (which would cause O(n²) in merge loop) ~keep
    let prev_start = preceding_text.len().saturating_sub(64);
    // Round up to the next UTF-8 char boundary. `str::ceil_char_boundary`
    // would do this in one line but it's only stable since Rust 1.91,
    // above our MSRV (1.88 — pinned by transitive deps). ~keep
    let prev_start = {
        let mut i = prev_start;
        while i < preceding_text.len() && !preceding_text.is_char_boundary(i) {
            i += 1;
        }
        i
    };
    let prev = preceding_text[prev_start..].trim_end();
    let next = following_text.trim_start();

    if prev.contains('@') {
        let after_at = prev.split('@').next_back().unwrap_or("");

        if !after_at.is_empty() && next.starts_with('.') {
            return true;
        }

        if after_at.ends_with('.') && next.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            return true;
        }
    }

    if prev.ends_with('@') && next.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
        return true;
    }

    false
}

/// Detect if bounding boxes indicate citation marker context.
/// Per PDF spec Section 9.3, citation markers have distinct visual properties:
/// - Smaller font size (typically 50-75% of body text)
/// - Raised position (superscript)
fn is_citation_context(
    prev_bbox: Option<&crate::geometry::Rect>,
    next_bbox: Option<&crate::geometry::Rect>,
    current_font_size: f32,
    prev_font_size: f32,
    next_font_size: f32,
) -> bool {
    let prev_ratio = prev_font_size / current_font_size;
    let next_ratio = next_font_size / current_font_size;

    // Superscript range: 50-75% of body text size ~keep
    const SUPERSCRIPT_MIN: f32 = 0.5;
    const SUPERSCRIPT_MAX: f32 = 0.75;

    let prev_is_superscript = (SUPERSCRIPT_MIN..=SUPERSCRIPT_MAX).contains(&prev_ratio);
    let next_is_superscript = (SUPERSCRIPT_MIN..=SUPERSCRIPT_MAX).contains(&next_ratio);

    if let (Some(prev_box), Some(next_box)) = (prev_bbox, next_bbox) {
        let vertical_offset = (prev_box.y - next_box.y).abs();
        let is_raised = vertical_offset > (current_font_size * 0.2);

        if (prev_is_superscript || next_is_superscript) && is_raised {
            return true;
        }
    }

    prev_is_superscript || next_is_superscript
}

/// Buffer for accumulating text from TJ array elements into a single span.
///
/// Per PDF Spec ISO 32000-1:2008, Section 9.4.4 NOTE 6:
/// "The performance of text searching (and other text extraction operations) is
/// significantly better if the text strings are as long as possible."
///
/// This buffer accumulates consecutive string elements from TJ arrays into
/// a single logical text span, only breaking on explicit word boundaries.
#[derive(Debug)]
struct TjBuffer {
    /// Accumulated Unicode text
    unicode: String,
    /// Text matrix at the start of this buffer
    start_matrix: Matrix,
    /// Font name when buffer started
    font_name: Option<String>,
    /// Fill color RGB when buffer started
    fill_color_rgb: (f32, f32, f32),
    /// Character spacing (Tc) when buffer started
    char_space: f32,
    /// Word spacing (Tw) when buffer started
    word_space: f32,
    /// Horizontal scaling (Th) when buffer started
    horizontal_scaling: f32,
    /// MCID when buffer started
    mcid: Option<u32>,
    /// Accumulated width from advance_position_for_string calls.
    /// Avoids redundant per-byte width recalculation in flush.
    accumulated_width: f32,
    /// Cached font reference — avoids per-Tj HashMap lookup in append.
    /// Set once at buffer creation, never changes (font change flushes buffer).
    cached_font: Option<Arc<FontInfo>>,
    /// Pre-computed effective font size (CTM × text_matrix scaling × font_size).
    /// Computed once at buffer creation to avoid matrix multiply + sqrt per flush.
    effective_font_size: f32,
    /// Pre-computed font weight from cached font reference.
    font_weight: FontWeight,
    /// Pre-computed italic flag from cached font reference.
    is_italic: bool,
    /// Whether the font is monospaced (from FixedPitch flag or name heuristic).
    is_monospace: bool,
    /// Per-character advance widths in text-space units (before user_h_scale).
    char_widths: Vec<f32>,
    /// Pre-computed user-space position (CTM applied to text matrix origin).
    /// Avoids two transform_point calls per flush.
    user_pos_x: f32,
    user_pos_y: f32,
    /// Pre-computed horizontal scale factor (CTM × text_matrix).
    /// Used to convert accumulated_width from text space to user space for bbox.
    user_h_scale: f32,
    /// Display rotation of this run in degrees, snapped to a quadrant when near
    /// one; `0.0` for ordinary horizontal text (see `snap_run_rotation`).
    rotation_degrees: f32,
    /// Negative determinant of the composed matrix — mirrored text (see
    /// `run_is_mirrored`), carried onto the emitted span so `page_bbox`
    /// reflects rather than rotates its across-axis.
    mirrored: bool,
    /// Writing mode (0 = horizontal, 1 = vertical) captured from the
    /// graphics state when the buffer started, so each emitted span
    /// carries the wmode it was rendered under. A font change flushes the
    /// buffer, so a single buffer never spans mixed writing modes.
    wmode: u8,
    /// Baseline shift as a ratio of font size (`Ts ÷ Tf size`, ISO 32000-1
    /// §9.3.7), captured from the graphics state when the buffer started.
    /// `> 0` superscript, `< 0` subscript, `0.0` on-baseline. Stored as a
    /// ratio so it is text/CTM-scale-independent and directly comparable to a
    /// font-size fraction by the sub/superscript rejoin.
    text_rise: f32,
    /// Text render mode (`Tr`, ISO 32000-1 §9.3.6), captured from the
    /// graphics state when the buffer started. `3`/`7` (invisible — neither
    /// filled nor stroked) means this run has no rendering-correctness
    /// pressure: an OCR-sandwich producer has no visual reason to mirror
    /// already-logical RTL glyph positions the way a *visible*-text
    /// producer would, so the geometric visual/logical detector's ascending-
    /// x signal is uninformative here — see `bidi::apply_rtl_verdict`.
    render_mode: u8,
}

/// Snap a run's display rotation (from the composed `CTM × T_m` rotation block,
/// `θ = atan2(b, a)`) to the nearest of `0 / 90 / 180 / -90` when it is within
/// `SNAP_TOL_DEG` of one, and treat everything within tolerance of `0` as exactly
/// horizontal (`0.0`). Mirrored text (negative matrix determinant) is reported as
/// its raw angle, not snapped, so it is never confused with a clean rotation.
fn snap_run_rotation(combined: &Matrix) -> f32 {
    const SNAP_TOL_DEG: f32 = 5.0;
    let (a, b, c, d) = (combined.a, combined.b, combined.c, combined.d);
    // Pure horizontal/180° fast path: b and c ~ 0 covers both 0° (a,d > 0)
    // and 180° (a,d < 0) — sin(0°) and sin(180°) are both 0, so the
    // off-diagonal terms alone can't tell them apart. Check the sign of
    // `a` (cos(0°)=1, cos(180°)=-1) to disambiguate. ~keep
    if b.abs() < 1e-4 && c.abs() < 1e-4 {
        return if a < 0.0 { 180.0 } else { 0.0 };
    }
    let mut deg = b.atan2(a).to_degrees();
    while deg > 180.0 {
        deg -= 360.0;
    }
    while deg <= -180.0 {
        deg += 360.0;
    }
    // Mirror (det < 0): leave the raw angle; the reading-order path treats any
    // non-zero rotation as a separate block regardless, and snapping a mirror to
    // a quadrant would misrepresent it. ~keep
    let det = a * d - b * c;
    if det < 0.0 {
        return if deg.abs() < SNAP_TOL_DEG { 0.0 } else { deg };
    }
    for &q in &[0.0_f32, 90.0, 180.0, -90.0] {
        if (deg - q).abs() <= SNAP_TOL_DEG {
            return q;
        }
    }
    deg
}

/// Negative determinant of the composed text rendering matrix: the run is
/// mirrored, so `rotation_degrees` alone cannot describe its frame (a mirrored
/// 90° run and a clean 90° run carry the same angle but opposite across-axes).
fn run_is_mirrored(combined: &Matrix) -> bool {
    combined.a * combined.d - combined.b * combined.c < 0.0
}

impl TjBuffer {
    /// Create a new empty buffer with current state.
    fn new(
        state: &crate::content::graphics_state::GraphicsState,
        mcid: Option<u32>,
        cached_font: Option<Arc<FontInfo>>,
    ) -> Self {
        let combined = state.ctm.multiply(&state.text_matrix);
        let effective_font_size = state.font_size * (combined.d * combined.d + combined.b * combined.b).sqrt();
        let user_h_scale = (combined.a * combined.a + combined.c * combined.c).sqrt();
        let font_weight = match &cached_font {
            Some(f) if f.is_bold() => FontWeight::Bold,
            _ => FontWeight::Normal,
        };
        let is_italic = cached_font.as_ref().map(|f| f.is_italic()).unwrap_or(false);
        // Invisible text (Tr 3/7, ISO 32000-1 §9.3.6) is never real visible
        // monospace content — it's an OCR text-sandwich layer sitting under
        // a scanned page image, or deliberately hidden text. Such layers
        // commonly use a synthetic font (conventionally named
        // "GlyphLessFont" by ocrmypdf/Tesseract and similar tools) whose
        // FontDescriptor sets the FixedPitch flag purely for positioning
        // simplicity — the glyphs are never rendered, so "monospace" has no
        // visual meaning to categorize by. Downstream markdown conversion
        // uses `is_monospace` to fence a line/paragraph as a code block; an
        // OCR'd scanned novel's dialogue tripping this on FixedPitch alone
        // fences narrative prose as code. ~keep
        let is_invisible_or_glyphless = state.render_mode == 3
            || state.render_mode == 7
            || cached_font
                .as_ref()
                .is_some_and(|f| f.base_font.to_uppercase().contains("GLYPHLESS"));
        let is_monospace = !is_invisible_or_glyphless
            && cached_font
                .as_ref()
                .is_some_and(|f| f.flags.is_some_and(|flags| flags & 1 != 0) || is_monospace_font(&f.base_font));
        let rotation_degrees = snap_run_rotation(&combined);
        let text_pos = state.text_matrix.transform_point(0.0, 0.0);
        let user_pos = state.ctm.transform_point(text_pos.x, text_pos.y);
        Self {
            unicode: String::new(),
            start_matrix: state.text_matrix,
            font_name: state.font_name.clone(),
            fill_color_rgb: state.fill_color_rgb,
            char_space: state.char_space,
            word_space: state.word_space,
            horizontal_scaling: state.horizontal_scaling,
            mcid,
            accumulated_width: 0.0,
            cached_font,
            effective_font_size,
            font_weight,
            is_italic,
            is_monospace,
            char_widths: Vec::new(),
            user_pos_x: user_pos.x,
            user_pos_y: user_pos.y,
            user_h_scale,
            rotation_degrees,
            mirrored: run_is_mirrored(&combined),
            wmode: state.text_wmode,
            text_rise: if state.font_size > 0.0 {
                state.text_rise / state.font_size
            } else {
                0.0
            },
            render_mode: state.render_mode,
        }
    }

    /// Check if the buffer is empty.
    fn is_empty(&self) -> bool {
        self.unicode.is_empty()
    }

    /// Push a decoded glyph's Unicode text onto `unicode`, dropping control
    /// characters other than tab/LF/CR. Takes the target buffer explicitly
    /// (not `&mut self`) so callers can hold an outstanding borrow of another
    /// `self` field (`cached_font`) across the call. Split out of `append`
    /// (same filter, unchanged) — it was inlined at two call sites and both
    /// nested four levels deep. ~keep
    fn push_filtered_unicode(unicode: &mut String, s: &str) {
        for ch in s.chars() {
            if ch >= '\x20' || ch == '\t' || ch == '\n' || ch == '\r' {
                unicode.push(ch);
            }
        }
    }

    /// Append a text string to the buffer.
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        // PDF spec Section 7.3.4.2: implementation limit of 32,767 bytes per string.
        // Malformed PDFs may exceed this, causing text blowup. ~keep
        let bytes = if bytes.len() > 32_767 { &bytes[..32_767] } else { bytes };

        let font = self.cached_font.as_deref();

        // Fast path: OneByte fonts push chars directly into buffer via lookup table.
        // Avoids String allocation in decode_text_to_unicode (2 allocations per call). ~keep
        if let Some(font) = font
            && font.subtype != "Type0"
        {
            // UTF-8-in-simple-font detection — see long comment in
            // `append_advance_buffer`. Some producers emit UTF-8 byte
            // sequences inside PDF string literals for fonts that only
            // declare a Latin encoding with no ToUnicode CMap. When the
            // entire byte slice is valid UTF-8 whose decoded chars
            // include at least one non-Latin-1 codepoint, treat it as
            // UTF-8 so we recover Cyrillic / Greek / CJK instead of
            // Latin-1 mojibake. ~keep
            if font.to_unicode.is_none() && bytes.len() >= 2 {
                let has_high = bytes.iter().any(|&b| b >= 0x80);
                if has_high
                    && let Ok(decoded) = std::str::from_utf8(bytes)
                    && decoded.chars().any(|c| c as u32 > 0xFF)
                {
                    for ch in decoded.chars() {
                        self.unicode.push(ch);
                    }
                    return Ok(());
                }
            }

            let table = font.get_byte_to_char_table();
            for &byte in bytes {
                let c = table[byte as usize];
                if c != '\0' {
                    self.unicode.push(c);
                } else if let Some(s) = font.char_to_unicode(byte as u32) {
                    if s != "\u{FFFD}" || preserve_unmapped_glyphs() {
                        Self::push_filtered_unicode(&mut self.unicode, &s);
                    }
                } else {
                    let fb = fallback_extraction_char_to_unicode(font, byte as u32);
                    if fb != "\u{FFFD}" || preserve_unmapped_glyphs() {
                        Self::push_filtered_unicode(&mut self.unicode, &fb);
                    }
                }
            }
            return Ok(());
        }

        let unicode_text = decode_text_to_unicode(
            bytes,
            font,
            DecodePolicy {
                preserve_unmapped: preserve_unmapped_glyphs(),
                decompose_ligatures: false,
                question_mark_for_invalid: true,
            },
            None,
        );
        self.unicode.push_str(&unicode_text);

        Ok(())
    }
}

/// Artifact type classification per PDF Spec Section 14.8.2.2
///
/// Artifacts are content that is not part of the document's logical structure,
/// such as headers, footers, page numbers, and decorative elements.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum ArtifactType {
    /// Pagination artifacts (headers, footers, page numbers)
    Pagination(PaginationSubtype),
    /// Layout artifacts (ruled lines, backgrounds, borders)
    Layout,
    /// Page artifacts (full-page backgrounds, watermarks)
    Page,
    /// Background graphics or decorations
    Background,
}

/// Pagination artifact subtypes per PDF Spec Section 14.8.2.2.1
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum PaginationSubtype {
    /// Page header content
    Header,
    /// Page footer content
    Footer,
    /// Watermark overlay
    Watermark,
    /// Page number
    PageNumber,
    /// Other pagination element
    Other,
}

/// Context for marked content sequences (per PDF Spec Section 14.6)
///
/// Tracks nested marked content tags to implement artifact filtering.
/// When content is marked as `/Artifact`, it should be excluded from text extraction.
#[derive(Debug, Clone, Default)]
struct MarkedContentContext {
    /// Never read back — classification (`is_artifact`, `is_placed_pdf`, OCG checks) is
    /// derived from the BMC/BDC operator's local `tag` binding, not this stored copy.
    #[allow(dead_code)]
    tag: String,
    is_artifact: bool,
    /// Artifact type classification for filtered content (PDF Spec Section 14.8.2.2)
    artifact_type: Option<ArtifactType>,
    /// ActualText for marked content (PDF Spec Section 14.9.4)
    /// Used to replace extracted text with correct representation
    /// e.g., ligatures (fi, fl, ffi, ffl), decorated glyphs
    actual_text: Option<String>,
    /// True once an ActualText replacement has been emitted from this
    /// MC scope. Per ISO 32000-1:2008 §14.9.4 the `/ActualText` of a
    /// marked-content sequence is the replacement for the ENTIRE
    /// sequence — even if it contains multiple `Tj` / `TJ` operators
    /// the replacement is emitted ONCE. The first Tj inside a scope
    /// flips this flag; subsequent Tj operators see it and skip the
    /// replacement path.
    actual_text_emitted: bool,
    /// Expansion text for abbreviations (PDF Spec Section 14.9.5)
    /// The /E entry provides the expansion of an abbreviation or acronym.
    /// e.g., "PDF" might expand to "Portable Document Format"
    ///
    /// Only ever read back from unit tests that inspect the stored context directly;
    /// production code never consumes it after storing it here.
    #[allow(dead_code)]
    expansion: Option<String>,
    /// Whether this marked content context is an excluded Optional Content Group (layer).
    ///
    /// Set when tag is "OC" and the OCG /Name matches one of the excluded layers.
    is_excluded_layer: bool,
    /// Whether this marked content context is an InDesign "placed PDF" figure.
    ///
    /// Set when the tag is `/PlacedPDF` — an Adobe InDesign-specific
    /// marked-content tag that wraps an imported/placed PDF rendered AS a
    /// figure (always nested inside a `/Figure` structure element). Its text
    /// content is the placed artwork's own glyphs (e.g. a draft galley of the
    /// manuscript with line numbers), NOT the document's logical text — the
    /// authoritative copy is re-typeset outside the placed region. Treating it
    /// as a figure (suppressing its text) matches what pdftotext/PyMuPDF do
    /// and removes duplicated / mojibake overlay text. See `is_content_suppressed`.
    is_placed_pdf: bool,
    /// MCID declared by this BDC (only BDC; BMC carries no /MCID).
    ///
    /// Stored here so EMC can restore the outer scope's MCID instead
    /// of blanking `current_mcid` unconditionally. A `Tj` issued
    /// AFTER an inner EMC must still attribute to its enclosing
    /// MCID-bearing scope (the PDF spec specifies marked-content
    /// nesting at §14.6).
    own_mcid: Option<u32>,
}

/// Text extractor that processes content streams.
///
/// This structure maintains the graphics state stack and font information
/// while processing operators to extract positioned text.
///
/// The extractor can work in two modes:
/// - **Span mode** (default): Extracts complete text strings as PDF provides them (PDF spec compliant)
/// - **Character mode**: Extracts individual characters (for special use cases)
#[derive(Debug)]
pub struct TextExtractor<'doc> {
    /// Graphics state stack for handling q/Q operators
    state_stack: GraphicsStateStack,
    /// Loaded fonts (name -> FontInfo). Arc-wrapped to avoid deep cloning across pages.
    fonts: HashMap<String, Arc<FontInfo>>,
    /// Extracted text spans (complete strings from Tj/TJ operators)
    spans: Vec<TextSpan>,
    /// Extracted characters (for backward compatibility)
    chars: Vec<TextChar>,
    /// Resources dictionary (for accessing XObjects and fonts)
    resources: Option<Object>,
    /// Reference to the document (for loading XObjects)
    document: Option<&'doc crate::document::PdfDocument>,
    /// Set of processed XObject references to avoid duplicates.
    /// Key is `(ObjectRef, ctm_key)` where `ctm_key` is the CTM at the time of
    /// the `Do` operator call, encoded as 6 millipoint-rounded i64 values.
    /// Using the CTM as part of the key allows the same Form XObject to be
    /// processed multiple times when invoked with different transformation
    /// matrices (e.g., the same XObject stamped at different positions on a page),
    /// while still preventing infinite recursion (same ref + same CTM).
    processed_xobjects: HashSet<(ObjectRef, [i64; 6])>,
    /// Cached XObject name → ObjectRef mapping for current resources context.
    /// Avoids expensive repeated resolution of the resources/XObject dict chain.
    cached_xobject_refs: HashMap<String, Option<ObjectRef>>,
    /// Current XObject recursion depth (0 = page level)
    xobject_depth: u32,
    /// Number of XObjects decoded on this page (for budget limiting)
    xobject_decode_count: u32,
    /// Configuration for text extraction heuristics
    config: TextExtractionConfig,
    /// Configuration for span merging behavior
    merging_config: SpanMergingConfig,
    /// Current marked content ID (for Tagged PDFs)
    ///
    /// Tracks the MCID of the currently active marked content sequence.
    /// Used to associate extracted text with structure tree elements.
    current_mcid: Option<u32>,
    /// Set of MCIDs whose BDC carried inline `/ActualText` on this
    /// page.
    ///
    /// Populated by the BDC handler whenever it observes
    /// `/ActualText` on the properties dictionary. The struct-tree-
    /// scope ActualText applier (in `document.rs`) uses this set to
    /// honour MC-scope-wins precedence: an ancestor StructElem's
    /// `/ActualText` must NOT override an MCID whose in-stream
    /// /ActualText has already been applied at extraction time
    /// (ISO 32000-1:2008 §14.6, §14.9.4).
    mc_actualtext_mcids: HashSet<u32>,
    /// Stack of marked content contexts (per PDF Spec Section 14.6)
    ///
    /// Tracks nested marked content tags to enable artifact filtering.
    /// When content is marked as `/Artifact`, it should be excluded from text extraction.
    marked_content_stack: Vec<MarkedContentContext>,
    /// True once a `/ReversedChars` marked-content sequence (ISO 32000-1
    /// §14.8.2.3.3) has been seen on this page. Such producers draw RTL glyphs
    /// individually with explicit positioning and mark real word boundaries with
    /// explicit space glyphs — so oxide must NOT additionally insert geometric
    /// word spaces between cursively-adjacent Arabic letters (which would shatter
    /// words, e.g. `إسبريسو` → `إس بر يسو`).
    saw_reversed_chars: bool,
    /// Whether we're currently inside an /Artifact marked content context
    ///
    /// Per PDF Spec Section 14.6, artifact content should be excluded from text extraction.
    /// This flag is true when any ancestor in the marked_content_stack has is_artifact=true.
    inside_artifact: bool,
    /// Layer names (Optional Content Groups) to exclude from extraction.
    ///
    /// When a BDC operator with tag "OC" references an OCG whose /Name matches
    /// one of these entries, all content within that marked content scope is suppressed.
    excluded_layers: HashSet<String>,
    /// Whether we're currently inside an excluded OCG layer.
    ///
    /// True when any ancestor in the marked_content_stack has is_excluded_layer=true.
    inside_excluded_layer: bool,
    /// Whether we're currently inside an InDesign `/PlacedPDF` figure region.
    ///
    /// True when any ancestor in the marked_content_stack has is_placed_pdf=true.
    /// Text inside a placed-PDF figure is the placed artwork's own glyphs and is
    /// suppressed (it is a figure, not logical text). See `MarkedContentContext::is_placed_pdf`.
    inside_placed_pdf: bool,
    /// When true, `/PlacedPDF` text is KEPT instead of suppressed for this page.
    ///
    /// The placed-PDF suppression assumes the placed region is a *decorative
    /// figure overlay* whose glyphs duplicate logical text that lives OUTSIDE it
    /// (the PMC8100493 draft-galley case). But some publishers (e.g. MATEC Web of
    /// Conferences) place the ENTIRE article body inside a single `/PlacedPDF`
    /// region, leaving almost nothing outside — there the placed text IS the
    /// page's logical content and suppressing it drops the whole page. Set by a
    /// cheap page-content-stream pre-scan (`placed_pdf_text_dominates`) that
    /// flips this on only when the placed text dominates and the non-placed text
    /// is negligible. pymupdf/pdftotext likewise extract the body in that case.
    placed_pdf_keep: bool,
    /// Ink / separation names to exclude from extraction.
    ///
    /// When a `cs` operator sets a Separation or DeviceN color space whose ink name(s)
    /// match one of these entries, subsequent text is suppressed until the color space changes.
    excluded_inks: HashSet<String>,
    /// Whether the current fill color space is an excluded ink.
    ///
    /// Set when SetFillColorSpace resolves to a Separation or DeviceN color space
    /// whose ink name(s) intersect with `excluded_inks`.
    inside_excluded_ink: bool,
    /// Extraction mode: true for spans, false for characters
    extract_spans: bool,
    /// Buffer for accumulating consecutive Tj operators into single spans
    ///
    /// Per PDF Spec ISO 32000-1:2008 Section 9.4.4 NOTE 6, text strings should
    /// be as long as possible. This buffer accumulates consecutive Tj operators
    /// until a positioning command or state change is encountered.
    tj_span_buffer: Option<TjBuffer>,
    /// Sequence counter for TextSpan ordering
    ///
    /// Used as a tie-breaker when sorting spans by Y-coordinate. Ensures
    /// that spans with identical Y-coordinates maintain extraction order.
    span_sequence_counter: usize,
    /// History of TJ array offsets for statistical analysis
    ///
    /// Tracks TJ offset values to detect justified vs. normal text through
    /// statistical distribution analysis (coefficient of variation).
    /// Used to dynamically adjust spacing thresholds per ISO 32000-1:2008 Section 9.4.4.
    tj_offset_history: Vec<f32>,
    /// Running sum / sum-of-squares so `analyze_tj_distribution` is O(1) rather
    /// than re-scanning the offset history (called once per TJ offset → O(n²)
    /// per page). `tj_stats_len` is the history length they cover; if the
    /// history is replaced wholesale, `analyze` recomputes once. f64 for precision.
    tj_sum: f64,
    tj_sum_sq: f64,
    tj_stats_len: usize,
    /// Character-level tracking for word boundary detection
    ///
    /// Collects CharacterInfo for each character during TJ array processing.
    /// This provides character-level positioning, width, and TJ offset data
    /// to WordBoundaryDetector for primary word boundary detection.
    /// Per ISO 32000-1:2008 Section 9.4.4, character-level analysis improves accuracy.
    tj_character_array: Vec<CharacterInfo>,
    /// Current X position in text space for character tracking
    ///
    /// Updated as each character in a TJ array is processed. Used to calculate
    /// x_position for CharacterInfo entries (not used after character collection).
    current_x_position: f32,
    /// Word boundary detection mode
    ///
    /// Controls whether WordBoundaryDetector is used as:
    /// - Tiebreaker: Only when TJ and geometric signals conflict (default)
    /// - Primary: Before creating TextSpans from tj_character_array
    word_boundary_mode: WordBoundaryMode,
    /// Cached current font (updated on Tf). Avoids per-Tj HashMap lookup
    /// in advance_position_for_string.
    cached_current_font: Option<Arc<FontInfo>>,
    /// Extraction-only simple-font width tables, keyed by stable `Arc` identity.
    /// Each embedded font is parsed at most once per extractor. ~keep
    extraction_width_tables: HashMap<usize, (Arc<FontInfo>, Arc<[f32; 256]>)>,
    /// Width table paired with `cached_current_font` for the text hot path. ~keep
    cached_extraction_widths: Option<Arc<[f32; 256]>>,
    /// Stack of MCID content-stream scopes (ISO 32000-1:2008 §14.7.4.3).
    ///
    /// Bottom of the stack is the page's own content-stream scope
    /// (`McidScope::Page(page_index)`). Each entry into a Form XObject
    /// via `Do` pushes a `McidScope::Form(form_ref)`; the matching
    /// pop restores the outer scope. The top of the stack stamps every
    /// `TextSpan` emitted while it is active. Tiling-Pattern walks are
    /// not currently traversed by the extractor (patterns rasterize
    /// independently); the spec-strict three-variant scope still
    /// covers `Pattern(_)` in the data model so future pattern-content
    /// walks can populate it.
    mcid_scope_stack: Vec<crate::structure::McidScope>,
}

/// Tracing target for every event raised under `extractors::text`, pinned to the
/// parent module path.
///
/// Targets are public API in this workspace and semver-relevant. Without this, each
/// child module would inherit its own `module_path!()` and every call site would
/// silently move from `xberg_native_pdf::extractors::text` to
/// `…::text::<child>`, breaking consumer `EnvFilter`s that name the old target. ~keep
pub(super) const LOG_TARGET: &str = module_path!();

impl<'doc> TextExtractor<'doc> {
    /// Fraction of a glyph's advance width considered "overlap" for
    /// duplicate detection. Used by both `deduplicate_overlapping_chars`
    /// and `deduplicate_overlapping_spans`.
    ///
    /// 0.30 comfortably catches real render-pass duplicates
    /// (stroke+fill, bold shadow, outline+fill) which sit well under
    /// 5 % of one advance apart, while staying below typical heaviest
    /// kerning (≤ 20 % of advance) so legitimate narrow-glyph
    /// neighbours (`ll`, `rr`, `II`, `ii`) are preserved.
    const DEDUP_OVERLAP_RATIO: f32 = 0.30;

    /// Absolute cap on the overlap window (in PDF points).
    ///
    /// Preserves the earlier ratio-only behaviour for pathologically
    /// oversized advance values (drop-caps, large display text) where
    /// 30 % of the advance would swallow legitimate neighbours.
    const DEDUP_OVERLAP_CAP_PT: f32 = 2.0;

    // ========================================================================
    // Debug/profiling helpers — exposed for examples/debug_katalog.rs
    // ======================================================================== ~keep

    /// Maximum XObject recursion depth. Text content in PDFs is rarely nested
    /// more than 2-3 levels. Deep nesting typically indicates complex vector
    /// graphics (charts, plots) with no text content.
    const MAX_XOBJECT_DEPTH: u32 = 10;

    const MAX_XOBJECT_DECODES: u32 = 500;
}

impl<'doc> Default for TextExtractor<'doc> {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to determine if a space should be inserted between two text spans
// based on character transition heuristics.
//
// This complements gap-based space detection by catching cases where the geometric
// gap is small but a space is semantically needed based on character patterns.
//
// # Detected Patterns
//
// - **CamelCase transitions**: `thenThe` → `then The` (lowercase followed by uppercase)
// - **Number-letter transitions**: `Figure1` → `Figure 1` (digit followed by letter)
// - **Letter-number transitions**: `page3` → `page 3` (letter followed by digit)
//
// # Arguments
//
// * `current_text` - The text of the current span
// * `next_text` - The text of the next span to be merged
//
// # Returns
//
// `true` if a space should be inserted based on character transitions,
// `false` if no space is needed
//
// # Preserves
//
// - Acronyms like "HTML", "PDF", "API" (all uppercase)
// - Normal word boundaries (already handled by gap detection)
// - Intentional concatenations within words
// DELETED: should_insert_space_heuristic()
// Character pattern heuristics (CamelCase detection, number-letter transitions)
// are not defined in ISO 32000-1:2008 PDF spec. Per spec-compliance refactoring,
// only spec-defined signals (TJ offsets, geometric gaps, boundary whitespace)
// are used for space insertion decisions. ~keep

#[cfg(test)]
mod tests;

#[cfg(test)]
mod config_tests;

mod adaptive_spacing;
mod advance;
mod clustering;
mod marked_content;
mod operators;
mod run;
mod setup;
mod span_merging;
mod span_ordering;
mod tj_arrays;
mod xobjects;
