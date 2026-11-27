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
// One of two audited exceptions to the crate-level `deny(unsafe_code)` above (the other is
// `release_memory::macos`, one FFI call to the system allocator): `global_asm!` is unsafe by
// definition. It emits no Rust code -- only a weak ELF alias -- so it cannot violate any Rust
// invariant.
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

// Public only for the xberg crate, which calls it between extraction phases. ~keep
#[doc(hidden)]
pub mod release_memory;

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
pub use annotations::{Annotation, LinkAction, LinkDestination, MeasureDictionary, PageViewport, PolygonAnnotation};
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
    pub fn row_band_then_x(a_y: f32, a_x: f32, b_y: f32, b_x: f32) -> Ordering {
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
        band_b.cmp(&band_a).then_with(|| safe_float_cmp(a_x, b_x))
    }

    /// Row band descending, then `x` ascending, then the baseline descending.
    ///
    /// Without the third key two spans sharing a band and an `x` compare
    /// `Equal` and `sort_by` stability decides, which is the incoming order
    /// rather than reading order. The baseline key is only meaningful for
    /// baselines the page actually draws; a caller ordering on a synthetic key
    /// must use [`row_band_then_x`] and break ties on the real geometry.
    #[inline]
    pub fn row_aware_span_cmp(a_y: f32, a_x: f32, b_y: f32, b_x: f32) -> Ordering {
        row_band_then_x(a_y, a_x, b_y, b_x).then_with(|| safe_float_cmp(b_y, a_y))
    }

    /// Writing-axis quadrant, then row band, then `x`, with no baseline
    /// tiebreak.
    ///
    /// Row banding compares baselines along page-y and orders within a band
    /// along page-x, which only means something for runs sharing a writing
    /// axis (ISO 32000-1:2008 §9.4.4). Pairs with a tiebreak on the baseline
    /// the page draws when the row key is shared between spans.
    #[inline]
    #[allow(dead_code)]
    pub fn row_band_then_x_axis(a_rot: f32, a_y: f32, a_x: f32, b_rot: f32, b_y: f32, b_x: f32) -> Ordering {
        quadrant_key(a_rot)
            .cmp(&quadrant_key(b_rot))
            .then_with(|| row_band_then_x(a_y, a_x, b_y, b_x))
    }

    /// Writing-axis bucket for a run's rotation: 0/90/180/270, or a distinct
    /// bucket for anything that is not within half a degree of a right angle.
    #[inline]
    #[allow(dead_code)]
    fn quadrant_key(rot: f32) -> i32 {
        if !rot.is_finite() {
            return i32::MAX;
        }
        let norm = rot.rem_euclid(360.0);
        for (q, angle) in [(0, 0.0), (1, 90.0), (2, 180.0), (3, 270.0)] {
            if (norm - angle).abs() <= 0.5 || (norm - (angle + 360.0)).abs() <= 0.5 {
                return q;
            }
        }
        4 + (norm as i32)
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
            Ordering::Equal => safe_float_cmp(b_x, a_x).then_with(|| safe_float_cmp(b_y, a_y)),
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
    #[allow(dead_code)]
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
            (
                std::cmp::Reverse(band),
                F32Ord(get_x(it)),
                std::cmp::Reverse(F32Ord(get_y(it))),
            )
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

    /// Give every span the baseline of the row it is printed on, so a row-band
    /// comparator sees one row per printed line.
    ///
    /// Quantizing each baseline onto a fixed grid decides row membership by
    /// which side of an arbitrary boundary a baseline lands on, and that is
    /// wrong wherever a row mixes font sizes. A timetable sets its times at
    /// 5 pt and its band names at 8 pt on the same rows; the name's baseline
    /// sits 3.3 pt below its own time's and only 2.0 pt above the next one's,
    /// so the name bands with the row *below* the one it is printed on.
    ///
    /// Neither edge of the box settles it alone. Producers align mixed sizes
    /// sometimes on the baseline and sometimes on the cap top — this very page
    /// does both — so two runs are taken to be aligned when *either* their
    /// baselines or their tops agree, whichever agrees better. ISO 32000-1:2008
    /// §9.4.4 computes the glyph displacement along the writing axis and sets
    /// the component for the other axis to 0: a horizontal run does not move
    /// vertically as it is painted, so both edges are fixed by the font and
    /// either may be the one the producer aligned on.
    ///
    /// The page's dominant text size defines the row grid. Rows are seeded
    /// from spans at that size, in descending baseline order; every remaining
    /// span then joins the row it aligns with *best*, rather than the first
    /// row within tolerance — a name centred between two rows is close to
    /// both, and only the better match is the row it is printed on. A span
    /// that aligns with no row seeds one of its own.
    ///
    /// Rows are formed per writing-axis quadrant, so a rotated run never joins
    /// a horizontal row.
    pub fn snap_baselines_to_rows(all_spans: &[crate::layout::TextSpan], indices: &[usize]) -> Vec<f32> {
        // Baseline and top of a span. A degenerate box falls back to the font
        // size so it still gets a row rather than becoming one.
        let edges = |i: usize| -> (f32, f32) {
            let b = &all_spans[i].bbox;
            let h = if b.height.is_finite() && b.height > 0.0 {
                b.height
            } else {
                all_spans[i].font_size.max(1.0)
            };
            (b.y, b.y + h)
        };
        let quadrant = |i: usize| -> i32 {
            let r = all_spans[i].rotation_degrees;
            if !r.is_finite() {
                return 0;
            }
            (r / 90.0).round().rem_euclid(4.0) as i32
        };
        // How far apart two runs are, taking the better-agreeing edge — but
        // only between runs of comparable height.
        //
        // Reading the better-agreeing edge is what lets a superscript, a drop
        // capital or a run whose box carries a descender join the line it
        // belongs to: at similar heights, agreement on either edge implies
        // agreement on the other. That implication fails once one run is much
        // taller than the other. A 19 pt centred title spanning three lines of
        // an 8 pt stamp beside it had a top edge 0.4 pt from the stamp's first
        // line and a baseline 11 pt away, so the better-agreeing edge put the
        // title *inside* the stamp's opening phrase and pushed the phrase's
        // second half onto the following line: `Prescribed by Treasury` /
        // title / `Department Treasury Dept. Cir. 1076`.
        //
        // Sharing a row means sharing a baseline. Above twice the height the
        // top edge stops being evidence of that and only the baseline counts.
        const COMPARABLE_HEIGHT_RATIO: f32 = 2.0;
        let distance = |a: usize, b: usize| -> f32 {
            let (a_base, a_top) = edges(a);
            let (b_base, b_top) = edges(b);
            let by_baseline = (a_base - b_base).abs();
            let (short, tall) = {
                let (ha, hb) = (a_top - a_base, b_top - b_base);
                (ha.min(hb), ha.max(hb))
            };
            if short > 0.0 && tall > short * COMPARABLE_HEIGHT_RATIO {
                return by_baseline;
            }
            by_baseline.min((a_top - b_top).abs())
        };

        // Leading shrinks with the type, so a flat 3 pt would join the lines
        // of 2.5 pt text; half the larger size caps it, and 3 pt still bounds
        // larger text. ~keep
        const ROW_TOLERANCE_FONT_FRACTION: f32 = 0.5;
        let row_tolerance = |a: usize, b: usize| -> f32 {
            let scale = all_spans[a].font_size.max(all_spans[b].font_size);
            if scale.is_finite() && scale > 0.0 {
                ROW_BAND_TOLERANCE_PT.min(scale * ROW_TOLERANCE_FONT_FRACTION)
            } else {
                ROW_BAND_TOLERANCE_PT
            }
        };

        let mut snapped: Vec<f32> = indices.iter().map(|&i| all_spans[i].bbox.y).collect();
        if indices.is_empty() {
            return snapped;
        }

        // The dominant text size, to 0.5 pt. Seeding rows from one size keeps
        // the grid regular; mixing every size in would let a run centred
        // between two rows define a row of its own between them.
        let mut tally: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for &i in indices {
            let fs = all_spans[i].font_size;
            if fs.is_finite() && fs > 0.0 {
                *tally.entry((fs * 2.0).round() as i32).or_insert(0) += 1;
            }
        }
        let modal = tally
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(k, _)| k);

        // Positions within `indices`, topmost baseline first, so a row is
        // always seeded by its upper edge.
        let mut order: Vec<usize> = (0..indices.len()).collect();
        order.sort_by(|&a, &b| safe_float_cmp(all_spans[indices[b]].bbox.y, all_spans[indices[a]].bbox.y));

        // Two runs cannot share a line and also share the space on it.
        //
        // Row membership is decided from vertical evidence alone, which is
        // right for runs printed side by side and wrong for runs printed on
        // top of each other. A page footer stamped over an earlier footer sits
        // within a fraction of a point of it — 0.145 pt between the cap tops of
        // a 7 pt and a 9 pt run — so every vertical test accepts the pair, the
        // row is then ordered by left edge, and the two footers come back
        // shuffled into one another: `The Molecular Probes The Molecular
        // Probes(R) Handbook: (TM) Handbook: A Guide to ...`.
        //
        // Horizontal extent settles it, and nothing else does. ISO 32000-1:2008
        // §9.4.4 advances the text position along the writing axis by each
        // glyph's displacement, so a run occupies one unbroken interval on that
        // axis; two runs whose intervals overlap substantially cannot both be
        // reading matter on one line, and one is drawn over the other.
        //
        // Substantially, because extractor boxes overreach to the right on
        // trailing whitespace and stretched advances, and adjacent runs on a
        // real line touch or overlap slightly through kerning. The bar is a
        // quarter of the shorter run and at least two points; the stamped
        // footers above overlap by 69.7 pt, which is 95% of the shorter one.
        const MIN_OVERLAP_PT: f32 = 2.0;
        const OVERLAP_FRACTION: f32 = 0.25;
        let x_extent = |i: usize| -> (f32, f32) {
            let b = &all_spans[i].bbox;
            let w = if b.width.is_finite() && b.width > 0.0 {
                b.width
            } else {
                0.0
            };
            (b.x, b.x + w)
        };
        let occupies_the_same_space = |i: usize, j: usize| -> bool {
            // A blank run competes for no reading space. Producers emit
            // space-only runs freely, and one drawn a fraction of a point under
            // a heading at the same left edge belongs to that heading's row —
            // separating it there would undo the rule that keeps a two-line
            // section title whole.
            if all_spans[i].text.trim().is_empty() || all_spans[j].text.trim().is_empty() {
                return false;
            }
            let ((li, ri), (lj, rj)) = (x_extent(i), x_extent(j));
            if !(li.is_finite() && ri.is_finite() && lj.is_finite() && rj.is_finite()) {
                return false;
            }
            let overlap = ri.min(rj) - li.max(lj);
            if overlap <= 0.0 {
                return false;
            }
            let shorter = (ri - li).min(rj - lj).max(0.0);
            overlap > MIN_OVERLAP_PT.max(shorter * OVERLAP_FRACTION)
        };

        // A row is remembered by the span that seeded it, and by everything
        // assigned to it — a candidate has to clear the space of every member,
        // not just the seed's, because the run it collides with may have joined
        // the row later.
        let mut rows: Vec<usize> = Vec::new();
        let mut members: Vec<Vec<usize>> = Vec::new();
        let mut row_of: Vec<Option<usize>> = vec![None; indices.len()];
        // Seeded rows indexed by their seed's baseline, ascending, so the
        // nearest-row search reads a window instead of every row on the page.
        //
        // `distance` is the smaller of the baseline gap and the top-edge gap,
        // and a top-edge gap differs from the baseline gap by at most the two
        // runs' heights, so `d <= ROW_BAND_TOLERANCE_PT` implies
        // `|baseline_i - baseline_seed| <= ROW_BAND_TOLERANCE_PT + h_i + h_max`.
        // Every row that could win is inside that window; the ones outside it
        // could only lose, and a loss and an absence take the same branch.
        let mut rows_by_baseline: Vec<(f32, usize)> = Vec::new();
        let h_max = indices
            .iter()
            .map(|&i| {
                let (b, t) = edges(i);
                (t - b).abs()
            })
            .filter(|h| h.is_finite())
            .fold(0.0f32, f32::max);
        let is_modal =
            |i: usize| -> bool { modal.is_some_and(|m| ((all_spans[i].font_size * 2.0).round() as i32) == m) };

        // Two passes over the same order: the dominant size lays down the
        // grid, then everything else attaches to it.
        for modal_pass in [true, false] {
            for &pos in &order {
                let i = indices[pos];
                if row_of[pos].is_some() || is_modal(i) != modal_pass {
                    continue;
                }
                if !all_spans[i].bbox.y.is_finite() {
                    continue;
                }
                let q = quadrant(i);
                let (base_i, top_i) = edges(i);
                let window = ROW_BAND_TOLERANCE_PT + (top_i - base_i).abs() + h_max;
                let (lo_b, hi_b) = (base_i - window, base_i + window);
                let from = rows_by_baseline.partition_point(|&(b, _)| b < lo_b);
                let to = rows_by_baseline.partition_point(|&(b, _)| b <= hi_b);
                let mut candidates: Vec<usize> = rows_by_baseline[from..to].iter().map(|&(_, r)| r).collect();
                // Row order, so a tie still resolves to the row seeded first.
                candidates.sort_unstable();
                let best = candidates
                    .into_iter()
                    .filter(|&r| quadrant(rows[r]) == q)
                    .map(|r| (distance(i, rows[r]), r, rows[r]))
                    .min_by(|a, b| safe_float_cmp(a.0, b.0));
                // The space test is applied to the row that wins on distance,
                // not to every row that might have. Scanning each candidate's
                // members for every span is quadratic in the spans on a page
                // and doubled the time to convert a 725-page book; a run is
                // only ever placed on its nearest row, so that is the only one
                // whose space it can be competing for. A run vetoed there opens
                // a row of its own, which is what it needs.
                match best {
                    Some((d, r, seed))
                        if d <= row_tolerance(i, seed)
                            && !members[r].iter().any(|&m| occupies_the_same_space(i, m)) =>
                    {
                        row_of[pos] = Some(seed);
                        members[r].push(i);
                    }
                    _ => {
                        let at = rows_by_baseline.partition_point(|&(b, _)| b <= base_i);
                        rows_by_baseline.insert(at, (base_i, rows.len()));
                        rows.push(i);
                        members.push(vec![i]);
                        row_of[pos] = Some(i);
                    }
                }
            }
        }

        for (pos, seed) in row_of.iter().enumerate() {
            if let Some(seed) = seed {
                snapped[pos] = all_spans[*seed].bbox.y;
            }
        }
        snapped
    }

    /// Total-order wrapper over `f32` for use as a sort key. For finite values
    /// `total_cmp` is identical to `safe_float_cmp` / `partial_cmp`.
    #[derive(Clone, Copy, PartialEq)]
    #[allow(dead_code)]
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

        /// Build a span at an explicit baseline/height for the row-snapping
        /// tests below.
        fn row_span(y: f32, height: f32, text: &str) -> crate::layout::TextSpan {
            row_span_at(0.0, y, height, text)
        }

        /// As [`row_span`], at an explicit left edge — for cases where the
        /// horizontal extents matter and must not overlap.
        fn row_span_at(x: f32, y: f32, height: f32, text: &str) -> crate::layout::TextSpan {
            crate::layout::TextSpan {
                text: text.to_string(),
                bbox: crate::geometry::Rect::new(x, y, 10.0, height),
                font_size: height,
                ..Default::default()
            }
        }

        /// A government form's masthead, at the geometry the file draws. A
        /// 19 pt centred title spans all three lines of an 8 pt stamp beside
        /// it, so its top edge lands 0.4 pt from the stamp's first line while
        /// its baseline sits 11.5 pt away.
        ///
        /// Taking the better-agreeing edge put the title on the stamp's
        /// opening row, which sorted it between `Prescribed by Treasury` and
        /// `Department` — splitting one phrase and gluing its second half to
        /// the line below.
        #[test]
        fn test_tall_centred_run_does_not_join_a_short_row_beside_it() {
            let spans = vec![
                row_span(723.0, 8.2, "Prescribed by Treasury"),
                row_span(716.0, 8.3, "Department"),
                row_span(709.1, 8.2, "Treasury Dept. Cir. 1076"),
                row_span(711.5, 19.3, "DIRECT DEPOSIT SIGN-UP FORM"),
            ];
            let idx: Vec<usize> = (0..spans.len()).collect();
            let rows = snap_baselines_to_rows(&spans, &idx);
            assert_ne!(
                rows[3], rows[0],
                "a run 2.4x the height of the line beside it does not share its row"
            );
        }

        /// The counter-case that keeps the narrowing honest. Two runs of
        /// comparable height whose tops agree exactly and whose baselines do
        /// not — a superscript, a drop capital, a box carrying a descender —
        /// must still snap together. This is the case the better-agreeing-edge
        /// rule exists for, and it passes with or without the height guard.
        #[test]
        fn comparable_runs_still_snap_on_their_better_edge() {
            // Side by side, as a superscript actually sits: sharing a line
            // means sharing neither ink nor the space it occupies.
            let spans = vec![
                row_span_at(0.0, 700.0, 10.0, "body"),
                row_span_at(12.0, 703.0, 7.0, "sup"),
            ];
            let idx: Vec<usize> = (0..spans.len()).collect();
            let rows = snap_baselines_to_rows(&spans, &idx);
            assert_eq!(
                rows[0], rows[1],
                "runs of similar height still join on whichever edge agrees"
            );
        }

        /// Lines of 2.5 pt text sit 3 pt apart. The flat 3 pt row tolerance
        /// joined them, and the left-to-right sort then emitted the lower line
        /// first.
        #[test]
        fn small_text_lines_at_normal_leading_stay_on_separate_rows() {
            let spans = vec![
                row_span_at(20.0, 720.0, 2.5, "upper"),
                row_span_at(0.0, 717.0, 2.5, "lower"),
            ];
            let idx: Vec<usize> = (0..spans.len()).collect();
            let rows = snap_baselines_to_rows(&spans, &idx);
            assert_ne!(rows[0], rows[1], "3 pt leading at 2.5 pt text is two lines");
        }

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

        /// Two spans in one band at the same x still order by baseline.
        #[test]
        fn test_sub_band_baseline_difference_still_decides() {
            assert_eq!(
                row_aware_span_cmp(98.36, 232.08, 98.21, 232.08),
                Ordering::Less,
                "one band, one x: the baseline must decide, or sort stability does"
            );
            assert_eq!(row_aware_span_cmp(98.21, 232.08, 98.36, 232.08), Ordering::Greater);
        }

        /// The banding still does its job: within a band, x decides whatever
        /// the baselines are doing. This is the case banding exists for and the
        /// tiebreak must not disturb it.
        #[test]
        fn x_still_decides_within_a_band() {
            assert_eq!(row_aware_span_cmp(98.36, 100.0, 98.21, 200.0), Ordering::Less);
            assert_eq!(row_aware_span_cmp(98.21, 100.0, 98.36, 200.0), Ordering::Less);
        }

        /// `row_band_then_x` deliberately stops before the baseline, so a
        /// caller sorting on a synthetic key can apply its own tiebreak on the
        /// real geometry. The two comparators must differ in exactly this way,
        /// or the split has no effect and the hazard comes back.
        #[test]
        fn test_band_and_x_comparator_leaves_a_same_x_tie_open() {
            assert_eq!(row_band_then_x(98.21, 232.08, 98.36, 232.08), Ordering::Equal);
            assert_eq!(row_aware_span_cmp(98.21, 232.08, 98.36, 232.08), Ordering::Greater);
            // Wherever x differs the two agree, so swapping one for the other
            // moves nothing except the tie.
            assert_eq!(
                row_band_then_x(98.36, 100.0, 98.21, 200.0),
                row_aware_span_cmp(98.36, 100.0, 98.21, 200.0)
            );
        }

        /// Two spans in one row share a row key — that is what the key is
        /// for — so any tiebreak read back from it compares them equal and
        /// hands their order to the sequence they arrived in. A space-only run
        /// drawn a fifth of a point under a heading, at the same left edge and
        /// emitted first, then sorted ahead of the heading's own text and broke
        /// a two-line section title into body text plus its last word.
        ///
        /// The row key settles the band and `x`; the baseline the page draws
        /// settles what is left.
        #[test]
        fn test_shared_row_key_leaves_the_baseline_to_decide() {
            use crate::layout::TextSpan;
            let span = |y: f32, text: &str| TextSpan {
                text: text.to_string(),
                bbox: crate::geometry::Rect::new(36.0, y, 100.0, 12.0),
                font_size: 12.0,
                ..Default::default()
            };
            // Emitted in the order a producer drew them: the space first.
            let spans = vec![span(745.73, " "), span(745.93, "Section Title")];
            let idx: Vec<usize> = (0..spans.len()).collect();
            let key = snap_baselines_to_rows(&spans, &idx);
            assert_eq!(
                key[0], key[1],
                "the two runs are one row, so the hazard this guards is real"
            );

            // Ordering on the key alone cannot separate them.
            assert_eq!(
                row_band_then_x_axis(0.0, key[0], 36.0, 0.0, key[1], 36.0),
                Ordering::Equal
            );
            // Adding the drawn baseline does, and puts the heading first.
            let ordered = row_band_then_x_axis(0.0, key[0], 36.0, 0.0, key[1], 36.0)
                .then_with(|| safe_float_cmp(spans[1].bbox.y, spans[0].bbox.y));
            assert_eq!(
                ordered,
                Ordering::Greater,
                "the run drawn higher on the page must be read first"
            );
        }

        /// A different band still wins over x.
        #[test]
        fn test_different_band_still_wins_over_x() {
            assert_eq!(row_aware_span_cmp(120.0, 400.0, 98.0, 50.0), Ordering::Less);
        }

        /// Identical geometry is genuinely equal — the comparator must not
        /// invent an order where there is no evidence for one.
        #[test]
        fn identical_geometry_is_equal() {
            assert_eq!(row_aware_span_cmp(98.36, 232.08, 98.36, 232.08), Ordering::Equal);
        }

        /// And the cached-key sort must agree with the comparator, or the two
        /// orderings diverge wherever both are used on the same data.
        #[test]
        fn test_cached_key_sort_agrees_with_the_comparator() {
            let data = [(98.21_f32, 232.08_f32), (98.36, 232.08), (98.30, 100.0)];
            let mut by_key = data.to_vec();
            sort_by_row_band(&mut by_key, |it| it.0, |it| it.1);
            let mut by_cmp = data.to_vec();
            by_cmp.sort_by(|a, b| row_aware_span_cmp(a.0, a.1, b.0, b.1));
            assert_eq!(by_key, by_cmp);
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
