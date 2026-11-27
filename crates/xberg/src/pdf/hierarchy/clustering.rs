//! Font size clustering for PDF hierarchy extraction.
//!
//! This module implements k-means clustering on font sizes to identify
//! document hierarchy levels (headings vs body text).

use super::types::TextBlock;
use crate::pdf::error::{PdfError, Result};

const KMEANS_MAX_ITERATIONS: usize = 100;
const KMEANS_CONVERGENCE_THRESHOLD: f32 = 0.01;

/// A cluster of text blocks with the same font size characteristics.
#[derive(Debug, Clone)]
pub struct FontSizeCluster {
    /// The centroid (mean) font size of this cluster
    pub centroid: f32,
    /// The text blocks that belong to this cluster
    pub members: Vec<TextBlock>,
}

/// Cluster text blocks by font size using k-means algorithm.
///
/// Uses k-means clustering to group text blocks by their font size, which helps
/// identify document hierarchy levels (H1, H2, Body, etc.). The algorithm:
/// 1. Extracts font sizes from text blocks
/// 2. Applies k-means clustering to group similar font sizes
/// 3. Sorts clusters by centroid size in descending order (largest = H1)
/// 4. Returns clusters with their member blocks
///
/// # Arguments
///
/// * `blocks` - Slice of TextBlock objects to cluster
/// * `k` - Number of clusters to create
///
/// # Returns
///
/// Result with vector of FontSizeCluster ordered by size (descending),
/// or an error if clustering fails
///
/// # Example
///
/// Not run as a doctest: `cluster_font_sizes` is `pub(crate)`, internal to the PDF
/// hierarchy pass. ([`TextBlock`] itself is public — only the function is not.)
/// Downstream crates see its effect as heading levels on the extracted document
/// structure.
///
/// ```ignore
/// use xberg::pdf::hierarchy::{TextBlock, BoundingBox, cluster_font_sizes};
///
/// let blocks = vec![
///     TextBlock {
///         text: "Title".to_string(),
///         bbox: BoundingBox { left: 0.0, top: 0.0, right: 100.0, bottom: 24.0 },
///         font_size: 24.0,
///     },
///     TextBlock {
///         text: "Body".to_string(),
///         bbox: BoundingBox { left: 0.0, top: 30.0, right: 100.0, bottom: 42.0 },
///         font_size: 12.0,
///     },
/// ];
///
/// let clusters = cluster_font_sizes(&blocks, 2).unwrap();
/// assert_eq!(clusters.len(), 2);
/// assert_eq!(clusters[0].centroid, 24.0); // Largest is first
/// ```
pub(crate) fn cluster_font_sizes(blocks: &[TextBlock], k: usize) -> Result<Vec<FontSizeCluster>> {
    if blocks.is_empty() {
        return Ok(Vec::new());
    }

    if k == 0 {
        return Err(PdfError::TextExtractionFailed("K must be greater than 0".to_string()));
    }

    let actual_k = k.min(blocks.len());

    let mut font_sizes: Vec<f32> = blocks.iter().map(|b| b.font_size).filter(|fs| fs.is_finite()).collect();
    if font_sizes.is_empty() {
        // Every block's font size was NaN/infinite (a PDF can produce this via a
        // degenerate text/font matrix), so there is no usable font-size signal at
        // all — treat it the same as the no-blocks case above rather than let the
        // `else` branch below underflow `font_sizes.len() - 1` on an empty `Vec`.
        return Ok(Vec::new());
    }
    font_sizes.sort_by(|a, b| b.total_cmp(a));
    font_sizes.dedup_by(|a, b| (*a - *b).abs() < 0.05);

    let mut centroids: Vec<f32> = Vec::new();

    if font_sizes.len() >= actual_k {
        let step = font_sizes.len() / actual_k;
        for i in 0..actual_k {
            let idx = i * step;
            centroids.push(font_sizes[idx.min(font_sizes.len() - 1)]);
        }
    } else {
        centroids = font_sizes.clone();

        let min_font = font_sizes[font_sizes.len() - 1];
        let max_font = font_sizes[0];
        let range = max_font - min_font;

        while centroids.len() < actual_k {
            let t = centroids.len() as f32 / (actual_k - 1) as f32;
            let interpolated = max_font - t * range;
            centroids.push(interpolated);
        }

        centroids.sort_by(|a, b| b.total_cmp(a));
    }

    let font_sizes: Vec<f32> = blocks.iter().map(|b| b.font_size).collect();

    let centroids = refine_centroids(&font_sizes, centroids, actual_k);

    let clusters = assign_blocks_to_centroids(blocks, &centroids);

    let mut result: Vec<FontSizeCluster> = Vec::new();

    for i in 0..actual_k {
        if !clusters[i].is_empty() {
            let centroid_value = centroids[i];
            result.push(FontSizeCluster {
                centroid: centroid_value,
                members: clusters[i].clone(),
            });
        }
    }

    result.sort_by(|a, b| b.centroid.total_cmp(&a.centroid));

    Ok(result)
}

/// Assign heading levels using the "most frequent cluster = Body" rule.
///
/// Instead of naively mapping the largest font size to H1, this function
/// identifies the cluster with the most members as body text. Only clusters
/// with fewer members AND sufficiently larger font size than body become headings.
///
/// # Arguments
///
/// * `clusters` - Slice of FontSizeCluster objects (sorted by centroid descending)
/// * `min_heading_ratio` - Minimum ratio of heading centroid to body centroid (e.g. 1.15)
///
/// # Returns
///
/// Vector of tuples `(centroid, heading_level)` where `None` means body text
/// and `Some(1..=6)` means H1-H6. Sorted by centroid descending.
///
/// # Scale invariance
///
/// `font_size` on a [`TextBlock`] is an opaque magnitude: for native PDFs it is
/// typographic points, for OCR-derived blocks it is a render-DPI-dependent pixel
/// measurement. This function previously also accepted a `min_heading_gap`
/// absolute-unit parameter and required a candidate cluster to clear
/// `body_centroid + min_heading_gap` in addition to the ratio. That is
/// mathematically indistinguishable, for any single scale-free function of
/// `body_centroid` alone, from just a second (smaller) ratio bound: for a fixed
/// gap `g`, `body + g` only ever equals `body * (1 + g / body)`, i.e. a ratio
/// that *shrinks* as `body_centroid` grows. On a 300 DPI OCR render a 21px body
/// cluster is already "large" in raw units even though it represents an
/// ordinary ~5pt-equivalent font, so the shrunk ratio let a 23px cluster
/// (23/21 = 1.095, well under a 1.15 ratio) through purely because 23 >= 21 +
/// 1.5. There is no absolute-unit choice that is simultaneously correct for
/// points and for pixels, so the gap term has been removed and the ratio is
/// now the sole (scale-invariant) test.
pub(crate) fn assign_heading_levels_smart(
    clusters: &[FontSizeCluster],
    min_heading_ratio: f32,
) -> Vec<(f32, Option<u8>)> {
    if clusters.is_empty() {
        return Vec::new();
    }

    if clusters.len() == 1 {
        return vec![(clusters[0].centroid, None)];
    }

    let body_idx = clusters
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| c.members.iter().map(|block| block.text.len()).sum::<usize>())
        .map(|(i, _)| i)
        .unwrap_or(0);

    let body_centroid = clusters[body_idx].centroid;

    let heading_threshold = body_centroid * min_heading_ratio;

    let mut heading_candidates: Vec<(usize, f32)> = clusters
        .iter()
        .enumerate()
        .filter(|(i, c)| *i != body_idx && c.centroid >= heading_threshold)
        .map(|(i, c)| (i, c.centroid))
        .collect();

    heading_candidates.sort_by(|a, b| b.1.total_cmp(&a.1));

    let max_headings = 6usize;
    let mut result: Vec<(f32, Option<u8>)> = Vec::with_capacity(clusters.len());

    for (i, cluster) in clusters.iter().enumerate() {
        if i == body_idx {
            result.push((cluster.centroid, None));
        } else if let Some(pos) = heading_candidates.iter().position(|(idx, _)| *idx == i) {
            if pos < max_headings {
                result.push((cluster.centroid, Some((pos + 1) as u8)));
            } else {
                result.push((cluster.centroid, None));
            }
        } else {
            result.push((cluster.centroid, None));
        }
    }

    result
}

/// Run the k-means refinement loop over the seeded centroids.
///
/// Iterates until assignments stop changing, the centroids stop moving, or
/// `KMEANS_MAX_ITERATIONS` is reached, and returns the refined centroids.
fn refine_centroids(font_sizes: &[f32], mut centroids: Vec<f32>, actual_k: usize) -> Vec<f32> {
    let mut prev_assignments: Vec<usize> = vec![0; font_sizes.len()];
    let mut assignments: Vec<usize> = Vec::with_capacity(font_sizes.len());
    let mut size_clusters: Vec<Vec<f32>> = vec![Vec::new(); centroids.len()];
    let mut new_centroids: Vec<f32> = Vec::with_capacity(actual_k);
    let mut first_iter = true;

    for _ in 0..KMEANS_MAX_ITERATIONS {
        assign_sizes_to_centroids_tracked(font_sizes, &centroids, &mut size_clusters, &mut assignments);

        let assignments_changed = if first_iter {
            first_iter = false;
            1
        } else {
            assignments
                .iter()
                .zip(prev_assignments.iter())
                .filter(|(a, b)| a != b)
                .count()
        };
        std::mem::swap(&mut prev_assignments, &mut assignments);

        if assignments_changed == 0 {
            break;
        }

        new_centroids.clear();
        for (i, cluster) in size_clusters.iter().enumerate() {
            if !cluster.is_empty() {
                new_centroids.push(cluster.iter().sum::<f32>() / cluster.len() as f32);
            } else {
                new_centroids.push(centroids[i]);
            }
        }

        let converged = centroids
            .iter()
            .zip(new_centroids.iter())
            .all(|(old, new)| (old - new).abs() < KMEANS_CONVERGENCE_THRESHOLD);

        std::mem::swap(&mut centroids, &mut new_centroids);

        if converged {
            break;
        }
    }

    centroids
}

/// Helper function to assign font sizes to their nearest centroid (for iteration loop).
///
/// Assigns font sizes to clusters without cloning full TextBlock objects, and also
/// records per-element cluster assignments so the caller can detect convergence via
/// unchanged assignments (in addition to the centroid-movement threshold).
///
/// Both outputs are caller-owned buffers, emptied here and refilled, so the
/// refinement loop allocates them once rather than once per iteration.
///
/// # Arguments
///
/// * `font_sizes` - Slice of font size values to assign
/// * `centroids` - Slice of centroid values (one per cluster)
/// * `clusters` - One buffer per centroid, refilled with the font sizes assigned to it
///   in `font_sizes` order
/// * `assignments` - Refilled with the per-element cluster indices (same length as
///   `font_sizes`)
fn assign_sizes_to_centroids_tracked(
    font_sizes: &[f32],
    centroids: &[f32],
    clusters: &mut [Vec<f32>],
    assignments: &mut Vec<usize>,
) {
    for cluster in clusters.iter_mut() {
        cluster.clear();
    }
    assignments.clear();

    for &size in font_sizes {
        let mut min_distance = f32::INFINITY;
        let mut best_cluster = 0;

        for (i, &centroid) in centroids.iter().enumerate() {
            let distance = (size - centroid).abs();
            if distance < min_distance {
                min_distance = distance;
                best_cluster = i;
            }
        }

        clusters[best_cluster].push(size);
        assignments.push(best_cluster);
    }
}

/// Helper function to assign blocks to their nearest centroid.
///
/// Iterates through blocks and finds the closest centroid for each block,
/// grouping them into clusters. Used in the final assignment step after convergence.
///
/// # Arguments
///
/// * `blocks` - Slice of TextBlock objects to assign
/// * `centroids` - Slice of centroid values (one per cluster)
///
/// # Returns
///
/// A vector of clusters, where each cluster contains the TextBlock objects
/// assigned to that centroid
fn assign_blocks_to_centroids(blocks: &[TextBlock], centroids: &[f32]) -> Vec<Vec<TextBlock>> {
    let mut clusters: Vec<Vec<TextBlock>> = vec![Vec::new(); centroids.len()];

    for block in blocks {
        let mut min_distance = f32::INFINITY;
        let mut best_cluster = 0;

        for (i, &centroid) in centroids.iter().enumerate() {
            let distance = (block.font_size - centroid).abs();
            if distance < min_distance {
                min_distance = distance;
                best_cluster = i;
            }
        }

        clusters[best_cluster].push(block.clone());
    }

    clusters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::BoundingBox;

    fn make_block(text: &str, font_size: f32) -> TextBlock {
        TextBlock {
            text: text.to_string(),
            bbox: BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: font_size,
            },
            font_size,
        }
    }

    /// Every block's font size is non-finite (NaN), which a PDF can produce via a
    /// degenerate text/font matrix. Before the `font_sizes.is_empty()` guard, the
    /// `.filter(|fs| fs.is_finite())` step emptied `font_sizes` entirely, and the
    /// `else` branch (taken whenever `font_sizes.len() < actual_k`, which includes
    /// zero) then computed `font_sizes[font_sizes.len() - 1]`: `0usize - 1`
    /// underflows (a debug-mode panic on its own), and in a release build (no
    /// `overflow-checks`, matching this workspace's profile) the wrapped
    /// `usize::MAX` index still panics on the following `Vec` index — bounds
    /// checks are independent of `overflow-checks`. `cluster_font_sizes` must
    /// instead return an empty cluster list, exactly like the pre-existing
    /// no-blocks-at-all case just above it.
    #[test]
    fn all_non_finite_font_sizes_returns_empty_clusters_instead_of_panicking() {
        let blocks = vec![make_block("Heading with a broken font matrix", f32::NAN)];

        let clusters = cluster_font_sizes(&blocks, 1).expect("must not panic on all-NaN font sizes");

        assert!(
            clusters.is_empty(),
            "no finite font-size signal exists, so no clusters should be produced, got {clusters:?}"
        );
    }

    /// Positive control: a normal single finite-font-size block (the same shape
    /// as the panic case, minus the NaN) must still produce one cluster centered
    /// on that font size — the fix must not turn ordinary single-block input into
    /// an empty result too.
    #[test]
    fn single_finite_font_size_still_produces_one_cluster() {
        let blocks = vec![make_block("Ordinary heading", 18.0)];

        let clusters = cluster_font_sizes(&blocks, 1).expect("clustering a single finite font size must succeed");

        assert_eq!(clusters.len(), 1, "expected exactly one cluster, got {clusters:?}");
        assert_eq!(clusters[0].centroid, 18.0);
        assert_eq!(clusters[0].members.len(), 1);
    }

    #[test]
    fn reused_assignment_buffers_hold_only_the_latest_assignment() {
        let font_sizes = [24.0, 12.0, 11.0];
        let mut clusters = vec![Vec::new(); 2];
        let mut assignments = Vec::new();

        assign_sizes_to_centroids_tracked(&font_sizes, &[24.0, 12.0], &mut clusters, &mut assignments);
        assert_eq!(clusters, [vec![24.0], vec![12.0, 11.0]]);
        assert_eq!(assignments, [0, 1, 1]);

        assign_sizes_to_centroids_tracked(&font_sizes, &[12.0, 24.0], &mut clusters, &mut assignments);
        assert_eq!(clusters, [vec![12.0, 11.0], vec![24.0]]);
        assert_eq!(assignments, [1, 0, 0]);
    }

    #[test]
    fn test_body_cluster_by_text_content_not_member_count() {
        let mut blocks = Vec::new();
        for i in 0..10 {
            blocks.push(make_block(&format!("Hdr{i}"), 8.0));
        }
        for _ in 0..3 {
            blocks.push(make_block("This is a longer body text paragraph with content.", 12.0));
        }

        let clusters = cluster_font_sizes(&blocks, 2).unwrap();
        let levels = assign_heading_levels_smart(&clusters, 1.15);

        let body_centroid = levels.iter().find(|(_, l)| l.is_none()).map(|(c, _)| *c);
        assert!(body_centroid.is_some(), "should have a body cluster");
        let bc = body_centroid.unwrap();
        assert!((bc - 12.0).abs() < 1.0, "body centroid should be near 12pt, got {bc}");
    }

    /// Builds two `FontSizeCluster`s directly (bypassing k-means) so the body/candidate
    /// centroids can be pinned to exact values, matching a measured input distribution.
    fn two_clusters(body_centroid: f32, candidate_centroid: f32) -> Vec<FontSizeCluster> {
        vec![
            FontSizeCluster {
                centroid: candidate_centroid,
                members: vec![make_block("Short", candidate_centroid)],
            },
            FontSizeCluster {
                centroid: body_centroid,
                members: vec![make_block(
                    "This cluster carries far more running text so char-weighted mass picks it as body.",
                    body_centroid,
                )],
            },
        ]
    }

    /// Real Tesseract hOCR `x_fsize` values from a 300 DPI scan (measured against
    /// `test_documents/images_extra/ocr_image.tiff`): body text clusters at 21, a
    /// secondary/subhead-looking tier clusters at 23. Their ratio is 23/21 = 1.095,
    /// which correctly fails the 1.15 `MIN_HEADING_FONT_RATIO` gate. Their absolute
    /// gap is 23 - 21 = 2.0, which incorrectly *clears* the old 1.5 absolute
    /// `MIN_HEADING_FONT_GAP`, so the pre-fix `min(ratio_bound, abs_bound)` combinator
    /// picked the weaker (abs) bound and promoted a 9%-larger body run to a heading.
    ///
    /// Fails without the fix: the removed code computed
    /// `heading_threshold = (21.0 * 1.15).min(21.0 + 1.5) = 22.5`, and 23.0 >= 22.5,
    /// so `candidate_level` was `Some(1)` instead of `None`.
    #[test]
    fn test_pixel_scale_ratio_gate_rejects_ocr_subhead_noise() {
        let clusters = two_clusters(21.0, 23.0);
        let levels = assign_heading_levels_smart(&clusters, 1.15);

        let candidate_level = levels
            .iter()
            .find(|(centroid, _)| (*centroid - 23.0).abs() < 0.01)
            .map(|(_, level)| *level);
        assert_eq!(
            candidate_level,
            Some(None),
            "a 23px cluster over a 21px body (ratio 1.095) must not be promoted to a heading"
        );
    }

    /// Pins the documented calibration point shared by `MIN_HEADING_FONT_RATIO`'s and
    /// the old `MIN_HEADING_FONT_GAP`'s doc comments (both cite a 10pt body): at
    /// exactly body=10, `body * 1.15` and `body + 1.5` are numerically identical
    /// (11.5), so removing the gap term changes nothing at this reference size. This
    /// does not fail without the fix (both formulas agree here); it documents why the
    /// native corpus, calibrated near a 10pt body, is expected to be unaffected.
    #[test]
    fn test_native_reference_body_boundary_is_unaffected_by_gap_removal() {
        let clusters = two_clusters(10.0, 11.5);
        let levels = assign_heading_levels_smart(&clusters, 1.15);

        let candidate_level = levels
            .iter()
            .find(|(centroid, _)| (*centroid - 11.5).abs() < 0.01)
            .map(|(_, level)| *level);
        assert_eq!(
            candidate_level,
            Some(Some(1)),
            "11.5 over a 10pt body sits exactly on the ratio boundary"
        );
    }

    /// Documents a genuine, deliberate *native*-scale behavior change: for a body font
    /// above the 10pt reference (e.g. the common 12pt Word default), the removed gap
    /// term let candidates clear a shrunk effective ratio (13.5/12 = 12.5%) instead of
    /// the full 15%. A 13.6pt candidate (ratio 1.133) cleared the old gap-relaxed bound
    /// but fails the pure-ratio bound used after this fix.
    ///
    /// Fails without the fix: the removed code computed
    /// `heading_threshold = (12.0 * 1.15).min(12.0 + 1.5) = 13.5`, and 13.6 >= 13.5, so
    /// `candidate_level` was `Some(1)` instead of `None`. If a real fixture relies on
    /// this specific relaxed-ratio window (body ~12pt, heading ~12.5-13.7pt), this test
    /// documents exactly why it would now read as body text — see report.
    #[test]
    fn test_native_body_above_reference_no_longer_gets_gap_relaxation() {
        let clusters = two_clusters(12.0, 13.6);
        let levels = assign_heading_levels_smart(&clusters, 1.15);

        let candidate_level = levels
            .iter()
            .find(|(centroid, _)| (*centroid - 13.6).abs() < 0.01)
            .map(|(_, level)| *level);
        assert_eq!(
            candidate_level,
            Some(None),
            "13.6pt over a 12pt body (ratio 1.133) no longer clears the pure-ratio 1.15 gate"
        );
    }

    #[test]
    fn test_body_cluster_equal_members_picks_more_content() {
        let blocks = vec![
            make_block("AB", 18.0),
            make_block("CD", 18.0),
            make_block("This is much longer body text content here.", 12.0),
            make_block("Another long paragraph of body text for the doc.", 12.0),
        ];

        let clusters = cluster_font_sizes(&blocks, 2).unwrap();
        let levels = assign_heading_levels_smart(&clusters, 1.15);

        let body_centroid = levels.iter().find(|(_, l)| l.is_none()).map(|(c, _)| *c);
        assert!(body_centroid.is_some());
        let bc = body_centroid.unwrap();
        assert!(
            (bc - 12.0).abs() < 1.0,
            "body should be 12pt cluster (more text), got {bc}"
        );
    }
}
