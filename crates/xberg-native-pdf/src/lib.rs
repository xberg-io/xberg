// SPDX-License-Identifier: MIT
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::explicit_counter_loop)]
#![allow(clippy::doc_overindented_list_items)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::redundant_guards)]
#![allow(clippy::regex_creation_in_loops)]
#![allow(clippy::manual_find)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::collapsible_match)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, allow(unused_variables))]

//! # xberg-native-pdf
//!
//! A PDF parsing, extraction and rendering library — the PDF engine behind
//! [`xberg`](https://github.com/xberg-io/xberg).
//!
//! **This crate began as a fork.** The upstream project it derives from ships bindings for
//! 19 languages, a CLI and an MCP server; this crate strips all of that down to the single
//! Rust library `xberg` consumes, and is not endorsed by or affiliated with upstream. The
//! upstream project, its authors and its licence are named in the repository's
//! `ATTRIBUTIONS.md`, which is the single place that record is kept.
//!
//! ## Core Features
//!
//! ### Reading & Extraction
//! - **Text Extraction**: Character, span, and page-level with font metadata and bounding boxes
//! - **Reading Order**: 4 pluggable strategies (XY-Cut, Structure Tree, Geometric, Simple)
//! - **Complex Scripts**: RTL (Arabic/Hebrew), CJK (Japanese/Korean/Chinese), Devanagari, Thai
//! - **Image Extraction**: Content streams, Form XObjects, inline images
//! - **Forms & Annotations**: Read form fields, all annotation types, bookmarks
//! - **Text Search**: Regex and case-insensitive search with page-level results
//!
//! ## Quick Start
//!
//! ```ignore
//! use xberg_native_pdf::PdfDocument;
//! use xberg_native_pdf::pipeline::{TextPipeline, TextPipelineConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a PDF
//! let mut doc = PdfDocument::open("paper.pdf")?;
//!
//! // Plain text, in reading order
//! let text = doc.extract_text(0)?;
//!
//! // Or drive the reading-order pipeline yourself (multi-column support)
//! let spans = doc.extract_spans(0)?;
//! let config = TextPipelineConfig::default();
//! let pipeline = TextPipeline::with_config(config.clone());
//! let ordered_spans = pipeline.process(spans, Default::default())?;
//! # Ok(())
//! # }
//! ```
//!
//! ## License
//!
//! MIT ([LICENSE](LICENSE) or <https://opensource.org/licenses/MIT>).
//!
//! The upstream project is offered as MIT OR Apache-2.0; this crate is
//! distributed under the MIT arm alone. Copyright in the bulk of the code
//! is held by Yury Fedoseev and the upstream contributors — see LICENSE,
//! which carries both notices, and ATTRIBUTIONS.md.

#![warn(missing_docs)]
// The crate contains no `unsafe`. The last two blocks were `mem::transmute`s to
// `'static` in the rasterizer's font cache, needed only because harfrust and
// ttf-parser each held their own borrowed parse of the same bytes. Migrating to
// fontations replaced those with a borrowed `FontRef` rebuilt per call plus an
// owned, `Arc`-backed `harfrust::Font`, so nothing has to be transmuted.
//
// `deny`, not `forbid`, for exactly ONE audited exception: the `global_asm!`
// __memcmpeq shim below, which is `unsafe` by definition and cannot be written any
// other way. `forbid` cannot be relaxed by an inner `allow`, so with `forbid` the
// crate does not compile on linux-x86_64 at all -- which is precisely how the
// v1.0.1 publish failed. Every other use of unsafe is still a hard error. ~keep
#![deny(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// The root of every `tracing` target this crate emits.
///
/// `module_path!()` at the crate root evaluates to the `[lib] name`, so this is the live
/// value rather than a copy of it and cannot drift when the crate is renamed.
///
/// It exists because consumers filter on this prefix, and the last time the name moved they
/// were matching a hardcoded string: the capture sink kept filtering on the former prefix,
/// every record was rejected, and for twelve days no warning about unparseable fonts reached
/// a caller (GH#697). Nothing failed to compile. Filter on this constant, not on a literal.
pub const LOG_TARGET_ROOT: &str = module_path!();

// Glibc 2.34 compatibility (#416): LLVM may emit calls to __memcmpeq@GLIBC_2.35,
// which does not exist in glibc 2.34 (Amazon Linux 2023, some Ubuntu 22.04 builds). ~keep

// A weak stub redirecting to plain memcmp satisfies the reference on older glibc;
// glibc 2.35's own definition wins when available. global_asm! works with both
// GNU ld and lld, unlike --defsym which lld rejects for PLT-resolved symbols. ~keep
// The single audited exception to the crate-level `deny(unsafe_code)` above:
// `global_asm!` is unsafe by definition. It emits no Rust code -- only a weak ELF
// alias -- so it cannot violate any Rust invariant.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[allow(unsafe_code)]
mod memcmpeq_shim {
    // `global_asm!` emits a global symbol regardless of the module it sits in, so
    // nesting it here to scope the `allow` changes nothing about the emitted alias.
    core::arch::global_asm!(
        ".weak __memcmpeq",
        ".type __memcmpeq, @function",
        "__memcmpeq:",
        "jmp memcmp@PLT",
    );
}

pub mod error;

pub(crate) mod cache;

/// Third-party code vendored directly into this crate rather than pulled in
/// as a Cargo dependency (see `ATTRIBUTIONS.md`). Implementation detail —
/// never part of the public API.
pub(crate) mod vendor;

pub mod document;
pub(crate) mod lexer;
pub mod object;
pub mod objstm;
pub(crate) mod parser;
/// Parser configuration options
pub mod parser_config;
pub mod xref;
pub mod xref_reconstruction;

pub mod decoders;

// PDF function evaluators (Type 4 PostScript calculator) ~keep
pub mod functions;

pub mod color;

// Pluggable cryptographic backend (FIPS / sovereign-jurisdiction
// providers). Issue #236. ~keep
pub mod crypto;

pub mod encryption;

pub mod geometry;
pub mod layout;

pub mod content;
pub mod extractors;
pub mod fonts;
pub mod optional_content;
pub mod text;

/// Core annotation types and enums per PDF spec
pub mod annotation_types;
pub mod annotations;
/// Content elements for PDF generation
pub mod elements;
pub mod outline;
/// True/destructive redaction + document sanitization (#231).
pub mod redaction;
/// PDF logical structure (Tagged PDFs)
pub mod structure;

/// Structured per-page extraction (`extract_structured`, #536)
pub mod structured;

pub mod converters;

pub mod pipeline;

pub mod fdf;

pub mod xfa;

pub mod search;

pub mod rendering;

pub mod api;

pub use pipeline::XYCutStrategy;

pub mod config;

pub use annotation_types::{
    AnnotationBorderStyle, AnnotationColor, AnnotationFlags, AnnotationSubtype, BorderEffectStyle, BorderStyleType,
    CaretSymbol, FileAttachmentIcon, FreeTextIntent, HighlightMode, LineEndingStyle, QuadPoint, ReplyType, StampType,
    TextAlignment, TextAnnotationIcon, TextMarkupType, WidgetFieldType,
};
pub use annotations::{Annotation, LinkAction, LinkDestination};
pub use config::{DocumentType, ExtractionProfile};
pub use document::{ExtractedImageRef, ImageFormat, PageRotation, PdfDocument, ReadingOrder};
pub use error::{Error, Result};
pub use extractors::images::{PdfFilter, PdfImageHandle};
pub use fonts::unicode_decode::{LOST_SYMBOL_GLYPH_MARKER, is_lost_glyph_marker};
pub use layout::PageText;
pub use outline::{Destination, OutlineItem};
pub use redaction::{
    FontInfoMetrics, OcgPolicy, RedactionOptions, RedactionRegion, RedactionReport, RegionSet, redact_content_stream,
};
pub use structured::{ColumnMode, RegionRole, StructuredPage, StructuredRegion};

pub use fonts::global_cache::{clear_global_font_cache, global_font_cache_stats, set_global_font_cache_capacity};

pub use fonts::cmap::{clear_cmap_cache, cmap_cache_size};

pub(crate) mod utils {
    //! Internal utility functions for the library.

    use std::cmp::Ordering;

    /// Safely truncate a string to at most `max_bytes` from the start
    /// without splitting a multi-byte UTF-8 character.
    ///
    /// Returns the full string if it is shorter than `max_bytes`.
    /// When truncation lands inside a multi-byte character, the boundary
    /// is rounded **down** to the nearest char boundary (floor).
    #[inline]
    pub fn safe_prefix(s: &str, max_bytes: usize) -> &str {
        if s.len() <= max_bytes {
            return s;
        }
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }

    /// Safely take the last `max_bytes` of a string without splitting
    /// a multi-byte UTF-8 character.
    ///
    /// Returns the full string if it is shorter than `max_bytes`.
    /// When the computed start offset lands inside a multi-byte character,
    /// the boundary is rounded **up** to the nearest char boundary (ceil).
    #[inline]
    pub fn safe_suffix(s: &str, max_bytes: usize) -> &str {
        if s.len() <= max_bytes {
            return s;
        }
        let start = s.len() - max_bytes;
        let mut safe_start = start;
        while safe_start < s.len() && !s.is_char_boundary(safe_start) {
            safe_start += 1;
        }
        &s[safe_start..]
    }

    /// Y-band tolerance used by `row_aware_span_cmp`.
    ///
    /// Two spans whose top-Y differs by less than this amount are treated
    /// as lying on the same row. Chosen to absorb typographic baseline
    /// jitter for 10-12pt body text and glyph-cluster offsets in CJK
    /// fonts without merging adjacent 14pt-leading lines.
    pub const ROW_BAND_TOLERANCE_PT: f32 = 3.0;

    /// Row-aware reading-order comparator for spans.
    ///
    /// Sorts primarily by "row band" (top-Y quantized to
    /// `ROW_BAND_TOLERANCE_PT`, larger Y first per PDF Spec ISO 32000-1:2008
    /// §8.3.2.3) and secondarily by X (left-to-right within a row). This
    /// keeps tabular layouts where cells in the same logical row have
    /// slightly different Y values (font-metric jitter, superscripts, CJK
    /// glyph centering) from being interleaved by a strict Y sort.
    ///
    /// Uses `i32` band keys so the ordering is a valid total order —
    /// comparing raw Y values with tolerance is non-transitive and would
    /// break `sort_by`.
    #[inline]
    pub fn row_aware_span_cmp(a_y: f32, a_x: f32, b_y: f32, b_x: f32) -> Ordering {
        // Non-finite Y (NaN/±Inf) cannot be quantized into an i32 band —
        // `as i32` saturates, collapsing distinct non-finite values into
        // the same band and reordering them unpredictably against finite
        // spans. Fall back to `safe_float_cmp` so non-finite values follow
        // the same NaN-last / total-order policy used everywhere else. ~keep
        if !a_y.is_finite() || !b_y.is_finite() {
            return safe_float_cmp(b_y, a_y).then_with(|| safe_float_cmp(a_x, b_x));
        }
        let band_a = (a_y / ROW_BAND_TOLERANCE_PT).round() as i32;
        let band_b = (b_y / ROW_BAND_TOLERANCE_PT).round() as i32;
        match band_b.cmp(&band_a) {
            Ordering::Equal => safe_float_cmp(a_x, b_x),
            other => other,
        }
    }

    /// Dominant text-matrix rotation of a page's spans, if any.
    ///
    /// Returns the snapped rotation (`90` / `180` / `-90`) shared by at
    /// least half of the page's non-whitespace spans, or `None` when the
    /// page is predominantly upright (or empty). The half-or-more majority
    /// mirrors the existing vertical-CJK (tategaki) vote: at most one
    /// rotation group can dominate, and a marginal stamp or figure label
    /// can never hijack the page frame. Rotations are grouped with the same
    /// 0.5° tolerance `order_rotated_blocks` uses, so free-angle (skewed)
    /// text never forms a quadrant group.
    pub(crate) fn dominant_rotation(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        let mut groups: Vec<(f32, usize)> = Vec::new();
        let mut total = 0usize;
        for s in spans {
            if s.text.trim().is_empty() {
                continue;
            }
            total += 1;
            if s.rotation_degrees == 0.0 {
                continue;
            }
            match groups.iter_mut().find(|(k, _)| (*k - s.rotation_degrees).abs() < 0.5) {
                Some(g) => g.1 += 1,
                None => groups.push((s.rotation_degrees, 1)),
            }
        }
        groups
            .into_iter()
            .max_by_key(|&(_, n)| n)
            .filter(|&(_, n)| n * 2 >= total && total > 0)
            .map(|(deg, _)| deg)
    }

    /// Right-to-left variant of [`row_aware_span_cmp`] (issues #656/#657).
    ///
    /// Identical row banding (lines top-to-bottom), but orders spans
    /// **right-to-left within a row** (X descending). A pure-RTL line's
    /// logical reading order *is* its rightmost-first geometric order, so
    /// sorting word-spans by descending X reconstructs logical order
    /// directly from page geometry — independent of whether the producer
    /// stored the run in visual or logical order. Used by the tagged
    /// struct-tree assemblers, which otherwise have no span-order pass for
    /// RTL (the untagged `reverse_rtl_visual_order_runs` is never reached
    /// on tagged pages).
    ///
    /// Retained as a tested geometric utility: the tagged RTL assembler now
    /// orders pure-RTL spans via `document::PdfDocument::order_pure_rtl_spans`
    /// (font-relative line grouping), which subsumes the fixed-band comparator,
    /// so this has no production caller at present.
    #[inline]
    #[allow(dead_code)]
    pub fn row_aware_span_cmp_rtl(a_y: f32, a_x: f32, b_y: f32, b_x: f32) -> Ordering {
        if !a_y.is_finite() || !b_y.is_finite() {
            return safe_float_cmp(b_y, a_y).then_with(|| safe_float_cmp(b_x, a_x));
        }
        let band_a = (a_y / ROW_BAND_TOLERANCE_PT).round() as i32;
        let band_b = (b_y / ROW_BAND_TOLERANCE_PT).round() as i32;
        match band_b.cmp(&band_a) {
            Ordering::Equal => safe_float_cmp(b_x, a_x),
            other => other,
        }
    }

    /// Sort spans into tategaki (vertical-writing) reading order:
    /// right-to-left across columns, top-to-bottom within each column (PDF
    /// user-space Y increases upward, so top-first means Y descending).
    ///
    /// Columns are found by single-linkage clustering of X-centers: order
    /// the centers right-to-left, then start a new column whenever the gap
    /// to the previous center exceeds `tol` (the median span width —
    /// tategaki CJK body text is functionally monospaced, so this
    /// approximates the column pitch: wide enough to keep one column
    /// together, narrow enough to separate the next).
    ///
    /// Comparing raw X-centers against a `|a - b| <= tol` tolerance
    /// *inside* a sort comparator is not transitive — a chain of spans
    /// each within `tol` of its neighbor can span far more than `tol`
    /// overall, so "same column" isn't an equivalence relation and
    /// `sort_by` can panic with "does not correctly implement a total
    /// order". Clustering into columns first and sorting by `(column, Y)`
    /// avoids this: every comparison is between two discrete, precomputed
    /// keys, which is transitive by construction. It's also more accurate
    /// than quantizing each X-center into a fixed-size band independently
    /// (e.g. `round(x / tol)`) — banding can split two spans that are only
    /// a couple points apart into different buckets if they straddle a
    /// bucket boundary, even though they're well within `tol` of each
    /// other; single-linkage clustering only looks at the gap between
    /// neighbors, so it has no such boundary effect.
    pub fn sort_vertical_tategaki<T>(items: Vec<T>, get_bbox: impl Fn(&T) -> &crate::geometry::Rect) -> Vec<T> {
        if items.len() < 2 {
            return items;
        }

        let mut widths: Vec<f32> = items.iter().map(|it| get_bbox(it).width.max(1.0)).collect();
        widths.sort_by(|a, b| safe_float_cmp(*a, *b));
        let tol = widths[widths.len() / 2].max(1.0);

        let centers: Vec<f32> = items
            .iter()
            .map(|it| {
                let b = get_bbox(it);
                b.x + b.width * 0.5
            })
            .collect();
        let ys: Vec<f32> = items.iter().map(|it| get_bbox(it).y).collect();

        // Right-to-left pass assigning column ids. Stable sort keeps ties
        // in input order, so clustering is deterministic. ~keep
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by(|&a, &b| safe_float_cmp(centers[b], centers[a]));

        let mut column = vec![0u32; items.len()];
        let mut current = 0u32;
        let mut prev = centers[order[0]];
        for &idx in &order[1..] {
            let center = centers[idx];
            // A NaN gap (either end non-finite) never chains, so a
            // non-finite center always starts its own column. ~keep
            let gap = prev - center;
            if gap.is_nan() || gap > tol {
                current += 1;
            }
            column[idx] = current;
            prev = center;
        }

        // Column ascending (columns were numbered right-to-left above),
        // then top-to-bottom within a column. Both keys are total orders. ~keep
        order.sort_by(|&a, &b| column[a].cmp(&column[b]).then_with(|| safe_float_cmp(ys[b], ys[a])));

        let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();
        order
            .into_iter()
            .map(|i| slots[i].take().expect("each index appears once"))
            .collect()
    }

    /// Safely compare two floating point numbers, handling NaN cases.
    ///
    /// NaN values are treated as equal to each other and greater than all other values.
    /// This ensures that sorting operations never panic due to NaN comparisons.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use std::cmp::Ordering;
    /// # use xberg_native_pdf::utils::safe_float_cmp;
    /// assert_eq!(safe_float_cmp(1.0, 2.0), Ordering::Less);
    /// assert_eq!(safe_float_cmp(2.0, 1.0), Ordering::Greater);
    /// assert_eq!(safe_float_cmp(1.0, 1.0), Ordering::Equal);
    ///
    /// // NaN handling
    /// assert_eq!(safe_float_cmp(f32::NAN, f32::NAN), Ordering::Equal);
    /// assert_eq!(safe_float_cmp(f32::NAN, 1.0), Ordering::Greater);
    /// assert_eq!(safe_float_cmp(1.0, f32::NAN), Ordering::Less);
    /// ```
    #[inline]
    pub fn safe_float_cmp(a: f32, b: f32) -> Ordering {
        match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => a.partial_cmp(&b).unwrap(),
        }
    }

    /// Sort `items` into row-band reading order, computing each element's band
    /// key once instead of re-quantizing on every `row_aware_span_cmp`
    /// comparison.
    ///
    /// When all `y`/`x` are finite this is a cached-key stable sort with the
    /// same order as `sort_by(row_aware_span_cmp)` (band descending, then `x`
    /// ascending — `f32::total_cmp` equals `safe_float_cmp` for finite values,
    /// and both are stable on ties). Otherwise it falls back to the comparator
    /// so the NaN/±∞ policy is unchanged.
    pub fn sort_by_row_band<T>(items: &mut [T], get_y: impl Fn(&T) -> f32, get_x: impl Fn(&T) -> f32) {
        let all_finite = items.iter().all(|it| get_y(it).is_finite() && get_x(it).is_finite());
        if !all_finite {
            items.sort_by(|a, b| row_aware_span_cmp(get_y(a), get_x(a), get_y(b), get_x(b)));
            return;
        }
        items.sort_by_cached_key(|it| {
            let band = (get_y(it) / ROW_BAND_TOLERANCE_PT).round() as i32;
            // Reverse band → larger Y (higher on page) first, matching the
            // comparator's `band_b.cmp(&band_a)`. ~keep
            (std::cmp::Reverse(band), F32Ord(get_x(it)))
        });
    }

    /// Sort into visual rows without making row membership depend on an
    /// absolute coordinate-grid boundary.
    ///
    /// Items are first ordered strictly top-to-bottom. Each row is then swept
    /// from a fixed topmost anchor and sorted left-to-right. The tolerance is
    /// proportional to the largest text scale seen in that row, bounded so it
    /// absorbs baseline jitter without chaining adjacent lines together.
    pub fn sort_by_visual_rows<T>(
        items: &mut [T],
        get_y: impl Fn(&T) -> f32,
        get_x: impl Fn(&T) -> f32,
        get_scale: impl Fn(&T) -> f32,
    ) {
        const ROW_TOLERANCE_SCALE_FACTOR: f32 = 0.25;

        let all_finite = items
            .iter()
            .all(|item| get_y(item).is_finite() && get_x(item).is_finite() && get_scale(item).is_finite());
        if !all_finite {
            items.sort_by(|a, b| row_aware_span_cmp(get_y(a), get_x(a), get_y(b), get_x(b)));
            return;
        }

        items.sort_by(|a, b| {
            get_y(b)
                .total_cmp(&get_y(a))
                .then_with(|| get_x(a).total_cmp(&get_x(b)))
        });

        let mut row_start = 0;
        while row_start < items.len() {
            let anchor_y = get_y(&items[row_start]);
            let mut row_scale = get_scale(&items[row_start]).abs();
            let mut row_end = row_start + 1;
            while row_end < items.len() {
                let candidate_scale = get_scale(&items[row_end]).abs();
                let tolerance =
                    (row_scale.max(candidate_scale) * ROW_TOLERANCE_SCALE_FACTOR).min(ROW_BAND_TOLERANCE_PT);
                if anchor_y - get_y(&items[row_end]) > tolerance {
                    break;
                }
                row_scale = row_scale.max(candidate_scale);
                row_end += 1;
            }

            items[row_start..row_end].sort_by(|a, b| get_x(a).total_cmp(&get_x(b)));
            row_start = row_end;
        }
    }

    /// Total-order wrapper over `f32` for use as a sort key. For finite values
    /// `total_cmp` is identical to `safe_float_cmp` / `partial_cmp`.
    #[derive(Clone, Copy, PartialEq)]
    struct F32Ord(f32);
    impl Eq for F32Ord {}
    impl PartialOrd for F32Ord {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for F32Ord {
        fn cmp(&self, other: &Self) -> Ordering {
            self.0.total_cmp(&other.0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The cached-key sort must produce the identical permutation to
        /// `sort_by(row_aware_span_cmp)` on finite inputs.
        #[test]
        fn test_sort_by_row_band_matches_comparator() {
            let raw: Vec<(f32, f32)> = (0..500)
                .map(|i| {
                    let y = ((i * 37 % 113) as f32) * 1.3;
                    let x = ((i * 71 % 97) as f32) * 2.1;
                    (y, x)
                })
                .collect();
            let mut a = raw.clone();
            let mut b = raw.clone();
            sort_by_row_band(&mut a, |t| t.0, |t| t.1);
            b.sort_by(|p, q| row_aware_span_cmp(p.0, p.1, q.0, q.1));
            assert_eq!(a, b, "cached-key sort must match the comparator permutation");
        }

        #[test]
        fn issue_1560_visual_rows_ignore_absolute_band_boundaries() {
            let mut spans = [
                (623.481_f32, 68.279_f32, 8.6_f32, "article"),
                (622.401, 50.0, 8.6, "position"),
                (622.401, 124.9, 8.6, "description"),
            ];

            sort_by_visual_rows(&mut spans, |span| span.0, |span| span.1, |span| span.2);

            assert_eq!(spans.map(|span| span.3), ["position", "article", "description"]);
        }

        #[test]
        fn issue_1560_visual_row_sort_is_translation_invariant() {
            let source = [
                (623.481_f32, 68.279_f32, 8.6_f32, "article"),
                (622.401, 50.0, 8.6, "position"),
                (610.0, 50.0, 8.6, "next-row"),
            ];
            let mut original = source;
            let mut translated = source.map(|(y, x, scale, id)| (y + 1.6, x, scale, id));

            sort_by_visual_rows(&mut original, |span| span.0, |span| span.1, |span| span.2);
            sort_by_visual_rows(&mut translated, |span| span.0, |span| span.1, |span| span.2);

            assert_eq!(
                original.map(|span| span.3),
                translated.map(|span| span.3),
                "shifting a page across the old 3pt bands must not change reading order"
            );
            assert_eq!(original.map(|span| span.3), ["position", "article", "next-row"]);
        }

        #[test]
        fn issue_1560_visual_row_tolerance_scales_down_for_tiny_text() {
            let mut spans = [(10.0_f32, 90.0_f32, 1.0_f32, "top"), (9.7, 10.0, 1.0, "bottom")];

            sort_by_visual_rows(&mut spans, |span| span.0, |span| span.1, |span| span.2);

            assert_eq!(spans.map(|span| span.3), ["top", "bottom"]);
        }

        #[test]
        fn test_safe_float_cmp_normal() {
            assert_eq!(safe_float_cmp(1.0, 2.0), Ordering::Less);
            assert_eq!(safe_float_cmp(2.0, 1.0), Ordering::Greater);
            assert_eq!(safe_float_cmp(1.5, 1.5), Ordering::Equal);
        }

        #[test]
        fn test_safe_float_cmp_nan() {
            assert_eq!(safe_float_cmp(f32::NAN, f32::NAN), Ordering::Equal);
            assert_eq!(safe_float_cmp(f32::NAN, 0.0), Ordering::Greater);
            assert_eq!(safe_float_cmp(0.0, f32::NAN), Ordering::Less);
        }

        fn tategaki_rect(x: f32, y: f32, w: f32) -> crate::geometry::Rect {
            crate::geometry::Rect::new(x, y, w, 12.0)
        }

        /// Two well-separated columns: rightmost column first, top-to-bottom
        /// within each (the ordering the pre-fix comparator also produced
        /// for the well-behaved case — this must not regress).
        #[test]
        fn test_sort_vertical_tategaki_two_columns() {
            let items = vec![
                ("D", tategaki_rect(300.0, 700.0, 12.0)),
                ("F", tategaki_rect(300.0, 676.0, 12.0)),
                ("B", tategaki_rect(500.0, 688.0, 12.0)),
                ("C", tategaki_rect(500.0, 676.0, 12.0)),
                ("A", tategaki_rect(500.0, 700.0, 12.0)),
                ("E", tategaki_rect(300.0, 688.0, 12.0)),
            ];
            let sorted = sort_vertical_tategaki(items, |it| &it.1);
            let order: String = sorted.iter().map(|it| it.0).collect();
            assert_eq!(order, "ABCDEF");
        }

        /// A chain of X-centers each within `tol` of its neighbor but
        /// spanning far more than `tol` overall made the old pairwise
        /// `|a - b| <= tol` comparator non-transitive (A<B, B<C, C<A),
        /// which panicked `sort_by` on Rust 1.81+. Single-linkage
        /// clustering must read the whole chain as one column, top to
        /// bottom, without panicking.
        #[test]
        fn test_sort_vertical_tategaki_chained_centers() {
            let items: Vec<(usize, crate::geometry::Rect)> = (0..64)
                .map(|i| (i, tategaki_rect(i as f32 * 8.0, ((i * 37) % 64) as f32 * 7.0, 10.0)))
                .collect();
            let sorted = sort_vertical_tategaki(items, |it| &it.1);
            assert_eq!(sorted.len(), 64);
            assert!(
                sorted.windows(2).all(|w| w[0].1.y >= w[1].1.y),
                "one chained column must read top-to-bottom"
            );
        }

        /// Two spans only 2pt apart (well within `tol`) must land in the
        /// same column even when their absolute X-centers straddle what
        /// would be a fixed quantization-bucket boundary (e.g. `tol`
        /// multiples of 100 straddling x=250). Single-linkage clustering
        /// only looks at the gap between neighbors, so it has no such
        /// boundary effect — unlike banding each center independently via
        /// `round(x / tol)`.
        #[test]
        fn test_sort_vertical_tategaki_no_boundary_straddle_effect() {
            let items = vec![
                ("near", tategaki_rect(249.0, 700.0, 100.0)),
                ("straddle", tategaki_rect(251.0, 690.0, 100.0)),
                ("far", tategaki_rect(10.0, 680.0, 100.0)),
            ];
            let sorted = sort_vertical_tategaki(items, |it| &it.1);
            let order: Vec<&str> = sorted.iter().map(|it| it.0).collect();
            assert_eq!(order, vec!["near", "straddle", "far"]);
        }

        /// Non-finite coordinates must not panic the sort, and every item
        /// must survive the permutation exactly once.
        #[test]
        fn test_sort_vertical_tategaki_non_finite() {
            let mut items: Vec<(usize, crate::geometry::Rect)> = (0..32)
                .map(|i| (i, tategaki_rect((i % 8) as f32 * 40.0, i as f32 * 5.0, 12.0)))
                .collect();
            items[3].1.x = f32::NAN;
            items[11].1.y = f32::NAN;
            items[17].1.width = f32::NAN;
            items[23].1.x = f32::INFINITY;
            let sorted = sort_vertical_tategaki(items, |it| &it.1);
            let mut ids: Vec<usize> = sorted.iter().map(|it| it.0).collect();
            ids.sort_unstable();
            assert_eq!(ids, (0..32).collect::<Vec<_>>());
        }

        #[test]
        fn test_safe_float_cmp_infinity() {
            assert_eq!(safe_float_cmp(f32::INFINITY, f32::INFINITY), Ordering::Equal);
            assert_eq!(safe_float_cmp(f32::INFINITY, 1.0), Ordering::Greater);
            assert_eq!(safe_float_cmp(f32::NEG_INFINITY, f32::INFINITY), Ordering::Less);
        }

        /// Verify that sort_by using safe_float_cmp never panics with NaN values.
        /// This is a regression test for the "total order" panic that affected 42
        /// PDFs across 5 test datasets (issue found in v0.3.11-pre).
        #[test]
        fn test_sort_with_nan_does_not_panic() {
            let mut values = [3.0_f32, f32::NAN, 1.0, f32::NAN, 2.0, f32::NAN, 0.5];
            values.sort_by(|a, b| safe_float_cmp(*a, *b));
            assert!(values[0..4].iter().all(|v| !v.is_nan()));
            assert!(values[4..].iter().all(|v| v.is_nan()));
        }

        /// Verify transitivity: if a < b and b < c then a < c.
        /// The previous `partial_cmp().unwrap_or(Equal)` pattern violated this
        /// when NaN was involved, causing Rust's sort to panic.
        #[test]
        fn test_safe_float_cmp_transitivity() {
            let a = 1.0_f32;
            let b = 2.0_f32;
            let nan = f32::NAN;

            assert_eq!(safe_float_cmp(a, b), Ordering::Less);
            assert_eq!(safe_float_cmp(b, nan), Ordering::Less);
            assert_eq!(safe_float_cmp(a, nan), Ordering::Less);
        }

        /// Cells in the same tabular row with slightly-different Y values
        /// must stay together and be ordered by X, not interleaved with
        /// cells from other rows.
        #[test]
        fn test_row_aware_span_cmp_tolerates_y_jitter() {
            // Row 1 at y ≈ 100 with small per-cell jitter.
            // Row 2 at y ≈ 86 (14pt leading below).
            // A strict Y sort would interleave them because some row-1
            // cells have lower Y than some row-2 cells. ~keep
            #[derive(Debug, Clone, Copy)]
            struct Cell {
                y: f32,
                x: f32,
                id: &'static str,
            }
            let mut cells = [
                Cell {
                    y: 100.5,
                    x: 50.0,
                    id: "r1-c1",
                },
                Cell {
                    y: 99.7,
                    x: 150.0,
                    id: "r1-c2",
                },
                Cell {
                    y: 100.2,
                    x: 250.0,
                    id: "r1-c3",
                },
                Cell {
                    y: 86.4,
                    x: 50.0,
                    id: "r2-c1",
                },
                Cell {
                    y: 85.8,
                    x: 150.0,
                    id: "r2-c2",
                },
                Cell {
                    y: 86.1,
                    x: 250.0,
                    id: "r2-c3",
                },
            ];
            cells.sort_by(|a, b| row_aware_span_cmp(a.y, a.x, b.y, b.x));
            let order: Vec<&str> = cells.iter().map(|c| c.id).collect();
            assert_eq!(
                order,
                vec!["r1-c1", "r1-c2", "r1-c3", "r2-c1", "r2-c2", "r2-c3"],
                "cells from the same row must stay contiguous and X-sorted"
            );
        }

        /// Row-aware comparator must still put distinct-leading rows in
        /// top-to-bottom reading order.
        #[test]
        fn test_row_aware_span_cmp_distinct_rows_descending() {
            let mut rows = [(100.0f32, 0.0f32, "top"), (50.0, 0.0, "middle"), (10.0, 0.0, "bottom")];
            rows.sort_by(|a, b| row_aware_span_cmp(a.0, a.1, b.0, b.1));
            assert_eq!(rows[0].2, "top");
            assert_eq!(rows[1].2, "middle");
            assert_eq!(rows[2].2, "bottom");
        }

        /// The comparator is used by sort_by, which requires a valid total
        /// order. Run a randomized stress test to confirm no transitivity
        /// panics.
        #[test]
        fn test_row_aware_span_cmp_is_total_order() {
            let mut v: Vec<(f32, f32)> = (0..200).map(|i| ((i as f32) * 0.73, ((i * 17) % 500) as f32)).collect();
            v.sort_by(|a, b| row_aware_span_cmp(a.0, a.1, b.0, b.1));
        }

        /// #656/#657: the RTL variant keeps rows top-to-bottom but orders
        /// X *descending* (right-to-left) within a row — a pure-RTL line's
        /// logical reading order.
        #[test]
        fn test_row_aware_span_cmp_rtl_within_row_is_descending() {
            let mut row = [
                (100.0f32, 10.0f32, "leftmost"),
                (100.0, 50.0, "mid"),
                (100.0, 90.0, "rightmost"),
            ];
            row.sort_by(|a, b| row_aware_span_cmp_rtl(a.0, a.1, b.0, b.1));
            assert_eq!(["rightmost", "mid", "leftmost"], [row[0].2, row[1].2, row[2].2]);
        }

        /// Rows still order top-to-bottom regardless of the within-row flip.
        #[test]
        fn test_row_aware_span_cmp_rtl_rows_top_to_bottom() {
            let mut rows = [(10.0f32, 0.0f32, "bottom"), (100.0, 0.0, "top"), (50.0, 0.0, "middle")];
            rows.sort_by(|a, b| row_aware_span_cmp_rtl(a.0, a.1, b.0, b.1));
            assert_eq!(["top", "middle", "bottom"], [rows[0].2, rows[1].2, rows[2].2]);
        }

        /// Must be a valid total order for `sort_by` (no transitivity panic).
        #[test]
        fn test_row_aware_span_cmp_rtl_is_total_order() {
            let mut v: Vec<(f32, f32)> = (0..200).map(|i| ((i as f32) * 0.73, ((i * 17) % 500) as f32)).collect();
            v.sort_by(|a, b| row_aware_span_cmp_rtl(a.0, a.1, b.0, b.1));
        }

        /// Sort a large array with mixed NaN/normal values to stress-test.
        #[test]
        fn test_sort_stress_with_nan() {
            let mut values: Vec<f32> = (0..100).map(|i| i as f32).collect();
            for i in (0..100).step_by(7) {
                values[i] = f32::NAN;
            }
            values.sort_by(|a, b| safe_float_cmp(*a, *b));
        }

        #[test]
        fn test_safe_prefix_ascii() {
            assert_eq!(safe_prefix("hello", 3), "hel");
            assert_eq!(safe_prefix("hello", 10), "hello");
            assert_eq!(safe_prefix("", 5), "");
            assert_eq!(safe_prefix("hi", 0), "");
        }

        #[test]
        fn test_safe_prefix_multibyte() {
            let text = "✚✳★✵";
            assert_eq!(safe_prefix(text, 10), "✚✳★");
            assert_eq!(safe_prefix(text, 9), "✚✳★");
            assert_eq!(safe_prefix(text, 12), "✚✳★✵");
        }

        #[test]
        fn test_safe_suffix_ascii() {
            assert_eq!(safe_suffix("hello", 3), "llo");
            assert_eq!(safe_suffix("hello", 10), "hello");
            assert_eq!(safe_suffix("", 5), "");
            assert_eq!(safe_suffix("hi", 0), "");
        }

        #[test]
        fn test_safe_suffix_multibyte() {
            let text = "AB✚✳★✵";
            // 14 - 10 = 4, byte 4 is inside ✚ → rounds up to 5 ~keep
            assert_eq!(safe_suffix(text, 10), "✳★✵");
        }
    }
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "VERSION must be three-part semver, got {VERSION}");
        assert!(
            parts
                .iter()
                .all(|p| p.chars().next().is_some_and(|c| c.is_ascii_digit())),
            "every VERSION component must start with a digit, got {VERSION}"
        );
    }

    #[test]
    fn test_name() {
        assert_eq!(NAME, "xberg-native-pdf");
    }
}
