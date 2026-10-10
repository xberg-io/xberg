//! Real extraction engine for `PdfBackend::Pdfium` (issue #702).
//!
//! # Scope
//!
//! Deliberately smaller than [`super::PdfExtractor::extract_core_native`]: page
//! count, per-page plain text (via `PdfPageText::inside_rect()` with the resolved
//! page margins), and Info-dictionary
//! metadata (Title/Author/Subject/Keywords/Creator/Producer/CreationDate/
//! ModificationDate). `xberg-pdfium-render`'s public API does not currently give
//! this crate anything to build the rest of the native contract from -- no table
//! detection, no diagram/DOT recovery, no layout-detection integration, no
//! annotation-to-`PdfAnnotation` mapping, no AcroForm/XFA field values, no
//! embedded-file extraction, and no OCR fallback path. None of that runs here.
//! Every call appends a `ProcessingWarning` naming the gap so a caller diffing
//! pdfium output against `xberg_native_pdf` output is never left assuming parity
//! silently. Header/footer inclusion is honored through the resolved page margins;
//! explicitly requested content filters that require document structure receive
//! separate capability warnings.
//!
//! # The pdfium library binding is process-global and happens once
//!
//! `pdfium_render::Pdfium` stores its resolved FFI bindings in a single
//! process-wide `once_cell::sync::OnceCell` internal to the fork. Binding twice
//! in one process is a logic error: `Pdfium::new`/`new_with_config` assert the
//! cell is still empty (a second call panics), and
//! `bind_to_library`/`bind_to_system_library` return
//! `PdfiumError::PdfiumLibraryBindingsAlreadyInitialized` on a second call even
//! when the first one succeeded. [`bind_once`] is therefore the *only* call site
//! in this engine that attempts a bind, and it caches the outcome -- success or
//! failure -- in its own `OnceCell` so every later [`extract`] call reuses that
//! outcome instead of touching the global a second time.
//!
//! # Library discovery is a runtime concern, not a build-time one
//!
//! `PDFIUM_STATIC_LIB_PATH` / `PDFIUM_DYNAMIC_LIB_PATH` only steer
//! `crates/xberg-pdfium-render/build.rs`'s linker flags at *build* time. The
//! dynamic binding path used here (no `pdfium_use_static` cfg -- this crate
//! never sets `PDFIUM_STATIC_LIB_PATH` for its own build) resolves every
//! `FPDF_*` symbol at *runtime* via `libloading`, which `dlopen`s the platform
//! library name (`libpdfium.so` / `.dylib` / `pdfium.dll`) from the current
//! working directory and then the system library search path. Neither env var
//! places anything there. [`bind_once`] additionally checks
//! `PDFIUM_DYNAMIC_LIB_PATH` at runtime and, if set, tries loading the
//! platform-named library from that directory before falling back to the
//! system search path. If nothing is found anywhere, extraction fails loudly
//! with `XbergError::MissingDependency` naming both env vars -- never a silent
//! empty document.

use once_cell::sync::OnceCell;
use pdfium_render::prelude::*;
use std::sync::Mutex;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::error::XbergError;
use crate::pdf::metadata::PdfMetadata as PdfSpecificMetadata;
use crate::pdf::native::text::PageMarginFractions;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractionMethod, Metadata, PageBoundary, PageStructure, PageUnitType, ProcessingWarning};

const PDFIUM_WARNING_SOURCE: &str = "pdf_backend_pdfium";
const UNSUPPORTED_REPEATING_TEXT_MESSAGE: &str =
    "the pdfium backend cannot apply ContentFilterConfig.strip_repeating_text; repeated text was preserved";
const UNSUPPORTED_FOOTNOTES_MESSAGE: &str = concat!(
    "the pdfium backend cannot apply ContentFilterConfig.include_footnotes; ",
    "footnote inclusion follows Pdfium's plain-text extraction"
);
const UNSUPPORTED_WATERMARKS_MESSAGE: &str = concat!(
    "the pdfium backend cannot apply ContentFilterConfig.include_watermarks; ",
    "watermark inclusion follows Pdfium's plain-text extraction"
);

/// Cached outcome of the one-and-only bind attempt for this process. `Ok(())`
/// once bound; `Err(message)` if binding failed, so later calls report the same
/// actionable error instead of re-touching the process-global bindings cell
/// (which would misreport a real failure as
/// `PdfiumLibraryBindingsAlreadyInitialized`).
static BIND_OUTCOME: OnceCell<std::result::Result<(), String>> = OnceCell::new();

/// Pdfium's public C API is process-global and does not support concurrent calls.
/// Keep the guard for the complete document lifetime, including `PdfDocument::drop()`,
/// so page/text handles cannot overlap another extraction's allocation and cleanup.
static PDFIUM_OPERATION_LOCK: Mutex<()> = Mutex::new(());

/// Binds the pdfium FFI library exactly once per process. See the module doc
/// comment for why this must not be attempted more than once, and why the two
/// `PDFIUM_*_LIB_PATH` env vars do not help locate the library here.
fn bind_once() -> Result<()> {
    let outcome = BIND_OUTCOME.get_or_init(|| {
        let bindings = if let Ok(dir) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir))
                .or_else(|_| Pdfium::bind_to_system_library())
        } else {
            Pdfium::bind_to_system_library()
        };

        match bindings {
            Ok(bindings) => {
                Pdfium::new(bindings);
                Ok(())
            }
            Err(err) => Err(format!(
                "could not load the pdfium shared library at runtime: {err}. Set \
                 PDFIUM_DYNAMIC_LIB_PATH to a directory containing the platform pdfium \
                 library (libpdfium.so / .dylib / pdfium.dll), set PDFIUM_STATIC_LIB_PATH \
                 to statically link pdfium into this binary at build time, or install \
                 pdfium on the system library search path."
            )),
        }
    });

    outcome
        .clone()
        .map_err(|message| XbergError::MissingDependency(format!("pdfium: {message}")))
}

/// Common (backend-agnostic) metadata read from the PDF Info dictionary.
#[derive(Default)]
struct CommonMetadata {
    title: Option<String>,
    subject: Option<String>,
    authors: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    created_at: Option<String>,
    modified_at: Option<String>,
    created_by: Option<String>,
    producer: Option<String>,
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() { None } else { Some(value) }
}

/// Reads the eight Info-dictionary tags pdfium exposes. Dates are returned as
/// the raw PDF date string (e.g. `D:20240102153000-05'00'`) rather than
/// normalized ISO 8601 -- `xberg-pdfium-render` gives no calendar parsing for
/// this format, and fabricating a normalized value would be worse than passing
/// through what the document actually says.
fn read_common_metadata(document: &PdfDocument<'_>) -> CommonMetadata {
    let metadata = document.metadata();
    let get = |tag: PdfDocumentMetadataTagType| -> Option<String> {
        metadata.get(tag).map(|t| t.value().to_string()).and_then(non_empty)
    };

    CommonMetadata {
        title: get(PdfDocumentMetadataTagType::Title),
        subject: get(PdfDocumentMetadataTagType::Subject),
        authors: get(PdfDocumentMetadataTagType::Author).map(|a| vec![a]),
        keywords: get(PdfDocumentMetadataTagType::Keywords).map(|k| {
            k.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        }),
        created_at: get(PdfDocumentMetadataTagType::CreationDate),
        modified_at: get(PdfDocumentMetadataTagType::ModificationDate),
        created_by: get(PdfDocumentMetadataTagType::Creator),
        producer: get(PdfDocumentMetadataTagType::Producer),
    }
}

/// Opens `content` with pdfium, trying each configured password in turn if the
/// document is encrypted and no password (or the wrong one) was supplied.
/// Mirrors the intent of `xberg_native_pdf`'s multi-password support without pulling
/// in its retry plumbing -- pdfium only accepts one password per open attempt.
fn open_document<'a>(pdfium: &'a Pdfium, content: &'a [u8], passwords: &[String]) -> Result<PdfDocument<'a>> {
    match pdfium.load_pdf_from_byte_slice(content, None) {
        Ok(document) => Ok(document),
        Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError)) if !passwords.is_empty() => {
            for password in passwords {
                if let Ok(document) = pdfium.load_pdf_from_byte_slice(content, Some(password.as_str())) {
                    return Ok(document);
                }
            }
            Err(XbergError::parsing(
                "pdfium could not open the document: it is password-protected and none of the \
                 configured passwords opened it",
            ))
        }
        Err(err) => Err(XbergError::parsing(format!(
            "pdfium failed to open the document: {err}"
        ))),
    }
}

fn page_content_rect(page_width: PdfPoints, page_height: PdfPoints, margins: PageMarginFractions) -> Option<PdfRect> {
    let bottom = page_height * margins.bottom;
    let top = page_height * (1.0 - margins.top);
    (bottom <= top).then(|| PdfRect::new(bottom, PdfPoints::ZERO, top, page_width))
}

fn unsupported_content_filter_warnings(config: &ExtractionConfig) -> Vec<ProcessingWarning> {
    let Some(filter) = config.content_filter.as_ref() else {
        return Vec::new();
    };
    let mut warnings = Vec::with_capacity(3);
    let warning = |message: &'static str| ProcessingWarning {
        source: std::borrow::Cow::Borrowed(PDFIUM_WARNING_SOURCE),
        message: std::borrow::Cow::Borrowed(message),
    };

    if filter.strip_repeating_text {
        warnings.push(warning(UNSUPPORTED_REPEATING_TEXT_MESSAGE));
    }
    if filter.include_footnotes {
        warnings.push(warning(UNSUPPORTED_FOOTNOTES_MESSAGE));
    }
    if filter.include_watermarks {
        warnings.push(warning(UNSUPPORTED_WATERMARKS_MESSAGE));
    }
    warnings
}

fn ensure_not_cancelled(cancel_token: Option<&crate::cancellation::CancellationToken>) -> Result<()> {
    if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(XbergError::Cancelled);
    }
    Ok(())
}

/// CPU-bound pdfium work, run inside `spawn_blocking` by [`extract`]. Builds the
/// `InternalDocument` directly rather than returning intermediate pieces --
/// there is no OCR/layout/table stage downstream of this backend (yet) that
/// would need them.
fn extract_blocking(
    content: &[u8],
    mime_type: &str,
    passwords: &[String],
    extract_metadata: bool,
    margins: PageMarginFractions,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<InternalDocument> {
    ensure_not_cancelled(cancel_token)?;
    let _operation_guard = PDFIUM_OPERATION_LOCK
        .lock()
        .map_err(|_| XbergError::parsing("pdfium operation lock was poisoned by an earlier extraction panic"))?;

    ensure_not_cancelled(cancel_token)?;
    extract_blocking_serialized(content, mime_type, passwords, extract_metadata, margins, cancel_token)
}

/// Concatenate every page's in-margin text, recording each page's byte range.
fn collect_page_text(
    document: &PdfDocument<'_>,
    page_count: u32,
    margins: PageMarginFractions,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<(String, Vec<PageBoundary>)> {
    let mut joined_text = String::new();
    let mut boundaries: Vec<PageBoundary> = Vec::with_capacity(page_count as usize);

    for (index, page) in document.pages().iter().enumerate() {
        ensure_not_cancelled(cancel_token)?;
        let page_number = (index + 1) as u32;
        let content_rect = page_content_rect(page.width(), page.height(), margins);
        let page_text = page
            .text()
            .map_err(|err| XbergError::parsing(format!("pdfium failed to read text on page {page_number}: {err}")))?;
        let content = content_rect.map_or_else(String::new, |rect| page_text.inside_rect(rect));

        let byte_start = joined_text.len();
        joined_text.push_str(&content);
        let byte_end = joined_text.len();
        joined_text.push_str("\n\n");

        boundaries.push(PageBoundary {
            byte_start,
            byte_end,
            page_number,
        });
    }

    Ok((joined_text, boundaries))
}

fn extract_blocking_serialized(
    content: &[u8],
    mime_type: &str,
    passwords: &[String],
    extract_metadata: bool,
    margins: PageMarginFractions,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<InternalDocument> {
    ensure_not_cancelled(cancel_token)?;
    let pdfium = Pdfium;
    let document = open_document(&pdfium, content, passwords)?;

    let page_count = document.pages().len();
    if page_count <= 0 {
        return Err(XbergError::parsing(
            "pdfium opened the document but it reports zero pages",
        ));
    }
    let page_count = page_count as u32;

    let (joined_text, boundaries) = collect_page_text(&document, page_count, margins, cancel_token)?;

    let mut doc = super::flat_pdf_document(&joined_text, mime_type, Some(&boundaries));

    let common = read_common_metadata(&document);
    let pdf_specific = PdfSpecificMetadata {
        producer: common.producer,
        is_encrypted: Some(!passwords.is_empty()),
        page_count: Some(page_count),
        ..Default::default()
    };

    if extract_metadata {
        doc.metadata = Metadata {
            title: common.title,
            subject: common.subject,
            authors: common.authors,
            keywords: common.keywords,
            created_at: common.created_at,
            modified_at: common.modified_at,
            created_by: common.created_by,
            pages: Some(PageStructure {
                total_count: page_count,
                unit_type: PageUnitType::Page,
                boundaries: Some(boundaries),
                pages: None,
            }),
            format: Some(crate::types::FormatMetadata::Pdf(pdf_specific)),
            ocr_used: false,
            ..Default::default()
        };
    }
    doc.metadata.additional.insert(
        std::borrow::Cow::Borrowed("extraction_method"),
        serde_json::Value::String(ExtractionMethod::Native.as_str().to_string()),
    );

    doc.processing_warnings.push(ProcessingWarning {
        source: std::borrow::Cow::Borrowed(PDFIUM_WARNING_SOURCE),
        message: std::borrow::Cow::Borrowed(
            "the pdfium backend currently extracts text, page count, and Info-dictionary \
             metadata only; tables, images, annotations, form fields, embedded files, and OCR \
             fallback are not implemented for this backend (issue #702). Content filtering is \
             limited to configured header/footer margins. Select the default native backend if \
             the document needs those capabilities.",
        ),
    });

    Ok(doc)
}

/// Extracts `content` via pdfium. Binds the library on first use (see
/// [`bind_once`]) and runs the pdfium calls themselves inside
/// `spawn_blocking`, since pdfium's C API is synchronous and CPU-bound and must
/// never block the async runtime.
pub(super) async fn extract(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
    bind_once()?;

    let passwords: Vec<String> = config
        .pdf_options
        .as_ref()
        .and_then(|options| options.passwords.clone())
        .unwrap_or_default();
    let extract_metadata = config
        .pdf_options
        .as_ref()
        .is_none_or(|options| options.extract_metadata);
    let margins = PageMarginFractions::from_extraction_config(Some(config));
    let unsupported_filter_warnings = unsupported_content_filter_warnings(config);
    let cancel_token = config.cancel_token.clone();
    let content = content.to_vec();
    let mime_type = mime_type.to_string();

    match tokio::task::spawn_blocking(move || {
        extract_blocking(
            &content,
            &mime_type,
            &passwords,
            extract_metadata,
            margins,
            cancel_token.as_ref(),
        )
    })
    .await
    {
        Ok(result) => result.map(|mut document| {
            document.processing_warnings.extend(unsupported_filter_warnings);
            document
        }),
        Err(join_error) => Err(XbergError::parsing(format!(
            "pdfium extraction task panicked: {join_error}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_stop_pdfium_page_work_when_cancelled() {
        let token = crate::cancellation::CancellationToken::new();
        token.cancel();

        let error = ensure_not_cancelled(Some(&token)).expect_err("cancelled pdfium work must stop");

        assert!(matches!(error, XbergError::Cancelled));
    }

    #[test]
    fn should_build_pdfium_content_rect_from_page_margin_fractions() {
        let rect = page_content_rect(
            PdfPoints::new(600.0),
            PdfPoints::new(1000.0),
            PageMarginFractions {
                top: 0.10,
                bottom: 0.20,
            },
        )
        .expect("non-overlapping margins should produce a content rectangle");

        assert_eq!(rect.left().value, 0.0);
        assert_eq!(rect.right().value, 600.0);
        assert_eq!(rect.bottom().value, 200.0);
        assert_eq!(rect.top().value, 900.0);
    }

    #[test]
    fn should_return_no_pdfium_content_rect_when_margins_overlap() {
        let rect = page_content_rect(
            PdfPoints::new(600.0),
            PdfPoints::new(1000.0),
            PageMarginFractions {
                top: 0.60,
                bottom: 0.60,
            },
        );

        assert!(rect.is_none());
    }

    #[test]
    fn should_use_full_pdfium_page_height_when_headers_and_footers_are_included() {
        let config = ExtractionConfig {
            pdf_options: Some(crate::core::config::PdfConfig {
                top_margin_fraction: Some(0.10),
                bottom_margin_fraction: Some(0.20),
                ..Default::default()
            }),
            content_filter: Some(crate::core::config::ContentFilterConfig {
                include_headers: true,
                include_footers: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let rect = page_content_rect(
            PdfPoints::new(600.0),
            PdfPoints::new(1000.0),
            PageMarginFractions::from_extraction_config(Some(&config)),
        )
        .expect("including headers and footers should retain the full page");

        assert_eq!(rect.bottom().value, 0.0);
        assert_eq!(rect.top().value, 1000.0);
    }

    #[test]
    fn should_not_warn_when_pdfium_content_filter_is_absent_or_only_uses_supported_controls() {
        assert!(unsupported_content_filter_warnings(&ExtractionConfig::default()).is_empty());

        let config = ExtractionConfig {
            content_filter: Some(crate::core::config::ContentFilterConfig {
                include_headers: true,
                include_footers: true,
                strip_repeating_text: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(unsupported_content_filter_warnings(&config).is_empty());
    }

    #[test]
    fn should_warn_when_explicit_default_content_filter_requests_repeating_text_removal() {
        let config = ExtractionConfig {
            content_filter: Some(crate::core::config::ContentFilterConfig::default()),
            ..Default::default()
        };

        let warnings = unsupported_content_filter_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].message, UNSUPPORTED_REPEATING_TEXT_MESSAGE);
    }

    #[test]
    fn should_warn_for_each_requested_unsupported_pdfium_content_filter() {
        let config = ExtractionConfig {
            content_filter: Some(crate::core::config::ContentFilterConfig {
                include_headers: true,
                include_footers: true,
                include_footnotes: true,
                strip_repeating_text: true,
                include_watermarks: true,
            }),
            ..Default::default()
        };

        let warnings = unsupported_content_filter_warnings(&config);
        assert_eq!(warnings.len(), 3);
        assert!(warnings.iter().all(|warning| warning.source == "pdf_backend_pdfium"));
        assert_eq!(
            warnings
                .iter()
                .map(|warning| warning.message.as_ref())
                .collect::<Vec<_>>(),
            [
                "the pdfium backend cannot apply ContentFilterConfig.strip_repeating_text; repeated text was preserved",
                concat!(
                    "the pdfium backend cannot apply ContentFilterConfig.include_footnotes; ",
                    "footnote inclusion follows Pdfium's plain-text extraction"
                ),
                concat!(
                    "the pdfium backend cannot apply ContentFilterConfig.include_watermarks; ",
                    "watermark inclusion follows Pdfium's plain-text extraction"
                ),
            ]
        );
    }
}
