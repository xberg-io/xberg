use super::model_file_manifest;

fn manifest() -> String {
    ["model.onnx", "model.onnx.data", "tokenizer.json", "config.json"]
        .map(|path| format!("{}  {path}\n", "a".repeat(64)))
        .concat()
}

#[test]
fn supplied_empty_manifests_cannot_disable_artifact_verification() {
    for content in ["", "# only comments\n"] {
        let error = model_file_manifest(Some(content), "model.onnx", &[]).expect_err("empty supplied manifest");
        assert!(error.contains("no artifacts"), "{error}");
    }
}

#[test]
fn selected_graph_must_be_pinned_before_any_artifact_resolution() {
    let error = model_file_manifest(Some(&manifest()), "missing.onnx", &[]).expect_err("missing graph");
    assert!(error.contains("missing.onnx"), "{error}");
}

#[test]
fn every_selected_external_weight_file_must_be_pinned_before_resolution() {
    let siblings = vec!["model.onnx.data".to_owned(), "missing.data".to_owned()];
    let error = model_file_manifest(Some(&manifest()), "model.onnx", &siblings).expect_err("missing weights");
    assert!(error.contains("missing.data"), "{error}");
}

#[test]
fn complete_selected_graph_and_weights_keep_the_manifest_entries() {
    let siblings = vec!["model.onnx.data".to_owned()];
    let entries = model_file_manifest(Some(&manifest()), "model.onnx", &siblings).expect("complete selection");
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0], ("model.onnx".to_owned(), "a".repeat(64)));
    assert_eq!(entries[1], ("model.onnx.data".to_owned(), "a".repeat(64)));
}

#[test]
fn absent_manifests_preserve_caller_managed_custom_models() {
    assert!(
        model_file_manifest(None, "caller/model.onnx", &["caller/weights.data".to_owned()])
            .unwrap()
            .is_empty()
    );
}
