//! Text extraction using pdf_oxide for cleaner word spacing.
//!
//! pdf_oxide parses PDF content streams directly (Tj/TJ operators) and uses
//! adaptive TJ-offset thresholds for word boundary detection, avoiding the
//! broken word spacing that pdfium's text page API produces for fonts with
//! broken CMap/ToUnicode tables.
//!
//! The current PDF file path is communicated via a thread-local to avoid
//! threading it through every function signature in the extraction pipeline.

use std::cell::RefCell;
use std::path::PathBuf;

#[cfg(feature = "pdf")]
use crate::pdf::hierarchy::SegmentData;

thread_local! {
    static CURRENT_PDF_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Set the current PDF file path for oxide text extraction.
/// Call this before entering the markdown pipeline.
pub(crate) fn set_current_pdf_path(path: Option<PathBuf>) {
    CURRENT_PDF_PATH.with(|cell| {
        *cell.borrow_mut() = path;
    });
}

/// Get the current PDF file path, if set.
pub(crate) fn current_pdf_path() -> Option<PathBuf> {
    CURRENT_PDF_PATH.with(|cell| cell.borrow().clone())
}

/// Extract text segments from a PDF file using pdf_oxide.
///
/// Returns segments per page (indexed by page number, 0-based).
/// Returns `None` if pdf_oxide fails to open or extract the document.
#[cfg(feature = "pdf")]
pub(crate) fn extract_segments_with_oxide(page_count: usize) -> Option<Vec<Vec<SegmentData>>> {
    let file_path = match current_pdf_path() {
        Some(p) => {
            tracing::debug!(path = %p.display(), "pdf_oxide: file path available");
            p
        }
        None => {
            tracing::debug!("pdf_oxide: no file path set (bytes-only extraction), skipping");
            return None;
        }
    };
    let mut pdf = match pdf_oxide::PdfDocument::open(&file_path) {
        Ok(pdf) => pdf,
        Err(e) => {
            tracing::debug!("pdf_oxide failed to open document: {e}");
            return None;
        }
    };

    let mut all_pages: Vec<Vec<SegmentData>> = Vec::with_capacity(page_count);

    for page_idx in 0..page_count {
        // Get page dimensions for coordinate conversion.
        // pdf_oxide spans use y=0 at top (screen coords).
        // Our pipeline uses PDF coords: y=0 at bottom.
        let page_height = pdf
            .get_page_media_box(page_idx)
            .ok()
            .map(|(_, lly, _, ury)| (ury - lly).abs())
            .unwrap_or(792.0); // Letter size fallback

        // Use default top-to-bottom ordering for the structure pipeline.
        // Column-aware reordering changes span sequence which breaks font-size
        // clustering for heading detection on single-column documents.
        let spans = match pdf.extract_spans(page_idx) {
            Ok(spans) => spans,
            Err(e) => {
                tracing::debug!(page = page_idx, "pdf_oxide extract_spans failed: {e}");
                all_pages.push(Vec::new());
                continue;
            }
        };

        let segments: Vec<SegmentData> = spans
            .into_iter()
            .filter(|span| {
                // Skip page furniture (headers/footers/watermarks)
                if span.artifact_type.is_some() {
                    return false;
                }
                !span.text.trim().is_empty()
            })
            .map(|span| {
                let is_bold = span.font_weight == pdf_oxide::layout::text_block::FontWeight::Bold;
                let bbox = &span.bbox;

                // Convert from screen coords (y=0 at top) to PDF coords (y=0 at bottom).
                // PDF baseline_y = page_height - screen_bottom
                let screen_bottom = bbox.y + bbox.height;
                let pdf_baseline_y = page_height - screen_bottom;
                let pdf_y = page_height - bbox.y - bbox.height;

                SegmentData {
                    text: span.text,
                    x: bbox.x,
                    y: pdf_y,
                    width: bbox.width,
                    height: bbox.height,
                    font_size: span.font_size,
                    is_bold,
                    is_italic: span.is_italic,
                    is_monospace: span.is_monospace,
                    baseline_y: pdf_baseline_y,
                }
            })
            .collect();

        all_pages.push(segments);
    }

    Some(all_pages)
}
