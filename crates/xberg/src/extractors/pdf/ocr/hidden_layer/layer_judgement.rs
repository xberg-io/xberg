//! Judge an invisible text layer against the page picture's OCR read.

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
