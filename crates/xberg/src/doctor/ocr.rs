//! Dispatch to the configured OCR backend's `probe()` via the registry.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::DoctorCheck;
use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineStage};
use crate::plugins::OcrBackend;
#[cfg(paddle_ocr)]
use crate::plugins::OcrBackendType;

#[derive(Clone)]
struct EffectiveProbeConfig {
    ordinal: usize,
    config: OcrConfig,
}

pub(super) fn probe_ocr(config: &ExtractionConfig) -> Vec<DoctorCheck> {
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    crate::plugins::ensure_ocr_backends_initialized();

    let registered = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let guard = registry.read();
        guard.registered_snapshot()
    };
    let expected = crate::plugins::registry::builtin_ocr_backend_names();
    audit_ocr_capabilities_from(config, &expected, registered)
}

fn builtin_fallback_name(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "paddleocr" => "paddle-ocr".to_string(),
        normalized => normalized.to_string(),
    }
}

fn audit_ocr_capabilities_from(
    config: &ExtractionConfig,
    expected: &[&str],
    registered: Vec<(String, Arc<dyn OcrBackend>)>,
) -> Vec<DoctorCheck> {
    let expected: BTreeSet<_> = expected.iter().map(|name| builtin_fallback_name(name)).collect();
    let registered: BTreeMap<_, _> = registered.into_iter().collect();
    let mut required_configs = configured_backend_configs(config, &expected, &registered);
    let implicit_tesseract = config.ocr.is_none() && expected.contains("tesseract");
    let unavailable_implicit_tesseract = config.ocr.is_none() && !expected.contains("tesseract");
    let forced_ocr = config.force_ocr || config.force_ocr_pages.as_ref().is_some_and(|pages| !pages.is_empty());
    if implicit_tesseract || forced_ocr && config.ocr.is_none() {
        required_configs
            .entry("tesseract".to_string())
            .or_default()
            .push(EffectiveProbeConfig {
                ordinal: 1,
                config: OcrConfig::implicit(),
            });
    }

    let mut names = expected.clone();
    names.extend(registered.keys().cloned());
    names.extend(required_configs.keys().cloned());
    if config.ocr.is_none() {
        names.insert("tesseract".to_string());
    }

    if names.is_empty() {
        return vec![DoctorCheck::skip(
            "ocr.tesseract",
            if config.effective_disable_ocr() {
                "OCR is disabled by configuration"
            } else {
                "default OCR backend is not compiled in (enable `ocr` or `ocr-wasm`)"
            },
        )];
    }

    names
        .into_iter()
        .map(|name| {
            let configured = required_configs.get(&name);
            let required = configured.is_some();
            if config.effective_disable_ocr() {
                return DoctorCheck::skip(format!("ocr.{name}"), "OCR is disabled by configuration");
            }

            let Some(backend) = registered.get(&name) else {
                return unregistered_backend_check(&name, &expected, required, unavailable_implicit_tesseract);
            };

            probe_registered_backend(backend.as_ref(), &name, configured, required)
        })
        .collect()
}

/// The check reported for a backend that is not in the registry: a skip when the default
/// tesseract backend simply was not compiled in, otherwise a fail (when something configured
/// demanded it) or a warn (when nothing did). ~keep
fn unregistered_backend_check(
    name: &str,
    expected: &BTreeSet<String>,
    required: bool,
    unavailable_implicit_tesseract: bool,
) -> DoctorCheck {
    if name == "tesseract" && unavailable_implicit_tesseract && !required {
        return DoctorCheck::skip(
            "ocr.tesseract",
            "default OCR backend is not compiled in (enable `ocr` or `ocr-wasm`)",
        );
    }
    let message = if expected.contains(name) {
        "compiled backend was not registered"
    } else {
        "required backend is not registered"
    };
    if required {
        DoctorCheck::fail(format!("ocr.{name}"), message)
    } else {
        DoctorCheck::warn(format!("ocr.{name}"), message)
    }
}

/// Probe a registered backend with every configuration that selected it, falling back to a single
/// default configuration when nothing did. A failing probe on an optional backend is downgraded to
/// a warning so an uninstalled extra does not fail the whole doctor run. ~keep
fn probe_registered_backend(
    backend: &dyn OcrBackend,
    name: &str,
    configured: Option<&Vec<EffectiveProbeConfig>>,
    required: bool,
) -> DoctorCheck {
    #[cfg(paddle_ocr)]
    if backend.backend_type() == OcrBackendType::PaddleOCR && !required {
        return optional_paddle_check();
    }

    let default_config = [EffectiveProbeConfig {
        ordinal: 1,
        config: backend_config(OcrConfig::implicit(), name),
    }];
    let probe_configs = configured.map_or(default_config.as_slice(), Vec::as_slice);
    let mut check = aggregate_probes(backend, probe_configs);
    check.name = format!("ocr.{name}");
    if check.status == super::ProbeStatus::Fail && !required {
        check.status = super::ProbeStatus::Warn;
        check.message = format!("optional backend probe failed: {}", check.message);
    }
    check
}

#[cfg(paddle_ocr)]
fn optional_paddle_check() -> DoctorCheck {
    let config = crate::paddle_ocr::PaddleOcrConfig::default();
    let manager = crate::paddle_ocr::ModelManager::new(config.resolve_cache_dir());
    let (present, missing, invalid) = manager.probe_manifest_presence();
    paddle_presence_check(present, missing, invalid)
}

#[cfg(paddle_ocr)]
fn paddle_presence_check(present: usize, missing: usize, invalid: usize) -> DoctorCheck {
    let message = format!(
        "optional model manifest presence: {present} present, {missing} missing, {invalid} invalid; checksums and inference were not checked"
    );
    if invalid == 0 {
        DoctorCheck::skip("ocr.paddle-ocr", message)
    } else {
        DoctorCheck::warn("ocr.paddle-ocr", message)
    }
}

fn configured_backend_configs(
    config: &ExtractionConfig,
    expected: &BTreeSet<String>,
    registered: &BTreeMap<String, Arc<dyn OcrBackend>>,
) -> BTreeMap<String, Vec<EffectiveProbeConfig>> {
    let Some(ocr) = config.ocr.as_ref() else {
        return BTreeMap::new();
    };
    let configs = effective_backend_configs(ocr);
    let mut by_backend: BTreeMap<String, Vec<EffectiveProbeConfig>> = BTreeMap::new();
    for mut effective in configs {
        let resolved = resolve_backend_name(&effective.config.backend, expected, registered);
        effective.config.backend = resolved.clone();
        by_backend.entry(resolved).or_default().push(effective);
    }
    by_backend
}

fn resolve_backend_name(
    requested: &str,
    expected: &BTreeSet<String>,
    registered: &BTreeMap<String, Arc<dyn OcrBackend>>,
) -> String {
    if registered.contains_key(requested) {
        return requested.to_string();
    }
    let fallback = builtin_fallback_name(requested);
    if registered.contains_key(&fallback) || expected.contains(&fallback) {
        fallback
    } else {
        requested.to_string()
    }
}

fn effective_backend_configs(ocr: &OcrConfig) -> Vec<EffectiveProbeConfig> {
    if let Some(pipeline) = ocr.pipeline.as_ref() {
        return ordered_stage_configs(ocr, &pipeline.stages);
    }
    #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
    if let Some(pipeline) = ocr.effective_pipeline() {
        return ordered_stage_configs(ocr, &pipeline.stages);
    }
    if ocr.vlm_config.is_some() {
        match &ocr.vlm_fallback {
            crate::core::config::VlmFallbackPolicy::Always => {
                let mut vlm = ocr.clone();
                vlm.backend = "vlm".to_string();
                vlm.backend_options = None;
                vlm.pipeline = None;
                return single_probe_config(vlm);
            }
            crate::core::config::VlmFallbackPolicy::OnLowQuality { .. } => {
                let mut primary = ocr.clone();
                primary.pipeline = None;
                primary.vlm_config = None;
                let mut vlm = primary.clone();
                vlm.backend = "vlm".to_string();
                vlm.vlm_config = ocr.vlm_config.clone();
                vlm.backend_options = None;
                return vec![
                    EffectiveProbeConfig {
                        ordinal: 1,
                        config: primary,
                    },
                    EffectiveProbeConfig {
                        ordinal: 2,
                        config: vlm,
                    },
                ];
            }
            crate::core::config::VlmFallbackPolicy::Disabled => {}
        }
    }
    single_probe_config(ocr.clone())
}

fn ordered_stage_configs(ocr: &OcrConfig, stages: &[OcrPipelineStage]) -> Vec<EffectiveProbeConfig> {
    let mut stages = stages.to_vec();
    stages.sort_by_key(|stage| std::cmp::Reverse(stage.priority));
    stages
        .iter()
        .enumerate()
        .map(|(index, stage)| EffectiveProbeConfig {
            ordinal: index + 1,
            config: stage_backend_config(ocr, stage),
        })
        .collect()
}

fn single_probe_config(config: OcrConfig) -> Vec<EffectiveProbeConfig> {
    vec![EffectiveProbeConfig { ordinal: 1, config }]
}

fn stage_backend_config(parent: &OcrConfig, stage: &OcrPipelineStage) -> OcrConfig {
    let mut config = parent.clone();
    config.backend = stage.backend.clone();
    if let Some(language) = stage.language.as_ref() {
        config.language = language.clone();
    }
    if let Some(tesseract) = stage.tesseract_config.as_ref() {
        config.tesseract_config = Some(tesseract.clone());
    }
    if stage.paddle_ocr_config.is_some() || stage.paddle_ocr_settings.is_some() {
        config.paddle_ocr_config = stage.paddle_ocr_config.clone();
        config.paddle_ocr_settings = stage.paddle_ocr_settings.clone();
    }
    config.vlm_config = stage.vlm_config.clone();
    config.backend_options = stage.backend_options.clone();
    config.pipeline = None;
    config
}

fn backend_config(mut config: OcrConfig, backend: &str) -> OcrConfig {
    config.backend = backend.to_string();
    config
}

fn aggregate_probes(backend: &dyn OcrBackend, configs: &[EffectiveProbeConfig]) -> DoctorCheck {
    let probes: Vec<_> = configs
        .iter()
        .map(|effective| (effective.ordinal, backend.probe(&effective.config)))
        .collect();
    if let [probe] = probes.as_slice() {
        return probe.1.clone();
    }
    let status = probes
        .iter()
        .max_by_key(|(_, check)| probe_severity(check.status))
        .map_or(super::ProbeStatus::Skip, |(_, check)| check.status);
    let message = probes
        .iter()
        .map(|(ordinal, check)| format!("stage {ordinal}: {}", check.message))
        .collect::<Vec<_>>()
        .join("; ");
    DoctorCheck {
        name: String::new(),
        status,
        message,
    }
}

fn probe_severity(status: super::ProbeStatus) -> u8 {
    match status {
        super::ProbeStatus::Pass => 0,
        super::ProbeStatus::Skip => 1,
        super::ProbeStatus::Warn => 2,
        super::ProbeStatus::Fail => 3,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
    use crate::types::ExtractedDocument;

    struct ProbeBackend {
        name: &'static str,
        status: crate::doctor::ProbeStatus,
        backend_type: OcrBackendType,
    }

    struct RecordingBackend {
        configs: Arc<Mutex<Vec<OcrConfig>>>,
    }

    impl Plugin for RecordingBackend {
        fn name(&self) -> &str {
            "recording"
        }
    }

    #[async_trait]
    impl OcrBackend for RecordingBackend {
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
            Ok(ExtractedDocument::default())
        }

        fn supports_language(&self, _: &str) -> bool {
            true
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }

        fn probe(&self, config: &OcrConfig) -> DoctorCheck {
            self.configs.lock().unwrap().push(config.clone());
            DoctorCheck::pass("recording", "recorded")
        }
    }

    impl Plugin for ProbeBackend {
        fn name(&self) -> &str {
            self.name
        }
    }

    #[async_trait]
    impl OcrBackend for ProbeBackend {
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
            Ok(ExtractedDocument::default())
        }

        fn supports_language(&self, _: &str) -> bool {
            true
        }

        fn backend_type(&self) -> OcrBackendType {
            self.backend_type
        }

        fn probe(&self, _: &OcrConfig) -> DoctorCheck {
            DoctorCheck {
                name: self.name.to_string(),
                status: self.status,
                message: "synthetic probe failure".to_string(),
            }
        }
    }

    fn backend(name: &'static str, status: crate::doctor::ProbeStatus) -> (String, Arc<dyn OcrBackend>) {
        backend_of_type(name, status, OcrBackendType::Custom)
    }

    fn backend_of_type(
        name: &'static str,
        status: crate::doctor::ProbeStatus,
        backend_type: OcrBackendType,
    ) -> (String, Arc<dyn OcrBackend>) {
        (
            name.to_string(),
            Arc::new(ProbeBackend {
                name,
                status,
                backend_type,
            }),
        )
    }

    #[test]
    fn optional_probe_failure_is_warning_but_selected_failure_is_fatal() {
        let optional = audit_ocr_capabilities_from(
            &ExtractionConfig::default(),
            &["optional"],
            vec![backend("optional", crate::doctor::ProbeStatus::Fail)],
        );
        assert_eq!(optional[0].name, "ocr.optional");
        assert_eq!(optional[0].status, crate::doctor::ProbeStatus::Warn);

        let selected = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "optional".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let required = audit_ocr_capabilities_from(
            &selected,
            &["optional"],
            vec![backend("optional", crate::doctor::ProbeStatus::Fail)],
        );
        assert_eq!(required[0].status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn missing_compiled_backend_is_reported_and_required_tesseract_is_fatal() {
        let checks = audit_ocr_capabilities_from(&ExtractionConfig::default(), &["optional", "tesseract"], vec![]);
        assert_eq!(
            checks.iter().map(|check| check.name.as_str()).collect::<Vec<_>>(),
            ["ocr.optional", "ocr.tesseract"]
        );
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Warn);
        assert_eq!(checks[1].status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn disabled_ocr_enumerates_all_capabilities_as_skipped() {
        let config = ExtractionConfig {
            disable_ocr: true,
            ..Default::default()
        };
        let checks = audit_ocr_capabilities_from(
            &config,
            &["alpha", "zeta"],
            vec![backend("custom", crate::doctor::ProbeStatus::Fail)],
        );
        assert_eq!(
            checks.iter().map(|check| check.name.as_str()).collect::<Vec<_>>(),
            ["ocr.alpha", "ocr.custom", "ocr.tesseract", "ocr.zeta"]
        );
        assert!(
            checks
                .iter()
                .all(|check| check.status == crate::doctor::ProbeStatus::Skip)
        );
    }

    #[test]
    fn unavailable_configured_pipeline_stage_fails_the_report() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(crate::core::config::OcrPipelineConfig {
                    stages: vec![crate::core::config::OcrPipelineStage {
                        backend: "missing-stage".to_string(),
                        priority: 100,
                        language: Some(vec!["deu".to_string()]),
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: Some(serde_json::json!({"mode": "accurate"})),
                    }],
                    quality_thresholds: Default::default(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = audit_ocr_capabilities_from(&config, &[], vec![]);

        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "ocr.missing-stage");
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn configured_vlm_fallback_is_required() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                vlm_fallback: crate::core::config::VlmFallbackPolicy::Always,
                vlm_config: Some(Default::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = audit_ocr_capabilities_from(&config, &[], vec![]);

        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "ocr.vlm");
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn optional_backend_failure_is_a_warning_even_when_named_vlm() {
        let checks = audit_ocr_capabilities_from(
            &ExtractionConfig::default(),
            &["vlm"],
            vec![backend("vlm", crate::doctor::ProbeStatus::Fail)],
        );

        let vlm = checks.iter().find(|check| check.name == "ocr.vlm").unwrap();
        assert_eq!(vlm.status, crate::doctor::ProbeStatus::Warn);
        let tesseract = checks.iter().find(|check| check.name == "ocr.tesseract").unwrap();
        assert_eq!(tesseract.status, crate::doctor::ProbeStatus::Skip);
    }

    #[test]
    fn pipeline_probe_preserves_stage_specific_configuration() {
        let configs = Arc::new(Mutex::new(Vec::new()));
        let backend: Arc<dyn OcrBackend> = Arc::new(RecordingBackend {
            configs: configs.clone(),
        });
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(crate::core::config::OcrPipelineConfig {
                    stages: vec![crate::core::config::OcrPipelineStage {
                        backend: "recording".to_string(),
                        priority: 100,
                        language: Some(vec!["deu".to_string()]),
                        tesseract_config: Some(Default::default()),
                        paddle_ocr_config: None,
                        paddle_ocr_settings: Some(
                            crate::paddle_ocr::PaddleOcrConfig::new("en").with_model_tier("server"),
                        ),
                        vlm_config: Some(crate::core::config::LlmConfig {
                            model: "test/model".to_string(),
                            ..Default::default()
                        }),
                        backend_options: Some(serde_json::json!({"mode": "accurate"})),
                    }],
                    quality_thresholds: Default::default(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let checks = audit_ocr_capabilities_from(&config, &[], vec![("recording".to_string(), backend)]);
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Pass);
        let captured = configs.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].language, ["deu"]);
        assert!(captured[0].tesseract_config.is_some());
        assert_eq!(
            captured[0]
                .paddle_ocr_settings
                .as_ref()
                .map(|paddle| paddle.model_tier.as_str()),
            Some("server")
        );
        assert_eq!(captured[0].vlm_config.as_ref().unwrap().model, "test/model");
        assert_eq!(
            captured[0].backend_options,
            Some(serde_json::json!({"mode": "accurate"}))
        );
    }

    #[test]
    fn low_quality_vlm_fallback_does_not_leak_vlm_config_into_primary_stage() {
        let ocr = OcrConfig {
            vlm_fallback: crate::core::config::VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.6 },
            vlm_config: Some(crate::core::config::LlmConfig {
                model: "test/model".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let configs = effective_backend_configs(&ocr);
        let primary = configs
            .iter()
            .find(|effective| effective.config.backend == "tesseract")
            .unwrap();
        let vlm = configs
            .iter()
            .find(|effective| effective.config.backend == "vlm")
            .unwrap();
        assert!(primary.config.vlm_config.is_none());
        assert_eq!(vlm.config.vlm_config.as_ref().unwrap().model, "test/model");
    }

    #[test]
    fn case_distinct_custom_backend_names_are_not_collapsed() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "Foo".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = audit_ocr_capabilities_from(
            &config,
            &[],
            vec![
                backend("Foo", crate::doctor::ProbeStatus::Fail),
                backend("foo", crate::doctor::ProbeStatus::Pass),
            ],
        );

        assert_eq!(
            checks.iter().map(|check| check.name.as_str()).collect::<Vec<_>>(),
            ["ocr.Foo", "ocr.foo"]
        );
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Fail);
        assert_eq!(checks[1].status, crate::doctor::ProbeStatus::Pass);
    }

    #[test]
    fn configured_vlm_uses_registered_backend_probe() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "vlm".to_string(),
                vlm_config: Some(crate::core::config::LlmConfig {
                    model: "test/model".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let checks = audit_ocr_capabilities_from(
            &config,
            &["vlm"],
            vec![backend("vlm", crate::doctor::ProbeStatus::Skip)],
        );
        let vlm = checks.iter().find(|check| check.name == "ocr.vlm").unwrap();
        assert_eq!(vlm.status, crate::doctor::ProbeStatus::Skip);
        assert_eq!(vlm.message, "synthetic probe failure");
    }

    #[test]
    fn configured_vlm_without_local_configuration_propagates_probe_failure() {
        let direct = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "vlm".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let direct_checks = audit_ocr_capabilities_from(
            &direct,
            &["vlm"],
            vec![backend("vlm", crate::doctor::ProbeStatus::Fail)],
        );
        let direct_vlm = direct_checks.iter().find(|check| check.name == "ocr.vlm").unwrap();
        assert_eq!(direct_vlm.status, crate::doctor::ProbeStatus::Fail);

        let pipeline = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(crate::core::config::OcrPipelineConfig {
                    stages: vec![crate::core::config::OcrPipelineStage {
                        backend: "vlm".to_string(),
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
        let pipeline_checks = audit_ocr_capabilities_from(
            &pipeline,
            &["vlm"],
            vec![backend("vlm", crate::doctor::ProbeStatus::Fail)],
        );
        let pipeline_vlm = pipeline_checks.iter().find(|check| check.name == "ocr.vlm").unwrap();
        assert_eq!(pipeline_vlm.status, crate::doctor::ProbeStatus::Fail);
    }

    #[test]
    fn custom_replacements_for_builtin_names_use_custom_probe_semantics() {
        let vlm_config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "vlm".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let vlm_checks = audit_ocr_capabilities_from(
            &vlm_config,
            &["vlm"],
            vec![backend("vlm", crate::doctor::ProbeStatus::Pass)],
        );
        let vlm = vlm_checks.iter().find(|check| check.name == "ocr.vlm").unwrap();
        assert_eq!(vlm.status, crate::doctor::ProbeStatus::Pass);

        let paddle_checks = audit_ocr_capabilities_from(
            &ExtractionConfig::default(),
            &["paddle-ocr"],
            vec![backend("paddle-ocr", crate::doctor::ProbeStatus::Fail)],
        );
        let paddle = paddle_checks
            .iter()
            .find(|check| check.name == "ocr.paddle-ocr")
            .unwrap();
        assert_eq!(paddle.status, crate::doctor::ProbeStatus::Warn);
    }

    #[test]
    fn repeated_backend_messages_use_global_priority_order_ordinals() {
        let stage = |backend: &str, priority, language: &str| crate::core::config::OcrPipelineStage {
            backend: backend.to_string(),
            priority,
            language: Some(vec![language.to_string()]),
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        };
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(crate::core::config::OcrPipelineConfig {
                    stages: vec![
                        stage("recording", 10, "deu"),
                        stage("other", 50, "fra"),
                        stage("recording", 100, "eng"),
                    ],
                    quality_thresholds: Default::default(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let configs = Arc::new(Mutex::new(Vec::new()));
        let recording: Arc<dyn OcrBackend> = Arc::new(RecordingBackend {
            configs: configs.clone(),
        });
        let checks = audit_ocr_capabilities_from(
            &config,
            &[],
            vec![
                ("recording".to_string(), recording),
                backend("other", crate::doctor::ProbeStatus::Pass),
            ],
        );

        let recording = checks.iter().find(|check| check.name == "ocr.recording").unwrap();
        assert_eq!(recording.message, "stage 1: recorded; stage 3: recorded");
        let captured = configs.lock().unwrap();
        assert_eq!(captured[0].language, ["eng"]);
        assert_eq!(captured[1].language, ["deu"]);
    }

    #[test]
    fn empty_force_pages_does_not_require_unavailable_tesseract() {
        let config = ExtractionConfig {
            force_ocr_pages: Some(Vec::new()),
            ..Default::default()
        };
        let checks = audit_ocr_capabilities_from(&config, &[], vec![]);
        assert_eq!(checks[0].name, "ocr.tesseract");
        assert_eq!(checks[0].status, crate::doctor::ProbeStatus::Skip);
    }

    #[test]
    fn force_controls_do_not_add_tesseract_to_explicit_custom_backend() {
        for config in [
            ExtractionConfig {
                force_ocr: true,
                ocr: Some(OcrConfig {
                    backend: "custom".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ExtractionConfig {
                force_ocr_pages: Some(vec![1]),
                ocr: Some(OcrConfig {
                    backend: "custom".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ] {
            let checks =
                audit_ocr_capabilities_from(&config, &[], vec![backend("custom", crate::doctor::ProbeStatus::Pass)]);
            assert_eq!(checks.len(), 1);
            assert_eq!(checks[0].name, "ocr.custom");
        }
    }
}
