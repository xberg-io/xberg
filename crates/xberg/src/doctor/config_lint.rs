//! Static configuration diagnostics: misconfigurations detectable without
//! touching the environment.
//!
//! Backend-name resolution itself is reported by the OCR probe dispatch (the
//! registry is the live source of truth; the `config_validation` backend list
//! predates the candle backends and is not widened to runtime).

use super::DoctorCheck;
use crate::core::config::ExtractionConfig;
use crate::core::config::ocr::VlmFallbackPolicy;

pub(super) fn lint_config(config: &ExtractionConfig) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    let validation_config = config_with_registered_custom_backends_substituted(config);
    let validation_ok = match validation_config.validate() {
        Ok(()) => true,
        Err(error) => {
            checks.push(DoctorCheck::fail("config.validation", error.to_string()));
            false
        }
    };

    if config.force_ocr && config.effective_disable_ocr() {
        checks.push(DoctorCheck::fail(
            "config.ocr",
            "force_ocr and disable_ocr cannot both be true",
        ));
    }

    let Some(ocr) = &config.ocr else { return checks };
    if !ocr.enabled {
        return checks;
    }

    if validation_ok
        && ocr.pipeline.is_none()
        && ocr.vlm_fallback != VlmFallbackPolicy::Disabled
        && ocr.vlm_config.is_none()
    {
        checks.push(DoctorCheck::fail(
            "config.ocr.vlm_fallback",
            "vlm_fallback is enabled but vlm_config is missing; provide an LlmConfig with model and API key",
        ));
    }

    #[cfg(paddle_ocr)]
    if ocr.backend == "paddle-ocr" || ocr.backend == "paddleocr" {
        let (_model, warnings) = crate::paddle_ocr::select_paddle_language(&ocr.effective_languages());
        for warning in warnings {
            checks.push(DoctorCheck::fail("config.ocr.languages", warning.message.into_owned()));
        }
    }

    checks
}

fn config_with_registered_custom_backends_substituted(config: &ExtractionConfig) -> ExtractionConfig {
    let custom_names: std::collections::HashSet<String> = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        registry
            .registered_snapshot()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    };
    let mut validation = config.clone();
    if let Some(ocr) = validation.ocr.as_mut() {
        if custom_names.contains(&ocr.backend) {
            ocr.backend = "tesseract".to_string();
        }
        if let Some(pipeline) = ocr.pipeline.as_mut() {
            for stage in &mut pipeline.stages {
                if custom_names.contains(&stage.backend) {
                    stage.backend = "tesseract".to_string();
                }
            }
        }
    }
    validation
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::*;
    use crate::core::config::OcrConfig;

    struct CustomDoctorBackend(&'static str);

    impl crate::plugins::Plugin for CustomDoctorBackend {
        fn name(&self) -> &str {
            self.0
        }
    }

    #[async_trait]
    impl crate::plugins::OcrBackend for CustomDoctorBackend {
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::types::ExtractedDocument> {
            Ok(Default::default())
        }

        fn supports_language(&self, _: &str) -> bool {
            true
        }

        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Tesseract
        }

        fn probe(&self, _: &OcrConfig) -> DoctorCheck {
            DoctorCheck::pass(self.0, "custom backend ready")
        }
    }

    #[test]
    fn vlm_fallback_without_vlm_config_fails() {
        let ocr = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 },
            ..OcrConfig::default()
        };
        let config = ExtractionConfig {
            ocr: Some(ocr),
            ..ExtractionConfig::default()
        };
        let checks = lint_config(&config);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Fail);
        assert_eq!(checks[0].name, "config.validation");
    }

    #[test]
    fn default_config_has_no_lint_failures() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ..ExtractionConfig::default()
        };
        let checks = lint_config(&config);
        #[cfg(not(paddle_ocr))]
        assert!(checks.is_empty());
        #[cfg(paddle_ocr)]
        assert!(checks.iter().all(|c| c.status != crate::doctor::ProbeStatus::Fail));
    }

    #[test]
    fn force_ocr_with_top_level_disable_is_a_configuration_failure() {
        let config = ExtractionConfig {
            force_ocr: true,
            disable_ocr: true,
            ..Default::default()
        };
        let checks = lint_config(&config);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "config.ocr");
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn force_ocr_with_disabled_ocr_object_is_a_configuration_failure() {
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = lint_config(&config);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "config.ocr");
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn doctor_accepts_exact_registered_custom_backend_name() {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        registry
            .write()
            .register(Arc::new(CustomDoctorBackend("custom-tesseract-validation")))
            .unwrap();
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "custom-tesseract-validation".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let report = crate::doctor::doctor(&config);
        registry.write().remove("custom-tesseract-validation").unwrap();

        assert!(
            report.checks.iter().all(|check| check.name != "config.validation"),
            "registered custom backend must not be rejected by the static allowlist: {:?}",
            report.checks
        );
        assert!(report.is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn doctor_accepts_custom_replacement_for_builtin_vlm_name() {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let original = registry
            .read()
            .registered_snapshot()
            .into_iter()
            .find(|(name, _)| name == "vlm");
        registry.write().register(Arc::new(CustomDoctorBackend("vlm"))).unwrap();
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "vlm".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let report = crate::doctor::doctor(&config);
        if let Some((_, backend)) = original {
            registry.write().register(backend).unwrap();
        } else {
            registry.write().remove("vlm").unwrap();
        }

        assert!(
            report.checks.iter().all(|check| check.name != "config.validation"),
            "live replacement must not be assigned built-in VLM validation semantics: {:?}",
            report.checks
        );
        assert!(report.is_ok());
    }

    #[test]
    fn invalid_numeric_ocr_configuration_reports_validation_failure() {
        let thresholds = crate::core::config::OcrQualityThresholds {
            min_alnum_ratio: 2.0,
            ..Default::default()
        };
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                quality_thresholds: Some(thresholds),
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = lint_config(&config);
        assert!(
            checks
                .iter()
                .any(|check| { check.name == "config.validation" && check.status == crate::doctor::ProbeStatus::Fail })
        );
    }

    #[test]
    fn invalid_pipeline_backend_reports_validation_failure() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(crate::core::config::OcrPipelineConfig {
                    stages: vec![crate::core::config::OcrPipelineStage {
                        backend: "not-a-backend".to_string(),
                        priority: 100,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: None,
                    }],
                    quality_thresholds: Default::default(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = lint_config(&config);
        assert!(
            checks
                .iter()
                .any(|check| { check.name == "config.validation" && check.status == crate::doctor::ProbeStatus::Fail })
        );
    }
}
