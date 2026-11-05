use super::*;
use crate::core::config::{OcrConfig, OcrPipelineStage};

fn stage(language: &str, psm: i32) -> OcrPipelineStage {
    serde_json::from_value(serde_json::json!({
        "backend":"tesseract", "language":[language], "tesseract_config":{"psm":psm}
    }))
    .unwrap()
}

fn configuration(stages: Vec<OcrPipelineStage>) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            pipeline: Some(crate::core::config::OcrPipelineConfig {
                stages,
                quality_thresholds: Default::default(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn every_distinct_osd_stage_is_selected_and_repeated_stage_inputs_are_deduplicated() {
    let config = configuration(vec![
        stage("eng", 1),
        stage("fra", 12),
        stage("eng", 1),
        stage("deu", 6),
    ]);
    let dependency = CacheDependency::for_config(&config, Some("application/pdf"));
    assert!(dependency.model.is_none());
    assert!(!dependency.uncertifiable);
    assert_eq!(dependency.stage_models.len(), 2);
    assert_eq!(dependency.stage_models[0].languages, ["eng"]);
    assert_eq!(dependency.stage_models[1].languages, ["fra"]);
    let edge = configuration(vec![stage("eng", 1); MAX_MODEL_SELECTIONS]);
    let edge = CacheDependency::for_config(&edge, Some("application/pdf"));
    assert!(!edge.uncertifiable);
    assert_eq!(edge.stage_models.len(), 1);
}

#[tokio::test]
async fn oversized_stage_collection_bypasses_caches_without_hashing_an_empty_context() {
    let config = configuration(vec![stage("eng", 1); MAX_MODEL_SELECTIONS + 1]);
    super::super::cacheability::scope(async {
        let dependency = CacheDependency::for_config(&config, Some("application/pdf"));
        assert!(dependency.uncertifiable);
        assert!(dependency.stage_models.is_empty());
        let result = fingerprint_with_reader(&[dependency], None, |_, _| {
            panic!("uncertifiable dependencies must not reach model IO")
        })
        .await
        .unwrap();
        assert!(result.is_none());
        assert!(!super::super::cacheability::is_cacheable());
    })
    .await;
}

#[test]
fn snapshot_selection_cap_prevents_the_first_excess_model_read() {
    let dependencies = (0..=MAX_MODEL_SELECTIONS)
        .map(|index| CacheDependency {
            model: Some(ModelSelection {
                languages: vec![format!("model-{index}")],
                directory: None,
            }),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let mut reads = 0;
    let result = fingerprint_models_using(&dependencies, None, |_| {
        reads += 1;
        Ok(blake3::hash(b"one"))
    });
    assert!(result.unwrap_err().to_string().contains("observation bound exceeded"));
    assert_eq!(reads, MAX_MODEL_SELECTIONS);
}

#[test]
fn distinct_stage_languages_sharing_a_canonical_root_hash_osd_once_per_snapshot() {
    let root = tempfile::tempdir().unwrap();
    for language in ["eng", "fra", "osd"] {
        std::fs::write(root.path().join(format!("{language}.traineddata")), b"one").unwrap();
    }
    let models = ["eng", "fra"].map(|language| ModelSelection {
        languages: vec![language.into()],
        directory: Some(root.path().into()),
    });
    let mut revisions = std::collections::HashMap::new();
    let mut reads = 0;
    for model in &models {
        model_fingerprint_with_reader(model, &mut revisions, |path| {
            reads += 1;
            super::super::cacheability::osd_revision(path)
        })
        .unwrap();
    }
    assert_eq!(reads, 1);
    assert_eq!(revisions.len(), 1);
    let first = fingerprint_models(
        &[CacheDependency {
            stage_models: models.to_vec(),
            ..Default::default()
        }],
        None,
    )
    .unwrap();
    std::fs::write(root.path().join("osd.traineddata"), b"two").unwrap();
    let second = fingerprint_models(
        &[CacheDependency {
            stage_models: models.to_vec(),
            ..Default::default()
        }],
        None,
    )
    .unwrap();
    assert_ne!(first, second, "a fresh snapshot must read changed model bytes");
}

#[tokio::test]
async fn uncertifiable_stage_dependencies_still_honor_cancellation() {
    let config = configuration(vec![stage("eng", 1); MAX_MODEL_SELECTIONS + 1]);
    let dependency = CacheDependency::for_config(&config, Some("image/png"));
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        fingerprint(&[dependency], Some(&cancel)).await,
        Err(crate::XbergError::Cancelled)
    ));
}

#[tokio::test]
async fn non_osd_and_other_backend_stages_keep_the_original_empty_context() {
    let mut other = stage("eng", 1);
    other.backend = "paddleocr".into();
    let mut inherited = stage("eng", 1);
    inherited.tesseract_config.as_mut().unwrap().psm = None;
    let mut config = configuration(vec![other, inherited, stage("jpn_vert", 5)]);
    config.ocr.as_mut().unwrap().tesseract_config = Some(crate::types::TesseractConfig {
        psm: Some(1),
        ..Default::default()
    });
    let dependency = CacheDependency::for_config(&config, Some("image/png"));
    assert!(dependency.model.is_none());
    assert!(dependency.stage_models.is_empty());
    assert!(!dependency.uncertifiable);
    let context = fingerprint_with_reader(&[dependency], None, |_, _| {
        panic!("no effective OSD stage may read models")
    })
    .await
    .unwrap();
    assert_eq!(context.as_deref(), Some(""));
}
