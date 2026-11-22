//! Real extraction engine for `PdfBackend::Pdfium` (issue #702).
//!
//! # Scope
//!
//! Deliberately smaller than [`super::PdfExtractor::extract_core_native`]: page
//! count, per-page plain text (the whole-page text API, filtered by character
//! boxes when page margins are configured), and Info-dictionary metadata
//! (Title/Author/Subject/Keywords/Creator/Producer/CreationDate/ModificationDate)
//! plus version, encryption and first-page size. `xberg-pdfium-render`'s public
//! API does not currently give this crate anything to build the rest of the
//! native contract from -- no table detection, no diagram/DOT recovery, no
//! layout-detection integration, no annotation-to-`PdfAnnotation` mapping, no
//! AcroForm/XFA field values, no embedded-file extraction, and no OCR fallback
//! path. None of that runs here. Instead of one blanket notice, a
//! `ProcessingWarning` is emitted for each option the caller actually set that
//! this engine cannot honour (see `warnings`), so a caller diffing pdfium output
//! against `xberg_native_pdf` output is never left assuming parity silently.
//!
//! The output is tagged `pdf_backend = "pdfium"` in `Metadata::additional`.
//! `extraction_method` stays `native`: that enum records whether text came from
//! the text layer or from OCR, which is true here, and it has no engine axis.
//!
//! Page count, extraction time limit and `max_content_size` are enforced inside
//! the page loop, as the native path does.
//!
//! # The pdfium library binding is process-global and happens once
//!
//! `pdfium_render::Pdfium` stores its resolved FFI bindings in a single
//! process-wide `once_cell::sync::OnceCell` internal to the fork. Binding twice
//! in one process is a logic error: `Pdfium::new`/`new_with_config` assert the
//! cell is still empty (a second call panics), and the `bind_*` functions return
//! `PdfiumError::PdfiumLibraryBindingsAlreadyInitialized` on a second call even
//! when the first one succeeded. [`bind_once`] is therefore the *only* call site
//! in this engine that attempts a bind, and it caches the outcome -- success or
//! failure -- in its own `OnceCell`. A host application that bound pdfium before
//! this engine ran is not an error: the existing bindings are reused.
//!
//! # Library discovery is a runtime concern, not a build-time one
//!
//! `PDFIUM_STATIC_LIB_PATH` / `PDFIUM_DYNAMIC_LIB_PATH` only steer
//! `crates/xberg-pdfium-render/build.rs`'s linker flags at *build* time. In a
//! build without `PDFIUM_STATIC_LIB_PATH` every `FPDF_*` symbol is resolved at
//! *runtime* via `libloading`, which `dlopen`s the platform library name
//! (`libpdfium.so` / `.dylib` / `pdfium.dll`) from the system library search
//! path. [`bind_once`] additionally checks `PDFIUM_DYNAMIC_LIB_PATH` at runtime
//! and, if set, tries loading the platform-named library from that directory
//! first. A build with `PDFIUM_STATIC_LIB_PATH` links pdfium into the binary and
//! binds to it directly; there is no library to locate. If nothing binds,
//! extraction fails loudly with `XbergError::MissingDependency` carrying every
//! error that was hit -- never a silent empty document.
//!
//! The dlopen and symbol probe run on the blocking pool, not the async executor.

mod page_text;
mod warnings;

use once_cell::sync::OnceCell;
use pdfium_render::prelude::*;
use std::any::Any;
use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::Duration;

use crate::Result;
use crate::cancellation::{CancellationToken, check_cancelled};
use crate::core::config::ExtractionConfig;
use crate::error::XbergError;
use crate::extractors::security::{SecurityBudget, enforce_page_count};
use crate::pdf::metadata::PdfMetadata as PdfSpecificMetadata;
use crate::pdf::native::metadata::{parse_authors, parse_keywords, parse_pdf_date};
use crate::pdf::native::text::PageMarginFractions;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractionMethod, Metadata, PageBoundary, PageStructure, PageUnitType, ProcessingWarning};

/// Value of the `pdf_backend` metadata key on every document this engine produces.
const PDF_BACKEND_NAME: &str = "pdfium";
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(5);

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

/// Both load errors, when a directory was tried first, and the guidance for the build in use.
fn bind_failure_message(statically_linked: bool, directory_attempt: Option<(&str, &str)>, final_error: &str) -> String {
    let mut message = String::new();
    if statically_linked {
        message.push_str(&format!(
            "could not bind the statically linked pdfium library: {final_error}. This build was \
             compiled with PDFIUM_STATIC_LIB_PATH, so PDFIUM_DYNAMIC_LIB_PATH and the system \
             library search path are not consulted; rebuild with a valid PDFIUM_STATIC_LIB_PATH, \
             or without it to load a shared library at runtime."
        ));
        return message;
    }
    if let Some((directory, error)) = directory_attempt {
        message.push_str(&format!(
            "could not load the pdfium shared library from PDFIUM_DYNAMIC_LIB_PATH={directory}: {error}; "
        ));
    }
    message.push_str(&format!(
        "could not load the pdfium shared library from the system library search path: {final_error}. \
         Set PDFIUM_DYNAMIC_LIB_PATH to a directory containing the platform pdfium library \
         (libpdfium.so / .dylib / pdfium.dll), install pdfium on the system library search path, or \
         rebuild with PDFIUM_STATIC_LIB_PATH to link pdfium statically."
    ));
    message
}

fn is_already_bound(error: &PdfiumError) -> bool {
    matches!(error, PdfiumError::PdfiumLibraryBindingsAlreadyInitialized)
}

fn bind_process_global() -> std::result::Result<(), String> {
    let directory = std::env::var("PDFIUM_DYNAMIC_LIB_PATH")
        .ok()
        .filter(|directory| !directory.is_empty());
    let statically_linked = Pdfium::is_statically_linked();

    let mut directory_failure: Option<(String, String)> = None;
    let mut bindings = None;
    if let Some(directory) = directory.as_deref().filter(|_| !statically_linked) {
        match Pdfium::bind_to_library_in_dir(Path::new(directory)) {
            Ok(bound) => bindings = Some(bound),
            Err(error) if is_already_bound(&error) => return Ok(()),
            Err(error) => directory_failure = Some((directory.to_string(), error.to_string())),
        }
    }

    let bindings = match bindings {
        Some(bound) => bound,
        None => match Pdfium::bind_to_linked_or_system_library() {
            Ok(bound) => bound,
            Err(error) if is_already_bound(&error) => return Ok(()),
            Err(error) => {
                let directory_attempt = directory_failure
                    .as_ref()
                    .map(|(directory, error)| (directory.as_str(), error.as_str()));
                return Err(bind_failure_message(
                    statically_linked,
                    directory_attempt,
                    &error.to_string(),
                ));
            }
        },
    };

    // `Pdfium::new` asserts that no bindings exist yet; a host binding concurrently trips it.
    catch_unwind(AssertUnwindSafe(|| {
        Pdfium::new(bindings);
    }))
    .map_err(|payload| format!("pdfium initialisation panicked: {}", panic_message(payload.as_ref())))
}

/// Binds the pdfium FFI library exactly once per process. See the module doc
/// comment for why this must not be attempted more than once. Blocking: call it
/// from the blocking pool.
fn bind_once() -> Result<()> {
    BIND_OUTCOME
        .get_or_init(bind_process_global)
        .clone()
        .map_err(|message| XbergError::MissingDependency(format!("pdfium: {message}")))
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string())
}

/// Takes `lock`, polling so that a cancelled extraction does not wait behind another one's
/// pages, and recovering from a poisoned lock: the guarded value is `()`, so poisoning records
/// only that an earlier holder panicked, which [`run_guarded`] already reported as an error.
fn acquire_lock<'a>(lock: &'a Mutex<()>, cancel_token: Option<&CancellationToken>) -> Result<MutexGuard<'a, ()>> {
    loop {
        check_cancelled(cancel_token)?;
        match lock.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Ok(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => std::thread::sleep(LOCK_POLL_INTERVAL),
        }
    }
}

/// Runs `work`, turning a panic in the pdfium wrapper into an error instead of unwinding
/// through the held lock.
fn run_guarded<T>(work: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|payload| {
        Err(XbergError::parsing(format!(
            "pdfium panicked during extraction: {}",
            panic_message(payload.as_ref())
        )))
    })
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

/// Reads the eight Info-dictionary tags pdfium exposes. Dates are normalised to ISO 8601 the
/// way the native engine does; authors and keywords are split with the same parsers.
fn read_common_metadata(document: &PdfDocument<'_>) -> CommonMetadata {
    let metadata = document.metadata();
    let get = |tag: PdfDocumentMetadataTagType| -> Option<String> {
        metadata.get(tag).map(|t| t.value().to_string()).and_then(non_empty)
    };

    CommonMetadata {
        title: get(PdfDocumentMetadataTagType::Title),
        subject: get(PdfDocumentMetadataTagType::Subject),
        authors: get(PdfDocumentMetadataTagType::Author)
            .map(|author| parse_authors(&author))
            .filter(|authors| !authors.is_empty()),
        keywords: get(PdfDocumentMetadataTagType::Keywords)
            .map(|keywords| parse_keywords(&keywords))
            .filter(|keywords| !keywords.is_empty()),
        created_at: get(PdfDocumentMetadataTagType::CreationDate).map(|date| parse_pdf_date(&date)),
        modified_at: get(PdfDocumentMetadataTagType::ModificationDate).map(|date| parse_pdf_date(&date)),
        created_by: get(PdfDocumentMetadataTagType::Creator),
        producer: get(PdfDocumentMetadataTagType::Producer),
    }
}

fn format_pdf_version(version: PdfDocumentVersion) -> Option<String> {
    match version {
        PdfDocumentVersion::Unset => None,
        PdfDocumentVersion::Pdf1_0 => Some("1.0".to_string()),
        PdfDocumentVersion::Pdf1_1 => Some("1.1".to_string()),
        PdfDocumentVersion::Pdf1_2 => Some("1.2".to_string()),
        PdfDocumentVersion::Pdf1_3 => Some("1.3".to_string()),
        PdfDocumentVersion::Pdf1_4 => Some("1.4".to_string()),
        PdfDocumentVersion::Pdf1_5 => Some("1.5".to_string()),
        PdfDocumentVersion::Pdf1_6 => Some("1.6".to_string()),
        PdfDocumentVersion::Pdf1_7 => Some("1.7".to_string()),
        PdfDocumentVersion::Pdf2_0 => Some("2.0".to_string()),
        PdfDocumentVersion::Other(value) if value >= 10 => Some(format!("{}.{}", value / 10, value % 10)),
        PdfDocumentVersion::Other(value) => Some(value.to_string()),
    }
}

/// `None` when pdfium could not report a revision: unknown is not "unencrypted".
fn revision_is_encrypted(revision: std::result::Result<PdfSecurityHandlerRevision, PdfiumError>) -> Option<bool> {
    revision
        .ok()
        .map(|revision| revision != PdfSecurityHandlerRevision::Unprotected)
}

/// Message for a document pdfium refused for want of a valid password; matches the native engine.
fn password_failure_message(passwords_supplied: bool) -> &'static str {
    if passwords_supplied {
        "PDF is encrypted and none of the supplied passwords authenticated"
    } else {
        "PDF is encrypted and requires a password; set pdf_options.passwords"
    }
}

fn is_password_error(error: &PdfiumError) -> bool {
    matches!(
        error,
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError)
    )
}

/// Opens `content` with pdfium, trying each configured password in turn if the
/// document is encrypted and no password (or the wrong one) was supplied.
/// Mirrors the intent of `xberg_native_pdf`'s multi-password support without pulling
/// in its retry plumbing -- pdfium only accepts one password per open attempt.
fn open_document<'a>(pdfium: &'a Pdfium, content: &'a [u8], passwords: &[String]) -> Result<PdfDocument<'a>> {
    match pdfium.load_pdf_from_byte_slice(content, None) {
        Ok(document) => Ok(document),
        Err(error) if is_password_error(&error) => {
            for password in passwords {
                if let Ok(document) = pdfium.load_pdf_from_byte_slice(content, Some(password.as_str())) {
                    return Ok(document);
                }
            }
            Err(XbergError::parsing(password_failure_message(!passwords.is_empty())))
        }
        Err(err) => Err(XbergError::parsing(format!(
            "pdfium failed to open the document: {err}"
        ))),
    }
}

/// pdfium reports the page count as a signed integer; a negative one is an error, zero pages is an
/// empty document as in the native engine.
fn checked_page_count(raw: i32) -> Result<u32> {
    u32::try_from(raw).map_err(|_| XbergError::parsing(format!("pdfium reported an invalid page count ({raw})")))
}

/// Everything one extraction needs besides the document bytes. Built on the calling thread so the
/// blocking closure owns plain data.
struct EngineOptions {
    passwords: Vec<String>,
    extract_metadata: bool,
    margins: PageMarginFractions,
    max_pages: Option<usize>,
    budget: SecurityBudget,
    ocr_disabled: bool,
    option_warnings: Vec<ProcessingWarning>,
    cancel_token: Option<CancellationToken>,
}

impl EngineOptions {
    fn from_config(config: &ExtractionConfig) -> Self {
        Self {
            passwords: config
                .pdf_options
                .as_ref()
                .and_then(|options| options.passwords.clone())
                .unwrap_or_default(),
            extract_metadata: config
                .pdf_options
                .as_ref()
                .is_none_or(|options| options.extract_metadata),
            margins: PageMarginFractions::from_extraction_config(Some(config)),
            max_pages: config.security_limits.as_ref().and_then(|limits| limits.max_pages),
            budget: SecurityBudget::from_config(config),
            ocr_disabled: config.effective_disable_ocr(),
            option_warnings: warnings::unsupported_option_warnings(config),
            cancel_token: config.cancel_token.clone(),
        }
    }
}

/// CPU-bound pdfium work, run inside `spawn_blocking` by [`extract`]. Builds the
/// `InternalDocument` directly rather than returning intermediate pieces --
/// there is no OCR/layout/table stage downstream of this backend (yet) that
/// would need them.
fn extract_blocking(content: &[u8], mime_type: &str, options: EngineOptions) -> Result<InternalDocument> {
    let cancel_token = options.cancel_token.clone();
    let _operation_guard = acquire_lock(&PDFIUM_OPERATION_LOCK, cancel_token.as_ref())?;
    run_guarded(|| extract_blocking_serialized(content, mime_type, options))
}

struct PageTexts {
    text: String,
    boundaries: Vec<PageBoundary>,
    blank_pages: Vec<u32>,
    margin_unavailable_pages: Vec<u32>,
    first_unread_page: Option<u32>,
}

/// Text of one page: the whole-page text API when no margin applies, otherwise the characters
/// inside the content band. `None` for the band means it could not be placed on this page.
fn read_page_text(
    page: &PdfPage<'_>,
    margins: PageMarginFractions,
) -> std::result::Result<(String, bool), PdfiumError> {
    let page_text = page.text()?;
    if page_text::margins_are_zero(margins) {
        return Ok((page_text.whole_page(), true));
    }
    Ok(match page_text::page_text_within_margins(page, &page_text, margins) {
        Some(filtered) => (filtered, true),
        None => (page_text.whole_page(), false),
    })
}

/// Concatenate every page's text, recording each page's byte range. Stops at the first page the
/// extraction time limit left unread and enforces `max_content_size` as text accumulates.
fn collect_page_text(document: &PdfDocument<'_>, page_count: u32, options: &mut EngineOptions) -> Result<PageTexts> {
    let mut texts = PageTexts {
        text: String::new(),
        boundaries: Vec::with_capacity(page_count as usize),
        blank_pages: Vec::new(),
        margin_unavailable_pages: Vec::new(),
        first_unread_page: None,
    };
    let cancel_token = options.cancel_token.clone();
    let _page_loop = cancel_token.as_ref().map(CancellationToken::enter_page_loop);

    for (index, page) in document.pages().iter().enumerate() {
        check_cancelled(cancel_token.as_ref())?;
        let page_number = (index + 1) as u32;
        if let Some(token) = cancel_token.as_ref().filter(|token| token.stop_requested()) {
            token.note_unread_page(page_number);
            texts.first_unread_page = Some(page_number);
            break;
        }

        let (raw, band_applied) = read_page_text(&page, options.margins)
            .map_err(|err| XbergError::parsing(format!("pdfium failed to read text on page {page_number}: {err}")))?;
        if !band_applied {
            texts.margin_unavailable_pages.push(page_number);
        }
        let content = page_text::clean_page_text(raw);
        options.budget.account_text(content.len())?;
        if content.trim().is_empty() {
            texts.blank_pages.push(page_number);
        }

        let byte_start = texts.text.len();
        texts.text.push_str(&content);
        let byte_end = texts.text.len();
        texts.text.push_str("\n\n");

        texts.boundaries.push(PageBoundary {
            byte_start,
            byte_end,
            page_number,
        });
    }

    Ok(texts)
}

fn first_page_size(document: &PdfDocument<'_>) -> (Option<i64>, Option<i64>) {
    match document.pages().page_size(0) {
        Ok(rect) => (
            Some(rect.width().value.round() as i64),
            Some(rect.height().value.round() as i64),
        ),
        Err(_) => (None, None),
    }
}

fn extract_blocking_serialized(
    content: &[u8],
    mime_type: &str,
    mut options: EngineOptions,
) -> Result<InternalDocument> {
    let pdfium = Pdfium;
    let document = open_document(&pdfium, content, &options.passwords)?;

    let page_count = checked_page_count(document.pages().len() as i32)?;
    enforce_page_count(page_count as usize, options.max_pages)?;

    let texts = collect_page_text(&document, page_count, &mut options)?;

    let mut doc = super::flat_pdf_document(&texts.text, mime_type, Some(&texts.boundaries));

    let common = read_common_metadata(&document);
    let (width, height) = if page_count > 0 {
        first_page_size(&document)
    } else {
        (None, None)
    };
    let pdf_specific = PdfSpecificMetadata {
        pdf_version: format_pdf_version(document.version()),
        producer: common.producer,
        is_encrypted: revision_is_encrypted(document.permissions().security_handler_revision()),
        width,
        height,
        page_count: Some(page_count),
        ..Default::default()
    };

    if options.extract_metadata {
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
                boundaries: Some(texts.boundaries),
                pages: None,
            }),
            format: Some(crate::types::FormatMetadata::Pdf(pdf_specific)),
            ocr_used: false,
            ..Default::default()
        };
    }
    doc.metadata.additional.insert(
        Cow::Borrowed("extraction_method"),
        serde_json::Value::String(ExtractionMethod::Native.as_str().to_string()),
    );
    doc.metadata.additional.insert(
        Cow::Borrowed("pdf_backend"),
        serde_json::Value::String(PDF_BACKEND_NAME.to_string()),
    );

    doc.processing_warnings.append(&mut options.option_warnings);
    if page_count == 0 {
        doc.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed(warnings::WARNING_SOURCE),
            message: Cow::Borrowed("pdfium opened the document but it reports zero pages"),
        });
    }
    doc.processing_warnings.extend(warnings::missing_ocr_fallback_warning(
        &texts.blank_pages,
        options.ocr_disabled,
    ));
    if !texts.margin_unavailable_pages.is_empty() {
        doc.processing_warnings
            .push(warnings::margin_filter_unavailable_warning(
                &texts.margin_unavailable_pages,
            ));
    }
    if let Some(first_unread) = texts.first_unread_page {
        doc.processing_warnings
            .push(warnings::time_limit_warning(first_unread, page_count));
    }

    Ok(doc)
}

/// Extracts `content` via pdfium. Binds the library on first use (see
/// [`bind_once`]) and runs the pdfium calls themselves -- the bind included --
/// inside `spawn_blocking`, since pdfium's C API is synchronous and CPU-bound
/// and must never block the async runtime.
pub(super) async fn extract(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
    check_cancelled(config.cancel_token.as_ref())?;
    let options = EngineOptions::from_config(config);
    let content = content.to_vec();
    let mime_type = mime_type.to_string();

    match tokio::task::spawn_blocking(move || {
        bind_once()?;
        extract_blocking(&content, &mime_type, options)
    })
    .await
    {
        Ok(result) => result,
        Err(join_error) => Err(XbergError::parsing(format!(
            "pdfium extraction task panicked: {join_error}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn options_for(config: &ExtractionConfig) -> EngineOptions {
        EngineOptions::from_config(config)
    }

    #[test]
    fn should_stop_pdfium_extraction_before_opening_the_document_when_cancelled() {
        let token = CancellationToken::new();
        token.cancel();
        let config = ExtractionConfig {
            cancel_token: Some(token),
            ..Default::default()
        };

        let result = extract_blocking(b"", "application/pdf", options_for(&config));
        assert!(matches!(result, Err(XbergError::Cancelled)));
    }

    #[test]
    fn should_report_both_load_errors_when_a_library_directory_was_tried() {
        let message = bind_failure_message(false, Some(("/opt/pdfium", "no such file")), "libpdfium not found");
        assert!(
            message.contains("PDFIUM_DYNAMIC_LIB_PATH=/opt/pdfium: no such file"),
            "{message}"
        );
        assert!(
            message.contains("system library search path: libpdfium not found"),
            "{message}"
        );
    }

    #[test]
    fn should_report_one_load_error_without_a_library_directory() {
        let message = bind_failure_message(false, None, "libpdfium not found");
        assert!(!message.contains("PDFIUM_DYNAMIC_LIB_PATH="), "{message}");
        assert!(message.contains("libpdfium not found"));
        assert!(message.contains("PDFIUM_STATIC_LIB_PATH"));
    }

    #[test]
    fn should_not_send_a_static_build_to_runtime_library_paths() {
        let message = bind_failure_message(true, Some(("/ignored", "ignored")), "symbol missing");
        assert!(message.contains("statically linked"), "{message}");
        assert!(message.contains("symbol missing"));
        assert!(!message.contains("/ignored"), "{message}");
        assert!(message.contains("rebuild"));
    }

    #[test]
    fn should_treat_only_already_initialized_as_an_existing_binding() {
        assert!(is_already_bound(&PdfiumError::PdfiumLibraryBindingsAlreadyInitialized));
        assert!(!is_already_bound(&PdfiumError::PageIndexOutOfBounds));
    }

    #[test]
    fn should_take_a_free_lock() {
        let lock = Mutex::new(());
        assert!(acquire_lock(&lock, None).is_ok());
    }

    #[test]
    fn should_recover_a_lock_poisoned_by_an_earlier_panic() {
        let lock = Arc::new(Mutex::new(()));
        let poisoner = Arc::clone(&lock);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock().unwrap();
            panic!("simulated pdfium wrapper panic");
        })
        .join();
        assert!(lock.is_poisoned(), "the control must actually poison the lock");

        assert!(acquire_lock(&lock, None).is_ok());
    }

    #[test]
    fn should_stop_waiting_for_a_held_lock_when_cancelled() {
        let lock = Mutex::new(());
        let token = CancellationToken::new();
        let _held = lock.lock().unwrap();
        let canceller = token.clone();

        let result = std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(30));
                canceller.cancel();
            });
            acquire_lock(&lock, Some(&token)).map(|_| ())
        });

        assert!(matches!(result, Err(XbergError::Cancelled)));
    }

    #[test]
    fn should_map_a_panic_to_a_parsing_error_with_its_message() {
        let result: Result<()> = run_guarded(|| panic!("assertion failed: handle"));
        let error = result.expect_err("a panic must become an error");
        assert!(matches!(error, XbergError::Parsing { .. }));
        assert!(error.to_string().contains("assertion failed: handle"), "{error}");
    }

    #[test]
    fn should_pass_a_non_panicking_result_through() {
        assert_eq!(run_guarded(|| Ok(7)).unwrap(), 7);
    }

    #[test]
    fn should_extract_a_message_from_each_panic_payload_kind() {
        assert_eq!(panic_message(&"static message"), "static message");
        assert_eq!(panic_message(&String::from("owned message")), "owned message");
        assert_eq!(panic_message(&42_u32), "unknown panic");
    }

    #[test]
    fn should_format_every_pdf_version() {
        let cases = [
            (PdfDocumentVersion::Unset, None),
            (PdfDocumentVersion::Pdf1_0, Some("1.0")),
            (PdfDocumentVersion::Pdf1_4, Some("1.4")),
            (PdfDocumentVersion::Pdf1_7, Some("1.7")),
            (PdfDocumentVersion::Pdf2_0, Some("2.0")),
            (PdfDocumentVersion::Other(18), Some("1.8")),
            (PdfDocumentVersion::Other(9), Some("9")),
        ];
        for (version, expected) in cases {
            assert_eq!(format_pdf_version(version).as_deref(), expected);
        }
    }

    #[test]
    fn should_derive_encryption_from_the_security_handler_revision() {
        let cases = [
            (Ok(PdfSecurityHandlerRevision::Unprotected), Some(false)),
            (Ok(PdfSecurityHandlerRevision::Revision2), Some(true)),
            (Ok(PdfSecurityHandlerRevision::Revision3), Some(true)),
            (Ok(PdfSecurityHandlerRevision::Revision4), Some(true)),
            (Err(PdfiumError::UnknownPdfSecurityHandlerRevision), None),
        ];
        for (revision, expected) in cases {
            assert_eq!(revision_is_encrypted(revision), expected);
        }
    }

    #[test]
    fn should_name_the_password_requirement_like_the_native_engine() {
        assert_eq!(
            password_failure_message(false),
            "PDF is encrypted and requires a password; set pdf_options.passwords"
        );
        assert_eq!(
            password_failure_message(true),
            "PDF is encrypted and none of the supplied passwords authenticated"
        );
    }

    #[test]
    fn should_recognise_only_pdfiums_password_error_as_a_password_failure() {
        assert!(is_password_error(&PdfiumError::PdfiumLibraryInternalError(
            PdfiumInternalError::PasswordError
        )));
        assert!(!is_password_error(&PdfiumError::PdfiumLibraryInternalError(
            PdfiumInternalError::FormatError
        )));
    }

    #[test]
    fn should_accept_zero_pages_and_reject_a_negative_page_count() {
        assert_eq!(checked_page_count(0).unwrap(), 0);
        assert_eq!(checked_page_count(12).unwrap(), 12);
        assert!(checked_page_count(-1).is_err());
    }

    #[test]
    fn should_read_limits_and_options_from_the_config() {
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(3),
                max_content_size: 10,
                ..Default::default()
            }),
            disable_ocr: true,
            pdf_options: Some(crate::core::config::PdfConfig {
                passwords: Some(vec!["secret".to_string()]),
                extract_metadata: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut options = options_for(&config);

        assert_eq!(options.max_pages, Some(3));
        assert_eq!(options.passwords, ["secret"]);
        assert!(!options.extract_metadata);
        assert!(options.ocr_disabled);
        assert!(options.budget.account_text(10).is_ok());
        assert!(
            options.budget.account_text(1).is_err(),
            "text past max_content_size must exhaust the budget"
        );
        assert!(enforce_page_count(4, options.max_pages).is_err());
        assert!(enforce_page_count(3, options.max_pages).is_ok());
    }

    #[test]
    fn should_default_to_reading_metadata_with_no_page_cap() {
        let options = options_for(&ExtractionConfig::default());
        assert!(options.extract_metadata);
        assert_eq!(options.max_pages, None);
        assert!(options.passwords.is_empty());
        assert!(!options.ocr_disabled);
    }
}
