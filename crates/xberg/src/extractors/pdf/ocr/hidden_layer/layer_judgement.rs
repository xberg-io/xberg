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

/// Fragmentation is deliberately not a veto: a table of numbers is mostly short tokens.
pub(in crate::extractors::pdf::ocr) fn confident_read_replaces_layer(
    low_confidence: bool,
    dictionary_noise: bool,
    score: Option<f64>,
    native: &str,
    read: &str,
) -> bool {
    !low_confidence
        && !dictionary_noise
        && confidence_clears_replacement_floor(score)
        && ocr_supersedes_disjoint_layer(native, read)
}

/// Whether the language check judges the covering layer of a probed page plausible prose. Only a
/// covered page (`needed == Some(false)`) with layer text can be.
pub(in crate::extractors::pdf::ocr) fn layer_judged_plausible(
    needed: Option<bool>,
    layer_text: Option<&str>,
    thresholds: &OcrQualityThresholds,
) -> bool {
    needed == Some(false)
        && layer_text.is_some_and(|text| {
            let boundary = crate::types::PageBoundary {
                byte_start: 0,
                byte_end: text.len(),
                page_number: 1,
            };
            !crate::extractors::pdf::ocr::plausibility::scan_text_plausibility(
                text,
                Some(&[boundary]),
                None,
                thresholds,
            )
            .plausible
            .is_empty()
        })
}

/// The pages the OCR probe reads, from `(page index, coverage verdict, layer judged plausible)`.
pub(in crate::extractors::pdf::ocr) fn pages_to_probe(decisions: Vec<(usize, Option<bool>, bool)>) -> Vec<usize> {
    decisions
        .into_iter()
        .filter_map(|(index, needed, plausible)| {
            let probed = super::probes_page(needed, plausible);
            tracing::debug!(
                page = index + 1,
                ?needed,
                plausible,
                probed,
                "Hidden PDF text probe decision"
            );
            probed.then_some(index)
        })
        .collect()
}
