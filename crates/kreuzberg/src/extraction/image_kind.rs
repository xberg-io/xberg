//! Image classification and clustering for document extraction.
//!
//! This module provides heuristic classification of extracted images and
//! spatial clustering to identify raster tile fragments that compose a single figure.

use crate::types::{ExtractedImage, ImageKind};
use std::collections::HashMap;

/// Pixel-area below which the small-image rules (Icon / Decoration) trigger.
/// Tuned for typical icon sizes (16×16 to 64×64).
const SMALL_IMAGE_AREA: u64 = 64 * 64;
/// Aspect-ratio band that distinguishes a near-square Icon from a Decoration strip.
const ICON_ASPECT_LOW: f64 = 0.8;
const ICON_ASPECT_HIGH: f64 = 1.2;
/// A Decoration is a tiny image with an extreme aspect ratio outside this band.
const DECORATION_ASPECT_LOW: f64 = 0.2;
const DECORATION_ASPECT_HIGH: f64 = 5.0;
/// Pixel-area above which a JPEG is biased toward Photograph.
const LARGE_JPEG_AREA: u64 = 800 * 800;
/// Pixel-area below which low-entropy images are classified as Chart rather than
/// Photograph (charts tend to be small, palette-poor compared to photos).
const SMALL_CHART_AREA: u64 = 400 * 400;
/// Shannon-entropy threshold (bits / byte) that biases an image toward Photograph.
const HIGH_ENTROPY_THRESHOLD: f64 = 6.0;
/// Shannon-entropy threshold below which a small image is classified as Chart.
const LOW_ENTROPY_THRESHOLD: f64 = 3.0;
/// Hard cap on the source-image pixel count we are willing to fully decode for
/// entropy analysis. Beyond this we skip the entropy step rather than risk a
/// multi-gigabyte allocation in `image::load_from_memory`.
const MAX_CLASSIFY_PIXELS: u64 = 64 * 1024 * 1024; // 64 megapixels

/// Classify an image based on its metadata and visual properties.
///
/// Uses a rule cascade over already-captured signals: dimensions, aspect ratio,
/// colorspace, bits-per-component, format, and histogram entropy on a downsampled
/// 64×64 thumbnail.
///
/// # Arguments
///
/// * `bytes` — Raw image bytes (should be decodable to standard formats)
/// * `format` — Image format (e.g., "jpeg", "png", "ccitt")
/// * `width` — Image width in pixels
/// * `height` — Image height in pixels
/// * `colorspace` — Colorspace name (e.g., "RGB", "CMYK", "Gray", "Indexed")
/// * `bits_per_component` — Bits per color component (e.g., 1, 8, 16)
/// * `is_mask` — Whether this image is a transparency or alpha mask
///
/// # Returns
///
/// A tuple of `(ImageKind, confidence)` where confidence is in [0.0, 1.0].
/// Returns `(Unknown, 0.0)` if bytes cannot be decoded.
pub fn classify(
    bytes: &[u8],
    format: &str,
    width: Option<u32>,
    height: Option<u32>,
    colorspace: Option<&str>,
    bits_per_component: Option<u32>,
    is_mask: bool,
) -> (ImageKind, f32) {
    // Short-circuit: explicit mask flag
    if is_mask {
        return (ImageKind::Mask, 0.95);
    }

    // Extract dimensions, defaulting to 1 if missing to avoid panic on area calc
    let w = width.unwrap_or(1);
    let h = height.unwrap_or(1);
    let area = (w as u64) * (h as u64);

    // Aspect ratio: prefer f64 for precision
    let aspect = if h > 0 { (w as f64) / (h as f64) } else { 1.0 };

    // Degenerate dimensions (0×0 / 0×N / N×0): skip every dimension-gated rule.
    // No useful classification can be inferred and entropy decode would fail
    // anyway. Leave it for the entropy path or fall through to Unknown.
    if w == 0 || h == 0 {
        return (ImageKind::Unknown, 0.0);
    }

    // Rule: Tiny square → Icon (small icons are typically 16×16 to 64×64)
    if area < SMALL_IMAGE_AREA && aspect > ICON_ASPECT_LOW && aspect < ICON_ASPECT_HIGH {
        return (ImageKind::Icon, 0.85);
    }

    // Rule: Tiny image with anything-but-square aspect → Decoration.
    // Note: this catches the gap between the Icon band and the prior Decoration
    // band (e.g. small image with aspect 0.25), which would otherwise reach the
    // entropy path and end up Unknown.
    if area < SMALL_IMAGE_AREA && !(ICON_ASPECT_LOW..=ICON_ASPECT_HIGH).contains(&aspect) {
        let confidence = if (DECORATION_ASPECT_LOW..=DECORATION_ASPECT_HIGH).contains(&aspect) {
            0.65 // moderate-aspect tiny image: less confident
        } else {
            0.80 // extreme-aspect tiny strip: high confidence
        };
        return (ImageKind::Decoration, confidence);
    }

    // Rule: Gray + 1bpp → TextBlock
    if colorspace == Some("Gray") && bits_per_component == Some(1) {
        return (ImageKind::TextBlock, 0.75);
    }

    // Rule: CMYK + 8bpp → Photograph
    if colorspace == Some("CMYK") && bits_per_component == Some(8) {
        return (ImageKind::Photograph, 0.70);
    }

    // Rule: JPEG + large area → Photograph
    if format == "jpeg" && area > LARGE_JPEG_AREA {
        return (ImageKind::Photograph, 0.85);
    }

    // Rule: FlateDecode + Indexed colorspace → Diagram (palette images = vector-rasterized)
    if format == "flate" && colorspace == Some("Indexed") {
        return (ImageKind::Diagram, 0.65);
    }

    // Rule: CCITT format → Mask (bilevel, typically masks or text)
    if format == "ccitt" {
        return (ImageKind::Mask, 0.85);
    }

    // Entropy-based classification: only attempt for images we are willing to
    // fully decode. Rejecting oversized inputs up front prevents the
    // `image::load_from_memory` call inside `compute_entropy_on_thumbnail` from
    // allocating multi-gigabyte buffers for a crafted PDF/DOCX image stream.
    if area > 0
        && area <= MAX_CLASSIFY_PIXELS
        && let Ok(entropy) = compute_entropy_on_thumbnail(bytes, w, h)
    {
        // High entropy → Photograph
        if entropy > HIGH_ENTROPY_THRESHOLD {
            return (ImageKind::Photograph, 0.65);
        }
        // Low entropy + small → Chart
        if entropy < LOW_ENTROPY_THRESHOLD && area < SMALL_CHART_AREA {
            return (ImageKind::Chart, 0.60);
        }
    }

    // Fallback
    (ImageKind::Unknown, 0.50)
}

/// Iterative path-compressing find for the cluster_tiles union-find.
///
/// Two passes: first walk to the root, then re-walk and rewrite each parent
/// pointer to that root. Iterative to avoid stack overflow on adversarial
/// inputs (a chain of N parent pointers would otherwise consume N stack frames).
fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    let mut root = x;
    while parent[root] != root {
        root = parent[root];
    }
    while parent[x] != root {
        let next = parent[x];
        parent[x] = root;
        x = next;
    }
    root
}

/// Union two nodes in the union-find, rooting at the smaller index for
/// determinism (so cluster IDs follow document reading order).
fn uf_union(parent: &mut [usize], x: usize, y: usize) {
    let px = uf_find(parent, x);
    let py = uf_find(parent, y);
    if px != py {
        let (smaller, larger) = if px < py { (px, py) } else { (py, px) };
        parent[larger] = smaller;
    }
}

/// Compute entropy of a downsampled 64×64 thumbnail.
///
/// Attempts to load the image using the `image` crate, resize to 64×64,
/// and compute Shannon entropy of the flattened RGB histogram.
/// Returns `Err` if the image cannot be decoded or is too small.
///
/// Only available when the `image-processing` feature is enabled (via ocr or ocr-wasm).
#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
fn compute_entropy_on_thumbnail(bytes: &[u8], _width: u32, _height: u32) -> Result<f64, String> {
    use image::imageops::FilterType;

    // Attempt to load the image
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;

    // Resize to 64×64 for analysis
    let thumb = img.resize_exact(64, 64, FilterType::Lanczos3);

    // Convert to RGB for consistent analysis
    let rgb = thumb.to_rgb8();
    let pixels = rgb.as_raw();

    // Compute histogram of all channels combined
    let mut histogram = vec![0u32; 256];
    for &byte in pixels {
        histogram[byte as usize] += 1;
    }

    // Compute Shannon entropy
    let total = pixels.len() as f64;
    let mut entropy = 0.0;
    for count in histogram {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.log2();
        }
    }

    Ok(entropy)
}

/// Fallback entropy computation when image crate is unavailable.
#[cfg(not(any(feature = "ocr", feature = "ocr-wasm")))]
fn compute_entropy_on_thumbnail(_bytes: &[u8], _width: u32, _height: u32) -> Result<f64, String> {
    Err("Image processing not available".to_string())
}

/// Cluster spatially adjacent, similarly-sized images on a page.
///
/// Groups images that appear to be tiles of a single figure (e.g., a technical
/// drawing composed of dozens of raster fragments). For each group with 2+ members,
/// assigns a shared `cluster_id` and reclassifies members as `TileFragment`.
///
/// Clustering criteria:
/// - Images must be on the same page
/// - Images must be classified as `Drawing`, `Diagram`, or `TileFragment` (or unclassified with area < 300×300)
/// - Bounding boxes (if present) must be spatially adjacent: within half a tile-side
///   (`min(width, height) / 2`) of each other
/// - Dimensions must match within ±20%
/// - Emits one `info!` span per page with cluster count and max cluster size
pub fn cluster_tiles(images: &mut [ExtractedImage]) {
    if images.is_empty() {
        return;
    }

    // Group by page
    let mut by_page: HashMap<Option<u32>, Vec<usize>> = HashMap::new();
    for (idx, img) in images.iter().enumerate() {
        by_page.entry(img.page_number).or_default().push(idx);
    }

    let mut next_cluster_id = 1u32;

    // Process each page independently
    for (page_num, indices) in by_page {
        if indices.len() < 2 {
            continue; // No clustering for singletons
        }

        // Filter candidates: must be a drawable kind or unclassified with small area
        let mut candidates: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&idx| {
                let img = &images[idx];
                let is_drawable = matches!(
                    img.image_kind,
                    Some(ImageKind::Drawing | ImageKind::Diagram | ImageKind::TileFragment)
                );
                let is_unclassified_small = img.image_kind.is_none()
                    && (img.width.unwrap_or(0) as u64) * (img.height.unwrap_or(0) as u64) < (300 * 300);
                is_drawable || is_unclassified_small
            })
            .collect();

        if candidates.len() < 2 {
            continue; // Can't cluster fewer than 2
        }

        // Pre-check: do candidates have similar dimensions?
        // Collect all dimensions
        let dims: Vec<_> = candidates
            .iter()
            .map(|&idx| {
                let img = &images[idx];
                (img.width.unwrap_or(0), img.height.unwrap_or(0))
            })
            .collect();

        // Find median dimensions to establish a baseline
        let mut widths: Vec<_> = dims.iter().map(|(w, _)| *w).collect();
        let mut heights: Vec<_> = dims.iter().map(|(_, h)| *h).collect();
        widths.sort();
        heights.sort();

        let median_w = widths[widths.len() / 2] as f64;
        let median_h = heights[heights.len() / 2] as f64;

        if median_w < 1.0 || median_h < 1.0 {
            continue; // Skip if dimensions are degenerate
        }

        // Filter candidates again: keep only those within ±20% of median
        let candidates_filtered: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&idx| {
                let img = &images[idx];
                let w = img.width.unwrap_or(0) as f64;
                let h = img.height.unwrap_or(0) as f64;
                let w_ratio = w / median_w;
                let h_ratio = h / median_h;
                (0.8..=1.2).contains(&w_ratio) && (0.8..=1.2).contains(&h_ratio)
            })
            .collect();

        if candidates_filtered.len() < 2 {
            continue; // Can't cluster
        }

        candidates = candidates_filtered;

        // Build union-find for spatial clustering
        let n = candidates.len();
        let mut parent: Vec<usize> = (0..n).collect();

        // Connect spatially adjacent candidates
        for (i, idx_i) in candidates.iter().enumerate() {
            for (j, idx_j) in candidates.iter().enumerate().skip(i + 1) {
                let idx_i = *idx_i;
                let idx_j = *idx_j;
                let img_i = &images[idx_i];
                let img_j = &images[idx_j];

                let should_connect = if let (Some(bbox_i), Some(bbox_j)) = (&img_i.bounding_box, &img_j.bounding_box) {
                    // Both have bounding boxes: check spatial adjacency
                    let min_dim = (img_i.width.unwrap_or(0) as i32)
                        .min(img_i.height.unwrap_or(0) as i32)
                        .min(img_j.width.unwrap_or(0) as i32)
                        .min(img_j.height.unwrap_or(0) as i32) as f64;

                    if min_dim < 1.0 {
                        false
                    } else {
                        // Tile grids typically have near-zero gaps between tiles; allow up
                        // to half a tile of slack to absorb compression-induced jitter while
                        // still separating logically-distinct figures on the same page.
                        let threshold = min_dim / 2.0;
                        let dx = (bbox_i.x0.max(bbox_j.x0) - bbox_i.x1.min(bbox_j.x1)).max(0.0);
                        let dy = (bbox_i.y0.max(bbox_j.y0) - bbox_i.y1.min(bbox_j.y1)).max(0.0);
                        let dist = (dx * dx + dy * dy).sqrt();
                        dist <= threshold
                    }
                } else {
                    // Fallback when bounding boxes are unavailable (lopdf path):
                    // tiles in tile-grids tend to be emitted in a contiguous run
                    // of image_index values. Cap proximity at a small window so
                    // dozens of bbox-less images on a page don't all merge.
                    const NO_BBOX_INDEX_WINDOW: i32 = 3;
                    (idx_i as i32 - idx_j as i32).abs() <= NO_BBOX_INDEX_WINDOW
                };

                if should_connect {
                    uf_union(&mut parent, i, j);
                }
            }
        }

        // Group by connected component
        let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, idx_i) in candidates.iter().enumerate() {
            let root = uf_find(&mut parent, i);
            clusters.entry(root).or_default().push(*idx_i);
        }

        // Assign cluster_id to clusters with 2+ members. Sort by smallest member
        // index so cluster IDs are deterministic and match document reading order.
        let mut cluster_count = 0;
        let mut max_cluster_size = 0;
        let mut multi_clusters: Vec<Vec<usize>> = clusters.into_values().filter(|cluster| cluster.len() >= 2).collect();
        for cluster in &mut multi_clusters {
            cluster.sort_unstable();
        }
        multi_clusters.sort_by_key(|cluster| cluster[0]);
        for cluster in multi_clusters {
            cluster_count += 1;
            max_cluster_size = max_cluster_size.max(cluster.len());
            for idx in cluster {
                images[idx].cluster_id = Some(next_cluster_id);
                if matches!(images[idx].image_kind, Some(ImageKind::Drawing | ImageKind::Diagram)) {
                    images[idx].image_kind = Some(ImageKind::TileFragment);
                }
            }
            // Saturating to keep cluster_id well-defined even on the
            // physically-impossible 4 -billion-cluster input.
            next_cluster_id = next_cluster_id.saturating_add(1);
        }

        // Emit info span
        if cluster_count > 0 {
            tracing::info!(
                target: "kreuzberg::image_kind",
                page = ?page_num,
                cluster_count,
                max_cluster_size,
                "clustered tile fragments"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;
    use image::Rgba;

    #[test]
    fn test_classify_returns_mask_for_is_mask_true() {
        let (kind, conf) = classify(&[], "jpeg", Some(100), Some(100), None, None, true);
        assert_eq!(kind, ImageKind::Mask);
        assert_eq!(conf, 0.95);
    }

    #[test]
    fn test_classify_returns_icon_for_small_square() {
        let (kind, conf) = classify(&[], "png", Some(48), Some(48), None, None, false);
        assert_eq!(kind, ImageKind::Icon);
        assert_eq!(conf, 0.85);
    }

    #[test]
    fn test_classify_returns_decoration_for_tiny_strip() {
        let (kind, conf) = classify(&[], "png", Some(10), Some(100), None, None, false);
        assert_eq!(kind, ImageKind::Decoration);
        assert_eq!(conf, 0.80);
    }

    #[test]
    fn test_classify_returns_textblock_for_gray_1bpp() {
        let (kind, conf) = classify(&[], "png", Some(200), Some(200), Some("Gray"), Some(1), false);
        assert_eq!(kind, ImageKind::TextBlock);
        assert_eq!(conf, 0.75);
    }

    #[test]
    fn test_classify_returns_photograph_for_cmyk_8bpp() {
        let (kind, conf) = classify(&[], "jpeg", Some(800), Some(800), Some("CMYK"), Some(8), false);
        assert_eq!(kind, ImageKind::Photograph);
        assert_eq!(conf, 0.70);
    }

    #[test]
    fn test_classify_returns_photograph_for_large_jpeg() {
        let (kind, conf) = classify(&[], "jpeg", Some(1000), Some(1000), None, None, false);
        assert_eq!(kind, ImageKind::Photograph);
        assert_eq!(conf, 0.85);
    }

    #[test]
    fn test_classify_returns_diagram_for_flate_indexed() {
        let (kind, conf) = classify(&[], "flate", Some(200), Some(200), Some("Indexed"), None, false);
        assert_eq!(kind, ImageKind::Diagram);
        assert_eq!(conf, 0.65);
    }

    #[test]
    fn test_classify_returns_mask_for_ccitt() {
        let (kind, conf) = classify(&[], "ccitt", Some(200), Some(200), None, None, false);
        assert_eq!(kind, ImageKind::Mask);
        assert_eq!(conf, 0.85);
    }

    #[test]
    fn test_classify_returns_photograph_for_high_entropy_thumbnail() {
        let mut state: u32 = 0x9E37_79B9;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state & 0xFF) as u8
        };
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(100, 100, |_x, _y| Rgba([next(), next(), next(), 255]));

        let mut bytes = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();

        let (kind, conf) = classify(&bytes, "png", Some(100), Some(100), None, None, false);
        assert_eq!(kind, ImageKind::Photograph);
        assert!(conf >= 0.6, "confidence {} should be >= 0.6", conf);
    }

    #[test]
    fn test_classify_returns_chart_for_low_entropy_small_image() {
        // Create a 2-color PNG (low entropy): half red, half blue
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(256, 256, |x, _y| {
            if x < 128 {
                Rgba([255, 0, 0, 255]) // Red
            } else {
                Rgba([0, 0, 255, 255]) // Blue
            }
        });

        let mut bytes = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();

        let (kind, conf) = classify(&bytes, "png", Some(256), Some(256), None, None, false);
        assert_eq!(kind, ImageKind::Chart);
        assert!(conf >= 0.55, "confidence {} should be >= 0.55", conf);
    }

    #[test]
    fn test_classify_returns_unknown_for_truncated_bytes() {
        let truncated = vec![0x89, 0x50, 0x4E, 0x47]; // Partial PNG magic
        let (kind, conf) = classify(&truncated, "png", Some(100), Some(100), None, None, false);
        assert_eq!(kind, ImageKind::Unknown);
        assert_eq!(conf, 0.50);
    }

    #[test]
    fn test_classify_never_panics_on_garbage_input() {
        // Test with various edge cases and garbage input
        let test_cases = vec![
            (&[][..], "unknown", Some(0u32), Some(0u32), None, None, false),
            (
                b"garbage",
                "jpeg",
                Some(1u32),
                Some(1u32),
                Some("RGB"),
                Some(8u32),
                false,
            ),
            (
                b"\xFF\xD8\xFF\xFF",
                "jpeg",
                Some(10000u32),
                Some(10000u32),
                None,
                None,
                false,
            ),
            (b"\x89PNG\r\n\x1a\n", "png", Some(0u32), Some(0u32), None, None, false),
            (
                b"",
                "unknown",
                Some(65536u32),
                Some(65536u32),
                Some("CMYK"),
                Some(16u32),
                true,
            ),
        ];

        for (bytes, fmt, w, h, cs, bpc, is_mask) in test_cases {
            // Should not panic
            let _ = classify(bytes, fmt, w, h, cs, bpc, is_mask);
        }
    }

    #[test]
    fn test_cluster_tiles_groups_adjacent_similar_tiles() {
        let mut images = vec![
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 0,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 100.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 1,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 101.0,
                    y0: 0.0,
                    x1: 201.0,
                    y1: 100.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
        ];

        cluster_tiles(&mut images);

        assert_eq!(images[0].cluster_id, Some(1));
        assert_eq!(images[1].cluster_id, Some(1));
        assert_eq!(images[0].image_kind, Some(ImageKind::TileFragment));
        assert_eq!(images[1].image_kind, Some(ImageKind::TileFragment));
    }

    #[test]
    fn test_cluster_tiles_keeps_singletons_unclustered() {
        let mut images = vec![ExtractedImage {
            data: bytes::Bytes::new(),
            format: "png".into(),
            image_index: 0,
            page_number: Some(1),
            width: Some(100),
            height: Some(100),
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
            image_kind: Some(ImageKind::Photograph),
            kind_confidence: Some(0.8),
            cluster_id: None,
        }];

        cluster_tiles(&mut images);

        assert_eq!(images[0].cluster_id, None);
        assert_eq!(images[0].image_kind, Some(ImageKind::Photograph));
    }

    #[test]
    fn test_cluster_tiles_separates_distant_tiles() {
        let mut images = vec![
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 0,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 100.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 1,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 500.0,
                    y0: 500.0,
                    x1: 600.0,
                    y1: 600.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
        ];

        cluster_tiles(&mut images);

        assert_eq!(images[0].cluster_id, None);
        assert_eq!(images[1].cluster_id, None);
    }

    #[test]
    fn test_cluster_tiles_separates_dissimilar_kinds() {
        let mut images = vec![
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 0,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: Some(ImageKind::Photograph),
                kind_confidence: Some(0.8),
                cluster_id: None,
            },
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 1,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: Some(ImageKind::Photograph),
                kind_confidence: Some(0.8),
                cluster_id: None,
            },
        ];

        cluster_tiles(&mut images);

        assert_eq!(images[0].cluster_id, None);
        assert_eq!(images[1].cluster_id, None);
    }

    #[test]
    fn test_cluster_tiles_falls_back_when_bounding_boxes_missing() {
        let mut images = vec![
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 0,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 1,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
        ];

        cluster_tiles(&mut images);

        // Should cluster by proximity in index + matching dimensions
        assert_eq!(images[0].cluster_id, Some(1));
        assert_eq!(images[1].cluster_id, Some(1));
    }

    #[test]
    fn test_cluster_tiles_assigns_unique_ids() {
        let mut images = vec![
            // Cluster 1
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 0,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 100.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 1,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 101.0,
                    y0: 0.0,
                    x1: 201.0,
                    y1: 100.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Drawing),
                kind_confidence: Some(0.7),
                cluster_id: None,
            },
            // Cluster 2
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 2,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 0.0,
                    y0: 200.0,
                    x1: 100.0,
                    y1: 300.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Diagram),
                kind_confidence: Some(0.65),
                cluster_id: None,
            },
            ExtractedImage {
                data: bytes::Bytes::new(),
                format: "png".into(),
                image_index: 3,
                page_number: Some(1),
                width: Some(100),
                height: Some(100),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 101.0,
                    y0: 200.0,
                    x1: 201.0,
                    y1: 300.0,
                }),
                source_path: None,
                image_kind: Some(ImageKind::Diagram),
                kind_confidence: Some(0.65),
                cluster_id: None,
            },
        ];

        cluster_tiles(&mut images);

        assert_eq!(images[0].cluster_id, Some(1));
        assert_eq!(images[1].cluster_id, Some(1));
        assert_eq!(images[2].cluster_id, Some(2));
        assert_eq!(images[3].cluster_id, Some(2));
    }

    #[test]
    fn test_cluster_tiles_is_deterministic() {
        let make_images = || {
            vec![
                ExtractedImage {
                    data: bytes::Bytes::new(),
                    format: "png".into(),
                    image_index: 0,
                    page_number: Some(1),
                    width: Some(100),
                    height: Some(100),
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description: None,
                    ocr_result: None,
                    bounding_box: None,
                    source_path: None,
                    image_kind: Some(ImageKind::Drawing),
                    kind_confidence: Some(0.7),
                    cluster_id: None,
                },
                ExtractedImage {
                    data: bytes::Bytes::new(),
                    format: "png".into(),
                    image_index: 1,
                    page_number: Some(1),
                    width: Some(100),
                    height: Some(100),
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description: None,
                    ocr_result: None,
                    bounding_box: None,
                    source_path: None,
                    image_kind: Some(ImageKind::Drawing),
                    kind_confidence: Some(0.7),
                    cluster_id: None,
                },
            ]
        };

        let mut images1 = make_images();
        let mut images2 = make_images();

        cluster_tiles(&mut images1);
        cluster_tiles(&mut images2);

        assert_eq!(images1[0].cluster_id, images2[0].cluster_id);
        assert_eq!(images1[1].cluster_id, images2[1].cluster_id);
    }

    #[test]
    fn test_classify_skips_entropy_for_oversized_image() {
        // Reported dimensions exceeding MAX_CLASSIFY_PIXELS must short-circuit
        // before image::load_from_memory allocates the full source buffer. The
        // function must never panic and must fall through to Unknown rather
        // than spending CPU/memory on a thumbnail decode for a hostile input.
        let bytes = b"\x89PNG\r\n\x1a\nbogus body".to_vec();
        let (kind, conf) = classify(&bytes, "png", Some(20_000), Some(20_000), None, None, false);
        assert_eq!(kind, ImageKind::Unknown);
        assert_eq!(conf, 0.50);
    }

    #[test]
    fn test_cluster_tiles_isolates_clusters_per_page() {
        // Identical adjacent-tile pairs on two different pages must NEVER share
        // a cluster_id — clustering is page-scoped.
        let mut images = vec![];
        for page in 1..=2 {
            for col in 0..2 {
                images.push(ExtractedImage {
                    data: bytes::Bytes::new(),
                    format: "png".into(),
                    image_index: ((page - 1) * 2 + col),
                    page_number: Some(page),
                    width: Some(100),
                    height: Some(100),
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description: None,
                    ocr_result: None,
                    bounding_box: Some(crate::types::BoundingBox {
                        x0: (col as f64) * 101.0,
                        y0: 0.0,
                        x1: (col as f64) * 101.0 + 100.0,
                        y1: 100.0,
                    }),
                    source_path: None,
                    image_kind: Some(ImageKind::Drawing),
                    kind_confidence: Some(0.7),
                    cluster_id: None,
                });
            }
        }
        cluster_tiles(&mut images);
        // Page 1 tiles share a cluster, page 2 tiles share a different cluster.
        assert!(images[0].cluster_id.is_some());
        assert_eq!(images[0].cluster_id, images[1].cluster_id);
        assert_eq!(images[2].cluster_id, images[3].cluster_id);
        assert_ne!(images[0].cluster_id, images[2].cluster_id);
    }

    #[test]
    fn test_classify_does_not_panic_on_zero_dimensions() {
        let bytes = b"\x89PNG\r\n\x1a\nbody".to_vec();
        let (kind, conf) = classify(&bytes, "png", Some(0), Some(0), None, None, false);
        // Degenerate 0×0 inputs short-circuit to Unknown with zero confidence —
        // every rule is dimension-gated and there is nothing meaningful to infer.
        assert_eq!(kind, ImageKind::Unknown);
        assert_eq!(conf, 0.0);
    }

    #[test]
    fn test_image_kind_serde_round_trips_all_variants() {
        // Pin every variant's JSON spelling so future renames are caught.
        let variants = [
            (ImageKind::Photograph, "photograph"),
            (ImageKind::Diagram, "diagram"),
            (ImageKind::Chart, "chart"),
            (ImageKind::Drawing, "drawing"),
            (ImageKind::TextBlock, "text_block"),
            (ImageKind::Decoration, "decoration"),
            (ImageKind::Logo, "logo"),
            (ImageKind::Icon, "icon"),
            (ImageKind::TileFragment, "tile_fragment"),
            (ImageKind::Mask, "mask"),
            (ImageKind::Unknown, "unknown"),
        ];
        for (kind, expected) in variants {
            let json = serde_json::to_string(&kind).expect("serialize");
            assert_eq!(json, format!("\"{expected}\""), "wrong wire name for {kind:?}");
            let round_trip: ImageKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(round_trip, kind);
        }
    }
}
