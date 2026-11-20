//! Rejection of layout-pipeline rows whose requested detector never ran.
//!
//! A failed layout pass leaves plausibly extracted text behind, so a benchmark that trusts the
//! row would score a fallback read as the requested detector's output.

use xberg::types::ExtractedDocument;

/// The layout failure a document reports, if its extraction continued without layout hints.
pub(crate) fn layout_failure_in_document(document: &ExtractedDocument) -> Option<String> {
    document
        .processing_warnings
        .iter()
        .find(|warning| warning.source == "layout")
        .map(|warning| message(&warning.message))
}

/// The layout failure an extracted item reports, if its document continued without layout
/// hints. The CLI emits xberg's own `processing_warnings`, so a request for a specific local
/// detector that failed is visible here instead of being scored as that detector's output. ~keep
pub(crate) fn layout_failure_in_item(item: &serde_json::Value) -> Option<String> {
    let warnings = item.get("processing_warnings")?.as_array()?;
    let warning = warnings
        .iter()
        .find(|warning| warning.get("source").and_then(serde_json::Value::as_str) == Some("layout"))?;
    Some(message(
        warning
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown layout error"),
    ))
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
}
