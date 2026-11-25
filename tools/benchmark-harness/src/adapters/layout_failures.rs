//! Rejection of layout-pipeline rows whose requested detector never ran.
//!
//! A failed layout pass leaves plausibly extracted text behind, so a benchmark that trusts the
//! row would score a fallback read as the requested detector's output.

use xberg::types::ExtractedDocument;

/// Warning sources that report a failed or skipped layout pass: `layout` from the PDF extractor
/// and `layout-ocr` from the image extractor's layout-region OCR.
const LAYOUT_WARNING_SOURCES: [&str; 2] = ["layout", "layout-ocr"];

/// Warning sources from the PDF and image table-recognition paths. Their "table model is not
/// available" warning means layout found tables but their structure was never recognized; CI
/// prefetches the TATR model, so it signals a broken setup that would misreport layout quality.
const TABLE_WARNING_SOURCES: [&str; 2] = ["pdf_tables", "image_tables"];

/// Text of `layout::table_model_unavailable_warning`.
const TABLE_MODEL_UNAVAILABLE_MARKER: &str = "table model is not available";

/// Emitted when the CPU provider recovered the layout pass; the layout was applied, so on its own
/// this clause leaves the row valid.
const RECOVERED_ON_CPU_MARKER: &str = "recovered on CPU";

/// Texts of clauses that report lost layout: a failed detector pass, and pages that could not be
/// prepared for it. xberg joins these onto the CPU-recovery clause in a single `layout` warning
/// (`combine_layout_warnings`), so a recovery marker must not exempt the whole message.
const LAYOUT_LOSS_MARKERS: [&str; 2] = ["layout detection failed", "could not be prepared for layout analysis"];

fn is_layout_failure(source: &str, message: &str) -> bool {
    if TABLE_WARNING_SOURCES.contains(&source) {
        return message.contains(TABLE_MODEL_UNAVAILABLE_MARKER);
    }
    LAYOUT_WARNING_SOURCES.contains(&source)
        && (LAYOUT_LOSS_MARKERS.iter().any(|marker| message.contains(marker))
            || !message.contains(RECOVERED_ON_CPU_MARKER))
}

/// The layout failure a document reports, if its extraction continued without layout hints.
pub(crate) fn layout_failure_in_document(document: &ExtractedDocument) -> Option<String> {
    document
        .processing_warnings
        .iter()
        .find(|warning| is_layout_failure(&warning.source, &warning.message))
        .map(|warning| message(&warning.message))
}

/// The layout failure an extracted item reports, if its document continued without layout
/// hints. The CLI emits xberg's own `processing_warnings`, so a request for a specific local
/// detector that failed is visible here instead of being scored as that detector's output. ~keep
pub(crate) fn layout_failure_in_item(item: &serde_json::Value) -> Option<String> {
    let warnings = item.get("processing_warnings")?.as_array()?;
    let warning = warnings.iter().find(|warning| {
        is_layout_failure(
            warning_text(warning, "source").unwrap_or_default(),
            warning_text(warning, "message").unwrap_or_default(),
        )
    })?;
    Some(message(
        warning_text(warning, "message").unwrap_or("unknown layout error"),
    ))
}

fn warning_text<'a>(warning: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    warning.get(key).and_then(serde_json::Value::as_str)
}

fn message(reason: &str) -> String {
    format!("layout detection failed, so this document was extracted without the requested detector: {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use xberg::types::ProcessingWarning;

    fn warning(source: &'static str, message: &'static str) -> ProcessingWarning {
        ProcessingWarning {
            source: source.into(),
            message: message.into(),
        }
    }

    fn item(warnings: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "content": "text", "processing_warnings": warnings })
    }

    const REASON: &str = "layout detection failed (model not found); document extracted without layout hints";

    /// Negative control: a failed layout pass must be visible with its reason, not silently
    /// scored as the requested detector's output. ~keep
    #[test]
    fn a_layout_failure_is_reported_with_its_reason_in_both_shapes() {
        let mut document = ExtractedDocument::default();
        document.processing_warnings.push(warning("layout", REASON));
        let from_document = layout_failure_in_document(&document).expect("document layout failure detected");
        let from_item = layout_failure_in_item(&item(serde_json::json!([{ "source": "layout", "message": REASON }])))
            .expect("item layout failure detected");
        assert_eq!(from_document, from_item);
        assert!(from_document.contains("model not found"), "{from_document}");
        assert!(
            from_document.contains("without the requested detector"),
            "{from_document}"
        );
    }

    #[test]
    fn warnings_from_other_sources_leave_the_row_usable() {
        let mut document = ExtractedDocument::default();
        document
            .processing_warnings
            .push(warning("ocr", "Page 1 OCR used 8 source-image strips."));
        assert_eq!(layout_failure_in_document(&document), None);
        assert_eq!(
            layout_failure_in_item(&item(serde_json::json!([{ "source": "ocr", "message": "strips" }]))),
            None
        );
        assert_eq!(layout_failure_in_item(&serde_json::json!({ "content": "text" })), None);
    }

    #[test]
    fn an_image_layout_ocr_failure_is_reported_in_both_shapes() {
        let reason = "Layout-region OCR failed after whole-image OCR succeeded; retained whole-image output";
        let mut document = ExtractedDocument::default();
        document.processing_warnings.push(warning("layout-ocr", reason));
        assert!(layout_failure_in_document(&document).is_some());
        assert!(
            layout_failure_in_item(&item(
                serde_json::json!([{ "source": "layout-ocr", "message": reason }])
            ))
            .is_some()
        );
    }

    /// Negative control: the exact text xberg emits when a CPU recovery and unrenderable pages
    /// coincide; the recovered clause must not hide the lost pages. ~keep
    #[test]
    fn a_cpu_recovery_combined_with_a_page_loss_is_still_a_failure() {
        let combined = "automatic layout inference failed (CoreML ExecuteKernel failed); recovered on CPU; \
                        layout: 1 page(s) could not be prepared for layout analysis and were treated as empty: \
                        page 2 failed to render: boom";
        let mut document = ExtractedDocument::default();
        document.processing_warnings.push(warning("layout", combined));
        let from_document = layout_failure_in_document(&document).expect("page loss must be detected");
        assert!(from_document.contains("could not be prepared"), "{from_document}");
        assert!(
            layout_failure_in_item(&item(serde_json::json!([{ "source": "layout", "message": combined }]))).is_some()
        );
    }

    #[test]
    fn a_cpu_recovery_combined_with_a_detector_failure_is_still_a_failure() {
        let combined = "automatic layout inference failed (provider error); recovered on CPU; \
                        layout detection failed (model not found); document extracted without layout hints";
        assert!(is_layout_failure("layout", combined));
    }

    const TABLE_MODEL_WARNING: &str = "layout detection found table regions but the TATR table model is not \
                                       available; table structure recognition was skipped; check that the \
                                       model is downloaded";

    #[test]
    fn a_skipped_table_model_is_a_failure_from_both_table_paths() {
        for source in ["pdf_tables", "image_tables"] {
            let mut document = ExtractedDocument::default();
            document.processing_warnings.push(warning(source, TABLE_MODEL_WARNING));
            let from_document = layout_failure_in_document(&document).expect("document table failure detected");
            let from_item = layout_failure_in_item(&item(
                serde_json::json!([{ "source": source, "message": TABLE_MODEL_WARNING }]),
            ))
            .expect("item table failure detected");
            assert_eq!(from_document, from_item);
            assert!(from_document.contains("TATR"), "{from_document}");
        }
    }

    #[test]
    fn other_table_warnings_and_sources_are_not_layout_failures() {
        assert!(!is_layout_failure("pdf_tables", "table 2 had no detectable cells"));
        assert!(!is_layout_failure("ocr", TABLE_MODEL_WARNING));
    }

    #[test]
    fn a_cpu_recovered_layout_pass_is_not_a_failure() {
        let reason = "automatic layout inference failed (provider error); recovered on CPU";
        let mut document = ExtractedDocument::default();
        document.processing_warnings.push(warning("layout", reason));
        assert_eq!(layout_failure_in_document(&document), None);
        assert_eq!(
            layout_failure_in_item(&item(serde_json::json!([{ "source": "layout", "message": reason }]))),
            None
        );
    }
}
