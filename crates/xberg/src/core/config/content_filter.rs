//! Cross-extractor content filtering configuration.

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// Cross-extractor content filtering configuration.
///
/// Controls whether "furniture" content (headers, footers, page numbers,
/// watermarks, repeating text) is included in or stripped from extraction
/// results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
/// with format-specific implementation.
///
/// When `None` on `ExtractionConfig`, each extractor uses its current
/// default behavior unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentFilterConfig {
    /// Include running headers in extraction output.
    ///
    /// - PDF: Disables top-margin furniture stripping and prevents the layout
    ///   model from treating `PageHeader`-classified regions as furniture.
    /// - DOC/DOCX/ODT/RTF/HWPX/WordPerfect: Appends header text after the body, in
    ///   markdown and plain output, on the `Header` content layer. DOCX
    ///   emits each distinct header variant (first-page, even, odd) once, not once
    ///   per page.
    /// - ODP: No effect; header text is not emitted.
    /// - HTML/EPUB: Keeps `<header>` element content.
    ///
    /// Default: `false` (headers are stripped or excluded).
    #[serde(default)]
    pub include_headers: bool,

    /// Include running footers in extraction output.
    ///
    /// - PDF: Disables bottom-margin furniture stripping and prevents the layout
    ///   model from treating `PageFooter`-classified regions as furniture.
    /// - DOC/DOCX/ODT/RTF/HWPX/WordPerfect: Appends footer text after the body, in
    ///   markdown and plain output, on the `Footer` content layer. DOCX
    ///   emits each distinct footer variant (first-page, even, odd) once, not once
    ///   per page.
    /// - ODP: No effect; footer text is not emitted.
    /// - HTML/EPUB: Keeps `<footer>` element content.
    ///
    /// Default: `false` (footers are stripped or excluded).
    #[serde(default)]
    pub include_footers: bool,

    /// Include footnote bodies in extraction output.
    ///
    /// - PDF. Includes footnotes recognized during structured extraction in
    ///   the body, including layout-model footnotes and short native notes
    ///   identified by a superscript reference and a tagged bottom rule. By
    ///   default, those native notes stay on the `Footnote` content layer
    ///   while Plain and Markdown omit them from the body. Flat Plain
    ///   extraction does not use this classifier.
    /// - DOC. Includes footnote definitions in extracted output.
    /// - Other formats. No effect currently.
    ///
    /// Defaults to `false`, like `include_headers` and `include_footers`.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub include_footnotes: bool,

    /// Enable the heuristic cross-page repeating text detector.
    ///
    /// When `true` (default), text that repeats verbatim across a supermajority
    /// of pages is classified as furniture and stripped.  Disable this if brand
    /// names or repeated headings are being incorrectly removed by the heuristic.
    ///
    /// This flag also gates a same-page rule: a body paragraph is removed when
    /// its text is also carried by a table detected on the same page (catches
    /// table content that PDF extraction renders both as a table and as body
    /// text). The comparison preserves case and never touches headings, list
    /// items, code blocks, formulas, or captions, so it cannot delete a body
    /// sentence merely for repeating an earlier heading's words (GH#1623).
    ///
    /// Note: when a layout-detection model is active, the model may independently
    /// classify page-header / page-footer / footnote regions as furniture on a
    /// per-page basis. To preserve those regions, set `include_headers = true`,
    /// `include_footers = true`, `include_footnotes = true`, or any combination,
    /// in addition to disabling this flag.
    ///
    /// Primarily affects PDF extraction.
    ///
    /// Default: `true`.
    #[serde(default = "default_true")]
    pub strip_repeating_text: bool,

    /// Include watermark text in extraction output.
    ///
    /// - PDF: Keeps watermark artifacts and arXiv identifiers.
    /// - Other formats: No effect currently.
    ///
    /// Default: `false` (watermarks are stripped).
    #[serde(default)]
    pub include_watermarks: bool,
}

impl Default for ContentFilterConfig {
    fn default() -> Self {
        Self {
            include_headers: false,
            include_footers: false,
            include_footnotes: false,
            strip_repeating_text: true,
            include_watermarks: false,
        }
    }
}
