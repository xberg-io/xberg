use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn inputs() -> (AnnotationManifest, Value) {
    let coco = json!({"images":[{"id":1,"width":100,"height":100}],
        "categories":[{"id":1,"name":"Body"}],
        "annotations":[{"id":9,"image_id":1,"category_id":1,"bbox":[1,2,3,4],"iscrowd":0}]});
    let manifest = AnnotationManifest {
        schema_version: 1,
        source: "independent synthetic annotations".into(),
        coco_file: "coco.json".into(),
        coco_sha256: String::new(),
        category_mapping: BTreeMap::from([(1, "text".into())]),
        prediction_projection: BTreeMap::from([("text".into(), "text".into())]),
        pages: vec![PageBinding {
            document: "page".into(),
            page: 1,
            image_id: 1,
            render_sha256: "a".repeat(64),
            width: 100,
            height: 100,
            expected_annotations: 1,
        }],
    };
    (manifest, coco)
}

fn frozen_decode(mut manifest: AnnotationManifest, coco: &Value) -> Result<Vec<AnnotatedPage>, String> {
    let bytes = serde_json::to_vec(coco).unwrap();
    manifest.coco_sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    decode(&manifest, &bytes)
}

#[test]
fn converts_declared_xywh_to_pixel_corners_without_changing_source_identity() {
    let (manifest, coco) = inputs();
    let pages = frozen_decode(manifest, &coco).unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].regions[0].annotation_id, 9);
    assert_eq!(pages[0].regions[0].source_category_id, 1);
    assert_eq!(pages[0].regions[0].region.bbox, [1., 2., 4., 6.]);
}

#[test]
fn changed_annotation_bytes_fail_the_frozen_hash() {
    let (manifest, coco) = inputs();
    assert_eq!(
        decode(&manifest, &serde_json::to_vec(&coco).unwrap()).unwrap_err(),
        "geometry COCO hash mismatch"
    );
}

#[test]
fn missing_page_and_missing_annotations_have_distinct_failures() {
    let (manifest, mut coco) = inputs();
    coco["images"] = json!([]);
    assert_eq!(
        frozen_decode(manifest.clone(), &coco).unwrap_err(),
        "COCO annotation references missing image"
    );
    let (_, mut coco) = inputs();
    coco["annotations"] = json!([]);
    assert_eq!(
        frozen_decode(manifest, &coco).unwrap_err(),
        "geometry annotation count mismatch"
    );
}

#[test]
fn explicit_zero_annotation_page_remains_valid_and_missing_image_never_does() {
    let (mut manifest, mut coco) = inputs();
    manifest.pages[0].expected_annotations = 0;
    coco["annotations"] = json!([]);
    assert!(frozen_decode(manifest.clone(), &coco).unwrap()[0].regions.is_empty());
    coco["images"] = json!([]);
    assert_eq!(
        frozen_decode(manifest, &coco).unwrap_err(),
        "geometry binding references missing image"
    );
}

#[test]
fn duplicate_annotations_unknown_categories_and_crowds_are_rejected() {
    let (manifest, coco) = inputs();
    for (field, value, error) in [
        ("category_id", json!(2), "COCO annotation references unmapped category"),
        (
            "iscrowd",
            json!(1),
            "crowd and ignored COCO annotations are unsupported",
        ),
        ("bbox", json!([1, 2, -3, 4]), "invalid or off-page geometry box"),
    ] {
        let mut changed = coco.clone();
        changed["annotations"][0][field] = value;
        assert_eq!(frozen_decode(manifest.clone(), &changed).unwrap_err(), error);
    }
    let mut changed = coco;
    let duplicate = changed["annotations"][0].clone();
    changed["annotations"].as_array_mut().unwrap().push(duplicate);
    assert_eq!(
        frozen_decode(manifest, &changed).unwrap_err(),
        "duplicate COCO annotation ID"
    );
}

#[test]
fn mapping_page_dimensions_and_duplicate_bindings_fail_closed() {
    let (manifest, coco) = inputs();
    let mut changed = manifest.clone();
    changed.category_mapping.clear();
    assert_eq!(
        frozen_decode(changed, &coco).unwrap_err(),
        "geometry mapping must explicitly cover every source category"
    );
    let mut changed = manifest.clone();
    changed.pages[0].width = 90;
    assert_eq!(
        frozen_decode(changed, &coco).unwrap_err(),
        "geometry annotation dimensions mismatch"
    );
    let mut changed = manifest;
    changed.pages.push(changed.pages[0].clone());
    assert_eq!(
        frozen_decode(changed, &coco).unwrap_err(),
        "invalid or duplicate geometry page binding"
    );
}

fn retained() -> (RetainedAnnotations, String, String) {
    let (mut manifest, coco) = inputs();
    let coco_json = serde_json::to_string(&coco).unwrap();
    manifest.coco_sha256 = digest(coco_json.as_bytes());
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    let manifest_hash = digest(manifest_json.as_bytes());
    let coco_hash = manifest.coco_sha256.clone();
    (
        RetainedAnnotations {
            manifest_json,
            coco_json,
        },
        manifest_hash,
        coco_hash,
    )
}

#[test]
fn retained_manifest_valid_digest_mutation_is_rejected() {
    let (sources, _, coco_hash) = retained();
    assert_eq!(
        sources.decode(&"a".repeat(64), &coco_hash).unwrap_err(),
        "geometry manifest hash mismatch"
    );
}

#[test]
fn retained_coco_valid_digest_mutation_is_rejected() {
    let (sources, manifest_hash, _) = retained();
    assert_eq!(
        sources.decode(&manifest_hash, &"b".repeat(64)).unwrap_err(),
        "geometry COCO hash mismatch"
    );
}

#[test]
fn retained_source_content_mutations_cannot_reuse_original_identities() {
    let (sources, manifest_hash, coco_hash) = retained();
    let (manifest, pages) = sources.decode(&manifest_hash, &coco_hash).unwrap();
    assert_eq!(manifest.source, "independent synthetic annotations");
    assert_eq!(pages[0].regions[0].region.bbox, [1., 2., 4., 6.]);
    let mut changed = sources.clone();
    changed.manifest_json.push(' ');
    assert!(changed.decode(&manifest_hash, &coco_hash).is_err());
    let mut changed = sources;
    changed.coco_json.push(' ');
    assert!(changed.decode(&manifest_hash, &coco_hash).is_err());
}

#[test]
fn retained_sources_require_both_exact_source_fields() {
    let (sources, _, _) = retained();
    let value = serde_json::to_value(sources).unwrap();
    for field in ["manifest_json", "coco_json"] {
        let mut changed = value.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<RetainedAnnotations>(changed).is_err());
    }
}
