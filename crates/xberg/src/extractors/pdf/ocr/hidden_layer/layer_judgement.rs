//! Judge an invisible text layer against the page picture's OCR read.

use crate::core::config::OcrQualityThresholds;

/// Largest percentage of an invisible layer's words that a confident OCR read may repeat while the
/// read still replaces the layer. Measured on NACA 1135 p25, whose junk layer shares 9% of its
/// words with its OCR read after normalization; the other audited NACA 1135 layers share 47% or
/// more (57% to 70% for all but p53).
const MAX_REPLACEABLE_LAYER_OVERLAP_PERCENT: usize = 15;
/// A layer shorter than this many words is too small to call a different text rather than a
/// spelling variant: a stamp or a page number.
const MIN_JUDGED_LAYER_WORDS: usize = 20;

/// A word lowercased with its leading and trailing punctuation removed. Digits stay.
fn normalized(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()
}

fn normalized_words(text: &str) -> Option<Vec<String>> {
    Some(
        super::bounded_words(text)?
            .into_iter()
            .map(normalized)
            .filter(|word| !word.is_empty())
            .collect(),
    )
}

/// Whether an OCR read and an invisible layer are so nearly disjoint that the layer is not a
/// partial transcript of the raster but a different text, so the read may replace it. The layer
/// must already be proven invisible and the read confident; this judges only the overlap, on
/// normalized words, and only for a layer of at least [`MIN_JUDGED_LAYER_WORDS`] of them.
///
/// The audit that precedes this accepts only unembedded simple Type1 fonts without a ToUnicode
/// map, so the layer is Latin text and whitespace tokenization is sound; CJK text cannot reach it.
pub(in crate::extractors::pdf::ocr) fn ocr_supersedes_disjoint_layer(native: &str, candidate: &str) -> bool {
    let (Some(native), Some(candidate)) = (normalized_words(native), normalized_words(candidate)) else {
        return false;
    };
    if native.len() < MIN_JUDGED_LAYER_WORDS || candidate.is_empty() {
        return false;
    }
    let mut available: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for word in &candidate {
        *available.entry(word).or_default() += 1;
    }
    let shared = native
        .iter()
        .filter(|word| {
            available.get_mut(word.as_str()).is_some_and(|count| {
                let present = *count > 0;
                *count = count.saturating_sub(1);
                present
            })
        })
        .count();
    shared * 100 <= native.len() * MAX_REPLACEABLE_LAYER_OVERLAP_PERCENT
}

/// Whether a page's calibrated OCR score alone clears the default `min_ocr_mean_confidence`.
/// Replacing an invisible layer needs this even when the caller lowered or disabled the gate,
/// because the replaced text is discarded.
pub(in crate::extractors::pdf::ocr) fn confidence_clears_replacement_floor(score: Option<f64>) -> bool {
    score.is_some_and(|score| score >= OcrQualityThresholds::default().min_ocr_mean_confidence / 100.0)
}
