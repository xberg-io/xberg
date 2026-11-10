//! PDF document extractor.
//!
//! Provides extraction of text, metadata, tables, and images from PDF documents
//! using xberg_native_pdf (pure Rust). Supports both native text extraction and OCR fallback.

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
mod automatic_fallback;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod automatic_fallback_tests;
mod extraction;
#[cfg(feature = "layout-detection")]
mod layout_hints;
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
mod layout_runner;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
mod mixed_tables;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
mod unreadable_script;
#[cfg(all(test, feature = "pdf", feature = "layout-detection", feature = "tokio-runtime"))]
pub(crate) use layout_runner::inherit_test_selected_page_detections;
pub(crate) mod ocr;
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
mod outline_ocr;
mod pages;
#[cfg(all(feature = "pdf", feature = "pdf-pdfium"))]
mod pdfium_engine;
#[cfg(feature = "layout-detection")]
pub(crate) mod reading_order;
#[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
mod region_vlm;
#[cfg(feature = "pdf")]
pub(crate) mod rotation;
mod unmapped_glyphs;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractionMethod, Metadata};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use automatic_fallback::{automatic_page_acceptance_supported, with_native_page_acceptance};
use extraction::extract_all_from_native_document;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use mixed_tables::merge_mixed_ocr_tables;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn extraction_method_after_mixed_ocr(replacements: &ahash::AHashMap<u32, String>) -> ExtractionMethod {
    if replacements.is_empty() {
        ExtractionMethod::Native
    } else {
        ExtractionMethod::Mixed
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn boundaries_for_ocr_output(
    extraction_method: ExtractionMethod,
    text: &str,
    native_boundaries: Option<&[crate::types::PageBoundary]>,
    replacements: Option<&ahash::AHashMap<u32, String>>,
    ocr_page_texts: Option<&[String]>,
) -> Option<Vec<crate::types::PageBoundary>> {
    match extraction_method {
        ExtractionMethod::Native => native_boundaries.map(<[_]>::to_vec),
        ExtractionMethod::Mixed => native_boundaries.map(|boundaries| {
            replacements.map_or_else(
                || boundaries.to_vec(),
                |accepted| ocr::boundaries_after_replacements(boundaries, accepted),
            )
        }),
        ExtractionMethod::Ocr => exact_boundaries_for_page_texts(text, ocr_page_texts?),
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn exact_boundaries_for_page_texts(text: &str, page_texts: &[String]) -> Option<Vec<crate::types::PageBoundary>> {
    let mut boundaries = Vec::with_capacity(page_texts.len());
    let mut search_offset = 0usize;

    for (index, page_text) in page_texts.iter().enumerate() {
        let byte_start = if page_text.is_empty() {
            search_offset
        } else {
            search_offset + text.get(search_offset..)?.find(page_text)?
        };
        let byte_end = byte_start + page_text.len();
        boundaries.push(crate::types::PageBoundary {
            byte_start,
            byte_end,
            page_number: (index + 1) as u32,
        });
        search_offset = byte_end;
    }

    Some(boundaries)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn accepted_mixed_ocr_elements(
    structured_pages: &ahash::AHashMap<u32, InternalDocument>,
) -> Vec<crate::types::OcrElement> {
    let mut page_numbers = structured_pages.keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    page_numbers
        .into_iter()
        .filter_map(|page_number| structured_pages.get(&page_number))
        .filter_map(|page| page.prebuilt_ocr_elements.as_ref())
        .flatten()
        .cloned()
        .collect()
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn accepted_mixed_ocr_tables(structured_pages: &ahash::AHashMap<u32, InternalDocument>) -> Vec<crate::types::Table> {
    let mut page_numbers = structured_pages.keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    page_numbers
        .into_iter()
        .filter_map(|page_number| structured_pages.get(&page_number))
        .flat_map(|page| page.tables.iter())
        .cloned()
        .collect()
}

/// One coordinate frame per OCR page that has both a captured frame (GH#1645) and public
/// elements -- a page whose elements were all filtered out (margins, quality gate) has
/// nothing for a consumer to join the frame to, so it is omitted rather than emitted with an
/// empty element set. Sorted by page number for a deterministic, reader-facing array; the
/// backing `structured_pages` is an `AHashMap`, whose iteration order is unspecified.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn accepted_mixed_ocr_coordinate_frames(
    structured_pages: &ahash::AHashMap<u32, InternalDocument>,
) -> Vec<crate::types::internal::OcrPageCoordinateFrame> {
    let mut page_numbers = structured_pages.keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    page_numbers
        .into_iter()
        .filter_map(|page_number| structured_pages.get(&page_number))
        .filter(|page| {
            page.prebuilt_ocr_elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty())
        })
        .filter_map(|page| page.ocr_coordinate_frame)
        .collect()
}

#[cfg(feature = "pdf")]
const PDF_OUTLINES_MARKER: &[u8] = b"/Outlines";
#[cfg(feature = "pdf")]
const PDF_PREVIOUS_XREF_MARKER: &[u8] = b"/Prev";

#[cfg(feature = "pdf")]
fn contains_pdf_marker(content: &[u8], marker: &[u8]) -> bool {
    memchr::memmem::find(content, marker).is_some()
}

#[cfg(feature = "pdf")]
fn catalog_needs_lopdf_compatibility_pass(catalog: Option<&xberg_native_pdf::object::Object>) -> bool {
    let Some(catalog) = catalog else {
        return true;
    };
    let Some(dictionary) = catalog.as_dict() else {
        return true;
    };
    dictionary.contains_key("Outlines")
}

#[cfg(feature = "pdf")]
fn raw_pdf_needs_lopdf_compatibility_pass(content: &[u8]) -> bool {
    contains_pdf_marker(content, PDF_PREVIOUS_XREF_MARKER) || contains_pdf_marker(content, PDF_OUTLINES_MARKER)
}

/// Reject a document whose page count exceeds `security_limits.max_pages` before any
/// per-page work (layout detection, OCR, rendering) begins (#1451).
///
/// Counts pages with `xberg_native_pdf` first and falls back to `lopdf` when that cannot
/// open the document at all.
///
/// The fallback exists because the `xberg_native_pdf` open is the sole gate on this cap, and
/// `raw_pdf_needs_lopdf_compatibility_pass` below documents that some structures only
/// `lopdf` can walk — so without it, a document `xberg_native_pdf` rejects skips the cap
/// entirely and is then handed to an extraction path that may still recover it. That
/// is the shape a hostile input would take against a security limit.
///
/// Measured, so the comment does not outlive the fact: over the 488 PDFs in
/// `test_documents/`, exactly one defeats the `xberg_native_pdf` count — `corrupt_truncated.pdf`,
/// which `lopdf` also cannot read and which fails extraction anyway. Encrypted documents
/// are *not* among the failures: a PDF's page tree is structure, not string or stream
/// data, so `xberg_native_pdf` counts an AES-256 document's pages without ever authenticating.
/// No password handling is needed here, and adding it would buy nothing.
///
/// A document neither parser can count is let through rather than rejected. `max_pages`
/// is opt-in (`SecurityLimits` defaults it to `None`, the fast path below), and
/// failing closed on "cannot tell" would turn a parse failure into a spurious
/// `TooManyPages`, masking the real error on documents the extraction path may still
/// recover.
#[cfg(feature = "pdf")]
fn enforce_page_limit(content: &[u8], config: &ExtractionConfig) -> Result<()> {
    let max_pages = config.security_limits.as_ref().and_then(|limits| limits.max_pages);
    let Some(max_pages) = max_pages else {
        return Ok(());
    };

    let Some(page_count) = native_page_count(content).or_else(|| lopdf_page_count(content)) else {
        return Ok(());
    };

    Ok(crate::extractors::security::enforce_page_count(
        page_count,
        Some(max_pages),
    )?)
}

/// Whether a failed OCR fallback has left nothing at all to return.
///
/// The automatic fallback exists to replace native text that scored below the OCR trigger, so on
/// failure, returning that native text with a warning is right whenever there is some. When there
/// is none, "success with empty content" is indistinguishable from a legitimately empty PDF: the
/// caller cannot tell that OCR was required and produced nothing, because `quality_score` is
/// documented as not a completeness signal and `ProcessingWarning` is free text.
///
/// `run_ocr_pipeline_for_page` already builds a typed error for exactly this case, and its comment
/// states the intent — "Degrading a per-page failure to a warning must not turn a wholesale OCR
/// failure into a silently empty document". That guarantee was defeated one frame up, in the
/// automatic-fallback arms of `extract_core_native`. The `force_ocr` path never had the bug: it
/// propagates the identical error with `?`.
///
/// Whitespace-only counts as nothing: a document yielding `"\n\n  "` is as total a loss as `""`.
/// So does text of only U+FFFD: it is the same predicate that sends the page to OCR. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn failed_ocr_fallback_is_total_loss(native_text: &str) -> bool {
    native_text_holds_no_content(native_text)
}

#[cfg(feature = "ocr-pipeline")]
use crate::plugins::registry::automatic_ocr_backend_is_registered;

/// Page count via `xberg_native_pdf`. `None` when it cannot open or count the document,
/// in which case the caller falls back to [`lopdf_page_count`].
#[cfg(feature = "pdf")]
fn native_page_count(content: &[u8]) -> Option<usize> {
    xberg_native_pdf::PdfDocument::from_bytes(content.to_vec())
        .ok()?
        .page_count()
        .ok()
}

/// Page count via `lopdf`, for documents `xberg_native_pdf` could not open or count.
#[cfg(feature = "pdf")]
fn lopdf_page_count(content: &[u8]) -> Option<usize> {
    let document = lopdf::Document::load_mem(content).ok()?;
    Some(document.get_pages().len())
}

#[cfg(feature = "pdf")]
fn parsed_pdf_needs_lopdf_compatibility_pass(document: &crate::pdf::native::NativeDocument) -> bool {
    match document.doc.catalog() {
        Ok(catalog) => catalog_needs_lopdf_compatibility_pass(Some(&catalog)),
        Err(error) => {
            tracing::debug!(
                error = %error,
                "xberg_native_pdf catalog inspection failed; retaining lopdf compatibility pass"
            );
            catalog_needs_lopdf_compatibility_pass(None)
        }
    }
}

#[cfg(feature = "pdf")]
fn extract_lopdf_compatibility_data(
    content: &[u8],
) -> (
    Vec<crate::pdf::bookmarks::PdfOutlineEntry>,
    Vec<crate::types::ExtractedUri>,
    Option<Vec<crate::types::DocumentRevision>>,
) {
    match lopdf::Document::load_mem(content) {
        Ok(lopdf_doc) => {
            let entries = crate::pdf::bookmarks::extract_outline_entries(&lopdf_doc);
            let uris = crate::pdf::bookmarks::extract_bookmarks_from_entries(&entries);
            let revisions = crate::pdf::xref_revisions::extract_pdf_xref_revisions(content, &lopdf_doc);
            (entries, uris, revisions)
        }
        Err(_) => (Vec::new(), Vec::new(), None),
    }
}

#[cfg(feature = "pdf")]
struct NativeBlockingOutput {
    extraction: extraction::PdfExtractionPhaseResult,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    image_gap: Option<crate::extraction::image_ocr::pdf_gap::Placement>,
    bookmark_uris: Vec<crate::types::ExtractedUri>,
    pdf_revisions: Option<Vec<crate::types::DocumentRevision>>,
    diagrams: Vec<crate::types::diagram::DiagramGraph>,
    #[cfg(feature = "layout-detection")]
    layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")]
    layout_hints: Option<Vec<Vec<crate::pdf::structure::types::LayoutHint>>>,
    #[cfg(feature = "layout-detection")]
    layout_acceleration_override: Option<crate::core::config::acceleration::AccelerationConfig>,
}

#[cfg(feature = "pdf")]
fn ensure_pdf_not_cancelled(config: &ExtractionConfig) -> Result<()> {
    if config
        .cancel_token
        .as_ref()
        .is_some_and(crate::cancellation::CancellationToken::is_cancelled)
    {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

#[cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn spawn_pdf_blocking<F, T>(work: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| crate::XbergError::Other(format!("native PDF extraction task failed: {error}")))?
}

#[cfg(all(feature = "pdf", any(not(feature = "tokio-runtime"), target_arch = "wasm32")))]
async fn spawn_pdf_blocking<F, T>(work: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    work()
}

#[cfg(feature = "pdf")]
fn extract_native_blocking(
    content: Vec<u8>,
    config: ExtractionConfig,
    #[cfg(feature = "layout-detection")] layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")] layout_results: Option<Vec<crate::pdf::structure::types::PageLayoutResult>>,
    #[cfg(feature = "layout-detection")] layout_hints: Option<Vec<Vec<crate::pdf::structure::types::LayoutHint>>>,
    #[cfg(feature = "layout-detection")] layout_acceleration_override: Option<
        crate::core::config::acceleration::AccelerationConfig,
    >,
    #[cfg(feature = "layout-detection")] region_text_layout: Option<
        crate::engine::layout_region_text::RegionTextLayout,
    >,
) -> Result<NativeBlockingOutput> {
    ensure_pdf_not_cancelled(&config)?;
    let passwords = config
        .pdf_options
        .as_ref()
        .and_then(|options| options.passwords.as_deref())
        .unwrap_or(&[]);
    let raw_compatibility_signal = raw_pdf_needs_lopdf_compatibility_pass(&content);
    let compatibility_data = raw_compatibility_signal.then(|| extract_lopdf_compatibility_data(&content));
    let mut native_document = crate::pdf::native::NativeDocument::open_bytes_with_passwords(&content, passwords)?;

    let compatibility_data = match compatibility_data {
        Some(data) => Some(data),
        None if parsed_pdf_needs_lopdf_compatibility_pass(&native_document) => {
            drop(native_document);
            let data = extract_lopdf_compatibility_data(&content);
            native_document = crate::pdf::native::NativeDocument::open_bytes_with_passwords(&content, passwords)?;
            Some(data)
        }
        None => None,
    };
    let (outline_entries, bookmark_uris, pdf_revisions) = compatibility_data.unwrap_or_default();
    ensure_pdf_not_cancelled(&config)?;
    let diagrams = if wants_dot_output(&config) {
        crate::extraction::diagram::pdf::recover(&mut native_document)
    } else {
        Vec::new()
    };
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let mut image_gap = None;
    let extraction = extract_all_from_native_document(
        native_document,
        &config,
        &outline_entries,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        &mut image_gap,
        #[cfg(feature = "layout-detection")]
        extraction::NativeLayoutInput {
            hints: layout_hints.as_deref(),
            images: layout_images.as_deref(),
            results: layout_results.as_deref(),
            acceleration: layout_acceleration_override.as_ref(),
            region_text: region_text_layout.as_ref(),
        },
    )?;
    ensure_pdf_not_cancelled(&config)?;

    Ok(NativeBlockingOutput {
        extraction,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        image_gap,
        bookmark_uris,
        pdf_revisions,
        diagrams,
        #[cfg(feature = "layout-detection")]
        layout_images,
        #[cfg(feature = "layout-detection")]
        layout_hints,
        #[cfg(feature = "layout-detection")]
        layout_acceleration_override,
    })
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PdfDocumentOrigin {
    Native,
    Mixed,
    Ocr,
}

/// Flat paragraph document for PDF text that arrived without structure.
///
/// `text` is whichever of native or OCR text won the extraction-method decision, and
/// neither is guaranteed LF-only: `crate::pdf::text::fix_pdf_control_chars` explicitly
/// whitelists `\r` as a character to preserve, and the VLM OCR backend returns model
/// markdown verbatim from an HTTP response. Normalize before splitting (#316).
/// `boundaries` must describe `text` itself, not some earlier revision of it -- on the
/// mixed path that means the output of `ocr::boundaries_after_replacements`, since OCR
/// replacement shifts every later offset. When supplied, each paragraph is tagged with
/// the page its start offset falls in.
///
/// Tagging matters well beyond bookkeeping: `ExtractedDocument.pages` is derived from
/// `element.page` (`extraction::derive::build_pages`), so a flat document with no page
/// tags yields `pages: None` -- indistinguishable at the call site from "this document
/// has no pages", and a silent shape change between the native and OCR paths for the
/// same input. Every arm that falls back here previously lost per-page access entirely. ~keep
fn flat_pdf_document(
    text: &str,
    mime_type: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    doc.mime_type = mime_type.to_string();
    let normalized = crate::extraction::transform::normalize_line_endings(text);

    // Normalization rewrites CRLF to LF, which shifts offsets; boundaries are only
    // usable when it changed nothing. Falling back to untagged paragraphs is the
    // pre-existing behaviour, so a CRLF document is no worse off than before.
    let usable: Option<&[crate::types::PageBoundary]> = boundaries.filter(|bounds| {
        normalized.len() == text.len()
            && !bounds.is_empty()
            && bounds
                .iter()
                .all(|b| b.byte_start <= b.byte_end && b.byte_end <= normalized.len())
    });

    let page_at = |offset: usize| -> Option<u32> {
        usable?
            .iter()
            .find(|b| offset >= b.byte_start && offset < b.byte_end)
            .map(|b| b.page_number)
    };

    let mut cursor = 0usize;
    for raw in normalized.split("\n\n") {
        let offset = cursor;
        cursor += raw.len() + 2; // the split consumed the two-byte separator
        let paragraph = raw.trim();
        if paragraph.is_empty() {
            continue;
        }
        // Offset of the trimmed text, so a paragraph sitting just after a page break is
        // attributed to the page its visible content is on rather than the previous one.
        let text_offset = offset + (raw.len() - raw.trim_start().len());
        let element = InternalElement::text(ElementKind::Paragraph, paragraph, 0);
        doc.push_element(match page_at(text_offset) {
            Some(page) => element.with_page(page),
            None => element,
        });
    }
    doc
}

#[cfg(feature = "pdf")]
fn apply_extracted_pdf_metadata(
    metadata: &mut Metadata,
    extracted: crate::pdf::metadata::PdfExtractionMetadata,
    enabled: bool,
) {
    if !enabled {
        return;
    }

    metadata.title = extracted.title;
    metadata.subject = extracted.subject;
    metadata.authors = extracted.authors;
    metadata.keywords = extracted.keywords;
    metadata.created_at = extracted.created_at;
    metadata.modified_at = extracted.modified_at;
    metadata.created_by = extracted.created_by;
    metadata.pages = extracted.page_structure;
    metadata.format = Some(crate::types::FormatMetadata::Pdf(extracted.pdf_specific));
}

const MIN_NATIVE_TOKENS_FOR_STRUCTURE_COVERAGE: usize = 20;
const MIN_STRUCTURED_NATIVE_TOKEN_COVERAGE: f64 = 0.70;

fn normalized_pdf_tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split_whitespace()
        .map(|word| {
            word.trim_matches(|character: char| character.is_ascii_punctuation())
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
}

fn structured_native_token_coverage(document: &InternalDocument, native_text: &str) -> Option<f64> {
    let native_tokens = normalized_pdf_tokens(native_text).collect::<Vec<_>>();
    if native_tokens.len() < MIN_NATIVE_TOKENS_FOR_STRUCTURE_COVERAGE {
        return None;
    }
    let native_token_count = native_tokens.len();

    let mut represented = ahash::AHashMap::<String, usize>::new();
    for token in document
        .elements
        .iter()
        .flat_map(|element| normalized_pdf_tokens(&element.text))
        .chain(
            document
                .tables
                .iter()
                .flat_map(|table| table.cells.iter().flatten())
                .flat_map(|cell| normalized_pdf_tokens(cell)),
        )
    {
        *represented.entry(token).or_default() += 1;
    }

    let mut matched = 0usize;
    for token in native_tokens {
        if let Some(remaining) = represented.get_mut(&token)
            && *remaining > 0
        {
            *remaining -= 1;
            matched += 1;
        }
    }
    Some(matched as f64 / native_token_count as f64)
}

fn select_native_pdf_document(
    text: &str,
    mime_type: &str,
    pre_rendered_doc: Option<InternalDocument>,
    boundaries: Option<&[crate::types::PageBoundary]>,
) -> (InternalDocument, bool) {
    let Some(mut document) = pre_rendered_doc else {
        return (flat_pdf_document(text, mime_type, boundaries), false);
    };
    document.mime_type = mime_type.to_string();

    let coverage = structured_native_token_coverage(&document, text);
    if coverage.is_none_or(|coverage| coverage >= MIN_STRUCTURED_NATIVE_TOKEN_COVERAGE) {
        return (document, true);
    }

    tracing::warn!(
        coverage = coverage.unwrap_or_default(),
        minimum_coverage = MIN_STRUCTURED_NATIVE_TOKEN_COVERAGE,
        "PDF structure omitted substantial native text; using complete native text"
    );
    (flat_pdf_document(text, mime_type, boundaries), false)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[allow(clippy::too_many_arguments)]
fn select_pdf_document(
    extraction_method: ExtractionMethod,
    text: &str,
    mime_type: &str,
    pre_rendered_doc: Option<InternalDocument>,
    ocr_internal_doc: Option<InternalDocument>,
    ocr_results: Option<&ahash::AHashMap<u32, String>>,
    structured_ocr_pages: Option<&ahash::AHashMap<u32, InternalDocument>>,
    boundaries: Option<&[crate::types::PageBoundary]>,
    output_format: &crate::core::config::OutputFormat,
) -> (InternalDocument, PdfDocumentOrigin, bool) {
    let (mut doc, origin, structured) = match extraction_method {
        ExtractionMethod::Native => {
            let (document, structured) = select_native_pdf_document(text, mime_type, pre_rendered_doc, boundaries);
            (document, PdfDocumentOrigin::Native, structured)
        }
        ExtractionMethod::Mixed => {
            // A native document with no text layer at all (a fully scanned PDF) makes
            // `pre_rendered_doc` `None`: `extract_document_structure_from_segments` ran
            // over zero segments and its caller (`extraction.rs`) collapses an empty
            // result to `None` rather than an empty `InternalDocument`. That used to fall
            // straight through to `flat_pdf_document`, which only ever emits `Paragraph`
            // elements from `text` -- discarding every heading/list-item the document-global
            // OCR heuristic (`ocr::heuristically_restructured_ocr_pages`) recovered into
            // `structured_ocr_pages`, because nothing here ever looked at that map. `text`
            // has already had the OCR replacements applied (see `apply_ocr_page_replacements`
            // upstream) so `flat_pdf_document` still seeds the right per-page paragraphs for
            // every page, OCR'd or not; the merge below then replaces the OCR'd pages'
            // flattened paragraphs with their structured elements, exactly like the
            // `Some(pre_rendered_doc)` branch already does for a native document.
            //
            // Gated on `output_format != Plain`: `render_plain` renders straight off
            // `doc.elements`, so merging in `structured_ocr_pages` (paragraph-segmented
            // per-page OCR documents, not the raw line-broken text `flat_pdf_document`
            // splits on) would change Plain-format bytes for a document that previously
            // never took this branch's merge path. Plain output for this `None` case
            // must stay exactly what `flat_pdf_document(text, ..)` alone produced before
            // this fix.
            let had_native_document = pre_rendered_doc.is_some();
            let mut doc = pre_rendered_doc.unwrap_or_else(|| flat_pdf_document(text, mime_type, boundaries));
            let should_merge = had_native_document || *output_format != crate::core::config::OutputFormat::Plain;
            let merged_ocr_content = if should_merge && let Some(results) = ocr_results {
                if let Some(structured_pages) = structured_ocr_pages {
                    ocr::merge_structured_ocr_pages_into_internal_document(&mut doc, results, structured_pages);
                } else {
                    ocr::merge_ocr_pages_into_internal_document(&mut doc, results);
                }
                true
            } else {
                false
            };
            (doc, PdfDocumentOrigin::Mixed, had_native_document || merged_ocr_content)
        }
        ExtractionMethod::Ocr => match ocr_internal_doc {
            Some(doc) => (doc, PdfDocumentOrigin::Ocr, true),
            None => (
                flat_pdf_document(text, mime_type, boundaries),
                PdfDocumentOrigin::Ocr,
                false,
            ),
        },
    };
    if extraction_method == ExtractionMethod::Mixed
        && let (Some(replacements), Some(pages)) = (ocr_results, structured_ocr_pages)
    {
        // Plain output can skip structural merging but still owns the accepted pages' diagnostics. ~keep
        ocr::merge_source_strip_frames(&mut doc, replacements, pages);
    }
    doc.mime_type = mime_type.to_string();
    (doc, origin, structured)
}

fn attach_unrepresented_tables(doc: &mut InternalDocument, tables: Vec<crate::types::Table>) {
    if doc.tables.is_empty() {
        for table in tables {
            doc.push_table(table);
        }
    }
}

/// Share of a table's cell tokens that must already be in the element stream
/// before the table counts as represented; also the share of a table's or a
/// paragraph's tokens a VLM region answer must carry to replace it.
const MIN_TABLE_TOKEN_REPRESENTATION: f64 = 0.90;
/// Cell-token count below which the containment check abstains: on a handful of
/// tokens an incidental overlap with unrelated prose is likely, and injecting a
/// duplicate is cheaper than dropping a real table.
const MIN_TABLE_TOKENS_FOR_CONTAINMENT: usize = 8;

/// Tokens of `texts`, as a consumable multiset.
fn token_multiset<'a>(texts: impl IntoIterator<Item = &'a str>) -> ahash::AHashMap<String, usize> {
    let mut represented = ahash::AHashMap::<String, usize>::new();
    for text in texts {
        for token in normalized_pdf_tokens(text) {
            *represented.entry(token).or_default() += 1;
        }
    }
    represented
}

/// Tokens the element stream already renders, as a consumable multiset.
fn element_token_multiset(doc: &InternalDocument) -> ahash::AHashMap<String, usize> {
    token_multiset(doc.elements.iter().map(|element| element.text.as_str()))
}

/// Whether `table`'s cell text is already carried by the element stream.
///
/// Compares on `normalized_pdf_tokens`, so whitespace, line wrapping and edge
/// punctuation do not matter — a table reconstructed from prose lines that are
/// still in the stream verbatim reads as represented. Accounting is multiset
/// based like [`structured_native_token_coverage`]: each text occurrence backs at
/// most one cell occurrence, and the occurrences are consumed only when the table
/// is judged represented, so one table cannot mask the next.
///
/// Both bailouts err toward returning `false` (inject), because a duplicated
/// table costs precision while a dropped one costs content.
fn table_is_represented(table: &crate::types::Table, represented: &mut ahash::AHashMap<String, usize>) -> bool {
    let tokens = token_multiset(table.cells.iter().flatten().map(String::as_str));
    if tokens.values().sum::<usize>() < MIN_TABLE_TOKENS_FOR_CONTAINMENT {
        return false;
    }
    take_if_represented(tokens, represented)
}

/// Takes `tokens` out of `pool` when the pool holds at least `MIN_TABLE_TOKEN_REPRESENTATION` of them, so
/// two texts cannot claim the same tokens. Without verifiable tokens, preserve the original content. ~keep
fn take_if_represented(tokens: ahash::AHashMap<String, usize>, pool: &mut ahash::AHashMap<String, usize>) -> bool {
    let total: usize = tokens.values().sum();
    if total == 0 {
        return false;
    }
    let matched: usize = tokens
        .iter()
        .map(|(token, count)| (*count).min(pool.get(token).copied().unwrap_or_default()))
        .sum();
    if (matched as f64 / total as f64) < MIN_TABLE_TOKEN_REPRESENTATION {
        return false;
    }
    for (token, count) in tokens {
        if let Some(remaining) = pool.get_mut(&token) {
            *remaining = remaining.saturating_sub(count);
        }
    }
    true
}

fn inject_unrepresented_table_elements(doc: &mut InternalDocument, allow_injection: bool) {
    if !allow_injection
        || doc
            .elements
            .iter()
            .any(|element| matches!(element.kind, ElementKind::Table { .. }))
    {
        return;
    }

    // The `Table`-element guard above only ever fires on the structured path;
    // `flat_pdf_document` builds nothing but `Paragraph`s, so on the flat path
    // every detected table was injected on top of native text that already
    // contained the words it was reconstructed from — the same content rendered
    // twice (pdfa_031). Inject only the tables whose cells are genuinely absent. ~keep
    let mut represented = element_token_multiset(doc);
    let unrepresented = (0..doc.tables.len() as u32)
        .filter(|table_index| !table_is_represented(&doc.tables[*table_index as usize], &mut represented))
        .collect::<Vec<_>>();
    for table_index in unrepresented {
        doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
    }
}

/// Surface filled AcroForm/XFA field values in rendered content (issue #64).
///
/// `doc.form_fields` already reaches the typed `ExtractedDocument.form_fields`
/// API, but nothing renders it into `content`. For the plain-text path this is
/// usually harmless: `native::text`'s `append_missing_widget_values` already
/// splices Widget `/V` values into the flat native text before it is chopped
/// into `Paragraph` elements. But the *structured* path (Markdown/HTML/Djot,
/// built from `pdf::native::hierarchy`'s span segments) never sees that
/// splice — a filled, non-flattened form renders with none of its entered
/// values.
///
/// Follows `inject_unrepresented_table_elements`'s pattern: push one
/// `ElementKind::Paragraph` per field that has a non-empty value and isn't
/// already present verbatim somewhere in the document (the containment check
/// is what keeps the plain-text path, which already has the value, from
/// getting a duplicate).
fn inject_unrepresented_form_field_elements(doc: &mut InternalDocument, form_fields: &[crate::types::PdfFormField]) {
    for field in form_fields {
        let Some(value) = field.value.as_ref().filter(|v| !v.is_empty()) else {
            continue;
        };
        if doc.elements.iter().any(|element| element.text.contains(value.as_str())) {
            continue;
        }
        let display_name = if field.full_name.is_empty() {
            field.name.as_str()
        } else {
            field.full_name.as_str()
        };
        doc.push_element(InternalElement::text(
            ElementKind::Paragraph,
            format!("{display_name}: {value}"),
            0,
        ));
    }
}

/// Compute language/dictionary-implausible pages (issue #1696), record them on
/// `pdf_metadata.pdf_specific.implausible_text_pages`, and merge them into
/// `pdf_metadata.pdf_specific.scanned_pages` so `OcrStrategy::ScannedPages` (via
/// `scanned_pages_to_ocr`, which reads `scanned_pages`) picks the signal up for free --
/// mirroring how `native::metadata::ocr_routing_pages` already merges `fabricated_text_pages`
/// into the same field for the provenance signal.
///
/// Runs unconditionally (using the default `OcrQualityThresholds` when `config.ocr` is `None`)
/// so `ScannedPages` and the metadata field both see the signal regardless of whether the
/// caller configured OCR explicitly. Whenever `Auto` will not act on the signal, a deduped
/// warning is pushed instead, keeping the defect visible rather than silently discarded; when
/// it will, the warning is suppressed, since it would otherwise tell the caller the pages were
/// dropped on the floor while `Auto` was in fact about to route them. That "will it run"
/// question is answered by `near_empty_ocr_fallback_applies` and nothing else -- see the
/// comment on the warning below for why a second derivation of it is not acceptable. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn record_implausible_text_pages(
    config: &ExtractionConfig,
    pdf_metadata: &mut crate::pdf::metadata::PdfExtractionMetadata,
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) {
    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
    let thresholds = ocr_config.effective_thresholds();

    let scan = ocr::scan_text_plausibility(
        native_text,
        boundaries,
        pdf_metadata.pdf_specific.page_count,
        &thresholds,
    );

    pdf_metadata.pdf_specific.implausible_text_pages = thresholds
        .enable_plausibility_ocr_routing
        .then(|| scan.implausible.clone());

    // An abstention is not a pass. Say so, or an empty `implausible_text_pages` reads as a
    // clean bill of health on a document the check never got to look at (issue #1709). Forced
    // OCR is the exception: it discards the native layer, so the caller never receives the text
    // this notice is about and has already taken the action it would advise. ~keep
    if !config.force_ocr && scan.judged == 0 && !scan.unjudged.is_empty() {
        crate::core::diagnostics::push_warning_deduped(
            warnings,
            crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "No page of this document holds enough prose to read, so the language check \
                     for a wrong glyph-to-Unicode mapping could not judge any of the {} page(s) \
                     it examined (issue #1709). An empty `implausible_text_pages` reports here \
                     that the check did not run, not that the text layer is correct. If the text \
                     looks wrong, extract the document again with OCR forced.",
                    scan.unjudged.len()
                )),
            },
        );
    }

    let implausible_pages = scan.implausible;
    if implausible_pages.is_empty() {
        return;
    }

    let mut merged = pdf_metadata.pdf_specific.scanned_pages.clone().unwrap_or_default();
    merged.extend(implausible_pages.iter().copied());
    merged.sort_unstable();
    merged.dedup();
    pdf_metadata.pdf_specific.scanned_pages = Some(merged);

    // GH#1890: this must call the routing predicate, never re-derive it. The earlier
    // `config.ocr.is_none() && ocr_near_empty_fallback != Some(true)` agreed with
    // `near_empty_ocr_fallback_applies` only until GH#1752 added `Some(false)`: with an `ocr`
    // block and the fallback switched off, the `else if` in `extract_core_native` that gates the
    // routing does not run AND that condition suppressed this warning, so the caller got garbage
    // text with no diagnostic at all. `Some(true)` with no `ocr` block and no registered automatic
    // backend diverged the same way. ~keep
    if !near_empty_ocr_fallback_applies(config, native_text) {
        crate::core::diagnostics::push_warning_deduped(
            warnings,
            crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "Page(s) {implausible_pages:?} do not read as any real detectable language, \
                     suggesting a wrong glyph-to-Unicode mapping (issue #1696), but the automatic \
                     OCR fallback does not run for this extraction, so they were not routed to \
                     OCR. Set `ocr`, and leave `ocr_near_empty_fallback` unset or set it to \
                     `true`, to route these pages to OCR."
                )),
            },
        );
    }
}

/// Whether `OcrStrategy::ScannedPages` folds the per-page text-quality gate into its page
/// selection, on top of the pages scan detection flagged.
///
/// `ExtractionConfig::ocr_scanned_page_quality_gate` is the caller's explicit answer;
/// `None` derives it from whether an `ocr` block is present, which is what this condition
/// was before GH#1752 gave the behaviour a setting of its own. `Some(true)` without an `ocr`
/// block also requires a registered automatic backend, mirroring the sibling
/// `near_empty_ocr_fallback_applies`'s own `Some(true)` arm (its doc comment states the same
/// requirement) -- without this, a whole-document failure inside the gate selected every page
/// for OCR with nothing able to run it, turning a would-be native-text degradation into a
/// hard extraction failure. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn scanned_page_quality_gate_enabled(config: &ExtractionConfig) -> bool {
    config
        .ocr_scanned_page_quality_gate
        .map(|on| on && (config.ocr.is_some() || automatic_ocr_backend_is_registered()))
        .unwrap_or(config.ocr.is_some())
}

/// True when the native text is blank or holds only U+FFFD. The extractor keeps one U+FFFD for
/// each Type0 glyph without a Unicode value, so a page that lost every glyph reads as U+FFFD
/// and must count as absent text. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn native_text_holds_no_content(native_text: &str) -> bool {
    native_text.chars().all(|c| c.is_whitespace() || c == '\u{FFFD}')
}

/// Whether the `Auto` near-empty fallback branch runs for this document.
///
/// `ExtractionConfig::ocr_near_empty_fallback` is the caller's explicit answer:
///
/// - `Some(true)` runs the branch whenever there is something to run it with, so the
///   text-quality gate inside it -- not the much narrower "no text at all" test -- decides.
///   An automatic OCR backend must be registered, exactly as #1338's carve-out requires.
/// - `Some(false)` never runs it, even with an explicit `ocr` block.
/// - `None` derives the answer the way this condition did before GH#1752 gave the behaviour
///   a setting of its own: always with an `ocr` block, and otherwise only for genuinely
///   absent native text, so a legitimately sparse PDF stays native under a default config.
///   ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn near_empty_ocr_fallback_applies(config: &ExtractionConfig, native_text: &str) -> bool {
    match config.ocr_near_empty_fallback {
        Some(false) => false,
        Some(true) => config.ocr.is_some() || automatic_ocr_backend_is_registered(),
        None => {
            config.ocr.is_some() || (native_text_holds_no_content(native_text) && automatic_ocr_backend_is_registered())
        }
    }
}

/// Pages to OCR under `OcrStrategy::ScannedPages`, 1-indexed.
///
/// The union of detected scans and pages failing the text-quality gate, so never
/// a subset of what `Auto` would OCR -- unless the gate is off, in which case only
/// detected scans are selected. See [`scanned_page_quality_gate_enabled`].
///
/// `None` means fall through to the `Auto` gate: wrong strategy, no page
/// qualifies, or the gate wants the whole document rather than a page subset.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn scanned_pages_to_ocr(
    config: &ExtractionConfig,
    pdf_metadata: &crate::pdf::metadata::PdfExtractionMetadata,
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
) -> Option<Vec<u32>> {
    use crate::core::config::OcrStrategy;

    if !matches!(config.ocr_strategy, OcrStrategy::ScannedPages { .. }) {
        return None;
    }

    let mut pages = pdf_metadata.pdf_specific.scanned_pages.clone()?;

    if scanned_page_quality_gate_enabled(config) {
        // Thresholds come from the caller's `ocr` block when there is one; the gate can now
        // be switched on without one, and then reads the same defaults every other
        // no-explicit-block OCR route reads. ~keep
        let default_ocr_config = crate::core::config::OcrConfig::default();
        let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
        let decision = ocr::evaluate_per_page_ocr(
            native_text,
            boundaries,
            pdf_metadata.pdf_specific.page_count,
            &ocr_config.effective_thresholds(),
        );
        if decision.whole_doc_failure {
            // A whole-document failure is itself a `ScannedPages` signal (see
            // discarding the signal and relying on the caller's `Auto` fallthrough.
            let page_count = pdf_metadata.pdf_specific.page_count.unwrap_or(0);
            return if page_count == 0 {
                None
            } else {
                Some((1..=page_count).collect())
            };
        }
        pages.extend(decision.failing_pages);
    }

    pages.sort_unstable();
    pages.dedup();

    if pages.is_empty() { None } else { Some(pages) }
}
use pages::{assign_hierarchy_to_pages, assign_tables_and_images_to_pages, reconcile_native_page_tables};

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn replace_tables_with_ocr_output(tables: &mut Vec<crate::types::Table>, mut ocr_tables: Vec<crate::types::Table>) {
    if ocr_tables.is_empty() {
        return;
    }

    ocr_tables.sort_by_key(|table| table.page_number);
    *tables = ocr_tables;
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
fn prepare_ocr_layout_inputs(
    images: Vec<image::RgbImage>,
    mut detections: Vec<crate::layout::DetectionResult>,
) -> (Vec<image::DynamicImage>, Vec<crate::layout::DetectionResult>) {
    if detections.len() != images.len() {
        tracing::warn!(
            images = images.len(),
            detections = detections.len(),
            "OCR layout input cardinality mismatch; discarding detections while reusing page rasters"
        );
        detections = images
            .iter()
            .map(|image| crate::layout::DetectionResult {
                page_width: image.width(),
                page_height: image.height(),
                detections: Vec::new(),
            })
            .collect();
    } else {
        for (page_index, (image, detection)) in images.iter().zip(&mut detections).enumerate() {
            if detection.page_width != image.width() || detection.page_height != image.height() {
                tracing::warn!(
                    page = page_index + 1,
                    image_width = image.width(),
                    image_height = image.height(),
                    detection_width = detection.page_width,
                    detection_height = detection.page_height,
                    "OCR layout dimensions mismatch; discarding detections for this page"
                );
                *detection = crate::layout::DetectionResult {
                    page_width: image.width(),
                    page_height: image.height(),
                    detections: Vec::new(),
                };
            }
        }
    }

    let images = images.into_iter().map(image::DynamicImage::ImageRgb8).collect();
    (images, detections)
}

/// Layout gate audit trail: 1-indexed skipped pages and per-page decision
/// reasons, both `None` unless the `Auto` gate or a `pages` list ran on the
/// whole-document OCR route (the mixed OCR route records none).
///
/// Unused only when both OCR features and the pdf + layout-detection pair
/// are off; the allow is scoped wider because cfg algebra for "either user
/// present" is not worth the noise.
#[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(dead_code))]
type OcrLayoutGateDecisions = (Option<Vec<u32>>, Option<Vec<String>>);

/// Whether the markdown layout pass's rasters may be reused as OCR input.
///
/// Under `LayoutStrategy::Auto` or a `pages` list with `SkipRender`,
/// gate-skipped pages carry 64x64 white placeholders. Rotated pages also
/// remain in the display-space coordinate frame required by Markdown layout.
/// Reuse is therefore allowed only when every page ran the model and no page
/// has an effective `/Rotate`; otherwise OCR reruns layout with normalized
/// rasters. ~keep
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
fn markdown_layout_reusable_for_ocr(
    decisions: Option<&[crate::pdf::layout_gate::PageGateDecision]>,
    page_rotations: &[u32],
) -> bool {
    decisions.is_none_or(|decisions| decisions.iter().all(|decision| decision.run_layout))
        && markdown_layout_usable_for_native(page_rotations)
}

/// Whether the markdown layout rasters may still feed the native path when OCR cannot reuse
/// them: a rotated page's detections are in display space, but formula recognition maps them
/// against the MediaBox (`pdf_page_sizes_pt`).
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
fn markdown_layout_usable_for_native(page_rotations: &[u32]) -> bool {
    page_rotations.iter().all(|rotation| rotation.is_multiple_of(360))
}

/// Convert gate decisions into the metadata representation.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn layout_gate_metadata(decisions: Option<&[crate::pdf::layout_gate::PageGateDecision]>) -> OcrLayoutGateDecisions {
    let Some(decisions) = decisions else {
        return (None, None);
    };
    let gated = decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| !decision.run_layout)
        .map(|(page_index, _)| page_index as u32 + 1)
        .collect();
    let reasons = decisions
        .iter()
        .map(|decision| decision.reason.wire_name().to_string())
        .collect();
    (Some(gated), Some(reasons))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
fn config_with_layout_acceleration_override(
    config: &ExtractionConfig,
    acceleration_override: Option<crate::core::config::acceleration::AccelerationConfig>,
) -> std::borrow::Cow<'_, ExtractionConfig> {
    let Some(acceleration) = acceleration_override else {
        return std::borrow::Cow::Borrowed(config);
    };

    let mut effective = config.clone();
    if let Some(layout) = effective.layout.as_mut() {
        layout.acceleration = Some(acceleration);
    } else {
        effective.acceleration = Some(acceleration);
    }
    std::borrow::Cow::Owned(effective)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
type WholePageOcrOutput = (
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    OcrLayoutGateDecisions,
    Option<crate::types::ProcessingWarning>,
    Vec<crate::types::ProcessingWarning>,
    bool,
);

#[cfg(all(test, paddle_ocr, feature = "ocr"))]
async fn run_ocr_with_layout(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<ocr::PageOcrHints>,
    #[cfg(feature = "layout-detection")] precomputed_layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")] precomputed_layout_detections: Option<Vec<crate::layout::DetectionResult>>,
    #[cfg(feature = "layout-detection")] precomputed_layout_acceleration_override: Option<
        crate::core::config::acceleration::AccelerationConfig,
    >,
) -> ocr::OcrRunResult<WholePageOcrOutput> {
    run_ocr_with_layout_pages(
        content,
        config,
        path,
        PageOcrLayoutContext {
            pages: None,
            hints: page_ocr_hints,
        },
        #[cfg(feature = "layout-detection")]
        precomputed_layout_images,
        #[cfg(feature = "layout-detection")]
        precomputed_layout_detections,
        #[cfg(feature = "layout-detection")]
        precomputed_layout_acceleration_override,
    )
    .await
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
struct PageOcrLayoutContext<'a> {
    pages: Option<&'a crate::core::layout_pages::LayoutPageSelection>,
    hints: Option<ocr::PageOcrHints>,
}

/// Run OCR with optional layout detection on PDF bytes.
///
/// Reuses detections from native extraction when available. Otherwise, when
/// layout detection is configured, it runs a soft-failing layout pass before
/// OCR. Layout failures are logged and OCR continues without layout hints.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
async fn run_ocr_with_layout_pages(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    context: PageOcrLayoutContext<'_>,
    #[cfg(feature = "layout-detection")] precomputed_layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")] precomputed_layout_detections: Option<Vec<crate::layout::DetectionResult>>,
    #[cfg(feature = "layout-detection")] precomputed_layout_acceleration_override: Option<
        crate::core::config::acceleration::AccelerationConfig,
    >,
) -> ocr::OcrRunResult<WholePageOcrOutput> {
    let PageOcrLayoutContext {
        pages: layout_pages,
        hints: page_ocr_hints,
    } = context;
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut layout_warning = None;
    #[cfg(not(all(feature = "pdf", feature = "layout-detection")))]
    let layout_warning = None;

    // `xberg_native_pdf` glyph-drop warnings captured while the layout pass rendered
    // its pages (#353); populated only on the layout-detection path, since
    // that is the only render call site routed through `spawn_blocking`.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut layout_glyph_drop_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    #[cfg(not(all(feature = "pdf", feature = "layout-detection")))]
    let layout_glyph_drop_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut ocr_layout_gate_decisions: OcrLayoutGateDecisions = (None, None);
    #[cfg(not(all(feature = "pdf", feature = "layout-detection")))]
    let ocr_layout_gate_decisions: OcrLayoutGateDecisions = (None, None);

    let _ = &layout_pages;
    #[cfg(feature = "layout-detection")]
    let mut layout_acceleration_override = precomputed_layout_acceleration_override;

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut layout_raster_dpis = crate::pdf::render::OcrRasterDpis::new();
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let owned_layout = if precomputed_layout_detections.is_none() || precomputed_layout_images.is_none() {
        if let Some(layout_config) = config.resolved_layout_config() {
            let thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
            let default_security_limits = crate::extractors::security::SecurityLimits::default();
            let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
            match layout_runner::run_layout_for_ocr_with_pages(
                content,
                layout_config.as_ref(),
                thread_budget,
                security_limits,
                config.images.as_ref(),
                layout_pages,
            )
            .await
            {
                Ok((
                    layout_runner::LayoutAttempt {
                        output:
                            layout_runner::LayoutRunOutput {
                                data: Some(layout),
                                gate_decisions,
                                raster_dpis,
                            },
                        acceleration_override,
                        warning,
                    },
                    glyph_drop_warnings,
                )) => {
                    layout_acceleration_override = acceleration_override;
                    layout_raster_dpis = raster_dpis;
                    layout_warning = warning;
                    layout_glyph_drop_warnings = glyph_drop_warnings;
                    ocr_layout_gate_decisions = layout_gate_metadata(gate_decisions.as_deref());
                    Some(layout)
                }
                Ok((
                    layout_runner::LayoutAttempt {
                        output:
                            layout_runner::LayoutRunOutput {
                                data: None,
                                gate_decisions,
                                ..
                            },
                        acceleration_override: _,
                        warning,
                    },
                    glyph_drop_warnings,
                )) => {
                    layout_warning = warning;
                    layout_glyph_drop_warnings = glyph_drop_warnings;
                    tracing::info!("OCR layout: layout gate skipped every page, continuing without layout assembly");
                    ocr_layout_gate_decisions = layout_gate_metadata(gate_decisions.as_deref());
                    None
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "OCR layout detection failed; continuing without layout assembly"
                    );
                    layout_warning = Some(layout_runner::layout_failure_warning(&error));
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let layout_inputs = match (precomputed_layout_images, precomputed_layout_detections) {
        (Some(images), Some(detections)) => Some((images, detections)),
        _ => owned_layout.map(|(images, _, _, detections)| (images, detections)),
    };
    #[cfg(all(not(feature = "pdf"), feature = "layout-detection"))]
    let layout_inputs = precomputed_layout_images.zip(precomputed_layout_detections);

    #[cfg(feature = "layout-detection")]
    let prepared_layout_inputs =
        layout_inputs.map(|(images, detections)| prepare_ocr_layout_inputs(images, detections));
    #[cfg(feature = "layout-detection")]
    let ocr_images = prepared_layout_inputs.as_ref().map(|(images, _)| images.as_slice());
    #[cfg(feature = "layout-detection")]
    let layout_detections = prepared_layout_inputs
        .as_ref()
        .map(|(_, detections)| detections.as_slice());

    #[cfg(feature = "layout-detection")]
    let effective_config = config_with_layout_acceleration_override(config, layout_acceleration_override);
    #[cfg(feature = "layout-detection")]
    let config = effective_config.as_ref();

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let page_ocr_hints = {
        let mut hints = page_ocr_hints;
        if prepared_layout_inputs.is_some() && !layout_raster_dpis.is_empty() {
            hints.get_or_insert_with(Default::default).raster_dpis = Some(std::sync::Arc::new(layout_raster_dpis));
        }
        hints
    };

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
    #[cfg(all(
        paddle_ocr,
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let ocr_images_absent = {
        #[cfg(feature = "layout-detection")]
        let absent = ocr_images.is_none();
        #[cfg(not(feature = "layout-detection"))]
        let absent = true;
        absent
    };

    #[cfg(all(
        paddle_ocr,
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let mut accepted_first_pass = None;
    #[cfg(all(
        paddle_ocr,
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    if ocr_config.pipeline.is_none()
        && ocr_config.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
        && ocr_config.backend == "tesseract"
        && ocr_images_absent
        && let Some(pipeline) = ocr_config.effective_pipeline()
    {
        accepted_first_pass = ocr::accepted_single_page_first_pass(
            content,
            config,
            path,
            page_ocr_hints.clone(),
            &pipeline.quality_thresholds,
        )
        .await;
    }

    #[cfg(all(
        paddle_ocr,
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let has_accepted_first_pass = accepted_first_pass.is_some();
    #[cfg(not(all(
        paddle_ocr,
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    )))]
    let has_accepted_first_pass = false;

    if let Some(pipeline) = ocr_config.effective_pipeline()
        && !has_accepted_first_pass
    {
        #[cfg(paddle_ocr)]
        // The synthesized classical pipeline is an automatic page-quality fallback. Running
        // it here as one document lets a good page's score hide a bad page, so use the same
        // page-local runner as selected-page OCR. Explicit pipelines retain their configured
        // document scope because their stages may intentionally process the document. ~keep
        if ocr_config.pipeline.is_none()
            && ocr_config.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
            && ocr_config.backend == "tesseract"
        {
            let (text, tables, elements, document, usage, page_texts, rasters, formulas, preprocessing, confidence) =
                Box::pin(ocr::extract_full_document_ocr_pipeline_per_page(
                    content,
                    config,
                    path,
                    page_ocr_hints,
                    #[cfg(feature = "layout-detection")]
                    prepared_layout_inputs.as_ref(),
                ))
                .await?;
            #[cfg(feature = "formula-recognition")]
            let (mut document, mut formulas) = (document, formulas);
            #[cfg(feature = "formula-recognition")]
            if let (Some((images, detections)), Some(layout)) =
                (prepared_layout_inputs.as_ref(), config.layout.as_ref())
            {
                recognize_pdf_formula_regions(document.as_mut(), &mut formulas, images, detections, layout, content)
                    .await;
            }
            return Ok((
                text,
                tables,
                elements,
                document,
                usage,
                page_texts,
                rasters,
                formulas,
                preprocessing,
                confidence,
                ocr_layout_gate_decisions,
                layout_warning,
                layout_glyph_drop_warnings,
                false,
            ));
        }

        let (
            mut text,
            mut ocr_tables,
            ocr_elements,
            mut pipeline_doc,
            llm_usage,
            ocr_pts,
            pipeline_rasters,
            pipeline_formulas,
            preprocessing,
            pipeline_ocr_confidence,
        ) = Box::pin(ocr::run_ocr_pipeline(
            Some(content),
            #[cfg(feature = "layout-detection")]
            ocr_images,
            #[cfg(not(feature = "layout-detection"))]
            None,
            #[cfg(feature = "layout-detection")]
            layout_detections,
            config,
            &pipeline,
            path,
            page_ocr_hints,
        ))
        .await?;
        // GH#1892: the whole-document route repaired nothing, so `numeric_repair` was honoured for
        // `force_ocr_pages` and silently ignored for `force_ocr` under one identical flag. Placed
        // ahead of formula recognition on purpose -- see the function's own doc comment. ~keep
        if ocr::numeric_repair_enabled(config) {
            ocr::apply_numeric_repair_to_whole_document_ocr(&mut text, pipeline_doc.as_mut(), &mut ocr_tables);
        }
        #[cfg(feature = "formula-recognition")]
        let (mut pipeline_doc, mut pipeline_formulas) = (pipeline_doc, pipeline_formulas);
        #[cfg(feature = "formula-recognition")]
        if let (Some((images, detections)), Some(layout)) = (prepared_layout_inputs.as_ref(), config.layout.as_ref()) {
            recognize_pdf_formula_regions(
                pipeline_doc.as_mut(),
                &mut pipeline_formulas,
                images,
                detections,
                layout,
                content,
            )
            .await;
        }
        return Ok((
            text,
            ocr_tables,
            ocr_elements,
            pipeline_doc,
            llm_usage,
            ocr_pts,
            pipeline_rasters,
            pipeline_formulas,
            preprocessing,
            pipeline_ocr_confidence,
            ocr_layout_gate_decisions,
            layout_warning,
            layout_glyph_drop_warnings,
            false,
        ));
    }

    let (
        (
            mut text,
            _mean_conf,
            mut ocr_tables,
            ocr_elements,
            mut ocr_doc,
            llm_usage,
            ocr_pts,
            ocr_rasters,
            formulas,
            preprocessing,
            ocr_confidence,
        ),
        discarded_visible_native_page,
    ) = {
        #[cfg(all(
            paddle_ocr,
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let first_pass = accepted_first_pass;
        #[cfg(not(all(
            paddle_ocr,
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        )))]
        let first_pass: Option<_> = None;
        match first_pass {
            Some(output) => output,
            None => {
                Box::pin(ocr::extract_with_ocr_with_page_hints_and_evidence(
                    Some(content),
                    #[cfg(feature = "layout-detection")]
                    ocr_images,
                    #[cfg(not(feature = "layout-detection"))]
                    None,
                    #[cfg(feature = "layout-detection")]
                    layout_detections,
                    config,
                    path,
                    page_ocr_hints,
                ))
                .await?
            }
        }
    };
    // GH#1892: same repair as the pipeline branch above -- this is the other half of the
    // whole-document route, the one `force_ocr` and the near-empty `Auto` fallback reach. ~keep
    if ocr::numeric_repair_enabled(config) {
        ocr::apply_numeric_repair_to_whole_document_ocr(&mut text, ocr_doc.as_mut(), &mut ocr_tables);
    }
    #[cfg(feature = "formula-recognition")]
    let (mut ocr_doc, mut formulas) = (ocr_doc, formulas);
    #[cfg(feature = "formula-recognition")]
    if let (Some((images, detections)), Some(layout)) = (prepared_layout_inputs.as_ref(), config.layout.as_ref()) {
        recognize_pdf_formula_regions(ocr_doc.as_mut(), &mut formulas, images, detections, layout, content).await;
    }
    Ok((
        text,
        ocr_tables,
        ocr_elements,
        ocr_doc,
        llm_usage,
        ocr_pts,
        ocr_rasters,
        formulas,
        preprocessing,
        ocr_confidence,
        ocr_layout_gate_decisions,
        layout_warning,
        layout_glyph_drop_warnings,
        discarded_visible_native_page,
    ))
}

/// The formula detections of one page, sorted by raw `y1` then `x1`.
///
/// This comparator deliberately matches the order the OCR side channel emits
/// formulas in (`glm_ocr_backend::process_paired` sorts its regions the same
/// way). Pairing falls back to position when the page has no geometry or
/// overlap cannot pair every region, so both sides must sort identically. It
/// is NOT the row-banded reading order the image extractor uses for element
/// layout; do not "unify" them while that fallback exists.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn formula_regions_in_reading_order(
    detection: &crate::layout::DetectionResult,
) -> Vec<&crate::layout::LayoutDetection> {
    let mut regions: Vec<&crate::layout::LayoutDetection> = detection
        .detections
        .iter()
        .filter(|d| matches!(d.class_name, crate::layout::LayoutClass::Formula))
        .collect();
    regions.sort_by(|a, b| a.bbox.y1.total_cmp(&b.bbox.y1).then(a.bbox.x1.total_cmp(&b.bbox.x1)));
    regions
}

#[cfg(all(
    test,
    feature = "pdf",
    feature = "layout-detection",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
mod layout_page_selection_tests;

#[cfg(all(test, feature = "formula-recognition", feature = "layout-detection"))]
mod formula_region_tests;

/// A page raster the formula recognizer can crop, however the flow stores it.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
trait AsPageRgb: Sync {
    fn page_rgb(&self) -> std::borrow::Cow<'_, image::RgbImage>;
}
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
impl AsPageRgb for image::DynamicImage {
    fn page_rgb(&self) -> std::borrow::Cow<'_, image::RgbImage> {
        std::borrow::Cow::Owned(self.to_rgb8())
    }
}
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
impl AsPageRgb for image::RgbImage {
    fn page_rgb(&self) -> std::borrow::Cow<'_, image::RgbImage> {
        std::borrow::Cow::Borrowed(self)
    }
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn bbox_intersection_area(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let ix = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
    let iy = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
    ix * iy
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn bbox_area(b: &crate::types::BoundingBox) -> f64 {
    (b.x1 - b.x0) * (b.y1 - b.y0)
}

/// Intersection area over the smaller box's area, in a shared coordinate
/// space. 0.0 when either box is degenerate.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn bbox_overlap_ratio(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let min_area = bbox_area(a).min(bbox_area(b));
    if min_area <= 0.0 {
        0.0
    } else {
        bbox_intersection_area(a, b) / min_area
    }
}

/// Intersection over union. Breaks ties between overlaps of 1.0, where one
/// box sits inside several others.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn bbox_iou(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let intersection = bbox_intersection_area(a, b);
    let union = bbox_area(a) + bbox_area(b) - intersection;
    if union <= 0.0 { 0.0 } else { intersection / union }
}

/// A detection must cover a slot's box (or vice versa) by at least this
/// fraction of the smaller area to pair with it geometrically.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
const FORMULA_PAIR_MIN_OVERLAP: f64 = 0.5;

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn complete_formula_assignment(
    start: usize,
    choices: &[Vec<usize>],
    assigned: &mut [Option<usize>],
    slot_owners: &mut [Option<usize>],
) -> bool {
    let mut pending = std::collections::VecDeque::from([start]);
    let mut visited_regions = vec![false; assigned.len()];
    let mut predecessors = vec![None; slot_owners.len()];
    visited_regions[start] = true;
    while let Some(region) = pending.pop_front() {
        for &slot in &choices[region] {
            if predecessors[slot].is_some() {
                continue;
            }
            predecessors[slot] = Some(region);
            if let Some(owner) = slot_owners[slot] {
                if !visited_regions[owner] {
                    visited_regions[owner] = true;
                    pending.push_back(owner);
                }
            } else {
                let mut next_slot = slot;
                while let Some(region) = predecessors[next_slot] {
                    let previous_slot = assigned[region].replace(next_slot);
                    slot_owners[next_slot] = Some(region);
                    let Some(previous_slot) = previous_slot else { break };
                    next_slot = previous_slot;
                }
                return true;
            }
        }
    }
    false
}

/// Assign each detected region to at most one formula slot on one side
/// (side-channel formulas or formula elements).
///
/// With region geometry, region/slot pairs whose bounding boxes (both in PDF
/// points) overlap enough are initially accepted best first, by overlap and
/// then IoU, independently of region order.
/// When the counts match, contested slots are reassigned along augmenting
/// paths when needed to pair every region. If no complete matching exists,
/// geometry is not trusted for the page and pairing is positional in reading
/// order, as it is without geometry; the OCR side
/// channel emits its formulas in the same sort (see
/// [`formula_regions_in_reading_order`]). When the counts differ, unpaired
/// regions stay unassigned, and without geometry nothing is assigned.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn assign_formula_slots(
    region_count: usize,
    slot_bboxes: &[Option<crate::types::BoundingBox>],
    regions_pts: Option<&[crate::types::BoundingBox]>,
) -> Vec<Option<usize>> {
    let counts_match = slot_bboxes.len() == region_count;
    let positional = || (0..region_count).map(Some).collect();
    let Some(regions_pts) = regions_pts else {
        return if counts_match {
            positional()
        } else {
            vec![None; region_count]
        };
    };
    debug_assert_eq!(regions_pts.len(), region_count);
    let mut candidates: Vec<(usize, usize, f64, f64)> = Vec::new();
    for (region_idx, region) in regions_pts.iter().enumerate() {
        for (slot_idx, slot) in slot_bboxes.iter().enumerate() {
            let Some(slot) = slot else { continue };
            let overlap = bbox_overlap_ratio(region, slot);
            if overlap >= FORMULA_PAIR_MIN_OVERLAP {
                candidates.push((region_idx, slot_idx, overlap, bbox_iou(region, slot)));
            }
        }
    }
    candidates.sort_by(|&(_, _, overlap_a, iou_a), &(_, _, overlap_b, iou_b)| {
        overlap_b.total_cmp(&overlap_a).then(iou_b.total_cmp(&iou_a))
    });
    let mut assigned: Vec<Option<usize>> = vec![None; region_count];
    let mut slot_taken = vec![false; slot_bboxes.len()];
    for &(region_idx, slot_idx, _, _) in &candidates {
        if assigned[region_idx].is_none() && !slot_taken[slot_idx] {
            assigned[region_idx] = Some(slot_idx);
            slot_taken[slot_idx] = true;
        }
    }
    if counts_match && assigned.contains(&None) {
        let mut choices = vec![Vec::new(); region_count];
        for (region_idx, slot_idx, _, _) in candidates {
            choices[region_idx].push(slot_idx);
        }
        let mut slot_owners = vec![None; slot_bboxes.len()];
        for (region, slot) in assigned.iter().enumerate() {
            if let Some(slot) = slot {
                slot_owners[*slot] = Some(region);
            }
        }
        for region in 0..region_count {
            if assigned[region].is_none()
                && !complete_formula_assignment(region, &choices, &mut assigned, &mut slot_owners)
            {
                return positional();
            }
        }
    }
    assigned
}

/// Whether an unpaired detection may become a new formula on this page.
///
/// Without region geometry, a count mismatch cannot pair regions with the
/// existing formula text, so a synthesized formula would duplicate one the
/// page already carries. Synthesis is safe when geometry exists, or when the
/// page carries no formula text at all (then there is nothing to duplicate).
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn formula_synthesis_allowed(has_region_geometry: bool, formula_slot_count: usize, element_slot_count: usize) -> bool {
    has_region_geometry || (formula_slot_count == 0 && element_slot_count == 0)
}

/// Per-page MediaBox sizes in points, for mapping detection bboxes into PDF
/// point space. `None` when the document cannot be opened (for example an
/// encrypted PDF); synthesized formulas then keep pixel coordinates, the
/// documented fallback for pages whose geometry is unavailable.
///
/// The page rasters reaching formula recognition are MediaBox-oriented: OCR
/// rasters are normalized, and markdown rasters survive only when no page is
/// rotated (`markdown_layout_usable_for_native`).
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn pdf_page_sizes_pt(content: &[u8], page_count: usize) -> Option<Vec<(f32, f32)>> {
    let doc = xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()).ok()?;
    Some(
        (0..page_count)
            .map(|index| crate::pdf::render::get_page_dimensions_pt(&doc, index))
            .collect(),
    )
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
struct FormulaPageInput<'a> {
    page_index: usize,
    image: &'a dyn AsPageRgb,
    detection: &'a crate::layout::DetectionResult,
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
struct FormulaPageInputs<'a> {
    page_count: usize,
    pages: Vec<FormulaPageInput<'a>>,
}

// ~keep A sparse borrowed view retains physical page indices and pairs each detection with its
// own raster. Original Markdown inputs remain untouched for VLM region extraction.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn select_formula_page_inputs<'a>(
    native: Option<(&'a [image::RgbImage], &'a [crate::layout::DetectionResult])>,
    mixed: Option<(&'a [image::DynamicImage], &'a [crate::layout::DetectionResult])>,
) -> FormulaPageInputs<'a> {
    let page_count = native
        .map_or(0, |(images, _)| images.len())
        .max(mixed.map_or(0, |(images, _)| images.len()));
    let pages = (0..page_count)
        .filter_map(|page_index| {
            let mixed_page = mixed.and_then(|(images, detections)| {
                let detection = detections.get(page_index)?;
                if !detection
                    .detections
                    .iter()
                    .any(|region| matches!(region.class_name, crate::layout::LayoutClass::Formula))
                {
                    return None;
                }
                let image = images.get(page_index)?;
                Some(FormulaPageInput {
                    page_index,
                    image: image.as_rgb8().map_or(image as &dyn AsPageRgb, |rgb| rgb),
                    detection,
                })
            });
            mixed_page.or_else(|| {
                let (images, detections) = native?;
                Some(FormulaPageInput {
                    page_index,
                    image: images.get(page_index)?,
                    detection: detections.get(page_index)?,
                })
            })
        })
        .collect();
    FormulaPageInputs { page_count, pages }
}

/// Recognize layout-detected formula regions on rendered PDF pages.
///
/// Each detected region pairs with the formula text the extraction side
/// already produced (side-channel formulas and formula elements), per page,
/// by bounding-box overlap in PDF points. When the counts match but overlap
/// cannot pair every region, the page pairs in reading order instead (see
/// [`assign_formula_slots`] for the rule). Recognized LaTeX replaces the
/// text of paired slots. A region paired with nothing — for example on a
/// scanned page whose OCR backend emits plain text only — becomes a new
/// formula with its bbox in PDF points. Failures keep the existing text.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
async fn recognize_pdf_formula_regions(
    ocr_doc: Option<&mut crate::types::internal::InternalDocument>,
    formulas: &mut Vec<crate::types::Formula>,
    images: &[impl AsPageRgb],
    detections: &[crate::layout::DetectionResult],
    layout: &crate::core::config::LayoutDetectionConfig,
    content: &[u8],
) {
    let pages = images
        .iter()
        .zip(detections)
        .enumerate()
        .map(|(page_index, (image, detection))| FormulaPageInput {
            page_index,
            image,
            detection,
        })
        .collect();
    let inputs = FormulaPageInputs {
        page_count: images.len(),
        pages,
    };
    recognize_pdf_formula_page_inputs(ocr_doc, formulas, &inputs, layout, content).await;
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
async fn recognize_pdf_formula_page_inputs(
    mut ocr_doc: Option<&mut crate::types::internal::InternalDocument>,
    formulas: &mut Vec<crate::types::Formula>,
    inputs: &FormulaPageInputs<'_>,
    layout: &crate::core::config::LayoutDetectionConfig,
    content: &[u8],
) {
    if layout.formula_model.is_none() {
        return;
    }
    let page_count = inputs.page_count;
    let mut element_slots: Vec<Vec<(Option<crate::types::BoundingBox>, &mut String)>> =
        (0..page_count).map(|_| Vec::new()).collect();
    if let Some(doc) = ocr_doc.as_deref_mut() {
        for element in doc.elements.iter_mut() {
            if matches!(element.kind, crate::types::internal::ElementKind::Formula)
                && let Some(page) = element.page
                && let Some(slot) = element_slots.get_mut((page as usize).saturating_sub(1))
            {
                slot.push((element.bbox, &mut element.text));
            }
        }
    }
    let mut missing_elements = Vec::new();
    // Page MediaBox sizes, parsed once on first need (geometric pairing or a
    // synthesized bbox); fully positional pages never pay for the parse.
    let mut page_sizes_pt: Option<Option<Vec<(f32, f32)>>> = None;
    for input in &inputs.pages {
        let page_idx = input.page_index;
        let image = input.image;
        let detection = input.detection;
        let regions = formula_regions_in_reading_order(detection);
        let page_number = (page_idx + 1) as u32;
        if regions.is_empty() {
            tracing::debug!(
                page = page_number,
                detections = detection.detections.len(),
                "layout detected no formula regions on this page; formula recognition skipped"
            );
            continue;
        }

        let mut synthesized: Vec<crate::types::Formula> = Vec::new();
        {
            let mut formula_slots: Vec<&mut crate::types::Formula> =
                formulas.iter_mut().filter(|f| f.page == Some(page_number)).collect();
            let elements = element_slots
                .get_mut(page_idx)
                .map(Vec::as_mut_slice)
                .unwrap_or(&mut []);

            let rgb = image.page_rgb();
            let rgb: &image::RgbImage = &rgb;

            let counts_differ = formula_slots.len() != regions.len() || elements.len() != regions.len();
            let formula_boxes: Vec<Option<crate::types::BoundingBox>> = formula_slots.iter().map(|f| f.bbox).collect();
            let element_boxes: Vec<Option<crate::types::BoundingBox>> =
                elements.iter().map(|(bbox, _)| *bbox).collect();
            let page_size = if counts_differ || formula_boxes.iter().chain(&element_boxes).any(Option::is_some) {
                page_sizes_pt
                    .get_or_insert_with(|| pdf_page_sizes_pt(content, page_count))
                    .as_ref()
                    .and_then(|sizes| sizes.get(page_idx).copied())
            } else {
                None
            };
            let regions_pts: Option<Vec<crate::types::BoundingBox>> = page_size.map(|(page_w_pt, page_h_pt)| {
                regions
                    .iter()
                    .map(|region| {
                        let pixel = crate::types::BoundingBox {
                            x0: f64::from(region.bbox.x1),
                            y0: f64::from(region.bbox.y1),
                            x1: f64::from(region.bbox.x2),
                            y1: f64::from(region.bbox.y2),
                        };
                        crate::pdf::render::pixel_bbox_to_pdf_points(
                            pixel,
                            rgb.width(),
                            rgb.height(),
                            page_w_pt,
                            page_h_pt,
                        )
                    })
                    .collect()
            });
            if counts_differ && (!formula_slots.is_empty() || !elements.is_empty()) {
                if regions_pts.is_some() {
                    tracing::warn!(
                        page = page_number,
                        detections = regions.len(),
                        formulas = formula_slots.len(),
                        elements = elements.len(),
                        "formula counts differ from detections; pairing regions by bounding-box overlap"
                    );
                } else {
                    tracing::warn!(
                        page = page_number,
                        detections = regions.len(),
                        formulas = formula_slots.len(),
                        elements = elements.len(),
                        "formula counts differ from detections and the page geometry is unavailable; keeping OCR text on this page"
                    );
                }
            }
            let allow_synthesis = formula_synthesis_allowed(regions_pts.is_some(), formula_slots.len(), elements.len());
            let formula_assign = assign_formula_slots(regions.len(), &formula_boxes, regions_pts.as_deref());
            let element_assign = assign_formula_slots(regions.len(), &element_boxes, regions_pts.as_deref());

            for (region_idx, region) in regions.iter().enumerate() {
                if formula_assign[region_idx].is_none() && element_assign[region_idx].is_none() && !allow_synthesis {
                    continue;
                }
                let Some((x, y, w, h)) = region.bbox.clamp_to_image(rgb.width(), rgb.height()) else {
                    continue;
                };
                let crop = image::imageops::crop_imm(rgb, x, y, w, h).to_image();
                match crate::formula_recognition::recognize_crop_blocking(crop, layout.acceleration.clone()).await {
                    Ok(Some(latex)) => {
                        let formula_slot = formula_assign[region_idx].and_then(|i| formula_slots.get_mut(i));
                        let element_slot = element_assign[region_idx].and_then(|i| elements.get_mut(i));
                        let bbox = formula_slot.as_ref().and_then(|slot| slot.bbox).unwrap_or_else(|| {
                            regions_pts
                                .as_ref()
                                .map(|pts| pts[region_idx])
                                .unwrap_or(crate::types::BoundingBox {
                                    x0: f64::from(x),
                                    y0: f64::from(y),
                                    x1: f64::from(x + w),
                                    y1: f64::from(y + h),
                                })
                        });
                        if element_slot.is_none() && (formula_slot.is_some() || allow_synthesis) {
                            missing_elements.push((
                                crate::types::Formula {
                                    latex: latex.clone(),
                                    bbox: Some(bbox),
                                    page: Some(page_number),
                                },
                                regions_pts.is_some(),
                            ));
                        }
                        let mut replaced = false;
                        if let Some(slot) = formula_slot {
                            slot.latex = latex.clone();
                            replaced = true;
                        }
                        if let Some(entry) = element_slot {
                            *entry.1 = latex.clone();
                            replaced = true;
                        }
                        if !replaced && allow_synthesis {
                            synthesized.push(crate::types::Formula {
                                latex,
                                bbox: Some(bbox),
                                page: Some(page_number),
                            });
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(error = %error, page = page_number, "pdf formula recognition failed; keeping OCR text");
                    }
                }
            }
        }
        if !synthesized.is_empty() {
            tracing::info!(
                page = page_number,
                formulas = synthesized.len(),
                "recognized formulas from layout regions with no extracted formula text"
            );
            formulas.extend(synthesized);
        }
    }
    drop(element_slots);
    if let Some(doc) = ocr_doc {
        insert_missing_formula_elements(doc, missing_elements);
    }
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn insert_missing_formula_elements(doc: &mut InternalDocument, missing_elements: Vec<(crate::types::Formula, bool)>) {
    let mut deferred_ids = std::collections::HashSet::new();
    for (formula, has_point_geometry) in missing_elements {
        insert_recognized_formula_element(doc, formula, has_point_geometry, &mut deferred_ids);
    }
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn insert_recognized_formula_element(
    doc: &mut InternalDocument,
    formula: crate::types::Formula,
    has_point_geometry: bool,
    deferred_ids: &mut std::collections::HashSet<crate::types::internal::InternalElementId>,
) {
    let mut at = doc.elements.len();
    for (index, element) in doc.elements.iter().enumerate() {
        if element.page > formula.page
            || (has_point_geometry
                && element.page == formula.page
                && element
                    .bbox
                    .zip(formula.bbox)
                    .is_some_and(|(existing, recognized)| existing.y1 < recognized.y0))
        {
            at = index;
            break;
        }
    }
    let before_later_page = doc.elements.get(at).is_some_and(|element| element.page > formula.page);
    // ~keep New root-level math must precede unopened containers without splitting their first member from the start.
    while at > 0
        && doc.elements[at - 1].kind.is_container_start()
        && (doc.elements[at - 1].page.is_none() || doc.elements[at - 1].page >= formula.page)
    {
        at -= 1;
    }
    // ~keep Cross a following page's boundary only for earlier-page math; same-page math stays after its boundary.
    if before_later_page {
        while at > 0 && matches!(doc.elements[at - 1].kind, ElementKind::PageBreak) {
            at -= 1;
        }
    }
    let Some(safe_at) = formula_container_boundary(doc, at, formula.page) else {
        doc.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("formula_recognition"),
            message: std::borrow::Cow::Owned(format!(
                "recognized formula on page {} retained in side channel; body insertion declined because no safe same-page container boundary exists",
                formula.page.unwrap_or(0),
            )),
        });
        return;
    };
    let is_deferred = safe_at > at;
    if is_deferred {
        // ~keep Only formulas deferred by this batch share its insertion cursor; original following math stays after them.
        at = safe_at;
        while doc.elements.get(at).is_some_and(|element| {
            matches!(element.kind, ElementKind::Formula)
                && element.page == formula.page
                && deferred_ids.contains(&element.id)
        }) {
            at += 1;
        }
    } else {
        at = safe_at;
    }
    let first_on_page = !doc.elements.iter().any(|element| element.page == formula.page);
    if first_on_page && at > 0 && !matches!(doc.elements[at - 1].kind, ElementKind::PageBreak) {
        insert_formula_document_element(doc, at, InternalElement::text(ElementKind::PageBreak, "", 0));
        at += 1;
    }
    let mut element = InternalElement::text(ElementKind::Formula, formula.latex, 0);
    element.page = formula.page;
    element.bbox = formula.bbox;
    insert_formula_document_element(doc, at, element);
    if is_deferred {
        deferred_ids.insert(doc.elements[at].id.clone());
    }
    if first_on_page
        && doc
            .elements
            .get(at + 1)
            .is_some_and(|next| !matches!(next.kind, ElementKind::PageBreak))
    {
        insert_formula_document_element(doc, at + 1, InternalElement::text(ElementKind::PageBreak, "", 0));
    }
}

// ~keep Detection geometry cannot assign list-item ownership. Defer standalone math until all open containers close.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn formula_container_boundary(doc: &InternalDocument, at: usize, page: Option<u32>) -> Option<usize> {
    let mut open = Vec::new();
    for element in &doc.elements[..at] {
        update_formula_container_stack(&mut open, element.kind);
    }
    if open.is_empty() {
        return Some(at);
    }
    for (offset, element) in doc.elements[at..].iter().enumerate() {
        if matches!(element.kind, ElementKind::PageBreak)
            || element.page.is_some_and(|element_page| Some(element_page) != page)
        {
            return None;
        }
        update_formula_container_stack(&mut open, element.kind);
        if open.is_empty() {
            return Some(at + offset + 1);
        }
    }
    None
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn update_formula_container_stack(open: &mut Vec<ElementKind>, kind: ElementKind) {
    if kind.is_container_start() {
        open.push(kind);
    } else if let Some(position) = open.iter().rposition(|start| {
        matches!(
            (start, kind),
            (ElementKind::ListStart { .. }, ElementKind::ListEnd)
                | (ElementKind::QuoteStart, ElementKind::QuoteEnd)
                | (ElementKind::GroupStart, ElementKind::GroupEnd)
        )
    }) {
        open.truncate(position);
    }
}

/// Put a picture's OCR text anchor after the last element of its own page, so the recovered
/// text reads before the next page instead of after the whole document.
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
fn insert_image_ocr_text_anchor(doc: &mut InternalDocument, anchor: InternalElement) {
    let next_page = anchor.page.and_then(|page| {
        doc.elements
            .iter()
            .position(|element| element.page.is_some_and(|other| other > page))
    });
    let Some(at) = next_page else {
        doc.push_element(anchor);
        return;
    };
    for relationship in &mut doc.relationships {
        if relationship.source >= at as u32 {
            relationship.source += 1;
        }
        if let crate::types::internal::RelationshipTarget::Index(target) = &mut relationship.target
            && *target >= at as u32
        {
            *target += 1;
        }
    }
    doc.elements.insert(at, anchor);
}

#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn insert_formula_document_element(doc: &mut InternalDocument, at: usize, mut element: InternalElement) {
    // ~keep Insertion positions can be reused after shifts; allocate a fresh identity without renumbering old IDs.
    let mut identity_index = doc.elements.len() as u32;
    loop {
        element.id = crate::types::internal::InternalElementId::generate(
            element.kind.discriminant(),
            &element.text,
            element.page,
            identity_index,
        );
        if doc.elements.iter().all(|existing| existing.id != element.id) {
            break;
        }
        identity_index = identity_index.wrapping_add(1);
    }
    // ~keep Preserve stable element IDs while moving positional relationship endpoints with their elements.
    for relationship in &mut doc.relationships {
        if relationship.source >= at as u32 {
            relationship.source += 1;
        }
        if let crate::types::internal::RelationshipTarget::Index(target) = &mut relationship.target
            && *target >= at as u32
        {
            *target += 1;
        }
    }
    doc.elements.insert(at, element);
}

#[cfg(all(
    test,
    feature = "pdf",
    feature = "layout-detection",
    feature = "ocr",
    feature = "formula-recognition",
    feature = "tokio-runtime"
))]
pub(crate) async fn recognize_formula_regions_for_test(
    doc: &mut InternalDocument,
    formulas: &mut Vec<crate::types::Formula>,
    images: &[image::RgbImage],
    detections: &[crate::layout::DetectionResult],
    layout: &crate::core::config::LayoutDetectionConfig,
) {
    recognize_pdf_formula_regions(Some(doc), formulas, images, detections, layout, &[]).await;
}

/// Whether the caller actually asked for the recovered diagram, i.e. the
/// request's output format resolves to the `dot` renderer.
///
/// `OutputFormat::Custom` is how any renderer, built-in or registered, is
/// selected (see `plugins::registry::RendererRegistry`), so matching the
/// renderer name here is the same test `derive_extraction_result` uses to
/// decide which renderer runs — not a looser proxy for it.
#[cfg(feature = "pdf")]
fn wants_dot_output(config: &ExtractionConfig) -> bool {
    matches!(&config.output_format, crate::core::config::OutputFormat::Custom(name) if name == "dot")
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn attach_pdf_preprocessing_metadata(
    pages: &mut Option<Vec<crate::types::PageContent>>,
    by_page: &ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
) -> Option<crate::types::ImagePreprocessingMetadata> {
    if let Some(pages) = pages {
        for page in pages {
            page.image_preprocessing = by_page.get(&page.page_number).cloned();
        }
    }

    by_page
        .iter()
        .min_by_key(|(page_number, _)| *page_number)
        .map(|(_, metadata)| metadata.clone())
}

/// Attach each OCR'd page's confidence summary to its [`crate::types::PageContent`] (#1568).
///
/// `by_page` is keyed by 1-based page number and only holds pages an OCR route actually ran,
/// so a natively extracted page keeps `ocr_confidence: None` -- absence means "not OCR'd",
/// distinct from an entry whose `score` is `None` because the backend has no calibrated
/// legibility scale. Unlike `attach_pdf_preprocessing_metadata` there is no document-level
/// aggregate: averaging per-page confidences across pages OCR'd by different backends (the
/// pipeline route's stages) would compare incomparable scales. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn attach_pdf_ocr_confidence(
    pages: &mut Option<Vec<crate::types::PageContent>>,
    by_page: &ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
) {
    if let Some(pages) = pages {
        for page in pages {
            page.ocr_confidence = by_page.get(&page.page_number).cloned();
        }
    }
}

/// Replace a page's `content` with OCR text and recompute `is_blank` from it.
///
/// With `keep_native_content`, the text layer that `content` held moves to `native_content`
/// when it is not blank and differs from the OCR text. The same rule applies on every OCR route.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn replace_page_content_with_ocr_text(page: &mut crate::types::PageContent, ocr_text: &str, keep_native_content: bool) {
    let ocr_content = crate::pdf::text::fix_pdf_control_chars(ocr_text).into_owned();
    let native = std::mem::replace(&mut page.content, ocr_content);
    let keep = keep_native_content && !native.trim().is_empty() && native != page.content;
    page.native_content = keep.then_some(native);
    page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
}

/// Write the OCR text of each page into the page that has the same position.
///
/// One OCR text for a document of several pages is the text of the whole document: it goes to
/// the first page, and the content of every later page is replaced with empty text.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn replace_pages_with_ocr_page_texts(
    pages: &mut [crate::types::PageContent],
    ocr_page_texts: &[String],
    keep_native_content: bool,
) {
    for (page, text) in pages.iter_mut().zip(ocr_page_texts) {
        replace_page_content_with_ocr_text(page, text, keep_native_content);
    }

    if ocr_page_texts.len() == 1 {
        for page in pages.iter_mut().skip(1) {
            replace_page_content_with_ocr_text(page, "", keep_native_content);
        }
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn automatic_unmapped_text_hints(metadata: &crate::pdf::metadata::PdfExtractionMetadata) -> Option<ocr::PageOcrHints> {
    let pages = metadata.pdf_specific.fabricated_text_pages.as_ref()?;
    (!pages.is_empty()).then(|| ocr::PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: Some(std::sync::Arc::new(pages.iter().copied().collect())),
        native_page_layers: None,
        ..Default::default()
    })
}

/// Hints for the whole-document OCR fallback: the fabricated-text pages, plus the readable text
/// layer of each page, so a page whose OCR text is judged recognition noise keeps its layer.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn whole_document_ocr_hints(
    metadata: &crate::pdf::metadata::PdfExtractionMetadata,
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
    config: &ExtractionConfig,
) -> Option<ocr::PageOcrHints> {
    let thresholds = config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.quality_thresholds.clone())
        .unwrap_or_default();
    let layers = ocr::clean_native_page_layers(native_text, boundaries.unwrap_or(&[]), None, &thresholds);
    let layers = (!layers.is_empty()).then(|| std::sync::Arc::new(layers));
    let mut hints = automatic_unmapped_text_hints(metadata);
    if layers.is_some() {
        hints
            .get_or_insert_with(|| ocr::PageOcrHints {
                source_dpi: None,
                known_full_page_scan: false,
                single_block_pages: None,
                native_page_layers: None,
                ..Default::default()
            })
            .native_page_layers = layers;
    }
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    if config.ocr_strategy == crate::core::config::OcrStrategy::Auto
        && config.ocr.as_ref().is_some_and(|ocr| {
            ocr.pipeline.is_none() && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
        })
        && let Some([boundary]) = boundaries
        && boundary.page_number == 1
        && let Some(native) = native_text.get(boundary.byte_start..boundary.byte_end)
        && native.contains(unmapped_glyphs::LOST_GLYPH_MARKER)
    {
        hints.get_or_insert_with(Default::default).single_native_type0_page = Some(native.into());
    }
    hints
}

#[cfg(all(
    feature = "layout-detection",
    any(feature = "formula-recognition", feature = "liter-llm"),
    any(feature = "ocr", feature = "ocr-pipeline")
))]
fn native_layout_readers_configured(config: &ExtractionConfig) -> bool {
    let formula_reads =
        cfg!(feature = "formula-recognition") && config.layout.as_ref().is_some_and(|l| l.formula_model.is_some());
    #[cfg(feature = "liter-llm")]
    let vlm_reads = region_vlm::region_vlm_config(config).is_some();
    #[cfg(not(feature = "liter-llm"))]
    let vlm_reads = false;
    formula_reads || vlm_reads
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use unreadable_script::{dropped_script_warnings, every_page_dropped_script, pages_that_dropped_script};

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    not(all(feature = "ocr", not(target_arch = "wasm32")))
))]
fn every_page_dropped_script(_: &[u32], _: &str, _: Option<&[crate::types::PageBoundary]>, _: &[String]) -> bool {
    false
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    not(all(feature = "ocr", not(target_arch = "wasm32")))
))]
fn pages_that_dropped_script(_: &[u32], _: &str, _: Option<&[crate::types::PageBoundary]>, _: &[String]) -> Vec<u32> {
    Vec::new()
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    not(all(feature = "ocr", not(target_arch = "wasm32")))
))]
fn dropped_script_warnings(
    _: &[u32],
    _: &str,
    _: Option<&[crate::types::PageBoundary]>,
) -> Vec<crate::types::ProcessingWarning> {
    Vec::new()
}

/// PDF document extractor using xberg_native_pdf.
#[cfg_attr(alef, alef(skip))]
pub struct PdfExtractor;

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Plugin for PdfExtractor {
    fn name(&self) -> &str {
        "pdf-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for PdfExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        crate::pdf::render::with_task_render_warnings(self.extract_core(content, mime_type, config, None)).await
    }

    #[cfg(feature = "tokio-runtime")]
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        #[cfg(feature = "pdf")]
        crate::pdf::native_text::set_current_pdf_path(Some(path.to_path_buf()));
        // Async on native (non-blocking tokio::fs); sync fallback on wasm32 where tokio's `fs`
        // feature is unavailable. See `core::io::read_file_async`. ~keep
        let bytes = crate::core::io::read_file_async(path).await?;
        let result =
            crate::pdf::render::with_task_render_warnings(self.extract_core(&bytes, mime_type, config, Some(path)))
                .await;
        #[cfg(feature = "pdf")]
        crate::pdf::native_text::set_current_pdf_path(None);
        result
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl crate::core::layout_pages::LayoutPagesExtractor for PdfExtractor {
    async fn extract_with_layout_pages(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        pages: &crate::core::layout_pages::LayoutPageSelection,
    ) -> Result<InternalDocument> {
        if config
            .pdf_options
            .as_ref()
            .is_some_and(|options| options.backend != crate::core::config::PdfBackend::Native)
        {
            return Err(crate::XbergError::validation(
                "explicit layout-page selection requires the native PDF backend".to_string(),
            ));
        }
        enforce_page_limit(content, config)?;
        let page_count = native_page_count(content).ok_or_else(|| {
            crate::XbergError::validation("unable to count PDF pages for explicit layout selection".to_string())
        })?;
        let mut document = self
            .extract_core_native_with_layout_pages(content, mime_type, config, None, Some(pages))
            .await?;
        for warning in pages.warnings(page_count) {
            crate::core::diagnostics::push_warning_deduped(&mut document.processing_warnings, warning);
        }
        Ok(document)
    }
}

impl PdfExtractor {
    /// Core extraction logic shared between extract_bytes and extract_file.
    ///
    /// Accepts an optional `path` which is passed to OCR backends to allow
    /// direct document-level processing (bypassing page rendering).
    ///
    /// This is the dispatch seam for `PdfConfig::backend` (#702): the single place
    /// that routes a PDF extraction request to a backend implementation. Adding a
    /// real `PdfBackend::Pdfium` engine means adding a branch here, not touching
    /// `extract_content`/`extract_path` or any caller. The dispatch is enforced here
    /// -- in the core crate, below every binding -- rather than only in
    /// `xberg-cli`'s `ExtractionOverrides::validate`, because any caller that builds
    /// `ExtractionConfig` directly (library use, the API/MCP servers, or a language
    /// binding) sets `PdfConfig::backend` without ever going through CLI validation.
    /// Without an enforcement point here, such a caller selecting
    /// `PdfBackend::Pdfium` would have silently gotten `xberg_native_pdf` output mislabeled
    /// as pdfium.
    async fn extract_core(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "pdf", size_bytes = content.len(), "extraction starting");
        #[cfg(feature = "pdf")]
        {
            if let Some(options) = config.pdf_options.as_ref() {
                options.validate()?;
            }
            let backend = config
                .pdf_options
                .as_ref()
                .map(|options| options.backend)
                .unwrap_or_default();
            match backend {
                crate::core::config::PdfBackend::Native => {
                    self.extract_core_native(content, mime_type, config, path).await
                }
                crate::core::config::PdfBackend::Pdfium => {
                    self.extract_core_pdfium(content, mime_type, config, path).await
                }
            }
        }
        #[cfg(not(feature = "pdf"))]
        self.extract_core_native(content, mime_type, config, path).await
    }

    /// Dispatch target for `PdfBackend::Pdfium`.
    ///
    /// Without the `pdf-pdfium` Cargo feature, `crates/xberg-pdfium-render` is not
    /// even compiled in, so this fails loudly rather than falling back to
    /// `extract_core_native`: silently reusing `xberg_native_pdf` output under a caller's
    /// `PdfBackend::Pdfium` selection would produce a document the caller believes
    /// came from pdfium, which is worse than an error.
    #[cfg(all(feature = "pdf", not(feature = "pdf-pdfium")))]
    async fn extract_core_pdfium(
        &self,
        _content: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        Err(crate::XbergError::validation(
            "PDF extraction requested backend 'pdfium', but this build was compiled \
             without the 'pdf-pdfium' Cargo feature, so no pdfium extraction engine is \
             available. Use the default 'native' backend instead, or rebuild with \
             --features pdf-pdfium to enable pdfium extraction.",
        ))
    }

    /// Dispatch target for `PdfBackend::Pdfium` when the `pdf-pdfium` feature is
    /// enabled (#702).
    ///
    /// Delegates to [`pdfium_engine::extract`], a deliberately smaller engine than
    /// `extract_core_native`: see that module's doc comment for exactly what it
    /// extracts and what it does not (no tables, images, annotations, form fields,
    /// embedded files, or OCR fallback). Binding to the pdfium shared library is a
    /// runtime concern -- see `pdfium_engine::bind_once` -- so this can still fail
    /// with an actionable `XbergError::MissingDependency` even though the feature
    /// compiled cleanly.
    #[cfg(all(feature = "pdf", feature = "pdf-pdfium"))]
    async fn extract_core_pdfium(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        enforce_page_limit(content, config)?;
        pdfium_engine::extract(content, mime_type, config).await
    }

    /// Core extraction via the xberg_native_pdf backend.
    ///
    /// Runs text + metadata, tables, and annotation extraction through the native
    /// modules, then builds an `InternalDocument` using the same post-processing
    /// pipeline (OCR evaluation, page assembly, image extraction, bookmarks, etc.).
    #[cfg(feature = "pdf")]
    async fn extract_core_native(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        // ~keep Keep the added compatibility wrapper from doubling the native future's stack footprint.
        Box::pin(self.extract_core_native_with_layout_pages(content, mime_type, config, path, None)).await
    }

    #[cfg(feature = "pdf")]
    async fn extract_core_native_with_layout_pages(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
        layout_pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
    ) -> Result<InternalDocument> {
        let _ = (&path, &layout_pages);

        enforce_page_limit(content, config)?;

        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        #[allow(unused_mut, unused_variables)]
        let (
            mut markdown_layout_images,
            markdown_layout_results,
            markdown_layout_hints,
            mut markdown_layout_detections,
            markdown_layout_gate_decisions,
            markdown_layout_warning,
            mut markdown_layout_acceleration_override,
            markdown_layout_glyph_drop_warnings,
        ) = layout_runner::maybe_run_layout_for_markdown_with_pages(content, config, layout_pages).await;

        let blocking_content = content.to_vec();
        let blocking_config = config.clone();
        #[cfg(feature = "layout-detection")]
        let region_text_layout = layout_pages
            .filter(|pages| pages.region_text.is_some() && crate::engine::layout_region_text::capture_allowed(config))
            .zip(markdown_layout_detections.as_deref())
            .map(
                |(selection, detections)| crate::engine::layout_region_text::RegionTextLayout {
                    selection: selection.clone(),
                    detector_classes: detections
                        .iter()
                        .map(|page| page.detections.iter().map(|detection| detection.class_name).collect())
                        .collect(),
                },
            );
        let native_output = spawn_pdf_blocking(move || {
            extract_native_blocking(
                blocking_content,
                blocking_config,
                #[cfg(feature = "layout-detection")]
                markdown_layout_images,
                #[cfg(feature = "layout-detection")]
                markdown_layout_results,
                #[cfg(feature = "layout-detection")]
                markdown_layout_hints,
                #[cfg(feature = "layout-detection")]
                markdown_layout_acceleration_override,
                #[cfg(feature = "layout-detection")]
                region_text_layout,
            )
        })
        .await?;
        let bookmark_uris = native_output.bookmark_uris;
        let pdf_revisions = native_output.pdf_revisions;
        let diagrams = native_output.diagrams;
        #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
        let mut markdown_layout_images = native_output.layout_images;
        #[cfg(all(feature = "layout-detection", not(any(feature = "ocr", feature = "ocr-pipeline"))))]
        let _markdown_layout_images = native_output.layout_images;
        #[cfg(all(feature = "layout-detection", feature = "liter-llm"))]
        let markdown_layout_hints = native_output.layout_hints;
        #[cfg(all(feature = "layout-detection", not(feature = "liter-llm")))]
        let _markdown_layout_hints = native_output.layout_hints;
        #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
        let mut markdown_layout_acceleration_override = native_output.layout_acceleration_override;
        #[cfg(all(feature = "layout-detection", not(any(feature = "ocr", feature = "ocr-pipeline"))))]
        let _markdown_layout_acceleration_override = native_output.layout_acceleration_override;

        #[allow(unused_variables, unused_mut)]
        let (
            mut pdf_metadata,
            native_text,
            mut tables,
            mut page_contents,
            mut boundaries,
            mut pre_rendered_doc,
            _has_font_encoding_issues,
            annotation_text_fallback,
            pdf_annotations,
            mut extracted_images,
            pdf_form_fields,
            mut pdf_extraction_warnings,
            pdf_page_labels,
            pdf_page_coordinate_frames,
            mut outlined_ocr_pages,
        ) = native_output.extraction;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if config.pdf_options.as_ref().is_some_and(|p| p.ocr_inline_images)
            && let Some(ref mut imgs) = extracted_images
            && !imgs.is_empty()
        {
            let resolved_ocr_config = ocr::resolve_ocr_config(config);
            let ocr_config = resolved_ocr_config.as_ref();
            crate::plugins::ensure_ocr_backends_initialized();
            let backend = {
                let registry = crate::plugins::registry::get_ocr_backend_registry();
                registry.read().get(&ocr_config.backend)?
            };
            let mut ocr_config_with_format = ocr_config.clone();
            ocr_config_with_format.output_format = Some(config.output_format.clone());
            // GH#1651: this embedded-image route never carried the caller's decode limits,
            // so every decode ran under `SecurityLimits::default()`. Conditional so a limit
            // set directly on `OcrConfig` is not replaced by `None`. ~keep
            if let Some(limits) = config.security_limits.clone() {
                ocr_config_with_format.security_limits = Some(limits);
            }
            for img in imgs.iter_mut() {
                if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    break;
                }
                match backend.process_image(&img.data, &ocr_config_with_format).await {
                    Ok(mut ocr_result) => {
                        ocr_config.apply_public_element_policy(&mut ocr_result);
                        img.ocr_result = Some(Box::new(ocr_result));
                    }
                    Err(e) => {
                        tracing::warn!(
                            page = img.page_number,
                            image_index = img.image_index,
                            error = %e,
                            "inline image OCR failed; image returned without OCR result"
                        );
                    }
                }
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_tables: Vec<crate::types::Table> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_coordinate_frames: Vec<crate::types::internal::OcrPageCoordinateFrame> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_internal_doc: Option<InternalDocument> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_texts: Option<Vec<String>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_results_map: Option<ahash::AHashMap<u32, String>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut structured_ocr_pages: Option<ahash::AHashMap<u32, InternalDocument>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_rasters: Option<Vec<crate::types::ExtractedImage>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_formulas: Vec<crate::types::Formula> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata> =
            ahash::AHashMap::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence> =
            ahash::AHashMap::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_fallback_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_failures: Vec<crate::types::OcrPageFailure> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        #[allow(unused_assignments)]
        let mut ocr_layout_gate_audit: OcrLayoutGateDecisions = (None, None);
        #[cfg(feature = "formula-recognition")]
        let mut mixed_formula_layout_inputs = None;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        record_implausible_text_pages(
            config,
            &mut pdf_metadata,
            &native_text,
            boundaries.as_deref(),
            &mut ocr_fallback_warnings,
        );

        // Only formula recognition and VLM regions read the rasters on the native path.
        #[cfg(all(
            feature = "pdf",
            feature = "layout-detection",
            any(feature = "ocr", feature = "ocr-pipeline"),
            any(feature = "formula-recognition", feature = "liter-llm")
        ))]
        let mut native_only_layout_inputs = None;
        #[cfg(all(
            feature = "pdf",
            feature = "layout-detection",
            any(feature = "ocr", feature = "ocr-pipeline")
        ))]
        {
            // These values exist only with layout detection; keep the reuse gate in the same cfg block. ~keep
            let markdown_page_rotations = markdown_layout_images
                .as_ref()
                .map(|images| crate::pdf::render::get_page_rotations_from_bytes(content, images.len()))
                .unwrap_or_default();
            if !markdown_layout_reusable_for_ocr(markdown_layout_gate_decisions.as_deref(), &markdown_page_rotations) {
                let native_inputs = (markdown_layout_images.take(), markdown_layout_detections.take());
                markdown_layout_acceleration_override = None;
                #[cfg(any(feature = "formula-recognition", feature = "liter-llm"))]
                {
                    // Keep the rasters only when this run configures a native reader of them.
                    if native_layout_readers_configured(config)
                        && markdown_layout_usable_for_native(&markdown_page_rotations)
                    {
                        native_only_layout_inputs = Some(native_inputs);
                    }
                }
                #[cfg(not(any(feature = "formula-recognition", feature = "liter-llm")))]
                drop(native_inputs);
            }
        }

        // The result of a targeted per-page run, shared by the `RunFallbackOnPages` arm and the
        // whole-document arm's retry of the pages whose native script the whole read dropped. ~keep
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        macro_rules! apply_targeted_mixed_run {
            ($mixed_run:expr) => {{
                let mixed_run = $mixed_run;
                #[cfg(feature = "formula-recognition")]
                {
                    mixed_formula_layout_inputs = mixed_run.formula_layout_inputs;
                }
                let (
                    mixed,
                    results_map,
                    mixed_structured_pages,
                    mixed_llm_usage,
                    mixed_rstrs,
                    mixed_formulas,
                    mixed_preprocessing,
                    mixed_ocr_confidence,
                    mixed_warnings,
                    mixed_page_failures,
                ) = mixed_run.output;
                let extraction_method = extraction_method_after_mixed_ocr(&results_map);
                ocr_llm_usage = mixed_llm_usage;
                ocr_results_map = Some(results_map);
                structured_ocr_pages = Some(mixed_structured_pages);
                ocr_page_rasters = mixed_rstrs;
                if !mixed_formulas.is_empty() {
                    ocr_formulas = mixed_formulas;
                }
                ocr_preprocessing_by_page.extend(mixed_preprocessing);
                ocr_confidence_by_page.extend(mixed_ocr_confidence);
                ocr_page_failures.extend(mixed_page_failures);
                ocr_fallback_warnings.extend(mixed_warnings);
                (mixed, extraction_method)
            }};
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let (mut text, extraction_method) = if config.effective_disable_ocr() {
            (native_text, ExtractionMethod::Native)
        } else if config.force_ocr {
            let (
                ocr_text,
                ocr_tbls,
                ocr_elems,
                ocr_doc,
                llm_usage,
                ocr_pts,
                ocr_rstrs,
                formulas,
                preprocessing,
                page_ocr_confidence,
                gate_audit,
                layout_warning,
                layout_glyph_drop_warnings,
                _,
            ) = run_ocr_with_layout_pages(
                content,
                config,
                path,
                PageOcrLayoutContext {
                    pages: layout_pages,
                    hints: None,
                },
                #[cfg(feature = "layout-detection")]
                markdown_layout_images.take(),
                #[cfg(feature = "layout-detection")]
                markdown_layout_detections.take(),
                #[cfg(feature = "layout-detection")]
                markdown_layout_acceleration_override.take(),
            )
            .await?;
            if let Some(warning) = layout_warning {
                crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
            }
            for warning in layout_glyph_drop_warnings {
                crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
            }
            ocr_layout_gate_audit = gate_audit;
            ocr_tables = ocr_tbls;
            ocr_elements = ocr_elems;
            ocr_internal_doc = ocr_doc;
            ocr_llm_usage = llm_usage;
            ocr_page_texts = Some(ocr_pts);
            ocr_page_rasters = ocr_rstrs;
            ocr_formulas = formulas;
            ocr_preprocessing_by_page = preprocessing;
            ocr_confidence_by_page = page_ocr_confidence;
            (ocr_text, ExtractionMethod::Ocr)
        } else if let Some(ref ocr_pages) = config.force_ocr_pages {
            if !ocr_pages.is_empty() {
                if let Some(ref bounds) = boundaries {
                    if !bounds.is_empty() {
                        let mixed_run = ocr::extract_mixed_ocr_native_with_layout_pages(
                            &native_text,
                            bounds,
                            (ocr_pages, &[]),
                            content,
                            config,
                            layout_pages,
                            (&[], &[]),
                        )
                        .await?;
                        #[cfg(feature = "formula-recognition")]
                        {
                            mixed_formula_layout_inputs = mixed_run.formula_layout_inputs;
                        }
                        let (
                            mixed,
                            results_map,
                            mixed_structured_pages,
                            mixed_llm_usage,
                            mixed_rstrs,
                            mixed_formulas,
                            mixed_preprocessing,
                            mixed_ocr_confidence,
                            mixed_warnings,
                            mixed_page_failures,
                        ) = mixed_run.output;
                        let extraction_method = extraction_method_after_mixed_ocr(&results_map);
                        ocr_llm_usage = mixed_llm_usage;
                        ocr_results_map = Some(results_map);
                        structured_ocr_pages = Some(mixed_structured_pages);
                        ocr_page_rasters = mixed_rstrs;
                        if !mixed_formulas.is_empty() {
                            ocr_formulas = mixed_formulas;
                        }
                        ocr_preprocessing_by_page.extend(mixed_preprocessing);
                        ocr_confidence_by_page.extend(mixed_ocr_confidence);
                        ocr_page_failures.extend(mixed_page_failures);
                        ocr_fallback_warnings.extend(mixed_warnings);
                        (mixed, extraction_method)
                    } else {
                        tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                        (native_text, ExtractionMethod::Native)
                    }
                } else {
                    tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                    (native_text, ExtractionMethod::Native)
                }
            } else {
                (native_text, ExtractionMethod::Native)
            }
        } else if let Some(scanned_pages) =
            scanned_pages_to_ocr(config, &pdf_metadata, &native_text, boundaries.as_deref())
                .filter(|_| config.ocr.is_some() || automatic_ocr_backend_is_registered())
        {
            // A scanner's invisible sidecar passes the gate below, so detected
            // pages are selected before it runs. ~keep
            if let Some(ref bounds) = boundaries
                && !bounds.is_empty()
            {
                // This OCR run is AUTOMATIC -- scanned-page detection asked for it, the
                // caller did not -- so a failure here must degrade to the native text rather
                // than abort, exactly as the quality-gate fallback below already does. With
                // `ocr-pipeline` enabled and no backend registered (a legitimate feature
                // selection: `ocr` implies `ocr-pipeline`, not the reverse) the `?` that used
                // to be here failed an ordinary PDF extraction that never requested OCR. The
                // EXPLICIT sites (`force_ocr_pages` above, `ocr_inline_images`) keep their
                // hard error, because there the caller asked for something the build cannot
                // do. See GH#1610. ~keep
                match ocr::extract_mixed_ocr_native_with_layout_pages(
                    &native_text,
                    bounds,
                    (
                        &scanned_pages,
                        pdf_metadata
                            .pdf_specific
                            .fabricated_text_pages
                            .as_deref()
                            .unwrap_or(&[]),
                    ),
                    content,
                    config,
                    layout_pages,
                    (&[], &[]),
                )
                .await
                {
                    Ok(mixed_run) => {
                        #[cfg(feature = "formula-recognition")]
                        {
                            mixed_formula_layout_inputs = mixed_run.formula_layout_inputs;
                        }
                        let (
                            mixed,
                            results_map,
                            mixed_structured_pages,
                            mixed_llm_usage,
                            mixed_rstrs,
                            mixed_formulas,
                            mixed_preprocessing,
                            mixed_ocr_confidence,
                            mixed_warnings,
                            mixed_page_failures,
                        ) = mixed_run.output;
                        // `Mixed` must mean "OCR contributed text", not "OCR was attempted". When
                        // every candidate page was rejected (blank render, failed decode, empty
                        // backend output) nothing was replaced and the result IS the native text --
                        // reporting `Mixed` there tells a caller the document was OCR'd when it was
                        // not, which is how a silent whole-document OCR failure reads as success. ~keep
                        let mixed_method = extraction_method_after_mixed_ocr(&results_map);
                        let ocr_contributed = mixed_method == ExtractionMethod::Mixed;
                        if !ocr_contributed {
                            tracing::warn!(
                                candidate_pages = scanned_pages.len(),
                                "OCR was attempted on every detected scanned page but no page produced usable \
                                 text; reporting the native extraction method rather than `mixed`"
                            );
                        }
                        ocr_llm_usage = mixed_llm_usage;
                        ocr_results_map = Some(results_map);
                        structured_ocr_pages = Some(mixed_structured_pages);
                        ocr_page_rasters = mixed_rstrs;
                        if !mixed_formulas.is_empty() {
                            ocr_formulas = mixed_formulas;
                        }
                        ocr_preprocessing_by_page.extend(mixed_preprocessing);
                        ocr_confidence_by_page.extend(mixed_ocr_confidence);
                        ocr_page_failures.extend(mixed_page_failures);
                        ocr_fallback_warnings.extend(mixed_warnings);
                        if ocr_contributed {
                            (mixed, mixed_method)
                        } else {
                            (mixed, ExtractionMethod::Native)
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            candidate_pages = ?scanned_pages,
                            "Automatic OCR of detected scanned pages failed; using native text extraction result"
                        );
                        if failed_ocr_fallback_is_total_loss(&native_text) {
                            return Err(e);
                        }
                        ocr_page_failures.extend(ocr::route_level_ocr_page_failures(
                            &scanned_pages,
                            &e,
                            &native_text,
                            bounds,
                        ));
                        ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("ocr"),
                            message: std::borrow::Cow::Owned(format!(
                                "Automatic OCR of detected scanned pages {scanned_pages:?} failed ({e}); those \
                                 pages retain their native text, which may be empty or incomplete."
                            )),
                        });
                        (native_text, ExtractionMethod::Native)
                    }
                }
            } else {
                tracing::warn!("scanned pages detected but no page boundaries available; using native text");
                (native_text, ExtractionMethod::Native)
            }
        } else if near_empty_ocr_fallback_applies(config, &native_text) {
            // Under `Auto`, a PDF with NO native text at all (a scan / missing text layer)
            // must reach OCR even when no explicit `ocr` config was given — the default
            // `ocr: None` ("OCR disabled") otherwise silently discarded the detected scan
            // and returned an empty result (#1338). This is gated on genuinely-absent text
            // a legitimately sparse/short PDF must stay native under a default config, and
            // heuristic quality failures still require an explicit `ocr` config to trigger
            // OCR. An explicit `ocr` config keeps its full per-page gate behavior.
            // `ocr_near_empty_fallback` overrides that derivation in either direction
            // (GH#1752); see `near_empty_ocr_fallback_applies`. ~keep
            let default_ocr_config = crate::core::config::OcrConfig::default();
            let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
            let thresholds = ocr_config.effective_thresholds();
            let mut decision = ocr::evaluate_per_page_ocr(
                &native_text,
                boundaries.as_deref(),
                pdf_metadata.pdf_specific.page_count,
                &thresholds,
            );
            #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
            let native_script_pages: Vec<u32> = Vec::new();
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            crate::core::diagnostics::dedup_extend_warnings(
                &mut ocr_fallback_warnings,
                unreadable_script::retain_unreadable_script_pages(
                    &mut decision,
                    &native_text,
                    boundaries.as_deref(),
                    pdf_metadata.pdf_specific.page_count,
                    content,
                    config,
                ),
            );
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let native_script_pages =
                unreadable_script::native_script_pages(&decision, &native_text, boundaries.as_deref(), content, config);

            // `NativeTextStats`' character-class checks cannot see a font whose mapping
            // legitimately (per the §9.10.2 cascade) resolves to the wrong-but-ordinary
            // letters and punctuation (issue #1667): the text is structurally clean. The
            // provenance signal computed above (issue #1254) is a fact about how the text
            // was derived, not a shape guess, and `ScannedPages` already routes it via
            // `scanned_pages_to_ocr`; `Auto` must consult it too rather than silently
            // discarding it. ~keep
            let has_boundaries = boundaries.as_deref().is_some_and(|b| !b.is_empty());
            ocr::apply_flagged_pages(
                &mut decision,
                pdf_metadata
                    .pdf_specific
                    .fabricated_text_pages
                    .as_deref()
                    .unwrap_or(&[]),
                has_boundaries,
                pdf_metadata.pdf_specific.page_count,
            );

            // A `/ToUnicode` CMap that resolves every glyph to *a* character, but
            // consistently the WRONG one, is file-backed exactly like a correct mapping --
            // `fabricated_text_pages`' provenance check cannot see it either (issue #1696).
            // `record_implausible_text_pages` already computed and merged this into
            // `scanned_pages` above; `Auto` reads it here the same way it reads the
            // provenance signal, so both share one union step. ~keep
            ocr::apply_flagged_pages(
                &mut decision,
                pdf_metadata
                    .pdf_specific
                    .implausible_text_pages
                    .as_deref()
                    .unwrap_or(&[]),
                has_boundaries,
                pdf_metadata.pdf_specific.page_count,
            );

            if decision.whole_doc_failure {
                outlined_ocr_pages.clear();
            } else {
                outlined_ocr_pages.retain(|page| !decision.failing_pages.contains(page));
                // Keep the selected-page acceptance check even when every page qualifies. ~keep
                ocr::apply_flagged_pages(&mut decision, &outlined_ocr_pages, has_boundaries, None);
            }

            tracing::debug!(
                target: "xberg::pdf::ocr",
                fallback = decision.fallback,
                non_whitespace = decision.stats.non_whitespace,
                alnum = decision.stats.alnum,
                meaningful_words = decision.stats.meaningful_words,
                avg_non_whitespace = decision.avg_non_whitespace,
                avg_alnum = decision.avg_alnum,
                alnum_ratio = decision.stats.alnum_ratio,
                fragmented_word_ratio = decision.stats.fragmented_word_ratio,
                avg_word_length = decision.stats.avg_word_length,
                word_count = decision.stats.word_count,
                consecutive_repeat_ratio = decision.stats.consecutive_repeat_ratio,
                "per-page OCR gate decision",
            );

            let total_chars = native_text.chars().count();
            let alnum_ws_chars = native_text
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .count();
            let alnum_ws_ratio = if total_chars > 0 {
                alnum_ws_chars as f64 / total_chars as f64
            } else {
                1.0
            };

            let outcome = ocr::evaluate_ocr_skip_gate(
                pre_rendered_doc.is_some(),
                total_chars,
                alnum_ws_ratio,
                &decision,
                &thresholds,
            );
            let outcome = if automatic_page_acceptance_supported(config) {
                with_native_page_acceptance(
                    outcome,
                    &decision.failing_pages,
                    boundaries.as_deref(),
                    &native_text,
                    pdf_metadata.pdf_specific.page_count,
                )
            } else {
                outcome
            };
            match outcome {
                ocr::OcrGateOutcome::SkipNonText => {
                    tracing::debug!(
                        alnum_ws_ratio,
                        total_chars,
                        alnum_ws_chars,
                        "Skipping OCR: content is non-textual and pre-rendered structured doc available"
                    );
                    (native_text, ExtractionMethod::Native)
                }
                ocr::OcrGateOutcome::SkipSubstantive => {
                    tracing::debug!(
                        total_chars,
                        alnum_ws_ratio,
                        ocr_fallback = decision.fallback,
                        "Skipping OCR: pre-rendered structured doc available with substantive native text"
                    );
                    (native_text, ExtractionMethod::Native)
                }
                ocr::OcrGateOutcome::RunFallback => {
                    // With marked pages the retry below may keep some pages native, and the formula and
                    // region-VLM stages read their rasters, so the inputs are held until the retry is
                    // decided. The whole read consumes the reusable Markdown inputs, so a clone is held
                    // when a native reader is configured; the clone holds every page raster a second time
                    // until the retry is decided. ~keep
                    #[cfg(all(
                        feature = "layout-detection",
                        any(feature = "formula-recognition", feature = "liter-llm")
                    ))]
                    let mut held_markdown_inputs = if native_script_pages.is_empty() {
                        drop(native_only_layout_inputs.take());
                        None
                    } else if native_only_layout_inputs.is_none() && native_layout_readers_configured(config) {
                        Some((markdown_layout_images.clone(), markdown_layout_detections.clone()))
                    } else {
                        None
                    };
                    // `images.run_ocr_on_images` controls whether extracted images are
                    // separately OCR'd (`core/pipeline/mod.rs`); it does not gate this
                    // document-level page OCR fallback. Reading it here made any
                    // `ImageExtractionConfig` (whose `run_ocr_on_images` defaults to `true`)
                    // silently suppress page OCR on scanned PDFs, leaving `pages[].content`
                    // empty (#1576). `RunFallbackOnPages` below has never read this field;
                    // this now matches it. ~keep
                    let whole_read = run_ocr_with_layout_pages(
                        content,
                        config,
                        path,
                        PageOcrLayoutContext {
                            pages: layout_pages,
                            hints: whole_document_ocr_hints(&pdf_metadata, &native_text, boundaries.as_deref(), config),
                        },
                        #[cfg(feature = "layout-detection")]
                        markdown_layout_images.take(),
                        #[cfg(feature = "layout-detection")]
                        markdown_layout_detections.take(),
                        #[cfg(feature = "layout-detection")]
                        markdown_layout_acceleration_override.take(),
                    )
                    .await;
                    // Some marked pages lost their script to the whole read while others did not
                    // (a scan beside a native page): the pages go through the per-page route, which
                    // keeps the native text of the marked ones. That is a second OCR run over the
                    // document, and the per-page route reports its own warnings and page failures, so
                    // those of the discarded whole read are dropped with it. When the retry fails the
                    // whole read stays the fallback. ~keep
                    let dropped_pages = match (&whole_read, boundaries.as_deref()) {
                        (Ok(read), Some(bounds)) if !bounds.is_empty() => {
                            let read_pages = &read.5;
                            if every_page_dropped_script(&native_script_pages, &native_text, Some(bounds), read_pages) {
                                Vec::new()
                            } else {
                                pages_that_dropped_script(&native_script_pages, &native_text, Some(bounds), read_pages)
                            }
                        }
                        _ => Vec::new(),
                    };
                    let partial_script_drop = !dropped_pages.is_empty();
                    let targeted_retry = match boundaries.as_deref() {
                        Some(bounds) if partial_script_drop => Some(
                            ocr::extract_mixed_ocr_native_with_layout_pages(
                                &native_text,
                                bounds,
                                (
                                    &decision.failing_pages,
                                    pdf_metadata
                                        .pdf_specific
                                        .fabricated_text_pages
                                        .as_deref()
                                        .unwrap_or(&[]),
                                ),
                                content,
                                config,
                                layout_pages,
                                (&outlined_ocr_pages, &native_script_pages),
                            )
                            .await,
                        ),
                        _ => None,
                    };
                    let retried = match targeted_retry {
                        Some(Ok(mixed_run)) => {
                            // A page the retry could not read that the whole read did read would come out
                            // empty, so the retry is discarded and the whole read used. ~keep
                            let loses_whole_read = whole_read.as_ref().is_ok_and(|read| {
                                mixed_run.output.9.iter().any(|failure| {
                                    !failure.recovered
                                        && !dropped_pages.contains(&failure.page)
                                        && failure
                                            .page
                                            .checked_sub(1)
                                            .and_then(|index| read.5.get(index as usize))
                                            .is_some_and(|text| !text.trim().is_empty())
                                })
                            });
                            if loses_whole_read {
                                tracing::warn!(
                                    failing_pages = ?decision.failing_pages,
                                    "Targeted OCR could not read a page the whole-document read had read; \
                                     using the whole-document read"
                                );
                                ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                    source: std::borrow::Cow::Borrowed("ocr"),
                                    message: std::borrow::Cow::Owned(format!(
                                        "Pages {dropped_pages:?} are in a script that the OCR read did not \
                                         produce, and the targeted OCR that keeps their native text could not \
                                         read every other page; their native text was replaced by the \
                                         whole-document OCR read."
                                    )),
                                });
                                None
                            } else {
                                Some(apply_targeted_mixed_run!(mixed_run))
                            }
                        }
                        Some(Err(error)) => {
                            tracing::warn!(
                                error = %error,
                                failing_pages = ?decision.failing_pages,
                                "Targeted OCR of the pages that dropped their native script failed; \
                                 using the whole-document read"
                            );
                            ensure_pdf_not_cancelled(config)?;
                            ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                source: std::borrow::Cow::Borrowed("ocr"),
                                message: std::borrow::Cow::Owned(format!(
                                    "Pages {dropped_pages:?} are in a script that the OCR read did not produce, \
                                     and the targeted OCR that keeps their native text failed ({error}); \
                                     their native text was replaced by the whole-document OCR read."
                                )),
                            });
                            None
                        }
                        None => None,
                    };
                    #[cfg(all(
                        feature = "layout-detection",
                        any(feature = "formula-recognition", feature = "liter-llm")
                    ))]
                    if retried.is_some() {
                        native_only_layout_inputs = native_only_layout_inputs.take().or(held_markdown_inputs.take());
                    } else {
                        drop(native_only_layout_inputs.take());
                        drop(held_markdown_inputs.take());
                    }
                    if let Some(retried) = retried {
                        retried
                    } else {
                        match whole_read {
                            Ok((
                                ocr_text,
                                ocr_tbls,
                                ocr_elems,
                                ocr_doc,
                                llm_usage,
                                ocr_pts,
                                ocr_rstrs,
                                formulas,
                                preprocessing,
                                page_ocr_confidence,
                                gate_audit,
                                layout_warning,
                                layout_glyph_drop_warnings,
                                discarded_visible_native_page,
                            )) if !every_page_dropped_script(
                                &native_script_pages,
                                &native_text,
                                boundaries.as_deref(),
                                &ocr_pts,
                            ) =>
                            {
                                if let Some(warning) = layout_warning {
                                    crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
                                }
                                for warning in layout_glyph_drop_warnings {
                                    crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
                                }
                                let preserve_native_after_discard = discarded_visible_native_page
                                    && pdf_metadata
                                        .pdf_specific
                                        .fabricated_text_pages
                                        .as_ref()
                                        .is_some_and(Vec::is_empty)
                                    && pdf_metadata
                                        .pdf_specific
                                        .implausible_text_pages
                                        .as_ref()
                                        .is_some_and(Vec::is_empty);
                                if preserve_native_after_discard {
                                    let (warnings, failures) = ocr_doc
                                        .map(|doc| (doc.processing_warnings, doc.ocr_page_failures))
                                        .unwrap_or_default();
                                    crate::core::diagnostics::dedup_extend_warnings(
                                        &mut ocr_fallback_warnings,
                                        warnings,
                                    );
                                    ocr_page_failures.extend(failures);
                                    (native_text, ExtractionMethod::Native)
                                } else {
                                    ocr_layout_gate_audit = gate_audit;
                                    ocr_tables = ocr_tbls;
                                    ocr_elements = ocr_elems;
                                    ocr_internal_doc = ocr_doc;
                                    ocr_llm_usage = llm_usage;
                                    ocr_page_texts = Some(ocr_pts);
                                    ocr_page_rasters = ocr_rstrs;
                                    ocr_formulas = formulas;
                                    ocr_preprocessing_by_page = preprocessing;
                                    ocr_confidence_by_page = page_ocr_confidence;
                                    (ocr_text, ExtractionMethod::Ocr)
                                }
                            }
                            Ok((_, _, _, ocr_doc, _, _, _, _, _, _, _, _, _, _)) => {
                                let (warnings, failures) = ocr_doc
                                    .map(|doc| (doc.processing_warnings, doc.ocr_page_failures))
                                    .unwrap_or_default();
                                // The discarded read's text is not in the result: its failures were not
                                // recovered by it, and layout inputs and llm_usage go with it, as in the
                                // sibling arms.
                                crate::core::diagnostics::dedup_extend_warnings(
                                    &mut ocr_fallback_warnings,
                                    warnings
                                        .into_iter()
                                        .filter(|warning| !warning.message.contains("was recovered from"))
                                        .collect(),
                                );
                                ocr_page_failures.extend(failures.into_iter().map(|failure| {
                                    crate::types::OcrPageFailure {
                                        recovered: false,
                                        ..failure
                                    }
                                }));
                                crate::core::diagnostics::dedup_extend_warnings(
                                    &mut ocr_fallback_warnings,
                                    dropped_script_warnings(&native_script_pages, &native_text, boundaries.as_deref()),
                                );
                                (native_text, ExtractionMethod::Native)
                            }
                            Err(failed_run) => {
                                tracing::warn!(
                                    error = %failed_run.error,
                                    "OCR fallback failed; using native text extraction result"
                                );
                                if failed_ocr_fallback_is_total_loss(&native_text) {
                                    return Err(failed_run.error);
                                }
                                // ~keep A cancelled run did not fail to read its pages, so it adds no
                                // record. The token decides: a pipeline run reports a cancellation as
                                // the failure of each of its stages, not as the cancellation error.
                                if ensure_pdf_not_cancelled(config).is_ok() {
                                    ocr_page_failures.extend(ocr::failed_run_page_failures(
                                        &failed_run,
                                        pdf_metadata.pdf_specific.page_count.unwrap_or(0),
                                        &native_text,
                                        boundaries.as_deref().unwrap_or(&[]),
                                    ));
                                }
                                let e = &failed_run.error;
                                ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                    source: std::borrow::Cow::Borrowed("ocr"),
                                    message: std::borrow::Cow::Owned(format!(
                                        "OCR fallback failed ({e}); returning native text that was below the \
                                         quality threshold which triggered OCR. Extracted content may be empty \
                                         or incomplete."
                                    )),
                                });
                                (native_text, ExtractionMethod::Native)
                            }
                        }
                    }
                }
                ocr::OcrGateOutcome::RunFallbackOnPages(pages) => match boundaries.as_deref() {
                    Some(bounds) if !bounds.is_empty() => {
                        match ocr::extract_mixed_ocr_native_with_layout_pages(
                            &native_text,
                            bounds,
                            (
                                &pages,
                                pdf_metadata
                                    .pdf_specific
                                    .fabricated_text_pages
                                    .as_deref()
                                    .unwrap_or(&[]),
                            ),
                            content,
                            config,
                            layout_pages,
                            (&outlined_ocr_pages, &native_script_pages),
                        )
                        .await
                        {
                            Ok(mixed_run) => apply_targeted_mixed_run!(mixed_run),
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    failing_pages = ?pages,
                                    "Targeted OCR fallback failed; using native text extraction result"
                                );
                                if failed_ocr_fallback_is_total_loss(&native_text) {
                                    return Err(e);
                                }
                                ocr_page_failures.extend(ocr::route_level_ocr_page_failures(
                                    &pages,
                                    &e,
                                    &native_text,
                                    bounds,
                                ));
                                ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                    source: std::borrow::Cow::Borrowed("ocr"),
                                    message: std::borrow::Cow::Owned(format!(
                                        "Targeted OCR fallback failed ({e}) for pages {pages:?}; those pages \
                                         retain native text that was below the OCR-trigger quality threshold \
                                         and may be empty or incomplete."
                                    )),
                                });
                                (native_text, ExtractionMethod::Native)
                            }
                        }
                    }
                    _ => {
                        tracing::warn!(
                            failing_pages = ?pages,
                            "Targeted OCR requested but no page boundaries available; using native text"
                        );
                        if failed_ocr_fallback_is_total_loss(&native_text) {
                            return Err(crate::XbergError::Plugin {
                                message: format!(
                                    "Targeted OCR was required for pages {pages:?} but no page boundaries \
                                     were available, and no native text could be recovered"
                                ),
                                plugin_name: "ocr".to_string(),
                            });
                        }
                        ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("ocr"),
                            message: std::borrow::Cow::Owned(format!(
                                "Targeted OCR was required for pages {pages:?} but no page boundaries were \
                                 available; those pages retain native text that was below the OCR-trigger \
                                 quality threshold and may be empty or incomplete."
                            )),
                        });
                        (native_text, ExtractionMethod::Native)
                    }
                },
                #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
                ocr::OcrGateOutcome::UseNative => (native_text, ExtractionMethod::Native),
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                ocr::OcrGateOutcome::UseNative => {
                    let probe = ocr::probe_incomplete_hidden_layers(
                        &native_text,
                        boundaries.as_deref().unwrap_or(&[]),
                        pdf_metadata.pdf_specific.scanned_pages.as_deref().unwrap_or(&[]),
                        content,
                        config,
                    )
                    .await?;
                    if let Some(mixed_run) = probe {
                        let (text, replacements, _, _, _, _, preprocessing, confidence, warnings, page_failures) =
                            mixed_run.output;
                        let method = extraction_method_after_mixed_ocr(&replacements);
                        ocr_preprocessing_by_page.extend(
                            preprocessing
                                .into_iter()
                                .filter(|(page, _)| replacements.contains_key(page)),
                        );
                        ocr_confidence_by_page.extend(
                            confidence
                                .into_iter()
                                .filter(|(page, _)| replacements.contains_key(page)),
                        );
                        ocr_fallback_warnings.extend(warnings);
                        ocr_page_failures.extend(page_failures);
                        ocr_results_map = Some(replacements);
                        (text, method)
                    } else {
                        (native_text, ExtractionMethod::Native)
                    }
                }
            }
        } else {
            (native_text, ExtractionMethod::Native)
        };

        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let (mut text, extraction_method) = (native_text, ExtractionMethod::Native);

        // The routing gates above have read the lost-glyph markers. Pages that stay native must not
        // show them, so remove them from every native structure before any later step reads it.
        // The text itself, which carries byte offsets, follows once its page boundaries are known. ~keep
        for page in page_contents.iter_mut().flatten() {
            unmapped_glyphs::strip_page(page);
        }
        for table in &mut tables {
            unmapped_glyphs::strip_table(table);
        }
        if let Some(native_doc) = pre_rendered_doc.as_mut() {
            unmapped_glyphs::strip_document(native_doc);
        }

        #[cfg(all(
            feature = "pdf",
            feature = "layout-detection",
            any(feature = "ocr", feature = "ocr-pipeline"),
            any(feature = "formula-recognition", feature = "liter-llm")
        ))]
        if let Some((images, detections)) = native_only_layout_inputs {
            markdown_layout_images = images;
            markdown_layout_detections = detections;
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        // Full-document OCR is authoritative for tables when it produced
        // them. The structured OCR document already contains the same table
        // values, so later document assembly must not inject them a second
        // time. ~keep
        replace_tables_with_ocr_output(&mut tables, ocr_tables);

        let (images, image_fallback_warning): (
            Option<Vec<crate::types::ExtractedImage>>,
            Option<crate::types::ProcessingWarning>,
        ) = (extracted_images, None);

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        {
            let keep_native_content = config.pages.as_ref().is_some_and(|pages| pages.keep_native_content);

            if let Some(pts) = ocr_page_texts.as_ref() {
                if let Some(ref mut pages) = page_contents {
                    replace_pages_with_ocr_page_texts(pages, pts, keep_native_content);
                } else {
                    page_contents = Some(
                        pts.iter()
                            .enumerate()
                            .map(|(i, text)| {
                                let content = crate::pdf::text::fix_pdf_control_chars(text).into_owned();
                                let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&content));
                                crate::types::PageContent {
                                    page_number: (i + 1) as u32,
                                    content,
                                    tables: Vec::new(),
                                    image_indices: vec![],
                                    image_preprocessing: None,
                                    hierarchy: None,
                                    is_blank,
                                    layout_regions: None,
                                    speaker_notes: None,
                                    section_name: None,
                                    sheet_name: None,
                                    ocr_confidence: None,
                                    native_content: None,
                                }
                            })
                            .collect(),
                    );
                }
            }

            if let Some(results_map) = ocr_results_map.as_ref()
                && let Some(ref mut pages) = page_contents
            {
                for page in pages.iter_mut() {
                    if let Some(ocr_text) = results_map.get(&page.page_number) {
                        replace_page_content_with_ocr_text(page, ocr_text, keep_native_content);
                    }
                }
            }

            if let Some(ref content_pages) = page_contents
                && let Some(ref mut page_structure) = pdf_metadata.page_structure
                && let Some(ref mut info_pages) = page_structure.pages
            {
                for info in info_pages.iter_mut() {
                    if let Some(content_page) = content_pages.iter().find(|p| p.page_number == info.number) {
                        info.is_blank = content_page.is_blank;
                    }
                }
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let image_preprocessing = attach_pdf_preprocessing_metadata(&mut page_contents, &ocr_preprocessing_by_page);
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        attach_pdf_ocr_confidence(&mut page_contents, &ocr_confidence_by_page);
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let image_preprocessing = None;

        #[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(unused_variables))]
        let annotation_fallback_applied = annotation_text_fallback.is_some();
        if let Some(annotation_text_fallback) = annotation_text_fallback.as_ref() {
            extraction::apply_annotation_text_fallback(
                annotation_text_fallback,
                #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
                ocr_page_texts.as_deref(),
                #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
                None,
                #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
                ocr_results_map.as_ref(),
                #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
                None,
                #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
                (extraction_method == ExtractionMethod::Ocr),
                #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
                false,
                extraction::AnnotationFallbackTarget {
                    text: &mut text,
                    page_contents: &mut page_contents,
                    boundaries: &mut boundaries,
                    pdf_metadata: &mut pdf_metadata,
                    page_config: config.pages.as_ref(),
                },
            );
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if extraction_method == ExtractionMethod::Mixed
            && let Some(ref accepted_pages) = structured_ocr_pages
        {
            merge_mixed_ocr_tables(&mut tables, accepted_pages);
        }

        let mut final_pages =
            assign_tables_and_images_to_pages(page_contents, &tables, images.as_deref().unwrap_or(&[]));

        let pre_formatted_output: Option<String> = None;

        let used_ocr = extraction_method.used_ocr();

        // `boundaries` index the NATIVE text; on the mixed path the OCR replacements have
        // already shifted every later offset, so re-map before handing them to the document
        // selector or page tagging would attribute content to the wrong page. ~keep
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut selector_boundaries = if annotation_fallback_applied {
            boundaries.clone()
        } else {
            boundaries_for_ocr_output(
                extraction_method,
                &text,
                boundaries.as_deref(),
                ocr_results_map.as_ref(),
                ocr_page_texts.as_deref(),
            )
        };
        // OCR text replaces the native text on a fully OCR-read document; any other text is native.
        let lost_glyphs = if extraction_method == ExtractionMethod::Ocr {
            0
        } else {
            #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
            let page_boundaries = selector_boundaries.as_mut();
            #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
            let page_boundaries = boundaries.as_mut();
            #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
            let ocr_pages: Vec<u32> = ocr_results_map
                .iter()
                .flat_map(|replaced| replaced.keys().copied())
                .collect();
            #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
            let ocr_pages: Vec<u32> = Vec::new();
            unmapped_glyphs::strip_text(&mut text, page_boundaries, &ocr_pages)
        };
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if let Some(ref mut page_structure) = pdf_metadata.page_structure {
            page_structure.boundaries = selector_boundaries.clone();
        }
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        if lost_glyphs > 0
            && let Some(ref mut page_structure) = pdf_metadata.page_structure
        {
            page_structure.boundaries = boundaries.clone();
        }
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let (mut doc, document_origin, document_is_structured) = select_pdf_document(
            extraction_method,
            &text,
            mime_type,
            pre_rendered_doc,
            ocr_internal_doc.take(),
            ocr_results_map.as_ref(),
            structured_ocr_pages.as_ref(),
            selector_boundaries.as_deref(),
            &config.output_format,
        );
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if extraction_method == ExtractionMethod::Mixed
            && let Some(ref accepted_pages) = structured_ocr_pages
        {
            ocr_elements = accepted_mixed_ocr_elements(accepted_pages);
            ocr_coordinate_frames = accepted_mixed_ocr_coordinate_frames(accepted_pages);
        }
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let (mut doc, document_is_structured) =
            select_native_pdf_document(&text, mime_type, pre_rendered_doc, boundaries.as_deref());
        // #1575: `doc.metadata` is fully replaced by a fresh `Metadata { .. }` literal below
        // (native/mixed extraction never populates `doc.metadata` this early, so this is a
        // no-op for them), which would otherwise discard the OCR pipeline's `psm`/`language`/
        // `tesseract_dict_invalid_word_ratio` keys set on `ocr_internal_doc.metadata.additional`
        // (`extract_with_ocr_for_page`) before they ever reached a caller. Taken here and
        // merged back in after that literal is built. ~keep
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let ocr_backend_additional_metadata = std::mem::take(&mut doc.metadata.additional);
        if let Some(annotation_text_fallback) = annotation_text_fallback.as_ref() {
            extraction::append_annotation_fallback_elements(annotation_text_fallback, &mut doc);
        }
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        tracing::debug!(?document_origin, document_is_structured, "selected PDF document origin");

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        crate::core::diagnostics::dedup_extend_warnings(
            &mut doc.processing_warnings,
            std::mem::take(&mut ocr_fallback_warnings),
        );
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        doc.ocr_page_failures.append(&mut ocr_page_failures);
        if let Some(warning) = unmapped_glyphs::lost_glyph_warning(lost_glyphs, config) {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        crate::core::diagnostics::dedup_extend_warnings(
            &mut doc.processing_warnings,
            std::mem::take(&mut pdf_extraction_warnings),
        );
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        doc.processing_warnings.append(&mut pdf_extraction_warnings);

        // #340: drain any xberg_native_pdf glyph-drop warnings captured while this
        // document's pages were rendered by *this* task (OCR rasterization
        // and any other call routed through `render_page_capturing_glyph_drops`
        // in `crate::pdf::render` that runs inline on it;
        // `with_task_render_warnings` keeps them with the task). The drain is unconditionally cheap and correct even when
        // nothing was ever captured: `install_pdf_render_diagnostics` is
        // opt-in and only an embedding application calls it (a library must
        // not seize the process-global `log` backend on its own — see
        // `pdf::render` for why), so this buffer stays empty for every caller
        // that has not opted in and this loop is a no-op `Vec::is_empty`
        // check. Deduped because a multi-page document can hit the identical
        // xberg_native_pdf cause on many pages.
        //
        // Layout-detection rasterization runs inside `tokio::task::spawn_blocking`
        // on a different OS thread, so this drain never sees those warnings —
        // `layout_runner::run_layout_for_pdf_pages_async` drains them itself
        // from inside that closure and threads them back through its return
        // value instead (#353); they are merged below via
        // `markdown_layout_glyph_drop_warnings` (markdown path) and were
        // already folded into `ocr_fallback_warnings` above (OCR path). ~keep
        for pdf_render_warning in crate::pdf::render::take_xberg_native_pdf_render_warnings() {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, pdf_render_warning);
        }

        // Surface a hard layout-inference failure (e.g. CoreML kernel error) so
        // degraded no-layout output is never silent to the caller (#1344). Runs
        // whenever layout-detection is on, independent of OCR.
        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        if let Some(warning) = markdown_layout_warning {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
        }

        // #353: merge `xberg_native_pdf` glyph-drop warnings captured while the
        // markdown layout pass rendered its pages off-thread inside
        // `spawn_blocking` (see the drain comment above).
        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        for warning in markdown_layout_glyph_drop_warnings {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
        }

        // Record the layout gate's audit trail from whichever pass ran
        // it. Compiled whenever `pdf` is on so `ocr_layout_gate_audit` keeps
        // a reader in profiles without layout-detection (it is always
        // `(None, None)` there and the fields stay `None`). ~keep
        {
            #[cfg(feature = "layout-detection")]
            #[allow(unused_mut)]
            let (mut gated_pages, mut gate_reasons) = layout_gate_metadata(markdown_layout_gate_decisions.as_deref());
            #[cfg(not(feature = "layout-detection"))]
            #[allow(unused_mut)]
            let (mut gated_pages, mut gate_reasons): OcrLayoutGateDecisions = (None, None);
            #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
            if gated_pages.is_none() {
                let (ocr_gated_pages, ocr_gate_reasons) = ocr_layout_gate_audit;
                gated_pages = ocr_gated_pages;
                gate_reasons = ocr_gate_reasons;
            }
            pdf_metadata.pdf_specific.layout_gated_pages = gated_pages;
            pdf_metadata.pdf_specific.layout_gate_reasons = gate_reasons;
        }

        let extract_pdf_metadata = config
            .pdf_options
            .as_ref()
            .is_none_or(|options| options.extract_metadata);
        doc.metadata = Metadata {
            output_format: pre_formatted_output,
            image_preprocessing,
            ocr_used: used_ocr,
            ..Default::default()
        };
        apply_extracted_pdf_metadata(&mut doc.metadata, pdf_metadata, extract_pdf_metadata);
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed("extraction_method"),
            serde_json::Value::String(extraction_method.as_str().to_string()),
        );
        // #1575: restore the OCR-pipeline keys taken above, now that the fresh `Metadata`
        // literal they would otherwise have been lost to already exists.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        doc.metadata.additional.extend(ocr_backend_additional_metadata);

        // GH#1645: one authoritative processed-raster coordinate frame per OCR page with
        // public elements, so a multi-page consumer can normalize `OcrElement` geometry
        // without relying on the single document-wide `ocr_processed_image_width/height`
        // pair, which cannot describe differently sized, preprocessed, or rotated pages.
        // Rides in `additional` rather than a new public binding type, same as the
        // `page_labels` key below (issue #66).
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_coordinate_frames.is_empty() {
            doc.metadata.additional.insert(
                std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PAGE_COORDINATE_FRAMES_METADATA_KEY),
                serde_json::json!(ocr_coordinate_frames),
            );
        }

        // Issue #66: `/PageLabels` — one display label per page, index-aligned
        // with `pdf_metadata.page_structure`/`PageBoundary::page_number`.
        // `PdfMetadata` is an alef-listed type (no new public fields), so this
        // rides in `additional` instead. ~keep
        if extract_pdf_metadata && let Some(labels) = pdf_page_labels {
            doc.metadata
                .additional
                .insert(std::borrow::Cow::Borrowed("page_labels"), serde_json::json!(labels));
        }

        // Issue #64: surface filled field values in rendered content for
        // output shapes (Markdown/HTML/Djot) that the plain-text widget
        // splice in `native::text` never touches.
        inject_unrepresented_form_field_elements(&mut doc, &pdf_form_fields);
        doc.form_fields = pdf_form_fields;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let allow_table_injection = (!document_is_structured
            && (config.output_format != crate::core::config::OutputFormat::Plain
                || document_origin == PdfDocumentOrigin::Ocr))
            || (document_origin == PdfDocumentOrigin::Ocr && doc.tables.is_empty());
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let allow_table_injection =
            document_is_structured || config.output_format != crate::core::config::OutputFormat::Plain;
        attach_unrepresented_tables(&mut doc, tables);
        inject_unrepresented_table_elements(&mut doc, allow_table_injection);

        if let Some(imgs) = images {
            // The OCR path has its own guarded injection block below (see the `#[cfg(feature = "ocr")]`
            let image_anchor = extraction::image_anchor(config);
            let document_has_image_elements = doc.elements.iter().any(|element| {
                #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                if element.image_ocr_text_anchor_index().is_some() {
                    return true;
                }
                matches!(element.kind, ElementKind::Image { .. })
            });
            if !document_has_image_elements && image_anchor != extraction::ImageAnchor::None {
                for (idx, img) in imgs.iter().enumerate() {
                    // Page OCR already read the picture as part of the page, so the picture's
                    // own OCR text is not rendered there: no anchor, and a placeholder without
                    // the text. The OCR merge of a structured document does the same for the
                    // pages it replaces. ~keep
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                    let page_text_is_from_ocr = extraction_method == ExtractionMethod::Ocr
                        || img.page_number.is_some_and(|page| {
                            ocr_results_map
                                .as_ref()
                                .is_some_and(|results| results.contains_key(&page))
                        });
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                    if image_anchor == extraction::ImageAnchor::OcrTextOnly {
                        if !page_text_is_from_ocr {
                            insert_image_ocr_text_anchor(
                                &mut doc,
                                InternalElement::image_ocr_text_anchor(idx as u32, img.page_number),
                            );
                        }
                        continue;
                    }
                    let mut elem = InternalElement::text(
                        ElementKind::Image {
                            image_index: idx as u32,
                        },
                        "",
                        0,
                    );
                    elem.page = img.page_number;
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                    if page_text_is_from_ocr {
                        elem.suppress_image_ocr_rendering();
                    }
                    doc.push_element(elem);
                }
            }
            doc.images = imgs;
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let ocr_rasters_bypass = ocr_page_rasters.is_none();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if let Some(rasters) = ocr_page_rasters {
            let base_idx = doc.images.len() as u32;
            for (offset, mut raster) in rasters.into_iter().enumerate() {
                raster.image_index = base_idx + offset as u32;
                doc.images.push(raster);
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if used_ocr
            && ocr_rasters_bypass
            && extraction_method == ExtractionMethod::Ocr
            && config.images.as_ref().is_some_and(|c| c.include_page_rasters)
        {
            doc.processing_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("page_rasters"),
                message: std::borrow::Cow::Borrowed(
                    "include_page_rasters is set but no page rasters were produced; \
                     the active OCR backend used document-level processing without per-page rendering",
                ),
            });
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if used_ocr && !doc.images.is_empty() {
            let images_enabled = config.images.as_ref().map(|c| c.extract_images).unwrap_or(false)
                || config.pdf_options.as_ref().map(|p| p.extract_images).unwrap_or(false);
            if images_enabled && config.images.as_ref().map(|c| c.inject_placeholders).unwrap_or(false) {
                let referenced_images: std::collections::HashSet<u32> = doc
                    .elements
                    .iter()
                    .filter_map(|element| match element.kind {
                        ElementKind::Image { image_index } => Some(image_index),
                        _ => None,
                    })
                    .collect();
                let elems: Vec<InternalElement> = doc
                    .images
                    .iter()
                    .filter(|image| !referenced_images.contains(&image.image_index))
                    .map(|img| {
                        let elem = InternalElement::text(
                            ElementKind::Image {
                                image_index: img.image_index,
                            },
                            "",
                            0,
                        );
                        if let Some(page) = img.page_number {
                            elem.with_page(page)
                        } else {
                            elem
                        }
                    })
                    .collect();
                for elem in elems {
                    doc.push_element(elem);
                }
            }
        }

        if let Some(warning) = image_fallback_warning {
            doc.processing_warnings.push(warning);
        }
        doc.annotations = pdf_annotations;

        {
            use crate::types::annotations::PdfAnnotationType;
            use crate::types::uri::{ExtractedUri, UriKind};

            let uris: Vec<ExtractedUri> = doc
                .annotations
                .as_ref()
                .map(|annotations| {
                    annotations
                        .iter()
                        .filter(|a| a.annotation_type == PdfAnnotationType::Link)
                        .filter_map(|a| {
                            a.content.as_ref().map(|url| {
                                let kind = if url.starts_with('#') {
                                    UriKind::Anchor
                                } else if url.starts_with("mailto:") {
                                    UriKind::Email
                                } else {
                                    UriKind::Hyperlink
                                };
                                ExtractedUri {
                                    url: url.clone(),
                                    label: Some(url.clone()),
                                    page: Some(a.page_number),
                                    kind,
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            for uri in uris {
                doc.push_uri(uri);
            }
        }

        #[cfg(feature = "pdf")]
        {
            for uri in bookmark_uris {
                doc.push_uri(uri);
            }
            doc.revisions = pdf_revisions;
        }

        #[cfg(all(feature = "pdf", feature = "tokio-runtime"))]
        {
            let (embedded_children, embedded_warnings) =
                crate::pdf::embedded_files::extract_and_process_embedded_files(content, config).await;
            if !embedded_children.is_empty() {
                match doc.children {
                    Some(ref mut existing) => existing.extend(embedded_children),
                    None => doc.children = Some(embedded_children),
                }
            }
            for warning in embedded_warnings {
                doc.processing_warnings.push(warning);
            }
        }

        if let Some(ref mut pages) = final_pages {
            assign_hierarchy_to_pages(pages, &doc);
        }

        // GH#1653 + GH#1654: one raw-MediaBox coordinate frame per page that ended up with
        // hierarchy blocks, now that `assign_hierarchy_to_pages` above has populated
        // `PageContent::hierarchy`. One shared record covers both issues: a consumer needs
        // the MediaBox origin (which can be non-zero and negative) and the page rotation
        // together to place a page's raw-space geometry. Rides in `additional` rather than a
        // new public binding type, same as the `page_labels` key below (issue #66) and
        // `ocr_page_coordinate_frames` above (GH#1645). ~keep
        if let Some(ref pages) = final_pages {
            let pages_with_hierarchy: std::collections::HashSet<u32> = pages
                .iter()
                .filter(|page| {
                    page.hierarchy
                        .as_ref()
                        .is_some_and(|hierarchy| !hierarchy.blocks.is_empty())
                })
                .map(|page| page.page_number)
                .collect();
            let pdf_page_coordinate_frames: Vec<_> = pdf_page_coordinate_frames
                .into_iter()
                .filter(|frame| pages_with_hierarchy.contains(&frame.page_number))
                .collect();
            if !pdf_page_coordinate_frames.is_empty() {
                doc.metadata.additional.insert(
                    std::borrow::Cow::Borrowed("pdf_page_coordinate_frames"),
                    serde_json::json!(pdf_page_coordinate_frames),
                );
            }
        }

        // Native snapshots were captured before canonical filtering, ordering and IDs.
        // Reconcile only tables here; OCR and custom extractor page ownership is separate.
        if extraction_method == ExtractionMethod::Native
            && let Some(ref mut pages) = final_pages
        {
            reconcile_native_page_tables(pages, &doc.tables);
        }
        doc.prebuilt_pages = final_pages;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_elements.is_empty() {
            doc.prebuilt_ocr_elements = Some(ocr_elements);
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_formulas.is_empty() {
            doc.formulas = ocr_formulas;
        }

        // Native pages with layout hints carry formula elements holding the
        // region's glyph text; a configured model replaces it with LaTeX.
        // Accepted mixed OCR pages use their own layout inputs; untouched pages
        // retain their Markdown inputs. Each page has one formula owner. ~keep
        #[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
        if let Some(layout) = config.layout.as_ref() {
            let mut side_channel = std::mem::take(&mut doc.formulas);
            let mixed_inputs = mixed_formula_layout_inputs.take();
            let inputs = select_formula_page_inputs(
                markdown_layout_images
                    .as_deref()
                    .zip(markdown_layout_detections.as_deref()),
                mixed_inputs
                    .as_ref()
                    .map(|(images, detections)| (images.as_slice(), detections.as_slice())),
            );
            recognize_pdf_formula_page_inputs(Some(&mut doc), &mut side_channel, &inputs, layout, content).await;
            doc.formulas = side_channel;
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_llm_usage.is_empty() {
            doc.llm_usage = Some(ocr_llm_usage);
        }

        doc.diagrams = diagrams;

        tracing::debug!(
            elements = doc.elements.len(),
            tables = doc.tables.len(),
            has_pages = doc.prebuilt_pages.is_some(),
            diagrams = doc.diagrams.len(),
            "InternalDocument finalized (native path)"
        );

        #[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
        {
            if let Some(vlm_cfg) = region_vlm::region_vlm_config(config)
                && let (Some(layout_images), Some(hints), Some(detections)) = (
                    markdown_layout_images.as_deref(),
                    markdown_layout_hints.as_deref(),
                    markdown_layout_detections.as_deref(),
                )
            {
                let region_results = region_vlm::extract_vlm_regions(layout_images, hints, detections, vlm_cfg).await;
                if !region_results.is_empty() {
                    tracing::debug!(
                        count = region_results.len(),
                        "injecting VLM region results into document"
                    );
                    region_vlm::inject_region_results(&mut doc, region_results);
                }
            }
        }

        {
            let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
            for elem in &doc.elements {
                budget.account_text(elem.text.len())?;
            }
        }

        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if extraction_method == ExtractionMethod::Native
            && let Some(placement) = native_output.image_gap
        {
            crate::extraction::image_ocr::pdf_gap::publish(placement);
        }
        Ok(doc)
    }

    /// Fallback extraction path when pdf feature is not enabled.
    #[cfg(not(feature = "pdf"))]
    async fn extract_core_native(
        &self,
        _content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        let doc = InternalDocument::new(mime_type);
        Ok(doc)
    }
}

#[cfg(test)]
mod tests;
