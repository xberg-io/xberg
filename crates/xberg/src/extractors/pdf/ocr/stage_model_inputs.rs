use crate::core::config::{OcrConfig, OcrPipelineStage};
#[cfg(any(test, all(feature = "ocr", not(target_arch = "wasm32"))))]
use crate::types::TesseractConfig;
#[cfg(any(test, all(feature = "ocr", not(target_arch = "wasm32"))))]
use std::path::Path;

pub(crate) struct StageModelInputs<'a> {
    parent: &'a OcrConfig,
    stage: &'a OcrPipelineStage,
}

impl<'a> StageModelInputs<'a> {
    pub(crate) fn new(parent: &'a OcrConfig, stage: &'a OcrPipelineStage) -> Self {
        Self { parent, stage }
    }
    pub(crate) fn backend(&self) -> &str {
        &self.stage.backend
    }
    #[cfg(any(test, all(feature = "ocr", not(target_arch = "wasm32"))))]
    pub(crate) fn tesseract(&self) -> Option<&TesseractConfig> {
        self.stage
            .tesseract_config
            .as_ref()
            .or(self.parent.tesseract_config.as_ref())
    }
    #[cfg(any(test, all(feature = "ocr", not(target_arch = "wasm32"))))]
    pub(crate) fn directory(&self) -> Option<&Path> {
        self.parent.tessdata_path.as_deref()
    }
    #[cfg(any(test, all(feature = "ocr", not(target_arch = "wasm32"))))]
    pub(crate) fn languages(&self) -> Vec<String> {
        crate::core::config::ocr::effective_tesseract_language_for(
            self.stage.language.as_deref().unwrap_or(&self.parent.language),
            self.tesseract(),
        )
    }
    pub(crate) fn apply_to(&self, target: &mut OcrConfig) {
        target.backend = self.backend().into();
        target
            .language
            .clone_from(self.stage.language.as_ref().unwrap_or(&self.parent.language));
        if let Some(tesseract) = &self.stage.tesseract_config {
            target.tesseract_config = Some(tesseract.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_projection(top: serde_json::Value, mut stage: serde_json::Value, language: &str, psm: Option<i32>) {
        let mut parent: OcrConfig = serde_json::from_value(top).unwrap();
        parent.tessdata_path = Some("parent-model-root".into());
        stage["backend"] = "tesseract".into();
        let stage: OcrPipelineStage = serde_json::from_value(stage).unwrap();
        let inputs = StageModelInputs::new(&parent, &stage);
        let mut actual = parent.clone();
        inputs.apply_to(&mut actual);
        assert_eq!(actual.backend, "tesseract");
        assert_eq!(actual.effective_tesseract_language(), [language]);
        assert_eq!(inputs.languages(), actual.effective_tesseract_language());
        assert_eq!(inputs.tesseract().and_then(|config| config.psm), psm);
        assert_eq!(actual.tesseract_config.as_ref().and_then(|config| config.psm), psm);
        assert_eq!(inputs.directory(), actual.tessdata_path.as_deref());
    }

    #[test]
    fn inherited_settings_and_explicit_unset_psm_match_the_runner() {
        for (top, stage, language, psm) in [
            (
                serde_json::json!({"language":["deu"],"tesseract_config":{"language":["fra"],"psm":3}}),
                serde_json::json!({}),
                "deu",
                Some(3),
            ),
            (
                serde_json::json!({"tesseract_config":{"language":["fra"],"psm":1}}),
                serde_json::json!({}),
                "fra",
                Some(1),
            ),
            (
                serde_json::json!({"tesseract_config":{"language":["fra"],"psm":1}}),
                serde_json::json!({"tesseract_config":{}}),
                "eng",
                None,
            ),
        ] {
            assert_projection(top, stage, language, psm);
        }
    }

    #[test]
    fn stage_and_vertical_language_overrides_match_the_runner() {
        for (top, stage, language, psm) in [
            (
                serde_json::json!({"language":["deu"],"tesseract_config":{"psm":3}}),
                serde_json::json!({"language":["eng"],"tesseract_config":{"language":["fra"],"psm":12}}),
                "fra",
                Some(12),
            ),
            (
                serde_json::json!({}),
                serde_json::json!({"language":["jpn_vert"]}),
                "jpn_vert",
                None,
            ),
            (
                serde_json::json!({}),
                serde_json::json!({"language":["jpn_vert"],"tesseract_config":{"psm":1}}),
                "jpn_vert",
                Some(1),
            ),
        ] {
            assert_projection(top, stage, language, psm);
        }
    }

    #[test]
    fn empty_language_fallbacks_keep_the_existing_reconciliation() {
        for (stage, language, psm) in [
            (
                serde_json::json!({"language":[],"tesseract_config":{"language":["fra"],"psm":6}}),
                "eng",
                Some(6),
            ),
            (
                serde_json::json!({"tesseract_config":{"language":[],"psm":0}}),
                "eng",
                Some(0),
            ),
        ] {
            assert_projection(serde_json::json!({}), stage, language, psm);
        }
    }
}
