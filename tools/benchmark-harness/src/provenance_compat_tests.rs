use super::*;

#[test]
fn old_framework_provenance_deserializes_without_thread_budget() {
    let provenance: FrameworkProvenance = serde_json::from_value(serde_json::json!({
        "name": "xberg-markdown-baseline-batch",
        "version": "1.0.0",
        "executable": null,
        "models": [],
        "batch_capability": null,
        "requested_workers": 4,
        "effective_workers": null,
        "worker_semantics": "legacy",
        "effective_warmup_iterations": 0,
        "eligible_documents": 4,
        "batch_partitions": 1
    }))
    .unwrap();

    assert_eq!(provenance.configured_thread_budget, None);
    assert_eq!(provenance.tessdata_model, None);
}

#[test]
fn framework_provenance_serializes_best_tessdata_model_and_omits_the_default() {
    let mut provenance: FrameworkProvenance = serde_json::from_value(serde_json::json!({
        "name": "xberg-markdown-baseline-batch",
        "version": "1.0.0",
        "executable": null,
        "models": [],
        "batch_capability": null,
        "requested_workers": 4,
        "effective_workers": null,
        "worker_semantics": "legacy",
        "effective_warmup_iterations": 0,
        "eligible_documents": 4,
        "batch_partitions": 1
    }))
    .unwrap();

    assert!(
        serde_json::to_value(&provenance)
            .unwrap()
            .get("tessdata_model")
            .is_none()
    );

    provenance.tessdata_model = Some(xberg::TessdataModel::Best);
    let value = serde_json::to_value(&provenance).unwrap();
    assert_eq!(value["tessdata_model"], "best");
    let restored: FrameworkProvenance = serde_json::from_value(value).unwrap();
    assert_eq!(restored.tessdata_model, Some(xberg::TessdataModel::Best));
}

#[test]
fn xberg_provenance_records_only_a_non_default_tessdata_model() {
    use crate::adapters::subprocess::SubprocessAdapter;

    let best = SubprocessAdapter::new("xberg-test", "echo", vec![], vec![], vec!["pdf".to_string()])
        .with_tessdata_model(xberg::TessdataModel::Best);
    let fast = SubprocessAdapter::new("xberg-test", "echo", vec![], vec![], vec!["pdf".to_string()])
        .with_tessdata_model(xberg::TessdataModel::Fast);
    let other = SubprocessAdapter::new("docling", "python", vec![], vec![], vec!["pdf".to_string()])
        .with_tessdata_model(xberg::TessdataModel::Best);

    assert_eq!(best.configured_tessdata_model(), Some(xberg::TessdataModel::Best));
    assert_eq!(fast.configured_tessdata_model(), None);
    assert_eq!(other.configured_tessdata_model(), None);
}
