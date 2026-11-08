//! Judge an invisible text layer against the page picture's OCR read.

/// A layer whose words average more than this many characters has no usable word boundaries.
/// Natural text runs 4 to 7 (the NACA 1135 layers that match their scans: 4.2 to 5.5); the junk
/// layer of NACA 1135 p25 averages 19.
const MAX_PLAUSIBLE_AVERAGE_WORD_LENGTH: f64 = 12.0;
const MIN_WORDS_FOR_WORD_LENGTH_CHECK: usize = 20;

/// Whether the text of a layer, already proven invisible and geometrically covering its raster,
/// is too unlike words to trust without reading the raster.
pub(in crate::extractors::pdf::ocr) fn layer_words_are_implausible(native: &str) -> bool {
    let Some(words) = super::bounded_words(native) else {
        return false;
    };
    let characters: usize = words.iter().map(|word| word.chars().count()).sum();
    words.len() >= MIN_WORDS_FOR_WORD_LENGTH_CHECK
        && characters as f64 > words.len() as f64 * MAX_PLAUSIBLE_AVERAGE_WORD_LENGTH
}

/// Largest percentage of an invisible layer's words that a confident OCR read may repeat while the
/// read still replaces the layer. Measured on NACA 1135 p25, whose junk layer shares 21 of 424
/// words (5%) with its OCR read; a layer that matches its raster shares most of its words.
const MAX_REPLACEABLE_LAYER_OVERLAP_PERCENT: usize = 15;

/// Whether an OCR read and an invisible layer are so nearly disjoint that the layer is not a
/// partial transcript of the raster but a different text, so the read may replace it. The layer
/// must already be proven invisible and the read confident; this judges only the overlap.
pub(in crate::extractors::pdf::ocr) fn ocr_supersedes_disjoint_layer(native: &str, candidate: &str) -> bool {
    let (Some(native), Some(candidate)) = (super::bounded_words(native), super::bounded_words(candidate)) else {
        return false;
    };
    if native.is_empty() || candidate.is_empty() {
        return false;
    }
    let mut available: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for word in &candidate {
        *available.entry(word).or_default() += 1;
    }
    let shared = native
        .iter()
        .filter(|word| {
            available.get_mut(*word).is_some_and(|count| {
                let present = *count > 0;
                *count = count.saturating_sub(1);
                present
            })
        })
        .count();
    shared * 100 <= native.len() * MAX_REPLACEABLE_LAYER_OVERLAP_PERCENT
}
