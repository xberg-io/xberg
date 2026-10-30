use super::*;
use crate::corpus::CorpusDocument;
use serde_json::json;

fn scored() -> ModelBenchmarkReport {
    let temp = tempfile::tempdir().unwrap();
    let image = temp.path().join("page.png");
    image::RgbImage::new(100, 200).save(&image).unwrap();
    let fixture = temp.path().join("page.json");
    std::fs::write(&fixture, "{}").unwrap();
    let mut report = super::report();
    let image_hash = hash_file(&image).unwrap();
    let fixture_hash = hash_file(&fixture).unwrap();
    report.expected_documents.get_mut("page").unwrap().document_sha256 = image_hash.clone();
    report.expected_documents.get_mut("page").unwrap().fixture_sha256 = fixture_hash.clone();
    report.rows[0].document_sha256 = image_hash.clone();
    report.rows[0].render_sha256 = image_hash.clone();
    report.rows[0].fixture_sha256 = fixture_hash;
    let coco = json!({"images":[{"id":1,"width":100,"height":200}],
        "categories":[{"id":1,"name":"Equation"}], "annotations":[
        {"id":1,"image_id":1,"category_id":1,"bbox":[5,10,25,30]},
        {"id":2,"image_id":1,"category_id":1,"bbox":[50,60,20,20]}]});
    let coco_bytes = serde_json::to_vec(&coco).unwrap();
    std::fs::write(temp.path().join("coco.json"), &coco_bytes).unwrap();
    let manifest = json!({"schema_version":1,"source":"independent synthetic geometry",
        "coco_file":"coco.json","coco_sha256":hash_bytes(&coco_bytes),
        "category_mapping":{"1":"formula"},"prediction_projection":{"formula":"formula"},
        "pages":[{"document":"page","page":1,"image_id":1,"render_sha256":image_hash,
        "width":100,"height":200,"expected_annotations":2}]});
    let path = temp.path().join("annotations.json");
    std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let document = CorpusDocument {
        name: "page".into(),
        document_path: image,
        file_type: "png".into(),
        file_size: 1,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata: Default::default(),
        fixture_path: fixture,
    };
    let plan = super::super::geometry::load(&path, &[document], &report.expected_documents).unwrap();
    report.geometry = Some(plan.finish(&report).unwrap());
    report.schema_version = 3;
    report.quality_assessed = true;
    report
}

#[test]
fn scored_report_roundtrips_exact_source_bound_metrics_without_claiming_qualification() {
    let report = scored();
    report.validate().unwrap();
    let geometry = report.geometry.as_ref().unwrap();
    assert!(!geometry.qualification_complete);
    assert_eq!(geometry.rows.len(), 1);
    assert_eq!(geometry.iterations.len(), 1);
    assert_eq!(geometry.rows[0].arms[0].metrics.mean_ap_50_95, Some(1.0));
    let decoded: ModelBenchmarkReport = serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
    decoded.validate().unwrap();
}

#[test]
fn fabricated_metric_or_missing_score_rows_fail_recomputation() {
    let mut report = scored();
    report.geometry.as_mut().unwrap().rows[0].arms[0]
        .metrics
        .micro
        .true_positives += 1;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: geometry scores do not match retained detections and annotations"
    );
    let mut report = scored();
    report.geometry.as_mut().unwrap().rows.clear();
    assert!(
        report
            .validate()
            .unwrap_err()
            .to_string()
            .contains("geometry scores do not match")
    );
}

#[test]
fn unmapped_predictions_never_silently_disappear_from_quality_results() {
    let mut report = scored();
    report
        .geometry
        .as_mut()
        .unwrap()
        .prediction_projection
        .remove("formula");
    assert!(
        report
            .validate()
            .unwrap_err()
            .to_string()
            .contains("geometry projection requires")
    );
}

#[test]
fn annotation_or_scorer_identity_changes_invalidate_scored_artifacts() {
    let mut report = scored();
    report.geometry.as_mut().unwrap().scorer_sha256 = "0".repeat(64);
    assert!(
        report
            .validate()
            .unwrap_err()
            .to_string()
            .contains("geometry evidence identity")
    );
    let mut report = scored();
    report.geometry.as_mut().unwrap().pages[0].regions.clear();
    assert!(
        report
            .validate()
            .unwrap_err()
            .to_string()
            .contains("geometry evidence identity")
    );
}

#[test]
fn valid_but_changed_manifest_digest_invalidates_saved_report() {
    let mut report = scored();
    report.geometry.as_mut().unwrap().annotation_manifest_sha256 = "a".repeat(64);
    assert!(report.validate().is_err());
}

#[test]
fn valid_but_changed_coco_digest_invalidates_saved_report() {
    let mut report = scored();
    report.geometry.as_mut().unwrap().coco_sha256 = "b".repeat(64);
    assert!(report.validate().is_err());
}

#[test]
fn retained_source_and_normalized_annotations_must_agree() {
    let original = serde_json::to_value(scored()).unwrap();
    for field in ["manifest_json", "coco_json"] {
        let mut changed = original.clone();
        changed["geometry"]["annotation_sources"][field] = json!("{}");
        let report: ModelBenchmarkReport = serde_json::from_value(changed).unwrap();
        assert!(report.validate().is_err(), "{field}");
    }
    let mut report = scored();
    let geometry = report.geometry.as_mut().unwrap();
    geometry.pages[0].regions[0].region.bbox[0] += 1.0;
    geometry.ground_truth_sha256 = hash_bytes(&serde_json::to_vec(&geometry.pages).unwrap());
    assert!(report.validate().is_err());
    for field in [
        "annotation_sources",
        "pages",
        "source",
        "category_mapping",
        "prediction_projection",
    ] {
        let mut changed = original.clone();
        changed["geometry"].as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<ModelBenchmarkReport>(changed).is_err(),
            "{field}"
        );
    }
}

#[test]
fn rehashed_retained_manifest_cannot_disagree_with_saved_normalization() {
    let mut report = scored();
    let geometry = report.geometry.as_mut().unwrap();
    let mut manifest: serde_json::Value = serde_json::from_str(&geometry.annotation_sources.manifest_json).unwrap();
    manifest["source"] = json!("changed source description");
    geometry.annotation_sources.manifest_json = serde_json::to_string(&manifest).unwrap();
    geometry.annotation_manifest_sha256 = hash_bytes(geometry.annotation_sources.manifest_json.as_bytes());
    assert!(
        report
            .validate()
            .unwrap_err()
            .to_string()
            .contains("normalized annotations differ")
    );
}
