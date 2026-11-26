use crate::core::config::{ExtractionConfig, OcrConfig};
use std::borrow::Cow;

pub(in crate::extractors::pdf) fn resolve_ocr_config(config: &ExtractionConfig) -> Cow<'_, OcrConfig> {
    let mut resolved = config.ocr_or_default();
    // Forward the outer opt-out only after routing: materializing config.ocr changes the
    // near-empty native-text fallback decision. Backend options also preserve automatic
    // Tesseract defaults, which an explicit tesseract_config would otherwise replace. ~keep
    if !config.use_cache {
        let options = &mut resolved.to_mut().backend_options;
        let value = options.get_or_insert_with(|| serde_json::json!({}));
        if !value.is_object() {
            *value = serde_json::json!({});
        }
        value["use_cache"] = serde_json::Value::Bool(false);
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::TessdataModel;

    #[test]
    fn a_scanned_pdf_without_an_ocr_section_reads_with_the_top_level_set() {
        let config = ExtractionConfig {
            tessdata_model: Some(TessdataModel::Best),
            ..Default::default()
        };

        assert!(config.ocr.is_none());
        assert_eq!(resolve_ocr_config(&config).tessdata_model, Some(TessdataModel::Best));
        assert_eq!(
            resolve_ocr_config(&ExtractionConfig::default()).effective_tessdata_model(),
            TessdataModel::Fast
        );
    }

    #[test]
    fn an_ocr_section_without_a_set_inherits_and_one_with_a_set_wins() {
        let inherit = ExtractionConfig {
            tessdata_model: Some(TessdataModel::Best),
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };
        let own = ExtractionConfig {
            tessdata_model: Some(TessdataModel::Best),
            ocr: Some(OcrConfig {
                tessdata_model: Some(TessdataModel::Fast),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(resolve_ocr_config(&inherit).tessdata_model, Some(TessdataModel::Best));
        assert_eq!(resolve_ocr_config(&own).tessdata_model, Some(TessdataModel::Fast));
    }
}
