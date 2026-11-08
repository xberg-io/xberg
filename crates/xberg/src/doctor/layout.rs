//! ~keep: Layout capability audit with cache-only optional checks.

use super::DoctorCheck;
use crate::core::config::{AccelerationConfig, ExtractionConfig, LayoutDetectionModel};
use crate::layout::engine::{CustomModelVariant, LayoutEngine, LayoutEngineConfig, ModelBackend};
use crate::layout::model_manager::{LayoutModelManager, ModelCacheProbe};

pub(super) fn probe_layout(config: &ExtractionConfig) -> Vec<DoctorCheck> {
    let manager = LayoutModelManager::new(None);
    probe_layout_with_manager(config, &manager)
}

// Every push below is individually `#[cfg]`-gated, so which checks exist -- and in what
// order -- depends on the feature set; a `vec![]` literal cannot express that. The lint
// only trips once four or more of the pushes survive cfg-stripping, which is why it fires
// on `formula-recognition,pdf` (four consecutive pushes) but not on `full` (two). ~keep
#[allow(
    clippy::vec_init_then_push,
    reason = "the pushes are cfg-gated; a vec![] literal cannot express them"
)]
fn probe_layout_with_manager(config: &ExtractionConfig, manager: &LayoutModelManager) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let selected_model = config.layout.as_ref().map(|layout| layout.model);
    let v3_selected = selected_model == Some(LayoutDetectionModel::PpDocLayoutV3);
    #[cfg(feature = "layout-detection")]
    checks.push(match (&config.layout, manager.probe_model_cache("pp_doclayout_v3")) {
        (Some(layout), ModelCacheProbe::Present) if v3_selected => match manager.verified_model_path("pp_doclayout_v3")
        {
            Some(path) => probe_configured_model(
                "layout.pp-doclayout-v3",
                CustomModelVariant::PpDocLayoutV3,
                layout,
                &path,
            ),
            None => DoctorCheck::fail("layout.pp-doclayout-v3", "cached model failed checksum validation"),
        },
        (Some(_), ModelCacheProbe::Invalid(message)) if v3_selected => {
            DoctorCheck::fail("layout.pp-doclayout-v3", message)
        }
        (_, status) => cache_check("layout.pp-doclayout-v3", status),
    });
    #[cfg(not(feature = "layout-detection"))]
    if v3_selected {
        checks.push(DoctorCheck::fail(
            "layout.pp-doclayout-v3",
            "selected layout model requires the ORT-backed `layout-detection` feature",
        ));
    }
    checks.push(match (&config.layout, manager.probe_model_cache("rtdetr")) {
        (Some(layout), ModelCacheProbe::Present) if !v3_selected => match manager.verified_model_path("rtdetr") {
            Some(path) => probe_configured_model("layout.rtdetr", CustomModelVariant::RtDetr, layout, &path),
            None => DoctorCheck::fail("layout.rtdetr", "cached model failed checksum validation"),
        },
        (Some(_), ModelCacheProbe::Invalid(message)) if !v3_selected => DoctorCheck::fail("layout.rtdetr", message),
        (_, status) => cache_check("layout.rtdetr", status),
    });
    #[cfg(feature = "formula-recognition")]
    checks.push(formula_cache_check(config));
    #[cfg(not(feature = "formula-recognition"))]
    if config
        .layout
        .as_ref()
        .is_some_and(|layout| layout.formula_model.is_some())
    {
        checks.push(DoctorCheck::fail(
            "layout.formula.latex-ocr",
            "configured formula recognition is not compiled in (enable `formula-recognition`)",
        ));
    }
    #[cfg(feature = "pdf")]
    checks.push(table_cache_check(
        config,
        manager,
        "layout.table.classifier",
        "table_classifier",
    ));
    #[cfg(all(feature = "pdf", not(feature = "layout-detection")))]
    if let Some(layout) = config.layout.as_ref()
        && let Some(check) = unsupported_tract_table_check(layout.table_model)
    {
        checks.push(check);
    }
    #[cfg(all(feature = "layout-detection", feature = "pdf"))]
    checks.extend([
        table_cache_check(config, manager, "layout.table.slanet-plus", "slanet_plus"),
        table_cache_check(config, manager, "layout.table.slanet-wired", "slanet_wired"),
        table_cache_check(config, manager, "layout.table.slanet-wireless", "slanet_wireless"),
        table_cache_check(config, manager, "layout.table.tatr", "tatr"),
    ]);
    #[cfg(feature = "layout-detection")]
    checks.push(DoctorCheck::skip(
        "layout.yolo",
        "no managed YOLO model is bundled; provide a custom local model path",
    ));
    checks.sort_by(|left, right| left.name.cmp(&right.name));
    checks
}

#[cfg(all(feature = "pdf", not(feature = "layout-detection")))]
fn unsupported_tract_table_check(table_model: crate::core::config::TableModel) -> Option<DoctorCheck> {
    let selected = match table_model {
        crate::core::config::TableModel::Tatr => "tatr",
        crate::core::config::TableModel::SlanetWired => "slanet-wired",
        crate::core::config::TableModel::SlanetWireless => "slanet-wireless",
        crate::core::config::TableModel::SlanetPlus => "slanet-plus",
        crate::core::config::TableModel::SlanetAuto => "slanet-auto",
        crate::core::config::TableModel::Disabled => return None,
    };
    Some(DoctorCheck::fail(
        format!("layout.table.{selected}"),
        "selected table structure model requires the ORT-backed `layout-detection` feature",
    ))
}

#[cfg(feature = "formula-recognition")]
fn formula_cache_check(config: &ExtractionConfig) -> DoctorCheck {
    formula_cache_check_in(config, None)
}

#[cfg(feature = "formula-recognition")]
fn formula_cache_check_in(config: &ExtractionConfig, cache_dir: Option<&std::path::Path>) -> DoctorCheck {
    let configured = config
        .layout
        .as_ref()
        .is_some_and(|layout| layout.formula_model.is_some());
    let (present, missing, invalid) = crate::formula_recognition::probe_models_in(cache_dir);
    let message = format!(
        "formula model presence: {present} present, {missing} missing, {invalid} invalid; inference was not checked"
    );
    if !configured {
        return if invalid == 0 {
            DoctorCheck::skip("layout.formula.latex-ocr", message)
        } else {
            DoctorCheck::warn("layout.formula.latex-ocr", message)
        };
    }
    if invalid > 0 {
        return DoctorCheck::fail("layout.formula.latex-ocr", message);
    }
    if missing > 0 {
        return DoctorCheck::skip("layout.formula.latex-ocr", message);
    }
    if crate::formula_recognition::cached_models_verified_in(cache_dir) {
        DoctorCheck::skip(
            "layout.formula.latex-ocr",
            "configured formula models passed checksum validation; inference is checked on document input",
        )
    } else {
        DoctorCheck::fail(
            "layout.formula.latex-ocr",
            "configured formula model failed checksum validation",
        )
    }
}

#[cfg(feature = "pdf")]
fn table_cache_check(
    config: &ExtractionConfig,
    manager: &LayoutModelManager,
    name: &str,
    model_type: &str,
) -> DoctorCheck {
    let required = config
        .layout
        .as_ref()
        .is_some_and(|layout| table_model_requires(layout.table_model, model_type));
    let status = manager.probe_model_cache(model_type);
    if !required {
        return cache_check(name, status);
    }
    match status {
        ModelCacheProbe::Present if manager.is_model_verified(model_type) => DoctorCheck::skip(
            name,
            "selected model passed checksum validation; inference is checked on document input",
        ),
        ModelCacheProbe::Present => DoctorCheck::fail(name, "selected cached model failed checksum validation"),
        ModelCacheProbe::Missing => DoctorCheck::skip(name, "selected model is not cached; first use may download it"),
        ModelCacheProbe::Invalid(message) => DoctorCheck::fail(name, message),
    }
}

#[cfg(feature = "pdf")]
fn table_model_requires(table_model: crate::core::config::TableModel, model_type: &str) -> bool {
    match table_model {
        crate::core::config::TableModel::Tatr => model_type == "tatr",
        crate::core::config::TableModel::SlanetWired => model_type == "slanet_wired",
        crate::core::config::TableModel::SlanetWireless => model_type == "slanet_wireless",
        crate::core::config::TableModel::SlanetPlus => model_type == "slanet_plus",
        crate::core::config::TableModel::SlanetAuto => {
            matches!(model_type, "table_classifier" | "slanet_wired" | "slanet_wireless")
        }
        crate::core::config::TableModel::Disabled => false,
    }
}

fn cache_check(name: &str, status: ModelCacheProbe) -> DoctorCheck {
    match status {
        ModelCacheProbe::Present => DoctorCheck::skip(
            name,
            "model is present, readable, and has the expected size; integrity and inference were not checked",
        ),
        ModelCacheProbe::Missing => DoctorCheck::skip(name, "model not cached locally; first use may download it"),
        ModelCacheProbe::Invalid(message) => DoctorCheck::warn(name, message),
    }
}

fn probe_configured_model(
    name: &str,
    variant: CustomModelVariant,
    layout: &crate::core::config::LayoutDetectionConfig,
    model_path: &std::path::Path,
) -> DoctorCheck {
    match run_inference(&variant, layout, model_path, layout.acceleration.clone()) {
        Ok(detections) => DoctorCheck::pass(
            name,
            format!("inference ok ({detections} detections on synthetic page)"),
        ),
        Err(error) if runtime_retries_on_cpu(layout) => probe_cpu_fallback(name, &variant, layout, model_path, error),
        Err(error) => DoctorCheck::fail(
            name,
            format!("inference failed with configured execution provider: {error}"),
        ),
    }
}

fn probe_cpu_fallback(
    name: &str,
    variant: &CustomModelVariant,
    layout: &crate::core::config::LayoutDetectionConfig,
    model_path: &std::path::Path,
    provider_error: crate::layout::LayoutError,
) -> DoctorCheck {
    let acceleration = Some(AccelerationConfig {
        provider: crate::core::config::ExecutionProviderType::Cpu,
        ..Default::default()
    });
    match run_inference(variant, layout, model_path, acceleration) {
        Ok(_) => DoctorCheck::warn(
            name,
            format!(
                "automatic execution provider failed, CPU works; runtime retries on CPU with a processing warning ({provider_error})"
            ),
        ),
        Err(cpu_error) => DoctorCheck::fail(
            name,
            format!("inference failed (automatic provider: {provider_error}; CPU retry: {cpu_error})"),
        ),
    }
}

fn run_inference(
    variant: &CustomModelVariant,
    layout: &crate::core::config::LayoutDetectionConfig,
    model_path: &std::path::Path,
    acceleration: Option<AccelerationConfig>,
) -> Result<usize, crate::layout::LayoutError> {
    let engine_config = LayoutEngineConfig {
        backend: ModelBackend::Custom {
            path: model_path.to_path_buf(),
            variant: variant.clone(),
        },
        confidence_threshold: layout.confidence_threshold,
        apply_heuristics: layout.apply_heuristics,
        cache_dir: None,
        acceleration,
    };
    let mut engine = LayoutEngine::from_config(engine_config)?;
    Ok(engine.detect(&synthetic_page())?.detections.len())
}

fn runtime_retries_on_cpu(layout: &crate::core::config::LayoutDetectionConfig) -> bool {
    #[cfg(feature = "layout-detection")]
    {
        crate::ort_discovery::execution_provider_override().is_none()
            && layout
                .acceleration
                .as_ref()
                .is_none_or(|acceleration| acceleration.provider == crate::core::config::ExecutionProviderType::Auto)
    }
    #[cfg(not(feature = "layout-detection"))]
    {
        let _ = layout;
        false
    }
}

fn synthetic_page() -> image::RgbImage {
    let mut image = image::RgbImage::from_pixel(640, 480, image::Rgb([245, 245, 245]));
    let dark = image::Rgb([30, 30, 30]);
    for (top, left, width, height) in [
        (40_u32, 60_u32, 520_u32, 14_u32),
        (70, 60, 480, 14),
        (100, 60, 500, 14),
        (160, 60, 240, 120),
        (160, 340, 240, 120),
        (320, 60, 520, 14),
        (350, 60, 440, 14),
    ] {
        for y in top..(top + height) {
            for x in left..(left + width) {
                image.put_pixel(x, y, dark);
            }
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::layout::model_manager::RTDETR_MODEL_SIZE_BYTES;

    #[cfg(all(feature = "layout-detection", feature = "pdf"))]
    fn write_sparse_model(root: &std::path::Path, model_type: &str) -> std::path::PathBuf {
        let (filename, size) = LayoutModelManager::model_test_spec(model_type).unwrap();
        let directory = root.join(model_type);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(filename);
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(size).unwrap();
        path
    }

    #[test]
    fn full_layout_capability_audit_is_sorted_and_cache_only() {
        let root = TempDir::new().unwrap();
        let cache = root.path().join("absent-cache");
        let manager = LayoutModelManager::new(Some(cache.clone()));
        let checks = probe_layout_with_manager(&ExtractionConfig::default(), &manager);

        let names = checks.iter().map(|check| check.name.as_str()).collect::<Vec<_>>();
        let mut expected = vec!["layout.rtdetr"];
        #[cfg(feature = "formula-recognition")]
        expected.push("layout.formula.latex-ocr");
        #[cfg(feature = "layout-detection")]
        expected.extend(["layout.pp-doclayout-v3", "layout.yolo"]);
        #[cfg(feature = "pdf")]
        expected.push("layout.table.classifier");
        #[cfg(all(feature = "layout-detection", feature = "pdf"))]
        expected.extend([
            "layout.table.slanet-plus",
            "layout.table.slanet-wired",
            "layout.table.slanet-wireless",
            "layout.table.tatr",
        ]);
        expected.sort_unstable();
        assert_eq!(names, expected);
        assert!(
            checks
                .iter()
                .all(|check| check.status == crate::doctor::ProbeStatus::Skip)
        );
        assert!(!cache.exists(), "layout doctor audit must not create model cache paths");
    }

    #[test]
    fn configured_rtdetr_rejects_same_size_corruption_without_resolving_a_model() {
        let root = TempDir::new().unwrap();
        let model_dir = root.path().join("rtdetr");
        std::fs::create_dir_all(&model_dir).unwrap();
        let path = model_dir.join("model.onnx");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(RTDETR_MODEL_SIZE_BYTES).unwrap();
        let manager = LayoutModelManager::new(Some(root.path().to_path_buf()));
        let config = ExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        };

        let checks = probe_layout_with_manager(&config, &manager);
        let rtdetr = checks.iter().find(|check| check.name == "layout.rtdetr").unwrap();
        assert_eq!(rtdetr.status, crate::doctor::ProbeStatus::Fail);
        assert_eq!(rtdetr.message, "cached model failed checksum validation");
        assert_eq!(std::fs::metadata(path).unwrap().len(), RTDETR_MODEL_SIZE_BYTES);
    }

    #[cfg(any(not(feature = "layout-detection"), feature = "pdf"))]
    fn v3_config() -> ExtractionConfig {
        ExtractionConfig {
            layout: Some(crate::core::config::LayoutDetectionConfig {
                model: LayoutDetectionModel::PpDocLayoutV3,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[cfg(all(feature = "layout-detection", feature = "pdf"))]
    #[test]
    fn selected_v3_same_size_corruption_is_fatal_and_leaves_rtdetr_cache_only() {
        let root = TempDir::new().unwrap();
        write_sparse_model(root.path(), "pp_doclayout_v3");
        let manager = LayoutModelManager::new(Some(root.path().to_path_buf()));

        let checks = probe_layout_with_manager(&v3_config(), &manager);
        let v3 = checks
            .iter()
            .find(|check| check.name == "layout.pp-doclayout-v3")
            .unwrap();
        assert_eq!(v3.status, crate::doctor::ProbeStatus::Fail);
        assert_eq!(v3.message, "cached model failed checksum validation");
        let rtdetr = checks.iter().find(|check| check.name == "layout.rtdetr").unwrap();
        assert_eq!(rtdetr.status, crate::doctor::ProbeStatus::Skip);
    }

    #[cfg(all(feature = "layout-detection", feature = "pdf"))]
    #[test]
    fn unselected_v3_corruption_stays_a_cache_warning() {
        let root = TempDir::new().unwrap();
        write_sparse_model(root.path(), "pp_doclayout_v3");
        let manager = LayoutModelManager::new(Some(root.path().to_path_buf()));
        let config = ExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        };

        let checks = probe_layout_with_manager(&config, &manager);
        let v3 = checks
            .iter()
            .find(|check| check.name == "layout.pp-doclayout-v3")
            .unwrap();
        assert_ne!(v3.status, crate::doctor::ProbeStatus::Fail);
    }

    #[cfg(not(feature = "layout-detection"))]
    #[test]
    fn tract_layout_reports_selected_v3_fatal() {
        let manager = LayoutModelManager::new(Some(TempDir::new().unwrap().path().to_path_buf()));
        let checks = probe_layout_with_manager(&v3_config(), &manager);
        let v3 = checks
            .iter()
            .find(|check| check.name == "layout.pp-doclayout-v3")
            .unwrap();
        assert_eq!(v3.status, crate::doctor::ProbeStatus::Fail);
    }

    #[cfg(all(feature = "layout-detection", feature = "pdf"))]
    #[test]
    fn selected_tatr_same_size_corruption_is_fatal() {
        let root = TempDir::new().unwrap();
        write_sparse_model(root.path(), "tatr");
        let manager = LayoutModelManager::new(Some(root.path().to_path_buf()));
        let config = ExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        };

        let checks = probe_layout_with_manager(&config, &manager);
        let tatr = checks.iter().find(|check| check.name == "layout.table.tatr").unwrap();
        assert_eq!(tatr.status, crate::doctor::ProbeStatus::Fail);
        assert_eq!(tatr.message, "selected cached model failed checksum validation");
    }

    #[cfg(all(feature = "layout-detection", feature = "pdf"))]
    #[test]
    fn slanet_auto_requires_classifier_and_both_structure_models() {
        let root = TempDir::new().unwrap();
        write_sparse_model(root.path(), "table_classifier");
        let manager = LayoutModelManager::new(Some(root.path().to_path_buf()));
        let config = ExtractionConfig {
            layout: Some(crate::core::config::LayoutDetectionConfig {
                table_model: crate::core::config::TableModel::SlanetAuto,
                ..Default::default()
            }),
            ..Default::default()
        };

        let checks = probe_layout_with_manager(&config, &manager);
        let status = |name| checks.iter().find(|check| check.name == name).unwrap().status;
        assert_eq!(status("layout.table.classifier"), crate::doctor::ProbeStatus::Fail);
        assert_eq!(status("layout.table.slanet-wired"), crate::doctor::ProbeStatus::Skip);
        assert_eq!(status("layout.table.slanet-wireless"), crate::doctor::ProbeStatus::Skip);
        assert_eq!(status("layout.table.slanet-plus"), crate::doctor::ProbeStatus::Skip);
        assert_eq!(status("layout.table.tatr"), crate::doctor::ProbeStatus::Skip);
    }

    #[cfg(feature = "formula-recognition")]
    #[test]
    fn configured_formula_wrong_size_artifact_is_fatal() {
        let root = TempDir::new().unwrap();
        std::fs::write(root.path().join("image_resizer.onnx"), b"truncated").unwrap();
        let config = ExtractionConfig {
            layout: Some(crate::core::config::LayoutDetectionConfig {
                formula_model: Some(crate::core::config::layout::FormulaModel::LatexOcr),
                ..Default::default()
            }),
            ..Default::default()
        };

        let check = formula_cache_check_in(&config, Some(root.path()));
        assert_eq!(check.status, crate::doctor::ProbeStatus::Fail);
        assert_eq!(check.name, "layout.formula.latex-ocr");
    }

    #[cfg(all(feature = "pdf", not(feature = "layout-detection")))]
    #[test]
    fn tract_layout_reports_configured_ort_only_table_model_fatal() {
        let manager = LayoutModelManager::new(Some(TempDir::new().unwrap().path().to_path_buf()));
        let config = ExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        };
        let checks = probe_layout_with_manager(&config, &manager);
        let tatr = checks.iter().find(|check| check.name == "layout.table.tatr").unwrap();
        assert_eq!(tatr.status, crate::doctor::ProbeStatus::Fail);
    }
}
