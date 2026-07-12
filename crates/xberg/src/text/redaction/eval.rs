//! PII detection accuracy evaluation harness.
//!
//! Scores detector output (`Vec<PatternMatch>`) against a hand-labeled
//! ground-truth corpus using overlap-span matching: a detection counts as a
//! true positive if it overlaps a ground-truth span of the same category by at
//! least one byte, not on an exact-offset match — regex/NER span boundaries can
//! differ by a character or two on real text, so exact-offset matching would
//! produce false negatives for perfectly good detections.
//!
//! Design mirrors `anno-rag`'s `crates/anno-rag/src/pii_eval.rs` (same
//! overlap-span approach), adapted to xberg's `PatternMatch` detector output
//! and `PiiCategory` keys.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::text::redaction::patterns::PatternMatch;

/// A hand-labeled ground-truth PII span in a corpus document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrueSpan {
    pub start: usize,
    pub end: usize,
    pub category: String,
}

/// Per-category precision/recall/F1 derived by [`score`].
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryScore {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

/// Overlap-span matching of `detected` against `truth`, returning a score per
/// category (keyed by `PiiCategory`'s `Debug` string, e.g. `"Email"`).
///
/// A detection is a true positive if it overlaps any truth span of the same
/// category by at least one byte (`d.start < t.end && d.end > t.start`). A
/// truth span is a false negative if no same-category detection overlaps it.
pub fn score(detected: &[PatternMatch], truth: &[TrueSpan]) -> BTreeMap<String, CategoryScore> {
    let mut categories: BTreeSet<String> = BTreeSet::new();
    for m in detected {
        categories.insert(format!("{:?}", m.category));
    }
    for t in truth {
        categories.insert(t.category.clone());
    }

    let mut out = BTreeMap::new();
    for cat in &categories {
        let dets: Vec<&PatternMatch> = detected
            .iter()
            .filter(|m| format!("{:?}", m.category) == *cat)
            .collect();
        let truths: Vec<&TrueSpan> = truth.iter().filter(|t| t.category == *cat).collect();

        let mut tp = 0usize;
        let mut fp = 0usize;
        for d in &dets {
            let overlaps = truths.iter().any(|t| d.start < t.end && d.end > t.start);
            if overlaps {
                tp += 1;
            } else {
                fp += 1;
            }
        }

        let mut fn_ = 0usize;
        for t in &truths {
            let covered = dets.iter().any(|d| d.start < t.end && d.end > t.start);
            if !covered {
                fn_ += 1;
            }
        }

        // `tp + fp == 0` only happens when this category has zero detections
        // (it's in `categories` because it appears in `truth` only) — every
        // truth span for it is necessarily a false negative, so reporting
        // precision as 1.0 ("vacuously perfect") would misrepresent a
        // category the detector never even attempted; 0.0 makes the failure
        // visible instead of hiding it behind a perfect-looking number.
        // Symmetric reasoning for `tp + fn_ == 0` (detected-only category,
        // no ground truth for it — everything detected is a false positive).
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 {
            tp as f64 / (tp + fn_) as f64
        } else {
            0.0
        };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        out.insert(
            cat.clone(),
            CategoryScore {
                true_positives: tp,
                false_positives: fp,
                false_negatives: fn_,
                precision,
                recall,
                f1,
            },
        );
    }
    out
}

/// A single ground-truth span entry as written in `annotations.toml`.
#[derive(Debug, serde::Deserialize)]
struct SpanDef {
    file: String,
    start: usize,
    end: usize,
    category: String,
}

#[derive(Debug, serde::Deserialize)]
struct Annotations {
    span: Vec<SpanDef>,
}

/// Load a labeled corpus from `dir`: reads `annotations.toml` (one
/// `[[span]]` array-table per ground-truth span, each referencing a file in
/// `dir`) and returns, per document, its text plus its ground-truth spans.
///
/// Returns an error if `annotations.toml` is missing/malformed or a referenced
/// file cannot be read.
pub fn load_corpus(dir: &Path) -> std::io::Result<Vec<(String, Vec<TrueSpan>)>> {
    let ann_path = dir.join("annotations.toml");
    let raw = std::fs::read_to_string(&ann_path)?;
    let ann: Annotations =
        toml::from_str(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let mut by_file: BTreeMap<String, Vec<TrueSpan>> = BTreeMap::new();
    for s in ann.span {
        by_file.entry(s.file.clone()).or_default().push(TrueSpan {
            start: s.start,
            end: s.end,
            category: s.category,
        });
    }

    let mut out = Vec::new();
    for (file, spans) in by_file {
        let text = std::fs::read_to_string(dir.join(&file))?;
        out.push((text, spans));
    }
    Ok(out)
}

/// Convenience: run the regex-only detector (`scan_text` with no category
/// filter) over `text`, apply the same post-detection validators `redact()`
/// runs (IBAN checksum, Luhn), and score the validated output against
/// `truth`. Exposed for callers that don't want to wire `scan_text` +
/// `apply_validators` themselves.
///
/// Deliberately scores the *validated* output, not raw `scan_text` — Task 1's
/// checksum validators exist specifically to cut credit-card/IBAN false
/// positives, so an eval harness that skipped them couldn't detect precision
/// regressions in the path `redact()` actually uses.
pub fn evaluate_text(text: &str, truth: &[TrueSpan]) -> BTreeMap<String, CategoryScore> {
    use crate::text::redaction::validators::{EntityValidator, apply_validators};

    let validators: Vec<Box<dyn EntityValidator>> = vec![
        Box::new(crate::text::redaction::validators::iban::IbanChecksumValidator),
        Box::new(crate::text::redaction::validators::luhn::LuhnValidator),
    ];
    let raw = crate::text::redaction::patterns::scan_text(text, &[]);
    let (detected, _rejections) = apply_validators(raw, text, &validators);
    score(&detected, truth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::redaction::PiiCategory;

    #[test]
    fn score_recognizes_true_positives_and_false_negatives() {
        let detected = vec![
            PatternMatch {
                start: 0,
                end: 5,
                category: PiiCategory::Email,
                text: "a@b.c".into(),
            },
            // Pure false positive: does not overlap any truth span.
            PatternMatch {
                start: 10,
                end: 20,
                category: PiiCategory::Phone,
                text: "5551234567".into(),
            },
        ];
        let truth = vec![
            TrueSpan {
                start: 0,
                end: 5,
                category: "Email".into(),
            },
            // Missed by the detector entirely -> false negative.
            TrueSpan {
                start: 100,
                end: 110,
                category: "Phone".into(),
            },
        ];

        let scores = score(&detected, &truth);
        let email = &scores["Email"];
        assert_eq!(email.true_positives, 1);
        assert_eq!(email.false_positives, 0);
        assert_eq!(email.false_negatives, 0);
        assert_eq!(email.f1, 1.0);

        let phone = &scores["Phone"];
        assert_eq!(phone.true_positives, 0);
        assert_eq!(phone.false_positives, 1);
        assert_eq!(phone.false_negatives, 1);
        assert_eq!(phone.precision, 0.0);
        assert_eq!(phone.recall, 0.0);
        assert_eq!(phone.f1, 0.0);
    }

    #[test]
    fn score_treats_off_by_one_overlap_as_true_positive() {
        // Detector span is one byte short on the right but still overlaps.
        let detected = vec![PatternMatch {
            start: 0,
            end: 4,
            category: PiiCategory::Email,
            text: "a@b.".into(),
        }];
        let truth = vec![TrueSpan {
            start: 0,
            end: 5,
            category: "Email".into(),
        }];
        let scores = score(&detected, &truth);
        assert_eq!(scores["Email"].true_positives, 1);
        assert_eq!(scores["Email"].false_negatives, 0);
        assert_eq!(scores["Email"].f1, 1.0);
    }

    #[test]
    fn score_flags_non_overlapping_detection_as_false_positive() {
        let detected = vec![PatternMatch {
            start: 50,
            end: 60,
            category: PiiCategory::Iban,
            text: "FR7630006000011234567890189".into(),
        }];
        // Truth is in a different location and category -> detector is a FP.
        let truth = vec![TrueSpan {
            start: 0,
            end: 5,
            category: "Email".into(),
        }];
        let scores = score(&detected, &truth);
        assert_eq!(scores["Iban"].false_positives, 1);
        assert_eq!(scores["Iban"].precision, 0.0);
        assert_eq!(scores["Email"].false_negatives, 1);
    }

    #[test]
    fn corpus_eval_meets_f1_floor() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pii_eval");
        let corpus = load_corpus(Path::new(dir)).expect("load corpus");

        let mut any = false;
        for (content, truth) in &corpus {
            any = true;
            let scores = evaluate_text(content, truth);
            // Guard only the categories we hand-labeled. The gate catches
            // silent detection regressions (missed spans => false negatives)
            // and gross precision collapse; it intentionally ignores
            // cross-pattern false positives in categories we did not label.
            for t in truth {
                let cat = &t.category;
                let s = scores
                    .get(cat)
                    .unwrap_or_else(|| panic!("no score emitted for labeled category {cat}"));
                assert!(
                    s.f1 >= 0.85,
                    "category {cat} F1 {:.3} below floor 0.85 (tp={}, fp={}, fn={})",
                    s.f1,
                    s.true_positives,
                    s.false_positives,
                    s.false_negatives
                );
                // Every hand-labeled span must be detected (no silent misses).
                assert_eq!(
                    s.false_negatives, 0,
                    "category {cat} has {} undetected ground-truth spans",
                    s.false_negatives
                );
            }
        }
        assert!(any, "corpus loader found no documents");
    }
}
