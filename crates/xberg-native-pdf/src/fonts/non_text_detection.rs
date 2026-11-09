//! Non-text content detection for PDF extraction.
//!
//! This module detects whether character sequences represent non-text content
//! (such as embedded figures, diagrams, or other visual elements) rather than
//! actual text. This helps avoid extracting garbled characters from figures
//! that have high percentages of unmapped glyphs.
//!
//! # Phase 3: Enhanced ToUnicode Fallback
//!
//! Phase 3 improves extraction quality by:
//! 1. Detecting non-text content sequences
//! 2. Computing character mapping confidence scores
//! 3. Marking or skipping figures/diagrams in output
//! 4. Preserving text extraction accuracy

use crate::layout::TextSpan;

/// Confidence score for character-to-Unicode mapping (0.0 to 1.0).
///
/// Represents how confident we are that a given character code
/// maps to valid Unicode text rather than being garbage/diagram content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterConfidence {
    /// Overall confidence score (0.0 = certain garbage, 1.0 = certain text)
    pub score: f32,
    /// Reason for the confidence score
    pub reason: ConfidenceReason,
}

/// Reason why a character has a certain confidence score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceReason {
    /// Character has explicit ToUnicode mapping
    MappedByToUnicode,
    /// Character in standard encoding (ASCII, Latin-1, etc.)
    StandardEncoding,
    /// Character from font's built-in encoding
    FontEncoding,
    /// Fallback mapping using font name hints (Symbol, Wingdings, etc.)
    FontHintFallback,
    /// Unmapped character with no mapping available
    Unmapped,
    /// Character appears in suspicious context (likely diagram/figure)
    SuspiciousContext,
}

impl CharacterConfidence {
    /// Create a confidence score for a mapped character.
    pub fn mapped() -> Self {
        Self {
            score: 0.95,
            reason: ConfidenceReason::MappedByToUnicode,
        }
    }

    /// Create a confidence score for a standard encoding character.
    pub fn standard_encoding() -> Self {
        Self {
            score: 0.9,
            reason: ConfidenceReason::StandardEncoding,
        }
    }

    /// Create a confidence score for an unmapped character.
    pub fn unmapped() -> Self {
        Self {
            score: 0.3,
            reason: ConfidenceReason::Unmapped,
        }
    }

    /// Create a confidence score for a suspicious context.
    pub fn suspicious(score: f32) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            reason: ConfidenceReason::SuspiciousContext,
        }
    }
}

/// Statistics for non-text content detection.
#[derive(Debug, Clone, Default)]
pub struct NonTextStats {
    /// Total characters analyzed
    pub total_chars: usize,
    /// Number of mapped characters
    pub mapped_chars: usize,
    /// Number of unmapped characters
    pub unmapped_chars: usize,
    /// Average confidence score
    pub avg_confidence: f32,
    /// Percentage of unmapped characters (0.0 to 1.0)
    pub unmapped_ratio: f32,
    /// Likely non-text content flag
    pub likely_non_text: bool,
}

/// Detector for non-text content in character sequences.
#[derive(Debug, Clone)]
pub struct NonTextDetector {
    /// Threshold for unmapped ratio to classify as non-text (default: 0.5)
    pub unmapped_threshold: f32,
    /// Threshold for confidence score to classify as non-text (default: 0.4)
    pub confidence_threshold: f32,
    /// Minimum sequence length to evaluate
    pub min_sequence_length: usize,
    /// Span-level non-ASCII ratio above which a span is treated as non-text
    /// content and dropped by `mark_non_text_spans` (default: 0.3). Set to
    /// `>= 1.0` to disable the non-ASCII drop entirely — appropriate for CJK,
    /// accented-Latin, or currency/math-heavy documents where a high
    /// non-ASCII ratio is normal content, not noise (PDX-7, liteparse report).
    pub non_ascii_drop_threshold: f32,
    /// Whether `mark_non_text_spans` drops spans containing characters in the
    /// "suspicious" Unicode blocks (misc symbols, dingbats, emoji, math
    /// operators). Default `true` preserves historical behaviour; set `false`
    /// to keep symbol/math glyphs that the text path retains (PDX-7).
    pub drop_suspicious_unicode: bool,
}

impl Default for NonTextDetector {
    fn default() -> Self {
        Self {
            unmapped_threshold: 0.5,
            confidence_threshold: 0.4,
            min_sequence_length: 10,
            // Defaults preserve the historical span-drop behaviour; callers
            // that need symbol/CJK/accented content can relax these. ~keep
            non_ascii_drop_threshold: 0.3,
            drop_suspicious_unicode: true,
        }
    }
}

impl NonTextDetector {
    /// Create a new non-text detector with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a character sequence for non-text content indicators.
    ///
    /// # Arguments
    ///
    /// * `text` - The extracted text (may contain unmapped characters)
    /// * `confidences` - Per-character confidence scores
    /// * `font_name` - Name of the font (for heuristics)
    ///
    /// # Returns
    ///
    /// Statistics about the sequence and whether it's likely non-text content.
    pub fn analyze_sequence(&self, text: &str, confidences: &[CharacterConfidence], font_name: &str) -> NonTextStats {
        if text.len() < self.min_sequence_length {
            return NonTextStats::default();
        }

        let total_chars = text.len();
        let mapped_chars = confidences
            .iter()
            .filter(|c| c.reason != ConfidenceReason::Unmapped)
            .count();
        let unmapped_chars = total_chars - mapped_chars;
        let unmapped_ratio = unmapped_chars as f32 / total_chars as f32;

        let avg_confidence = if !confidences.is_empty() {
            confidences.iter().map(|c| c.score).sum::<f32>() / confidences.len() as f32
        } else {
            0.0
        };

        let likely_non_text = unmapped_ratio > self.unmapped_threshold
            || avg_confidence < self.confidence_threshold
            || self.is_diagram_font(font_name);

        NonTextStats {
            total_chars,
            mapped_chars,
            unmapped_chars,
            avg_confidence,
            unmapped_ratio,
            likely_non_text,
        }
    }

    /// Check if a font name suggests diagram/symbol content.
    fn is_diagram_font(&self, font_name: &str) -> bool {
        let name_lower = font_name.to_lowercase();
        [
            "symbol",
            "wingdings",
            "webdings",
            "zapf dingbats",
            "dingbats",
            "mathematical alphanumeric",
        ]
        .iter()
        .any(|&pattern| name_lower.contains(pattern))
    }

    /// Detect and mark sequences as non-text content.
    ///
    /// This method analyzes spans and marks those that likely represent
    /// figures, diagrams, or other non-text content.
    pub fn mark_non_text_spans(&self, spans: &[TextSpan]) -> Vec<SpanClassification> {
        spans
            .iter()
            .enumerate()
            .map(|(idx, span)| {
                let non_ascii_ratio =
                    span.text.chars().filter(|c| !c.is_ascii()).count() as f32 / span.text.len().max(1) as f32;

                let non_ascii_drop = non_ascii_ratio > self.non_ascii_drop_threshold;
                let suspicious_drop = self.drop_suspicious_unicode && has_suspicious_patterns(&span.text);
                let is_likely_non_text = non_ascii_drop || suspicious_drop;

                SpanClassification {
                    span_index: idx,
                    span: span.clone(),
                    is_non_text: is_likely_non_text,
                    confidence: if is_likely_non_text { 0.6 } else { 0.9 },
                }
            })
            .collect()
    }
}

/// Classification of a text span.
#[derive(Debug, Clone)]
pub struct SpanClassification {
    /// Index of the span in original array
    pub span_index: usize,
    /// The text span itself
    pub span: TextSpan,
    /// Whether this span likely contains non-text content
    pub is_non_text: bool,
    /// Confidence in the classification (0.0 to 1.0)
    pub confidence: f32,
}

/// Check if text contains suspicious patterns indicating non-text content.
fn has_suspicious_patterns(text: &str) -> bool {
    let special_char_count = text
        .chars()
        .filter(|c| {
            let code = *c as u32;
            matches!(
                code,
                0x2600..=0x27BF |   // Miscellaneous Symbols and Dingbats ~keep
                0x1F300..=0x1F9FF | // Emoticons and pictographs ~keep
                0x2200..=0x22FF |   // Mathematical Operators ~keep
                0x2A00..=0x2AFF |   // Supplemental Mathematical Operators ~keep
                0x0080..=0x009F     // C1 Control Codes (often unmapped) ~keep
            )
        })
        .count();

    let special_ratio = special_char_count as f32 / text.len().max(1) as f32;

    // If >40% of characters are from special Unicode blocks, likely diagram ~keep
    special_ratio > 0.4
}

/// Compute mapping confidence for a character sequence.
///
/// Analyzes how many characters in a sequence have valid Unicode mappings
/// versus how many are unmapped or garbled.
pub fn compute_sequence_confidence(text: &str, mapped_count: usize, font_name: &str) -> CharacterConfidence {
    if text.is_empty() {
        return CharacterConfidence::unmapped();
    }

    let total = text.len();
    let mapped_ratio = mapped_count as f32 / total as f32;

    let score: f32 = if mapped_ratio > 0.9 {
        0.85
    } else if mapped_ratio > 0.75 {
        0.7
    } else if mapped_ratio > 0.5 {
        0.5
    } else {
        0.2
    };

    CharacterConfidence {
        score: score.clamp(0.0_f32, 1.0_f32),
        reason: if is_likely_diagram_font(font_name) {
            ConfidenceReason::SuspiciousContext
        } else {
            ConfidenceReason::Unmapped
        },
    }
}

/// Check if a font name suggests symbol/diagram content.
fn is_likely_diagram_font(font_name: &str) -> bool {
    let name_lower = font_name.to_lowercase();
    name_lower.contains("symbol")
        || name_lower.contains("wingdings")
        || name_lower.contains("webdings")
        || name_lower.contains("dingbats")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_confidence_mapped() {
        let conf = CharacterConfidence::mapped();
        assert_eq!(conf.reason, ConfidenceReason::MappedByToUnicode);
        assert!(conf.score > 0.9);
    }

    #[test]
    fn test_character_confidence_unmapped() {
        let conf = CharacterConfidence::unmapped();
        assert_eq!(conf.reason, ConfidenceReason::Unmapped);
        assert!(conf.score < 0.5);
    }

    #[test]
    fn test_non_text_detector_high_unmapped_ratio() {
        let detector = NonTextDetector::default();

        let confidences = vec![
            CharacterConfidence::unmapped(),
            CharacterConfidence::unmapped(),
            CharacterConfidence::unmapped(),
            CharacterConfidence::mapped(),
            CharacterConfidence::mapped(),
            CharacterConfidence::unmapped(),
            CharacterConfidence::unmapped(),
            CharacterConfidence::unmapped(),
            CharacterConfidence::unmapped(),
            CharacterConfidence::unmapped(),
        ];

        let stats = detector.analyze_sequence("äöüäöüäöüX", &confidences, "Helvetica");
        assert!(stats.likely_non_text);
    }

    #[test]
    fn test_non_text_detector_symbol_font() {
        let detector = NonTextDetector::default();
        let confidences = vec![CharacterConfidence::mapped(); 10];

        let stats = detector.analyze_sequence("test content 123", &confidences, "Symbol");
        assert!(stats.likely_non_text);
    }

    #[test]
    fn test_non_text_detector_normal_text() {
        let detector = NonTextDetector::default();
        let confidences = vec![CharacterConfidence::mapped(); 10];

        let stats = detector.analyze_sequence("hello world test", &confidences, "Arial");
        assert!(!stats.likely_non_text);
    }

    #[test]
    fn test_suspicious_patterns() {
        assert!(!has_suspicious_patterns("The quick brown fox"));

        assert!(!has_suspicious_patterns("Café résumé naïve"));
    }

    #[test]
    fn test_sequence_confidence_high_mapped() {
        let conf = compute_sequence_confidence("Hello World", 11, "Arial");
        assert!(conf.score > 0.7);
    }

    #[test]
    fn test_sequence_confidence_low_mapped() {
        let conf = compute_sequence_confidence("☺♦♠♥♣", 1, "Arial");
        assert!(conf.score < 0.5);
    }

    // PDX-7 (liteparse report): the span-drop heuristics in mark_non_text_spans
    // must be configurable so symbol/CJK/accented content can be preserved.
    // Defaults keep historical behaviour; relaxing the knobs keeps the content. ~keep
    fn span_with_text(text: &str) -> crate::layout::TextSpan {
        use crate::geometry::Rect;
        use crate::layout::{Color, FontWeight, TextSpan};
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(0.0, 0.0, 10.0, 12.0),
            font_name: "Helvetica".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    #[test]
    fn test_non_text_drop_is_configurable() {
        // --- non-ASCII-ratio knob ---
        // Pure CJK is ~33% non-ASCII (1 char / 3 UTF-8 bytes), over the 0.3
        // default — real text the heuristic wrongly drops. CJK is not in the
        // "suspicious" Unicode blocks, so this isolates the non-ASCII gate. ~keep
        let cjk = span_with_text("日本語のテキスト処理");
        assert!(
            NonTextDetector::default().mark_non_text_spans(std::slice::from_ref(&cjk))[0].is_non_text,
            "default: CJK dropped by the non-ASCII ratio gate"
        );
        let na_off = NonTextDetector {
            non_ascii_drop_threshold: 1.0,
            ..NonTextDetector::default()
        };
        assert!(
            !na_off.mark_non_text_spans(&[cjk])[0].is_non_text,
            "PDX-7: CJK preserved when the non-ASCII drop is disabled"
        );

        // --- suspicious-Unicode knob ---
        // C1 control codes (0x0080-0x009F, 2 bytes each) push special_ratio
        // over has_suspicious_patterns' 0.4 cutoff. Disable the non-ASCII gate
        // so we isolate the suspicious-Unicode gate. ~keep
        let ctrl = span_with_text("\u{0080}\u{0081}\u{0082}");
        let susp_on = NonTextDetector {
            non_ascii_drop_threshold: 1.0,
            drop_suspicious_unicode: true,
            ..NonTextDetector::default()
        };
        assert!(
            susp_on.mark_non_text_spans(std::slice::from_ref(&ctrl))[0].is_non_text,
            "suspicious-Unicode gate drops the span when enabled"
        );
        let susp_off = NonTextDetector {
            non_ascii_drop_threshold: 1.0,
            drop_suspicious_unicode: false,
            ..NonTextDetector::default()
        };
        assert!(
            !susp_off.mark_non_text_spans(&[ctrl])[0].is_non_text,
            "PDX-7: content preserved when the suspicious-Unicode drop is disabled"
        );
    }
}
