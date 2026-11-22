//! Topic boundary detection via embedding similarity.
//!
//! Compares adjacent segment embeddings to find points where the topic shifts
//! beyond a configurable threshold, producing boundary markers for the merge step.

/// Compute cosine similarity between two vectors.
///
/// Returns a value in `[-1.0, 1.0]`. If either vector has near-zero magnitude
/// the function returns `0.0` rather than producing `NaN`.
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "vectors must have equal length");

    // Accumulate in f64: squared f32 components overflow f32 above ~1.8e19.
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    for (&ai, &bi) in a.iter().zip(b.iter()) {
        let (ai, bi) = (f64::from(ai), f64::from(bi));
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f64::from(f32::EPSILON) {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0) as f32
    }
}

/// Detect topic boundaries across a sequence of text segments.
///
/// Embeds all segments in a single batch, then marks a boundary wherever the
/// cosine similarity between consecutive embeddings drops below `threshold`.
/// Pre-existing forced boundaries (e.g. from structural cues) are preserved.
///
/// # Arguments
///
/// * `segment_texts` — ordered slice of segment strings
/// * `forced_boundaries` — per-segment flags; `true` means a boundary is already
///   decided and should not be overridden
/// * `embedding_config` — model and batch-size configuration forwarded to
///   [`crate::embeddings::embed_texts`]
/// * `threshold` — similarity below this value triggers a new boundary
///
/// # Returns
///
/// A `Vec<bool>` of the same length as `segment_texts` where `true` marks the
/// start of a new topic group.
pub(crate) fn detect_topic_boundaries(
    segment_texts: &[&str],
    forced_boundaries: &[bool],
    embedding_config: &crate::EmbeddingConfig,
    threshold: f32,
) -> crate::error::Result<Vec<bool>> {
    let n = segment_texts.len();
    if n == 0 {
        return Ok(Vec::new());
    }

    let mut boundaries = vec![false; n];
    boundaries[0] = true;

    for (i, &forced) in forced_boundaries.iter().enumerate().take(n) {
        if forced {
            boundaries[i] = true;
        }
    }

    if n < 2 {
        return Ok(boundaries);
    }

    let embeddings = crate::embeddings::embed_texts(segment_texts, embedding_config)?;

    if embeddings.len() != n {
        return Err(crate::XbergError::validation(format!(
            "expected {} embeddings, got {}",
            n,
            embeddings.len()
        )));
    }

    for i in 1..n {
        if boundaries[i] {
            continue;
        }
        let sim = cosine_similarity(&embeddings[i - 1], &embeddings[i]);
        if sim < threshold {
            boundaries[i] = true;
        }
    }

    Ok(boundaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "identical vectors should have similarity ~1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "orthogonal vectors should have similarity ~0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim + 1.0).abs() < 1e-6,
            "opposite vectors should have similarity ~-1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_normalized() {
        let norm = (1.0_f32 * 1.0 + 2.0 * 2.0 + 3.0 * 3.0).sqrt();
        let a: Vec<f32> = vec![1.0 / norm, 2.0 / norm, 3.0 / norm];

        let norm2 = (4.0_f32 * 4.0 + 5.0 * 5.0 + 6.0 * 6.0).sqrt();
        let b: Vec<f32> = vec![4.0 / norm2, 5.0 / norm2, 6.0 / norm2];

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - dot).abs() < 1e-6,
            "for unit vectors cosine_similarity should equal dot product: sim={sim}, dot={dot}"
        );
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "zero vector should yield similarity 0.0, got {sim}");
    }

    #[test]
    fn cosine_similarity_large_vectors() {
        let a: Vec<f32> = (0..100).map(|i| (i as f32).sin()).collect();
        let b: Vec<f32> = (0..100).map(|i| (i as f32).cos()).collect();

        let sim = cosine_similarity(&a, &b);
        assert!(
            (-1.0..=1.0).contains(&sim),
            "similarity should be in [-1, 1], got {sim}"
        );

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        let expected = dot / (na * nb);
        assert!(
            (sim - expected).abs() < 1e-5,
            "mismatch: sim={sim}, expected={expected}"
        );
    }

    #[test]
    fn cosine_similarity_very_small_values() {
        let a = vec![1e-20_f32, 1e-20, 1e-20];
        let b = vec![1e-20_f32, 1e-20, 1e-20];
        let sim = cosine_similarity(&a, &b);
        assert!(!sim.is_nan(), "should not be NaN for very small values");
    }

    #[test]
    fn cosine_similarity_stays_finite_on_large_finite_vectors() {
        let a = vec![1e30_f32; 3];
        let b = vec![-1e30_f32; 3];
        let same = cosine_similarity(&a, &a);
        let opposite = cosine_similarity(&a, &b);
        assert!(same.is_finite() && (same - 1.0).abs() < 1e-6, "got {same}");
        assert!(opposite.is_finite() && (opposite + 1.0).abs() < 1e-6, "got {opposite}");
    }

    #[test]
    #[should_panic(expected = "vectors must have equal length")]
    fn cosine_similarity_mismatched_lengths_panics() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        cosine_similarity(&a, &b);
    }

    #[test]
    fn cosine_similarity_topic_shift_simulation() {
        let seg0 = vec![1.0, 0.9, 0.1, 0.0];
        let seg1 = vec![0.95, 0.85, 0.15, 0.05];
        let seg2 = vec![0.1, 0.0, 0.9, 1.0];

        let sim_same = cosine_similarity(&seg0, &seg1);
        let sim_shift = cosine_similarity(&seg1, &seg2);

        assert!(
            sim_same > 0.9,
            "same-topic segments should have high similarity, got {sim_same}"
        );
        assert!(
            sim_shift < 0.5,
            "topic-shift segments should have low similarity, got {sim_shift}"
        );

        let threshold = 0.75;
        assert!(sim_same >= threshold, "same-topic pair should be above threshold");
        assert!(sim_shift < threshold, "topic-shift pair should be below threshold");
    }
}
