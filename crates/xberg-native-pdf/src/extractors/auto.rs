//! Comprehensive auto-extraction — public vocabulary.
//!
//! The configured-once [`AutoExtractor`] returns *all* recoverable text
//! from a PDF — native text **and** text embedded in images,
//! image-tables and figures — decided per-page/per-region, with a
//! machine-readable [`ReasonCode`] for every degraded result.
//!
//! This module is the **dependency root** and is *pure PDF inspection*:
//! it carries no OCR feature gate — this crate has never had one — and is
//! fully testable (00-common-foundation §5/§9). The
//! classification signal model (T2/T3) and the [`AutoExtractor`]
//! pipeline (T4–T8) build on these types.
//!
//! Design contracts (api-design.md §3, README "Locked decisions"):
//! - **Strictly additive** — existing `extract_text`/`extract_spans`/…
//!   are byte-identical; this is a *new* surface.
//! - **Single mode knob** [`ExtractMode`] (Tika/unstructured pattern),
//!   not boolean soup.
//! - **Typed reason per region** — never a bare empty string; degraded
//!   results always say *why* (the #1 cross-tool user pain).
//! - **Graceful, never fail-loud** — OCR unavailable → warn + native
//!   fallback + [`ReasonCode::OcrRequestedButUnavailable`] (best-effort
//!   extraction degrades gracefully; only security ops fail-closed).
//! - All public types are `#[non_exhaustive]` + `serde` (the JSON
//!   C-ABI boundary, matching the shipped split-by-bookmarks idiom) so
//!   later tiers (T2/T3) are additive and non-breaking.

use serde::{Deserialize, Serialize};

/// How [`AutoExtractor`](crate::extractors::auto) decides text-vs-OCR.
///
/// One knob collapses off/auto/force (Tika `OCR_STRATEGY` /
/// unstructured `strategy` — the cleanest mode model; the multi-boolean
/// designs are the ones that caused upstream bugs). `#[non_exhaustive]`
/// → a future tier is a non-breaking addition (T2/T3 are deliberately
/// *not* present — `tier-model-strategy.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExtractMode {
    /// Native text layer only — never OCR (≈ today's `extract_text`).
    TextOnly,
    /// Per-region heuristic: text-layer vs OCR vs image-table recovery.
    /// The default (README locked decision 2).
    #[default]
    Auto,
    /// OCR everything, ignore any native text layer (the
    /// "suspect text layer" forced-OCR escape hatch).
    ForceOcr,
}

/// Options for [`AutoExtractor`](crate::extractors::auto).
///
/// Plain struct + `Default` + `#[non_exhaustive]` (house style:
/// `RedactionOptions`/`SplitByBookmarksOptions`); construct with
/// struct-update (`..Default::default()`), a preset
/// ([`fast`](Self::fast)/[`balanced`](Self::balanced)/
/// [`high_fidelity`](Self::high_fidelity)), or the
/// [`builder`](Self::builder) (mirrors the in-repo `OcrConfigBuilder`).
/// `serde` is the JSON config wire at the C-ABI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
#[non_exhaustive]
pub struct AutoExtractOptions {
    /// Text-vs-OCR mode. Default [`ExtractMode::Auto`].
    pub mode: ExtractMode,
    /// Reconstruct image-tables into a structured [`TableData`] via the
    /// existing spatial detector over OCR spans (default `true`).
    pub reconstruct_image_tables: bool,
    /// Emit positioned `Figure`/`Table` placeholders in the text flow
    /// so reading order is never silently corrupted (default `true`).
    pub emit_placeholders: bool,
    /// Auto-decision confidence threshold (`None` = preset/calibrated).
    pub min_text_confidence: Option<f32>,
    /// Image-table reconstruction confidence threshold (`None` =
    /// preset/calibrated).
    pub table_confidence: Option<f32>,
    /// Force OCR on these **0-based** page indices regardless of `mode`
    /// (additive on `Auto`; does not change [`ExtractMode`]). Empty =
    /// honour `mode` for every page.
    pub force_ocr_pages: Vec<usize>,
}

impl Default for AutoExtractOptions {
    fn default() -> Self {
        // `balanced()` is the documented default (README locked dec. 2;
        // api-design.md §3). ~keep
        Self::balanced()
    }
}

impl AutoExtractOptions {
    /// Text-layer biased, no layout/table work — the cheapest path.
    #[must_use]
    pub fn fast() -> Self {
        Self {
            mode: ExtractMode::TextOnly,
            reconstruct_image_tables: false,
            emit_placeholders: true,
            min_text_confidence: None,
            table_confidence: None,
            force_ocr_pages: Vec::new(),
        }
    }

    /// The default — `Auto` per-region routing with image-table
    /// reconstruction (calibrated thresholds).
    #[must_use]
    pub fn balanced() -> Self {
        Self {
            mode: ExtractMode::Auto,
            reconstruct_image_tables: true,
            emit_placeholders: true,
            min_text_confidence: None,
            table_confidence: None,
            force_ocr_pages: Vec::new(),
        }
    }

    /// Aggressive OCR + image-table recovery (still local-CPU T1; no
    /// VLM/cloud — `tier-model-strategy.md`).
    #[must_use]
    pub fn high_fidelity() -> Self {
        Self {
            mode: ExtractMode::Auto,
            reconstruct_image_tables: true,
            emit_placeholders: true,
            // Lower bars → escalate to OCR / table recovery sooner. ~keep
            min_text_confidence: Some(0.55),
            table_confidence: Some(0.45),
            force_ocr_pages: Vec::new(),
        }
    }

    /// Fluent builder (mirrors the in-repo `OcrConfigBuilder`).
    #[must_use]
    pub fn builder() -> AutoExtractOptionsBuilder {
        AutoExtractOptionsBuilder::new()
    }
}

/// Fluent builder for [`AutoExtractOptions`] — same shape as the
/// canonical in-repo `OcrConfigBuilder` (`mut self -> Self`, validating
/// setters, terminal `build`).
#[derive(Debug, Clone, Default)]
pub struct AutoExtractOptionsBuilder {
    opts: AutoExtractOptions,
}

impl AutoExtractOptionsBuilder {
    /// Start from the [`balanced`](AutoExtractOptions::balanced) preset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            opts: AutoExtractOptions::balanced(),
        }
    }

    /// Set the text-vs-OCR [`ExtractMode`].
    #[must_use]
    pub fn mode(mut self, mode: ExtractMode) -> Self {
        self.opts.mode = mode;
        self
    }

    /// Toggle image-table reconstruction.
    #[must_use]
    pub fn reconstruct_image_tables(mut self, yes: bool) -> Self {
        self.opts.reconstruct_image_tables = yes;
        self
    }

    /// Toggle figure/table placeholders in the text flow.
    #[must_use]
    pub fn emit_placeholders(mut self, yes: bool) -> Self {
        self.opts.emit_placeholders = yes;
        self
    }

    /// Auto-decision confidence threshold (clamped to `0.0..=1.0`).
    #[must_use]
    pub fn min_text_confidence(mut self, c: f32) -> Self {
        self.opts.min_text_confidence = Some(c.clamp(0.0, 1.0));
        self
    }

    /// Image-table confidence threshold (clamped to `0.0..=1.0`).
    #[must_use]
    pub fn table_confidence(mut self, c: f32) -> Self {
        self.opts.table_confidence = Some(c.clamp(0.0, 1.0));
        self
    }

    /// Force OCR on these 0-based page indices (additive on `mode`).
    #[must_use]
    pub fn force_ocr_pages<I: IntoIterator<Item = usize>>(mut self, pages: I) -> Self {
        self.opts.force_ocr_pages = pages.into_iter().collect();
        self
    }

    /// Finalise.
    #[must_use]
    pub fn build(self) -> AutoExtractOptions {
        self.opts
    }
}

/// Per-page classification outcome (T0/T0.5 — pure inspection).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PageKind {
    /// Usable native text dominates — extract the text layer.
    TextLayer,
    /// Image-dominated, no/garbled text — OCR the page.
    Scanned,
    /// Native text **and** image regions containing text — hybrid
    /// (native + region OCR).
    ImageText,
    /// Heterogeneous within the page (text + image-table/figure).
    Mixed,
    /// Blank/near-empty — neither extract nor OCR; not an error.
    Empty,
}

/// Machine-readable *why* for a region/page result — **the #1
/// cross-tool user-pain fix**. A non-degraded result is
/// [`Ok`](ReasonCode::Ok); any degraded one **must** name the cause.
/// `#[non_exhaustive]`, frozen `snake_case` wire tokens (append-only —
/// never renumber/rename, the `PadesLevel` lesson).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasonCode {
    /// Extracted cleanly.
    Ok,
    /// Native text present and high-confidence.
    NativeTextHighConfidence,
    /// No text layer on the page at all.
    NoTextLayerPresent,
    /// Text layer present but below the usable-quality threshold.
    TextLayerBelowThreshold,
    /// Glyphs without usable ToUnicode/`(cid:NN)`/garbled mapping.
    GlyphMappingMissing,
    /// Encrypted and not authorised to extract.
    EncryptedNoExtractPermission,
    /// An image-table was reconstructed into [`TableData`].
    ImageTableReconstructed,
    /// A table region was detected but structure could not be
    /// recovered (graceful — heuristic fallback used, not fail-loud).
    ImageTableNoStructure,
    /// A chart/figure was detected; its internal data is **not**
    /// transcribed (honest scope boundary — placeholder emitted).
    ChartNotTranscribed,
    /// OCR was needed but unavailable (feature off / models absent /
    /// `mode = TextOnly`) → fell back to native text + warned.
    OcrRequestedButUnavailable,
    /// OCR ran but confidence was low → native used/merged.
    OcrLowConfidenceFallback,
    /// Region/page yielded no recoverable content.
    Empty,
}

/// Where a region's text came from. `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExtractSource {
    /// xberg-native-pdf native text-layer extraction.
    NativeText,
    /// OCR (region or full page).
    Ocr,
    /// Structured image-table reconstruction.
    ImageTableRecovery,
    /// Degraded fallback to native after an unavailable/low-confidence
    /// higher tier (paired with a non-`Ok` [`ReasonCode`]).
    Fallback,
}

/// Region classification. `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RegionKind {
    /// Body / inline text.
    Text,
    /// Heading (from structure tree or font-size heuristic).
    Heading,
    /// Tabular region (native or reconstructed image-table).
    Table,
    /// Figure/illustration (text recovered via region OCR if any).
    Figure,
    /// Chart/plot — internal data not transcribed (honest boundary).
    Chart,
}

/// A 4-point quadrilateral bounding box in PDF points (top-left,
/// top-right, bottom-right, bottom-left). A quad (not an AABB) so
/// skewed/rotated regions survive (cases I/J — Textract/Azure
/// convention). `serde` for the JSON C-ABI boundary.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quad {
    /// `[tl, tr, br, bl]`, each `[x, y]` in PDF points.
    pub points: [[f32; 2]; 4],
}

impl Quad {
    /// Axis-aligned quad from `(x, y, w, h)` (PDF points, origin
    /// bottom-left).
    #[must_use]
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            points: [[x, y + h], [x + w, y + h], [x + w, y], [x, y]],
        }
    }
}

/// Structured table payload (serde-friendly — what crosses the JSON
/// C-ABI). Decoupled from the internal
/// `structure::table_extractor::Table` on purpose: T7 maps the spatial
/// detector's output into this; consumers get cells + markdown.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct TableData {
    /// Row-major cell text.
    pub rows: Vec<Vec<String>>,
    /// First row is a header.
    pub has_header: bool,
    /// GitHub-flavoured Markdown rendering of the table.
    pub markdown: String,
}

/// One classified, extracted region of a page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct Region {
    /// 4-point bbox in PDF points.
    pub bbox: Quad,
    /// Region classification.
    pub kind: RegionKind,
    /// Recovered text (`""` if none — e.g. an un-transcribed chart;
    /// the bbox + reason still locate it for reading order).
    pub text: String,
    /// Reconstructed table (only for [`RegionKind::Table`]).
    pub table: Option<TableData>,
    /// Confidence `0.0..=1.0`, always present.
    pub confidence: f32,
    /// Where the text came from.
    pub source: ExtractSource,
    /// Why this source/outcome (typed; never opaque).
    pub reason: ReasonCode,
}

/// Document-level rollup status. `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExtractionStatus {
    /// Every region extracted cleanly.
    Complete,
    /// Some regions degraded (see per-region [`ReasonCode`]) but text
    /// was recovered — *not* an error (Docling `PARTIAL_SUCCESS`).
    PartialSuccess,
    /// No usable text recovered anywhere.
    NoTextRecovered,
}

/// Per-page extraction result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct PageExtraction {
    /// 0-based page index.
    pub page: usize,
    /// Per-page classification.
    pub kind: PageKind,
    /// Assembled, reading-ordered text for the page.
    pub text: String,
    /// Per-region results (bbox + reason always present, even when
    /// `text` is empty — pain #2/#8).
    pub regions: Vec<Region>,
    /// Page-level confidence `0.0..=1.0`.
    pub confidence: f32,
    /// Page-level rollup reason.
    pub reason: ReasonCode,
    /// Whether OCR actually ran for this page.
    pub ocr_used: bool,
    /// Page-level status.
    pub status: ExtractionStatus,
}

/// Whole-document extraction result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct DocumentExtraction {
    /// Per-page results (per-page decision — never one forced doc mode;
    /// case Q).
    pub pages: Vec<PageExtraction>,
    /// Document rollup status.
    pub status: ExtractionStatus,
    /// 0-based page indices a cheap preflight says need OCR.
    pub pages_needing_ocr: Vec<usize>,
}

/// Lightweight document classification (the cheap `classify_document`
/// preflight — no OCR, no rasterisation).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct DocumentClassification {
    /// Per-page kinds.
    pub pages: Vec<PageKind>,
    /// 0-based page indices needing OCR.
    pub pages_needing_ocr: Vec<usize>,
    /// Aggregate `mostly_text` / `mostly_scanned` / `mixed`.
    pub summary: DocumentSummary,
}

/// Aggregate document character. `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DocumentSummary {
    /// Predominantly born-digital text.
    MostlyText,
    /// Predominantly scanned/image.
    MostlyScanned,
    /// Heterogeneous (case Q).
    Mixed,
    /// No meaningful content.
    Empty,
}

// ───────────────────────── classifier (T2/T2.5/T3) ─────────────────────────
//
// Pure inspection of xberg-native-pdf *internals* — never the flattened output
// string (00-common-foundation §9). Lowest-level decision logic takes
// **injected primitives** so it is unit-testable without a
// `PdfDocument` (00-common-foundation §8 — the `sanitize_catalog`
// injected-resolver precedent). `PdfDocument::classify_page` /
// `classify_document` (in `document.rs`) gather the internal signals and
// delegate here. No OCR feature gate — this crate has never had one. ~keep

/// Dominant raster codec on a page — a strong scan-vs-pictorial prior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageCodecClass {
    /// No raster images.
    None,
    /// CCITT Group 3/4 fax — 1-bit, almost always a scan.
    Ccitt,
    /// JBIG2 — 1-bit, almost always a scan.
    Jbig2,
    /// DCT/JPEG — photo or colour scan.
    Dct,
    /// Flate/other raster.
    Other,
}

/// Document-level scanner-vs-authoring prior (case P). Weak, never
/// decisive — only a tie-break nudge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProducerPrior {
    /// Producer/Creator looks like scanner/OCR software.
    Scanner,
    /// Producer/Creator looks like authoring software.
    Authoring,
    /// Unknown / no Info.
    Unknown,
}

/// Per-page signals gathered from xberg-native-pdf internals (T2). All ratios
/// are `0.0..=1.0`. Serde so it can ride the explainable result.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct PageSignals {
    /// Non-artifact glyph count from the native text layer.
    pub text_glyph_count: usize,
    /// Σ text-box area ∩ page ÷ page area.
    pub text_area_ratio: f32,
    /// Σ CTM-transformed image-box area ∩ page ÷ page area (clamped;
    /// summed not max'd → multi-strip scans, case J, are caught).
    pub image_area_ratio: f32,
    /// Dominant raster codec.
    pub codec: ImageCodecClass,
    /// Tr-mode-3 (invisible) glyphs ÷ all glyphs — the OCR-sidecar /
    /// painted-over signal (cases C/C2).
    pub invisible_text_ratio: f32,
    /// (U+FFFD + control + replacement) ÷ chars — `(cid:NN)`/garbled.
    pub garbled_ratio: f32,
    /// Short-fragmented-"word" ratio (broken CMaps split every glyph).
    pub fragmented_word_ratio: f32,
    /// Consecutive/duplicated-token run ratio — 2-column born-digital
    /// scramble (the T0.5 addition, research §3a).
    pub consecutive_repeat_ratio: f32,
    /// Path-ish ops ÷ all content ops — outlined/vectorised text
    /// (case F).
    pub vector_path_density: f32,
    /// `MarkInfo.marked && !suspects` — high-precision digital prior.
    pub has_reliable_structure: bool,
    /// Scanner-vs-authoring producer prior (weak).
    pub producer_prior: ProducerPrior,
    /// No text, no significant image, no paths.
    pub page_is_empty: bool,
}

/// Cheap per-page classification (the `classify_page` preflight — no
/// OCR, no rasterisation): kind + confidence + typed reason + the raw
/// signals (explainable; our differentiator over downstream wrappers).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct PageClassification {
    /// 0-based page index.
    pub page: usize,
    /// Decided page kind.
    pub kind: PageKind,
    /// Confidence `0.0..=1.0`.
    pub confidence: f32,
    /// Typed reason for the decision.
    pub reason: ReasonCode,
    /// Raw signals (explainability).
    pub signals: PageSignals,
}

/// Whether `text` is CJK/Hangul-dominant: a majority of its non-whitespace
/// characters fall in the Han, Hiragana, Katakana, or Hangul Unicode
/// blocks. These scripts don't use inter-word spaces, so glyph-adjacency
/// word clustering naturally produces short (often 1-2 character) tokens —
/// a `frag`/`avg_word_len` signal calibrated for space-separated Latin
/// text misreads that as fragmentation. Used to skip the Latin-specific
/// checks in [`text_quality_gate`] for such text; script-agnostic signals
/// (garbled/repeat ratio) still apply normally.
#[must_use]
pub fn is_cjk_dominant_text(text: &str) -> bool {
    let mut total = 0usize;
    let mut cjk = 0usize;
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        }
        total += 1;
        let cp = c as u32;
        if (0x4E00..=0x9FFF).contains(&cp)   // CJK Unified Ideographs ~keep
            || (0x3400..=0x4DBF).contains(&cp) // CJK Extension A ~keep
            || (0x3040..=0x309F).contains(&cp) // Hiragana ~keep
            || (0x30A0..=0x30FF).contains(&cp) // Katakana ~keep
            || (0xAC00..=0xD7A3).contains(&cp)
        // Hangul Syllables ~keep
        {
            cjk += 1;
        }
    }
    total > 0 && (cjk as f32 / total as f32) > 0.5
}

/// T0.5 enriched text-quality gate (research §3a).
/// Pure: operates on the native-extracted text only. Returns the
/// degrading [`ReasonCode`] if the "born-digital" text is unusable, or
/// `None` if it is good.
///
/// Signals (thresholds are conservative defaults; presets/calibration
/// own tuning — we encode the signals, not 16 user knobs):
/// - U+FFFD / replacement / control ratio (`(cid:NN)`/garbled);
/// - **critical** short-fragmented-word ratio — a hard trigger that
///   overrides everything else;
/// - **consecutive-repeat / column-scramble** ratio;
/// - average word length & alphanumeric ratio;
/// - Unicode-block-aware script validity (no ASCII bias — case H).
#[must_use]
pub fn text_quality_gate(text: &str) -> Option<ReasonCode> {
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    if total < 16 {
        // Too little to judge; let the coverage cascade decide. ~keep
        return None;
    }
    // Half-open `..` ranges are intentional: HT(0x09) LF(0x0A) VT(0x0B)
    // FF(0x0C) CR(0x0D) and SP(0x20) are legitimate whitespace and are
    // deliberately NOT counted as garbled (the range stops *before*
    // 0x09 and *before* 0x20). ~keep
    let bad = chars
        .iter()
        .filter(|&&c| {
            crate::is_lost_glyph_marker(c)
                || ('\u{0}'..'\u{9}').contains(&c)
                || ('\u{E}'..'\u{20}').contains(&c)
                || ('\u{E000}'..='\u{F8FF}').contains(&c) // private-use (notdef/tofu) ~keep
        })
        .count();
    let garbled_ratio = bad as f32 / total as f32;
    if garbled_ratio > 0.20 {
        return Some(ReasonCode::GlyphMappingMissing);
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() >= 8 {
        let frag = words.iter().filter(|w| w.chars().count() <= 2).count() as f32 / words.len() as f32;
        // CJK/Hangul text has no inter-word spaces, so glyph-adjacency
        // clustering naturally produces short tokens — `frag` and
        // `avg_word_len` below are calibrated for space-separated Latin
        // text and would otherwise misread ordinary dense CJK prose as
        // fragmented. The repeat-ratio check (script-agnostic) still
        // applies normally. ~keep
        let cjk_dominant = is_cjk_dominant_text(text);
        // Critical hard-trigger (broken CMap splitting every glyph). ~keep
        if frag > 0.80 && !cjk_dominant {
            return Some(ReasonCode::GlyphMappingMissing);
        }
        // Consecutive-repeat / 2-column scramble: long runs of the
        // same token, or duplicated adjacent tokens. ~keep
        let mut repeats = 0usize;
        for w in words.windows(2) {
            if w[0] == w[1] {
                repeats += 1;
            }
        }
        let repeat_ratio = repeats as f32 / words.len() as f32;
        let avg_word_len = words.iter().map(|w| w.chars().count()).sum::<usize>() as f32 / words.len() as f32;
        if repeat_ratio > 0.30 || (frag > 0.55 && avg_word_len < 2.5 && !cjk_dominant) {
            return Some(ReasonCode::TextLayerBelowThreshold);
        }
    }

    let alnum = chars.iter().filter(|c| c.is_alphanumeric()).count();
    if (alnum as f32 / total as f32) < 0.20 {
        return Some(ReasonCode::TextLayerBelowThreshold);
    }
    None
}

/// T3 decision cascade — pure, from gathered [`PageSignals`] + config.
/// Deterministic ordered guards then a coverage/density verdict (KISS:
/// a rule cascade, not ML). Returns `(kind, confidence, reason)`.
#[must_use]
pub fn classify_from_signals(s: &PageSignals, cfg: &AutoExtractOptions) -> (PageKind, f32, ReasonCode) {
    // Calibrated defaults (research §3/§4); presets/cfg override. ~keep
    let min_text_conf = cfg.min_text_confidence.unwrap_or(0.70);
    let scan_cover_min = 0.80_f32;
    let sparse_text_max = 0.10_f32;
    let min_glyphs = 24_usize;

    if s.page_is_empty {
        return (PageKind::Empty, 0.99, ReasonCode::Empty);
    }

    let usable_text = s.text_glyph_count >= min_glyphs && s.garbled_ratio <= 0.20 && s.fragmented_word_ratio <= 0.80;

    // Outlined/vectorised text (case F): paths dominate, ~no text/raster. ~keep
    if !usable_text && s.vector_path_density > 0.60 && s.image_area_ratio < 0.20 {
        return (PageKind::Scanned, 0.80, ReasonCode::NoTextLayerPresent);
    }

    if let Some(verdict) = scan_dominant_verdict(s, usable_text, scan_cover_min, sparse_text_max) {
        return verdict;
    }

    // Genuine hybrid: real text AND sub-page image region(s) (cases D/S). ~keep
    if usable_text && s.image_area_ratio > 0.05 && s.image_area_ratio < scan_cover_min {
        return (PageKind::ImageText, 0.75, ReasonCode::Ok);
    }

    // Usable native text dominates (case A, good sidecar). ~keep
    if usable_text {
        let mut conf = min_text_conf.max(0.80);
        if s.has_reliable_structure {
            conf = (conf + 0.10).min(0.99);
        }
        return (PageKind::TextLayer, conf, ReasonCode::NativeTextHighConfidence);
    }

    if s.text_glyph_count > 0 {
        return low_glyph_count_verdict(s);
    }

    let conf = match s.producer_prior {
        ProducerPrior::Scanner => 0.85,
        _ => 0.70,
    };
    (PageKind::Scanned, conf, ReasonCode::NoTextLayerPresent)
}

/// Verdict for a scan-dominant page (`image_area_ratio >= scan_cover_min`):
/// either an OCR sidecar already covers it, or the text present is too
/// sparse relative to the scan to outweigh it. Returns `None` when the page
/// isn't scan-dominant, or is but neither sub-case applies (the caller then
/// falls through to the next guard in the cascade). Split out of
/// [`classify_from_signals`] purely to keep that function's overall cascade
/// legible. ~keep
fn scan_dominant_verdict(
    s: &PageSignals,
    usable_text: bool,
    scan_cover_min: f32,
    sparse_text_max: f32,
) -> Option<(PageKind, f32, ReasonCode)> {
    if s.image_area_ratio < scan_cover_min {
        return None;
    }
    // OCR sidecar already present and usable (cases C/C2) → keep it. ~keep
    if usable_text && s.invisible_text_ratio >= 0.50 {
        return Some((PageKind::TextLayer, 0.85, ReasonCode::NativeTextHighConfidence));
    }
    // Sparse text over a scan (case G) — the headline fix: text
    // *presence* is not enough; coverage must be relative. ~keep
    if s.text_area_ratio < sparse_text_max || !usable_text {
        let conf = if s.codec == ImageCodecClass::Ccitt || s.codec == ImageCodecClass::Jbig2 {
            0.95
        } else {
            0.85
        };
        // A full-bleed background image with a real, usable text
        // layer (a slide headline, a deck cover) is not "no text
        // layer" — the text is there, mapped correctly, and
        // extractable; it just doesn't cover enough of the page to
        // outweigh the scan-dominant image coverage. Report the
        // honest reason (coverage too low) rather than claiming no
        // text layer exists at all, which would tell a caller who
        // inspects `reason` there is nothing to extract. ~keep
        let reason = if usable_text {
            ReasonCode::TextLayerBelowThreshold
        } else {
            ReasonCode::NoTextLayerPresent
        };
        return Some((PageKind::Scanned, conf, reason));
    }
    None
}

/// Verdict when a page has some text glyphs but not enough to count as
/// `usable_text` (see [`classify_from_signals`]): distinguishes a short but
/// cleanly-mapped text page from one whose glyph mapping is actually broken.
/// Split out of [`classify_from_signals`] purely to keep that function's
/// overall cascade legible. ~keep
fn low_glyph_count_verdict(s: &PageSignals) -> (PageKind, f32, ReasonCode) {
    // Distinguish "few but CLEAN glyphs" from "garbled glyphs".
    // A short page of well-mapped, image-free text (e.g. a cover
    // line, a stub, a placeholder) is a *text* page — not a scan.
    // Only genuinely garbled glyphs (broken CMap/CID) or text
    // sitting over raster route to OCR. Without this guard a tiny
    // valid text PDF was misclassified `Scanned` and wrongly
    // listed in `pages_needing_ocr`. ~keep
    let clean = s.garbled_ratio <= 0.20 && s.fragmented_word_ratio <= 0.80;
    if clean && s.image_area_ratio < 0.05 {
        return (PageKind::TextLayer, 0.60, ReasonCode::NativeTextHighConfidence);
    }
    (PageKind::Scanned, 0.80, ReasonCode::GlyphMappingMissing)
}

mod summary;
pub use summary::summarise;

// ─────────────────────────── AutoExtractor (T4–T8) ─────────────────────────
//
// The configured-once, reusable object (api-design.md §3 — rejected the
// document-bolted sketch). Construction is cheap & infallible (no I/O,
// never downloads). The native/markdown/html/classify
// surface needs **no** `ocr` feature; OCR enrichment is `#[cfg(feature
// = "ocr")]` and **always** degrades gracefully (warn + native
// fallback + typed reason — never fail-loud; only security ops
// fail-closed — `feedback_extraction_graceful_fallback`). ~keep

use crate::document::PdfDocument;

/// Comprehensive auto-extractor — configure once, reuse across pages
/// and documents.
#[derive(Debug, Clone)]
pub struct AutoExtractor {
    opts: AutoExtractOptions,
}

impl Default for AutoExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoExtractor {
    /// Default extractor (`balanced` preset, `ExtractMode::Auto`).
    /// Infallible, no I/O, never downloads.
    #[must_use]
    pub fn new() -> Self {
        Self {
            opts: AutoExtractOptions::balanced(),
        }
    }

    /// Never touches OCR/models at all (`ExtractMode::TextOnly`) — the
    /// `new(DisableOcr)` shape.
    #[must_use]
    pub fn text_only() -> Self {
        Self {
            opts: AutoExtractOptions::fast(),
        }
    }

    /// Construct with explicit options.
    #[must_use]
    pub fn with(opts: AutoExtractOptions) -> Self {
        Self { opts }
    }

    /// The active options.
    #[must_use]
    pub fn options(&self) -> &AutoExtractOptions {
        &self.opts
    }

    /// Cheap whole-document classification preflight (no OCR).
    pub fn classify(&self, doc: &PdfDocument) -> crate::Result<DocumentClassification> {
        doc.classify_document()
    }

    /// Per-page text. `Auto`/`ForceOcr` route via classification; OCR is
    /// attempted only with the `ocr` feature and **always** falls back
    /// to native text + a `tracing::warn` if unavailable/failed (never
    /// crashes, never silent-empty — the reason is observable via
    /// [`extract_page`](Self::extract_page)).
    pub fn extract_text(&self, doc: &PdfDocument, page: usize) -> crate::Result<String> {
        // TextOnly is the cheapest path and must NOT classify —
        // classification pulls spans/images, defeating the stated
        // "fast" contract. `doc.extract_text` itself
        // fails closed on an encrypted-unauthenticated PDF (case L),
        // so the security invariant holds without classifying here. ~keep
        if matches!(self.opts.mode, ExtractMode::TextOnly) {
            return doc.extract_text(page);
        }
        // Auto/ForceOcr: classify (fails closed on encrypted, case L)
        // then route. `route` owns the OCR-vs-native decision and the
        // observable provenance. ~keep
        let cls = doc.classify_page(page)?;
        Ok(self.route(doc, page, &cls)?.0)
    }

    /// The single source of truth for the Auto/ForceOcr routing
    /// decision **and** its resulting provenance: returns the text plus
    /// the *actual* `(ExtractSource, ReasonCode)`. A native fallback
    /// after a failed / empty / not-compiled OCR is reported as
    /// [`ExtractSource::Fallback`], never mislabelled as
    /// [`ExtractSource::Ocr`]. Always falls back to
    /// native text with a precise `tracing::warn`; never crashes, never
    /// silent-empty. `cls` is the already-computed classification (the
    /// caller classifies once — no double work for the rich path).
    fn route(
        &self,
        doc: &PdfDocument,
        page: usize,
        cls: &PageClassification,
    ) -> crate::Result<(String, ExtractSource, ReasonCode)> {
        let force = matches!(self.opts.mode, ExtractMode::ForceOcr) || self.opts.force_ocr_pages.contains(&page);
        let needs_ocr = force || matches!(cls.kind, PageKind::Scanned | PageKind::ImageText | PageKind::Mixed);
        if !needs_ocr {
            return Ok((doc.extract_text(page)?, ExtractSource::NativeText, cls.reason));
        }
        // The page needs OCR, but this build has no OCR backend — always
        // fall back to native text. ~keep
        tracing::warn!(
            "auto-extract: OCR unavailable (no OCR backend compiled in) \
             for page {page} (kind={:?}); falling back to native \
             text (reason OcrRequestedButUnavailable)",
            cls.kind
        );
        // The classifier wanted OCR but it is unavailable. Do NOT
        // blanket-label the native fallback "partial / OCR-unavailable":
        // if the native text we got is itself high-quality (passes the
        // T0.5 gate) the extraction is genuinely COMPLETE — trust the
        // gate over the routing guess. Only when the native fallback is
        // *also* poor is the result truly degraded. Without this a
        // misclassified text page reported `partial_success` /
        // `ocr_requested_but_unavailable` despite a perfect extraction. ~keep
        let native = doc.extract_text(page)?;
        if !native.trim().is_empty() && text_quality_gate(&native).is_none() {
            Ok((native, ExtractSource::NativeText, ReasonCode::NativeTextHighConfidence))
        } else {
            Ok((native, ExtractSource::Fallback, ReasonCode::OcrRequestedButUnavailable))
        }
    }

    /// Whole-document text (the common LLM/RAG case), reading-ordered
    /// per page.
    pub fn extract_document_text(&self, doc: &PdfDocument) -> crate::Result<String> {
        let n = doc.page_count()?;
        let mut out = String::new();
        for p in 0..n {
            if p > 0 {
                out.push_str("\n\n");
            }
            out.push_str(&self.extract_text(doc, p)?);
        }
        Ok(out)
    }

    /// Rich per-page extraction: classified text + per-region results,
    /// each with a 4-point bbox and a typed [`ReasonCode`] — **never a
    /// bare empty string** (the #1 user-pain fix). Reading-order region
    /// granularity (per-image-region OCR / image-table reconstruction)
    /// is layered by the OCR pipeline tasks without changing this
    /// surface.
    pub fn extract_page(&self, doc: &PdfDocument, page: usize) -> crate::Result<PageExtraction> {
        let cls = doc.classify_page(page)?;

        // Use `route()`'s ACTUAL provenance instead of re-deriving
        // source/reason from `cls.kind` + `cfg!(ocr)` + text-emptiness.
        // The old heuristic mislabelled an OCR-attempted-but-failed
        // native fallback as `source=Ocr, reason=Ok` and computed
        // `ocr_used` from the feature flag rather than real usage.
        // `route()` is the single source of truth and
        // already returns the truthful `(text, source, reason)`;
        // `TextOnly` keeps its classify-free fast path (mirrors
        // `extract_text`). `ocr_used` is now a fact: OCR text was used. ~keep
        let (text, source, reason) = if matches!(self.opts.mode, ExtractMode::TextOnly) {
            (doc.extract_text(page)?, ExtractSource::NativeText, cls.reason)
        } else {
            self.route(doc, page, &cls)?
        };
        let ocr_used = source == ExtractSource::Ocr;
        let bbox = doc
            .get_page_media_box(page)
            .map(|(x0, y0, x1, y1)| Quad::from_xywh(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()))
            .unwrap_or(Quad::from_xywh(0.0, 0.0, 0.0, 0.0));
        // A high-confidence native text result is COMPLETE, not partial
        // — `NativeTextHighConfidence` is a success reason, same as `Ok`
        // (do not report `partial_success` for a full extraction). ~keep
        let status = if text.trim().is_empty() {
            ExtractionStatus::NoTextRecovered
        } else if matches!(reason, ReasonCode::Ok | ReasonCode::NativeTextHighConfidence) {
            ExtractionStatus::Complete
        } else {
            ExtractionStatus::PartialSuccess
        };
        let region = Region {
            bbox,
            kind: RegionKind::Text,
            text: text.clone(),
            table: None,
            confidence: cls.confidence,
            source,
            reason,
        };
        Ok(PageExtraction {
            page,
            kind: cls.kind,
            text,
            regions: vec![region],
            confidence: cls.confidence,
            reason,
            ocr_used,
            status,
        })
    }

    /// Rich whole-document extraction (per-page — never a forced doc
    /// mode, case Q) + aggregate status + `pages_needing_ocr`.
    pub fn extract_document(&self, doc: &PdfDocument) -> crate::Result<DocumentExtraction> {
        let n = doc.page_count()?;
        let mut pages = Vec::with_capacity(n);
        let mut need = Vec::new();
        for p in 0..n {
            let pe = self.extract_page(doc, p)?;
            if matches!(pe.kind, PageKind::Scanned | PageKind::ImageText | PageKind::Mixed) {
                need.push(p);
            }
            pages.push(pe);
        }
        let any_text = pages.iter().any(|p| !p.text.trim().is_empty());
        let all_ok = pages
            .iter()
            .all(|p| matches!(p.reason, ReasonCode::Ok | ReasonCode::NativeTextHighConfidence));
        let status = if !any_text {
            ExtractionStatus::NoTextRecovered
        } else if all_ok {
            ExtractionStatus::Complete
        } else {
            ExtractionStatus::PartialSuccess
        };
        Ok(DocumentExtraction {
            pages,
            status,
            pages_needing_ocr: need,
        })
    }
}

#[cfg(test)]
mod tests;
